use serde_json::{json, Value};
use crate::util::{as_f64, mean};

pub fn suggest_methods(variety_ratio: f64) -> Vec<&'static str> {
    if variety_ratio <= 1.0 { vec!["monitor"] } else if variety_ratio < 2.0 { vec!["filter", "summarize"] } else { vec!["aggregate", "filter", "sample", "prioritize"] }
}

pub fn filter(items: &[Value], strategy: &str, options: &Value) -> Vec<Value> {
    match strategy {
        "threshold" => filter_threshold(items, options),
        "priority" => filter_priority(items, options),
        "frequency" => filter_frequency(items, options),
        "recency" => filter_recency(items, options),
        "relevance" => filter_relevance(items, options),
        _ => items.to_vec(),
    }
}

fn filter_threshold(items: &[Value], options: &Value) -> Vec<Value> { let min = options.get("min").and_then(as_f64).unwrap_or(0.0); items.iter().filter(|v| value_score(v) >= min).cloned().collect() }
fn filter_priority(items: &[Value], options: &Value) -> Vec<Value> { let min = options.get("min_priority").and_then(as_f64).unwrap_or(0.5); items.iter().filter(|v| v.get("priority").and_then(as_f64).unwrap_or(0.0) >= min).cloned().collect() }
fn filter_frequency(items: &[Value], options: &Value) -> Vec<Value> { let limit = options.get("limit").and_then(|v| v.as_u64()).unwrap_or(items.len() as u64) as usize; items.iter().take(limit).cloned().collect() }
fn filter_recency(items: &[Value], options: &Value) -> Vec<Value> { filter_frequency(items, options) }
fn filter_relevance(items: &[Value], options: &Value) -> Vec<Value> { let min = options.get("min_relevance").and_then(as_f64).unwrap_or(0.0); items.iter().filter(|v| v.get("relevance").and_then(as_f64).unwrap_or(1.0) >= min).cloned().collect() }

pub fn aggregate(items: &[Value], strategy: &str, _options: &Value) -> Value {
    let values: Vec<f64> = items.iter().filter_map(as_f64).collect();
    match strategy {
        "sum" => json!(values.iter().sum::<f64>()),
        "average" => json!(mean(&values)),
        "max" => json!(values.into_iter().fold(f64::NEG_INFINITY, f64::max)),
        "min" => json!(values.into_iter().fold(f64::INFINITY, f64::min)),
        "mode" => items.first().cloned().unwrap_or(Value::Null),
        "weighted" => json!(mean(&values)),
        _ => json!({"count": items.len()}),
    }
}

pub fn summarize(items: &[Value], format: &str, options: &Value) -> Value {
    match format {
        "statistics" => { let vals: Vec<f64> = items.iter().filter_map(as_f64).collect(); json!({"count": items.len(), "mean": mean(&vals), "sum": vals.iter().sum::<f64>()}) }
        "categories" => json!({"categories": group_count(items, "category")}),
        "time_series" => json!({"points": items, "window": options.get("window")}),
        _ => json!({"count": items.len()}),
    }
}

fn group_count(items: &[Value], key: &str) -> Value {
    let mut obj = serde_json::Map::new();
    for item in items {
        let k = item.get(key).and_then(|v| v.as_str()).unwrap_or("unknown");
        let current = obj.get(k).and_then(|v| v.as_i64()).unwrap_or(0);
        obj.insert(k.into(), json!(current + 1));
    }
    Value::Object(obj)
}
fn value_score(v: &Value) -> f64 { as_f64(v).or_else(|| v.get("value").and_then(as_f64)).unwrap_or(0.0) }
