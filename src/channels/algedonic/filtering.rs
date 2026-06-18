//! Filter definitions and evaluation for algedonic signals.
//!
//! Filters are JSON-configured predicates over priority, severity, source,
//! signal kind, or a placeholder predicate kind. The actor applies all enabled
//! filters before recording a signal or alert route.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::signals::{AlgedonicSignal, Severity, SignalKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterKind {
    Severity,
    Priority,
    Source,
    Type,
    Predicate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub kind: FilterKind,
    pub name: String,
    pub config: Value,
    pub enabled: bool,
}

pub fn default_filters() -> Vec<Filter> {
    vec![
        create_filter(
            FilterKind::Priority,
            "drop_low_priority",
            json!({"min_priority": 0.25}),
            true,
        ),
        create_filter(
            FilterKind::Severity,
            "critical_fast_path",
            json!({"min_severity": "low"}),
            true,
        ),
    ]
}

pub fn create_filter(
    kind: FilterKind,
    name: impl Into<String>,
    config: Value,
    enabled: bool,
) -> Filter {
    Filter {
        kind,
        name: name.into(),
        config,
        enabled,
    }
}

pub fn validate_filters(filters: &[Filter]) -> Result<(), String> {
    for f in filters {
        if f.name.trim().is_empty() {
            return Err("filter name is empty".into());
        }
    }
    Ok(())
}

pub fn apply_filters(signal: &AlgedonicSignal, filters: &[Filter]) -> bool {
    filters.iter().filter(|f| f.enabled).all(|f| match f.kind {
        FilterKind::Priority => {
            signal.priority
                >= f.config
                    .get("min_priority")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
        }
        FilterKind::Severity => {
            signal.severity
                >= parse_severity(
                    f.config
                        .get("min_severity")
                        .and_then(|v| v.as_str())
                        .unwrap_or("low"),
                )
        }
        FilterKind::Source => f
            .config
            .get("allow")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .any(|v| v.as_str() == Some(signal.source.as_str()))
            })
            .unwrap_or(true),
        FilterKind::Type => f
            .config
            .get("kind")
            .and_then(|v| v.as_str())
            .map(|k| kind_name(signal.kind) == k)
            .unwrap_or(true),
        FilterKind::Predicate => true,
    })
}

pub fn analyze_filter_effectiveness(
    filters: &[Filter],
    signal_history: &[AlgedonicSignal],
) -> Value {
    let total = signal_history.len() as f64;
    let passed = signal_history
        .iter()
        .filter(|s| apply_filters(s, filters))
        .count() as f64;
    json!({
        "total_signals": total,
        "passed_signals": passed,
        "blocked_signals": total - passed,
        "pass_rate": if total == 0.0 { 1.0 } else { passed / total },
        "filter_count": filters.len()
    })
}

fn parse_severity(s: &str) -> Severity {
    match s {
        "critical" => Severity::Critical,
        "high" => Severity::High,
        "medium" => Severity::Medium,
        _ => Severity::Low,
    }
}

fn kind_name(kind: SignalKind) -> &'static str {
    match kind {
        SignalKind::Pain => "pain",
        SignalKind::Pleasure => "pleasure",
        SignalKind::Anomaly => "anomaly",
        SignalKind::Opportunity => "opportunity",
        SignalKind::Emergency => "emergency",
    }
}
