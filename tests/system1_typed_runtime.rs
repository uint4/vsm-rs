use std::fmt::{Display, Formatter};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use vsm_rs::async_trait;
use vsm_rs::error::{ApplicationFailure, FrameworkError, WorkError};
use vsm_rs::protocol::events::{RuntimeEvent, RuntimeReport, System1Event};
use vsm_rs::protocol::system1::{
    Acknowledgement, AuditEvidence, AuditRequest, CapacitySnapshot, CoordinationView, UnitCommand,
    UnitDescriptor, WorkOptions, WorkRequest, WorkResponse, WorkResult,
};
use vsm_rs::protocol::{
    Priority, RecursionPath, RuntimeId, SnapshotKey, SnapshotRecord, SnapshotVersion, SubsystemRole,
};
use vsm_rs::roles::{
    BoxOperationalUnit, EventSink, OperationalUnit, OperationalUnitFactory, ReportSink,
    RoleContext, UnitCandidate, UnitRoleContext, UnitSelectionPolicy, ViableSystem,
    WorkMeasurement, WorkModel,
};
use vsm_rs::{UnitAdmissionLimits, UnitRegistration, UnitSnapshotConfig, VsmBuilder};

#[derive(Clone, Debug)]
struct DomainWork {
    required: Vec<Capability>,
    reject: bool,
    delay: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DomainOutcome {
    unit_id: UnitId,
    restored: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DomainError(&'static str);

impl Display for DomainError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for DomainError {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Capability(&'static str);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct UnitId(&'static str);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    value: String,
}

struct DomainSystem;

impl ViableSystem for DomainSystem {
    type Work = DomainWork;
    type Outcome = DomainOutcome;
    type AppError = DomainError;
    type Capability = Capability;
    type UnitId = UnitId;
    type UnitSnapshot = Snapshot;
}

#[tokio::test]
async fn typed_system1_processes_domain_result() {
    let runtime = runtime_builder()
        .start()
        .await
        .expect("runtime should start");
    let descriptor = descriptor("alpha", "ship");

    runtime
        .system1()
        .register_descriptor(descriptor)
        .await
        .expect("unit should register");
    let outcome = runtime
        .system1()
        .process_work(work("ship"))
        .await
        .expect("work should succeed");

    assert_eq!(outcome.unit_id, UnitId("alpha"));
}

#[tokio::test]
async fn work_model_validation_rejects_before_unit_dispatch() {
    let runtime = runtime_builder()
        .start()
        .await
        .expect("runtime should start");
    let error = runtime
        .system1()
        .process_work(DomainWork {
            reject: true,
            ..work("ship")
        })
        .await
        .expect_err("work model should reject");

    assert!(matches!(
        error,
        WorkError::Application(ApplicationFailure::Rejected(DomainError("rejected")))
    ));
}

#[tokio::test]
async fn custom_selector_changes_routing() {
    let runtime = runtime_builder()
        .unit_selection_policy(PickUnit(UnitId("beta")))
        .start()
        .await
        .expect("runtime should start");

    runtime
        .system1()
        .register_descriptor(descriptor("alpha", "ship"))
        .await
        .expect("alpha should register");
    runtime
        .system1()
        .register_descriptor(descriptor("beta", "ship"))
        .await
        .expect("beta should register");
    let outcome = runtime
        .system1()
        .process_work(work("ship"))
        .await
        .expect("work should succeed");

    assert_eq!(outcome.unit_id, UnitId("beta"));
}

#[tokio::test]
async fn no_suitable_unit_emits_resource_shortage_event() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let runtime = runtime_builder()
        .event_sink(RecordingEventSink {
            events: Arc::clone(&events),
        })
        .start()
        .await
        .expect("runtime should start");

    runtime
        .system1()
        .register_descriptor(descriptor("alpha", "ship"))
        .await
        .expect("unit should register");
    let error = runtime
        .system1()
        .process_work(work("invoice"))
        .await
        .expect_err("missing capability should fail");

    let events = events
        .lock()
        .expect("event recorder should not be poisoned");
    assert!(matches!(
        error,
        WorkError::Framework(FrameworkError::Unavailable { .. })
    ));
    assert!(events.iter().any(|event| event == "resource_shortage"));
}

#[tokio::test]
async fn admission_limit_returns_backpressured() {
    let runtime = runtime_builder()
        .start()
        .await
        .expect("runtime should start");
    let registration = UnitRegistration::new(
        descriptor("alpha", "ship"),
        runtime.system1().roles().operational_unit_factory(),
    )
    .with_admission(UnitAdmissionLimits::max_in_flight(0));

    runtime
        .system1()
        .register_unit(registration)
        .await
        .expect("unit should register");
    let error = runtime
        .system1()
        .process_work(work("ship"))
        .await
        .expect_err("admission limit should reject work");

    assert!(matches!(
        error,
        WorkError::Framework(FrameworkError::Backpressured { .. })
    ));
}

#[tokio::test]
async fn expired_deadline_returns_framework_timeout() {
    let runtime = runtime_builder()
        .start()
        .await
        .expect("runtime should start");
    runtime
        .system1()
        .register_descriptor(descriptor("alpha", "ship"))
        .await
        .expect("unit should register");

    let request = WorkRequest::new(DomainWork {
        delay: Duration::from_millis(50),
        ..work("ship")
    })
    .with_options(WorkOptions {
        deadline: Some(Utc::now() - chrono::Duration::milliseconds(1)),
        priority: Priority::Normal,
    });
    let error = runtime
        .system1()
        .process(request)
        .await
        .expect_err("expired deadline should time out");

    assert!(matches!(
        error,
        WorkError::Framework(FrameworkError::Timeout { .. })
    ));
}

#[tokio::test]
async fn drain_and_unregister_update_unit_lifecycle() {
    let runtime = runtime_builder()
        .start()
        .await
        .expect("runtime should start");
    runtime
        .system1()
        .register_descriptor(descriptor("alpha", "ship"))
        .await
        .expect("unit should register");

    let acknowledgement = runtime
        .system1()
        .drain_unit(&UnitId("alpha"))
        .await
        .expect("drain should succeed");
    let error = runtime
        .system1()
        .process_work(work("ship"))
        .await
        .expect_err("drained unit should reject work");
    runtime
        .system1()
        .unregister_unit(&UnitId("alpha"))
        .await
        .expect("unregister should succeed");

    assert!(acknowledgement.accepted);
    assert!(matches!(
        error,
        WorkError::Framework(FrameworkError::Backpressured { .. })
    ));
    assert!(runtime
        .system1()
        .list_units()
        .expect("list should succeed")
        .is_empty());
}

#[tokio::test]
async fn snapshot_restore_and_save_use_state_store() {
    let key = SnapshotKey::new(
        RuntimeId::from_string("typed-system1-test"),
        RecursionPath::root(),
        SubsystemRole::System1,
    )
    .with_entity("alpha");
    let store = RecordingStateStore {
        load: Arc::new(Mutex::new(Some(SnapshotRecord::new(
            key.clone(),
            SnapshotVersion::INITIAL,
            Snapshot {
                value: "restored".to_string(),
            },
        )))),
        saved: Arc::new(Mutex::new(Vec::new())),
    };
    let saved = Arc::clone(&store.saved);
    let runtime = runtime_builder()
        .runtime_id(RuntimeId::from_string("typed-system1-test"))
        .state_store(store)
        .start()
        .await
        .expect("runtime should start");
    let registration = UnitRegistration::new(
        descriptor("alpha", "ship"),
        runtime.system1().roles().operational_unit_factory(),
    )
    .with_snapshot(UnitSnapshotConfig::keyed(key, SnapshotVersion::INITIAL));

    runtime
        .system1()
        .register_unit(registration)
        .await
        .expect("unit should restore");
    let outcome = runtime
        .system1()
        .process_work(work("ship"))
        .await
        .expect("work should succeed");
    runtime
        .system1()
        .unregister_unit(&UnitId("alpha"))
        .await
        .expect("unregister should save snapshot");

    assert_eq!(outcome.restored, Some("restored".to_string()));
    assert_eq!(
        saved
            .lock()
            .expect("saved snapshots should not be poisoned")
            .last()
            .expect("snapshot should be saved")
            .value,
        "restored"
    );
}

fn runtime_builder() -> VsmBuilder<DomainSystem> {
    VsmBuilder::new()
        .work_model(TestWorkModel)
        .operational_unit_factory(TestUnitFactory::default())
}

fn descriptor(unit_id: &'static str, capability: &'static str) -> UnitDescriptor<DomainSystem> {
    UnitDescriptor::new(UnitId(unit_id), [Capability(capability)])
}

fn work(capability: &'static str) -> DomainWork {
    DomainWork {
        required: vec![Capability(capability)],
        reject: false,
        delay: Duration::from_millis(0),
    }
}

struct TestWorkModel;

#[async_trait]
impl WorkModel<DomainSystem> for TestWorkModel {
    async fn validate_work(
        &self,
        _context: &RoleContext<DomainSystem>,
        request: WorkRequest<DomainSystem>,
    ) -> Result<(), WorkError<DomainError>> {
        if request.work.reject {
            Err(ApplicationFailure::Rejected(DomainError("rejected")).into())
        } else {
            Ok(())
        }
    }

    async fn required_capabilities(
        &self,
        _context: &RoleContext<DomainSystem>,
        request: WorkRequest<DomainSystem>,
    ) -> Result<Vec<Capability>, WorkError<DomainError>> {
        Ok(request.work.required)
    }

    async fn measurements(
        &self,
        _context: &RoleContext<DomainSystem>,
        _request: WorkRequest<DomainSystem>,
        _response: WorkResponse<DomainSystem>,
    ) -> Result<Vec<WorkMeasurement>, WorkError<DomainError>> {
        Ok(Vec::new())
    }
}

#[derive(Default)]
struct TestUnitFactory {
    snapshots: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl OperationalUnitFactory<DomainSystem> for TestUnitFactory {
    async fn create_unit(
        &self,
        _context: &RoleContext<DomainSystem>,
        descriptor: &UnitDescriptor<DomainSystem>,
    ) -> Result<BoxOperationalUnit<DomainSystem>, FrameworkError> {
        Ok(Box::new(TestUnit {
            descriptor: descriptor.clone(),
            restored: None,
            snapshots: Arc::clone(&self.snapshots),
        }))
    }
}

struct TestUnit {
    descriptor: UnitDescriptor<DomainSystem>,
    restored: Option<String>,
    snapshots: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl OperationalUnit<DomainSystem> for TestUnit {
    async fn descriptor(
        &mut self,
        _context: &UnitRoleContext<DomainSystem>,
    ) -> Result<UnitDescriptor<DomainSystem>, FrameworkError> {
        Ok(self.descriptor.clone())
    }

    async fn capacity(
        &mut self,
        _context: &UnitRoleContext<DomainSystem>,
    ) -> Result<CapacitySnapshot, FrameworkError> {
        Ok(CapacitySnapshot::new(0, None, 0.0))
    }

    async fn handle_work(
        &mut self,
        _context: &UnitRoleContext<DomainSystem>,
        request: WorkRequest<DomainSystem>,
    ) -> WorkResult<DomainSystem> {
        tokio::time::sleep(request.work.delay).await;
        Ok(DomainOutcome {
            unit_id: self.descriptor.unit_id.clone(),
            restored: self.restored.clone(),
        })
    }

    async fn handle_command(
        &mut self,
        _context: &UnitRoleContext<DomainSystem>,
        command: UnitCommand<DomainSystem>,
    ) -> Result<Acknowledgement, FrameworkError> {
        Ok(Acknowledgement::accepted(command.metadata))
    }

    async fn coordination_view(
        &mut self,
        context: &UnitRoleContext<DomainSystem>,
    ) -> Result<CoordinationView<DomainSystem>, FrameworkError> {
        Ok(CoordinationView {
            metadata: context.metadata().clone(),
            unit_id: self.descriptor.unit_id.clone(),
            capabilities: self.descriptor.capabilities.clone(),
            capacity: CapacitySnapshot::new(0, None, 0.0),
            snapshot_version: None,
        })
    }

    async fn audit_evidence(
        &mut self,
        context: &UnitRoleContext<DomainSystem>,
        _request: AuditRequest<DomainSystem>,
    ) -> Result<AuditEvidence<DomainSystem>, FrameworkError> {
        Ok(AuditEvidence {
            metadata: context.metadata().clone(),
            unit_id: self.descriptor.unit_id.clone(),
            capabilities: self.descriptor.capabilities.clone(),
            capacity: CapacitySnapshot::new(0, None, 0.0),
            snapshot_version: None,
            snapshot: None,
        })
    }

    async fn snapshot(
        &mut self,
        _context: &UnitRoleContext<DomainSystem>,
    ) -> Result<Snapshot, FrameworkError> {
        let value = self
            .restored
            .clone()
            .unwrap_or_else(|| format!("snapshot-{:?}", self.descriptor.unit_id));
        self.snapshots
            .lock()
            .expect("snapshot recorder should not be poisoned")
            .push(value.clone());
        Ok(Snapshot { value })
    }

    async fn restore(
        &mut self,
        _context: &UnitRoleContext<DomainSystem>,
        snapshot: SnapshotRecord<Snapshot>,
    ) -> Result<(), FrameworkError> {
        self.restored = Some(snapshot.snapshot.value);
        Ok(())
    }
}

struct PickUnit(UnitId);

#[async_trait]
impl UnitSelectionPolicy<DomainSystem> for PickUnit {
    async fn select_unit(
        &self,
        _context: &RoleContext<DomainSystem>,
        _request: WorkRequest<DomainSystem>,
        candidates: &[UnitCandidate<DomainSystem>],
    ) -> Result<Option<UnitId>, FrameworkError> {
        Ok(candidates
            .iter()
            .find(|candidate| candidate.descriptor.unit_id == self.0)
            .map(|candidate| candidate.descriptor.unit_id.clone()))
    }
}

struct RecordingEventSink {
    events: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl EventSink<DomainSystem> for RecordingEventSink {
    async fn record_event(&self, event: RuntimeEvent<DomainSystem>) -> Result<(), FrameworkError> {
        if let RuntimeEvent::System1(event) = event {
            match *event {
                System1Event::ResourceShortage(_) => self
                    .events
                    .lock()
                    .expect("event recorder should not be poisoned")
                    .push("resource_shortage".to_string()),
                System1Event::UnitRegistered(_) => self
                    .events
                    .lock()
                    .expect("event recorder should not be poisoned")
                    .push("unit_registered".to_string()),
                System1Event::UnitUnregistered { .. } => self
                    .events
                    .lock()
                    .expect("event recorder should not be poisoned")
                    .push("unit_unregistered".to_string()),
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ReportSink<DomainSystem> for RecordingEventSink {
    async fn record_report(
        &self,
        _report: RuntimeReport<DomainSystem>,
    ) -> Result<(), FrameworkError> {
        Ok(())
    }
}

struct RecordingStateStore {
    load: Arc<Mutex<Option<SnapshotRecord<Snapshot>>>>,
    saved: Arc<Mutex<Vec<Snapshot>>>,
}

#[async_trait]
impl vsm_rs::roles::StateStore<DomainSystem> for RecordingStateStore {
    async fn load_unit_snapshot(
        &self,
        _key: &SnapshotKey,
    ) -> Result<Option<SnapshotRecord<Snapshot>>, FrameworkError> {
        Ok(self
            .load
            .lock()
            .expect("snapshot load store should not be poisoned")
            .clone())
    }

    async fn save_unit_snapshot(
        &self,
        record: SnapshotRecord<Snapshot>,
    ) -> Result<(), FrameworkError> {
        self.saved
            .lock()
            .expect("snapshot save store should not be poisoned")
            .push(record.snapshot);
        Ok(())
    }
}
