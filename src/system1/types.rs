//! Shared System 1 configuration, snapshot, and coordination types.
//!
//! These types describe unit registration, operational-variety measurements,
//! metrics snapshots, unit summaries, and the tagged coordination requests that
//! System 1 accepts over the coordination channel.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type UnitId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitConfig {
    pub id: UnitId,
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub auto_restart: bool,
    #[serde(default)]
    pub metadata: Value,
}

impl UnitConfig {
    pub fn new<I, S>(id: impl Into<UnitId>, capabilities: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            id: id.into(),
            capabilities: capabilities.into_iter().map(Into::into).collect(),
            auto_restart: true,
            metadata: Value::Null,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarietyMeasurement {
    pub timestamp: DateTime<Utc>,
    pub input: f64,
    pub output: f64,
    pub ratio: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VarietyTrend {
    Increasing,
    Decreasing,
    Stable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarietySnapshot {
    pub input: f64,
    pub output: f64,
    pub ratio: f64,
    pub trend: VarietyTrend,
}

impl Default for VarietySnapshot {
    fn default() -> Self {
        Self {
            input: 0.0,
            output: 0.0,
            ratio: 1.0,
            trend: VarietyTrend::Stable,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub transaction_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub invalid_transaction_count: u64,
    pub no_suitable_unit_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitSummary {
    pub id: UnitId,
    pub status: String,
    pub config: UnitConfig,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoordinationRequest {
    SyncState { unit_ids: Vec<UnitId> },
    LoadBalance { unit_ids: Vec<UnitId> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkMigrationDirection {
    In,
    Out,
}
