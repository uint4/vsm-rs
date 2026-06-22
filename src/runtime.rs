//! Typed runtime handles for the trait-driven surface.

use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::config::RuntimeConfig;
use crate::error::FrameworkError;
use crate::kernel::event_bus::ObserverEventBus;
pub use crate::kernel::event_bus::{ObserverBusSnapshot, ObserverId, ObserverSubscription};
use crate::kernel::recursion::RecursionRuntime;
use crate::kernel::registry::RuntimeDirectory;
use crate::kernel::system1::System1Runtime;
use crate::kernel::system2::System2Runtime;
use crate::kernel::system3::System3Runtime;
use crate::kernel::system4::System4Runtime;
use crate::kernel::system5::System5Runtime;
use crate::kernel::variety::VarietyRuntime;
use crate::protocol::algedonic::{
    AlgedonicAcknowledgement, AlgedonicCycle, AlgedonicSeverity, AlgedonicSignalKind,
    AlgedonicSignalRecord, AlgedonicSnapshot,
};
use crate::protocol::recursion::{ChildRuntimeDescriptor, ChildRuntimeSnapshot, RecursionSnapshot};
use crate::protocol::system1::{
    Acknowledgement, AuditEvidence, AuditRequest, CapacitySnapshot, CoordinationView,
    ResourceShortageRequest, UnitCommand, UnitDescriptor, WorkRequest, WorkResponse, WorkResult,
};
use crate::protocol::system2::{
    CoordinationAcknowledgement, CoordinationCycle, CoordinationIntervention, System2Snapshot,
};
use crate::protocol::system3::{
    AuditResponse, DirectiveAcknowledgement, OperationalDirective, ResourceRequest,
    System3AuditRequest, System3ControlCycle, System3Snapshot,
};
use crate::protocol::system4::{
    EnvironmentSourceDescriptor, EnvironmentSourceStatus, EnvironmentalObservation,
    ForecastCalibration, System4IntelligenceCycle, System4Snapshot,
};
use crate::protocol::system5::{
    CrisisResponse, CrisisSeverity, CrisisSignal, DecisionEvidence, DecisionEvidenceKind,
    DecisionRequest, IdentityRecord, PolicyDirectiveAcknowledgement, System5DecisionCycle,
    System5Snapshot, ValueSet,
};
use crate::protocol::temporal::{TemporalAnalysis, TemporalSample, TemporalSnapshot};
use crate::protocol::variety::{
    VarietyAlgedonicTemporalSnapshot, VarietyCycle, VarietyEstimate, VarietyInterventionOutcome,
    VarietyObservation,
};
use crate::protocol::{
    CorrelationId, Priority, RecursionPath, RuntimeEvent, RuntimeId, SnapshotKey, SnapshotVersion,
    SubsystemRole, VsmAddress,
};
use crate::roles::RoleContext;
use crate::roles::{
    AlertSink, BoxOperationalUnit, Clock, EventSink, NoopAlertSink, NoopEventSink, NoopReportSink,
    NoopStateStore, NoopTelemetrySink, OperationalUnit, OperationalUnitFactory, ReportSink,
    SharedAlgedonicLifecyclePolicy, SharedAlgedonicPolicy, SharedAuditor, SharedCoordinationPolicy,
    SharedCrisisPolicy, SharedDecisionPolicy, SharedEnvironmentalSourceFactory, SharedForecaster,
    SharedIdentityProvider, SharedIntelligenceModel, SharedOperationalControlPolicy,
    SharedOperationalUnitFactory, SharedPerformanceModel, SharedRecursionTransducer,
    SharedResourceGovernance, SharedSignalInterpreter, SharedTemporalAnalysisPolicy,
    SharedUnitSelectionPolicy, SharedValuesEvaluator, SharedValuesProvider,
    SharedVarietyEngineeringPolicy, SharedVarietyModel, SharedWorkModel, StateStore, SystemClock,
    TelemetrySink, UnitRoleContext, ViableSystem,
};
use crate::shared::message::{ChannelKind, MessageKind, VsmMessage};

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

/// Per-unit admission limits enforced by the typed System 1 runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitAdmissionLimits {
    pub max_in_flight: Option<usize>,
}

impl UnitAdmissionLimits {
    /// Creates limits with no per-unit in-flight cap.
    pub fn unbounded() -> Self {
        Self {
            max_in_flight: None,
        }
    }

    /// Creates limits with a maximum number of in-flight work requests.
    pub fn max_in_flight(max_in_flight: usize) -> Self {
        Self {
            max_in_flight: Some(max_in_flight),
        }
    }
}

impl Default for UnitAdmissionLimits {
    fn default() -> Self {
        Self::unbounded()
    }
}

/// Snapshot behavior for one registered operational unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitSnapshotConfig {
    pub key: Option<SnapshotKey>,
    pub version: SnapshotVersion,
    pub save_on_unregister: bool,
}

impl UnitSnapshotConfig {
    /// Disables unit snapshot load/save for this registration.
    pub fn disabled() -> Self {
        Self {
            key: None,
            version: SnapshotVersion::INITIAL,
            save_on_unregister: false,
        }
    }

    /// Enables snapshot load/save for a specific key and version.
    pub fn keyed(key: SnapshotKey, version: SnapshotVersion) -> Self {
        Self {
            key: Some(key),
            version,
            save_on_unregister: true,
        }
    }
}

impl Default for UnitSnapshotConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

/// Typed registration for one System 1 operational unit.
pub struct UnitRegistration<V>
where
    V: ViableSystem,
{
    pub descriptor: UnitDescriptor<V>,
    pub factory: SharedOperationalUnitFactory<V>,
    pub admission: UnitAdmissionLimits,
    pub snapshot: UnitSnapshotConfig,
}

impl<V> Clone for UnitRegistration<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            descriptor: self.descriptor.clone(),
            factory: Arc::clone(&self.factory),
            admission: self.admission,
            snapshot: self.snapshot.clone(),
        }
    }
}

impl<V> UnitRegistration<V>
where
    V: ViableSystem,
{
    /// Creates a registration from a descriptor and restartable unit factory.
    pub fn new(descriptor: UnitDescriptor<V>, factory: SharedOperationalUnitFactory<V>) -> Self {
        Self {
            descriptor,
            factory,
            admission: UnitAdmissionLimits::default(),
            snapshot: UnitSnapshotConfig::default(),
        }
    }

    /// Sets per-unit admission limits.
    pub fn with_admission(mut self, admission: UnitAdmissionLimits) -> Self {
        self.admission = admission;
        self
    }

    /// Sets snapshot load/save behavior.
    pub fn with_snapshot(mut self, snapshot: UnitSnapshotConfig) -> Self {
        self.snapshot = snapshot;
        self
    }
}

/// Runtime view of one registered typed System 1 unit.
pub struct RegisteredUnit<V>
where
    V: ViableSystem,
{
    pub descriptor: UnitDescriptor<V>,
    pub in_flight: usize,
    pub admission: UnitAdmissionLimits,
    pub draining: bool,
}

impl<V> Clone for RegisteredUnit<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            descriptor: self.descriptor.clone(),
            in_flight: self.in_flight,
            admission: self.admission,
            draining: self.draining,
        }
    }
}

/// Shared child-runtime factory object.
pub type SharedChildRuntimeFactory<V> = Arc<dyn ChildRuntimeFactory<V>>;

/// Application-owned factory that starts one child VSM runtime.
#[ractor::async_trait]
pub trait ChildRuntimeFactory<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn start_child_runtime(
        &self,
        context: &RoleContext<V>,
        descriptor: &ChildRuntimeDescriptor<V>,
    ) -> Result<VsmRuntime<V>, FrameworkError>;
}

/// Registration request for one operational child runtime.
pub struct ChildRuntimeRegistration<V>
where
    V: ViableSystem,
{
    pub descriptor: ChildRuntimeDescriptor<V>,
    pub factory: SharedChildRuntimeFactory<V>,
    pub bridge_admission: UnitAdmissionLimits,
}

impl<V> Clone for ChildRuntimeRegistration<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            descriptor: self.descriptor.clone(),
            factory: Arc::clone(&self.factory),
            bridge_admission: self.bridge_admission,
        }
    }
}

impl<V> ChildRuntimeRegistration<V>
where
    V: ViableSystem,
{
    /// Creates a child runtime registration with unbounded bridge admission.
    pub fn new(
        descriptor: ChildRuntimeDescriptor<V>,
        factory: SharedChildRuntimeFactory<V>,
    ) -> Self {
        Self {
            descriptor,
            factory,
            bridge_admission: UnitAdmissionLimits::default(),
        }
    }

    /// Sets the admission limits for the parent-side child bridge unit.
    pub fn with_bridge_admission(mut self, admission: UnitAdmissionLimits) -> Self {
        self.bridge_admission = admission;
        self
    }
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

    pub(crate) fn event_sink(&self) -> Arc<dyn EventSink<V>> {
        Arc::clone(&self.event_sink)
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

    pub(crate) fn alert_sink(&self) -> Arc<dyn AlertSink> {
        Arc::clone(&self.alert_sink)
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

/// Runtime-selected System 2 role objects.
pub struct System2RuntimeRoles<V>
where
    V: ViableSystem,
{
    coordination_policy: SharedCoordinationPolicy<V>,
}

impl<V> Clone for System2RuntimeRoles<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            coordination_policy: Arc::clone(&self.coordination_policy),
        }
    }
}

impl<V> System2RuntimeRoles<V>
where
    V: ViableSystem,
{
    /// Creates a runtime role bundle.
    pub fn new(coordination_policy: SharedCoordinationPolicy<V>) -> Self {
        Self {
            coordination_policy,
        }
    }

    /// Returns the configured coordination policy object.
    pub fn coordination_policy(&self) -> SharedCoordinationPolicy<V> {
        Arc::clone(&self.coordination_policy)
    }
}

/// Runtime-selected System 3 role objects.
pub struct System3RuntimeRoles<V>
where
    V: ViableSystem,
{
    resource_governance: SharedResourceGovernance<V>,
    operational_control_policy: SharedOperationalControlPolicy<V>,
    auditor: SharedAuditor<V>,
}

impl<V> Clone for System3RuntimeRoles<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            resource_governance: Arc::clone(&self.resource_governance),
            operational_control_policy: Arc::clone(&self.operational_control_policy),
            auditor: Arc::clone(&self.auditor),
        }
    }
}

impl<V> System3RuntimeRoles<V>
where
    V: ViableSystem,
{
    /// Creates a runtime role bundle.
    pub fn new(
        resource_governance: SharedResourceGovernance<V>,
        operational_control_policy: SharedOperationalControlPolicy<V>,
        auditor: SharedAuditor<V>,
    ) -> Self {
        Self {
            resource_governance,
            operational_control_policy,
            auditor,
        }
    }

    /// Returns the configured resource governance object.
    pub fn resource_governance(&self) -> SharedResourceGovernance<V> {
        Arc::clone(&self.resource_governance)
    }

    /// Returns the configured operational control policy object.
    pub fn operational_control_policy(&self) -> SharedOperationalControlPolicy<V> {
        Arc::clone(&self.operational_control_policy)
    }

    /// Returns the configured System 3* auditor object.
    pub fn auditor(&self) -> SharedAuditor<V> {
        Arc::clone(&self.auditor)
    }
}

/// Runtime-selected System 4 role objects.
pub struct System4RuntimeRoles<V>
where
    V: ViableSystem,
{
    environmental_source_factory: SharedEnvironmentalSourceFactory<V>,
    signal_interpreter: SharedSignalInterpreter<V>,
    intelligence_model: SharedIntelligenceModel<V>,
    forecaster: SharedForecaster<V>,
}

impl<V> Clone for System4RuntimeRoles<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            environmental_source_factory: Arc::clone(&self.environmental_source_factory),
            signal_interpreter: Arc::clone(&self.signal_interpreter),
            intelligence_model: Arc::clone(&self.intelligence_model),
            forecaster: Arc::clone(&self.forecaster),
        }
    }
}

impl<V> System4RuntimeRoles<V>
where
    V: ViableSystem,
{
    /// Creates a runtime role bundle.
    pub fn new(
        environmental_source_factory: SharedEnvironmentalSourceFactory<V>,
        signal_interpreter: SharedSignalInterpreter<V>,
        intelligence_model: SharedIntelligenceModel<V>,
        forecaster: SharedForecaster<V>,
    ) -> Self {
        Self {
            environmental_source_factory,
            signal_interpreter,
            intelligence_model,
            forecaster,
        }
    }

    /// Returns the configured environmental source factory object.
    pub fn environmental_source_factory(&self) -> SharedEnvironmentalSourceFactory<V> {
        Arc::clone(&self.environmental_source_factory)
    }

    /// Returns the configured signal-interpreter object.
    pub fn signal_interpreter(&self) -> SharedSignalInterpreter<V> {
        Arc::clone(&self.signal_interpreter)
    }

    /// Returns the configured intelligence-model object.
    pub fn intelligence_model(&self) -> SharedIntelligenceModel<V> {
        Arc::clone(&self.intelligence_model)
    }

    /// Returns the configured forecaster object.
    pub fn forecaster(&self) -> SharedForecaster<V> {
        Arc::clone(&self.forecaster)
    }
}

/// Runtime-selected System 5 role objects.
pub struct System5RuntimeRoles<V>
where
    V: ViableSystem,
{
    identity_provider: SharedIdentityProvider<V>,
    values_provider: SharedValuesProvider<V>,
    values_evaluator: SharedValuesEvaluator<V>,
    decision_policy: SharedDecisionPolicy<V>,
    crisis_policy: SharedCrisisPolicy<V>,
}

impl<V> Clone for System5RuntimeRoles<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            identity_provider: Arc::clone(&self.identity_provider),
            values_provider: Arc::clone(&self.values_provider),
            values_evaluator: Arc::clone(&self.values_evaluator),
            decision_policy: Arc::clone(&self.decision_policy),
            crisis_policy: Arc::clone(&self.crisis_policy),
        }
    }
}

impl<V> System5RuntimeRoles<V>
where
    V: ViableSystem,
{
    /// Creates a runtime role bundle.
    pub fn new(
        identity_provider: SharedIdentityProvider<V>,
        values_provider: SharedValuesProvider<V>,
        values_evaluator: SharedValuesEvaluator<V>,
        decision_policy: SharedDecisionPolicy<V>,
        crisis_policy: SharedCrisisPolicy<V>,
    ) -> Self {
        Self {
            identity_provider,
            values_provider,
            values_evaluator,
            decision_policy,
            crisis_policy,
        }
    }

    /// Returns the configured identity provider.
    pub fn identity_provider(&self) -> SharedIdentityProvider<V> {
        Arc::clone(&self.identity_provider)
    }

    /// Returns the configured values provider.
    pub fn values_provider(&self) -> SharedValuesProvider<V> {
        Arc::clone(&self.values_provider)
    }

    /// Returns the configured values evaluator.
    pub fn values_evaluator(&self) -> SharedValuesEvaluator<V> {
        Arc::clone(&self.values_evaluator)
    }

    /// Returns the configured decision policy.
    pub fn decision_policy(&self) -> SharedDecisionPolicy<V> {
        Arc::clone(&self.decision_policy)
    }

    /// Returns the configured crisis policy.
    pub fn crisis_policy(&self) -> SharedCrisisPolicy<V> {
        Arc::clone(&self.crisis_policy)
    }
}

/// Runtime-selected variety, algedonic, and temporal strategy roles.
pub struct VarietyRuntimeRoles<V>
where
    V: ViableSystem,
{
    variety_engineering_policy: SharedVarietyEngineeringPolicy<V>,
    algedonic_lifecycle_policy: SharedAlgedonicLifecyclePolicy<V>,
    temporal_analysis_policy: SharedTemporalAnalysisPolicy<V>,
}

impl<V> Clone for VarietyRuntimeRoles<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            variety_engineering_policy: Arc::clone(&self.variety_engineering_policy),
            algedonic_lifecycle_policy: Arc::clone(&self.algedonic_lifecycle_policy),
            temporal_analysis_policy: Arc::clone(&self.temporal_analysis_policy),
        }
    }
}

impl<V> VarietyRuntimeRoles<V>
where
    V: ViableSystem,
{
    /// Creates a runtime role bundle.
    pub fn new(
        variety_engineering_policy: SharedVarietyEngineeringPolicy<V>,
        algedonic_lifecycle_policy: SharedAlgedonicLifecyclePolicy<V>,
        temporal_analysis_policy: SharedTemporalAnalysisPolicy<V>,
    ) -> Self {
        Self {
            variety_engineering_policy,
            algedonic_lifecycle_policy,
            temporal_analysis_policy,
        }
    }

    /// Returns the configured variety engineering policy.
    pub fn variety_engineering_policy(&self) -> SharedVarietyEngineeringPolicy<V> {
        Arc::clone(&self.variety_engineering_policy)
    }

    /// Returns the configured algedonic lifecycle policy.
    pub fn algedonic_lifecycle_policy(&self) -> SharedAlgedonicLifecyclePolicy<V> {
        Arc::clone(&self.algedonic_lifecycle_policy)
    }

    /// Returns the configured temporal analysis policy.
    pub fn temporal_analysis_policy(&self) -> SharedTemporalAnalysisPolicy<V> {
        Arc::clone(&self.temporal_analysis_policy)
    }
}

/// Runtime-selected operational-recursion role objects.
pub struct RecursionRuntimeRoles<V>
where
    V: ViableSystem,
{
    recursion_transducer: SharedRecursionTransducer<V>,
}

impl<V> Clone for RecursionRuntimeRoles<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            recursion_transducer: Arc::clone(&self.recursion_transducer),
        }
    }
}

impl<V> RecursionRuntimeRoles<V>
where
    V: ViableSystem,
{
    /// Creates a runtime recursion role bundle.
    pub fn new(recursion_transducer: SharedRecursionTransducer<V>) -> Self {
        Self {
            recursion_transducer,
        }
    }

    /// Returns the configured recursion transducer.
    pub fn recursion_transducer(&self) -> SharedRecursionTransducer<V> {
        Arc::clone(&self.recursion_transducer)
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
    runtime: Arc<System1Runtime<V>>,
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
            runtime: Arc::clone(&self.runtime),
        }
    }
}

impl<V> System1Handle<V>
where
    V: ViableSystem,
{
    fn new(
        config: RuntimeConfig,
        roles: System1RuntimeRoles<V>,
        ports: RuntimePorts<V>,
        runtime: Arc<System1Runtime<V>>,
    ) -> Self {
        Self {
            config,
            roles,
            ports,
            runtime,
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

    /// Registers one operational unit with an explicit restartable factory.
    pub async fn register_unit(
        &self,
        registration: UnitRegistration<V>,
    ) -> Result<UnitDescriptor<V>, FrameworkError> {
        self.runtime.register_unit(registration).await
    }

    /// Registers one operational unit using the runtime's default factory role.
    pub async fn register_descriptor(
        &self,
        descriptor: UnitDescriptor<V>,
    ) -> Result<UnitDescriptor<V>, FrameworkError> {
        self.register_unit(UnitRegistration::new(
            descriptor,
            self.roles.operational_unit_factory(),
        ))
        .await
    }

    /// Lists typed System 1 unit registrations owned by this runtime.
    pub fn list_units(&self) -> Result<Vec<RegisteredUnit<V>>, FrameworkError> {
        self.runtime.list_units()
    }

    /// Processes application work through the typed System 1 runtime.
    pub async fn process_work(&self, work: V::Work) -> WorkResult<V> {
        self.process(WorkRequest::new(work)).await
    }

    /// Processes a fully formed typed work request.
    pub async fn process(&self, request: WorkRequest<V>) -> WorkResult<V> {
        self.runtime.process(request).await
    }

    /// Processes a request and preserves framework metadata in the response.
    pub async fn process_response(&self, request: WorkRequest<V>) -> WorkResponse<V> {
        let metadata = request.metadata.clone();
        let result = self.process(request).await;
        WorkResponse { metadata, result }
    }

    /// Marks one unit as draining so it stops accepting new work.
    pub async fn drain_unit(&self, unit_id: &V::UnitId) -> Result<Acknowledgement, FrameworkError> {
        self.runtime.drain_unit(unit_id).await
    }

    /// Unregisters one unit and stops its actor adapter.
    pub async fn unregister_unit(
        &self,
        unit_id: &V::UnitId,
    ) -> Result<UnitDescriptor<V>, FrameworkError> {
        self.runtime.unregister_unit(unit_id).await
    }
}

/// Handle for the System 2 surface owned by a typed runtime.
pub struct System2Handle<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    roles: System2RuntimeRoles<V>,
    ports: RuntimePorts<V>,
    runtime: Arc<System2Runtime<V>>,
    system1_runtime: Arc<System1Runtime<V>>,
}

impl<V> Clone for System2Handle<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            roles: self.roles.clone(),
            ports: self.ports.clone(),
            runtime: Arc::clone(&self.runtime),
            system1_runtime: Arc::clone(&self.system1_runtime),
        }
    }
}

impl<V> System2Handle<V>
where
    V: ViableSystem,
{
    fn new(
        config: RuntimeConfig,
        roles: System2RuntimeRoles<V>,
        ports: RuntimePorts<V>,
        runtime: Arc<System2Runtime<V>>,
        system1_runtime: Arc<System1Runtime<V>>,
    ) -> Self {
        Self {
            config,
            roles,
            ports,
            runtime,
            system1_runtime,
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

    /// Returns the runtime-selected System 2 role bundle.
    pub fn roles(&self) -> &System2RuntimeRoles<V> {
        &self.roles
    }

    /// Builds a System 2 role context with runtime-scoped identity and ports.
    pub fn role_context(&self) -> RoleContext<V> {
        self.ports.role_context(
            self.config.runtime_id.clone(),
            self.config.recursion_path.clone(),
            SubsystemRole::System2,
        )
    }

    /// Runs one coordination cycle from explicit System 1 coordination views.
    pub async fn coordinate_views(
        &self,
        views: Vec<CoordinationView<V>>,
    ) -> Result<CoordinationCycle<V>, FrameworkError> {
        let mut cycle = self.runtime.coordinate_views(views).await?;
        let acknowledgements = self
            .deliver_interventions(cycle.interventions.clone())
            .await;
        let outcome = self
            .runtime
            .record_acknowledgements(acknowledgements)
            .await?;
        cycle.acknowledgements = outcome.acknowledgements;
        cycle.escalations = outcome.escalations;
        Ok(cycle)
    }

    /// Collects coordination views from typed System 1 units and runs System 2 policy.
    pub async fn coordinate_system1(&self) -> Result<CoordinationCycle<V>, FrameworkError> {
        let views = self.system1_runtime.coordination_views().await?;
        self.coordinate_views(views).await
    }

    /// Records externally supplied intervention acknowledgements.
    pub async fn acknowledge_interventions(
        &self,
        acknowledgements: Vec<CoordinationAcknowledgement<V>>,
    ) -> Result<CoordinationCycle<V>, FrameworkError> {
        self.runtime.record_acknowledgements(acknowledgements).await
    }

    /// Returns the current System 2 runtime snapshot.
    pub async fn snapshot(&self) -> Result<System2Snapshot<V>, FrameworkError> {
        self.runtime.snapshot().await
    }

    async fn deliver_interventions(
        &self,
        interventions: Vec<CoordinationIntervention<V>>,
    ) -> Vec<CoordinationAcknowledgement<V>> {
        let mut acknowledgements = Vec::new();

        for intervention in interventions {
            if !intervention.requires_ack {
                continue;
            }

            acknowledgements.extend(
                self.system1_runtime
                    .apply_coordination_intervention(intervention)
                    .await,
            );
        }

        acknowledgements
    }
}

/// Handle for the System 3 and System 3* surfaces owned by a typed runtime.
pub struct System3Handle<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    roles: System3RuntimeRoles<V>,
    ports: RuntimePorts<V>,
    runtime: Arc<System3Runtime<V>>,
    system1_runtime: Arc<System1Runtime<V>>,
}

impl<V> Clone for System3Handle<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            roles: self.roles.clone(),
            ports: self.ports.clone(),
            runtime: Arc::clone(&self.runtime),
            system1_runtime: Arc::clone(&self.system1_runtime),
        }
    }
}

impl<V> System3Handle<V>
where
    V: ViableSystem,
{
    fn new(
        config: RuntimeConfig,
        roles: System3RuntimeRoles<V>,
        ports: RuntimePorts<V>,
        runtime: Arc<System3Runtime<V>>,
        system1_runtime: Arc<System1Runtime<V>>,
    ) -> Self {
        Self {
            config,
            roles,
            ports,
            runtime,
            system1_runtime,
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

    /// Returns the runtime-selected System 3 role bundle.
    pub fn roles(&self) -> &System3RuntimeRoles<V> {
        &self.roles
    }

    /// Builds a System 3 role context with runtime-scoped identity and ports.
    pub fn role_context(&self) -> RoleContext<V> {
        self.ports.role_context(
            self.config.runtime_id.clone(),
            self.config.recursion_path.clone(),
            SubsystemRole::System3,
        )
    }

    /// Builds a System 3* audit role context with runtime-scoped identity and ports.
    pub fn audit_role_context(&self) -> RoleContext<V> {
        self.ports.role_context(
            self.config.runtime_id.clone(),
            self.config.recursion_path.clone(),
            SubsystemRole::System3Star,
        )
    }

    /// Runs one resource-governance and operational-control cycle.
    pub async fn govern_resources(
        &self,
        requests: Vec<ResourceRequest<V>>,
        performance: Vec<crate::protocol::system1::PerformanceObservation<V>>,
    ) -> Result<System3ControlCycle<V>, FrameworkError> {
        let mut cycle = self.runtime.govern_resources(requests, performance).await?;
        let acknowledgements = self.deliver_directives(cycle.directives.clone()).await;
        let outcome = self
            .runtime
            .record_directive_acknowledgements(acknowledgements)
            .await?;
        cycle.directive_acknowledgements = outcome.directive_acknowledgements;
        cycle.summaries.extend(outcome.summaries);
        Ok(cycle)
    }

    /// Converts a System 1 resource-shortage event into a System 3 governance cycle.
    pub async fn handle_resource_shortage(
        &self,
        shortage: ResourceShortageRequest<V>,
    ) -> Result<System3ControlCycle<V>, FrameworkError> {
        self.govern_resources(vec![ResourceRequest::from_shortage(shortage)], Vec::new())
            .await
    }

    /// Records externally supplied directive acknowledgements.
    pub async fn acknowledge_directives(
        &self,
        acknowledgements: Vec<DirectiveAcknowledgement<V>>,
    ) -> Result<System3ControlCycle<V>, FrameworkError> {
        self.runtime
            .record_directive_acknowledgements(acknowledgements)
            .await
    }

    /// Runs System 3* audit using evidence collected directly from System 1 units.
    pub async fn audit_system1(
        &self,
        request: System3AuditRequest<V>,
    ) -> Result<AuditResponse<V>, FrameworkError> {
        let system1_request = AuditRequest {
            metadata: request.metadata.child(),
            scope: request.scope.clone(),
        };
        let evidence = self.system1_runtime.audit_evidence(system1_request).await?;
        let evidence = apply_audit_boundary(&request, evidence);
        self.runtime.perform_audit(request, evidence).await
    }

    /// Runs System 3* audit with evidence supplied by the caller.
    pub async fn audit_with_evidence(
        &self,
        request: System3AuditRequest<V>,
        evidence: Vec<AuditEvidence<V>>,
    ) -> Result<AuditResponse<V>, FrameworkError> {
        let evidence = apply_audit_boundary(&request, evidence);
        self.runtime.perform_audit(request, evidence).await
    }

    /// Returns the current System 3 runtime snapshot.
    pub async fn snapshot(&self) -> Result<System3Snapshot<V>, FrameworkError> {
        self.runtime.snapshot().await
    }

    async fn deliver_directives(
        &self,
        directives: Vec<OperationalDirective<V>>,
    ) -> Vec<DirectiveAcknowledgement<V>> {
        let mut acknowledgements = Vec::new();

        for directive in directives {
            if !directive.requires_ack {
                continue;
            }

            acknowledgements.extend(
                self.system1_runtime
                    .apply_operational_directive(directive)
                    .await,
            );
        }

        acknowledgements
    }
}

/// Handle for the System 4 environmental intelligence surface.
pub struct System4Handle<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    roles: System4RuntimeRoles<V>,
    ports: RuntimePorts<V>,
    runtime: Arc<System4Runtime<V>>,
    system3_runtime: Arc<System3Runtime<V>>,
}

impl<V> Clone for System4Handle<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            roles: self.roles.clone(),
            ports: self.ports.clone(),
            runtime: Arc::clone(&self.runtime),
            system3_runtime: Arc::clone(&self.system3_runtime),
        }
    }
}

impl<V> System4Handle<V>
where
    V: ViableSystem,
{
    fn new(
        config: RuntimeConfig,
        roles: System4RuntimeRoles<V>,
        ports: RuntimePorts<V>,
        runtime: Arc<System4Runtime<V>>,
        system3_runtime: Arc<System3Runtime<V>>,
    ) -> Self {
        Self {
            config,
            roles,
            ports,
            runtime,
            system3_runtime,
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

    /// Returns the runtime-selected System 4 role bundle.
    pub fn roles(&self) -> &System4RuntimeRoles<V> {
        &self.roles
    }

    /// Builds a System 4 role context with runtime-scoped identity and ports.
    pub fn role_context(&self) -> RoleContext<V> {
        self.ports.role_context(
            self.config.runtime_id.clone(),
            self.config.recursion_path.clone(),
            SubsystemRole::System4,
        )
    }

    /// Registers one environmental source for the typed intelligence pipeline.
    pub async fn register_source(
        &self,
        descriptor: EnvironmentSourceDescriptor,
    ) -> Result<EnvironmentSourceStatus, FrameworkError> {
        self.runtime.register_source(descriptor).await
    }

    /// Lists registered environmental sources.
    pub async fn list_sources(&self) -> Result<Vec<EnvironmentSourceStatus>, FrameworkError> {
        self.runtime.list_sources().await
    }

    /// Polls all registered sources and returns normalized observations.
    pub async fn collect_observations(
        &self,
    ) -> Result<Vec<EnvironmentalObservation>, FrameworkError> {
        self.runtime.collect_observations().await
    }

    /// Runs one intelligence cycle and annotates proposals with System 3 feasibility context.
    pub async fn run_intelligence_cycle(&self) -> Result<System4IntelligenceCycle, FrameworkError> {
        let mut cycle = self.runtime.run_cycle().await?;
        self.attach_system3_feasibility(&mut cycle).await;
        self.runtime
            .record_proposals(cycle.proposals.clone())
            .await?;
        Ok(cycle)
    }

    /// Compares forecasts with actual observations for calibration.
    pub async fn calibrate_forecasts(
        &self,
        actuals: Vec<EnvironmentalObservation>,
    ) -> Result<Vec<ForecastCalibration>, FrameworkError> {
        self.runtime.calibrate(actuals).await
    }

    /// Returns the current typed System 4 runtime snapshot.
    pub async fn snapshot(&self) -> Result<System4Snapshot, FrameworkError> {
        self.runtime.snapshot().await
    }

    async fn attach_system3_feasibility(&self, cycle: &mut System4IntelligenceCycle) {
        if cycle.proposals.is_empty() {
            return;
        }

        let context = self.role_context();
        let feasibility = match self.system3_runtime.snapshot().await {
            Ok(snapshot) => crate::kernel::system4::feasibility_from_system3_snapshot(
                &context,
                Some(&snapshot),
                None,
            ),
            Err(err) => crate::kernel::system4::feasibility_from_system3_snapshot(
                &context,
                None,
                Some(format!("System 3 feasibility unavailable: {err}")),
            ),
        };
        let destination = VsmAddress::new(
            self.config.runtime_id.clone(),
            self.config.recursion_path.clone(),
            SubsystemRole::System5,
        );

        for proposal in &mut cycle.proposals {
            if proposal.feasibility.is_none() {
                proposal.feasibility = Some(feasibility.clone());
            }
            if proposal.destination.is_none() {
                proposal.destination = Some(destination.clone());
            }
        }
    }
}

/// Handle for the System 5 policy surface.
pub struct System5Handle<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    roles: System5RuntimeRoles<V>,
    ports: RuntimePorts<V>,
    runtime: Arc<System5Runtime<V>>,
    system3_runtime: Arc<System3Runtime<V>>,
    system4_runtime: Arc<System4Runtime<V>>,
}

impl<V> Clone for System5Handle<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            roles: self.roles.clone(),
            ports: self.ports.clone(),
            runtime: Arc::clone(&self.runtime),
            system3_runtime: Arc::clone(&self.system3_runtime),
            system4_runtime: Arc::clone(&self.system4_runtime),
        }
    }
}

impl<V> System5Handle<V>
where
    V: ViableSystem,
{
    fn new(
        config: RuntimeConfig,
        roles: System5RuntimeRoles<V>,
        ports: RuntimePorts<V>,
        runtime: Arc<System5Runtime<V>>,
        system3_runtime: Arc<System3Runtime<V>>,
        system4_runtime: Arc<System4Runtime<V>>,
    ) -> Self {
        Self {
            config,
            roles,
            ports,
            runtime,
            system3_runtime,
            system4_runtime,
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

    /// Returns the runtime-selected System 5 role bundle.
    pub fn roles(&self) -> &System5RuntimeRoles<V> {
        &self.roles
    }

    /// Builds a System 5 role context with runtime-scoped identity and ports.
    pub fn role_context(&self) -> RoleContext<V> {
        self.ports.role_context(
            self.config.runtime_id.clone(),
            self.config.recursion_path.clone(),
            SubsystemRole::System5,
        )
    }

    /// Reads the current identity provider output.
    pub async fn identity(&self) -> Result<IdentityRecord, FrameworkError> {
        self.runtime.identity().await
    }

    /// Reads the current values provider output.
    pub async fn values(&self) -> Result<ValueSet, FrameworkError> {
        self.runtime.values().await
    }

    /// Runs one decision cycle with current System 3 and System 4 context attached.
    pub async fn decide(
        &self,
        mut request: DecisionRequest<V>,
    ) -> Result<System5DecisionCycle<V>, FrameworkError> {
        self.attach_system_context(&mut request).await;
        self.runtime.decide(request).await
    }

    /// Handles a crisis signal through the typed System 5 crisis policy.
    pub async fn handle_crisis(
        &self,
        signal: CrisisSignal,
    ) -> Result<CrisisResponse<V>, FrameworkError> {
        self.runtime.handle_crisis(signal).await
    }

    /// Handles an algedonic crisis signal through the typed System 5 crisis policy.
    pub async fn handle_algedonic_signal(
        &self,
        mut signal: CrisisSignal,
    ) -> Result<CrisisResponse<V>, FrameworkError> {
        if signal.source.is_none() {
            signal.source = Some(VsmAddress::new(
                self.config.runtime_id.clone(),
                self.config.recursion_path.clone(),
                SubsystemRole::Algedonic,
            ));
        }
        self.handle_crisis(signal).await
    }

    /// Records externally supplied directive acknowledgements.
    pub async fn acknowledge_directives(
        &self,
        acknowledgements: Vec<PolicyDirectiveAcknowledgement<V>>,
    ) -> Result<System5Snapshot<V>, FrameworkError> {
        self.runtime.acknowledge_directives(acknowledgements).await
    }

    /// Returns the current typed System 5 runtime snapshot.
    pub async fn snapshot(&self) -> Result<System5Snapshot<V>, FrameworkError> {
        self.runtime.snapshot().await
    }

    async fn attach_system_context(&self, request: &mut DecisionRequest<V>) {
        if let Ok(snapshot) = self.system3_runtime.snapshot().await {
            request
                .operational_summaries
                .extend(snapshot.summaries.iter().cloned());
        }

        if let Ok(snapshot) = self.system4_runtime.snapshot().await {
            request
                .adaptation_proposals
                .extend(snapshot.proposals.iter().cloned());
        }
    }
}

/// Handle for the variety, algedonic, and temporal lifecycle surface.
pub struct VarietyHandle<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    roles: VarietyRuntimeRoles<V>,
    ports: RuntimePorts<V>,
    runtime: Arc<VarietyRuntime<V>>,
    system5_runtime: Arc<System5Runtime<V>>,
}

impl<V> Clone for VarietyHandle<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            roles: self.roles.clone(),
            ports: self.ports.clone(),
            runtime: Arc::clone(&self.runtime),
            system5_runtime: Arc::clone(&self.system5_runtime),
        }
    }
}

impl<V> VarietyHandle<V>
where
    V: ViableSystem,
{
    fn new(
        config: RuntimeConfig,
        roles: VarietyRuntimeRoles<V>,
        ports: RuntimePorts<V>,
        runtime: Arc<VarietyRuntime<V>>,
        system5_runtime: Arc<System5Runtime<V>>,
    ) -> Self {
        Self {
            config,
            roles,
            ports,
            runtime,
            system5_runtime,
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

    /// Returns the runtime-selected variety, algedonic, and temporal role bundle.
    pub fn roles(&self) -> &VarietyRuntimeRoles<V> {
        &self.roles
    }

    /// Builds a variety role context with runtime-scoped identity and ports.
    pub fn role_context(&self) -> RoleContext<V> {
        self.ports.role_context(
            self.config.runtime_id.clone(),
            self.config.recursion_path.clone(),
            SubsystemRole::Variety,
        )
    }

    /// Records one variety observation and runs the configured engineering policy.
    pub async fn record_variety(
        &self,
        observation: VarietyObservation<V>,
    ) -> Result<VarietyCycle<V>, FrameworkError> {
        self.runtime.record_variety(observation).await
    }

    /// Records an estimate as a variety observation.
    pub async fn record_variety_estimate(
        &self,
        estimate: VarietyEstimate,
    ) -> Result<VarietyCycle<V>, FrameworkError> {
        self.record_variety(VarietyObservation::new(estimate)).await
    }

    /// Records externally supplied variety intervention outcomes.
    pub async fn record_variety_outcomes(
        &self,
        outcomes: Vec<VarietyInterventionOutcome<V>>,
    ) -> Result<VarietyAlgedonicTemporalSnapshot<V>, FrameworkError> {
        self.runtime.record_variety_outcomes(outcomes).await
    }

    /// Handles one typed algedonic signal and dispatches urgent signals to System 5.
    pub async fn handle_algedonic_signal(
        &self,
        signal: AlgedonicSignalRecord<V>,
    ) -> Result<AlgedonicCycle<V>, FrameworkError> {
        let mut cycle = self.runtime.process_algedonic(signal).await?;

        if cycle.signal.requires_system5_dispatch() {
            let response = self
                .system5_runtime
                .handle_crisis(crisis_signal_from_algedonic(
                    &cycle.signal,
                    &self.config.runtime_id,
                    &self.config.recursion_path,
                ))
                .await?;
            cycle = self
                .runtime
                .record_system5_dispatch(cycle.signal.signal_id.clone(), response)
                .await?;
        }

        Ok(cycle)
    }

    /// Converts and handles a legacy brokered algedonic message.
    pub async fn handle_legacy_algedonic_message(
        &self,
        message: VsmMessage,
    ) -> Result<AlgedonicCycle<V>, FrameworkError> {
        let signal = legacy_message_to_algedonic_record(message)?;
        self.handle_algedonic_signal(signal).await
    }

    /// Converts and handles an advanced algedonic actor signal.
    pub async fn handle_advanced_algedonic_signal(
        &self,
        signal: crate::channels::algedonic::signals::AlgedonicSignal,
    ) -> Result<AlgedonicCycle<V>, FrameworkError> {
        self.handle_algedonic_signal(advanced_signal_to_algedonic_record(signal))
            .await
    }

    /// Records externally supplied algedonic acknowledgements.
    pub async fn acknowledge_algedonic(
        &self,
        acknowledgements: Vec<AlgedonicAcknowledgement<V>>,
    ) -> Result<VarietyAlgedonicTemporalSnapshot<V>, FrameworkError> {
        self.runtime.acknowledge_algedonic(acknowledgements).await
    }

    /// Expires overdue algedonic signals and records escalation lifecycle entries.
    pub async fn expire_algedonic(
        &self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<crate::protocol::algedonic::AlgedonicEscalation<V>>, FrameworkError> {
        self.runtime.expire_algedonic(now).await
    }

    /// Records one temporal sample and refreshes generic aggregates.
    pub async fn record_temporal_sample(
        &self,
        sample: TemporalSample,
    ) -> Result<TemporalSnapshot, FrameworkError> {
        self.runtime.record_temporal_sample(sample).await
    }

    /// Runs the configured temporal analysis policy over current aggregates.
    pub async fn analyze_temporal(&self) -> Result<TemporalAnalysis, FrameworkError> {
        self.runtime.analyze_temporal().await
    }

    /// Returns the current retained variety, algedonic, and temporal lifecycle snapshot.
    pub async fn snapshot(&self) -> Result<VarietyAlgedonicTemporalSnapshot<V>, FrameworkError> {
        self.runtime.snapshot().await
    }

    /// Returns the retained algedonic lifecycle snapshot.
    pub async fn algedonic_snapshot(&self) -> Result<AlgedonicSnapshot<V>, FrameworkError> {
        Ok(self.snapshot().await?.algedonic)
    }
}

/// Handle for the operational-recursion surface owned by a typed runtime.
struct RecursionParentRuntimes<V>
where
    V: ViableSystem,
{
    system1: Arc<System1Runtime<V>>,
    system3: Arc<System3Runtime<V>>,
    variety: Arc<VarietyRuntime<V>>,
    system5: Arc<System5Runtime<V>>,
}

impl<V> Clone for RecursionParentRuntimes<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            system1: Arc::clone(&self.system1),
            system3: Arc::clone(&self.system3),
            variety: Arc::clone(&self.variety),
            system5: Arc::clone(&self.system5),
        }
    }
}

pub struct RecursionHandle<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    roles: RecursionRuntimeRoles<V>,
    ports: RuntimePorts<V>,
    runtime: Arc<RecursionRuntime<V>>,
    parent: RecursionParentRuntimes<V>,
}

impl<V> Clone for RecursionHandle<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            roles: self.roles.clone(),
            ports: self.ports.clone(),
            runtime: Arc::clone(&self.runtime),
            parent: self.parent.clone(),
        }
    }
}

impl<V> RecursionHandle<V>
where
    V: ViableSystem,
{
    fn new(
        config: RuntimeConfig,
        roles: RecursionRuntimeRoles<V>,
        ports: RuntimePorts<V>,
        runtime: Arc<RecursionRuntime<V>>,
        parent: RecursionParentRuntimes<V>,
    ) -> Self {
        Self {
            config,
            roles,
            ports,
            runtime,
            parent,
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

    /// Returns the runtime-selected recursion role bundle.
    pub fn roles(&self) -> &RecursionRuntimeRoles<V> {
        &self.roles
    }

    /// Builds a recursion role context with runtime-scoped identity and ports.
    pub fn role_context(&self) -> RoleContext<V> {
        self.ports.role_context(
            self.config.runtime_id.clone(),
            self.config.recursion_path.clone(),
            SubsystemRole::Custom("recursion".to_string()),
        )
    }

    /// Starts and registers a child runtime as a parent-side System 1 bridge unit.
    pub async fn register_child_runtime(
        &self,
        registration: ChildRuntimeRegistration<V>,
    ) -> Result<ChildRuntimeSnapshot<V>, FrameworkError> {
        let child_id = registration.descriptor.child_id.clone();
        let descriptor = registration.descriptor.unit_descriptor.clone();
        let capacity = registration.descriptor.capacity.clone();
        let bridge_admission = registration.bridge_admission;
        let snapshot = self.runtime.register_child(registration).await?;
        let bridge_factory = Arc::new(ChildRuntimeUnitFactory::new(
            Arc::clone(&self.runtime),
            child_id,
            capacity,
        ));
        let bridge_registration =
            UnitRegistration::new(descriptor, bridge_factory).with_admission(bridge_admission);
        self.parent
            .system1
            .register_unit(bridge_registration)
            .await?;
        Ok(snapshot)
    }

    /// Lists child runtimes currently retained by the recursion manager.
    pub fn list_children(&self) -> Result<Vec<ChildRuntimeSnapshot<V>>, FrameworkError> {
        self.runtime.list_children()
    }

    /// Returns the private component directory snapshot for one child runtime.
    pub fn child_directory_snapshot(
        &self,
        child_id: &str,
    ) -> Result<RuntimeDirectorySnapshot, FrameworkError> {
        self.runtime.child_runtime(child_id)?.directory_snapshot()
    }

    /// Delegates a typed work request directly to a registered child runtime.
    pub async fn delegate_work(
        &self,
        child_id: &str,
        request: WorkRequest<V>,
    ) -> Result<WorkResponse<V>, FrameworkError> {
        self.runtime.delegate_work(child_id, request).await
    }

    /// Escalates a child resource shortage to this parent runtime's System 3.
    pub async fn escalate_resource_shortage(
        &self,
        child_id: &str,
        shortage: ResourceShortageRequest<V>,
    ) -> Result<System3ControlCycle<V>, FrameworkError> {
        let request = self
            .runtime
            .translate_resource_escalation(child_id, shortage)
            .await?;
        let mut cycle = self
            .parent
            .system3
            .govern_resources(vec![request], Vec::new())
            .await?;
        let mut acknowledgements = Vec::new();
        for directive in cycle.directives.clone() {
            if !directive.requires_ack {
                continue;
            }
            acknowledgements.extend(
                self.parent
                    .system1
                    .apply_operational_directive(directive)
                    .await,
            );
        }
        let outcome = self
            .parent
            .system3
            .record_directive_acknowledgements(acknowledgements)
            .await?;
        cycle.directive_acknowledgements = outcome.directive_acknowledgements;
        cycle.summaries.extend(outcome.summaries);
        Ok(cycle)
    }

    /// Escalates a child algedonic signal to this parent runtime.
    pub async fn escalate_algedonic_signal(
        &self,
        child_id: &str,
        signal: AlgedonicSignalRecord<V>,
    ) -> Result<AlgedonicCycle<V>, FrameworkError> {
        let parent_signal = self
            .runtime
            .translate_algedonic_escalation(child_id, signal)
            .await?;
        let mut cycle = self.parent.variety.process_algedonic(parent_signal).await?;

        if cycle.signal.requires_system5_dispatch() {
            let response = self
                .parent
                .system5
                .handle_crisis(crisis_signal_from_algedonic(
                    &cycle.signal,
                    &self.config.runtime_id,
                    &self.config.recursion_path,
                ))
                .await?;
            cycle = self
                .parent
                .variety
                .record_system5_dispatch(cycle.signal.signal_id.clone(), response)
                .await?;
        }

        Ok(cycle)
    }

    /// Transduces and delivers a parent policy directive to a child runtime.
    pub async fn transduce_policy_directive(
        &self,
        child_id: &str,
        directive: OperationalDirective<V>,
    ) -> Result<Vec<DirectiveAcknowledgement<V>>, FrameworkError> {
        let Some(child_directive) = self
            .runtime
            .transduce_policy_directive(child_id, directive)
            .await?
        else {
            return Ok(Vec::new());
        };
        let child_runtime = self.runtime.child_runtime(child_id)?;
        Ok(child_runtime
            .system1_runtime
            .apply_operational_directive(child_directive)
            .await)
    }

    /// Summarizes a child System 4 intelligence cycle at the parent boundary.
    pub async fn record_intelligence_summary(
        &self,
        child_id: &str,
        cycle: &System4IntelligenceCycle,
    ) -> Result<crate::protocol::recursion::RecursionIntelligenceSummary, FrameworkError> {
        self.runtime
            .record_intelligence_summary(child_id, cycle)
            .await
    }

    /// Returns the current recursion manager snapshot.
    pub fn snapshot(&self) -> Result<RecursionSnapshot<V>, FrameworkError> {
        self.runtime.snapshot()
    }
}

struct ChildRuntimeUnitFactory<V>
where
    V: ViableSystem,
{
    recursion_runtime: Arc<RecursionRuntime<V>>,
    child_id: String,
    capacity: CapacitySnapshot,
}

impl<V> ChildRuntimeUnitFactory<V>
where
    V: ViableSystem,
{
    fn new(
        recursion_runtime: Arc<RecursionRuntime<V>>,
        child_id: String,
        capacity: CapacitySnapshot,
    ) -> Self {
        Self {
            recursion_runtime,
            child_id,
            capacity,
        }
    }
}

#[ractor::async_trait]
impl<V> OperationalUnitFactory<V> for ChildRuntimeUnitFactory<V>
where
    V: ViableSystem,
{
    async fn create_unit(
        &self,
        context: &RoleContext<V>,
        descriptor: &UnitDescriptor<V>,
    ) -> Result<BoxOperationalUnit<V>, FrameworkError> {
        let _ = context;
        Ok(Box::new(ChildRuntimeOperationalUnit {
            recursion_runtime: Arc::clone(&self.recursion_runtime),
            child_id: self.child_id.clone(),
            descriptor: descriptor.clone(),
            fallback_capacity: self.capacity.clone(),
        }))
    }
}

struct ChildRuntimeOperationalUnit<V>
where
    V: ViableSystem,
{
    recursion_runtime: Arc<RecursionRuntime<V>>,
    child_id: String,
    descriptor: UnitDescriptor<V>,
    fallback_capacity: CapacitySnapshot,
}

#[ractor::async_trait]
impl<V> OperationalUnit<V> for ChildRuntimeOperationalUnit<V>
where
    V: ViableSystem,
{
    async fn descriptor(
        &mut self,
        context: &UnitRoleContext<V>,
    ) -> Result<UnitDescriptor<V>, FrameworkError> {
        let _ = context;
        Ok(self.descriptor.clone())
    }

    async fn capacity(
        &mut self,
        context: &UnitRoleContext<V>,
    ) -> Result<CapacitySnapshot, FrameworkError> {
        let _ = context;
        self.recursion_runtime
            .child_capacity(&self.child_id)
            .await
            .or_else(|_| Ok(self.fallback_capacity.clone()))
    }

    async fn handle_work(
        &mut self,
        context: &UnitRoleContext<V>,
        request: WorkRequest<V>,
    ) -> WorkResult<V> {
        let _ = context;
        match self
            .recursion_runtime
            .delegate_work(&self.child_id, request)
            .await
        {
            Ok(response) => response.result,
            Err(error) => Err(error.into()),
        }
    }

    async fn handle_command(
        &mut self,
        context: &UnitRoleContext<V>,
        command: UnitCommand<V>,
    ) -> Result<Acknowledgement, FrameworkError> {
        let _ = context;
        Ok(Acknowledgement::accepted(command.metadata.child()))
    }

    async fn coordination_view(
        &mut self,
        context: &UnitRoleContext<V>,
    ) -> Result<CoordinationView<V>, FrameworkError> {
        Ok(CoordinationView {
            metadata: context.metadata().clone(),
            unit_id: context.unit_id().clone(),
            capabilities: self.descriptor.capabilities.clone(),
            capacity: self.capacity(context).await?,
            snapshot_version: None,
        })
    }

    async fn audit_evidence(
        &mut self,
        context: &UnitRoleContext<V>,
        request: AuditRequest<V>,
    ) -> Result<AuditEvidence<V>, FrameworkError> {
        Ok(AuditEvidence {
            metadata: request.metadata.child(),
            unit_id: context.unit_id().clone(),
            capabilities: self.descriptor.capabilities.clone(),
            capacity: self.capacity(context).await?,
            snapshot_version: None,
            snapshot: None,
        })
    }
}

fn legacy_message_to_algedonic_record<V>(
    message: VsmMessage,
) -> Result<AlgedonicSignalRecord<V>, FrameworkError>
where
    V: ViableSystem,
{
    if message.channel != ChannelKind::Algedonic {
        return Err(FrameworkError::InvalidProtocol {
            reason: format!(
                "expected algedonic channel, received {}",
                message.channel.as_str()
            ),
        });
    }

    let kind = legacy_message_kind_to_algedonic_kind(&message.kind, &message.payload);
    let severity = legacy_payload_severity(&message.kind, &message.payload);
    let reason = first_string(
        &message.payload,
        &["reason", "message", "summary", "description"],
    )
    .unwrap_or_else(|| format!("{:?} from {}", message.kind, message.from.subscriber_id()));
    let priority = first_number(&message.payload, &["priority", "urgency"])
        .unwrap_or_else(|| severity.score())
        .clamp(0.0, 1.0);

    let mut signal = AlgedonicSignalRecord::new(kind, severity, reason).with_priority(priority);
    signal.signal_id = message.id;
    signal.source_label = Some(message.from.subscriber_id().to_string());
    signal.proposed_at = message.timestamp;
    signal.metadata.source = Some(VsmAddress::new(
        RuntimeId::from_string("legacy-broker"),
        RecursionPath::root(),
        legacy_system_id_to_role(message.from),
    ));
    signal.metadata.destination = Some(VsmAddress::new(
        RuntimeId::from_string("legacy-broker"),
        RecursionPath::root(),
        legacy_system_id_to_role(message.to),
    ));
    if let Some(correlation_id) = message.correlation_id {
        signal.metadata.correlation_id = CorrelationId::from_string(correlation_id);
    }
    signal.metadata.priority = priority_to_protocol_priority(severity, priority);
    signal.details = value_details(&message.payload);
    if let Some(dedupe_key) = first_string(&message.payload, &["dedupe_key", "dedupe"]) {
        signal.dedupe_key = Some(dedupe_key);
    }

    Ok(signal)
}

fn advanced_signal_to_algedonic_record<V>(
    signal: crate::channels::algedonic::signals::AlgedonicSignal,
) -> AlgedonicSignalRecord<V>
where
    V: ViableSystem,
{
    let kind = match signal.kind {
        crate::channels::algedonic::signals::SignalKind::Pain => AlgedonicSignalKind::Pain,
        crate::channels::algedonic::signals::SignalKind::Pleasure => AlgedonicSignalKind::Pleasure,
        crate::channels::algedonic::signals::SignalKind::Anomaly => AlgedonicSignalKind::Anomaly,
        crate::channels::algedonic::signals::SignalKind::Opportunity => {
            AlgedonicSignalKind::Opportunity
        }
        crate::channels::algedonic::signals::SignalKind::Emergency => {
            AlgedonicSignalKind::Emergency
        }
    };
    let severity = match signal.severity {
        crate::channels::algedonic::signals::Severity::Low => AlgedonicSeverity::Low,
        crate::channels::algedonic::signals::Severity::Medium => AlgedonicSeverity::Medium,
        crate::channels::algedonic::signals::Severity::High => AlgedonicSeverity::High,
        crate::channels::algedonic::signals::Severity::Critical => AlgedonicSeverity::Critical,
    };
    let reason = first_string(
        &signal.data,
        &["reason", "message", "summary", "description"],
    )
    .unwrap_or_else(|| {
        format!(
            "{:?} signal from {} with priority {:.2}",
            signal.kind, signal.source, signal.priority
        )
    });

    let mut record =
        AlgedonicSignalRecord::new(kind, severity, reason).with_priority(signal.priority);
    record.signal_id = signal.id;
    record.source_label = Some(signal.source);
    record.proposed_at = signal.timestamp;
    record.details = value_details(&signal.data);
    record
        .details
        .insert("urgency".to_string(), format!("{:.3}", signal.urgency));
    record
}

fn crisis_signal_from_algedonic<V>(
    signal: &AlgedonicSignalRecord<V>,
    runtime_id: &RuntimeId,
    recursion_path: &RecursionPath,
) -> CrisisSignal
where
    V: ViableSystem,
{
    let mut crisis = CrisisSignal::new(
        algedonic_to_crisis_severity(signal.severity),
        &signal.reason,
    )
    .from_source(signal.source.clone().unwrap_or_else(|| {
        VsmAddress::new(
            runtime_id.clone(),
            recursion_path.clone(),
            SubsystemRole::Algedonic,
        )
    }))
    .with_evidence(DecisionEvidence::new(
        DecisionEvidenceKind::Crisis,
        format!(
            "algedonic {:?} signal {} with priority {:.2}",
            signal.kind, signal.signal_id, signal.priority
        ),
    ));
    crisis.metadata = signal.metadata.child();
    crisis.metadata.destination = Some(VsmAddress::new(
        runtime_id.clone(),
        recursion_path.clone(),
        SubsystemRole::System5,
    ));
    crisis.signal_id = signal.signal_id.clone();
    crisis
}

fn legacy_message_kind_to_algedonic_kind(
    kind: &MessageKind,
    payload: &Value,
) -> AlgedonicSignalKind {
    if let Some(kind) = first_string(payload, &["kind", "signal_kind", "type"]) {
        match kind.as_str() {
            "pain" => return AlgedonicSignalKind::Pain,
            "pleasure" => return AlgedonicSignalKind::Pleasure,
            "opportunity" => return AlgedonicSignalKind::Opportunity,
            "emergency" => return AlgedonicSignalKind::Emergency,
            "anomaly" => return AlgedonicSignalKind::Anomaly,
            _ => {}
        }
    }

    match kind {
        MessageKind::PainSignal | MessageKind::Critical | MessageKind::Emergency => {
            AlgedonicSignalKind::Pain
        }
        MessageKind::PleasureSignal => AlgedonicSignalKind::Pleasure,
        MessageKind::EmergencySignal => AlgedonicSignalKind::Emergency,
        MessageKind::Alert => AlgedonicSignalKind::Anomaly,
        _ => AlgedonicSignalKind::Anomaly,
    }
}

fn legacy_payload_severity(kind: &MessageKind, payload: &Value) -> AlgedonicSeverity {
    if let Some(severity) = first_string(payload, &["severity", "level"]) {
        match severity.as_str() {
            "critical" => return AlgedonicSeverity::Critical,
            "high" => return AlgedonicSeverity::High,
            "low" => return AlgedonicSeverity::Low,
            _ => return AlgedonicSeverity::Medium,
        }
    }

    match kind {
        MessageKind::Critical | MessageKind::Emergency | MessageKind::EmergencySignal => {
            AlgedonicSeverity::Critical
        }
        MessageKind::Alert | MessageKind::PainSignal => AlgedonicSeverity::High,
        _ => AlgedonicSeverity::Medium,
    }
}

fn algedonic_to_crisis_severity(severity: AlgedonicSeverity) -> CrisisSeverity {
    match severity {
        AlgedonicSeverity::Low => CrisisSeverity::Low,
        AlgedonicSeverity::Medium => CrisisSeverity::Medium,
        AlgedonicSeverity::High => CrisisSeverity::High,
        AlgedonicSeverity::Critical => CrisisSeverity::Critical,
    }
}

fn priority_to_protocol_priority(severity: AlgedonicSeverity, priority: f64) -> Priority {
    if matches!(severity, AlgedonicSeverity::Critical) || priority >= 0.9 {
        Priority::Critical
    } else if matches!(severity, AlgedonicSeverity::High) || priority >= 0.75 {
        Priority::High
    } else if priority <= 0.25 {
        Priority::Low
    } else {
        Priority::Normal
    }
}

fn legacy_system_id_to_role(system_id: crate::shared::message::SystemId) -> SubsystemRole {
    match system_id {
        crate::shared::message::SystemId::System1 => SubsystemRole::System1,
        crate::shared::message::SystemId::System2 => SubsystemRole::System2,
        crate::shared::message::SystemId::System3 => SubsystemRole::System3,
        crate::shared::message::SystemId::System3Star => SubsystemRole::System3Star,
        crate::shared::message::SystemId::System4 => SubsystemRole::System4,
        crate::shared::message::SystemId::System5 => SubsystemRole::System5,
        crate::shared::message::SystemId::TemporalVariety => SubsystemRole::TemporalVariety,
        crate::shared::message::SystemId::Algedonic => SubsystemRole::Algedonic,
        crate::shared::message::SystemId::Telemetry => SubsystemRole::Telemetry,
        crate::shared::message::SystemId::All => SubsystemRole::Custom("all".to_string()),
        crate::shared::message::SystemId::External => SubsystemRole::Custom("external".to_string()),
        other => SubsystemRole::Custom(other.subscriber_id().to_string()),
    }
}

fn first_string(payload: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        payload
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn first_number(payload: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| payload.get(*key).and_then(Value::as_f64))
}

fn value_details(payload: &Value) -> std::collections::BTreeMap<String, String> {
    let mut details = std::collections::BTreeMap::new();

    if let Some(object) = payload.as_object() {
        for (key, value) in object {
            let rendered = match value {
                Value::Null => continue,
                Value::Bool(value) => value.to_string(),
                Value::Number(value) => value.to_string(),
                Value::String(value) => value.clone(),
                Value::Array(_) | Value::Object(_) => value.to_string(),
            };
            details.insert(key.clone(), rendered);
        }
    } else if !payload.is_null() {
        details.insert("payload".to_string(), payload.to_string());
    }

    details
}

pub(crate) struct RuntimeRoleBundles<V>
where
    V: ViableSystem,
{
    pub(crate) system1: System1RuntimeRoles<V>,
    pub(crate) system2: System2RuntimeRoles<V>,
    pub(crate) system3: System3RuntimeRoles<V>,
    pub(crate) system4: System4RuntimeRoles<V>,
    pub(crate) system5: System5RuntimeRoles<V>,
    pub(crate) variety: VarietyRuntimeRoles<V>,
    pub(crate) recursion: RecursionRuntimeRoles<V>,
}

impl<V> RuntimeRoleBundles<V>
where
    V: ViableSystem,
{
    pub(crate) fn new(
        system1: System1RuntimeRoles<V>,
        system2: System2RuntimeRoles<V>,
        system3: System3RuntimeRoles<V>,
        system4: System4RuntimeRoles<V>,
        system5: System5RuntimeRoles<V>,
        variety: VarietyRuntimeRoles<V>,
        recursion: RecursionRuntimeRoles<V>,
    ) -> Self {
        Self {
            system1,
            system2,
            system3,
            system4,
            system5,
            variety,
            recursion,
        }
    }
}

fn apply_audit_boundary<V>(
    request: &System3AuditRequest<V>,
    mut evidence: Vec<AuditEvidence<V>>,
) -> Vec<AuditEvidence<V>>
where
    V: ViableSystem,
{
    if !request.boundary.include_snapshots {
        for item in &mut evidence {
            item.snapshot = None;
        }
    }

    if let Some(max_items) = request.boundary.max_evidence_items {
        evidence.truncate(max_items);
    }

    evidence
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
    system1_runtime: Arc<System1Runtime<V>>,
    system2_roles: System2RuntimeRoles<V>,
    system2_runtime: Arc<System2Runtime<V>>,
    system3_roles: System3RuntimeRoles<V>,
    system3_runtime: Arc<System3Runtime<V>>,
    system4_roles: System4RuntimeRoles<V>,
    system4_runtime: Arc<System4Runtime<V>>,
    system5_roles: System5RuntimeRoles<V>,
    system5_runtime: Arc<System5Runtime<V>>,
    variety_roles: VarietyRuntimeRoles<V>,
    variety_runtime: Arc<VarietyRuntime<V>>,
    recursion_roles: RecursionRuntimeRoles<V>,
    recursion_runtime: Arc<RecursionRuntime<V>>,
    observer_bus: Arc<ObserverEventBus<V>>,
}

impl<V> VsmRuntime<V>
where
    V: ViableSystem,
{
    pub(crate) async fn new(
        config: RuntimeConfig,
        ports: RuntimePorts<V>,
        roles: RuntimeRoleBundles<V>,
    ) -> Result<Self, FrameworkError> {
        let RuntimeRoleBundles {
            system1: system1_roles,
            system2: system2_roles,
            system3: system3_roles,
            system4: system4_roles,
            system5: system5_roles,
            variety: variety_roles,
            recursion: recursion_roles,
        } = roles;
        let observer_bus = Arc::new(ObserverEventBus::new(
            ports.event_sink(),
            config.event_buffer_capacity,
        ));
        let observer_sink: Arc<dyn EventSink<V>> = observer_bus.clone();
        let ports = ports.with_event_sink(observer_sink);
        let system1_runtime =
            System1Runtime::start(config.clone(), system1_roles.clone(), ports.clone()).await?;
        let system2_runtime =
            System2Runtime::start(config.clone(), system2_roles.clone(), ports.clone()).await?;
        let system3_runtime =
            System3Runtime::start(config.clone(), system3_roles.clone(), ports.clone()).await?;
        let system4_runtime =
            System4Runtime::start(config.clone(), system4_roles.clone(), ports.clone()).await?;
        let system5_runtime =
            System5Runtime::start(config.clone(), system5_roles.clone(), ports.clone()).await?;
        let variety_runtime =
            VarietyRuntime::start(config.clone(), variety_roles.clone(), ports.clone()).await?;
        let recursion_runtime =
            RecursionRuntime::start(config.clone(), recursion_roles.clone(), ports.clone()).await?;

        let readiness = RuntimeReadiness::new(vec![
            ReadinessCheck::new(
                ReadinessGate::Infrastructure,
                ReadinessStatus::Ready,
                "runtime ports and instance identity configured",
            ),
            ReadinessCheck::new(
                ReadinessGate::SubsystemActors,
                ReadinessStatus::Ready,
                "typed System 1, System 2, System 3, System 4, System 5, variety/algedonic/temporal, and recursion actor adapters started",
            ),
            ReadinessCheck::new(
                ReadinessGate::RoleImplementations,
                ReadinessStatus::Ready,
                "required System 1 role objects validated; System 2, System 3, System 4, System 5, variety/algedonic/temporal, and recursion policies configured",
            ),
            ReadinessCheck::new(
                ReadinessGate::Subscriptions,
                ReadinessStatus::Ready,
                "typed observer event bus started",
            ),
            ReadinessCheck::new(
                ReadinessGate::Persistence,
                ReadinessStatus::Ready,
                "state store port configured; no-op store starts fresh",
            ),
        ]);

        let mut directory = RuntimeDirectory::new();
        register_runtime_components(&mut directory, &config);

        Ok(Self {
            config,
            readiness,
            lifecycle: Mutex::new(RuntimeState::Ready),
            directory: Mutex::new(directory),
            ports,
            system1_roles,
            system1_runtime,
            system2_roles,
            system2_runtime,
            system3_roles,
            system3_runtime,
            system4_roles,
            system4_runtime,
            system5_roles,
            system5_runtime,
            variety_roles,
            variety_runtime,
            recursion_roles,
            recursion_runtime,
            observer_bus,
        })
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
            Arc::clone(&self.system1_runtime),
        )
    }

    /// Returns a System 2 handle scoped to this runtime instance.
    pub fn system2(&self) -> System2Handle<V> {
        System2Handle::new(
            self.config.clone(),
            self.system2_roles.clone(),
            self.ports.clone(),
            Arc::clone(&self.system2_runtime),
            Arc::clone(&self.system1_runtime),
        )
    }

    /// Returns a System 3 handle scoped to this runtime instance.
    pub fn system3(&self) -> System3Handle<V> {
        System3Handle::new(
            self.config.clone(),
            self.system3_roles.clone(),
            self.ports.clone(),
            Arc::clone(&self.system3_runtime),
            Arc::clone(&self.system1_runtime),
        )
    }

    /// Returns a System 4 handle scoped to this runtime instance.
    pub fn system4(&self) -> System4Handle<V> {
        System4Handle::new(
            self.config.clone(),
            self.system4_roles.clone(),
            self.ports.clone(),
            Arc::clone(&self.system4_runtime),
            Arc::clone(&self.system3_runtime),
        )
    }

    /// Returns a System 5 handle scoped to this runtime instance.
    pub fn system5(&self) -> System5Handle<V> {
        System5Handle::new(
            self.config.clone(),
            self.system5_roles.clone(),
            self.ports.clone(),
            Arc::clone(&self.system5_runtime),
            Arc::clone(&self.system3_runtime),
            Arc::clone(&self.system4_runtime),
        )
    }

    /// Returns a variety, algedonic, and temporal handle scoped to this runtime instance.
    pub fn variety(&self) -> VarietyHandle<V> {
        VarietyHandle::new(
            self.config.clone(),
            self.variety_roles.clone(),
            self.ports.clone(),
            Arc::clone(&self.variety_runtime),
            Arc::clone(&self.system5_runtime),
        )
    }

    /// Returns an operational-recursion handle scoped to this runtime instance.
    pub fn recursion(&self) -> RecursionHandle<V> {
        RecursionHandle::new(
            self.config.clone(),
            self.recursion_roles.clone(),
            self.ports.clone(),
            Arc::clone(&self.recursion_runtime),
            RecursionParentRuntimes {
                system1: Arc::clone(&self.system1_runtime),
                system3: Arc::clone(&self.system3_runtime),
                variety: Arc::clone(&self.variety_runtime),
                system5: Arc::clone(&self.system5_runtime),
            },
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

    /// Subscribes an observer to typed runtime events.
    pub fn subscribe_events(
        &self,
        observer_id: impl Into<String>,
    ) -> Result<ObserverSubscription<V>, FrameworkError> {
        self.observer_bus
            .subscribe(ObserverId::from_string(observer_id))
    }

    /// Returns newest-first retained observer events.
    pub fn observer_event_history(&self) -> Result<Vec<RuntimeEvent<V>>, FrameworkError> {
        self.observer_bus.history()
    }

    /// Returns observer bus delivery metrics.
    pub fn observer_bus_snapshot(&self) -> Result<ObserverBusSnapshot, FrameworkError> {
        self.observer_bus.snapshot()
    }

    /// Returns true after shutdown has been acknowledged.
    pub fn is_shutdown(&self) -> Result<bool, FrameworkError> {
        Ok(self.state()? == RuntimeState::Shutdown)
    }

    /// Shuts the typed runtime handle down and returns an acknowledgement.
    pub async fn shutdown(&self) -> Result<ShutdownReport, FrameworkError> {
        let (previous_state, already_shutdown) = self.begin_shutdown()?;

        if !already_shutdown {
            self.recursion_runtime.shutdown().await?;
            self.shutdown_local_components().await?;
            self.finish_shutdown()?;
        }

        let current_state = self.state()?;

        Ok(ShutdownReport {
            runtime_id: self.config.runtime_id.clone(),
            previous_state,
            current_state,
            already_shutdown,
        })
    }

    pub(crate) async fn shutdown_without_children(&self) -> Result<(), FrameworkError> {
        let (_previous_state, already_shutdown) = self.begin_shutdown()?;

        if !already_shutdown {
            self.shutdown_local_components().await?;
            self.finish_shutdown()?;
        }

        Ok(())
    }

    fn begin_shutdown(&self) -> Result<(RuntimeState, bool), FrameworkError> {
        let mut lifecycle = self.lifecycle.lock().map_err(poisoned_lifecycle)?;
        let previous_state = *lifecycle;
        let already_shutdown = previous_state == RuntimeState::Shutdown;

        if !already_shutdown {
            *lifecycle = RuntimeState::ShuttingDown;
        }

        Ok((previous_state, already_shutdown))
    }

    async fn shutdown_local_components(&self) -> Result<(), FrameworkError> {
        self.variety_runtime.shutdown().await?;
        self.system5_runtime.shutdown().await?;
        self.system4_runtime.shutdown().await?;
        self.system3_runtime.shutdown().await?;
        self.system2_runtime.shutdown().await?;
        self.system1_runtime.shutdown().await?;
        self.directory
            .lock()
            .map_err(poisoned_directory)?
            .mark_all_shutdown();
        Ok(())
    }

    fn finish_shutdown(&self) -> Result<(), FrameworkError> {
        let mut lifecycle = self.lifecycle.lock().map_err(poisoned_lifecycle)?;
        *lifecycle = RuntimeState::Shutdown;
        Ok(())
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
        SubsystemRole::System2,
        "role-bundle",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::System2,
        "coordination-actor",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::System3,
        "role-bundle",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::System3,
        "control-actor",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::System3Star,
        "audit-actor",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::System4,
        "role-bundle",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::System4,
        "intelligence-actor",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::System4,
        "source-registry",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::System5,
        "role-bundle",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::System5,
        "policy-actor",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::Variety,
        "role-bundle",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::Variety,
        "variety-lifecycle-actor",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::Algedonic,
        "algedonic-lifecycle-bridge",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::TemporalVariety,
        "temporal-lifecycle-strategy",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::Custom("recursion".to_string()),
        "role-bundle",
        RuntimeComponentStatus::Ready,
    );
    directory.register(
        runtime_id,
        recursion_path,
        SubsystemRole::Custom("recursion".to_string()),
        "child-runtime-manager",
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
        SubsystemRole::EventSink,
        "typed-observer-bus",
        RuntimeComponentStatus::Ready,
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
