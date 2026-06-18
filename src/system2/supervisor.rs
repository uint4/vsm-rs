//! Supervisor specification for System 2.
//!
//! The legacy global actor tree no longer starts a JSON coordination
//! `ServiceActor` for System 2. The active System 2 core path is the typed
//! runtime handle returned by [`crate::VsmRuntime::system2`].

use ractor::concurrency::Duration;
use ractor_supervisor::{SupervisorArguments, SupervisorOptions, SupervisorStrategy};

pub fn supervisor_args() -> SupervisorArguments {
    SupervisorArguments {
        child_specs: Vec::new(),
        options: SupervisorOptions {
            strategy: SupervisorStrategy::OneForOne,
            max_restarts: 5,
            max_window: Duration::from_secs(10),
            reset_after: Some(Duration::from_secs(30)),
        },
    }
}
