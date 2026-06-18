//! Opt-in JSON helper algorithms for examples and prototypes.
//!
//! These helpers are not part of the typed System 4 control path. Applications
//! can call them from their own role implementations when simple heuristics are
//! useful during prototyping.

use serde_json::{json, Value};

use crate::prelude::now_json;
use crate::util::{as_f64, mean, std_dev};

/// Produces a simple scan report from JSON-like sources.
pub fn scan_environment(sources: &[Value], options: &Value) -> Value {
    let signals: Vec<Value> = sources
        .iter()
        .flat_map(|source| extract_signals(source, options))
        .collect();
    json!({
        "scanned_at": now_json(),
        "source_count": sources.len(),
        "signals": signals.clone(),
        "classification": classify_signals(&signals),
    })
}

/// Detects changed object fields between two JSON records.
pub fn detect_changes(current: &Value, previous: &Value) -> Value {
    let mut changes = Vec::new();
    if let (Some(current), Some(previous)) = (current.as_object(), previous.as_object()) {
        for (key, current_value) in current {
            let previous_value = previous.get(key).unwrap_or(&Value::Null);
            if current_value != previous_value {
                changes.push(json!({
                    "field": key,
                    "from": previous_value,
                    "to": current_value,
                    "magnitude": magnitude(current_value, previous_value),
                }));
            }
        }
    }
    let change_count = changes.len();
    json!({ "changes": changes, "change_count": change_count })
}

/// Classifies JSON signals into simple opportunity, threat, and weak buckets.
pub fn classify_signals(signals: &[Value]) -> Value {
    let mut opportunities = Vec::new();
    let mut threats = Vec::new();
    let mut weak = Vec::new();

    for signal in signals {
        let value = signal
            .get("value")
            .and_then(as_f64)
            .or_else(|| signal.get("score").and_then(as_f64))
            .unwrap_or(0.0);
        if value > 0.65 {
            opportunities.push(signal.clone());
        } else if value < -0.35 {
            threats.push(signal.clone());
        } else {
            weak.push(signal.clone());
        }
    }

    json!({
        "opportunities": opportunities,
        "threats": threats,
        "weak_signals": weak,
    })
}

/// Summarizes a numeric JSON history.
pub fn monitor_trends(history: &[Value]) -> Value {
    let values: Vec<f64> = history
        .iter()
        .filter_map(|value| value.get("value").and_then(as_f64))
        .collect();
    let trend = values.last().unwrap_or(&0.0) - values.first().unwrap_or(&0.0);
    json!({
        "points": values.len(),
        "mean": mean(&values),
        "trend": trend,
        "direction": if trend > 0.0 {
            "up"
        } else if trend < 0.0 {
            "down"
        } else {
            "flat"
        },
    })
}

/// Runs a simple JSON analytics helper.
pub fn analyze(data: &[Value], analysis_type: &str, options: &Value) -> Value {
    match analysis_type {
        "trend" => trend_analysis(data),
        "correlation" => correlate(data, options),
        "anomaly" => json!({ "anomalies": detect_anomalies(data, options) }),
        "insight" => generate_insights(data, options),
        _ => json!({ "summary": summary(data), "generated_at": now_json() }),
    }
}

/// Computes a simple correlation between `x` and `y` fields.
pub fn correlate(data: &[Value], _options: &Value) -> Value {
    let xs: Vec<f64> = data
        .iter()
        .filter_map(|value| value.get("x").and_then(as_f64))
        .collect();
    let ys: Vec<f64> = data
        .iter()
        .filter_map(|value| value.get("y").and_then(as_f64))
        .collect();
    let sample_size = xs.len().min(ys.len());
    let correlation = if sample_size == 0 {
        0.0
    } else {
        let x_mean = mean(&xs[..sample_size]);
        let y_mean = mean(&ys[..sample_size]);
        let numerator = (0..sample_size)
            .map(|index| (xs[index] - x_mean) * (ys[index] - y_mean))
            .sum::<f64>();
        let denominator = ((0..sample_size)
            .map(|index| (xs[index] - x_mean).powi(2))
            .sum::<f64>()
            * (0..sample_size)
                .map(|index| (ys[index] - y_mean).powi(2))
                .sum::<f64>())
        .sqrt()
        .max(1e-9);
        numerator / denominator
    };
    json!({ "correlation": correlation, "sample_size": sample_size })
}

/// Detects simple z-score anomalies in JSON numeric records.
pub fn detect_anomalies(data: &[Value], options: &Value) -> Vec<Value> {
    let threshold = options
        .get("z_threshold")
        .and_then(Value::as_f64)
        .unwrap_or(2.0);
    let values: Vec<f64> = data
        .iter()
        .filter_map(|value| {
            value
                .get("value")
                .and_then(as_f64)
                .or_else(|| as_f64(value))
        })
        .collect();
    let average = mean(&values);
    let deviation = std_dev(&values).max(1e-9);
    data.iter()
        .filter(|value| {
            let observed = value
                .get("value")
                .and_then(as_f64)
                .or_else(|| as_f64(value))
                .unwrap_or(average);
            ((observed - average) / deviation).abs() >= threshold
        })
        .cloned()
        .collect()
}

/// Generates simple JSON insights from a data set.
pub fn generate_insights(data: &[Value], options: &Value) -> Value {
    let summary = summary(data);
    let anomalies = detect_anomalies(data, options);
    json!({
        "summary": summary,
        "anomalies": anomalies,
        "recommendations": recommendations(data),
    })
}

/// Direct trend helper retained for examples.
pub async fn analyze_trends(
    data: Value,
    _window: impl Into<String>,
) -> crate::error::VsmResult<Value> {
    let values = data.as_array().cloned().unwrap_or_default();
    let nums: Vec<f64> = values
        .iter()
        .filter_map(|value| {
            value
                .get("value")
                .and_then(as_f64)
                .or_else(|| as_f64(value))
        })
        .collect();
    let delta = nums.last().copied().unwrap_or(0.0) - nums.first().copied().unwrap_or(0.0);
    Ok(json!({
        "direction": if delta > 0.0 {
            "increasing"
        } else if delta < 0.0 {
            "decreasing"
        } else {
            "stable"
        },
        "delta": delta,
        "points": nums.len(),
    }))
}

/// Produces a simple JSON forecast from numeric history.
pub fn forecast(history: &[Value], horizon: usize, model: &str) -> Value {
    let values: Vec<f64> = history
        .iter()
        .filter_map(|value| {
            value
                .get("value")
                .and_then(as_f64)
                .or_else(|| as_f64(value))
        })
        .collect();
    let last = values.last().copied().unwrap_or(0.0);
    let trend = if values.len() > 1 {
        (values.last().copied().unwrap_or(0.0) - values.first().copied().unwrap_or(0.0))
            / (values.len() as f64 - 1.0)
    } else {
        0.0
    };
    let points: Vec<Value> = (1..=horizon)
        .map(|step| {
            json!({
                "step": step,
                "value": match model {
                    "mean" => mean(&values),
                    "naive" => last,
                    _ => last + trend * step as f64,
                },
                "confidence": (1.0 - step as f64 / (horizon.max(1) as f64 * 2.0)).max(0.1),
            })
        })
        .collect();
    json!({ "model": model, "horizon": horizon, "forecast": points })
}

/// Generates simple optimistic and pessimistic JSON scenarios.
pub fn generate_scenarios(base_forecast: &Value, options: &Value) -> Value {
    let delta = options
        .get("scenario_delta")
        .and_then(Value::as_f64)
        .unwrap_or(0.15);
    json!({
        "base": base_forecast,
        "optimistic": adjust(base_forecast, 1.0 + delta),
        "pessimistic": adjust(base_forecast, 1.0 - delta),
    })
}

/// Compares a JSON forecast with actual observations using mean absolute error.
pub fn validate_forecast(forecast: &Value, actuals: &[Value]) -> Value {
    let forecast_points = forecast
        .get("forecast")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let sample_size = forecast_points.len().min(actuals.len());
    let error = if sample_size == 0 {
        0.0
    } else {
        (0..sample_size)
            .map(|index| {
                (forecast_points[index]
                    .get("value")
                    .and_then(as_f64)
                    .unwrap_or(0.0)
                    - actuals[index]
                        .get("value")
                        .and_then(as_f64)
                        .or_else(|| as_f64(&actuals[index]))
                        .unwrap_or(0.0))
                .abs()
            })
            .sum::<f64>()
            / sample_size as f64
    };
    json!({ "sample_size": sample_size, "mae": error })
}

fn trend_analysis(data: &[Value]) -> Value {
    let values: Vec<f64> = data
        .iter()
        .filter_map(|value| {
            value
                .get("value")
                .and_then(as_f64)
                .or_else(|| as_f64(value))
        })
        .collect();
    let trend = values.last().unwrap_or(&0.0) - values.first().unwrap_or(&0.0);
    json!({
        "trend": trend,
        "direction": if trend > 0.0 {
            "up"
        } else if trend < 0.0 {
            "down"
        } else {
            "flat"
        },
        "points": values.len(),
    })
}

fn summary(data: &[Value]) -> Value {
    let values: Vec<f64> = data
        .iter()
        .filter_map(|value| {
            value
                .get("value")
                .and_then(as_f64)
                .or_else(|| as_f64(value))
        })
        .collect();
    json!({ "count": data.len(), "mean": mean(&values), "std_dev": std_dev(&values) })
}

fn recommendations(data: &[Value]) -> Vec<Value> {
    if data.is_empty() {
        vec![json!({ "action": "collect_more_data" })]
    } else {
        vec![json!({ "action": "monitor", "confidence": 0.75 })]
    }
}

fn extract_signals(source: &Value, _options: &Value) -> Vec<Value> {
    source
        .get("signals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| {
            vec![json!({
                "source": source.get("id").cloned().unwrap_or(Value::Null),
                "value": source.get("value").cloned().unwrap_or(json!(0.0)),
                "timestamp": now_json(),
            })]
        })
}

fn magnitude(a: &Value, b: &Value) -> f64 {
    match (as_f64(a), as_f64(b)) {
        (Some(a), Some(b)) => (a - b).abs(),
        _ if a == b => 0.0,
        _ => 1.0,
    }
}

fn adjust(forecast: &Value, factor: f64) -> Value {
    let mut adjusted = forecast.clone();
    if let Some(points) = adjusted.get_mut("forecast").and_then(Value::as_array_mut) {
        for point in points {
            if let Some(value) = point.get_mut("value") {
                if let Some(number) = as_f64(value) {
                    *value = json!(number * factor);
                }
            }
        }
    }
    adjusted
}
