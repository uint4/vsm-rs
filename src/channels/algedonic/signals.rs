use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalKind { Pain, Pleasure, Anomaly, Opportunity, Emergency }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity { Low, Medium, High, Critical }

impl Severity {
    pub fn score(self) -> f64 {
        match self { Self::Low => 0.25, Self::Medium => 0.5, Self::High => 0.75, Self::Critical => 1.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgedonicSignal {
    pub id: String,
    pub kind: SignalKind,
    pub source: String,
    pub data: Value,
    pub severity: Severity,
    pub urgency: f64,
    pub priority: f64,
    pub timestamp: DateTime<Utc>,
    pub context: Value,
}

pub fn create_signal(kind: SignalKind, source: impl Into<String>, data: Value, severity: Severity) -> AlgedonicSignal {
    let urgency = data.get("urgency").and_then(|v| v.as_f64()).unwrap_or_else(|| severity.score());
    let mut signal = AlgedonicSignal {
        id: format!("sig_{}", Uuid::new_v4()),
        kind,
        source: source.into(),
        data,
        severity,
        urgency,
        priority: 0.0,
        timestamp: Utc::now(),
        context: json!({}),
    };
    signal.priority = calculate_priority(&signal);
    signal
}

pub fn create_aggregated_signal(pattern: &Value) -> AlgedonicSignal {
    let source = pattern.get("source").and_then(|v| v.as_str()).unwrap_or("aggregate");
    create_signal(SignalKind::Anomaly, source, pattern.clone(), Severity::High)
}

pub fn validate_signal(signal: &AlgedonicSignal) -> Result<(), String> {
    if signal.source.trim().is_empty() { return Err("source is empty".into()); }
    if !(0.0..=1.0).contains(&signal.urgency) { return Err("urgency must be 0..1".into()); }
    Ok(())
}

pub fn requires_emergency_bypass(signal: &AlgedonicSignal) -> bool {
    signal.severity == Severity::Critical || signal.urgency >= 0.9 || signal.priority >= 0.9
}

pub fn classify_signal(signal: &AlgedonicSignal) -> &'static str {
    match (signal.kind, signal.severity) {
        (SignalKind::Emergency, _) | (_, Severity::Critical) => "emergency",
        (SignalKind::Pain, Severity::High) => "threat",
        (SignalKind::Pleasure, Severity::High) => "opportunity",
        (SignalKind::Anomaly, _) => "anomaly",
        _ => "routine",
    }
}

pub fn calculate_priority(signal: &AlgedonicSignal) -> f64 {
    let kind_weight = match signal.kind {
        SignalKind::Emergency => 1.0,
        SignalKind::Pain => 0.85,
        SignalKind::Anomaly => 0.7,
        SignalKind::Opportunity => 0.6,
        SignalKind::Pleasure => 0.45,
    };
    ((signal.severity.score() * 0.45) + (signal.urgency * 0.35) + (kind_weight * 0.20)).min(1.0)
}

pub fn enrich_signal(mut signal: AlgedonicSignal, context: Value) -> AlgedonicSignal {
    signal.context = context;
    signal.priority = calculate_priority(&signal);
    signal
}

pub fn filter_signals(signals: &[AlgedonicSignal], criteria: &Value) -> Vec<AlgedonicSignal> {
    let min_priority = criteria.get("min_priority").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let source = criteria.get("source").and_then(|v| v.as_str());
    signals
        .iter()
        .filter(|s| s.priority >= min_priority)
        .filter(|s| source.map(|wanted| wanted == s.source).unwrap_or(true))
        .cloned()
        .collect()
}

pub fn group_signals(signals: &[AlgedonicSignal], by: &str) -> Value {
    let mut groups = serde_json::Map::new();
    for signal in signals {
        let key = match by { "source" => signal.source.clone(), "severity" => format!("{:?}", signal.severity), "kind" => format!("{:?}", signal.kind), _ => "all".into() };
        groups.entry(key).or_insert_with(|| json!([])).as_array_mut().unwrap().push(json!(signal));
    }
    Value::Object(groups)
}

pub fn parse_severity(value: &str) -> Severity {
    match value {
        "critical" => Severity::Critical,
        "high" => Severity::High,
        "low" => Severity::Low,
        _ => Severity::Medium,
    }
}
