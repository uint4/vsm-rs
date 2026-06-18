use std::fmt::{Display, Formatter};
use std::sync::{Arc, Mutex};

use vsm_rs::async_trait;
use vsm_rs::error::FrameworkError;
use vsm_rs::protocol::system1::{
    Acknowledgement, AuditEvidence, AuditRequest, CapacitySnapshot, CoordinationView, UnitCommand,
    UnitDescriptor, WorkRequest, WorkResult,
};
use vsm_rs::protocol::system2::{
    CoordinationAcknowledgement, CoordinationConflict, CoordinationConflictKind,
    CoordinationIntervention, CoordinationInterventionKind, CoordinationSeverity,
    CoordinationViewRecord,
};
use vsm_rs::roles::system1::testing::AcceptAllWorkModel;
use vsm_rs::roles::{
    BoxOperationalUnit, CoordinationPolicy, OperationalUnit, OperationalUnitFactory, RoleContext,
    UnitRoleContext, ViableSystem,
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
async fn system2_detects_conflict_delivers_intervention_and_records_acknowledgements() {
    let acknowledged = Arc::new(Mutex::new(Vec::new()));
    let runtime = runtime_builder(Arc::clone(&acknowledged))
        .coordination_policy(PairConflictPolicy)
        .start()
        .await
        .expect("runtime should start");

    register(&runtime, "alpha").await;
    register(&runtime, "beta").await;
    let cycle = runtime
        .system2()
        .coordinate_system1()
        .await
        .expect("coordination should run");
    let snapshot = runtime
        .system2()
        .snapshot()
        .await
        .expect("snapshot should return");

    assert_eq!(cycle.conflicts.len(), 1);
    assert_eq!(cycle.interventions.len(), 1);
    assert_eq!(cycle.acknowledgements.len(), 2);
    assert!(cycle.escalations.is_empty());
    assert_eq!(snapshot.views.len(), 2);
    assert!(snapshot.pending_interventions.is_empty());
    assert_eq!(
        acknowledged
            .lock()
            .expect("acknowledgement recorder should not be poisoned")
            .len(),
        2
    );
}

#[tokio::test]
async fn system2_escalates_rejected_intervention_to_system3_report_path() {
    let runtime = runtime_builder(Arc::new(Mutex::new(Vec::new())))
        .coordination_policy(PairConflictPolicy)
        .start()
        .await
        .expect("runtime should start");

    register(&runtime, "alpha").await;
    register(&runtime, "reject").await;
    let cycle = runtime
        .system2()
        .coordinate_system1()
        .await
        .expect("coordination should run");
    let snapshot = runtime
        .system2()
        .snapshot()
        .await
        .expect("snapshot should return");

    assert_eq!(cycle.escalations.len(), 1);
    assert_eq!(snapshot.escalations.len(), 1);
    assert!(cycle
        .acknowledgements
        .iter()
        .any(|ack| ack.unit_id == UnitId("reject")));
}

#[tokio::test]
async fn system2_view_versions_advance_on_repeated_observation() {
    let runtime = runtime_builder(Arc::new(Mutex::new(Vec::new())))
        .coordination_policy(PairConflictPolicy)
        .start()
        .await
        .expect("runtime should start");

    register(&runtime, "alpha").await;
    runtime
        .system2()
        .coordinate_system1()
        .await
        .expect("first coordination should run");
    runtime
        .system2()
        .coordinate_system1()
        .await
        .expect("second coordination should run");
    let snapshot = runtime
        .system2()
        .snapshot()
        .await
        .expect("snapshot should return");

    assert_eq!(snapshot.views[0].version.get(), 2);
}

#[tokio::test]
async fn default_system2_policy_is_noop_and_replaceable() {
    let runtime = runtime_builder(Arc::new(Mutex::new(Vec::new())))
        .start()
        .await
        .expect("runtime should start");

    register(&runtime, "alpha").await;
    register(&runtime, "beta").await;
    let cycle = runtime
        .system2()
        .coordinate_system1()
        .await
        .expect("coordination should run");

    assert!(cycle.conflicts.is_empty());
    assert!(cycle.interventions.is_empty());
}

fn runtime_builder(acknowledged: Arc<Mutex<Vec<UnitId>>>) -> VsmBuilder<DomainSystem> {
    VsmBuilder::new()
        .work_model(AcceptAllWorkModel::new([Capability("work")]))
        .operational_unit_factory(TestUnitFactory { acknowledged })
}

async fn register(runtime: &vsm_rs::VsmRuntime<DomainSystem>, unit_id: &'static str) {
    runtime
        .system1()
        .register_descriptor(UnitDescriptor::new(UnitId(unit_id), [Capability("work")]))
        .await
        .expect("unit should register");
}

struct PairConflictPolicy;

#[async_trait]
impl CoordinationPolicy<DomainSystem> for PairConflictPolicy {
    async fn detect_conflicts(
        &self,
        _context: &RoleContext<DomainSystem>,
        views: &[CoordinationViewRecord<DomainSystem>],
    ) -> Result<Vec<CoordinationConflict<DomainSystem>>, FrameworkError> {
        if views.len() < 2 {
            return Ok(Vec::new());
        }

        Ok(vec![CoordinationConflict::new(
            CoordinationConflictKind::CapacityPressure,
            views.iter().map(|record| record.view.unit_id.clone()),
            CoordinationSeverity::Medium,
            "test units require coordination",
        )])
    }

    async fn plan_interventions(
        &self,
        _context: &RoleContext<DomainSystem>,
        conflicts: &[CoordinationConflict<DomainSystem>],
        _views: &[CoordinationViewRecord<DomainSystem>],
    ) -> Result<Vec<CoordinationIntervention<DomainSystem>>, FrameworkError> {
        Ok(conflicts
            .iter()
            .map(|conflict| {
                CoordinationIntervention::new(
                    CoordinationInterventionKind::Constraint,
                    conflict.affected_units.clone(),
                    "smooth shared capacity",
                )
                .for_conflict(conflict.conflict_id.clone())
            })
            .collect())
    }
}

struct TestUnitFactory {
    acknowledged: Arc<Mutex<Vec<UnitId>>>,
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
            acknowledged: Arc::clone(&self.acknowledged),
        }))
    }
}

struct TestUnit {
    descriptor: UnitDescriptor<DomainSystem>,
    acknowledged: Arc<Mutex<Vec<UnitId>>>,
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

    async fn handle_coordination_intervention(
        &mut self,
        _context: &UnitRoleContext<DomainSystem>,
        intervention: CoordinationIntervention<DomainSystem>,
    ) -> Result<CoordinationAcknowledgement<DomainSystem>, FrameworkError> {
        if self.descriptor.unit_id == UnitId("reject") {
            return Ok(CoordinationAcknowledgement::rejected(
                &intervention,
                self.descriptor.unit_id.clone(),
                "unit rejected test intervention",
            ));
        }

        self.acknowledged
            .lock()
            .expect("acknowledgement recorder should not be poisoned")
            .push(self.descriptor.unit_id.clone());
        Ok(CoordinationAcknowledgement::accepted(
            &intervention,
            self.descriptor.unit_id.clone(),
        ))
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
