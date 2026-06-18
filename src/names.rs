//! Stable global actor names for the default runtime.
//!
//! The crate uses ractor's global registry instead of a separate process
//! registry. These names are therefore part of the runtime compatibility
//! surface: changing them can break actor lookup, channel subscription targets,
//! examples, and tests. Dynamic System 1 units are named with `system1_unit`.

pub const ROOT_SUPERVISOR: &str = "vsm.root_supervisor";
pub const DYNAMIC_SUPERVISOR: &str = "vsm.dynamic_supervisor";
pub const TELEMETRY_REPORTER: &str = "vsm.telemetry_reporter";

pub const CHANNEL_BROKER: &str = "vsm.channels.broker";
pub const CHANNELS_SUPERVISOR: &str = "vsm.channels.supervisor";
pub const ALGEDONIC: &str = "vsm.channels.algedonic";
pub const TEMPORAL_VARIETY: &str = "vsm.channels.temporal_variety";

pub const SYSTEM1_SUPERVISOR: &str = "vsm.system1.supervisor";
pub const SYSTEM1_UNIT_SUPERVISOR: &str = "vsm.system1.unit_supervisor";
pub const SYSTEM1_OPERATIONS: &str = "vsm.system1.operations";
pub const SYSTEM1_METRICS: &str = "vsm.system1.metrics";

pub const SYSTEM2_SUPERVISOR: &str = "vsm.system2.supervisor";
pub const SYSTEM2_COORDINATION: &str = "vsm.system2.coordination";

pub const SYSTEM3_SUPERVISOR: &str = "vsm.system3.supervisor";
pub const SYSTEM3_CONTROL: &str = "vsm.system3.control";

pub const SYSTEM4_SUPERVISOR: &str = "vsm.system4.supervisor";
pub const SYSTEM4_INTELLIGENCE: &str = "vsm.system4.intelligence";
pub const SYSTEM4_SCANNER: &str = "vsm.system4.scanner";
pub const SYSTEM4_ANALYTICS: &str = "vsm.system4.analytics";
pub const SYSTEM4_FORECASTING: &str = "vsm.system4.forecasting";

pub const SYSTEM5_SUPERVISOR: &str = "vsm.system5.supervisor";
pub const SYSTEM5_POLICY: &str = "vsm.system5.policy";
pub const SYSTEM5_IDENTITY: &str = "vsm.system5.identity";
pub const SYSTEM5_VALUES: &str = "vsm.system5.values";
pub const SYSTEM5_DECISIONS: &str = "vsm.system5.decisions";

pub fn system1_unit(id: &str) -> String {
    format!("vsm.system1.unit.{id}")
}
