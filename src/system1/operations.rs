use std::collections::HashMap;

use chrono::{DateTime, Utc};
use ractor::{call_t, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use ractor_supervisor::{ChildSpec, DynamicSupervisor, DynamicSupervisorMsg, Restart, SpawnFn};
use ractor::concurrency::Duration;
use serde_json::{json, Value};

use crate::channels;
use crate::channels::broker::VsmActorMsg;
use crate::domain::{ChannelKind, MessageKind, SystemId, VsmMessage};
use crate::error::VsmError;
use crate::names;
use crate::system1::metrics::MetricsStore;
use crate::system1::transaction::{
    calculate_input_variety, calculate_output_variety, Transaction, TransactionResult,
};
use crate::system1::types::{
    CoordinationRequest, MetricsSnapshot, UnitConfig, UnitId, UnitSummary, VarietyMeasurement,
    VarietySnapshot, VarietyTrend, WorkMigrationDirection,
};
use crate::system1::unit::{Unit, UnitMsg};

pub enum OperationsMsg {
    RegisterUnit(UnitConfig, RpcReplyPort<Result<UnitId, VsmError>>),
    ProcessTransaction(Transaction, RpcReplyPort<TransactionResult>),
    GetVariety(RpcReplyPort<VarietySnapshot>),
    GetMetrics(RpcReplyPort<MetricsSnapshot>),
    ListUnits(RpcReplyPort<Vec<UnitSummary>>),
    SendAlgedonicSignal(Value),
    Channel(VsmActorMsg),
}

impl From<VsmActorMsg> for OperationsMsg {
    fn from(value: VsmActorMsg) -> Self {
        Self::Channel(value)
    }
}

impl TryFrom<OperationsMsg> for VsmActorMsg {
    type Error = OperationsMsg;

    fn try_from(value: OperationsMsg) -> Result<Self, Self::Error> {
        match value {
            OperationsMsg::Channel(message) => Ok(message),
            other => Err(other),
        }
    }
}

#[derive(Debug, Clone)]
struct UnitInfo {
    actor_name: String,
    config: UnitConfig,
    started_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OperationsArgs {
    pub config: Value,
}

impl Default for OperationsArgs {
    fn default() -> Self {
        Self { config: json!({}) }
    }
}

pub struct OperationsState {
    units: HashMap<UnitId, UnitInfo>,
    unit_supervisor: ActorRef<DynamicSupervisorMsg>,
    metrics: MetricsStore,
    variety_log: Vec<VarietyMeasurement>,
    config: Value,
}

pub struct Operations;

#[ractor::async_trait]
impl Actor for Operations {
    type Msg = OperationsMsg;
    type State = OperationsState;
    type Arguments = OperationsArgs;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let unit_supervisor = ActorRef::<DynamicSupervisorMsg>::where_is(
            names::SYSTEM1_UNIT_SUPERVISOR.to_string(),
        )
        .ok_or_else(|| boxed_err(VsmError::ActorNotFound(names::SYSTEM1_UNIT_SUPERVISOR.to_string())))?;

        let channel_ref = myself.get_derived::<VsmActorMsg>();

        channels::subscribe(ChannelKind::Coordination, "system1", channel_ref.clone())
            .await
            .map_err(boxed_err)?;
        channels::subscribe(ChannelKind::Audit, "system1", channel_ref.clone())
            .await
            .map_err(boxed_err)?;
        channels::subscribe(ChannelKind::Command, "system1", channel_ref)
            .await
            .map_err(boxed_err)?;

        tracing::info!("System 1 Operations started");

        Ok(OperationsState {
            units: HashMap::new(),
            unit_supervisor,
            metrics: MetricsStore::default(),
            variety_log: Vec::new(),
            config: args.config,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            OperationsMsg::RegisterUnit(unit_config, reply) => {
                let result = register_unit_impl(unit_config, state).await;
                let _ = reply.send(result);
            }

            OperationsMsg::ProcessTransaction(transaction, reply) => {
                let result = process_transaction_impl(transaction, state).await;
                let _ = reply.send(result);
            }

            OperationsMsg::GetVariety(reply) => {
                let _ = reply.send(calculate_current_variety(&state.variety_log));
            }

            OperationsMsg::GetMetrics(reply) => {
                let _ = reply.send(state.metrics.snapshot());
            }

            OperationsMsg::ListUnits(reply) => {
                let _ = reply.send(list_units_impl(state).await);
            }

            OperationsMsg::SendAlgedonicSignal(signal) => {
                send_algedonic_signal_impl(signal).map_err(boxed_err)?;
            }

            OperationsMsg::Channel(VsmActorMsg::ChannelMessage(message)) => {
                handle_channel_message(message, state).await.map_err(boxed_err)?;
            }

            OperationsMsg::Channel(VsmActorMsg::AlgedonicSignal(message)) => {
                tracing::warn!(?message, "System 1 received unexpected algedonic signal");
            }
        }

        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        let _ = channels::unsubscribe(ChannelKind::Coordination, "system1").await;
        let _ = channels::unsubscribe(ChannelKind::Audit, "system1").await;
        let _ = channels::unsubscribe(ChannelKind::Command, "system1").await;
        Ok(())
    }
}

fn boxed_err<E>(err: E) -> ActorProcessingErr
where
    E: std::error::Error + Send + Sync + 'static,
{
    Box::new(err)
}

async fn register_unit_impl(
    unit_config: UnitConfig,
    state: &mut OperationsState,
) -> Result<UnitId, VsmError> {
    let unit_id = unit_config.id.clone();

    if state.units.contains_key(&unit_id) {
        return Err(VsmError::UnitAlreadyRegistered(unit_id));
    }

    let actor_name = Unit::actor_name(&unit_id);
    let config_for_spawn = unit_config.clone();

    let spec = ChildSpec {
        id: actor_name.clone(),
        restart: if unit_config.auto_restart {
            Restart::Permanent
        } else {
            Restart::Temporary
        },
        spawn_fn: SpawnFn::new(move |supervisor_cell, child_id| {
            let config = config_for_spawn.clone();
            async move {
                let (unit_ref, _join) = DynamicSupervisor::spawn_linked(
                    child_id,
                    Unit,
                    config,
                    supervisor_cell,
                )
                .await?;

                Ok(unit_ref.get_cell())
            }
        }),
        backoff_fn: None,
        reset_after: Some(Duration::from_secs(60)),
    };

    DynamicSupervisor::spawn_child(state.unit_supervisor.clone(), spec)
        .await
        .map_err(|err| VsmError::Supervisor(err.to_string()))?;

    let unit_ref = ActorRef::<UnitMsg>::where_is(actor_name.clone())
        .ok_or_else(|| VsmError::ActorNotFound(actor_name.clone()))?;

    let _status = call_t!(unit_ref, UnitMsg::GetStatus, 1_000)
        .map_err(|err| VsmError::Ractor(err.to_string()))?;

    state.units.insert(
        unit_id.clone(),
        UnitInfo {
            actor_name,
            config: unit_config.clone(),
            started_at: Utc::now(),
        },
    );

    channels::publish(VsmMessage::new(
        SystemId::System1,
        SystemId::System2,
        ChannelKind::Coordination,
        MessageKind::UnitRegistered,
        json!({
            "unit_id": unit_id,
            "capabilities": unit_config.capabilities.clone(),
        }),
    ))?;

    tracing::info!(unit_id = %unit_config.id, "registered operational unit");

    Ok(unit_config.id)
}

async fn process_transaction_impl(
    transaction: Transaction,
    state: &mut OperationsState,
) -> TransactionResult {
    if let Err(reason) = transaction.validate() {
        tracing::warn!(?transaction, %reason, "invalid transaction");
        let result = TransactionResult::InvalidTransaction(reason);
        state.metrics.record_transaction(&result);
        return result;
    }

    let selected = select_unit(&transaction, state).await;

    let result = match selected {
        Some((unit_id, unit_ref)) => match call_t!(
            unit_ref,
            UnitMsg::Process,
            5_000,
            transaction.clone()
        ) {
            Ok(result) => result,
            Err(err) => TransactionResult::UnitError(format!(
                "unit {unit_id} failed while processing transaction: {err}"
            )),
        },

        None => {
            let _ = channels::publish(VsmMessage::new(
                SystemId::System1,
                SystemId::System3,
                ChannelKind::ResourceBargain,
                MessageKind::UnitRequest,
                json!({
                    "transaction_type": transaction.kind.clone(),
                    "required_capabilities": transaction.required_capabilities.clone(),
                }),
            ));

            TransactionResult::NoSuitableUnit
        }
    };

    state.metrics.record_transaction(&result);
    let variety = calculate_variety(&transaction, &result);
    update_variety_log(state, variety);

    result
}

async fn select_unit(
    transaction: &Transaction,
    state: &mut OperationsState,
) -> Option<(UnitId, ActorRef<UnitMsg>)> {
    let mut best: Option<(UnitId, ActorRef<UnitMsg>, f64)> = None;
    let mut stale_units = Vec::new();

    for (unit_id, info) in state.units.iter() {
        let Some(unit_ref) = ActorRef::<UnitMsg>::where_is(info.actor_name.clone()) else {
            stale_units.push(unit_id.clone());
            continue;
        };

        let can_handle = call_t!(
            unit_ref.clone(),
            UnitMsg::CanHandle,
            1_000,
            transaction.clone()
        )
        .unwrap_or(false);

        if !can_handle {
            continue;
        }

        let load = call_t!(unit_ref.clone(), UnitMsg::GetLoad, 1_000).unwrap_or(f64::MAX);

        match &best {
            None => best = Some((unit_id.clone(), unit_ref, load)),
            Some((_best_unit_id, _best_ref, best_load)) if load < *best_load => {
                best = Some((unit_id.clone(), unit_ref, load));
            }
            _ => {}
        }
    }

    for unit_id in stale_units {
        tracing::warn!(unit_id = %unit_id, "removing stale System 1 unit from Operations state");
        state.units.remove(&unit_id);
    }

    best.map(|(unit_id, unit_ref, _load)| (unit_id, unit_ref))
}

fn calculate_variety(transaction: &Transaction, result: &TransactionResult) -> VarietyMeasurement {
    let input = calculate_input_variety(transaction);
    let output = calculate_output_variety(result);
    let ratio = output / input.max(1.0);

    VarietyMeasurement {
        timestamp: Utc::now(),
        input,
        output,
        ratio,
    }
}

fn update_variety_log(state: &mut OperationsState, measurement: VarietyMeasurement) {
    state.variety_log.insert(0, measurement);
    state.variety_log.truncate(1_000);
}

fn calculate_current_variety(log: &[VarietyMeasurement]) -> VarietySnapshot {
    if log.is_empty() {
        return VarietySnapshot::default();
    }

    let recent_len = log.len().min(100);
    let recent = &log[..recent_len];

    let avg_input = recent.iter().map(|item| item.input).sum::<f64>() / recent_len as f64;
    let avg_output = recent.iter().map(|item| item.output).sum::<f64>() / recent_len as f64;
    let ratio = avg_output / avg_input.max(1.0);

    VarietySnapshot {
        input: avg_input,
        output: avg_output,
        ratio,
        trend: calculate_trend(log),
    }
}

fn calculate_trend(log: &[VarietyMeasurement]) -> VarietyTrend {
    if log.len() < 2 {
        return VarietyTrend::Stable;
    }

    let recent_len = log.len().min(10);
    let recent = &log[..recent_len];

    let older_start = recent_len;
    let older_end = (older_start + 10).min(log.len());
    let older = &log[older_start..older_end];

    let recent_avg = recent.iter().map(|item| item.ratio).sum::<f64>() / recent.len() as f64;
    let older_avg = if older.is_empty() {
        recent_avg
    } else {
        older.iter().map(|item| item.ratio).sum::<f64>() / older.len() as f64
    };

    if recent_avg > older_avg * 1.1 {
        VarietyTrend::Increasing
    } else if recent_avg < older_avg * 0.9 {
        VarietyTrend::Decreasing
    } else {
        VarietyTrend::Stable
    }
}

async fn list_units_impl(state: &OperationsState) -> Vec<UnitSummary> {
    let mut units = Vec::new();

    for (unit_id, info) in &state.units {
        let status = match ActorRef::<UnitMsg>::where_is(info.actor_name.clone()) {
            Some(unit_ref) => call_t!(unit_ref, UnitMsg::GetStatus, 1_000)
                .unwrap_or_else(|_| "unknown".to_string()),
            None => "down".to_string(),
        };

        units.push(UnitSummary {
            id: unit_id.clone(),
            status,
            config: info.config.clone(),
            started_at: info.started_at,
        });
    }

    units
}

fn send_algedonic_signal_impl(signal: Value) -> Result<(), VsmError> {
    channels::publish(VsmMessage::new(
        SystemId::System1,
        SystemId::System5,
        ChannelKind::Algedonic,
        MessageKind::Alert,
        signal,
    ))?;

    tracing::warn!("algedonic signal sent from System 1 to System 5");
    Ok(())
}

async fn handle_channel_message(
    message: VsmMessage,
    state: &mut OperationsState,
) -> Result<(), VsmError> {
    match (message.channel, message.kind) {
        (ChannelKind::Command, MessageKind::Execute) => {
            handle_command(message.payload, state).await;
        }

        (ChannelKind::Coordination, MessageKind::Coordinate) => {
            handle_coordination(message.payload, state).await?;
        }

        (ChannelKind::Audit, MessageKind::AuditRequest) => {
            handle_audit(message.payload, state)?;
        }

        (channel, kind) => {
            tracing::debug!(?channel, ?kind, "System 1 ignored channel message");
        }
    }

    Ok(())
}

async fn handle_command(command: Value, state: &OperationsState) {
    tracing::info!(?command, "System 1 received command");

    for info in state.units.values() {
        if let Some(unit_ref) = ActorRef::<UnitMsg>::where_is(info.actor_name.clone()) {
            let _ = unit_ref.send_message(UnitMsg::ExecuteCommand(command.clone()));
        }
    }
}

async fn handle_coordination(payload: Value, state: &OperationsState) -> Result<(), VsmError> {
    let request: CoordinationRequest = serde_json::from_value(payload)
        .map_err(|err| VsmError::InvalidPayload(err.to_string()))?;

    match request {
        CoordinationRequest::SyncState { unit_ids } => sync_unit_states(unit_ids, state).await,
        CoordinationRequest::LoadBalance { unit_ids } => balance_unit_loads(unit_ids, state).await,
    }

    Ok(())
}

fn handle_audit(_payload: Value, state: &OperationsState) -> Result<(), VsmError> {
    channels::publish(VsmMessage::new(
        SystemId::System1,
        SystemId::System3Star,
        ChannelKind::Audit,
        MessageKind::AuditResponse,
        json!({
            "units": state.units.keys().cloned().collect::<Vec<_>>(),
            "metrics": state.metrics.snapshot(),
            "variety": calculate_current_variety(&state.variety_log),
            "timestamp": Utc::now(),
            "config": state.config.clone(),
        }),
    ))
}

async fn sync_unit_states(unit_ids: Vec<UnitId>, state: &OperationsState) {
    let mut states = Vec::new();

    for unit_id in unit_ids {
        let Some(info) = state.units.get(&unit_id) else {
            continue;
        };

        let Some(unit_ref) = ActorRef::<UnitMsg>::where_is(info.actor_name.clone()) else {
            continue;
        };

        if let Ok(unit_state) = call_t!(unit_ref, UnitMsg::GetState, 1_000) {
            states.push((unit_id, unit_state));
        }
    }

    let merged = merge_unit_states(states);

    for info in state.units.values() {
        if let Some(unit_ref) = ActorRef::<UnitMsg>::where_is(info.actor_name.clone()) {
            let _ = unit_ref.send_message(UnitMsg::UpdateState(merged.clone()));
        }
    }
}

fn merge_unit_states(states: Vec<(UnitId, Value)>) -> Value {
    let mut merged = serde_json::Map::new();

    for (_unit_id, state) in states {
        let Value::Object(map) = state else {
            continue;
        };

        for (key, incoming) in map {
            match merged.get(&key) {
                None => {
                    merged.insert(key, incoming);
                }
                Some(existing) => {
                    if compare_timestamps(&incoming, existing).is_gt() {
                        merged.insert(key, incoming);
                    }
                }
            }
        }
    }

    Value::Object(merged)
}

fn compare_timestamps(left: &Value, right: &Value) -> std::cmp::Ordering {
    let left_ts = left
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok());

    let right_ts = right
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok());

    match (left_ts, right_ts) {
        (Some(left_ts), Some(right_ts)) => left_ts.cmp(&right_ts),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

async fn balance_unit_loads(unit_ids: Vec<UnitId>, state: &OperationsState) {
    let mut loads = Vec::new();

    for unit_id in unit_ids {
        let Some(info) = state.units.get(&unit_id) else {
            continue;
        };

        let Some(unit_ref) = ActorRef::<UnitMsg>::where_is(info.actor_name.clone()) else {
            continue;
        };

        if let Ok(load) = call_t!(unit_ref.clone(), UnitMsg::GetLoad, 1_000) {
            loads.push((unit_id, unit_ref, load));
        }
    }

    if loads.is_empty() {
        return;
    }

    let total_load = loads.iter().map(|(_unit_id, _unit_ref, load)| *load).sum::<f64>();
    let avg_load = total_load / loads.len() as f64;

    for (_unit_id, unit_ref, load) in loads {
        if load > avg_load * 1.2 {
            let _ = unit_ref.send_message(UnitMsg::MigrateWork(
                WorkMigrationDirection::Out,
                load - avg_load,
            ));
        } else if load < avg_load * 0.8 {
            let _ = unit_ref.send_message(UnitMsg::MigrateWork(
                WorkMigrationDirection::In,
                avg_load - load,
            ));
        }
    }
}

pub fn operations_ref() -> Result<ActorRef<OperationsMsg>, VsmError> {
    ActorRef::<OperationsMsg>::where_is(names::SYSTEM1_OPERATIONS.to_string())
        .ok_or_else(|| VsmError::ActorNotFound(names::SYSTEM1_OPERATIONS.to_string()))
}

pub async fn register_unit(unit_config: UnitConfig) -> Result<UnitId, VsmError> {
    let operations = operations_ref()?;

    call_t!(
        operations,
        OperationsMsg::RegisterUnit,
        5_000,
        unit_config
    )
    .map_err(|err| VsmError::Ractor(err.to_string()))?
}

pub async fn process_transaction(transaction: Transaction) -> Result<TransactionResult, VsmError> {
    let operations = operations_ref()?;

    call_t!(
        operations,
        OperationsMsg::ProcessTransaction,
        10_000,
        transaction
    )
    .map_err(|err| VsmError::Ractor(err.to_string()))
}

pub async fn get_variety() -> Result<VarietySnapshot, VsmError> {
    let operations = operations_ref()?;

    call_t!(operations, OperationsMsg::GetVariety, 2_000)
        .map_err(|err| VsmError::Ractor(err.to_string()))
}

pub async fn get_metrics() -> Result<MetricsSnapshot, VsmError> {
    let operations = operations_ref()?;

    call_t!(operations, OperationsMsg::GetMetrics, 2_000)
        .map_err(|err| VsmError::Ractor(err.to_string()))
}

pub async fn list_units() -> Result<Vec<UnitSummary>, VsmError> {
    let operations = operations_ref()?;

    call_t!(operations, OperationsMsg::ListUnits, 2_000)
        .map_err(|err| VsmError::Ractor(err.to_string()))
}

pub fn send_algedonic_signal(signal: Value) -> Result<(), VsmError> {
    let operations = operations_ref()?;

    operations
        .send_message(OperationsMsg::SendAlgedonicSignal(signal))
        .map_err(|_err| VsmError::Ractor("failed to send algedonic request to System 1 Operations".to_string()))
}
