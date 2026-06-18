//! System 4 intelligence, scanning, analytics, and forecasting.
//!
//! System 4 runs four supervised JSON service actors. The Intelligence actor can
//! aggregate scanner, analytics, and forecasting module functions directly, but
//! the Scanner, Analytics, and Forecasting actors also remain independently
//! callable service/state boundaries.

pub mod analytics;
pub mod forecasting;
pub mod intelligence;
pub mod scanner;
pub mod supervisor;
