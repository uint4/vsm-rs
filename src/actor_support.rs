use ractor::{call_t, Actor, ActorCell, ActorProcessingErr, ActorRef, RpcReplyPort, SpawnErr};
use ractor::concurrency::Duration;
use ractor_supervisor::{ChildSpec, Restart, SpawnFn, Supervisor};
use serde_json::{json, Value};

use crate::channels::broker::VsmActorMsg;
use crate::error::{VsmError, VsmResult};
use crate::prelude::now_json;
use crate::shared::message::{ChannelKind, MessageKind, SystemId, VsmMessage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceKind {
    Algedonic,
    TemporalVariety,
    System2Coordination,
    System3Control,
    System4Intelligence,
    System4Scanner,
    System4Analytics,
    System4Forecasting,
    System5Policy,
    System5Identity,
    System5Values,
    System5Decisions,
    TelemetryReporter,
}

pub enum ServiceMsg {
    Call(String, Value, RpcReplyPort<VsmResult<Value>>),
    Cast(String, Value),
    Channel(VsmActorMsg),
    Tick(String),
}

impl From<VsmActorMsg> for ServiceMsg {
    fn from(value: VsmActorMsg) -> Self { Self::Channel(value) }
}

impl TryFrom<ServiceMsg> for VsmActorMsg {
    type Error = ServiceMsg;
    fn try_from(value: ServiceMsg) -> Result<Self, Self::Error> {
        match value { ServiceMsg::Channel(inner) => Ok(inner), other => Err(other) }
    }
}

#[derive(Clone)]
pub struct ServiceActor { pub kind: ServiceKind }

#[derive(Debug, Clone)]
pub struct ServiceState {
    pub kind: ServiceKind,
    pub data: Value,
    pub history: Vec<Value>,
}

impl ServiceState {
    pub fn new(kind: ServiceKind, config: Value) -> Self {
        let id = match kind {
            ServiceKind::System2Coordination => "system2",
            ServiceKind::System3Control => "system3",
            ServiceKind::System4Intelligence | ServiceKind::System4Scanner | ServiceKind::System4Analytics | ServiceKind::System4Forecasting => "system4",
            ServiceKind::System5Policy | ServiceKind::System5Identity | ServiceKind::System5Values | ServiceKind::System5Decisions => "system5",
            ServiceKind::Algedonic => "algedonic",
            ServiceKind::TemporalVariety => "temporal_variety",
            ServiceKind::TelemetryReporter => "telemetry",
        };
        Self { kind, data: json!({"id": id, "config": config, "started_at": now_json(), "status":"running"}), history: Vec::new() }
    }
    pub fn record(&mut self, event: Value) {
        self.history.insert(0, event);
        self.history.truncate(1_000);
    }
}

#[ractor::async_trait]
impl Actor for ServiceActor {
    type Msg = ServiceMsg;
    type State = ServiceState;
    type Arguments = Value;

    async fn pre_start(&self, myself: ActorRef<Self::Msg>, args: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        match self.kind {
            ServiceKind::System2Coordination => subscribe(myself.clone(), ChannelKind::Coordination, "system2").await?,
            ServiceKind::System3Control => {
                subscribe(myself.clone(), ChannelKind::ResourceBargain, "system3").await?;
                subscribe(myself.clone(), ChannelKind::Command, "system3").await?;
                subscribe(myself.clone(), ChannelKind::Audit, "system3").await?;
            }
            ServiceKind::System4Intelligence => subscribe(myself.clone(), ChannelKind::Command, "system4").await?,
            ServiceKind::System5Policy => subscribe(myself.clone(), ChannelKind::Algedonic, "system5").await?,
            ServiceKind::Algedonic => subscribe(myself.clone(), ChannelKind::Algedonic, "algedonic").await?,
            ServiceKind::TemporalVariety => subscribe(myself.clone(), ChannelKind::TemporalVariety, "temporal_variety").await?,
            _ => {}
        }
        Ok(ServiceState::new(self.kind, args))
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            ServiceMsg::Call(op, payload, reply) => {
                let result = handle_service_call(self.kind, &op, payload, state).await;
                if !reply.is_closed() { let _ = reply.send(result); }
            }
            ServiceMsg::Cast(op, payload) => { let _ = handle_service_cast(self.kind, &op, payload, state).await; }
            ServiceMsg::Channel(msg) => {
                let payload = match msg { VsmActorMsg::ChannelMessage(message) | VsmActorMsg::AlgedonicSignal(message) => serde_json::to_value(message).unwrap_or(Value::Null) };
                state.record(json!({"event":"channel_message", "payload": payload, "timestamp": now_json()}));
            }
            ServiceMsg::Tick(kind) => state.record(json!({"event":"tick", "kind": kind, "timestamp": now_json()})),
        }
        Ok(())
    }
}

async fn subscribe(myself: ActorRef<ServiceMsg>, channel: ChannelKind, subscriber_id: &'static str) -> Result<(), ActorProcessingErr> {
    crate::channels::subscribe(channel, subscriber_id, myself.get_derived::<VsmActorMsg>()).await.map_err(|e| -> ActorProcessingErr { e.into() })
}

pub async fn call_service(name: &str, op: &str, payload: Value) -> VsmResult<Value> {
    let actor = ActorRef::<ServiceMsg>::where_is(name.to_string()).ok_or_else(|| VsmError::ActorUnavailable(name.to_string()))?;
    call_t!(actor, ServiceMsg::Call, 5_000, op.to_string(), payload).map_err(|err| VsmError::Runtime(err.to_string()))?
}

pub fn cast_service(name: &str, op: &str, payload: Value) -> VsmResult<()> {
    let actor = ActorRef::<ServiceMsg>::where_is(name.to_string()).ok_or_else(|| VsmError::ActorUnavailable(name.to_string()))?;
    actor.send_message(ServiceMsg::Cast(op.to_string(), payload)).map_err(|err| VsmError::Runtime(err.to_string()))
}

pub fn service_child(name: &'static str, kind: ServiceKind, args: Value) -> ChildSpec {
    ChildSpec { id: name.to_string(), restart: Restart::Permanent, spawn_fn: SpawnFn::new(move |supervisor_cell, child_id| { let args = args.clone(); async move { spawn_service(supervisor_cell, child_id, kind, args).await } }), backoff_fn: None, reset_after: Some(Duration::from_secs(60)) }
}

async fn spawn_service(supervisor_cell: ActorCell, child_id: String, kind: ServiceKind, args: Value) -> Result<ActorCell, SpawnErr> {
    let (actor, _join) = Supervisor::spawn_linked(child_id, ServiceActor { kind }, args, supervisor_cell).await?;
    Ok(actor.get_cell())
}

pub async fn handle_service_call(kind: ServiceKind, op: &str, payload: Value, state: &mut ServiceState) -> VsmResult<Value> {
    state.record(json!({"event":"call", "op": op, "payload": payload.clone(), "timestamp": now_json()}));
    match kind {
        ServiceKind::Algedonic => crate::channels::algedonic::actor_call(op, payload, state).await,
        ServiceKind::TemporalVariety => crate::channels::temporal_variety::actor_call(op, payload, state).await,
        ServiceKind::System2Coordination => crate::system2::coordination::actor_call(op, payload, state).await,
        ServiceKind::System3Control => crate::system3::control::actor_call(op, payload, state).await,
        ServiceKind::System4Intelligence => crate::system4::intelligence::actor_call(op, payload, state).await,
        ServiceKind::System4Scanner => crate::system4::scanner::actor_call(op, payload, state).await,
        ServiceKind::System4Analytics => crate::system4::analytics::actor_call(op, payload, state).await,
        ServiceKind::System4Forecasting => crate::system4::forecasting::actor_call(op, payload, state).await,
        ServiceKind::System5Policy => crate::system5::policy::actor_call(op, payload, state).await,
        ServiceKind::System5Identity => crate::system5::identity::actor_call(op, payload, state).await,
        ServiceKind::System5Values => crate::system5::values::actor_call(op, payload, state).await,
        ServiceKind::System5Decisions => crate::system5::decisions::actor_call(op, payload, state).await,
        ServiceKind::TelemetryReporter => Ok(json!({"status":"ok", "history_len": state.history.len(), "data": state.data.clone()})),
    }
}

pub async fn handle_service_cast(kind: ServiceKind, op: &str, payload: Value, state: &mut ServiceState) -> VsmResult<()> {
    state.record(json!({"event":"cast", "op": op, "payload": payload.clone(), "timestamp": now_json()}));
    match kind {
        ServiceKind::Algedonic if op == "signal" => {
            crate::channels::publish(VsmMessage::new(SystemId::External, SystemId::System5, ChannelKind::Algedonic, MessageKind::Alert, payload))?;
            Ok(())
        }
        ServiceKind::System4Forecasting if op == "update_models" => { state.data["last_model_update"] = now_json(); Ok(()) }
        _ => Ok(()),
    }
}
