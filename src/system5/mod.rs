//! System 5 policy, identity, values, and decisions.
//!
//! System 5 runs four supervised JSON service actors with independent
//! `ServiceState`s. Use the Policy actor as the aggregate boundary when a single
//! coherent organizational state is required; calling the standalone Identity,
//! Values, or Decisions actors mutates only their own state.

pub mod decisions;
pub mod identity;
pub mod policy;
pub mod supervisor;
pub mod values;
