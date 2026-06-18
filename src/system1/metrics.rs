//! In-memory metrics for System 1 transaction outcomes.
//!
//! `MetricsStore` is owned by the Operations actor and summarizes transaction
//! totals, successes, failures, invalid inputs, and no-suitable-unit outcomes.
//! Metrics reset when the Operations actor restarts because there is no durable
//! metrics store in the current crate.

use crate::system1::transaction::TransactionResult;
use crate::system1::types::MetricsSnapshot;

#[derive(Debug, Clone, Default)]
pub struct MetricsStore {
    transaction_count: u64,
    success_count: u64,
    failure_count: u64,
    invalid_transaction_count: u64,
    no_suitable_unit_count: u64,
}

impl MetricsStore {
    pub fn record_transaction(&mut self, result: &TransactionResult) {
        self.transaction_count += 1;

        match result {
            TransactionResult::Ok(_) => self.success_count += 1,
            TransactionResult::InvalidTransaction(_) => {
                self.failure_count += 1;
                self.invalid_transaction_count += 1;
            }
            TransactionResult::NoSuitableUnit => {
                self.failure_count += 1;
                self.no_suitable_unit_count += 1;
            }
            TransactionResult::UnitUnavailable(_) | TransactionResult::UnitError(_) => {
                self.failure_count += 1;
            }
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            transaction_count: self.transaction_count,
            success_count: self.success_count,
            failure_count: self.failure_count,
            invalid_transaction_count: self.invalid_transaction_count,
            no_suitable_unit_count: self.no_suitable_unit_count,
        }
    }
}
