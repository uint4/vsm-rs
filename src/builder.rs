//! Builder for typed VSM runtime handles.

use std::sync::Arc;
use std::time::Duration;

use crate::config::RuntimeConfig;
use crate::error::FrameworkError;
use crate::protocol::{RecursionPath, RuntimeId};
use crate::roles::system1::defaults::{
    LowestLoadSelectionPolicy, NoopAlgedonicPolicy, NoopPerformanceModel, NoopVarietyModel,
};
use crate::roles::system2::defaults::NoopCoordinationPolicy;
use crate::roles::system3::defaults::{
    DenyAllResourceGovernance, NoopAuditor, NoopOperationalControlPolicy,
};
use crate::roles::system4::defaults::{
    NoopEnvironmentalSourceFactory, NoopForecaster, NoopIntelligenceModel, NoopSignalInterpreter,
};
use crate::roles::system5::defaults::{
    NoopCrisisPolicy, NoopDecisionPolicy, NoopIdentityProvider, NoopValuesEvaluator,
    NoopValuesProvider,
};
use crate::roles::variety::defaults::{
    DefaultAlgedonicLifecyclePolicy, NoopTemporalAnalysisPolicy, NoopVarietyEngineeringPolicy,
};
use crate::roles::{
    AlertSink, AlgedonicLifecyclePolicy, AlgedonicPolicy, Auditor, Clock, CoordinationPolicy,
    CrisisPolicy, DecisionPolicy, EnvironmentalSourceFactory, EventSink, Forecaster,
    IdentityProvider, IntelligenceModel, NoopAlertSink, NoopEventSink, NoopReportSink,
    NoopStateStore, NoopTelemetrySink, OperationalControlPolicy, OperationalUnitFactory,
    PerformanceModel, ReportSink, ResourceGovernance, SharedAlgedonicLifecyclePolicy,
    SharedAlgedonicPolicy, SharedAuditor, SharedCoordinationPolicy, SharedCrisisPolicy,
    SharedDecisionPolicy, SharedEnvironmentalSourceFactory, SharedForecaster,
    SharedIdentityProvider, SharedIntelligenceModel, SharedOperationalControlPolicy,
    SharedOperationalUnitFactory, SharedPerformanceModel, SharedResourceGovernance,
    SharedSignalInterpreter, SharedTemporalAnalysisPolicy, SharedUnitSelectionPolicy,
    SharedValuesEvaluator, SharedValuesProvider, SharedVarietyEngineeringPolicy,
    SharedVarietyModel, SharedWorkModel, SignalInterpreter, StateStore, SystemClock, TelemetrySink,
    TemporalAnalysisPolicy, UnitSelectionPolicy, ValuesEvaluator, ValuesProvider,
    VarietyEngineeringPolicy, VarietyModel, ViableSystem, WorkModel,
};
use crate::runtime::{
    RuntimePorts, RuntimeRoleBundles, System1RuntimeRoles, System2RuntimeRoles,
    System3RuntimeRoles, System4RuntimeRoles, System5RuntimeRoles, VarietyRuntimeRoles, VsmRuntime,
};

/// Builder for one typed VSM runtime instance.
///
/// The builder validates required System 1 roles up front, applies opt-in
/// defaults for optional policies, and returns an instance-scoped runtime
/// handle. Work execution is added by the System 1 actor-adapter milestone.
pub struct VsmBuilder<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    work_model: Option<SharedWorkModel<V>>,
    operational_unit_factory: Option<SharedOperationalUnitFactory<V>>,
    unit_selection_policy: Option<SharedUnitSelectionPolicy<V>>,
    performance_model: Option<SharedPerformanceModel<V>>,
    variety_model: Option<SharedVarietyModel<V>>,
    algedonic_policy: Option<SharedAlgedonicPolicy<V>>,
    coordination_policy: Option<SharedCoordinationPolicy<V>>,
    resource_governance: Option<SharedResourceGovernance<V>>,
    operational_control_policy: Option<SharedOperationalControlPolicy<V>>,
    auditor: Option<SharedAuditor<V>>,
    environmental_source_factory: Option<SharedEnvironmentalSourceFactory<V>>,
    signal_interpreter: Option<SharedSignalInterpreter<V>>,
    intelligence_model: Option<SharedIntelligenceModel<V>>,
    forecaster: Option<SharedForecaster<V>>,
    identity_provider: Option<SharedIdentityProvider<V>>,
    values_provider: Option<SharedValuesProvider<V>>,
    values_evaluator: Option<SharedValuesEvaluator<V>>,
    decision_policy: Option<SharedDecisionPolicy<V>>,
    crisis_policy: Option<SharedCrisisPolicy<V>>,
    variety_engineering_policy: Option<SharedVarietyEngineeringPolicy<V>>,
    algedonic_lifecycle_policy: Option<SharedAlgedonicLifecyclePolicy<V>>,
    temporal_analysis_policy: Option<SharedTemporalAnalysisPolicy<V>>,
    state_store: Arc<dyn StateStore<V>>,
    event_sink: Arc<dyn EventSink<V>>,
    report_sink: Arc<dyn ReportSink<V>>,
    telemetry_sink: Arc<dyn TelemetrySink>,
    alert_sink: Arc<dyn AlertSink>,
    clock: Arc<dyn Clock>,
}

impl<V> Default for VsmBuilder<V>
where
    V: ViableSystem,
{
    fn default() -> Self {
        Self {
            config: RuntimeConfig::default(),
            work_model: None,
            operational_unit_factory: None,
            unit_selection_policy: None,
            performance_model: None,
            variety_model: None,
            algedonic_policy: None,
            coordination_policy: None,
            resource_governance: None,
            operational_control_policy: None,
            auditor: None,
            environmental_source_factory: None,
            signal_interpreter: None,
            intelligence_model: None,
            forecaster: None,
            identity_provider: None,
            values_provider: None,
            values_evaluator: None,
            decision_policy: None,
            crisis_policy: None,
            variety_engineering_policy: None,
            algedonic_lifecycle_policy: None,
            temporal_analysis_policy: None,
            state_store: Arc::new(NoopStateStore::<V>::new()),
            event_sink: Arc::new(NoopEventSink::<V>::new()),
            report_sink: Arc::new(NoopReportSink::<V>::new()),
            telemetry_sink: Arc::new(NoopTelemetrySink),
            alert_sink: Arc::new(NoopAlertSink),
            clock: Arc::new(SystemClock),
        }
    }
}

impl<V> VsmBuilder<V>
where
    V: ViableSystem,
{
    /// Creates a builder with generated runtime identity and no-op ports.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the full runtime configuration.
    pub fn config(mut self, config: RuntimeConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets the runtime instance identity.
    pub fn runtime_id(mut self, runtime_id: RuntimeId) -> Self {
        self.config.runtime_id = runtime_id;
        self
    }

    /// Sets the recursion path for this runtime instance.
    pub fn recursion_path(mut self, recursion_path: RecursionPath) -> Self {
        self.config.recursion_path = recursion_path;
        self
    }

    /// Sets the runtime default work timeout.
    pub fn default_work_timeout(mut self, timeout: Duration) -> Self {
        self.config.default_work_timeout = timeout;
        self
    }

    /// Sets the readiness timeout used by future actor-backed startup.
    pub fn readiness_timeout(mut self, timeout: Duration) -> Self {
        self.config.readiness_timeout = timeout;
        self
    }

    /// Sets the shutdown timeout used by future actor-backed teardown.
    pub fn shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.config.shutdown_timeout = timeout;
        self
    }

    /// Sets the maximum number of units that can be registered.
    pub fn max_registered_units(mut self, max_units: usize) -> Self {
        self.config.max_registered_units = Some(max_units);
        self
    }

    /// Removes the maximum registered-unit limit.
    pub fn unbounded_registered_units(mut self) -> Self {
        self.config.max_registered_units = None;
        self
    }

    /// Sets the in-memory observer event buffer capacity.
    pub fn event_buffer_capacity(mut self, capacity: usize) -> Self {
        self.config.event_buffer_capacity = capacity;
        self
    }

    /// Sets the required work model role.
    pub fn work_model<M>(mut self, work_model: M) -> Self
    where
        M: WorkModel<V> + 'static,
    {
        self.work_model = Some(Arc::new(work_model));
        self
    }

    /// Sets the required work model role from a shared trait object.
    pub fn work_model_arc(mut self, work_model: SharedWorkModel<V>) -> Self {
        self.work_model = Some(work_model);
        self
    }

    /// Sets the required operational-unit factory role.
    pub fn operational_unit_factory<F>(mut self, factory: F) -> Self
    where
        F: OperationalUnitFactory<V> + 'static,
    {
        self.operational_unit_factory = Some(Arc::new(factory));
        self
    }

    /// Sets the required operational-unit factory role from a shared trait object.
    pub fn operational_unit_factory_arc(
        mut self,
        factory: SharedOperationalUnitFactory<V>,
    ) -> Self {
        self.operational_unit_factory = Some(factory);
        self
    }

    /// Sets the optional unit-selection policy role.
    pub fn unit_selection_policy<P>(mut self, policy: P) -> Self
    where
        P: UnitSelectionPolicy<V> + 'static,
    {
        self.unit_selection_policy = Some(Arc::new(policy));
        self
    }

    /// Sets the optional unit-selection policy from a shared trait object.
    pub fn unit_selection_policy_arc(mut self, policy: SharedUnitSelectionPolicy<V>) -> Self {
        self.unit_selection_policy = Some(policy);
        self
    }

    /// Sets the optional performance model role.
    pub fn performance_model<M>(mut self, model: M) -> Self
    where
        M: PerformanceModel<V> + 'static,
    {
        self.performance_model = Some(Arc::new(model));
        self
    }

    /// Sets the optional performance model from a shared trait object.
    pub fn performance_model_arc(mut self, model: SharedPerformanceModel<V>) -> Self {
        self.performance_model = Some(model);
        self
    }

    /// Sets the optional variety model role.
    pub fn variety_model<M>(mut self, model: M) -> Self
    where
        M: VarietyModel<V> + 'static,
    {
        self.variety_model = Some(Arc::new(model));
        self
    }

    /// Sets the optional variety model from a shared trait object.
    pub fn variety_model_arc(mut self, model: SharedVarietyModel<V>) -> Self {
        self.variety_model = Some(model);
        self
    }

    /// Sets the optional algedonic policy role.
    pub fn algedonic_policy<P>(mut self, policy: P) -> Self
    where
        P: AlgedonicPolicy<V> + 'static,
    {
        self.algedonic_policy = Some(Arc::new(policy));
        self
    }

    /// Sets the optional algedonic policy from a shared trait object.
    pub fn algedonic_policy_arc(mut self, policy: SharedAlgedonicPolicy<V>) -> Self {
        self.algedonic_policy = Some(policy);
        self
    }

    /// Sets the optional System 2 coordination policy role.
    pub fn coordination_policy<P>(mut self, policy: P) -> Self
    where
        P: CoordinationPolicy<V> + 'static,
    {
        self.coordination_policy = Some(Arc::new(policy));
        self
    }

    /// Sets the optional System 2 coordination policy from a shared trait object.
    pub fn coordination_policy_arc(mut self, policy: SharedCoordinationPolicy<V>) -> Self {
        self.coordination_policy = Some(policy);
        self
    }

    /// Sets the optional System 3 resource governance role.
    pub fn resource_governance<G>(mut self, governance: G) -> Self
    where
        G: ResourceGovernance<V> + 'static,
    {
        self.resource_governance = Some(Arc::new(governance));
        self
    }

    /// Sets the optional System 3 resource governance role from a shared trait object.
    pub fn resource_governance_arc(mut self, governance: SharedResourceGovernance<V>) -> Self {
        self.resource_governance = Some(governance);
        self
    }

    /// Sets the optional System 3 operational control policy role.
    pub fn operational_control_policy<P>(mut self, policy: P) -> Self
    where
        P: OperationalControlPolicy<V> + 'static,
    {
        self.operational_control_policy = Some(Arc::new(policy));
        self
    }

    /// Sets the optional System 3 operational control policy from a shared trait object.
    pub fn operational_control_policy_arc(
        mut self,
        policy: SharedOperationalControlPolicy<V>,
    ) -> Self {
        self.operational_control_policy = Some(policy);
        self
    }

    /// Sets the optional System 3* auditor role.
    pub fn auditor<A>(mut self, auditor: A) -> Self
    where
        A: Auditor<V> + 'static,
    {
        self.auditor = Some(Arc::new(auditor));
        self
    }

    /// Sets the optional System 3* auditor role from a shared trait object.
    pub fn auditor_arc(mut self, auditor: SharedAuditor<V>) -> Self {
        self.auditor = Some(auditor);
        self
    }

    /// Sets the optional System 4 environmental source factory role.
    pub fn environmental_source_factory<F>(mut self, factory: F) -> Self
    where
        F: EnvironmentalSourceFactory<V> + 'static,
    {
        self.environmental_source_factory = Some(Arc::new(factory));
        self
    }

    /// Sets the optional System 4 environmental source factory from a shared trait object.
    pub fn environmental_source_factory_arc(
        mut self,
        factory: SharedEnvironmentalSourceFactory<V>,
    ) -> Self {
        self.environmental_source_factory = Some(factory);
        self
    }

    /// Sets the optional System 4 signal interpreter role.
    pub fn signal_interpreter<I>(mut self, interpreter: I) -> Self
    where
        I: SignalInterpreter<V> + 'static,
    {
        self.signal_interpreter = Some(Arc::new(interpreter));
        self
    }

    /// Sets the optional System 4 signal interpreter from a shared trait object.
    pub fn signal_interpreter_arc(mut self, interpreter: SharedSignalInterpreter<V>) -> Self {
        self.signal_interpreter = Some(interpreter);
        self
    }

    /// Sets the optional System 4 intelligence model role.
    pub fn intelligence_model<M>(mut self, model: M) -> Self
    where
        M: IntelligenceModel<V> + 'static,
    {
        self.intelligence_model = Some(Arc::new(model));
        self
    }

    /// Sets the optional System 4 intelligence model from a shared trait object.
    pub fn intelligence_model_arc(mut self, model: SharedIntelligenceModel<V>) -> Self {
        self.intelligence_model = Some(model);
        self
    }

    /// Sets the optional System 4 forecaster role.
    pub fn forecaster<F>(mut self, forecaster: F) -> Self
    where
        F: Forecaster<V> + 'static,
    {
        self.forecaster = Some(Arc::new(forecaster));
        self
    }

    /// Sets the optional System 4 forecaster from a shared trait object.
    pub fn forecaster_arc(mut self, forecaster: SharedForecaster<V>) -> Self {
        self.forecaster = Some(forecaster);
        self
    }

    /// Sets the optional System 5 identity provider role.
    pub fn identity_provider<P>(mut self, provider: P) -> Self
    where
        P: IdentityProvider<V> + 'static,
    {
        self.identity_provider = Some(Arc::new(provider));
        self
    }

    /// Sets the optional System 5 identity provider from a shared trait object.
    pub fn identity_provider_arc(mut self, provider: SharedIdentityProvider<V>) -> Self {
        self.identity_provider = Some(provider);
        self
    }

    /// Sets the optional System 5 values provider role.
    pub fn values_provider<P>(mut self, provider: P) -> Self
    where
        P: ValuesProvider<V> + 'static,
    {
        self.values_provider = Some(Arc::new(provider));
        self
    }

    /// Sets the optional System 5 values provider from a shared trait object.
    pub fn values_provider_arc(mut self, provider: SharedValuesProvider<V>) -> Self {
        self.values_provider = Some(provider);
        self
    }

    /// Sets the optional System 5 values evaluator role.
    pub fn values_evaluator<E>(mut self, evaluator: E) -> Self
    where
        E: ValuesEvaluator<V> + 'static,
    {
        self.values_evaluator = Some(Arc::new(evaluator));
        self
    }

    /// Sets the optional System 5 values evaluator from a shared trait object.
    pub fn values_evaluator_arc(mut self, evaluator: SharedValuesEvaluator<V>) -> Self {
        self.values_evaluator = Some(evaluator);
        self
    }

    /// Sets the optional System 5 decision policy role.
    pub fn decision_policy<P>(mut self, policy: P) -> Self
    where
        P: DecisionPolicy<V> + 'static,
    {
        self.decision_policy = Some(Arc::new(policy));
        self
    }

    /// Sets the optional System 5 decision policy from a shared trait object.
    pub fn decision_policy_arc(mut self, policy: SharedDecisionPolicy<V>) -> Self {
        self.decision_policy = Some(policy);
        self
    }

    /// Sets the optional System 5 crisis policy role.
    pub fn crisis_policy<P>(mut self, policy: P) -> Self
    where
        P: CrisisPolicy<V> + 'static,
    {
        self.crisis_policy = Some(Arc::new(policy));
        self
    }

    /// Sets the optional System 5 crisis policy from a shared trait object.
    pub fn crisis_policy_arc(mut self, policy: SharedCrisisPolicy<V>) -> Self {
        self.crisis_policy = Some(policy);
        self
    }

    /// Sets the optional variety engineering policy role.
    pub fn variety_engineering_policy<P>(mut self, policy: P) -> Self
    where
        P: VarietyEngineeringPolicy<V> + 'static,
    {
        self.variety_engineering_policy = Some(Arc::new(policy));
        self
    }

    /// Sets the optional variety engineering policy from a shared trait object.
    pub fn variety_engineering_policy_arc(
        mut self,
        policy: SharedVarietyEngineeringPolicy<V>,
    ) -> Self {
        self.variety_engineering_policy = Some(policy);
        self
    }

    /// Sets the optional algedonic lifecycle policy role.
    pub fn algedonic_lifecycle_policy<P>(mut self, policy: P) -> Self
    where
        P: AlgedonicLifecyclePolicy<V> + 'static,
    {
        self.algedonic_lifecycle_policy = Some(Arc::new(policy));
        self
    }

    /// Sets the optional algedonic lifecycle policy from a shared trait object.
    pub fn algedonic_lifecycle_policy_arc(
        mut self,
        policy: SharedAlgedonicLifecyclePolicy<V>,
    ) -> Self {
        self.algedonic_lifecycle_policy = Some(policy);
        self
    }

    /// Sets the optional temporal analysis policy role.
    pub fn temporal_analysis_policy<P>(mut self, policy: P) -> Self
    where
        P: TemporalAnalysisPolicy<V> + 'static,
    {
        self.temporal_analysis_policy = Some(Arc::new(policy));
        self
    }

    /// Sets the optional temporal analysis policy from a shared trait object.
    pub fn temporal_analysis_policy_arc(mut self, policy: SharedTemporalAnalysisPolicy<V>) -> Self {
        self.temporal_analysis_policy = Some(policy);
        self
    }

    /// Sets the state store port. The default is [`NoopStateStore`].
    pub fn state_store<S>(mut self, state_store: S) -> Self
    where
        S: StateStore<V> + 'static,
    {
        self.state_store = Arc::new(state_store);
        self
    }

    /// Sets the state store port from a shared trait object.
    pub fn state_store_arc(mut self, state_store: Arc<dyn StateStore<V>>) -> Self {
        self.state_store = state_store;
        self
    }

    /// Sets the observer event sink port. The default drops events.
    pub fn event_sink<S>(mut self, sink: S) -> Self
    where
        S: EventSink<V> + 'static,
    {
        self.event_sink = Arc::new(sink);
        self
    }

    /// Sets the observer event sink port from a shared trait object.
    pub fn event_sink_arc(mut self, sink: Arc<dyn EventSink<V>>) -> Self {
        self.event_sink = sink;
        self
    }

    /// Sets the report sink port. The default drops reports.
    pub fn report_sink<S>(mut self, sink: S) -> Self
    where
        S: ReportSink<V> + 'static,
    {
        self.report_sink = Arc::new(sink);
        self
    }

    /// Sets the report sink port from a shared trait object.
    pub fn report_sink_arc(mut self, sink: Arc<dyn ReportSink<V>>) -> Self {
        self.report_sink = sink;
        self
    }

    /// Sets the telemetry sink port. The default drops telemetry.
    pub fn telemetry_sink<S>(mut self, sink: S) -> Self
    where
        S: TelemetrySink + 'static,
    {
        self.telemetry_sink = Arc::new(sink);
        self
    }

    /// Sets the telemetry sink port from a shared trait object.
    pub fn telemetry_sink_arc(mut self, sink: Arc<dyn TelemetrySink>) -> Self {
        self.telemetry_sink = sink;
        self
    }

    /// Sets the alert sink port. The default drops alerts.
    pub fn alert_sink<S>(mut self, sink: S) -> Self
    where
        S: AlertSink + 'static,
    {
        self.alert_sink = Arc::new(sink);
        self
    }

    /// Sets the alert sink port from a shared trait object.
    pub fn alert_sink_arc(mut self, sink: Arc<dyn AlertSink>) -> Self {
        self.alert_sink = sink;
        self
    }

    /// Sets the clock port. The default is system time.
    pub fn clock<C>(mut self, clock: C) -> Self
    where
        C: Clock + 'static,
    {
        self.clock = Arc::new(clock);
        self
    }

    /// Sets the clock port from a shared trait object.
    pub fn clock_arc(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Starts the typed runtime handle.
    ///
    /// This method is async so the signature can absorb actor-backed startup
    /// without another public API break in the next milestone.
    pub async fn start(self) -> Result<VsmRuntime<V>, FrameworkError> {
        let Self {
            config,
            work_model,
            operational_unit_factory,
            unit_selection_policy,
            performance_model,
            variety_model,
            algedonic_policy,
            coordination_policy,
            resource_governance,
            operational_control_policy,
            auditor,
            environmental_source_factory,
            signal_interpreter,
            intelligence_model,
            forecaster,
            identity_provider,
            values_provider,
            values_evaluator,
            decision_policy,
            crisis_policy,
            variety_engineering_policy,
            algedonic_lifecycle_policy,
            temporal_analysis_policy,
            state_store,
            event_sink,
            report_sink,
            telemetry_sink,
            alert_sink,
            clock,
        } = self;

        let roles = system1_roles(
            work_model,
            operational_unit_factory,
            unit_selection_policy,
            performance_model,
            variety_model,
            algedonic_policy,
        )?;
        let system2_roles = system2_roles(coordination_policy);
        let system3_roles = system3_roles(resource_governance, operational_control_policy, auditor);
        let system4_roles = system4_roles(
            environmental_source_factory,
            signal_interpreter,
            intelligence_model,
            forecaster,
        );
        let system5_roles = system5_roles(
            identity_provider,
            values_provider,
            values_evaluator,
            decision_policy,
            crisis_policy,
        );
        let variety_roles = variety_roles(
            variety_engineering_policy,
            algedonic_lifecycle_policy,
            temporal_analysis_policy,
        );
        let ports = RuntimePorts::noop()
            .with_state_store(state_store)
            .with_event_sink(event_sink)
            .with_report_sink(report_sink)
            .with_telemetry_sink(telemetry_sink)
            .with_alert_sink(alert_sink)
            .with_clock(clock);

        let roles = RuntimeRoleBundles::new(
            roles,
            system2_roles,
            system3_roles,
            system4_roles,
            system5_roles,
            variety_roles,
        );

        VsmRuntime::new(config, ports, roles).await
    }
}

fn system1_roles<V>(
    work_model: Option<SharedWorkModel<V>>,
    operational_unit_factory: Option<SharedOperationalUnitFactory<V>>,
    unit_selection_policy: Option<SharedUnitSelectionPolicy<V>>,
    performance_model: Option<SharedPerformanceModel<V>>,
    variety_model: Option<SharedVarietyModel<V>>,
    algedonic_policy: Option<SharedAlgedonicPolicy<V>>,
) -> Result<System1RuntimeRoles<V>, FrameworkError>
where
    V: ViableSystem,
{
    let work_model = work_model.ok_or_else(|| missing_required_role("WorkModel"))?;
    let operational_unit_factory =
        operational_unit_factory.ok_or_else(|| missing_required_role("OperationalUnitFactory"))?;
    let unit_selection_policy =
        unit_selection_policy.unwrap_or_else(|| Arc::new(LowestLoadSelectionPolicy));
    let performance_model = performance_model.unwrap_or_else(|| Arc::new(NoopPerformanceModel));
    let variety_model = variety_model.unwrap_or_else(|| Arc::new(NoopVarietyModel));
    let algedonic_policy = algedonic_policy.unwrap_or_else(|| Arc::new(NoopAlgedonicPolicy));

    Ok(System1RuntimeRoles::new(
        work_model,
        operational_unit_factory,
        unit_selection_policy,
        performance_model,
        variety_model,
        algedonic_policy,
    ))
}

fn missing_required_role(role: &'static str) -> FrameworkError {
    FrameworkError::InvalidProtocol {
        reason: format!("missing required System 1 role: {role}"),
    }
}

fn system2_roles<V>(
    coordination_policy: Option<SharedCoordinationPolicy<V>>,
) -> System2RuntimeRoles<V>
where
    V: ViableSystem,
{
    let coordination_policy =
        coordination_policy.unwrap_or_else(|| Arc::new(NoopCoordinationPolicy::<V>::new()));

    System2RuntimeRoles::new(coordination_policy)
}

fn system3_roles<V>(
    resource_governance: Option<SharedResourceGovernance<V>>,
    operational_control_policy: Option<SharedOperationalControlPolicy<V>>,
    auditor: Option<SharedAuditor<V>>,
) -> System3RuntimeRoles<V>
where
    V: ViableSystem,
{
    let resource_governance =
        resource_governance.unwrap_or_else(|| Arc::new(DenyAllResourceGovernance::<V>::new()));
    let operational_control_policy = operational_control_policy
        .unwrap_or_else(|| Arc::new(NoopOperationalControlPolicy::<V>::new()));
    let auditor = auditor.unwrap_or_else(|| Arc::new(NoopAuditor::<V>::new()));

    System3RuntimeRoles::new(resource_governance, operational_control_policy, auditor)
}

fn system4_roles<V>(
    environmental_source_factory: Option<SharedEnvironmentalSourceFactory<V>>,
    signal_interpreter: Option<SharedSignalInterpreter<V>>,
    intelligence_model: Option<SharedIntelligenceModel<V>>,
    forecaster: Option<SharedForecaster<V>>,
) -> System4RuntimeRoles<V>
where
    V: ViableSystem,
{
    let environmental_source_factory = environmental_source_factory
        .unwrap_or_else(|| Arc::new(NoopEnvironmentalSourceFactory::<V>::new()));
    let signal_interpreter =
        signal_interpreter.unwrap_or_else(|| Arc::new(NoopSignalInterpreter::<V>::new()));
    let intelligence_model =
        intelligence_model.unwrap_or_else(|| Arc::new(NoopIntelligenceModel::<V>::new()));
    let forecaster = forecaster.unwrap_or_else(|| Arc::new(NoopForecaster::<V>::new()));

    System4RuntimeRoles::new(
        environmental_source_factory,
        signal_interpreter,
        intelligence_model,
        forecaster,
    )
}

fn system5_roles<V>(
    identity_provider: Option<SharedIdentityProvider<V>>,
    values_provider: Option<SharedValuesProvider<V>>,
    values_evaluator: Option<SharedValuesEvaluator<V>>,
    decision_policy: Option<SharedDecisionPolicy<V>>,
    crisis_policy: Option<SharedCrisisPolicy<V>>,
) -> System5RuntimeRoles<V>
where
    V: ViableSystem,
{
    let identity_provider =
        identity_provider.unwrap_or_else(|| Arc::new(NoopIdentityProvider::<V>::new()));
    let values_provider =
        values_provider.unwrap_or_else(|| Arc::new(NoopValuesProvider::<V>::new()));
    let values_evaluator =
        values_evaluator.unwrap_or_else(|| Arc::new(NoopValuesEvaluator::<V>::new()));
    let decision_policy =
        decision_policy.unwrap_or_else(|| Arc::new(NoopDecisionPolicy::<V>::new()));
    let crisis_policy = crisis_policy.unwrap_or_else(|| Arc::new(NoopCrisisPolicy::<V>::new()));

    System5RuntimeRoles::new(
        identity_provider,
        values_provider,
        values_evaluator,
        decision_policy,
        crisis_policy,
    )
}

fn variety_roles<V>(
    variety_engineering_policy: Option<SharedVarietyEngineeringPolicy<V>>,
    algedonic_lifecycle_policy: Option<SharedAlgedonicLifecyclePolicy<V>>,
    temporal_analysis_policy: Option<SharedTemporalAnalysisPolicy<V>>,
) -> VarietyRuntimeRoles<V>
where
    V: ViableSystem,
{
    let variety_engineering_policy = variety_engineering_policy
        .unwrap_or_else(|| Arc::new(NoopVarietyEngineeringPolicy::<V>::new()));
    let algedonic_lifecycle_policy = algedonic_lifecycle_policy
        .unwrap_or_else(|| Arc::new(DefaultAlgedonicLifecyclePolicy::<V>::new()));
    let temporal_analysis_policy = temporal_analysis_policy
        .unwrap_or_else(|| Arc::new(NoopTemporalAnalysisPolicy::<V>::new()));

    VarietyRuntimeRoles::new(
        variety_engineering_policy,
        algedonic_lifecycle_policy,
        temporal_analysis_policy,
    )
}
