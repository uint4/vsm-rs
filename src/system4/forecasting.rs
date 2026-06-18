//! Forecasting helpers and service operations for System 4.
//!
//! Forecasts are generated from JSON history with simple linear, mean, or naive
//! models. Scenario generation and validation operate on JSON forecast shapes.
//! The service's model list is held in the actor's in-memory JSON state.

use serde_json::{json, Value};

use crate::util::{as_f64, mean};

pub fn forecast(history: &[Value], horizon: usize, model: &str) -> Value {
    let vals: Vec<f64> = history
        .iter()
        .filter_map(|v| v.get("value").and_then(as_f64).or_else(|| as_f64(v)))
        .collect();
    let last = vals.last().cloned().unwrap_or(0.0);
    let trend = if vals.len() > 1 {
        (vals.last().unwrap() - vals.first().unwrap()) / (vals.len() as f64 - 1.0)
    } else {
        0.0
    };
    let points: Vec<Value> = (1..=horizon).map(|i| json!({"step": i, "value": match model {"mean" => mean(&vals), "naive" => last, _ => last + trend * i as f64}, "confidence": (1.0 - i as f64/(horizon.max(1) as f64*2.0)).max(0.1)})).collect();
    json!({"model": model, "horizon": horizon, "forecast": points})
}

pub fn generate_scenarios(base_forecast: &Value, options: &Value) -> Value {
    let delta = options
        .get("scenario_delta")
        .and_then(Value::as_f64)
        .unwrap_or(0.15);
    json!({"base": base_forecast, "optimistic": adjust(base_forecast, 1.0+delta), "pessimistic": adjust(base_forecast, 1.0-delta)})
}

pub fn validate_forecast(forecast: &Value, actuals: &[Value]) -> Value {
    let f = forecast
        .get("forecast")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let n = f.len().min(actuals.len());
    let err = if n == 0 {
        0.0
    } else {
        (0..n)
            .map(|i| {
                (f[i].get("value").and_then(as_f64).unwrap_or(0.0)
                    - actuals[i]
                        .get("value")
                        .and_then(as_f64)
                        .or_else(|| as_f64(&actuals[i]))
                        .unwrap_or(0.0))
                .abs()
            })
            .sum::<f64>()
            / n as f64
    };
    json!({"sample_size": n, "mae": err})
}

pub async fn actor_call(
    op: &str,
    payload: Value,
    state: &mut crate::actor_support::ServiceState,
) -> crate::error::VsmResult<Value> {
    match op {
        "forecast" => Ok(forecast(
            &payload
                .get("history")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            payload.get("horizon").and_then(Value::as_u64).unwrap_or(10) as usize,
            payload
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or("linear"),
        )),
        "scenarios" => Ok(generate_scenarios(
            payload.get("base_forecast").unwrap_or(&Value::Null),
            &payload,
        )),
        "validate" => Ok(validate_forecast(
            payload.get("forecast").unwrap_or(&Value::Null),
            &payload
                .get("actuals")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
        )),
        "models" => Ok(state
            .data
            .get("models")
            .cloned()
            .unwrap_or_else(|| json!(["linear", "mean", "naive"]))),
        _ => Ok(json!({"status":"unknown_operation", "op":op})),
    }
}

fn adjust(forecast: &Value, factor: f64) -> Value {
    let mut f = forecast.clone();
    if let Some(arr) = f.get_mut("forecast").and_then(Value::as_array_mut) {
        for p in arr {
            if let Some(v) = p.get_mut("value") {
                if let Some(x) = as_f64(v) {
                    *v = json!(x * factor);
                }
            }
        }
    }
    f
}
