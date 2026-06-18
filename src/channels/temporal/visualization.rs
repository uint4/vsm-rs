//! Visualization data preparation for temporal-variety queries.
//!
//! The current implementation packages actor state and the requested output
//! format into JSON. Rendering and chart-specific transformations are left to
//! embedding applications or future adapters.

use serde_json::{json, Value};

pub fn prepare_data(state: &Value, opts: &Value) -> Value {
    json!({"format": opts.get("format").and_then(|v| v.as_str()).unwrap_or("json"), "state": state})
}
