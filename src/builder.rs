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
use crate::roles::{
    AlertSink, AlgedonicPolicy, Clock, CoordinationPolicy, EventSink, NoopAlertSink, NoopEventSink,
    NoopReportSink, NoopStateStore, NoopTelemetrySink, OperationalUnitFactory, PerformanceModel,
    ReportSink, SharedAlgedonicPolicy, SharedCoordinationPolicy, SharedOperationalUnitFactory,
    SharedPerformanceModel, SharedUnitSelectionPolicy, SharedVarietyModel, SharedWorkModel,
    StateStore, SystemClock, TelemetrySink, UnitSelectionPolicy, VarietyModel, ViableSystem,
    WorkModel,
};
use crate::runtime::{RuntimePorts, System1RuntimeRoles, System2RuntimeRoles, VsmRuntime};

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
        let ports = RuntimePorts::noop()
            .with_state_store(state_store)
            .with_event_sink(event_sink)
            .with_report_sink(report_sink)
            .with_telemetry_sink(telemetry_sink)
            .with_alert_sink(alert_sink)
            .with_clock(clock);

        VsmRuntime::new(config, ports, roles, system2_roles).await
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
