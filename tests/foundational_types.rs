use std::fmt::{Display, Formatter};

use serde_json::json;

use vsm_rs::cancellation::CancellationToken;
use vsm_rs::error::{ApplicationFailure, FrameworkError, WorkError};
use vsm_rs::legacy::system1::{
    descriptor_to_unit_config, resource_shortage_from_message, resource_shortage_to_message,
    transaction_result_to_work_response, transaction_to_work_request, unit_config_to_descriptor,
    work_request_to_transaction, work_response_to_transaction_result, LegacyJsonSystem,
};
use vsm_rs::protocol::system1::{
    CapacitySnapshot, PerformanceObservation, UnitDescriptor, WorkDisposition, WorkRequest,
    WorkResponse,
};
use vsm_rs::protocol::{
    DeliveryMetrics, DeliveryStatus, FrameworkEvent, ProtocolMetadata, RecursionPath,
    RuntimeControlMessage, RuntimeEvent, RuntimeId, RuntimeReport, SnapshotKey, SnapshotRecord,
    SnapshotVersion, SubsystemRole, System1ControlMessage, System1Report, VsmAddress,
};
use vsm_rs::roles::{
    EventSink, NoopEventSink, NoopReportSink, NoopStateStore, ReportSink, StateStore, ViableSystem,
};
use vsm_rs::system1::{Transaction, TransactionResult, UnitConfig};
use vsm_rs::{ChannelKind, MessageKind, SystemId, VsmMessage};

#[derive(Clone)]
struct DomainWork {
    bytes: Vec<u8>,
}

#[derive(Clone)]
struct DomainOutcome {
    code: u16,
}

#[derive(Debug)]
struct DomainError;

impl Display for DomainError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("domain error")
    }
}

impl std::error::Error for DomainError {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct DomainCapability(&'static str);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct DomainUnitId(&'static str);

struct DomainSnapshot;

struct DomainSystem;

impl ViableSystem for DomainSystem {
    type Work = DomainWork;
    type Outcome = DomainOutcome;
    type AppError = DomainError;
    type Capability = DomainCapability;
    type UnitId = DomainUnitId;
    type UnitSnapshot = DomainSnapshot;
}

#[test]
fn viable_system_accepts_non_serde_application_payloads() {
    let request = WorkRequest::<DomainSystem>::new(DomainWork {
        bytes: vec![1, 2, 3],
    });
    let descriptor =
        UnitDescriptor::<DomainSystem>::new(DomainUnitId("unit-a"), [DomainCapability("alpha")]);
    let response = WorkResponse::<DomainSystem> {
        metadata: ProtocolMetadata::new(),
        result: Ok(DomainOutcome { code: 204 }),
    };

    assert_eq!(request.work.bytes.len(), 3);
    assert_eq!(descriptor.unit_id.0, "unit-a");
    assert_eq!(descriptor.capabilities[0].capability.0, "alpha");
    assert_eq!(
        response.result.expect("outcome should be present").code,
        204
    );
}

#[test]
fn capacity_snapshot_clamps_load_and_reports_admission_state() {
    let capacity = CapacitySnapshot::new(2, Some(2), 1.25);

    assert_eq!(capacity.load, 1.0);
    assert!(!capacity.accepting_work);
}

#[test]
fn protocol_metadata_carries_instance_scoped_addresses_and_causation() {
    let runtime_id = RuntimeId::from_string("runtime-a");
    let path = RecursionPath::root().child("division-a");
    let source = VsmAddress::new(runtime_id, path, SubsystemRole::System1).with_entity("unit-a");
    let mut metadata = ProtocolMetadata::new();
    metadata.source = Some(source);

    let child = metadata.child();

    assert_eq!(child.causation_id, Some(metadata.correlation_id));
}

#[test]
fn typed_control_bus_records_do_not_require_json_payloads() {
    let message = RuntimeControlMessage::<DomainSystem>::System1(System1ControlMessage::Work(
        Box::new(WorkRequest::new(DomainWork { bytes: vec![42] })),
    ));
    let mut metrics = DeliveryMetrics::default();
    metrics.record(DeliveryStatus::TargetUnavailable);

    let RuntimeControlMessage::System1(System1ControlMessage::Work(request)) = message else {
        panic!("control message should carry System 1 work");
    };

    assert_eq!(request.work.bytes, vec![42]);
    assert_eq!(metrics.target_unavailable, 1);
}

#[test]
fn cancellation_token_maps_to_framework_work_error() {
    let token = CancellationToken::new();
    assert!(token.check_work::<DomainSystem>().is_ok());

    token.cancel();
    let error = token
        .check_work::<DomainSystem>()
        .expect_err("cancelled token should reject work");

    assert!(matches!(
        error,
        WorkError::Framework(FrameworkError::Cancelled)
    ));
}

#[tokio::test]
async fn noop_ports_are_non_persistent_and_non_blocking() {
    let store = NoopStateStore::<DomainSystem>::new();
    let key = SnapshotKey::new(
        RuntimeId::from_string("runtime-a"),
        RecursionPath::root(),
        SubsystemRole::System1,
    )
    .with_entity("unit-a");
    let record = SnapshotRecord::new(key.clone(), SnapshotVersion::INITIAL, DomainSnapshot);

    store
        .save_unit_snapshot(record)
        .await
        .expect("noop save should succeed");
    let loaded = store
        .load_unit_snapshot(&key)
        .await
        .expect("noop load should succeed");

    let event_sink = NoopEventSink::<DomainSystem>::new();
    event_sink
        .record_event(RuntimeEvent::Framework(Box::new(FrameworkEvent {
            metadata: ProtocolMetadata::new(),
            kind: "test".to_string(),
        })))
        .await
        .expect("noop event sink should succeed");

    let report_sink = NoopReportSink::<DomainSystem>::new();
    report_sink
        .record_report(RuntimeReport::System1(Box::new(
            System1Report::Performance(PerformanceObservation::<DomainSystem> {
                metadata: ProtocolMetadata::new(),
                unit_id: DomainUnitId("unit-a"),
                disposition: WorkDisposition::Completed,
                elapsed: None,
            }),
        )))
        .await
        .expect("noop report sink should succeed");

    assert!(loaded.is_none());
}

#[test]
fn legacy_transaction_round_trips_through_typed_work_request() {
    let transaction = Transaction::new(
        "invoice",
        vec!["billing".to_string()],
        json!({"customer": "customer-42"}),
    );

    let request = transaction_to_work_request(transaction.clone());
    let round_tripped = work_request_to_transaction(request);

    assert_eq!(round_tripped.id, transaction.id);
    assert_eq!(round_tripped.kind, transaction.kind);
    assert_eq!(
        round_tripped.required_capabilities,
        transaction.required_capabilities
    );
    assert_eq!(round_tripped.payload, transaction.payload);
}

#[test]
fn legacy_transaction_result_round_trips_through_typed_work_response() {
    let result = TransactionResult::InvalidTransaction("empty type".to_string());

    let response = transaction_result_to_work_response(result);
    let round_tripped = work_response_to_transaction_result(response);

    assert!(matches!(
        round_tripped,
        TransactionResult::InvalidTransaction(reason) if reason == "empty type"
    ));
}

#[test]
fn legacy_unit_config_round_trips_through_typed_descriptor() {
    let config = UnitConfig::new("unit-a", ["alpha", "beta"]);

    let descriptor = unit_config_to_descriptor(config.clone());
    let round_tripped = descriptor_to_unit_config(descriptor);

    assert_eq!(round_tripped.id, config.id);
    assert_eq!(round_tripped.capabilities, config.capabilities);
}

#[test]
fn legacy_resource_shortage_message_round_trips_through_typed_request() {
    let message = VsmMessage::new(
        SystemId::System1,
        SystemId::System3,
        ChannelKind::ResourceBargain,
        MessageKind::UnitRequest,
        json!({
            "transaction_type": "invoice",
            "required_capabilities": ["billing", "ledger"]
        }),
    );

    let request =
        resource_shortage_from_message(&message).expect("resource-shortage message should convert");
    let round_tripped = resource_shortage_to_message(request);

    assert_eq!(round_tripped.channel, ChannelKind::ResourceBargain);
    assert_eq!(round_tripped.kind, MessageKind::UnitRequest);
    assert_eq!(round_tripped.payload["transaction_type"], "invoice");
    assert_eq!(
        round_tripped.payload["required_capabilities"],
        json!(["billing", "ledger"])
    );
}

#[test]
fn work_error_preserves_application_and_framework_separation() {
    let application: WorkError<DomainError> = ApplicationFailure::Rejected(DomainError).into();
    let framework: WorkError<DomainError> = FrameworkError::Backpressured {
        reason: "unit limit reached".to_string(),
    }
    .into();

    assert!(matches!(application, WorkError::Application(_)));
    assert!(matches!(framework, WorkError::Framework(_)));
}

#[test]
fn legacy_json_system_uses_current_transaction_shape() {
    fn assert_legacy_system<V: ViableSystem<Work = Transaction, Outcome = serde_json::Value>>() {}

    assert_legacy_system::<LegacyJsonSystem>();
}
