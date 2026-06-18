use std::sync::{Arc, Mutex};

use ractor::{Actor, ActorProcessingErr, ActorRef};
use serde_json::json;
use serial_test::serial;
use tokio::time::{sleep, Duration};

use vsm_rs::actor_support::call_service;
use vsm_rs::channels::broker::VsmActorMsg;
use vsm_rs::system1::{Transaction, TransactionResult};
use vsm_rs::{channels, names, ChannelKind, MessageKind, SystemId, VsmApplication, VsmMessage};

type SeenMessages = Arc<Mutex<Vec<VsmMessage>>>;

enum ProbeMsg {
    Channel(VsmActorMsg),
}

impl From<VsmActorMsg> for ProbeMsg {
    fn from(message: VsmActorMsg) -> Self {
        Self::Channel(message)
    }
}

impl TryFrom<ProbeMsg> for VsmActorMsg {
    type Error = ProbeMsg;

    fn try_from(message: ProbeMsg) -> Result<Self, Self::Error> {
        match message {
            ProbeMsg::Channel(inner) => Ok(inner),
        }
    }
}

struct Probe;

#[ractor::async_trait]
impl Actor for Probe {
    type Msg = ProbeMsg;
    type State = SeenMessages;
    type Arguments = SeenMessages;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        seen: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(seen)
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        seen: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        let ProbeMsg::Channel(message) = message;
        let message = match message {
            VsmActorMsg::ChannelMessage(message) | VsmActorMsg::AlgedonicSignal(message) => message,
        };
        seen.lock()
            .expect("probe message store should not be poisoned")
            .push(message);
        Ok(())
    }
}

async fn start_app() -> VsmApplication {
    let app = vsm_rs::start().await.expect("app should start");
    sleep(Duration::from_millis(100)).await;
    app
}

async fn stop_app(app: VsmApplication) {
    app.supervisor.stop(Some("test complete".to_string()));
    let _ = app.join_handle.await;
}

#[tokio::test]
#[serial]
async fn startup_health_reports_root_supervisor_before_shutdown() {
    let app = start_app().await;

    let health = vsm_rs::health().await.expect("health should return");
    let root_supervisor_running = health["root_supervisor"] == true;

    stop_app(app).await;
    assert!(root_supervisor_running);
}

#[tokio::test]
#[serial]
async fn system1_no_suitable_unit_publishes_resource_request() {
    let app = start_app().await;

    let result = vsm_rs::system1::process_transaction(Transaction::new(
        "phase0_missing_capability",
        vec!["phase0_missing".to_string()],
        json!({"request": "characterize"}),
    ))
    .await
    .expect("transaction call should succeed");

    let no_suitable_unit = matches!(result, TransactionResult::NoSuitableUnit);

    sleep(Duration::from_millis(50)).await;
    let history = channels::history(ChannelKind::ResourceBargain)
        .await
        .expect("resource bargain history should return");

    let resource_request_seen = history.iter().any(|message| {
        message.kind == MessageKind::UnitRequest
            && message.payload["transaction_type"] == "phase0_missing_capability"
            && message.payload["required_capabilities"] == json!(["phase0_missing"])
    });

    stop_app(app).await;
    assert!(no_suitable_unit);
    assert!(resource_request_seen);
}

#[tokio::test]
#[serial]
async fn missing_target_falls_back_to_broadcast_bug_to_remove() {
    let app = start_app().await;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (probe, probe_handle) = Actor::spawn(None, Probe, Arc::clone(&seen))
        .await
        .expect("probe should spawn");

    channels::subscribe(
        ChannelKind::Audit,
        "phase0_probe_target_fallback",
        probe.get_derived::<VsmActorMsg>(),
    )
    .await
    .expect("probe should subscribe");

    let message = VsmMessage::audit(
        SystemId::System1,
        SystemId::System3Star,
        MessageKind::AuditResponse,
        json!({"scope": "phase0"}),
    );

    channels::publish(message.clone()).expect("publish should enqueue valid audit message");
    sleep(Duration::from_millis(50)).await;

    let observed = seen
        .lock()
        .expect("probe message store should not be poisoned")
        .iter()
        .any(|seen| seen.id == message.id);
    let _ = channels::unsubscribe(ChannelKind::Audit, "phase0_probe_target_fallback").await;
    probe.stop(Some("test complete".to_string()));
    let _ = probe_handle.await;
    stop_app(app).await;
    assert!(observed);
}

#[tokio::test]
#[serial]
async fn explicit_broadcast_retains_message_that_targeted_validation_rejects() {
    let app = start_app().await;
    let message = VsmMessage::command(
        SystemId::System1,
        SystemId::System2,
        MessageKind::Other("phase0_invalid_broadcast".to_string()),
        json!({"request": "characterize"}),
    );

    let targeted_validation_rejects = message.validate().is_err();

    channels::broadcast(ChannelKind::Command, message.clone())
        .expect("explicit broadcast currently bypasses targeted validation");
    sleep(Duration::from_millis(50)).await;

    let history = channels::history(ChannelKind::Command)
        .await
        .expect("command history should return");
    let retained = history.iter().any(|seen| seen.id == message.id);

    stop_app(app).await;
    assert!(targeted_validation_rejects);
    assert!(retained);
}

#[tokio::test]
#[serial]
async fn systems_2_to_5_service_calls_return_json_responses() {
    let app = start_app().await;

    let system2 = call_service(names::SYSTEM2_COORDINATION, "get_state", json!({}))
        .await
        .expect("system2 state should return");
    let system3 = call_service(names::SYSTEM3_CONTROL, "get_state", json!({}))
        .await
        .expect("system3 state should return");
    let system4 = call_service(
        names::SYSTEM4_INTELLIGENCE,
        "intelligence_report",
        json!({"sources": []}),
    )
    .await
    .expect("system4 report should return");
    let system5 = call_service(names::SYSTEM5_POLICY, "definitely_unknown", json!({}))
        .await
        .expect("system5 unknown operation should return JSON");

    let system2_running = system2["state"]["status"] == "running";
    let system3_running = system3["state"]["status"] == "running";
    let system4_report_shape = system4.get("scan").is_some() && system4.get("insights").is_some();
    let system5_unknown_operation = system5["status"] == "unknown_operation";
    stop_app(app).await;

    assert!(system2_running);
    assert!(system3_running);
    assert!(system4_report_shape);
    assert!(system5_unknown_operation);
}
