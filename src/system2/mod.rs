//! System 2 coordination and balancing.
//!
//! The typed runtime path owns System 2 coordination through
//! [`crate::VsmRuntime::system2`] and the public [`crate::CoordinationPolicy`]
//! role. Legacy JSON scheduling and balancing helpers are retained under
//! [`defaults`] as opt-in examples rather than core VSM semantics.

pub mod defaults;
pub mod supervisor;
