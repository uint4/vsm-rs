//! Supervisor specification for System 2.
//!
//! The supervisor starts one permanent coordination `ServiceActor` under a
//! one-for-one strategy. The service owns only in-memory JSON data and bounded
//! call/channel history.

use ractor::concurrency::Duration;
use ractor_supervisor::{SupervisorArguments, SupervisorOptions, SupervisorStrategy};
use serde_json::json;

use crate::actor_support::{service_child, ServiceKind};
use crate::names;

pub fn supervisor_args() -> SupervisorArguments {
    SupervisorArguments {
        child_specs: vec![service_child(names::SYSTEM2_COORDINATION, ServiceKind::System2Coordination, json!({"subsystem":"system2", "role":"coordination"}))],
        options: SupervisorOptions { strategy: SupervisorStrategy::OneForOne, max_restarts: 5, max_window: Duration::from_secs(10), reset_after: Some(Duration::from_secs(30)) },
    }
}
