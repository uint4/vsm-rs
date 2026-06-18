//! Resource balancing helpers for System 2.
//!
//! The balancer ranks JSON resource requests, derives or reuses total capacity,
//! allocates resources by priority, and reports simple efficiency and imbalance
//! summaries. It is a pure helper module and does not mutate actor state by
//! itself.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::util::{f64_map_from_value, mean, value_from_f64_map};

pub fn balance(requests: &[Value], current_allocations: &Value) -> Value {
    let total = total_resources(requests, current_allocations);
    let current = object_map(current_allocations);
    let sorted = sort_requests(requests);
    let mut remaining = total.clone();
    let mut allocations = serde_json::Map::new();

    for req in sorted {
        let unit = req.get("unit_id").and_then(Value::as_str).unwrap_or("unknown").to_string();
        let requested = f64_map_from_value(req.get("resources").unwrap_or(&Value::Null));
        let priority = req.get("priority").and_then(Value::as_f64).unwrap_or(1.0).max(0.1);
        let current_for_unit = current.get(&unit).cloned().unwrap_or_default();
        let allocation = allocate_one(&requested, &remaining, &current_for_unit, priority);
        subtract_from(&mut remaining, &allocation);
        allocations.insert(unit, value_from_f64_map(&allocation));
    }

    let mut result = Value::Object(allocations);
    if !validate_allocations(&result, &value_from_f64_map(&total)) {
        result = scale_to_total(&result, &total);
    }

    json!({"status":"ok", "allocations": result, "efficiency": calculate_efficiency(&result), "remaining": value_from_f64_map(&remaining)})
}

pub fn calculate_efficiency(allocations: &Value) -> Value {
    let allocations = object_map(allocations);
    let totals = sum_allocations_map(&allocations);
    let mut utilizations = Vec::new();
    for alloc in allocations.values() {
        let amount = alloc.values().sum::<f64>();
        let denom = totals.values().sum::<f64>().max(1.0);
        utilizations.push(amount / denom);
    }
    json!({
        "total_allocated": value_from_f64_map(&totals),
        "balance_score": balance_score(&allocations),
        "mean_utilization": mean(&utilizations),
        "unit_count": allocations.len()
    })
}

pub fn detect_imbalance(allocations: &Value) -> Value {
    let allocations = object_map(allocations);
    let means = resource_means(&allocations);
    let mut over = Vec::new();
    let mut under = Vec::new();
    for (unit, alloc) in &allocations {
        let score = alloc.values().sum::<f64>();
        let avg = means.values().sum::<f64>().max(1.0);
        if score > avg * 1.2 { over.push(json!({"unit_id": unit, "score": score})); }
        if score < avg * 0.8 { under.push(json!({"unit_id": unit, "score": score})); }
    }
    json!({"imbalanced": !over.is_empty() || !under.is_empty(), "overallocated": over, "underallocated": under, "means": value_from_f64_map(&means)})
}

pub fn suggest_rebalancing(allocations: &Value) -> Value {
    let imbalance = detect_imbalance(allocations);
    let mut suggestions = Vec::new();
    let over = imbalance.get("overallocated").and_then(Value::as_array).cloned().unwrap_or_default();
    let under = imbalance.get("underallocated").and_then(Value::as_array).cloned().unwrap_or_default();
    for from in over {
        for to in &under {
            suggestions.push(json!({
                "action": "transfer_capacity",
                "from": from.get("unit_id").cloned().unwrap_or(Value::Null),
                "to": to.get("unit_id").cloned().unwrap_or(Value::Null),
                "amount_hint": ((from.get("score").and_then(Value::as_f64).unwrap_or(0.0) - to.get("score").and_then(Value::as_f64).unwrap_or(0.0)).abs() / 2.0)
            }));
        }
    }
    json!({"suggestions": suggestions, "imbalance": imbalance})
}

fn total_resources(requests: &[Value], current_allocations: &Value) -> BTreeMap<String, f64> {
    let mut total = sum_allocations_map(&object_map(current_allocations));
    if total.is_empty() {
        for req in requests {
            for (k, v) in f64_map_from_value(req.get("resources").unwrap_or(&Value::Null)) {
                *total.entry(k).or_insert(0.0) += v * 1.25;
            }
        }
    }
    if total.is_empty() { total.insert("capacity".into(), 100.0); }
    total
}

fn sort_requests(requests: &[Value]) -> Vec<Value> {
    let mut out = requests.to_vec();
    out.sort_by(|a, b| b.get("priority").and_then(Value::as_f64).unwrap_or(0.0).partial_cmp(&a.get("priority").and_then(Value::as_f64).unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));
    out
}

fn allocate_one(requested: &BTreeMap<String, f64>, remaining: &BTreeMap<String, f64>, current: &BTreeMap<String, f64>, priority: f64) -> BTreeMap<String, f64> {
    let mut allocation = BTreeMap::new();
    for (resource, amount) in requested {
        let available = remaining.get(resource).cloned().unwrap_or(0.0);
        let existing = current.get(resource).cloned().unwrap_or(0.0);
        let target = (amount * priority).min(available + existing);
        allocation.insert(resource.clone(), target.max(0.0));
    }
    allocation
}

fn subtract_from(base: &mut BTreeMap<String, f64>, subtract: &BTreeMap<String, f64>) { for (k, v) in subtract { *base.entry(k.clone()).or_insert(0.0) -= *v; if let Some(x) = base.get_mut(k) { *x = x.max(0.0); } } }
fn object_map(value: &Value) -> BTreeMap<String, BTreeMap<String, f64>> { value.as_object().map(|o| o.iter().map(|(k,v)| (k.clone(), f64_map_from_value(v))).collect()).unwrap_or_default() }
fn sum_allocations_map(allocations: &BTreeMap<String, BTreeMap<String, f64>>) -> BTreeMap<String, f64> { let mut total=BTreeMap::new(); for alloc in allocations.values() { for (k,v) in alloc { *total.entry(k.clone()).or_insert(0.0)+=*v; } } total }
fn resource_means(allocations: &BTreeMap<String, BTreeMap<String, f64>>) -> BTreeMap<String, f64> { let mut t=sum_allocations_map(allocations); let n=allocations.len().max(1) as f64; for v in t.values_mut(){*v/=n;} t }
fn balance_score(allocations: &BTreeMap<String, BTreeMap<String, f64>>) -> f64 { let totals: Vec<f64> = allocations.values().map(|m| m.values().sum()).collect(); if totals.is_empty(){1.0}else{ let m=mean(&totals); let var=totals.iter().map(|v|(v-m).powi(2)).sum::<f64>()/totals.len() as f64; 1.0/(1.0+var.sqrt()) } }
fn validate_allocations(allocations: &Value, total: &Value) -> bool { let a=sum_allocations_map(&object_map(allocations)); let t=f64_map_from_value(total); a.iter().all(|(k,v)| *v <= t.get(k).cloned().unwrap_or(f64::INFINITY) + 0.001) }
fn scale_to_total(allocations: &Value, total: &BTreeMap<String, f64>) -> Value { let mut maps=object_map(allocations); let sum=sum_allocations_map(&maps); for (res, actual) in sum { let limit=total.get(&res).cloned().unwrap_or(actual); if actual > limit && actual > 0.0 { let factor=limit/actual; for alloc in maps.values_mut(){ if let Some(v)=alloc.get_mut(&res){*v*=factor;} } } } let obj=maps.into_iter().map(|(k,v)|(k,value_from_f64_map(&v))).collect(); Value::Object(obj) }
