use std::sync::{Arc, Mutex};

use ractor::{Actor, ActorProcessingErr, ActorRef};
use serde_json::json;
use serial_test::serial;
use tokio::time::{sleep, Duration};

use vsm_rs::actor_support::call_service;
use vsm_rs::channels::broker::VsmActorMsg;
use vsm_rs::protocol::DeliveryStatus;
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
async fn system1_no_suitable_unit_records_resource_request_dead_letter() {
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
    let dead_letters = channels::dead_letters(ChannelKind::ResourceBargain)
        .await
        .expect("resource bargain dead letters should return");

    let resource_request_dead_letter_seen = dead_letters.iter().any(|entry| {
        entry.message.kind == MessageKind::UnitRequest
            && entry.message.payload["transaction_type"] == "phase0_missing_capability"
            && entry.message.payload["required_capabilities"] == json!(["phase0_missing"])
    });

    stop_app(app).await;
    assert!(no_suitable_unit);
    assert!(resource_request_dead_letter_seen);
}

#[tokio::test]
#[serial]
async fn targeted_publish_reports_delivered_outcome() {
    let app = start_app().await;
    let message = VsmMessage::command(
        SystemId::System3,
        SystemId::System1,
        MessageKind::Execute,
        json!({"status": "phase0_outcome_probe"}),
    );

    let outcome = channels::publish_with_outcome(message)
        .await
        .expect("publish should return delivery outcome");
    let stats = channels::stats(ChannelKind::Command)
        .await
        .expect("stats should return");

    stop_app(app).await;
    assert_eq!(outcome.status, DeliveryStatus::Delivered);
    assert_eq!(outcome.delivered_to, 1);
    assert!(stats.delivery_metrics.delivered >= 1);
}

#[tokio::test]
#[serial]
async fn missing_target_returns_unavailable_without_broadcast() {
    let app = start_app().await;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (probe, probe_handle) = Actor::spawn(None, Probe, Arc::clone(&seen))
        .await
        .expect("probe should spawn");

    channels::subscribe(
        ChannelKind::Audit,
        "phase0_probe_no_fallback",
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

    let outcome = channels::publish_with_outcome(message.clone())
        .await
        .expect("publish should return outcome");
    sleep(Duration::from_millis(50)).await;

    let observed = seen
        .lock()
        .expect("probe message store should not be poisoned")
        .iter()
        .any(|seen| seen.id == message.id);
    let _ = channels::unsubscribe(ChannelKind::Audit, "phase0_probe_no_fallback").await;
    let dead_letters = channels::dead_letters(ChannelKind::Audit)
        .await
        .expect("dead letters should return");
    let dead_letter_seen = dead_letters
        .iter()
        .any(|entry| entry.message.id == message.id);
    probe.stop(Some("test complete".to_string()));
    let _ = probe_handle.await;
    stop_app(app).await;
    assert_eq!(outcome.status, DeliveryStatus::TargetUnavailable);
    assert!(!observed);
    assert!(dead_letter_seen);
}

#[tokio::test]
#[serial]
async fn explicit_broadcast_rejects_targeted_message() {
    let app = start_app().await;
    let message = VsmMessage::command(
        SystemId::System1,
        SystemId::System2,
        MessageKind::Other("phase0_invalid_broadcast".to_string()),
        json!({"request": "characterize"}),
    );

    let targeted_validation_rejects = message.validate().is_err();

    let outcome = channels::broadcast_with_outcome(ChannelKind::Command, message.clone())
        .await
        .expect("broadcast should return outcome");
    sleep(Duration::from_millis(50)).await;

    let history = channels::history(ChannelKind::Command)
        .await
        .expect("command history should return");
    let retained = history.iter().any(|seen| seen.id == message.id);
    let dead_letters = channels::dead_letters(ChannelKind::Command)
        .await
        .expect("dead letters should return");
    let dead_letter_seen = dead_letters
        .iter()
        .any(|entry| entry.message.id == message.id);

    stop_app(app).await;
    assert!(targeted_validation_rejects);
    assert_eq!(outcome.status, DeliveryStatus::RejectedByProtocol);
    assert!(!retained);
    assert!(dead_letter_seen);
}

#[tokio::test]
#[serial]
async fn system2_to_system5_json_service_dispatch_are_removed() {
    let app = start_app().await;

    let system2 = call_service(names::SYSTEM2_COORDINATION, "get_state", json!({})).await;
    let system3 = call_service(names::SYSTEM3_CONTROL, "get_state", json!({})).await;
    let system4 = call_service(
        "vsm.system4.intelligence",
        "intelligence_report",
        json!({"sources": []}),
    )
    .await;
    let system5 = call_service("vsm.system5.policy", "definitely_unknown", json!({})).await;

    let system2_unavailable = system2.is_err();
    let system3_unavailable = system3.is_err();
    let system4_unavailable = system4.is_err();
    let system5_unavailable = system5.is_err();
    stop_app(app).await;

    assert!(system2_unavailable);
    assert!(system3_unavailable);
    assert!(system4_unavailable);
    assert!(system5_unavailable);
}
