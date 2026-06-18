//! Supervisor specification for System 5.
//!
//! The supervisor starts Policy, Identity, Values, and Decisions as independent
//! permanent `ServiceActor` children. Restarting any child loses that child's
//! in-memory JSON state and bounded history.

use ractor::concurrency::Duration;
use ractor_supervisor::{SupervisorArguments, SupervisorOptions, SupervisorStrategy};
use serde_json::json;

use crate::actor_support::{service_child, ServiceKind};
use crate::names;

pub fn supervisor_args() -> SupervisorArguments {
    SupervisorArguments {
        child_specs: vec![
            service_child(names::SYSTEM5_POLICY, ServiceKind::System5Policy, json!({"subsystem":"system5", "role":"policy"})),
            service_child(names::SYSTEM5_IDENTITY, ServiceKind::System5Identity, json!({"subsystem":"system5", "role":"identity"})),
            service_child(names::SYSTEM5_VALUES, ServiceKind::System5Values, json!({"subsystem":"system5", "role":"values"})),
            service_child(names::SYSTEM5_DECISIONS, ServiceKind::System5Decisions, json!({"subsystem":"system5", "role":"decisions"})),
        ],
        options: SupervisorOptions { strategy: SupervisorStrategy::OneForOne, max_restarts: 5, max_window: Duration::from_secs(10), reset_after: Some(Duration::from_secs(30)) },
    }
}
