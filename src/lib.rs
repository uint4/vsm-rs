//! Rust/ractor port of the uploaded Elixir `vsm-core` application.
//!
//! The crate keeps the same conceptual module boundaries as `VSMCore` while
//! using typed Rust structs, `serde_json::Value` extension points, and ractor
//! actors/supervisors for OTP-style services.

pub mod actor_support;
pub mod app;
pub mod channels;
pub mod error;
pub mod names;
pub mod prelude;
pub mod shared;
pub mod system1;
pub mod system2;
pub mod system3;
pub mod system4;
pub mod system5;
pub mod telemetry_reporter;
pub mod util;
pub mod vsm_core;
pub mod domain;

pub use app::{start_application, start_vsm_core, VsmApplication};
pub use error::{VsmError, VsmResult};
pub use shared::message::{ChannelKind, MessageKind, SystemId, VsmMessage};

pub use vsm_core::{health, require_running, send_test_signal, start, status, stop, subsystem_state, test_signal};
