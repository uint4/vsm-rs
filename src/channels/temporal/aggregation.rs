use serde_json::{json, Value};

use super::timescales::Timescales;

pub fn generate_summary(state: &Value) -> Value { json!({"summary": state, "generated_at": chrono::Utc::now()}) }
pub fn hierarchical_aggregation(timescales: &Timescales) -> Value { json!({"scales": timescales.windows.keys().cloned().collect::<Vec<_>>()}) }
pub fn dimensional_aggregation(buffer: &[Value]) -> Value { json!({"count": buffer.len(), "dimensions": []}) }
pub fn time_based_summary(state: &Value, start_time: chrono::DateTime<chrono::Utc>, end_time: chrono::DateTime<chrono::Utc>) -> Value { json!({"state": state, "start_time": start_time, "end_time": end_time}) }
