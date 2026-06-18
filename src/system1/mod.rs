//! System 1 operational execution.
//!
//! System 1 is the main typed subsystem in the current port. `Operations` owns
//! the unit directory, metrics, and operational-variety history; supervised
//! `Unit` actors own local unit state and process demo transactions. The public
//! facade in this module is the preferred API for registration, transaction
//! routing, metrics, variety, and algedonic escalation from operations.

pub mod metrics;
pub mod operations;
pub mod supervisor;
pub mod transaction;
pub mod types;
pub mod unit;

pub use operations::{
    get_metrics, get_variety, list_units, operations_ref, process_transaction, register_unit,
    send_algedonic_signal, Operations, OperationsArgs, OperationsMsg,
};
pub use transaction::{Transaction, TransactionResult};
pub use types::{MetricsSnapshot, UnitConfig, UnitId, UnitSummary, VarietySnapshot};
