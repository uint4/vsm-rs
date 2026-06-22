//! Public role-adjacent foundations.

pub mod context;
pub mod ports;
pub mod system1;
pub mod system2;
pub mod system3;
pub mod system4;
pub mod system5;
pub mod types;
pub mod variety;

pub use context::{RoleContext, UnitRoleContext};
pub use ports::{
    AlertRecord, AlertSeverity, AlertSink, Clock, EventSink, IdGenerator, NoopAlertSink,
    NoopEventSink, NoopReportSink, NoopStateStore, NoopTelemetrySink, ReportSink, StateStore,
    SystemClock, TelemetryRecord, TelemetrySink, UuidIdGenerator,
};
pub use system1::{
    AlgedonicKind, AlgedonicPolicy, AlgedonicSeverity, AlgedonicSignal, BoxOperationalUnit,
    OperationalUnit, OperationalUnitFactory, PerformanceAssessment, PerformanceModel,
    SharedAlgedonicPolicy, SharedOperationalUnitFactory, SharedPerformanceModel,
    SharedUnitSelectionPolicy, SharedVarietyModel, SharedWorkModel, System1Roles, UnitCandidate,
    UnitSelectionPolicy, VarietyAssessment, VarietyModel, WorkMeasurement, WorkModel,
};
pub use system2::{CoordinationPolicy, SharedCoordinationPolicy, System2Roles};
pub use system3::{
    Auditor, OperationalControlPolicy, ResourceGovernance, SharedAuditor,
    SharedOperationalControlPolicy, SharedResourceGovernance, System3Roles,
};
pub use system4::{
    BoxEnvironmentalSource, EnvironmentalSource, EnvironmentalSourceFactory, Forecaster,
    IntelligenceModel, SharedEnvironmentalSourceFactory, SharedForecaster, SharedIntelligenceModel,
    SharedSignalInterpreter, SignalInterpreter, System4Roles,
};
pub use system5::{
    CrisisPolicy, DecisionPolicy, IdentityProvider, SharedCrisisPolicy, SharedDecisionPolicy,
    SharedIdentityProvider, SharedValuesEvaluator, SharedValuesProvider, System5Roles,
    ValuesEvaluator, ValuesProvider,
};
pub use types::ViableSystem;
pub use variety::{
    AlgedonicLifecyclePolicy, SharedAlgedonicLifecyclePolicy, SharedTemporalAnalysisPolicy,
    SharedVarietyEngineeringPolicy, TemporalAnalysisPolicy, VarietyAlgedonicTemporalRoles,
    VarietyEngineeringPolicy,
};
