//! Process-local alert history for the algedonic processor.
//!
//! Alert records combine a typed signal, the calculated route, and an alert
//! level. History is stored in a process-global mutex, so it survives actor
//! restart but is lost on process exit and is not a durable event log.

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Mutex;

use super::routing::RouteInfo;
use super::signals::AlgedonicSignal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRecord {
    pub signal: AlgedonicSignal,
    pub route: RouteInfo,
    pub level: String,
}

static ALERT_HISTORY: Lazy<Mutex<Vec<AlertRecord>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn send_critical_alert(signal: AlgedonicSignal, route_info: RouteInfo) -> AlertRecord {
    record(signal, route_info, "critical")
}

pub fn send_alert(signal: AlgedonicSignal, route_info: RouteInfo) -> AlertRecord {
    record(signal, route_info, "alert")
}

pub fn send_batch_alert(signals: Vec<AlgedonicSignal>, pattern: Value, route_info: RouteInfo) -> Value {
    let records: Vec<_> = signals.into_iter().map(|s| send_alert(s, route_info.clone())).collect();
    json!({"pattern": pattern, "alert_count": records.len(), "route": route_info, "records": records})
}

pub fn get_alert_history(options: &Value) -> Vec<AlertRecord> {
    let limit = options.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;
    let history = ALERT_HISTORY.lock().unwrap();
    history.iter().rev().take(limit).cloned().collect()
}

fn record(signal: AlgedonicSignal, route_info: RouteInfo, level: &str) -> AlertRecord {
    let record = AlertRecord { signal, route: route_info, level: level.into() };
    let mut history = ALERT_HISTORY.lock().unwrap();
    history.push(record.clone());
    if history.len() > 10_000 { history.drain(0..1000); }
    record
}
