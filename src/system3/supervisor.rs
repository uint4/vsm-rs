//! Supervisor specification for System 3.
//!
//! The legacy global supervisor is retained as a placeholder while typed
//! System 3 control and System 3* audit run under `VsmRuntime`.

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
