use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::util::{f64_map_from_value, mean, value_from_f64_map};

pub fn allocate(requests: &[Value], available: &Value, performance_data: &Value, policies: &[Value]) -> Value {
    let mut remaining = f64_map_from_value(available);
    let strategy = determine_strategy(policies);
    let mut allocations = serde_json::Map::new();
    let mut prioritized = requests.to_vec();
    prioritized.sort_by(|a,b| score_request(b, performance_data).partial_cmp(&score_request(a, performance_data)).unwrap_or(std::cmp::Ordering::Equal));
    for req in prioritized {
        let unit = req.get("unit_id").and_then(Value::as_str).unwrap_or("unknown");
        let requested = f64_map_from_value(req.get("resources").unwrap_or(&Value::Null));
        let mut granted = BTreeMap::new();
        let multiplier = match strategy.as_str() { "performance" => performance_multiplier(unit, performance_data), "fair_share" => 1.0 / (requests.len().max(1) as f64), "priority" => req.get("priority").and_then(Value::as_f64).unwrap_or(1.0).max(0.1), _ => 1.0 };
        for (res, amount) in requested {
            let want = amount * multiplier;
            let avail = remaining.get(&res).cloned().unwrap_or(0.0);
            let give = want.min(avail).max(0.0);
            granted.insert(res.clone(), give);
            *remaining.entry(res).or_insert(0.0) -= give;
        }
        allocations.insert(unit.to_string(), value_from_f64_map(&granted));
    }
    let allocations = apply_constraints(Value::Object(allocations), policies);
    json!({"status":"ok", "strategy": strategy, "allocations": allocations, "remaining": value_from_f64_map(&remaining)})
}

pub fn calculate_available(pool: &Value, allocations: &Value) -> Value {
    let mut available = f64_map_from_value(pool);
    for alloc in allocations.as_object().into_iter().flat_map(|m| m.values()) {
        for (k, v) in f64_map_from_value(alloc) { *available.entry(k).or_insert(0.0) -= v; }
    }
    for v in available.values_mut() { *v = v.max(0.0); }
    value_from_f64_map(&available)
}

pub fn available(pool: &Value, requested: &Value) -> bool { let p=f64_map_from_value(pool); f64_map_from_value(requested).iter().all(|(k,v)| p.get(k).cloned().unwrap_or(0.0) >= *v) }
pub fn deduct(pool: &Value, to_deduct: &Value) -> Value { calculate_available(pool, &json!({"deduct": to_deduct})) }

pub fn optimize_distribution(current_allocations: &Value, total_resources: &Value) -> Value {
    let total = f64_map_from_value(total_resources);
    let units: Vec<_> = current_allocations.as_object().map(|m| m.keys().cloned().collect()).unwrap_or_default();
    let share_count = units.len().max(1) as f64;
    let mut obj = serde_json::Map::new();
    for unit in units {
        let allocation = total.iter().map(|(k,v)| (k.clone(), v/share_count)).collect::<BTreeMap<_,_>>();
        obj.insert(unit, value_from_f64_map(&allocation));
    }
    json!({"optimized_allocations": Value::Object(obj), "efficiency": calculate_efficiency_scores(current_allocations)})
}

pub fn predict_resource_needs(historical_data: &[Value], lookahead_minutes: i64) -> Value {
    let mut by_res: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for item in historical_data { for (k,v) in f64_map_from_value(item) { by_res.entry(k).or_default().push(v); } }
    let mut pred = BTreeMap::new();
    for (k, values) in by_res { let trend = if values.len()>1 { values.last().unwrap()-values.first().unwrap() } else {0.0}; pred.insert(k, mean(&values) + trend * (lookahead_minutes as f64 / 60.0)); }
    json!({"lookahead_minutes": lookahead_minutes, "predicted_needs": value_from_f64_map(&pred)})
}

pub fn validate_allocations(allocations: &Value, constraints: &[Value]) -> Value {
    let mut violations = Vec::new();
    for c in constraints {
        if c.get("type").and_then(Value::as_str) == Some("total_limit") {
            let limit = f64_map_from_value(c.get("limit").unwrap_or(&Value::Null));
            let mut sum = BTreeMap::new();
            for alloc in allocations.as_object().into_iter().flat_map(|m| m.values()) { for (k,v) in f64_map_from_value(alloc){*sum.entry(k).or_insert(0.0)+=v;} }
            for (k,v) in sum { if v > limit.get(&k).cloned().unwrap_or(f64::INFINITY) { violations.push(json!({"resource":k,"actual":v,"limit":limit.get(&k)})); } }
        }
    }
    json!({"valid": violations.is_empty(), "violations": violations})
}

fn determine_strategy(policies:&[Value])->String{ policies.iter().find_map(|p| p.get("strategy").and_then(Value::as_str)).unwrap_or("adaptive").to_string() }
fn score_request(req:&Value, perf:&Value)->f64{ req.get("priority").and_then(Value::as_f64).unwrap_or(1.0) * performance_multiplier(req.get("unit_id").and_then(Value::as_str).unwrap_or(""), perf) }
fn performance_multiplier(unit:&str, perf:&Value)->f64{ perf.get(unit).and_then(|v|v.get("score")).and_then(Value::as_f64).unwrap_or(1.0).max(0.1) }
fn apply_constraints(allocations:Value, _policies:&[Value])->Value{ allocations }
fn calculate_efficiency_scores(allocations:&Value)->Value{ let vals:Vec<f64>=allocations.as_object().into_iter().flat_map(|m| m.values()).map(|v| f64_map_from_value(v).values().sum()).collect(); json!({"mean":mean(&vals),"unit_count":vals.len()}) }
