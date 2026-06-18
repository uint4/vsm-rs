//! Temporary adapters between current JSON-facing APIs and typed foundations.
//!
//! These adapters let the current examples round-trip through the new protocol
//! records while the actor runtime still uses the legacy facade internally.

pub mod system1;
