//! Descriptive routing rules for algedonic signals.
//!
//! Routing selects a destination string and path description from signal
//! priority and emergency status. The result is recorded with alerts but is not
//! itself actor delivery; broker publication is a separate step.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::signals::{requires_emergency_bypass, AlgedonicSignal, Severity};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub name: String,
    pub destination: String,
    pub min_priority: f64,
    pub emergency_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    pub destination: String,
    pub path: Vec<String>,
    pub bypass: bool,
    pub reason: String,
}

pub fn default_rules() -> Vec<RoutingRule> {
    vec![
        RoutingRule {
            name: "critical_to_system5".into(),
            destination: "system5".into(),
            min_priority: 0.85,
            emergency_only: false,
        },
        RoutingRule {
            name: "high_to_policy".into(),
            destination: "system5.policy".into(),
            min_priority: 0.70,
            emergency_only: false,
        },
        RoutingRule {
            name: "routine_to_s3".into(),
            destination: "system3".into(),
            min_priority: 0.00,
            emergency_only: false,
        },
    ]
}

pub fn emergency_route(signal: &AlgedonicSignal) -> RouteInfo {
    RouteInfo {
        destination: "system5".into(),
        path: vec!["algedonic".into(), "system5".into()],
        bypass: true,
        reason: format!("emergency priority {:.2}", signal.priority),
    }
}

pub fn route_signal(signal: &AlgedonicSignal, routing_rules: &[RoutingRule]) -> RouteInfo {
    if requires_emergency_bypass(signal) || signal.severity == Severity::Critical {
        return emergency_route(signal);
    }
    let rule = routing_rules
        .iter()
        .filter(|r| !r.emergency_only && signal.priority >= r.min_priority)
        .max_by(|a, b| a.min_priority.partial_cmp(&b.min_priority).unwrap())
        .cloned();
    let dest = rule
        .map(|r| r.destination)
        .unwrap_or_else(|| "system3".into());
    RouteInfo {
        path: calculate_route_path(signal, &dest, "direct"),
        destination: dest,
        bypass: false,
        reason: "rule_match".into(),
    }
}

pub fn calculate_route_path(
    _signal: &AlgedonicSignal,
    destination: &str,
    strategy: &str,
) -> Vec<String> {
    match strategy {
        "hierarchical" => vec![
            "algedonic".into(),
            "system3".into(),
            "system4".into(),
            destination.into(),
        ],
        _ => vec!["algedonic".into(), destination.into()],
    }
}

pub fn analyze_routing_patterns(history: &[RouteInfo]) -> Value {
    let bypasses = history.iter().filter(|r| r.bypass).count();
    let mut by_dest = serde_json::Map::new();
    for item in history {
        let current = by_dest
            .get(&item.destination)
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        by_dest.insert(item.destination.clone(), json!(current + 1));
    }
    json!({"routes": history.len(), "emergency_bypasses": bypasses, "by_destination": by_dest})
}

pub fn validate_routing_rules(rules: &[RoutingRule]) -> Result<(), String> {
    if rules.is_empty() {
        return Err("no routing rules".into());
    }
    for r in rules {
        if r.destination.is_empty() {
            return Err(format!("rule {} has no destination", r.name));
        }
    }
    Ok(())
}
