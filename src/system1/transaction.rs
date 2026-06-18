//! Transaction input and result types for System 1.
//!
//! Transactions are serializable JSON-backed work requests with a string kind
//! and required capabilities. The demo validation currently rejects only an
//! empty kind. Domain failures such as invalid transactions, missing units, and
//! unit errors are returned as `TransactionResult` values rather than transport
//! errors.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,

    #[serde(rename = "type")]
    pub kind: String,

    #[serde(default)]
    pub required_capabilities: Vec<String>,

    #[serde(default)]
    pub payload: Value,
}

impl Transaction {
    pub fn new(
        kind: impl Into<String>,
        required_capabilities: Vec<String>,
        payload: Value,
    ) -> Self {
        Self {
            id: format!("tx_{}", Uuid::new_v4()),
            kind: kind.into(),
            required_capabilities,
            payload,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.kind.trim().is_empty() {
            return Err("transaction type is empty".to_string());
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data", rename_all = "snake_case")]
pub enum TransactionResult {
    Ok(Value),
    InvalidTransaction(String),
    NoSuitableUnit,
    UnitUnavailable(String),
    UnitError(String),
}

impl TransactionResult {
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }
}

pub fn calculate_input_variety(transaction: &Transaction) -> f64 {
    let capability_variety = transaction.required_capabilities.len() as f64;
    let payload_variety = match &transaction.payload {
        Value::Object(map) => map.len() as f64,
        Value::Array(items) => items.len() as f64,
        Value::Null => 0.0,
        _ => 1.0,
    };

    1.0 + capability_variety + payload_variety
}

pub fn calculate_output_variety(result: &TransactionResult) -> f64 {
    match result {
        TransactionResult::Ok(Value::Object(map)) => map.len() as f64,
        TransactionResult::Ok(Value::Array(items)) => items.len() as f64,
        TransactionResult::Ok(Value::Null) => 0.0,
        TransactionResult::Ok(_) => 1.0,
        _ => 0.0,
    }
}
