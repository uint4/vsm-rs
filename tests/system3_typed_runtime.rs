use std::fmt::{Display, Formatter};

use vsm_rs::async_trait;
use vsm_rs::error::FrameworkError;
use vsm_rs::protocol::system1::{
    Acknowledgement, AuditEvidence, AuditRequest, AuditScope, CapacitySnapshot, CoordinationView,
    ResourceShortageRequest, UnitCommand, UnitDescriptor, WorkRequest, WorkResult,
};
use vsm_rs::protocol::system3::{
    AuditFinding, AuditResponse, AuditSeverity, ControlAckStatus, OperationalDirective,
    OperationalDirectiveKind, ResourceAllocation, ResourceDecision, ResourceRequest,
    System3AuditRequest,
};
use vsm_rs::protocol::{ProtocolMetadata, RuntimeEvent, SubsystemRole};
use vsm_rs::roles::system1::testing::AcceptAllWorkModel;
use vsm_rs::roles::{
    Auditor, BoxOperationalUnit, OperationalControlPolicy, OperationalUnit, OperationalUnitFactory,
    ResourceGovernance, RoleContext, UnitRoleContext, ViableSystem,
};
use vsm_rs::VsmBuilder;

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
struct Capability(&'static str);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct UnitId(&'static str);

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
async fn system3_resource_shortage_produces_allocation_and_acknowledgement() {
    let runtime = runtime_builder()
        .resource_governance(GrantGovernance)
        .start()
        .await
        .expect("runtime should start");

    let cycle = runtime
        .system3()
        .handle_resource_shortage(shortage("missing-capability"))
        .await
        .expect("resource governance should run");

    assert_eq!(cycle.allocations.len(), 1);
    assert_eq!(cycle.allocations[0].decision, ResourceDecision::Grant);
    assert_eq!(cycle.allocation_acknowledgements.len(), 1);
    assert!(cycle.allocations[0].authority.issued_by.is_some());
}

#[tokio::test]
async fn system3_delivers_directive_and_records_failed_acknowledgement() {
    let runtime = runtime_builder()
        .resource_governance(GrantGovernance)
        .operational_control_policy(RejectingDirectivePolicy)
        .start()
        .await
        .expect("runtime should start");

    register(&runtime, "reject").await;
    let cycle = runtime
        .system3()
        .govern_resources(
            vec![ResourceRequest::new([Capability("work")], "test")],
            Vec::new(),
        )
        .await
        .expect("control cycle should run");
    let snapshot = runtime
        .system3()
        .snapshot()
        .await
        .expect("snapshot should return");
    let history = runtime
        .observer_event_history()
        .expect("event history should return");

    assert_eq!(cycle.directives.len(), 1);
    assert_eq!(
        cycle.directive_acknowledgements[0].status,
        ControlAckStatus::Rejected
    );
    assert_eq!(snapshot.directive_acknowledgements.len(), 1);
    assert!(history.iter().any(|event| matches!(
        event,
        RuntimeEvent::System3(system3)
            if matches!(
                &**system3,
                vsm_rs::protocol::System3Event::DirectiveAcknowledgementFailed(_)
            )
    )));
}

#[tokio::test]
async fn system3star_collects_independent_audit_evidence_and_finds_units() {
    let runtime = runtime_builder()
        .auditor(FindingAuditor)
        .start()
        .await
        .expect("runtime should start");

    register(&runtime, "alpha").await;
    register(&runtime, "beta").await;
    let response = runtime
        .system3()
        .audit_system1(System3AuditRequest::new(
            AuditScope::AllUnits,
            "milestone test",
        ))
        .await
        .expect("audit should run");

    assert_eq!(response.findings.len(), 2);
    assert_eq!(
        response
            .metadata
            .source
            .as_ref()
            .map(|address| &address.role),
        Some(&SubsystemRole::System3Star)
    );
}

#[tokio::test]
async fn system3star_rejects_unauthorized_audit() {
    let runtime = runtime_builder()
        .auditor(FindingAuditor)
        .start()
        .await
        .expect("runtime should start");

    let mut request = System3AuditRequest::new(AuditScope::AllUnits, "not approved");
    request.authorization = vsm_rs::protocol::system3::AuditAuthorization::rejected("not approved");
    let error = match runtime.system3().audit_system1(request).await {
        Ok(_) => panic!("unauthorized audit should fail"),
        Err(error) => error,
    };

    assert!(
        matches!(error, FrameworkError::InvalidProtocol { reason } if reason.contains("not authorized"))
    );
}

#[tokio::test]
async fn default_system3_roles_deny_resources_and_noop_audit() {
    let runtime = runtime_builder()
        .start()
        .await
        .expect("runtime should start");

    let cycle = runtime
        .system3()
        .handle_resource_shortage(shortage("default-deny"))
        .await
        .expect("default governance should run");
    let response = runtime
        .system3()
        .audit_with_evidence(
            System3AuditRequest::new(AuditScope::AllUnits, "default audit"),
            Vec::new(),
        )
        .await
        .expect("default audit should run");

    assert_eq!(cycle.allocations[0].decision, ResourceDecision::Deny);
    assert!(response.findings.is_empty());
}

fn runtime_builder() -> VsmBuilder<DomainSystem> {
    VsmBuilder::new()
        .work_model(AcceptAllWorkModel::new([Capability("work")]))
        .operational_unit_factory(TestUnitFactory)
}

async fn register(runtime: &vsm_rs::VsmRuntime<DomainSystem>, unit_id: &'static str) {
    runtime
        .system1()
        .register_descriptor(UnitDescriptor::new(UnitId(unit_id), [Capability("work")]))
        .await
        .expect("unit should register");
}

fn shortage(reason: &'static str) -> ResourceShortageRequest<DomainSystem> {
    ResourceShortageRequest {
        metadata: ProtocolMetadata::new(),
        required_capabilities: vec![Capability("work")],
        work_label: Some("test-work".to_string()),
        reason: reason.to_string(),
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

struct RejectingDirectivePolicy;

#[async_trait]
impl OperationalControlPolicy<DomainSystem> for RejectingDirectivePolicy {
    async fn plan_directives(
        &self,
        _context: &RoleContext<DomainSystem>,
        _allocations: &[ResourceAllocation<DomainSystem>],
        _performance: &[vsm_rs::protocol::system1::PerformanceObservation<DomainSystem>],
    ) -> Result<Vec<OperationalDirective<DomainSystem>>, FrameworkError> {
        Ok(vec![OperationalDirective::new(
            OperationalDirectiveKind::Constrain,
            [UnitId("reject")],
            "test rejection path",
        )])
    }
}

struct FindingAuditor;

#[async_trait]
impl Auditor<DomainSystem> for FindingAuditor {
    async fn audit(
        &self,
        _context: &RoleContext<DomainSystem>,
        request: &System3AuditRequest<DomainSystem>,
        evidence: Vec<AuditEvidence<DomainSystem>>,
    ) -> Result<AuditResponse<DomainSystem>, FrameworkError> {
        let findings = evidence
            .into_iter()
            .map(|evidence| {
                AuditFinding::new(AuditSeverity::Low, "unit", "unit evidence observed")
                    .for_unit(evidence.unit_id)
            })
            .collect();
        Ok(AuditResponse::new(request, findings, Vec::new()))
    }
}

struct TestUnitFactory;

#[async_trait]
impl OperationalUnitFactory<DomainSystem> for TestUnitFactory {
    async fn create_unit(
        &self,
        _context: &RoleContext<DomainSystem>,
        descriptor: &UnitDescriptor<DomainSystem>,
    ) -> Result<BoxOperationalUnit<DomainSystem>, FrameworkError> {
        Ok(Box::new(TestUnit {
            descriptor: descriptor.clone(),
        }))
    }
}

struct TestUnit {
    descriptor: UnitDescriptor<DomainSystem>,
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
        Ok(CapacitySnapshot::new(0, Some(4), 0.1))
    }

    async fn handle_work(
        &mut self,
        _context: &UnitRoleContext<DomainSystem>,
        _request: WorkRequest<DomainSystem>,
    ) -> WorkResult<DomainSystem> {
        Ok(DomainOutcome)
    }

    async fn handle_command(
        &mut self,
        _context: &UnitRoleContext<DomainSystem>,
        command: UnitCommand<DomainSystem>,
    ) -> Result<Acknowledgement, FrameworkError> {
        if self.descriptor.unit_id == UnitId("reject") {
            return Ok(Acknowledgement::rejected(
                command.metadata,
                "unit rejected directive",
            ));
        }

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
            capacity: CapacitySnapshot::new(0, Some(4), 0.1),
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
            capacity: CapacitySnapshot::new(0, Some(4), 0.1),
            snapshot_version: None,
            snapshot: None,
        })
    }
}
