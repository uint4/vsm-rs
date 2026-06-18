//! Typed runtime handles for the trait-driven surface.

use std::sync::{Arc, Mutex};

use crate::config::RuntimeConfig;
use crate::error::FrameworkError;
use crate::kernel::registry::RuntimeDirectory;
use crate::protocol::{RecursionPath, RuntimeId, SubsystemRole, VsmAddress};
use crate::roles::RoleContext;
use crate::roles::{
    AlertSink, Clock, EventSink, NoopAlertSink, NoopEventSink, NoopReportSink, NoopStateStore,
    NoopTelemetrySink, ReportSink, SharedAlgedonicPolicy, SharedOperationalUnitFactory,
    SharedPerformanceModel, SharedUnitSelectionPolicy, SharedVarietyModel, SharedWorkModel,
    StateStore, SystemClock, TelemetrySink, ViableSystem,
};

/// Runtime lifecycle state visible through typed handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    Ready,
    ShuttingDown,
    Shutdown,
}

/// Readiness gates reported by a typed runtime handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReadinessGate {
    Infrastructure,
    SubsystemActors,
    RoleImplementations,
    Subscriptions,
    Persistence,
}

/// Status for one readiness gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessStatus {
    Ready,
    NotApplicable,
    Pending,
    Failed,
}

impl ReadinessStatus {
    fn satisfies_readiness(self) -> bool {
        matches!(self, Self::Ready | Self::NotApplicable)
    }
}

/// One readiness observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadinessCheck {
    pub gate: ReadinessGate,
    pub status: ReadinessStatus,
    pub detail: String,
}

impl ReadinessCheck {
    /// Creates a readiness observation.
    pub fn new(gate: ReadinessGate, status: ReadinessStatus, detail: impl Into<String>) -> Self {
        Self {
            gate,
            status,
            detail: detail.into(),
        }
    }
}

/// Snapshot of runtime readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeReadiness {
    checks: Vec<ReadinessCheck>,
}

impl RuntimeReadiness {
    /// Creates a readiness snapshot from checks.
    pub fn new(checks: Vec<ReadinessCheck>) -> Self {
        Self { checks }
    }

    /// Returns `true` when every readiness gate is satisfied.
    pub fn is_ready(&self) -> bool {
        self.checks
            .iter()
            .all(|check| check.status.satisfies_readiness())
    }

    /// Returns all readiness checks.
    pub fn checks(&self) -> &[ReadinessCheck] {
        &self.checks
    }

    /// Returns the check for a specific gate, when present.
    pub fn check(&self, gate: ReadinessGate) -> Option<&ReadinessCheck> {
        self.checks.iter().find(|check| check.gate == gate)
    }
}

/// Component lifecycle state in the private runtime directory snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeComponentStatus {
    Ready,
    NotApplicable,
    Shutdown,
}

/// Public snapshot of one private runtime component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeComponentSnapshot {
    pub internal_name: String,
    pub address: VsmAddress,
    pub status: RuntimeComponentStatus,
}

/// Public snapshot of the runtime's private component directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDirectorySnapshot {
    pub components: Vec<RuntimeComponentSnapshot>,
}

impl RuntimeDirectorySnapshot {
    /// Returns true when no component names are registered.
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Returns the number of registered components.
    pub fn len(&self) -> usize {
        self.components.len()
    }
}

/// Shutdown acknowledgement returned by runtime handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShutdownReport {
    pub runtime_id: RuntimeId,
    pub previous_state: RuntimeState,
    pub current_state: RuntimeState,
    pub already_shutdown: bool,
}

/// Shared runtime ports used to create role contexts.
pub struct RuntimePorts<V>
where
    V: ViableSystem,
{
    state_store: Arc<dyn StateStore<V>>,
    event_sink: Arc<dyn EventSink<V>>,
    report_sink: Arc<dyn ReportSink<V>>,
    telemetry_sink: Arc<dyn TelemetrySink>,
    alert_sink: Arc<dyn AlertSink>,
    clock: Arc<dyn Clock>,
}

impl<V> Clone for RuntimePorts<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            state_store: Arc::clone(&self.state_store),
            event_sink: Arc::clone(&self.event_sink),
            report_sink: Arc::clone(&self.report_sink),
            telemetry_sink: Arc::clone(&self.telemetry_sink),
            alert_sink: Arc::clone(&self.alert_sink),
            clock: Arc::clone(&self.clock),
        }
    }
}

impl<V> RuntimePorts<V>
where
    V: ViableSystem,
{
    /// Creates ports with no-op storage, reporting, telemetry, alerts, and system time.
    pub fn noop() -> Self {
        Self {
            state_store: Arc::new(NoopStateStore::<V>::new()),
            event_sink: Arc::new(NoopEventSink::<V>::new()),
            report_sink: Arc::new(NoopReportSink::<V>::new()),
            telemetry_sink: Arc::new(NoopTelemetrySink),
            alert_sink: Arc::new(NoopAlertSink),
            clock: Arc::new(SystemClock),
        }
    }

    pub(crate) fn with_state_store(mut self, state_store: Arc<dyn StateStore<V>>) -> Self {
        self.state_store = state_store;
        self
    }

    pub(crate) fn with_event_sink(mut self, event_sink: Arc<dyn EventSink<V>>) -> Self {
        self.event_sink = event_sink;
        self
    }

    pub(crate) fn with_report_sink(mut self, report_sink: Arc<dyn ReportSink<V>>) -> Self {
        self.report_sink = report_sink;
        self
    }

    pub(crate) fn with_telemetry_sink(mut self, telemetry_sink: Arc<dyn TelemetrySink>) -> Self {
        self.telemetry_sink = telemetry_sink;
        self
    }

    pub(crate) fn with_alert_sink(mut self, alert_sink: Arc<dyn AlertSink>) -> Self {
        self.alert_sink = alert_sink;
        self
    }

    pub(crate) fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Builds a role context bound to the configured runtime instance and ports.
    pub fn role_context(
        &self,
        runtime_id: RuntimeId,
        recursion_path: RecursionPath,
        role: SubsystemRole,
    ) -> RoleContext<V> {
        RoleContext::new(runtime_id, recursion_path, role)
            .with_clock(Arc::clone(&self.clock))
            .with_event_sink(Arc::clone(&self.event_sink))
            .with_report_sink(Arc::clone(&self.report_sink))
            .with_state_store(Arc::clone(&self.state_store))
    }
}

/// Runtime-selected System 1 role objects.
pub struct System1RuntimeRoles<V>
where
    V: ViableSystem,
{
    work_model: SharedWorkModel<V>,
    operational_unit_factory: SharedOperationalUnitFactory<V>,
    unit_selection_policy: SharedUnitSelectionPolicy<V>,
    performance_model: SharedPerformanceModel<V>,
    variety_model: SharedVarietyModel<V>,
    algedonic_policy: SharedAlgedonicPolicy<V>,
}

impl<V> Clone for System1RuntimeRoles<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            work_model: Arc::clone(&self.work_model),
            operational_unit_factory: Arc::clone(&self.operational_unit_factory),
            unit_selection_policy: Arc::clone(&self.unit_selection_policy),
            performance_model: Arc::clone(&self.performance_model),
            variety_model: Arc::clone(&self.variety_model),
            algedonic_policy: Arc::clone(&self.algedonic_policy),
        }
    }
}

impl<V> System1RuntimeRoles<V>
where
    V: ViableSystem,
{
    /// Creates a runtime role bundle.
    pub fn new(
        work_model: SharedWorkModel<V>,
        operational_unit_factory: SharedOperationalUnitFactory<V>,
        unit_selection_policy: SharedUnitSelectionPolicy<V>,
        performance_model: SharedPerformanceModel<V>,
        variety_model: SharedVarietyModel<V>,
        algedonic_policy: SharedAlgedonicPolicy<V>,
    ) -> Self {
        Self {
            work_model,
            operational_unit_factory,
            unit_selection_policy,
            performance_model,
            variety_model,
            algedonic_policy,
        }
    }

    /// Returns the configured work model object.
    pub fn work_model(&self) -> SharedWorkModel<V> {
        Arc::clone(&self.work_model)
    }

    /// Returns the configured operational-unit factory object.
    pub fn operational_unit_factory(&self) -> SharedOperationalUnitFactory<V> {
        Arc::clone(&self.operational_unit_factory)
    }

    /// Returns the configured unit-selection policy object.
    pub fn unit_selection_policy(&self) -> SharedUnitSelectionPolicy<V> {
        Arc::clone(&self.unit_selection_policy)
    }

    /// Returns the configured performance model object.
    pub fn performance_model(&self) -> SharedPerformanceModel<V> {
        Arc::clone(&self.performance_model)
    }

    /// Returns the configured variety model object.
    pub fn variety_model(&self) -> SharedVarietyModel<V> {
        Arc::clone(&self.variety_model)
    }

    /// Returns the configured algedonic policy object.
    pub fn algedonic_policy(&self) -> SharedAlgedonicPolicy<V> {
        Arc::clone(&self.algedonic_policy)
    }
}

/// Handle for the System 1 surface owned by a typed runtime.
pub struct System1Handle<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    roles: System1RuntimeRoles<V>,
    ports: RuntimePorts<V>,
}

impl<V> Clone for System1Handle<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            roles: self.roles.clone(),
            ports: self.ports.clone(),
        }
    }
}

impl<V> System1Handle<V>
where
    V: ViableSystem,
{
    fn new(config: RuntimeConfig, roles: System1RuntimeRoles<V>, ports: RuntimePorts<V>) -> Self {
        Self {
            config,
            roles,
            ports,
        }
    }

    /// Returns the runtime instance identity.
    pub fn runtime_id(&self) -> &RuntimeId {
        &self.config.runtime_id
    }

    /// Returns the recursion path for this runtime instance.
    pub fn recursion_path(&self) -> &RecursionPath {
        &self.config.recursion_path
    }

    /// Returns the runtime-selected System 1 role bundle.
    pub fn roles(&self) -> &System1RuntimeRoles<V> {
        &self.roles
    }

    /// Builds a System 1 role context with runtime-scoped identity and ports.
    pub fn role_context(&self) -> RoleContext<V> {
        self.ports.role_context(
            self.config.runtime_id.clone(),
            self.config.recursion_path.clone(),
            SubsystemRole::System1,
        )
    }
}

/// Typed runtime handle returned by [`crate::VsmBuilder`].
pub struct VsmRuntime<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    readiness: RuntimeReadiness,
    lifecycle: Mutex<RuntimeState>,
    directory: Mutex<RuntimeDirectory>,
    ports: RuntimePorts<V>,
    system1_roles: System1RuntimeRoles<V>,
}

impl<V> VsmRuntime<V>
where
    V: ViableSystem,
{
    pub(crate) fn new(
        config: RuntimeConfig,
        ports: RuntimePorts<V>,
        system1_roles: System1RuntimeRoles<V>,
    ) -> Self {
        let readiness = RuntimeReadiness::new(vec![
            ReadinessCheck::new(
                ReadinessGate::Infrastructure,
                ReadinessStatus::Ready,
                "runtime ports and instance identity configured",
            ),
            ReadinessCheck::new(
                ReadinessGate::SubsystemActors,
                ReadinessStatus::NotApplicable,
                "typed actor adapters start in the System 1 vertical slice",
            ),
            ReadinessCheck::new(
                ReadinessGate::RoleImplementations,
                ReadinessStatus::Ready,
                "required System 1 role objects validated",
            ),
            ReadinessCheck::new(
                ReadinessGate::Subscriptions,
                ReadinessStatus::NotApplicable,
                "typed observer bus starts in a later milestone",
            ),
            ReadinessCheck::new(
                ReadinessGate::Persistence,
                ReadinessStatus::Ready,
                "state store port configured; no-op store starts fresh",
            ),
        ]);

        let mut directory = RuntimeDirectory::new();
        register_runtime_components(&mut directory, &config);

        Self {
            config,
            readiness,
            lifecycle: Mutex::new(RuntimeState::Ready),
            directory: Mutex::new(directory),
            ports,
            system1_roles,
        }
    }

    /// Returns the runtime instance configuration.
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Returns the runtime instance identity.
    pub fn runtime_id(&self) -> &RuntimeId {
        &self.config.runtime_id
    }

    /// Returns the recursion path for this runtime instance.
    pub fn recursion_path(&self) -> &RecursionPath {
        &self.config.recursion_path
    }

    /// Returns the current lifecycle state.
    pub fn state(&self) -> Result<RuntimeState, FrameworkError> {
        Ok(*self.lifecycle.lock().map_err(poisoned_lifecycle)?)
    }

    /// Returns true when the runtime has completed startup readiness checks.
    pub fn is_ready(&self) -> bool {
        self.readiness.is_ready()
    }

    /// Returns the latest readiness snapshot.
    pub fn readiness(&self) -> RuntimeReadiness {
        self.readiness.clone()
    }

    /// Returns a snapshot of the private component directory.
    pub fn directory_snapshot(&self) -> Result<RuntimeDirectorySnapshot, FrameworkError> {
        Ok(self
            .directory
            .lock()
            .map_err(poisoned_directory)?
            .snapshot())
    }

    /// Returns a System 1 handle scoped to this runtime instance.
    pub fn system1(&self) -> System1Handle<V> {
        System1Handle::new(
            self.config.clone(),
            self.system1_roles.clone(),
            self.ports.clone(),
        )
    }

    /// Builds a role context for any subsystem role.
    pub fn role_context(&self, role: SubsystemRole) -> RoleContext<V> {
        self.ports.role_context(
            self.config.runtime_id.clone(),
            self.config.recursion_path.clone(),
            role,
        )
    }

    /// Returns true after shutdown has been acknowledged.
    pub fn is_shutdown(&self) -> Result<bool, FrameworkError> {
        Ok(self.state()? == RuntimeState::Shutdown)
    }

    /// Shuts the typed runtime handle down and returns an acknowledgement.
    pub async fn shutdown(&self) -> Result<ShutdownReport, FrameworkError> {
        let mut lifecycle = self.lifecycle.lock().map_err(poisoned_lifecycle)?;
        let previous_state = *lifecycle;
        let already_shutdown = previous_state == RuntimeState::Shutdown;

        if !already_shutdown {
            *lifecycle = RuntimeState::ShuttingDown;
            self.directory
                .lock()
                .map_err(poisoned_directory)?
                .mark_all_shutdown();
            *lifecycle = RuntimeState::Shutdown;
        }

        Ok(ShutdownReport {
            runtime_id: self.config.runtime_id.clone(),
            previous_state,
            current_state: *lifecycle,
            already_shutdown,
        })
    }
}

fn register_runtime_components(directory: &mut RuntimeDirectory, config: &RuntimeConfig) {
    let runtime_id = &config.runtime_id;
    let recursion_path = &config.recursion_path;

    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::System1,
        "role-bundle",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::StateStore,
        "state-store",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::EventSink,
        "event-sink",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::ReportSink,
        "report-sink",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::Telemetry,
        "telemetry-sink",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::TemporalVariety,
        "typed-observer-bus",
        RuntimeComponentStatus::NotApplicable,
    );
}

fn poisoned_lifecycle(
    _: std::sync::PoisonError<std::sync::MutexGuard<'_, RuntimeState>>,
) -> FrameworkError {
    FrameworkError::Runtime {
        reason: "runtime lifecycle mutex poisoned".to_string(),
    }
}

fn poisoned_directory(
    _: std::sync::PoisonError<std::sync::MutexGuard<'_, RuntimeDirectory>>,
) -> FrameworkError {
    FrameworkError::Runtime {
        reason: "runtime directory mutex poisoned".to_string(),
    }
}
