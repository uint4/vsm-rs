use std::fmt::{Display, Formatter};

use vsm_rs::error::FrameworkError;
use vsm_rs::protocol::system1::{CapacitySnapshot, UnitDescriptor, WorkRequest};
use vsm_rs::protocol::{RecursionPath, RuntimeId, SubsystemRole};
use vsm_rs::roles::system1::testing::{AcceptAllWorkModel, StaticOperationalUnitFactory};
use vsm_rs::roles::{UnitCandidate, ViableSystem};
use vsm_rs::{ReadinessGate, ReadinessStatus, RuntimeComponentStatus, RuntimeState, VsmBuilder};

#[derive(Clone, Debug)]
struct DomainWork;

#[derive(Clone, Debug)]
struct DomainOutcome;

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

#[tokio::test]
async fn builder_rejects_missing_required_roles() {
    let error = match VsmBuilder::<DomainSystem>::new().start().await {
        Ok(_) => panic!("builder should reject missing required roles"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        FrameworkError::InvalidProtocol { reason }
            if reason.contains("missing required System 1 role: WorkModel")
    ));

    let error = match VsmBuilder::<DomainSystem>::new()
        .work_model(AcceptAllWorkModel::new([DomainCapability("work")]))
        .start()
        .await
    {
        Ok(_) => panic!("builder should reject missing operational-unit factory"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        FrameworkError::InvalidProtocol { reason }
            if reason.contains("missing required System 1 role: OperationalUnitFactory")
    ));
}

#[tokio::test]
async fn builder_starts_ready_runtime_with_default_policies() {
    let runtime = builder_for("runtime-defaults")
        .start()
        .await
        .expect("runtime should start");

    assert!(runtime.is_ready());

    let readiness = runtime.readiness();
    assert_eq!(
        readiness
            .check(ReadinessGate::Infrastructure)
            .expect("infrastructure check should exist")
            .status,
        ReadinessStatus::Ready
    );
    assert_eq!(
        readiness
            .check(ReadinessGate::SubsystemActors)
            .expect("subsystem actor check should exist")
            .status,
        ReadinessStatus::Ready
    );

    let context = runtime.system1().role_context();
    let candidates = vec![
        UnitCandidate::new(
            UnitDescriptor::<DomainSystem>::new(DomainUnitId("loaded"), [DomainCapability("work")]),
            CapacitySnapshot::new(6, Some(10), 0.6),
        ),
        UnitCandidate::new(
            UnitDescriptor::<DomainSystem>::new(DomainUnitId("quiet"), [DomainCapability("work")]),
            CapacitySnapshot::new(1, Some(10), 0.1),
        ),
    ];
    let selected = runtime
        .system1()
        .roles()
        .unit_selection_policy()
        .select_unit(&context, WorkRequest::new(DomainWork), &candidates)
        .await
        .expect("default selector should run");

    assert_eq!(selected, Some(DomainUnitId("quiet")));
}

#[tokio::test]
async fn runtime_handles_are_instance_scoped_and_can_coexist() {
    let runtime_a = builder_for("runtime-a")
        .recursion_path(RecursionPath::root().child("division-a"))
        .start()
        .await
        .expect("runtime-a should start");
    let runtime_b = builder_for("runtime-b")
        .recursion_path(RecursionPath::root().child("division-b"))
        .start()
        .await
        .expect("runtime-b should start");

    assert_ne!(runtime_a.runtime_id(), runtime_b.runtime_id());
    assert_ne!(runtime_a.recursion_path(), runtime_b.recursion_path());

    let directory_a = runtime_a
        .directory_snapshot()
        .expect("runtime-a directory should be readable");
    let directory_b = runtime_b
        .directory_snapshot()
        .expect("runtime-b directory should be readable");

    assert!(!directory_a.is_empty());
    assert!(!directory_b.is_empty());
    assert!(directory_a.components.iter().all(|component| {
        component.address.runtime_id == *runtime_a.runtime_id()
            && component.internal_name.contains("runtime-a")
            && !component.internal_name.starts_with("vsm.")
    }));
    assert!(directory_b.components.iter().all(|component| {
        component.address.runtime_id == *runtime_b.runtime_id()
            && component.internal_name.contains("runtime-b")
            && !component.internal_name.starts_with("vsm.")
    }));
}

#[tokio::test]
async fn role_contexts_use_runtime_identity_and_ports() {
    let runtime = builder_for("runtime-context")
        .recursion_path(RecursionPath::root().child("division").child("team"))
        .start()
        .await
        .expect("runtime should start");

    let context = runtime.role_context(SubsystemRole::System4);

    assert_eq!(context.runtime_id(), runtime.runtime_id());
    assert_eq!(context.recursion_path(), runtime.recursion_path());
    assert_eq!(context.role(), &SubsystemRole::System4);
}

#[tokio::test]
async fn shutdown_returns_acknowledgement_and_is_idempotent() {
    let runtime = builder_for("runtime-shutdown")
        .start()
        .await
        .expect("runtime should start");

    let first = runtime.shutdown().await.expect("shutdown should succeed");
    assert_eq!(first.previous_state, RuntimeState::Ready);
    assert_eq!(first.current_state, RuntimeState::Shutdown);
    assert!(!first.already_shutdown);
    assert!(runtime
        .is_shutdown()
        .expect("shutdown state should be readable"));

    let directory = runtime
        .directory_snapshot()
        .expect("directory should be readable after shutdown");
    assert!(directory
        .components
        .iter()
        .all(|component| component.status == RuntimeComponentStatus::Shutdown));

    let second = runtime
        .shutdown()
        .await
        .expect("second shutdown should succeed");
    assert_eq!(second.previous_state, RuntimeState::Shutdown);
    assert_eq!(second.current_state, RuntimeState::Shutdown);
    assert!(second.already_shutdown);
}

fn builder_for(runtime_id: &'static str) -> VsmBuilder<DomainSystem> {
    let descriptor =
        UnitDescriptor::<DomainSystem>::new(DomainUnitId("unit-a"), [DomainCapability("work")]);
    let capacity = CapacitySnapshot::new(0, Some(4), 0.0);

    VsmBuilder::new()
        .runtime_id(RuntimeId::from_string(runtime_id))
        .work_model(AcceptAllWorkModel::new([DomainCapability("work")]))
        .operational_unit_factory(StaticOperationalUnitFactory::new(
            descriptor,
            capacity,
            DomainOutcome,
        ))
}
