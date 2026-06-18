//! System 3 control, resource allocation, and audit.
//!
//! System 3 is supervised as one JSON `ServiceActor` backed by pure resource
//! and audit helpers. Resource-bargain, command, and audit channel messages are
//! recorded in service history, but allocation or audit work happens only when
//! the corresponding service operation is called.

pub mod audit;
pub mod control;
pub mod resources;
pub mod supervisor;
