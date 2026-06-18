//! Application type family for typed VSM protocols.

use std::fmt::Debug;
use std::hash::Hash;

/// Minimal application type family shared by typed runtime protocols.
///
/// The framework owns runtime metadata, routing, supervision, deadlines, and
/// snapshots. Applications own the work payload, outcome, error, capability,
/// unit identity, and unit snapshot payload types.
pub trait ViableSystem: Send + Sync + 'static {
    type Work: Clone + Send + 'static;
    type Outcome: Clone + Send + 'static;
    type AppError: std::error::Error + Send + Sync + 'static;
    type Capability: Clone + Eq + Hash + Send + Sync + 'static + Debug;
    type UnitId: Clone + Eq + Hash + Send + Sync + 'static + Debug;
    type UnitSnapshot: Send + 'static;
}
