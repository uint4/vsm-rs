//! Public typed protocol foundations for the trait-driven runtime.
//!
//! These types are intentionally independent of `ractor`, actor names, and the
//! current JSON broker. They can be used by future runtime handles and role
//! adapters while the existing actors continue to serve the legacy facade.

pub mod address;
pub mod envelope;
pub mod events;
pub mod snapshot;
pub mod system1;

pub use address::{RecursionPath, RuntimeId, SubsystemRole, VsmAddress};
pub use envelope::{CorrelationId, Priority, ProtocolMetadata, ProtocolVersion, TraceContext};
pub use events::{FrameworkEvent, RuntimeEvent, RuntimeReport, System1Event, System1Report};
pub use snapshot::{SnapshotKey, SnapshotRecord, SnapshotVersion};
