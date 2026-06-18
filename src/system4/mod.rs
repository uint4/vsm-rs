//! System 4 environmental intelligence support.
//!
//! The public typed runtime surface lives in [`crate::runtime::System4Handle`],
//! [`crate::roles::system4`], and [`crate::protocol::system4`]. This module
//! keeps opt-in prototype helpers and the legacy default supervisor boundary.

pub mod defaults;
pub mod supervisor;
