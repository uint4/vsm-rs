//! Values state and alignment helpers for System 5.
//!
//! Values are JSON definitions with priorities and indicator words. Evaluation
//! checks indicator overlap against extracted subject keywords and returns a
//! weighted alignment score. Standalone Values actor calls do not update Policy
//! actor state.

use serde_json::{json, Value};

use crate::actor_support::ServiceState;
use crate::error::VsmResult;
use crate::prelude::now_json;

pub fn default_values() -> Value {
    json!([
        {"name":"viability", "priority":1.0, "indicators":["sustainable", "resilient", "adaptive"]},
        {"name":"autonomy", "priority":0.85, "indicators":["local", "empowered", "self-managing"]},
        {"name":"coherence", "priority":0.8, "indicators":["aligned", "coordinated", "integrated"]},
        {"name":"ethics", "priority":0.9, "indicators":["fair", "transparent", "responsible"]}
    ])
}

pub fn define_values(state: &mut ServiceState, values: Value) -> Value {
    state.data["values"] = values.clone();
    state.data["values_updated_at"] = now_json();
    values
}

pub fn evaluate_against_values(values: &Value, subject: &Value) -> Value {
    let subject_words = crate::prelude::value_keywords(subject);
    let mut scores = Vec::new();
    for value in values.as_array().cloned().unwrap_or_default() {
        let indicators = value
            .get("indicators")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut hits = 0;
        for ind in indicators {
            if let Some(s) = ind.as_str() {
                if subject_words.iter().any(|w| w.contains(s) || s.contains(w)) {
                    hits += 1;
                }
            }
        }
        let denom = value
            .get("indicators")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(1)
            .max(1) as f64;
        scores.push(json!({"value": value.get("name").cloned().unwrap_or(Value::Null), "score": hits as f64 / denom, "priority": value.get("priority").cloned().unwrap_or(json!(1.0))}));
    }
    let weighted = weighted_score(&scores);
    json!({"scores": scores, "overall_score": weighted, "aligned": weighted >= 0.5})
}

pub fn validate_policy(policy_area: &str, policy_details: &Value, values: &Value) -> Value {
    let eval = evaluate_against_values(values, policy_details);
    json!({"policy_area": policy_area, "valid": eval.get("aligned").and_then(Value::as_bool).unwrap_or(false), "evaluation": eval})
}

pub async fn actor_call(op: &str, payload: Value, state: &mut ServiceState) -> VsmResult<Value> {
    let current = state
        .data
        .get("values")
        .cloned()
        .unwrap_or_else(default_values);
    match op {
        "define_values" => Ok(define_values(state, payload)),
        "get_current_values" | "values" => Ok(current),
        "evaluate_against_values" | "check_alignment" => {
            Ok(evaluate_against_values(&current, &payload))
        }
        "validate_policy" => Ok(validate_policy(
            payload
                .get("policy_area")
                .and_then(Value::as_str)
                .unwrap_or("general"),
            payload.get("policy_details").unwrap_or(&payload),
            &current,
        )),
        "add_value" => {
            let mut arr = current.as_array().cloned().unwrap_or_default();
            arr.push(payload);
            Ok(define_values(state, Value::Array(arr)))
        }
        "update_value_priority" => {
            let name = payload.get("name").and_then(Value::as_str).unwrap_or("");
            let pr = payload
                .get("priority")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            let mut arr = current.as_array().cloned().unwrap_or_default();
            for v in &mut arr {
                if v.get("name").and_then(Value::as_str) == Some(name) {
                    v["priority"] = json!(pr);
                }
            }
            Ok(define_values(state, Value::Array(arr)))
        }
        _ => Ok(json!({"status":"unknown_operation", "op":op})),
    }
}

fn weighted_score(scores: &[Value]) -> f64 {
    let mut num = 0.0;
    let mut den = 0.0;
    for s in scores {
        let score = s.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let p = s.get("priority").and_then(Value::as_f64).unwrap_or(1.0);
        num += score * p;
        den += p;
    }
    if den == 0.0 {
        0.0
    } else {
        num / den
    }
}
