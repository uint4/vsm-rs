use serde_json::{json, Value};

pub fn prepare_data(state: &Value, opts: &Value) -> Value {
    json!({"format": opts.get("format").and_then(|v| v.as_str()).unwrap_or("json"), "state": state})
}
