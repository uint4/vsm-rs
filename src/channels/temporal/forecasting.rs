use serde_json::{json, Value};

use super::patterns;
use super::timescales::Timescales;
use crate::util::{mean, numeric_values};

pub fn generate_forecasts(timescales: &Timescales, horizons: &[usize]) -> Value {
    let mut result = serde_json::Map::new();
    for (scale, buffer) in &timescales.windows {
        let data: Vec<f64> = buffer.iter().flat_map(|m| numeric_values(&m.data)).collect();
        let base = mean(&data);
        let trend = patterns::detect_trends(&data).get("slope").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let forecasts: Vec<_> = horizons.iter().map(|h| json!({"horizon": h, "value": base + trend * *h as f64})).collect();
        result.insert(scale.clone(), json!(forecasts));
    }
    Value::Object(result)
}

pub fn update(timescales: &Timescales, patterns: &Value) -> Value {
    json!({"updated_at": chrono::Utc::now(), "patterns": patterns, "scale_count": timescales.windows.len()})
}

pub fn detect_anomalies(actual_data: &Value, forecasts: &Value) -> Value {
    json!({"actual": actual_data, "forecasts": forecasts, "anomaly_count": 0})
}

pub fn ensemble_forecast(timescales: &Timescales, horizon: usize) -> Value {
    generate_forecasts(timescales, &[horizon])
}
