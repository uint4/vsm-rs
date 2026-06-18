//! Numeric and JSON helpers used by pure algorithms.
//!
//! The helper functions convert permissive JSON inputs into numeric values,
//! summarize small numeric slices, and deep-merge JSON objects for policy and
//! identity state updates. They favor predictable fallback values over strict
//! schema enforcement because callers validate schemas at higher-level API
//! boundaries.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

pub fn as_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

pub fn map_get_f64(value: &Value, key: &str) -> Option<f64> {
    value.as_object()?.get(key).and_then(as_f64)
}

pub fn f64_map_from_value(value: &Value) -> BTreeMap<String, f64> {
    value
        .as_object()
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| as_f64(v).map(|n| (k.clone(), n)))
                .collect()
        })
        .unwrap_or_default()
}

pub fn value_from_f64_map(map: &BTreeMap<String, f64>) -> Value {
    let mut obj = Map::new();
    for (k, v) in map {
        obj.insert(k.clone(), Value::from(*v));
    }
    Value::Object(obj)
}

pub fn numeric_values(value: &Value) -> Vec<f64> {
    match value {
        Value::Array(items) => items.iter().filter_map(as_f64).collect(),
        Value::Object(obj) => obj.values().filter_map(as_f64).collect(),
        other => as_f64(other).into_iter().collect(),
    }
}

pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

pub fn variance(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let m = mean(values);
    values.iter().map(|v| (v - m).powi(2)).sum::<f64>() / values.len() as f64
}

pub fn std_dev(values: &[f64]) -> f64 {
    variance(values).sqrt()
}

pub fn clamp01(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

pub fn deep_merge(a: &mut Value, b: &Value) {
    match (a, b) {
        (Value::Object(a_obj), Value::Object(b_obj)) => {
            for (k, b_val) in b_obj {
                match a_obj.get_mut(k) {
                    Some(a_val) => deep_merge(a_val, b_val),
                    None => {
                        a_obj.insert(k.clone(), b_val.clone());
                    }
                }
            }
        }
        (a_val, b_val) => *a_val = b_val.clone(),
    }
}
