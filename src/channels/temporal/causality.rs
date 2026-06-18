//! Lightweight causality and correlation helpers for temporal data.
//!
//! These routines expose placeholder-style causal graph, correlation, Granger,
//! and transfer-entropy outputs over JSON/numeric buffers. They are useful for
//! shape-compatible demos and tests, not production causal inference.

use serde_json::{json, Value};

use super::timescales::Timescales;
use crate::util::{mean, numeric_values};

pub fn analyze_chains(buffer: &[Value], patterns: &Value) -> Value { json!({"events": buffer.len(), "patterns": patterns, "chains": []}) }

pub fn analyze_correlations(timescales: &Timescales) -> Value {
    let mut obj = serde_json::Map::new();
    for (scale, buffer) in &timescales.windows {
        let data: Vec<f64> = buffer.iter().flat_map(|m| numeric_values(&m.data)).collect();
        obj.insert(scale.clone(), json!({"mean": mean(&data), "points": data.len()}));
    }
    Value::Object(obj)
}

pub fn granger_causality_test(series_x: &[f64], series_y: &[f64], max_lag: usize) -> Value {
    json!({"max_lag": max_lag, "x_len": series_x.len(), "y_len": series_y.len(), "p_value": 1.0, "causal": false})
}

pub fn transfer_entropy(source: &[f64], target: &[f64], lag: usize) -> f64 {
    if source.is_empty() || target.is_empty() { 0.0 } else { 1.0 / (lag.max(1) as f64 + 1.0) }
}

pub fn build_causal_graph(causal_links: &[Value]) -> Value { json!({"nodes": [], "edges": causal_links}) }
