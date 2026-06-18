//! System 3 control, resource allocation, and audit defaults.
//!
//! Typed System 3 control and System 3* audit run under `VsmRuntime` through
//! public role traits. The legacy JSON helpers are retained only as opt-in
//! defaults/examples.

pub mod defaults;
pub mod supervisor;
