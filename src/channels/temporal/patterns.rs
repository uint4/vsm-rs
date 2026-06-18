//! Pattern detection helpers for temporal-variety timescales.
//!
//! The module derives simple cycle, trend, seasonality, and anomaly summaries
//! from numeric values in each timescale buffer. The calculations are
//! lightweight heuristics intended to describe current in-memory state.

use serde_json::{json, Value};

use super::timescales::Timescales;
use crate::util::{mean, numeric_values, std_dev};

pub fn analyze(timescales: &Timescales) -> Value {
    let mut obj = serde_json::Map::new();
    for (scale, buffer) in &timescales.windows {
        let data: Vec<f64> = buffer
            .iter()
            .flat_map(|m| numeric_values(&m.data))
            .collect();
        obj.insert(
            scale.clone(),
            json!({
                "cycles": detect_cycles(&data),
                "trends": detect_trends(&data),
                "seasonality": detect_seasonality(&data),
                "anomalies": detect_anomalies(&data)
            }),
        );
    }
    Value::Object(obj)
}

pub fn detect_cycles(data: &[f64]) -> Value {
    if data.len() < 4 {
        return json!([]);
    }
    let avg = mean(data);
    let crossings = data
        .windows(2)
        .filter(|w| (w[0] - avg).signum() != (w[1] - avg).signum())
        .count();
    json!({"crossings": crossings, "estimated_cycle_strength": crossings as f64 / data.len() as f64})
}

pub fn detect_trends(data: &[f64]) -> Value {
    if data.len() < 2 {
        return json!({"direction":"stable", "slope":0.0});
    }
    let first = data.first().copied().unwrap_or(0.0);
    let last = data.last().copied().unwrap_or(0.0);
    let slope = (last - first) / data.len() as f64;
    let direction = if slope > 0.05 {
        "increasing"
    } else if slope < -0.05 {
        "decreasing"
    } else {
        "stable"
    };
    json!({"direction": direction, "slope": slope})
}

pub fn detect_seasonality(data: &[f64]) -> Value {
    let sd = std_dev(data);
    json!({"seasonality_score": (sd / (mean(data).abs() + 1.0)).min(1.0)})
}

pub fn detect_anomalies(data: &[f64]) -> Value {
    let avg = mean(data);
    let sd = std_dev(data).max(0.0001);
    let anomalies: Vec<_> = data
        .iter()
        .enumerate()
        .filter(|(_, v)| (**v - avg).abs() > 3.0 * sd)
        .map(|(i, v)| json!({"index": i, "value": v}))
        .collect();
    json!(anomalies)
}
