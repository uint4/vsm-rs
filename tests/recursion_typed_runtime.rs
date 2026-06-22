use std::fmt::{Display, Formatter};
use std::sync::Arc;

use vsm_rs::async_trait;
use vsm_rs::error::FrameworkError;
use vsm_rs::protocol::algedonic::{AlgedonicSeverity, AlgedonicSignalKind, AlgedonicSignalRecord};
use vsm_rs::protocol::system1::{CapacitySnapshot, ResourceShortageRequest, UnitDescriptor};
use vsm_rs::protocol::system3::{
    ControlAckStatus, OperationalDirective, OperationalDirectiveKind, ResourceAllocation,
    ResourceDecision, ResourceRequest,
};
use vsm_rs::protocol::{ProtocolMetadata, RecursionPath, RuntimeId};
use vsm_rs::roles::system1::testing::{AcceptAllWorkModel, StaticOperationalUnitFactory};
use vsm_rs::roles::{ResourceGovernance, RoleContext, ViableSystem};
use vsm_rs::{
    ChildRuntimeDescriptor, ChildRuntimeFactory, ChildRuntimeRegistration, VsmBuilder, VsmRuntime,
};

#[derive(Clone, Debug)]
struct DomainWork;

#[derive(Clone, Debug, PartialEq, Eq)]
struct DomainOutcome {
    handled_by: &'static str,
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
struct Capability(&'static str);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct UnitId(&'static str);

#[derive(Clone, Debug)]
struct DomainSnapshot;

struct DomainSystem;

impl ViableSystem for DomainSystem {
    type Work = DomainWork;
    type Outcome = DomainOutcome;
    type AppError = DomainError;
    type Capability = Capability;
    type UnitId = UnitId;
    type UnitSnapshot = DomainSnapshot;
}

#[tokio::test]
async fn child_runtime_is_registered_as_system1_bridge_unit() {
    let parent = parent_builder("parent-delegate")
        .start()
        .await
        .expect("parent runtime should start");

    register_child(&parent, "child-a", "child-runtime-delegate")
        .await
        .expect("child should register");
    let outcome = parent
        .system1()
        .process_work(DomainWork)
        .await
        .expect("parent work should delegate to child");
    let children = parent.recursion().list_children().expect("children");

    assert_eq!(outcome.handled_by, "child-unit");
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].descriptor.child_id, "child-a");
    assert_eq!(
        parent.system1().list_units().expect("parent units").len(),
        1
    );

    parent.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
async fn child_resource_shortage_escalates_to_parent_system3() {
    let parent = parent_builder("parent-resource")
        .resource_governance(GrantGovernance)
        .start()
        .await
        .expect("parent runtime should start");

    register_child(&parent, "child-a", "child-runtime-resource")
        .await
        .expect("child should register");
    let cycle = parent
        .recursion()
        .escalate_resource_shortage("child-a", shortage("child capacity exhausted"))
        .await
        .expect("resource escalation should run");
    let snapshot = parent.recursion().snapshot().expect("recursion snapshot");

    assert_eq!(cycle.allocations.len(), 1);
    assert_eq!(cycle.allocations[0].decision, ResourceDecision::Grant);
    assert_eq!(snapshot.resource_escalations.len(), 1);
    assert_eq!(snapshot.resource_escalations[0].child_id, "child-a");

    parent.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
async fn child_algedonic_signal_escalates_to_parent_lifecycle() {
    let parent = parent_builder("parent-algedonic")
        .start()
        .await
        .expect("parent runtime should start");

    register_child(&parent, "child-a", "child-runtime-algedonic")
        .await
        .expect("child should register");
    let signal = AlgedonicSignalRecord::<DomainSystem>::new(
        AlgedonicSignalKind::Pain,
        AlgedonicSeverity::High,
        "child operational stress",
    )
    .from_source("child-a");
    let cycle = parent
        .recursion()
        .escalate_algedonic_signal("child-a", signal)
        .await
        .expect("algedonic escalation should run");
    let snapshot = parent.recursion().snapshot().expect("recursion snapshot");

    assert_eq!(cycle.signal.reason, "child operational stress");
    assert!(cycle.crisis_response.is_some());
    assert_eq!(snapshot.algedonic_escalations.len(), 1);

    parent.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
async fn parent_policy_directive_is_transduced_to_child_runtime() {
    let parent = parent_builder("parent-policy")
        .start()
        .await
        .expect("parent runtime should start");

    register_child(&parent, "child-a", "child-runtime-policy")
        .await
        .expect("child should register");
    let directive = OperationalDirective::new(
        OperationalDirectiveKind::Drain,
        [UnitId("child-unit")],
        "drain child unit",
    );
    let acknowledgements = parent
        .recursion()
        .transduce_policy_directive("child-a", directive)
        .await
        .expect("policy directive should transduce");
    let snapshot = parent.recursion().snapshot().expect("recursion snapshot");

    assert_eq!(acknowledgements.len(), 1);
    assert_eq!(acknowledgements[0].unit_id, UnitId("child-unit"));
    assert_eq!(acknowledgements[0].status, ControlAckStatus::Accepted);
    assert_eq!(snapshot.policy_directives.len(), 1);

    parent.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
async fn child_runtime_directories_are_instance_scoped() {
    let parent = parent_builder("parent-directory")
        .start()
        .await
        .expect("parent runtime should start");

    register_child(&parent, "child-a", "child-runtime-a")
        .await
        .expect("child a should register");
    register_child(&parent, "child-b", "child-runtime-b")
        .await
        .expect("child b should register");
    let a = parent
        .recursion()
        .child_directory_snapshot("child-a")
        .expect("child a directory");
    let b = parent
        .recursion()
        .child_directory_snapshot("child-b")
        .expect("child b directory");

    assert!(!a.is_empty());
    assert!(!b.is_empty());
    assert!(a.components.iter().all(|component| {
        component.internal_name.contains("child-runtime-a")
            && !component.internal_name.contains("child-runtime-b")
    }));
    assert!(b.components.iter().all(|component| {
        component.internal_name.contains("child-runtime-b")
            && !component.internal_name.contains("child-runtime-a")
    }));

    parent.shutdown().await.expect("shutdown should succeed");
}

fn parent_builder(runtime_id: &'static str) -> VsmBuilder<DomainSystem> {
    VsmBuilder::new()
        .runtime_id(RuntimeId::from_string(runtime_id))
        .work_model(AcceptAllWorkModel::new([Capability("work")]))
        .operational_unit_factory(StaticOperationalUnitFactory::new(
            UnitDescriptor::new(UnitId("parent-default"), [Capability("work")]),
            CapacitySnapshot::new(0, Some(1), 0.0),
            DomainOutcome {
                handled_by: "parent-default",
            },
        ))
}

async fn register_child(
    parent: &VsmRuntime<DomainSystem>,
    child_id: &'static str,
    child_runtime_id: &'static str,
) -> Result<(), FrameworkError> {
    let descriptor = ChildRuntimeDescriptor::new(
        child_id,
        RuntimeId::from_string(child_runtime_id),
        RecursionPath::root().child(child_id),
        UnitDescriptor::new(UnitId(child_id), [Capability("work")]),
        CapacitySnapshot::new(0, Some(4), 0.1),
    );
    let registration = ChildRuntimeRegistration::new(
        descriptor,
        Arc::new(BuildingChildFactory { child_runtime_id }),
    );

    parent
        .recursion()
        .register_child_runtime(registration)
        .await
        .map(|_| ())
}

fn child_builder(runtime_id: &'static str) -> VsmBuilder<DomainSystem> {
    VsmBuilder::new()
        .runtime_id(RuntimeId::from_string(runtime_id))
        .recursion_path(RecursionPath::root().child(runtime_id))
        .work_model(AcceptAllWorkModel::new([Capability("work")]))
        .operational_unit_factory(StaticOperationalUnitFactory::new(
            UnitDescriptor::new(UnitId("child-unit"), [Capability("work")]),
            CapacitySnapshot::new(0, Some(2), 0.0),
            DomainOutcome {
                handled_by: "child-unit",
            },
        ))
}

fn shortage(reason: &'static str) -> ResourceShortageRequest<DomainSystem> {
    ResourceShortageRequest {
        metadata: ProtocolMetadata::new(),
        required_capabilities: vec![Capability("work")],
        work_label: Some("child-work".to_string()),
        reason: reason.to_string(),
    }
}

struct BuildingChildFactory {
    child_runtime_id: &'static str,
}

#[async_trait]
impl ChildRuntimeFactory<DomainSystem> for BuildingChildFactory {
    async fn start_child_runtime(
        &self,
        _context: &RoleContext<DomainSystem>,
        descriptor: &ChildRuntimeDescriptor<DomainSystem>,
    ) -> Result<VsmRuntime<DomainSystem>, FrameworkError> {
        let runtime = child_builder(self.child_runtime_id)
            .recursion_path(descriptor.recursion_path.clone())
            .start()
            .await?;
        runtime
            .system1()
            .register_descriptor(UnitDescriptor::new(
                UnitId("child-unit"),
                [Capability("work")],
            ))
            .await?;
        Ok(runtime)
    }
}

struct GrantGovernance;

#[async_trait]
impl ResourceGovernance<DomainSystem> for GrantGovernance {
    async fn allocate_resources(
        &self,
        _context: &RoleContext<DomainSystem>,
        requests: &[ResourceRequest<DomainSystem>],
        _performance: &[vsm_rs::protocol::system1::PerformanceObservation<DomainSystem>],
    ) -> Result<Vec<ResourceAllocation<DomainSystem>>, FrameworkError> {
        Ok(requests
            .iter()
            .map(|request| ResourceAllocation::new(request, ResourceDecision::Grant))
            .collect())
    }
}
