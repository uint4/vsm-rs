//! Supervisor specification for System 4.
//!
//! The supervisor starts Intelligence, Scanner, Analytics, and Forecasting as
//! independent permanent `ServiceActor` children. Each actor owns separate
//! in-memory data and history even when the Intelligence actor calls shared
//! module functions directly.

use ractor::concurrency::Duration;
use ractor_supervisor::{SupervisorArguments, SupervisorOptions, SupervisorStrategy};
use serde_json::json;

use crate::actor_support::{service_child, ServiceKind};
use crate::names;

pub fn supervisor_args() -> SupervisorArguments {
    SupervisorArguments {
        child_specs: vec![
            service_child(names::SYSTEM4_INTELLIGENCE, ServiceKind::System4Intelligence, json!({"subsystem":"system4", "role":"intelligence"})),
            service_child(names::SYSTEM4_SCANNER, ServiceKind::System4Scanner, json!({"subsystem":"system4", "role":"scanner"})),
            service_child(names::SYSTEM4_ANALYTICS, ServiceKind::System4Analytics, json!({"subsystem":"system4", "role":"analytics"})),
            service_child(names::SYSTEM4_FORECASTING, ServiceKind::System4Forecasting, json!({"subsystem":"system4", "role":"forecasting"})),
        ],
        options: SupervisorOptions { strategy: SupervisorStrategy::OneForOne, max_restarts: 5, max_window: Duration::from_secs(10), reset_after: Some(Duration::from_secs(30)) },
    }
}
