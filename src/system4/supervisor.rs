//! Legacy supervisor boundary for System 4.
//!
//! System 4 has moved to the typed runtime surface. The legacy default
//! application still starts this supervisor so the static actor tree shape does
//! not churn during the migration, but it intentionally has no JSON service
//! children.

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
