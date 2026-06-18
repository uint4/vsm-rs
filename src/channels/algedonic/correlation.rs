//! Lightweight correlation helpers for algedonic signals.
//!
//! These pure functions summarize signal clusters, compare priorities, and
//! detect priority outliers for metrics output. The calculations are heuristic
//! starter implementations rather than statistical guarantees.

use serde_json::{json, Value};

use super::signals::AlgedonicSignal;
use crate::util::{mean, std_dev};

pub fn analyze_patterns(signals: &[AlgedonicSignal], _correlation_state: &Value) -> Value {
    let clusters = cluster_signals(signals, &json!({"by":"source"}));
    let priority_series: Vec<f64> = signals.iter().map(|s| s.priority).collect();
    json!({
        "signal_count": signals.len(),
        "avg_priority": mean(&priority_series),
        "priority_std_dev": std_dev(&priority_series),
        "clusters": clusters,
    })
}

pub fn calculate_correlation(signal1: &AlgedonicSignal, signal2: &AlgedonicSignal) -> f64 {
    let same_source = if signal1.source == signal2.source {
        0.35
    } else {
        0.0
    };
    let time_gap = (signal1.timestamp - signal2.timestamp).num_seconds().abs() as f64;
    let temporal = (1.0 - (time_gap / 3600.0)).max(0.0) * 0.25;
    let priority_similarity = (1.0 - (signal1.priority - signal2.priority).abs()).max(0.0) * 0.25;
    let same_kind = if signal1.kind == signal2.kind {
        0.15
    } else {
        0.0
    };
    same_source + temporal + priority_similarity + same_kind
}

pub fn cluster_signals(signals: &[AlgedonicSignal], options: &Value) -> Value {
    let by = options
        .get("by")
        .and_then(|v| v.as_str())
        .unwrap_or("source");
    let mut groups = serde_json::Map::new();
    for signal in signals {
        let key = match by {
            "kind" => format!("{:?}", signal.kind),
            "severity" => format!("{:?}", signal.severity),
            _ => signal.source.clone(),
        };
        groups
            .entry(key)
            .or_insert(json!([]))
            .as_array_mut()
            .unwrap()
            .push(json!(signal));
    }
    Value::Object(groups)
}

pub fn detect_anomalies(
    signals: &[AlgedonicSignal],
    _historical_patterns: &Value,
) -> Vec<AlgedonicSignal> {
    let priorities: Vec<f64> = signals.iter().map(|s| s.priority).collect();
    let avg = mean(&priorities);
    let sd = std_dev(&priorities).max(0.05);
    signals
        .iter()
        .filter(|s| s.priority > avg + (2.0 * sd))
        .cloned()
        .collect()
}
