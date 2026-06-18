//! Public typed protocol foundations for the trait-driven runtime.
//!
//! These types are intentionally independent of `ractor`, actor names, and the
//! current JSON broker. They can be used by future runtime handles and role
//! adapters while the existing actors continue to serve the legacy facade.

pub mod address;
pub mod bus;
pub mod envelope;
pub mod events;
pub mod snapshot;
pub mod system1;
pub mod system2;
pub mod system3;
pub mod system4;

pub use address::{RecursionPath, RuntimeId, SubsystemRole, VsmAddress};
pub use bus::{
    DeliveryMetrics, DeliveryStatus, RuntimeControlMessage, System1ControlMessage,
    System2ControlMessage, System3ControlMessage, System4ControlMessage,
};
pub use envelope::{CorrelationId, Priority, ProtocolMetadata, ProtocolVersion, TraceContext};
pub use events::{
    FrameworkEvent, RuntimeEvent, RuntimeReport, System1Event, System1Report, System2Event,
    System2Report, System3Event, System3Report, System4Event, System4Report,
};
pub use snapshot::{SnapshotKey, SnapshotRecord, SnapshotVersion};
