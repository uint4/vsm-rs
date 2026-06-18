//! Public role-adjacent foundations.

pub mod context;
pub mod ports;
pub mod system1;
pub mod system2;
pub mod system3;
pub mod types;

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
pub use types::ViableSystem;
