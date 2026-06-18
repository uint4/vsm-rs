//! Runtime configuration for the trait-driven builder surface.

use std::time::Duration;

use crate::protocol::{RecursionPath, RuntimeId};

const DEFAULT_WORK_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_READINESS_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_EVENT_BUFFER_CAPACITY: usize = 1024;

/// Configuration owned by one typed runtime instance.
///
/// The configuration is instance-scoped: generated runtime component addresses
/// and future actor names are derived from `runtime_id` plus `recursion_path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub runtime_id: RuntimeId,
    pub recursion_path: RecursionPath,
    pub default_work_timeout: Duration,
    pub readiness_timeout: Duration,
    pub shutdown_timeout: Duration,
    pub max_registered_units: Option<usize>,
    pub event_buffer_capacity: usize,
}

impl RuntimeConfig {
    /// Creates a configuration for a specific runtime instance.
    pub fn new(runtime_id: RuntimeId) -> Self {
        Self {
            runtime_id,
            recursion_path: RecursionPath::root(),
            default_work_timeout: DEFAULT_WORK_TIMEOUT,
            readiness_timeout: DEFAULT_READINESS_TIMEOUT,
            shutdown_timeout: DEFAULT_SHUTDOWN_TIMEOUT,
            max_registered_units: None,
            event_buffer_capacity: DEFAULT_EVENT_BUFFER_CAPACITY,
        }
    }

    /// Sets the recursion path for this runtime instance.
    pub fn with_recursion_path(mut self, recursion_path: RecursionPath) -> Self {
        self.recursion_path = recursion_path;
        self
    }

    /// Sets the runtime default work timeout.
    pub fn with_default_work_timeout(mut self, timeout: Duration) -> Self {
        self.default_work_timeout = timeout;
        self
    }

    /// Sets the readiness timeout used by future actor-backed startup.
    pub fn with_readiness_timeout(mut self, timeout: Duration) -> Self {
        self.readiness_timeout = timeout;
        self
    }

    /// Sets the shutdown timeout used by future actor-backed teardown.
    pub fn with_shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// Sets the maximum number of units that can be registered.
    pub fn with_max_registered_units(mut self, max_units: usize) -> Self {
        self.max_registered_units = Some(max_units);
        self
    }

    /// Removes the maximum registered-unit limit.
    pub fn with_unbounded_registered_units(mut self) -> Self {
        self.max_registered_units = None;
        self
    }

    /// Sets the in-memory observer event buffer capacity.
    pub fn with_event_buffer_capacity(mut self, capacity: usize) -> Self {
        self.event_buffer_capacity = capacity;
        self
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self::new(RuntimeId::new())
    }
}
