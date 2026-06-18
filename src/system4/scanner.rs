//! Environmental scanning helpers and service operations for System 4.
//!
//! The scanner extracts signals from JSON sources, classifies opportunities,
//! threats, and weak signals, detects field-level changes, and summarizes value
//! trends. Classification thresholds are simple numeric heuristics from the
//! usage guide.

use serde_json::{json, Value};

use crate::prelude::now_json;
use crate::util::{as_f64, mean};

pub fn scan_environment(sources: &[Value], options: &Value) -> Value {
    let signals: Vec<Value> = sources.iter().flat_map(|s| extract_signals(s, options)).collect();
    json!({"scanned_at": now_json(), "source_count": sources.len(), "signals": signals.clone(), "classification": classify_signals(&signals)})
}

pub fn detect_changes(current: &Value, previous: &Value) -> Value {
    let mut changes = Vec::new();
    if let (Some(c), Some(p)) = (current.as_object(), previous.as_object()) {
        for (k, cv) in c {
            let pv = p.get(k).unwrap_or(&Value::Null);
            if cv != pv { changes.push(json!({"field": k, "from": pv, "to": cv, "magnitude": magnitude(cv, pv)})); }
        }
    }
    let change_count = changes.len();
    json!({"changes": changes, "change_count": change_count})
}

pub fn classify_signals(signals: &[Value]) -> Value {
    let mut opportunities = Vec::new();
    let mut threats = Vec::new();
    let mut weak = Vec::new();
    for s in signals {
        let val = s.get("value").and_then(as_f64).or_else(|| s.get("score").and_then(as_f64)).unwrap_or(0.0);
        if val > 0.65 { opportunities.push(s.clone()); }
        else if val < -0.35 { threats.push(s.clone()); }
        else { weak.push(s.clone()); }
    }
    json!({"opportunities": opportunities, "threats": threats, "weak_signals": weak})
}

pub fn monitor_trends(history: &[Value]) -> Value {
    let values: Vec<f64> = history.iter().filter_map(|v| v.get("value").and_then(as_f64)).collect();
    let trend = values.last().unwrap_or(&0.0) - values.first().unwrap_or(&0.0);
    json!({"points": values.len(), "mean": mean(&values), "trend": trend, "direction": if trend>0.0 {"up"} else if trend<0.0 {"down"} else {"flat"}})
}

pub async fn actor_call(op: &str, payload: Value, _state: &mut crate::actor_support::ServiceState) -> crate::error::VsmResult<Value> {
    match op {
        "scan" | "scan_environment" => Ok(scan_environment(&payload.get("sources").and_then(Value::as_array).cloned().unwrap_or_default(), &payload)),
        "detect_changes" => Ok(detect_changes(payload.get("current").unwrap_or(&Value::Null), payload.get("previous").unwrap_or(&Value::Null))),
        "classify" => Ok(classify_signals(&payload.as_array().cloned().unwrap_or_default())),
        "trends" => Ok(monitor_trends(&payload.as_array().cloned().unwrap_or_default())),
        _ => Ok(json!({"status":"unknown_operation", "op":op}))
    }
}

fn extract_signals(source:&Value, _options:&Value)->Vec<Value>{ source.get("signals").and_then(Value::as_array).cloned().unwrap_or_else(|| vec![json!({"source": source.get("id").cloned().unwrap_or(Value::Null), "value": source.get("value").cloned().unwrap_or(json!(0.0)), "timestamp": now_json()})]) }
fn magnitude(a:&Value,b:&Value)->f64{ match (as_f64(a), as_f64(b)) { (Some(x),Some(y)) => (x-y).abs(), _ => if a==b {0.0}else{1.0} } }
