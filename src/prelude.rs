//! Small JSON and time helpers shared across modules.
//!
//! These functions keep common timestamp generation, keyword extraction, and
//! permissive JSON field access in one place. They are intentionally lightweight
//! utilities, not a public prelude that applications are expected to glob-import.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

pub fn now() -> DateTime<Utc> {
    Utc::now()
}

pub fn now_json() -> Value {
    json!(Utc::now())
}

pub fn now_string() -> String {
    Utc::now().to_rfc3339()
}

pub fn value_keywords(value: &Value) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    collect_keywords(value, &mut out);
    out
}

fn collect_keywords(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::String(s) => {
            for word in s
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .filter(|w| w.len() >= 3)
            {
                out.insert(word.to_ascii_lowercase());
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_keywords(item, out);
            }
        }
        Value::Object(map) => {
            for (key, value) in map {
                out.insert(key.to_ascii_lowercase());
                collect_keywords(value, out);
            }
        }
        _ => {}
    }
}

pub fn value_array(value: &Value, key: &str) -> Vec<Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

pub fn value_string(value: &Value, key: &str, default: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}
