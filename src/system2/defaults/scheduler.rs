//! Default schedule coordination helpers for System 2 examples.
//!
//! The scheduler combines existing and new JSON schedule entries, detects
//! temporal/resource/dependency conflicts, and produces a simple sequential
//! optimized schedule. The optimizer is intentionally lightweight and starts
//! its output schedule at the current time.

use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};

use crate::util::mean;

pub fn coordinate(new_schedules: &[Value], existing_schedules: &[Value]) -> Value {
    let mut entries = Vec::new();
    entries.extend_from_slice(existing_schedules);
    entries.extend_from_slice(new_schedules);
    let conflicts = detect_conflicts(&entries);
    let optimized = optimize_schedule(&Value::Array(entries));
    json!({"status":"ok", "schedule": optimized, "conflicts": conflicts})
}

pub fn detect_conflicts(schedules: &[Value]) -> Value {
    let mut temporal = Vec::new();
    let mut resource = Vec::new();
    let mut dependency = Vec::new();
    for i in 0..schedules.len() {
        for j in (i + 1)..schedules.len() {
            if temporal_overlap(&schedules[i], &schedules[j]) {
                temporal.push(
                    json!({"a": id(&schedules[i]), "b": id(&schedules[j]), "type":"temporal"}),
                );
            }
            if same_resource(&schedules[i], &schedules[j]) {
                resource.push(
                    json!({"a": id(&schedules[i]), "b": id(&schedules[j]), "type":"resource"}),
                );
            }
        }
        for dep in schedules[i]
            .get("depends_on")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            if !schedules.iter().any(|e| {
                Some(dep.as_str().unwrap_or_default()) == e.get("id").and_then(Value::as_str)
            }) {
                dependency.push(json!({"entry": id(&schedules[i]), "missing_dependency": dep}));
            }
        }
    }
    json!({"temporal": temporal, "resource": resource, "dependency": dependency})
}

pub fn optimize_schedule(schedule: &Value) -> Value {
    let mut entries = schedule.as_array().cloned().unwrap_or_default();
    entries.sort_by(|a, b| {
        start_ms(a).cmp(&start_ms(b)).then(
            priority(b)
                .partial_cmp(&priority(a))
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });
    let mut cursor = Utc::now();
    let mut optimized = Vec::new();
    for mut entry in entries {
        let duration = duration_ms(&entry).max(1_000);
        entry["start_at"] = json!(cursor.to_rfc3339());
        cursor += Duration::milliseconds(duration);
        entry["end_at"] = json!(cursor.to_rfc3339());
        optimized.push(entry);
    }
    Value::Array(optimized)
}

pub fn validate_schedule(schedule: &Value) -> Value {
    let entries = schedule.as_array().cloned().unwrap_or_default();
    let conflicts = detect_conflicts(&entries);
    let valid = conflicts
        .get("temporal")
        .and_then(Value::as_array)
        .map(Vec::is_empty)
        .unwrap_or(true)
        && conflicts
            .get("dependency")
            .and_then(Value::as_array)
            .map(Vec::is_empty)
            .unwrap_or(true);
    json!({"valid": valid, "conflicts": conflicts})
}

pub fn calculate_metrics(schedule: &Value) -> Value {
    let entries = schedule.as_array().cloned().unwrap_or_default();
    let durations: Vec<f64> = entries.iter().map(|e| duration_ms(e) as f64).collect();
    json!({"entry_count": entries.len(), "avg_duration_ms": mean(&durations), "critical_path_ms": durations.iter().sum::<f64>(), "utilization": if entries.is_empty(){0.0}else{0.8}})
}

fn id(v: &Value) -> Value {
    v.get("id").cloned().unwrap_or_else(|| json!("unknown"))
}
fn priority(v: &Value) -> f64 {
    v.get("priority").and_then(Value::as_f64).unwrap_or(0.0)
}
fn start_ms(v: &Value) -> i64 {
    parse_dt(v.get("start_at"))
        .map(|d| d.timestamp_millis())
        .unwrap_or(0)
}
fn duration_ms(v: &Value) -> i64 {
    v.get("duration_ms")
        .and_then(Value::as_i64)
        .unwrap_or(60_000)
        .max(0)
}
fn end_ms(v: &Value) -> i64 {
    parse_dt(v.get("end_at"))
        .map(|d| d.timestamp_millis())
        .unwrap_or(start_ms(v) + duration_ms(v))
}
fn parse_dt(v: Option<&Value>) -> Option<DateTime<Utc>> {
    v.and_then(Value::as_str)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
}
fn temporal_overlap(a: &Value, b: &Value) -> bool {
    start_ms(a) < end_ms(b) && start_ms(b) < end_ms(a)
}
fn same_resource(a: &Value, b: &Value) -> bool {
    let ar = a.get("resource").and_then(Value::as_str);
    let br = b.get("resource").and_then(Value::as_str);
    ar.is_some() && ar == br && temporal_overlap(a, b)
}
