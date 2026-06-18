//! Rust/ractor port of the Elixir `vsm-core` application.
//!
//! The crate models a Viable System Model runtime with a supervised actor tree,
//! typed System 1 operations, brokered VSM messages, and JSON-backed service
//! actors for the exploratory Systems 2-5 APIs. All default actors use stable
//! global names, so a process can run only one default application instance at
//! a time. State is currently in memory and should be treated as restart
//! volatile unless an embedding application adds persistence.
//!
//! See `docs/ARCHITECTURE.md` for runtime topology, `docs/USAGE.md` for
//! consumer workflows, and `docs/DEVELOPERS.md` for extension rules.

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
