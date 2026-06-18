use ractor::concurrency::Duration;
use ractor_supervisor::{SupervisorArguments, SupervisorOptions, SupervisorStrategy};
use serde_json::json;

use crate::actor_support::{service_child, ServiceKind};
use crate::names;

pub fn supervisor_args() -> SupervisorArguments {
    SupervisorArguments {
        child_specs: vec![service_child(names::SYSTEM3_CONTROL, ServiceKind::System3Control, json!({"subsystem":"system3", "role":"control"}))],
        options: SupervisorOptions { strategy: SupervisorStrategy::OneForOne, max_restarts: 5, max_window: Duration::from_secs(10), reset_after: Some(Duration::from_secs(30)) },
    }
}
