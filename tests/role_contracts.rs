use std::fmt::{Display, Formatter};
use std::sync::Arc;

use vsm_rs::async_trait;
use vsm_rs::error::{ApplicationFailure, FrameworkError, WorkError};
use vsm_rs::protocol::system1::{
    Acknowledgement, AuditEvidence, AuditRequest, AuditScope, CapacitySnapshot, CoordinationView,
    PerformanceObservation, UnitCommand, UnitCommandKind, UnitDescriptor, WorkDisposition,
    WorkOptions, WorkRequest, WorkResponse,
};
use vsm_rs::protocol::{
    FrameworkEvent, Priority, ProtocolMetadata, RecursionPath, RuntimeEvent, RuntimeId,
    SubsystemRole,
};
use vsm_rs::roles::system1::defaults::{
    LowestLoadSelectionPolicy, NoopAlgedonicPolicy, NoopPerformanceModel, NoopVarietyModel,
};
use vsm_rs::roles::system1::testing::{AcceptAllWorkModel, StaticOperationalUnitFactory};
use vsm_rs::roles::{
    AlgedonicKind, AlgedonicPolicy, AlgedonicSeverity, AlgedonicSignal, BoxOperationalUnit,
    OperationalUnit, OperationalUnitFactory, PerformanceAssessment, PerformanceModel, RoleContext,
    System1Roles, UnitCandidate, UnitRoleContext, UnitSelectionPolicy, VarietyAssessment,
    VarietyModel, ViableSystem, WorkMeasurement, WorkModel,
};

#[derive(Clone)]
struct DomainWork {
    kind: Capability,
    amount: u32,
}

#[derive(Clone)]
struct DomainOutcome {
    accepted: bool,
}

#[derive(Debug)]
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

#[derive(Debug)]
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

struct PaymentUnit {
    descriptor: UnitDescriptor<DomainSystem>,
    capacity: CapacitySnapshot,
}

impl PaymentUnit {
    fn new(unit_id: UnitId, load: f64) -> Self {
        Self {
            descriptor: UnitDescriptor::new(unit_id, [Capability("payment")]),
            capacity: CapacitySnapshot::new(0, Some(4), load),
        }
    }
}

#[async_trait]
impl OperationalUnit<DomainSystem> for PaymentUnit {
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
        Ok(self.capacity.clone())
    }

    async fn handle_work(
        &mut self,
        _context: &UnitRoleContext<DomainSystem>,
        request: WorkRequest<DomainSystem>,
    ) -> Result<DomainOutcome, WorkError<DomainError>> {
        if request.work.amount == 0 {
            Err(ApplicationFailure::Rejected(DomainError("amount must be positive")).into())
        } else {
            Ok(DomainOutcome { accepted: true })
        }
    }

    async fn handle_command(
        &mut self,
        _context: &UnitRoleContext<DomainSystem>,
        command: UnitCommand<DomainSystem>,
    ) -> Result<Acknowledgement, FrameworkError> {
        Ok(match command.kind {
            UnitCommandKind::Drain => Acknowledgement::accepted(command.metadata),
            _ => Acknowledgement::rejected(command.metadata, "unsupported command"),
        })
    }

    async fn coordination_view(
        &mut self,
        context: &UnitRoleContext<DomainSystem>,
    ) -> Result<CoordinationView<DomainSystem>, FrameworkError> {
        Ok(CoordinationView {
            metadata: context.metadata().clone(),
            unit_id: self.descriptor.unit_id.clone(),
            capabilities: self.descriptor.capabilities.clone(),
            capacity: self.capacity.clone(),
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
            capacity: self.capacity.clone(),
            snapshot_version: None,
            snapshot: None,
        })
    }
}

struct PaymentFactory;

#[async_trait]
impl OperationalUnitFactory<DomainSystem> for PaymentFactory {
    async fn create_unit(
        &self,
        _context: &RoleContext<DomainSystem>,
        descriptor: &UnitDescriptor<DomainSystem>,
    ) -> Result<BoxOperationalUnit<DomainSystem>, FrameworkError> {
        Ok(Box::new(PaymentUnit {
            descriptor: descriptor.clone(),
            capacity: CapacitySnapshot::new(0, Some(4), 0.2),
        }))
    }
}

struct DomainWorkModel;

#[async_trait]
impl WorkModel<DomainSystem> for DomainWorkModel {
    async fn validate_work(
        &self,
        _context: &RoleContext<DomainSystem>,
        request: WorkRequest<DomainSystem>,
    ) -> Result<(), WorkError<DomainError>> {
        if request.work.amount == 0 {
            Err(ApplicationFailure::Rejected(DomainError("amount must be positive")).into())
        } else {
            Ok(())
        }
    }

    async fn required_capabilities(
        &self,
        _context: &RoleContext<DomainSystem>,
        request: WorkRequest<DomainSystem>,
    ) -> Result<Vec<Capability>, WorkError<DomainError>> {
        Ok(vec![request.work.kind.clone()])
    }

    async fn measurements(
        &self,
        _context: &RoleContext<DomainSystem>,
        request: WorkRequest<DomainSystem>,
        _response: WorkResponse<DomainSystem>,
    ) -> Result<Vec<WorkMeasurement>, WorkError<DomainError>> {
        Ok(vec![WorkMeasurement::new(
            "amount",
            f64::from(request.work.amount),
        )
        .with_unit("units")])
    }
}

struct PreferFirstPolicy;

#[async_trait]
impl UnitSelectionPolicy<DomainSystem> for PreferFirstPolicy {
    async fn select_unit(
        &self,
        _context: &RoleContext<DomainSystem>,
        _request: WorkRequest<DomainSystem>,
        candidates: &[UnitCandidate<DomainSystem>],
    ) -> Result<Option<UnitId>, FrameworkError> {
        Ok(candidates
            .first()
            .map(|candidate| candidate.descriptor.unit_id.clone()))
    }
}

struct DomainPerformanceModel;

#[async_trait]
impl PerformanceModel<DomainSystem> for DomainPerformanceModel {
    async fn assess_performance(
        &self,
        _context: &RoleContext<DomainSystem>,
        observation: &PerformanceObservation<DomainSystem>,
        measurements: &[WorkMeasurement],
    ) -> Result<PerformanceAssessment, FrameworkError> {
        let quality = if observation.disposition == WorkDisposition::Completed {
            1.0
        } else {
            0.0
        };
        Ok(PerformanceAssessment::new(
            measurements.len() as f64,
            1.0,
            1.0,
            quality,
            0.0,
        ))
    }
}

struct DomainVarietyModel;

#[async_trait]
impl VarietyModel<DomainSystem> for DomainVarietyModel {
    async fn assess_variety(
        &self,
        _context: &RoleContext<DomainSystem>,
        request: WorkRequest<DomainSystem>,
        response: Option<WorkResponse<DomainSystem>>,
    ) -> Result<VarietyAssessment, FrameworkError> {
        let output = if response.is_some() { 1.0 } else { 0.0 };
        Ok(VarietyAssessment::new(
            f64::from(request.work.amount),
            output,
        ))
    }
}

struct DomainAlgedonicPolicy;

#[async_trait]
impl AlgedonicPolicy<DomainSystem> for DomainAlgedonicPolicy {
    async fn classify_algedonic(
        &self,
        _context: &RoleContext<DomainSystem>,
        observation: &PerformanceObservation<DomainSystem>,
        assessment: &PerformanceAssessment,
    ) -> Result<Option<AlgedonicSignal<DomainSystem>>, FrameworkError> {
        if assessment.risk > 0.8 {
            Ok(Some(AlgedonicSignal {
                unit_id: observation.unit_id.clone(),
                kind: AlgedonicKind::Pain,
                severity: AlgedonicSeverity::High,
                reason: "risk threshold exceeded".to_string(),
                measurements: Vec::new(),
            }))
        } else {
            Ok(None)
        }
    }
}

struct DomainRoleCatalog;

impl System1Roles<DomainSystem> for DomainRoleCatalog {
    type OperationalUnit = PaymentUnit;
    type OperationalUnitFactory = PaymentFactory;
    type WorkModel = DomainWorkModel;
    type UnitSelectionPolicy = PreferFirstPolicy;
    type PerformanceModel = DomainPerformanceModel;
    type VarietyModel = DomainVarietyModel;
    type AlgedonicPolicy = DomainAlgedonicPolicy;
}

fn role_context() -> RoleContext<DomainSystem> {
    RoleContext::new(
        RuntimeId::from_string("runtime-a"),
        RecursionPath::root(),
        SubsystemRole::System1,
    )
}

#[tokio::test]
async fn downstream_code_can_implement_every_first_wave_role() {
    let context = role_context();
    let unit_context = UnitRoleContext::new(context.clone(), UnitId("payments"));
    let request = WorkRequest::<DomainSystem>::new(DomainWork {
        kind: Capability("payment"),
        amount: 10,
    });
    let descriptor: UnitDescriptor<DomainSystem> =
        UnitDescriptor::new(UnitId("payments"), [Capability("payment")]);

    let work_model = DomainWorkModel;
    work_model
        .validate_work(&context, request.clone())
        .await
        .expect("work should validate");
    let required = work_model
        .required_capabilities(&context, request.clone())
        .await
        .expect("capabilities should be derived");

    let factory = PaymentFactory;
    let mut unit = factory
        .create_unit(&context, &descriptor)
        .await
        .expect("unit should be created");
    let outcome = unit
        .handle_work(&unit_context, request.clone())
        .await
        .expect("unit should complete work");
    let response = WorkResponse::<DomainSystem> {
        metadata: ProtocolMetadata::new(),
        result: Ok(outcome.clone()),
    };
    let measurements = work_model
        .measurements(&context, request.clone(), response)
        .await
        .expect("measurements should be derived");

    let candidates = vec![UnitCandidate::new(
        unit.descriptor(&unit_context)
            .await
            .expect("descriptor should be available"),
        unit.capacity(&unit_context)
            .await
            .expect("capacity should be available"),
    )];
    let selected = PreferFirstPolicy
        .select_unit(&context, request.clone(), &candidates)
        .await
        .expect("selection should succeed");
    let observation = PerformanceObservation::<DomainSystem> {
        metadata: ProtocolMetadata::new(),
        unit_id: selected.clone().expect("unit should be selected"),
        disposition: WorkDisposition::Completed,
        elapsed: None,
    };
    let assessment = DomainPerformanceModel
        .assess_performance(&context, &observation, &measurements)
        .await
        .expect("performance should be assessed");
    let variety_response = WorkResponse::<DomainSystem> {
        metadata: ProtocolMetadata::new(),
        result: Ok(outcome.clone()),
    };
    let variety = DomainVarietyModel
        .assess_variety(&context, request.clone(), Some(variety_response))
        .await
        .expect("variety should be assessed");
    let algedonic = DomainAlgedonicPolicy
        .classify_algedonic(&context, &observation, &assessment)
        .await
        .expect("algedonic policy should run");

    assert_eq!(required, vec![Capability("payment")]);
    assert!(outcome.accepted);
    assert!(variety.ratio > 0.0);
    assert!(algedonic.is_none());
}

#[test]
fn role_traits_are_dyn_compatible() {
    let descriptor: UnitDescriptor<DomainSystem> =
        UnitDescriptor::new(UnitId("payments"), [Capability("payment")]);

    let _unit: Box<dyn OperationalUnit<DomainSystem>> =
        Box::new(PaymentUnit::new(UnitId("payments"), 0.1));
    let _factory: Arc<dyn OperationalUnitFactory<DomainSystem>> = Arc::new(PaymentFactory);
    let _work_model: Arc<dyn WorkModel<DomainSystem>> = Arc::new(DomainWorkModel);
    let _selector: Arc<dyn UnitSelectionPolicy<DomainSystem>> = Arc::new(PreferFirstPolicy);
    let _performance: Arc<dyn PerformanceModel<DomainSystem>> = Arc::new(DomainPerformanceModel);
    let _variety: Arc<dyn VarietyModel<DomainSystem>> = Arc::new(DomainVarietyModel);
    let _algedonic: Arc<dyn AlgedonicPolicy<DomainSystem>> = Arc::new(DomainAlgedonicPolicy);
    let _catalog = DomainRoleCatalog;

    assert_eq!(descriptor.capabilities.len(), 1);
}

#[tokio::test]
async fn default_policies_are_opt_in_and_non_normative() {
    let context = role_context();
    let request = WorkRequest::<DomainSystem>::new(DomainWork {
        kind: Capability("payment"),
        amount: 10,
    });
    let candidates = vec![
        UnitCandidate::new(
            UnitDescriptor::new(UnitId("busy"), [Capability("payment")]),
            CapacitySnapshot::new(1, Some(4), 0.9),
        ),
        UnitCandidate::new(
            UnitDescriptor::new(UnitId("quiet"), [Capability("payment")]),
            CapacitySnapshot::new(1, Some(4), 0.1),
        ),
    ];

    let selected = LowestLoadSelectionPolicy
        .select_unit(&context, request.clone(), &candidates)
        .await
        .expect("default selector should run");
    let performance = NoopPerformanceModel
        .assess_performance(
            &context,
            &PerformanceObservation::<DomainSystem> {
                metadata: ProtocolMetadata::new(),
                unit_id: UnitId("quiet"),
                disposition: WorkDisposition::Completed,
                elapsed: None,
            },
            &[],
        )
        .await
        .expect("noop performance should run");
    let variety = NoopVarietyModel
        .assess_variety(&context, request.clone(), None)
        .await
        .expect("noop variety should run");
    let algedonic = NoopAlgedonicPolicy
        .classify_algedonic(
            &context,
            &PerformanceObservation::<DomainSystem> {
                metadata: ProtocolMetadata::new(),
                unit_id: UnitId("quiet"),
                disposition: WorkDisposition::Completed,
                elapsed: None,
            },
            &performance,
        )
        .await
        .expect("noop algedonic should run");

    assert_eq!(selected, Some(UnitId("quiet")));
    assert_eq!(performance, PerformanceAssessment::no_signal());
    assert_eq!(variety, VarietyAssessment::no_signal());
    assert!(algedonic.is_none());
}

#[tokio::test]
async fn testing_helpers_create_fresh_static_units() {
    let context = role_context();
    let descriptor = UnitDescriptor::new(UnitId("static"), [Capability("payment")]);
    let request = WorkRequest::<DomainSystem>::new(DomainWork {
        kind: Capability("payment"),
        amount: 3,
    });
    let work_model = AcceptAllWorkModel::<DomainSystem>::new([Capability("payment")])
        .with_measurements(vec![WorkMeasurement::new("accepted", 1.0)]);

    let factory = StaticOperationalUnitFactory::<DomainSystem>::new(
        descriptor.clone(),
        CapacitySnapshot::new(0, Some(1), 0.0),
        DomainOutcome { accepted: true },
    );
    let mut unit = factory
        .create_unit(&context, &descriptor)
        .await
        .expect("static unit should be created");
    let outcome = unit
        .handle_work(
            &UnitRoleContext::new(context.clone(), UnitId("static")),
            request.clone(),
        )
        .await
        .expect("static unit should complete work");
    let response = WorkResponse::<DomainSystem> {
        metadata: ProtocolMetadata::new(),
        result: Ok(outcome),
    };
    let measurements = work_model
        .measurements(&context, request, response)
        .await
        .expect("fixed measurements should be returned");

    assert_eq!(measurements[0].name, "accepted");
}

#[tokio::test]
async fn role_context_exposes_allowed_runtime_facilities() {
    let mut metadata = ProtocolMetadata::new();
    metadata.priority = Priority::High;
    let token = vsm_rs::cancellation::CancellationToken::new();
    let context = role_context()
        .with_metadata(metadata)
        .with_cancellation(token.clone());

    context
        .emit_event(RuntimeEvent::Framework(Box::new(FrameworkEvent {
            metadata: ProtocolMetadata::new(),
            kind: "contract-test".to_string(),
        })))
        .await
        .expect("noop event sink should accept events");

    token.cancel();

    assert_eq!(context.runtime_id().as_str(), "runtime-a");
    assert!(context.recursion_path().is_root());
    assert_eq!(context.metadata().priority, Priority::High);
    assert!(context.cancellation().is_cancelled());
}

#[tokio::test]
async fn operational_unit_default_snapshot_methods_fail_explicitly() {
    let context = role_context();
    let unit_context = UnitRoleContext::new(context, UnitId("payments"));
    let mut unit = PaymentUnit::new(UnitId("payments"), 0.1);

    let error = unit
        .snapshot(&unit_context)
        .await
        .expect_err("unsupported snapshot should fail explicitly");

    assert!(matches!(error, FrameworkError::InvalidProtocol { .. }));
}

#[test]
fn unit_candidate_checks_static_capability_eligibility() {
    let candidate: UnitCandidate<DomainSystem> = UnitCandidate::new(
        UnitDescriptor::new(
            UnitId("payments"),
            [Capability("payment"), Capability("card")],
        ),
        CapacitySnapshot::new(0, Some(2), 0.0),
    );

    assert!(candidate.advertises_all(&[Capability("payment"), Capability("card")]));
    assert!(!candidate.advertises_all(&[Capability("settlement")]));
}

#[tokio::test]
async fn operational_unit_accepts_framework_commands_without_actor_types() {
    let context = role_context();
    let unit_context = UnitRoleContext::new(context, UnitId("payments"));
    let mut unit = PaymentUnit::new(UnitId("payments"), 0.1);
    let command = UnitCommand::<DomainSystem> {
        metadata: ProtocolMetadata::new(),
        unit_id: UnitId("payments"),
        kind: UnitCommandKind::Drain,
    };

    let acknowledgement = unit
        .handle_command(&unit_context, command)
        .await
        .expect("drain command should be acknowledged");

    assert!(acknowledgement.accepted);
}

#[tokio::test]
async fn operational_unit_exposes_coordination_and_audit_views() {
    let context = role_context();
    let unit_context = UnitRoleContext::new(context, UnitId("payments"));
    let mut unit = PaymentUnit::new(UnitId("payments"), 0.1);
    let audit_request = AuditRequest::<DomainSystem> {
        metadata: ProtocolMetadata::new(),
        scope: AuditScope::AllUnits,
    };

    let coordination = unit
        .coordination_view(&unit_context)
        .await
        .expect("coordination view should be available");
    let audit = unit
        .audit_evidence(&unit_context, audit_request)
        .await
        .expect("audit evidence should be available");

    assert_eq!(coordination.unit_id, UnitId("payments"));
    assert_eq!(audit.unit_id, UnitId("payments"));
}

#[test]
fn work_options_remain_plain_framework_configuration() {
    let options = WorkOptions {
        deadline: None,
        priority: Priority::Critical,
    };

    assert_eq!(options.priority, Priority::Critical);
}
