//! Resource allocation helper algorithms for System 3 defaults.
//!
//! These pure functions are examples retained from the JSON port. They are not
//! core System 3 semantics.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::util::{f64_map_from_value, mean, value_from_f64_map};

pub fn allocate(
    requests: &[Value],
    available: &Value,
    performance_data: &Value,
    policies: &[Value],
) -> Value {
    let mut remaining = f64_map_from_value(available);
    let strategy = determine_strategy(policies);
    let mut allocations = serde_json::Map::new();
    let mut prioritized = requests.to_vec();
    prioritized.sort_by(|a, b| {
        score_request(b, performance_data)
            .partial_cmp(&score_request(a, performance_data))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for req in prioritized {
        let unit = req
            .get("unit_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let requested = f64_map_from_value(req.get("resources").unwrap_or(&Value::Null));
        let mut granted = BTreeMap::new();
        let multiplier = match strategy.as_str() {
            "performance" => performance_multiplier(unit, performance_data),
            "fair_share" => 1.0 / (requests.len().max(1) as f64),
            "priority" => req
                .get("priority")
                .and_then(Value::as_f64)
                .unwrap_or(1.0)
                .max(0.1),
            _ => 1.0,
        };
        for (resource, amount) in requested {
            let want = amount * multiplier;
            let available = remaining.get(&resource).cloned().unwrap_or(0.0);
            let give = want.min(available).max(0.0);
            granted.insert(resource.clone(), give);
            *remaining.entry(resource).or_insert(0.0) -= give;
        }
        allocations.insert(unit.to_string(), value_from_f64_map(&granted));
    }
    let allocations = apply_constraints(Value::Object(allocations), policies);
    json!({
        "status": "ok",
        "strategy": strategy,
        "allocations": allocations,
        "remaining": value_from_f64_map(&remaining)
    })
}

pub fn calculate_available(pool: &Value, allocations: &Value) -> Value {
    let mut available = f64_map_from_value(pool);
    for allocation in allocations.as_object().into_iter().flat_map(|m| m.values()) {
        for (key, value) in f64_map_from_value(allocation) {
            *available.entry(key).or_insert(0.0) -= value;
        }
    }
    for value in available.values_mut() {
        *value = value.max(0.0);
    }
    value_from_f64_map(&available)
}

pub fn available(pool: &Value, requested: &Value) -> bool {
    let pool = f64_map_from_value(pool);
    f64_map_from_value(requested)
        .iter()
        .all(|(key, value)| pool.get(key).cloned().unwrap_or(0.0) >= *value)
}

pub fn deduct(pool: &Value, to_deduct: &Value) -> Value {
    calculate_available(pool, &json!({"deduct": to_deduct}))
}

pub fn optimize_distribution(current_allocations: &Value, total_resources: &Value) -> Value {
    let total = f64_map_from_value(total_resources);
    let units: Vec<_> = current_allocations
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    let share_count = units.len().max(1) as f64;
    let mut object = serde_json::Map::new();
    for unit in units {
        let allocation = total
            .iter()
            .map(|(key, value)| (key.clone(), value / share_count))
            .collect::<BTreeMap<_, _>>();
        object.insert(unit, value_from_f64_map(&allocation));
    }
    json!({
        "optimized_allocations": Value::Object(object),
        "efficiency": calculate_efficiency_scores(current_allocations)
    })
}

pub fn predict_resource_needs(historical_data: &[Value], lookahead_minutes: i64) -> Value {
    let mut by_resource: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for item in historical_data {
        for (key, value) in f64_map_from_value(item) {
            by_resource.entry(key).or_default().push(value);
        }
    }
    let mut predicted = BTreeMap::new();
    for (key, values) in by_resource {
        let trend = if values.len() > 1 {
            values.last().unwrap_or(&0.0) - values.first().unwrap_or(&0.0)
        } else {
            0.0
        };
        predicted.insert(
            key,
            mean(&values) + trend * (lookahead_minutes as f64 / 60.0),
        );
    }
    json!({
        "lookahead_minutes": lookahead_minutes,
        "predicted_needs": value_from_f64_map(&predicted)
    })
}

pub fn validate_allocations(allocations: &Value, constraints: &[Value]) -> Value {
    let mut violations = Vec::new();
    for constraint in constraints {
        if constraint.get("type").and_then(Value::as_str) == Some("total_limit") {
            let limit = f64_map_from_value(constraint.get("limit").unwrap_or(&Value::Null));
            let mut sum = BTreeMap::new();
            for allocation in allocations.as_object().into_iter().flat_map(|m| m.values()) {
                for (key, value) in f64_map_from_value(allocation) {
                    *sum.entry(key).or_insert(0.0) += value;
                }
            }
            for (key, value) in sum {
                if value > limit.get(&key).cloned().unwrap_or(f64::INFINITY) {
                    violations.push(json!({
                        "resource": key,
                        "actual": value,
                        "limit": limit.get(&key)
                    }));
                }
            }
        }
    }
    json!({"valid": violations.is_empty(), "violations": violations})
}

fn determine_strategy(policies: &[Value]) -> String {
    policies
        .iter()
        .find_map(|policy| policy.get("strategy").and_then(Value::as_str))
        .unwrap_or("adaptive")
        .to_string()
}

fn score_request(request: &Value, performance: &Value) -> f64 {
    request
        .get("priority")
        .and_then(Value::as_f64)
        .unwrap_or(1.0)
        * performance_multiplier(
            request.get("unit_id").and_then(Value::as_str).unwrap_or(""),
            performance,
        )
}

fn performance_multiplier(unit: &str, performance: &Value) -> f64 {
    performance
        .get(unit)
        .and_then(|value| value.get("score"))
        .and_then(Value::as_f64)
        .unwrap_or(1.0)
        .max(0.1)
}

fn apply_constraints(allocations: Value, _policies: &[Value]) -> Value {
    allocations
}

fn calculate_efficiency_scores(allocations: &Value) -> Value {
    let values: Vec<f64> = allocations
        .as_object()
        .into_iter()
        .flat_map(|map| map.values())
        .map(|value| f64_map_from_value(value).values().sum())
        .collect();
    json!({"mean": mean(&values), "unit_count": values.len()})
}
