//! Rust/ractor port of the Elixir `vsm-core` application.
//!
//! The crate models a Viable System Model runtime with a supervised actor tree,
//! typed System 1 operations, typed System 2 coordination, typed System 3
//! control/audit, typed System 4 intelligence, typed System 5 policy decisions,
//! brokered VSM messages, and JSON-backed service actors for auxiliary APIs.
//! All default actors use stable global names, so a process can run only
//! one default application instance at a time. State is currently in memory and
//! should be treated as restart volatile unless an embedding application adds
//! persistence.
//!
//! See `docs/ARCHITECTURE.md` for runtime topology, `docs/USAGE.md` for
//! consumer workflows, and `docs/DEVELOPERS.md` for extension rules.

pub mod actor_support;
pub mod app;
pub mod builder;
pub mod cancellation;
pub mod channels;
pub mod config;
pub mod domain;
pub mod error;
mod kernel;
pub mod legacy;
pub mod names;
pub mod prelude;
pub mod protocol;
pub mod roles;
pub mod runtime;
pub mod shared;
pub mod system1;
pub mod system2;
pub mod system3;
pub mod system4;
pub mod system5;
pub mod telemetry_reporter;
pub mod util;
pub mod vsm_core;

pub use app::{start_application, start_vsm_core, VsmApplication};
pub use builder::VsmBuilder;
pub use channels::broker::{DeliveryOutcome, UndeliverableMessage};
pub use config::RuntimeConfig;
pub use error::{ApplicationFailure, FrameworkError, VsmError, VsmResult, WorkError};
pub use protocol::{
    DeliveryMetrics, DeliveryStatus, RuntimeControlMessage, System1ControlMessage,
    System2ControlMessage, System3ControlMessage, System4ControlMessage, System5ControlMessage,
};
pub use ractor::async_trait;
pub use roles::{
    AlgedonicPolicy, Auditor, CoordinationPolicy, CrisisPolicy, DecisionPolicy,
    EnvironmentalSource, EnvironmentalSourceFactory, Forecaster, IdentityProvider,
    IntelligenceModel, OperationalControlPolicy, OperationalUnit, OperationalUnitFactory,
    PerformanceModel, ResourceGovernance, RoleContext, SignalInterpreter, System1Roles,
    System2Roles, System3Roles, System4Roles, System5Roles, UnitRoleContext, UnitSelectionPolicy,
    ValuesEvaluator, ValuesProvider, VarietyModel, ViableSystem, WorkModel,
};
pub use runtime::{
    ObserverBusSnapshot, ObserverId, ObserverSubscription, ReadinessCheck, ReadinessGate,
    ReadinessStatus, RegisteredUnit, RuntimeComponentSnapshot, RuntimeComponentStatus,
    RuntimeDirectorySnapshot, RuntimePorts, RuntimeReadiness, RuntimeState, ShutdownReport,
    System1Handle, System1RuntimeRoles, System2Handle, System2RuntimeRoles, System3Handle,
    System3RuntimeRoles, System4Handle, System4RuntimeRoles, System5Handle, System5RuntimeRoles,
    UnitAdmissionLimits, UnitRegistration, UnitSnapshotConfig, VsmRuntime,
};
pub use shared::message::{ChannelKind, MessageKind, SystemId, VsmMessage};

pub use vsm_core::{
    health, require_running, send_test_signal, start, status, stop, subsystem_state, test_signal,
};
