//! Analytics helpers and service operations for System 4.
//!
//! Analytics supports summary, trend, correlation, anomaly, and insight-style
//! analysis over JSON/numeric data. The functions are lightweight heuristics;
//! `analyze_data` calls the Analytics actor, while `analyze_trends` is a direct
//! helper that does not use actor state.

use serde_json::{json, Value};

use crate::prelude::now_json;
use crate::util::{as_f64, mean, std_dev};

pub fn analyze(data: &[Value], analysis_type: &str, options: &Value) -> Value {
    match analysis_type {
        "trend" => trend_analysis(data),
        "correlation" => correlate(data, options),
        "anomaly" => json!({"anomalies": detect_anomalies(data, options)}),
        "insight" => generate_insights(data, options),
        _ => json!({"summary": summary(data), "generated_at": now_json()}),
    }
}

pub fn correlate(data: &[Value], _options: &Value) -> Value {
    let xs: Vec<f64> = data.iter().filter_map(|v| v.get("x").and_then(as_f64)).collect();
    let ys: Vec<f64> = data.iter().filter_map(|v| v.get("y").and_then(as_f64)).collect();
    let n = xs.len().min(ys.len());
    let corr = if n == 0 {0.0} else { let mx=mean(&xs[..n]); let my=mean(&ys[..n]); let num=(0..n).map(|i|(xs[i]-mx)*(ys[i]-my)).sum::<f64>(); let den=((0..n).map(|i|(xs[i]-mx).powi(2)).sum::<f64>()*(0..n).map(|i|(ys[i]-my).powi(2)).sum::<f64>()).sqrt().max(1e-9); num/den };
    json!({"correlation": corr, "sample_size": n})
}

pub fn detect_anomalies(data: &[Value], options: &Value) -> Vec<Value> {
    let threshold = options.get("z_threshold").and_then(Value::as_f64).unwrap_or(2.0);
    let vals: Vec<f64> = data.iter().filter_map(|v| v.get("value").and_then(as_f64).or_else(|| as_f64(v))).collect();
    let m = mean(&vals); let sd = std_dev(&vals).max(1e-9);
    data.iter().cloned().filter(|v| { let x=v.get("value").and_then(as_f64).or_else(|| as_f64(v)).unwrap_or(m); ((x-m)/sd).abs() >= threshold }).collect()
}

pub fn generate_insights(data: &[Value], options: &Value) -> Value {
    let summary = summary(data);
    let anomalies = detect_anomalies(data, options);
    json!({"summary": summary, "anomalies": anomalies, "recommendations": recommendations(data)})
}

pub async fn actor_call(op: &str, payload: Value, _state: &mut crate::actor_support::ServiceState) -> crate::error::VsmResult<Value> {
    let data = payload.get("data").and_then(Value::as_array).cloned().unwrap_or_else(|| payload.as_array().cloned().unwrap_or_default());
    match op {
        "analyze" => Ok(analyze(&data, payload.get("analysis_type").and_then(Value::as_str).unwrap_or("summary"), &payload)),
        "correlate" => Ok(correlate(&data, &payload)),
        "detect_anomalies" => Ok(json!({"anomalies": detect_anomalies(&data, &payload)})),
        "generate_insights" => Ok(generate_insights(&data, &payload)),
        _ => Ok(json!({"status":"unknown_operation", "op":op}))
    }
}

fn trend_analysis(data:&[Value])->Value{ let vals:Vec<f64>=data.iter().filter_map(|v| v.get("value").and_then(as_f64).or_else(|| as_f64(v))).collect(); let trend=vals.last().unwrap_or(&0.0)-vals.first().unwrap_or(&0.0); json!({"trend":trend,"direction":if trend>0.0{"up"}else if trend<0.0{"down"}else{"flat"},"points":vals.len()}) }
fn summary(data:&[Value])->Value{ let vals:Vec<f64>=data.iter().filter_map(|v| v.get("value").and_then(as_f64).or_else(|| as_f64(v))).collect(); json!({"count":data.len(),"mean":mean(&vals),"std_dev":std_dev(&vals)}) }
fn recommendations(data:&[Value])->Vec<Value>{ if data.is_empty(){vec![json!({"action":"collect_more_data"})]} else {vec![json!({"action":"monitor", "confidence":0.75})]} }

pub async fn analyze_trends(data: Value, _window: impl Into<String>) -> crate::error::VsmResult<Value> {
    let values = data.as_array().cloned().unwrap_or_default();
    let nums: Vec<f64> = values
        .iter()
        .filter_map(|v| v.get("value").and_then(as_f64).or_else(|| as_f64(v)))
        .collect();
    let delta = nums.last().copied().unwrap_or(0.0) - nums.first().copied().unwrap_or(0.0);
    Ok(json!({
        "direction": if delta > 0.0 { "increasing" } else if delta < 0.0 { "decreasing" } else { "stable" },
        "delta": delta,
        "points": nums.len(),
    }))
}

pub async fn analyze_data(data: Value, analysis_type: impl Into<String>) -> crate::error::VsmResult<Value> {
    crate::actor_support::call_service(
        crate::names::SYSTEM4_ANALYTICS,
        "analyze",
        json!({"data": data, "analysis_type": analysis_type.into()}),
    ).await
}
