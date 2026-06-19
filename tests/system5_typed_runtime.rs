use std::fmt::{Display, Formatter};
use std::time::Duration;

use vsm_rs::async_trait;
use vsm_rs::error::FrameworkError;
use vsm_rs::protocol::system1::{CapacitySnapshot, UnitDescriptor};
use vsm_rs::protocol::system3::{
    OperationalDirective, ResourceAllocation, ResourceDecision, ResourceRequest,
};
use vsm_rs::protocol::system4::{
    AdaptationProposal, EnvironmentSourceDescriptor, EnvironmentalMeasurement,
    EnvironmentalObservation, Forecast, ForecastPoint, IntelligenceAssessment, InterpretedSignal,
    Scenario, SignalKind,
};
use vsm_rs::protocol::system5::{
    CrisisResponse, CrisisSeverity, CrisisSignal, DecisionAlternative, DecisionRecord,
    DecisionRequest, DecisionStatus, IdentityRecord, PolicyAckStatus, PolicyDirective,
    PolicyDirectiveAcknowledgement, PolicyDirectiveKind, PolicyEscalation, ValueSet,
    ValueStatement, ValuesEvaluation,
};
use vsm_rs::protocol::{RuntimeEvent, RuntimeId, SubsystemRole, VsmAddress};
use vsm_rs::roles::system1::testing::{AcceptAllWorkModel, StaticOperationalUnitFactory};
use vsm_rs::roles::{
    CrisisPolicy, DecisionPolicy, EnvironmentalSource, EnvironmentalSourceFactory, Forecaster,
    IdentityProvider, IntelligenceModel, OperationalControlPolicy, ResourceGovernance, RoleContext,
    SignalInterpreter, ValuesEvaluator, ValuesProvider, ViableSystem,
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
async fn system5_records_decision_audit_trail_and_directive_acknowledgement() {
    let runtime = runtime_builder("system5-decision")
        .identity_provider(TestIdentityProvider)
        .values_provider(TestValuesProvider)
        .values_evaluator(PassingValuesEvaluator)
        .decision_policy(SelectingDecisionPolicy)
        .start()
        .await
        .expect("runtime should start");

    let directive = PolicyDirective::<DomainSystem>::new(
        PolicyDirectiveKind::Strategic,
        "increase operating capacity",
    )
    .with_target_units([UnitId("unit-a")]);
    let request = DecisionRequest::new("capacity expansion")
        .with_alternative(DecisionAlternative::new("expand now").with_directive(directive));
    let cycle = runtime
        .system5()
        .decide(request)
        .await
        .expect("decision should run");

    assert_eq!(cycle.decision.status, DecisionStatus::Approved);
    assert_eq!(cycle.identity.label, "example organization");
    assert_eq!(cycle.values.values.len(), 1);
    assert_eq!(cycle.decision.directives.len(), 1);
    assert!(cycle.decision.evaluation.is_some());

    let acknowledgement = PolicyDirectiveAcknowledgement::accepted(&cycle.decision.directives[0]);
    let snapshot = runtime
        .system5()
        .acknowledge_directives(vec![acknowledgement])
        .await
        .expect("acknowledgement should record");

    assert_eq!(
        snapshot.directive_acknowledgements[0].status,
        PolicyAckStatus::Accepted
    );
    assert!(runtime
        .observer_event_history()
        .expect("event history should return")
        .iter()
        .any(|event| matches!(
            event,
            RuntimeEvent::System5(system5)
                if matches!(
                    &**system5,
                    vsm_rs::protocol::System5Event::DirectiveAcknowledged { success: true, .. }
                )
        )));

    runtime.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
async fn system5_decision_receives_system3_and_system4_context() {
    let runtime = runtime_builder("system5-context")
        .resource_governance(GrantGovernance)
        .operational_control_policy(NoopDirectivePolicy)
        .environmental_source_factory(TestSourceFactory)
        .signal_interpreter(TestInterpreter)
        .intelligence_model(TestIntelligenceModel)
        .forecaster(TestForecaster)
        .decision_policy(ContextAwareDecisionPolicy)
        .start()
        .await
        .expect("runtime should start");

    runtime
        .system3()
        .govern_resources(
            vec![ResourceRequest::new([Capability("work")], "test context")],
            Vec::new(),
        )
        .await
        .expect("System 3 cycle should run");
    runtime
        .system4()
        .register_source(EnvironmentSourceDescriptor::new("market"))
        .await
        .expect("source should register");
    runtime
        .system4()
        .run_intelligence_cycle()
        .await
        .expect("System 4 cycle should run");

    let cycle = runtime
        .system5()
        .decide(
            DecisionRequest::new("balance operations and future")
                .with_alternative(DecisionAlternative::new("rebalance portfolio")),
        )
        .await
        .expect("decision should run");

    assert!(!cycle.request.operational_summaries.is_empty());
    assert_eq!(cycle.request.adaptation_proposals.len(), 1);
    assert!(cycle
        .decision
        .rationale
        .contains("operational summaries and 1 future proposals"));
    assert!(cycle
        .decision
        .evidence
        .iter()
        .any(|evidence| evidence.summary.contains("System 3 summary")));
    assert!(cycle
        .decision
        .evidence
        .iter()
        .any(|evidence| evidence.summary.contains("System 4 proposal")));

    runtime.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
async fn default_system5_roles_do_not_impose_policy_meaning() {
    let runtime = runtime_builder("system5-defaults")
        .start()
        .await
        .expect("runtime should start");

    let cycle = runtime
        .system5()
        .decide(DecisionRequest::new("unconfigured decision"))
        .await
        .expect("default decision should run");

    assert_eq!(cycle.decision.status, DecisionStatus::Deferred);
    assert_eq!(cycle.identity.label, "unconfigured identity");
    assert!(cycle.values.values.is_empty());

    runtime.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
async fn algedonic_crisis_signal_can_escalate_to_parent_recursion() {
    let runtime = runtime_builder("system5-crisis")
        .crisis_policy(EscalatingCrisisPolicy)
        .start()
        .await
        .expect("runtime should start");

    let response = runtime
        .system5()
        .handle_algedonic_signal(CrisisSignal::new(
            CrisisSeverity::Critical,
            "regional outage",
        ))
        .await
        .expect("crisis should run");
    let snapshot = runtime.system5().snapshot().await.expect("snapshot");

    assert_eq!(response.escalations.len(), 1);
    assert!(response.escalations[0].requires_parent);
    assert_eq!(snapshot.crises.len(), 1);
    assert!(runtime
        .observer_event_history()
        .expect("event history should return")
        .iter()
        .any(|event| matches!(
            event,
            RuntimeEvent::System5(system5)
                if matches!(
                    &**system5,
                    vsm_rs::protocol::System5Event::CrisisHandled {
                        escalation_count: 1,
                        ..
                    }
                )
        )));

    runtime.shutdown().await.expect("shutdown should succeed");
}

fn runtime_builder(runtime_id: &'static str) -> VsmBuilder<DomainSystem> {
    let descriptor = UnitDescriptor::<DomainSystem>::new(UnitId("unit-a"), [Capability("work")]);
    let capacity = CapacitySnapshot::new(0, Some(4), 0.0);

    VsmBuilder::new()
        .runtime_id(RuntimeId::from_string(runtime_id))
        .work_model(AcceptAllWorkModel::new([Capability("work")]))
        .operational_unit_factory(StaticOperationalUnitFactory::new(
            descriptor,
            capacity,
            DomainOutcome,
        ))
}

struct TestIdentityProvider;

#[async_trait]
impl IdentityProvider<DomainSystem> for TestIdentityProvider {
    async fn provide_identity(
        &self,
        _context: &RoleContext<DomainSystem>,
    ) -> Result<IdentityRecord, FrameworkError> {
        Ok(IdentityRecord::new("example organization").with_purpose("serve test domain"))
    }
}

struct TestValuesProvider;

#[async_trait]
impl ValuesProvider<DomainSystem> for TestValuesProvider {
    async fn provide_values(
        &self,
        _context: &RoleContext<DomainSystem>,
    ) -> Result<ValueSet, FrameworkError> {
        Ok(ValueSet::new([ValueStatement::new("resilience", 1.0)]))
    }
}

struct PassingValuesEvaluator;

#[async_trait]
impl ValuesEvaluator<DomainSystem> for PassingValuesEvaluator {
    async fn evaluate_values(
        &self,
        _context: &RoleContext<DomainSystem>,
        _request: &DecisionRequest<DomainSystem>,
        identity: &IdentityRecord,
        values: &ValueSet,
    ) -> Result<ValuesEvaluation, FrameworkError> {
        let mut evaluation = ValuesEvaluation::neutral(identity, values);
        evaluation.score = 0.92;
        evaluation.rationale = Some("fixture aligns".to_string());
        Ok(evaluation)
    }
}

struct SelectingDecisionPolicy;

#[async_trait]
impl DecisionPolicy<DomainSystem> for SelectingDecisionPolicy {
    async fn decide(
        &self,
        _context: &RoleContext<DomainSystem>,
        request: &DecisionRequest<DomainSystem>,
        identity: &IdentityRecord,
        values: &ValueSet,
        evaluation: &ValuesEvaluation,
    ) -> Result<DecisionRecord<DomainSystem>, FrameworkError> {
        let selected = request
            .alternatives
            .first()
            .cloned()
            .unwrap_or_else(|| DecisionAlternative::new("defer"));
        Ok(DecisionRecord::new(
            request,
            identity,
            values,
            DecisionStatus::Approved,
            "selected by fixture",
        )
        .with_evaluation(evaluation.clone())
        .with_selected(selected))
    }
}

struct ContextAwareDecisionPolicy;

#[async_trait]
impl DecisionPolicy<DomainSystem> for ContextAwareDecisionPolicy {
    async fn decide(
        &self,
        _context: &RoleContext<DomainSystem>,
        request: &DecisionRequest<DomainSystem>,
        identity: &IdentityRecord,
        values: &ValueSet,
        evaluation: &ValuesEvaluation,
    ) -> Result<DecisionRecord<DomainSystem>, FrameworkError> {
        let selected = request
            .alternatives
            .first()
            .cloned()
            .unwrap_or_else(|| DecisionAlternative::new("defer"));
        Ok(DecisionRecord::new(
            request,
            identity,
            values,
            DecisionStatus::Approved,
            format!(
                "saw {} operational summaries and {} future proposals",
                request.operational_summaries.len(),
                request.adaptation_proposals.len()
            ),
        )
        .with_evaluation(evaluation.clone())
        .with_selected(selected))
    }
}

struct EscalatingCrisisPolicy;

#[async_trait]
impl CrisisPolicy<DomainSystem> for EscalatingCrisisPolicy {
    async fn respond_to_crisis(
        &self,
        context: &RoleContext<DomainSystem>,
        signal: &CrisisSignal,
        identity: &IdentityRecord,
        values: &ValueSet,
    ) -> Result<CrisisResponse<DomainSystem>, FrameworkError> {
        let request = DecisionRequest::new(format!("crisis: {}", signal.summary));
        let evaluation = ValuesEvaluation::neutral(identity, values);
        let mut decision = DecisionRecord::new(
            &request,
            identity,
            values,
            DecisionStatus::Crisis,
            "escalate outside local authority",
        )
        .with_evaluation(evaluation);
        decision.directives.push(
            PolicyDirective::new(PolicyDirectiveKind::CrisisResponse, "stabilize")
                .with_required_ack(false),
        );
        decision
            .escalations
            .push(
                PolicyEscalation::new("outside local authority").to_parent(VsmAddress::new(
                    context.runtime_id().clone(),
                    context.recursion_path().child("parent"),
                    SubsystemRole::System5,
                )),
            );
        Ok(CrisisResponse::new(signal, decision))
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

struct NoopDirectivePolicy;

#[async_trait]
impl OperationalControlPolicy<DomainSystem> for NoopDirectivePolicy {
    async fn plan_directives(
        &self,
        _context: &RoleContext<DomainSystem>,
        _allocations: &[ResourceAllocation<DomainSystem>],
        _performance: &[vsm_rs::protocol::system1::PerformanceObservation<DomainSystem>],
    ) -> Result<Vec<OperationalDirective<DomainSystem>>, FrameworkError> {
        Ok(Vec::new())
    }
}

struct TestSourceFactory;

#[async_trait]
impl EnvironmentalSourceFactory<DomainSystem> for TestSourceFactory {
    async fn create_source(
        &self,
        _context: &RoleContext<DomainSystem>,
        descriptor: &EnvironmentSourceDescriptor,
    ) -> Result<Box<dyn EnvironmentalSource<DomainSystem>>, FrameworkError> {
        Ok(Box::new(TestSource {
            source_id: descriptor.source_id.clone(),
        }))
    }
}

struct TestSource {
    source_id: String,
}

#[async_trait]
impl EnvironmentalSource<DomainSystem> for TestSource {
    async fn observe(
        &mut self,
        _context: &RoleContext<DomainSystem>,
        _descriptor: &EnvironmentSourceDescriptor,
    ) -> Result<Vec<EnvironmentalObservation>, FrameworkError> {
        Ok(vec![EnvironmentalObservation::new(self.source_id.clone())
            .with_measurement(EnvironmentalMeasurement::new(
                "demand", 0.8,
            ))])
    }
}

struct TestInterpreter;

#[async_trait]
impl SignalInterpreter<DomainSystem> for TestInterpreter {
    async fn interpret(
        &self,
        _context: &RoleContext<DomainSystem>,
        observations: &[EnvironmentalObservation],
    ) -> Result<Vec<InterpretedSignal>, FrameworkError> {
        Ok(observations
            .iter()
            .map(|observation| {
                InterpretedSignal::new(SignalKind::Opportunity)
                    .from_observation(observation)
                    .with_strength(0.7)
            })
            .collect())
    }
}

struct TestIntelligenceModel;

#[async_trait]
impl IntelligenceModel<DomainSystem> for TestIntelligenceModel {
    async fn assess(
        &self,
        _context: &RoleContext<DomainSystem>,
        signals: &[InterpretedSignal],
    ) -> Result<IntelligenceAssessment, FrameworkError> {
        Ok(IntelligenceAssessment::new(signals.to_vec()))
    }
}

struct TestForecaster;

#[async_trait]
impl Forecaster<DomainSystem> for TestForecaster {
    async fn forecast(
        &self,
        _context: &RoleContext<DomainSystem>,
        assessment: &IntelligenceAssessment,
        _signals: &[InterpretedSignal],
    ) -> Result<Vec<Forecast>, FrameworkError> {
        let mut forecast = Forecast::new(assessment, Duration::from_secs(60));
        forecast
            .points
            .push(ForecastPoint::new(Duration::from_secs(60), 0.8, 0.75));
        Ok(vec![forecast])
    }

    async fn plan_scenarios(
        &self,
        _context: &RoleContext<DomainSystem>,
        _assessment: &IntelligenceAssessment,
        forecasts: &[Forecast],
    ) -> Result<Vec<Scenario>, FrameworkError> {
        let mut scenario = Scenario::new("demand increase");
        scenario.forecast_id = forecasts
            .first()
            .map(|forecast| forecast.forecast_id.clone());
        Ok(vec![scenario])
    }

    async fn propose_adaptations(
        &self,
        _context: &RoleContext<DomainSystem>,
        _assessment: &IntelligenceAssessment,
        _forecasts: &[Forecast],
        scenarios: &[Scenario],
    ) -> Result<Vec<AdaptationProposal>, FrameworkError> {
        let mut proposal = AdaptationProposal::new("add capacity", "demand increase");
        proposal.scenario_id = scenarios
            .first()
            .map(|scenario| scenario.scenario_id.clone());
        Ok(vec![proposal])
    }

    async fn calibrate(
        &self,
        _context: &RoleContext<DomainSystem>,
        _forecasts: &[Forecast],
        _actuals: &[EnvironmentalObservation],
    ) -> Result<Vec<vsm_rs::protocol::system4::ForecastCalibration>, FrameworkError> {
        Ok(Vec::new())
    }
}
