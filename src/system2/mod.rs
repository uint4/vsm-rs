//! System 2 coordination and balancing.
//!
//! System 2 is supervised as a JSON `ServiceActor` plus pure scheduling and
//! balancing helpers. Its coordination-channel subscription records incoming
//! events in service history; schedule coordination and resource balancing run
//! only when called through the explicit service facade.

pub mod balancer;
pub mod coordination;
pub mod scheduler;
pub mod supervisor;
