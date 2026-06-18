//! Public role-adjacent foundations.

pub mod ports;
pub mod types;

pub use ports::{EventSink, NoopEventSink, NoopReportSink, NoopStateStore, ReportSink, StateStore};
pub use types::ViableSystem;
