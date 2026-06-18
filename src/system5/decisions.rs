//! Decision scoring and history helpers for System 5.
//!
//! Decisions are made from JSON options and criteria by multiplying named
//! option fields by criterion weights and choosing the highest score. The
//! selected decision is stored in the owning actor's in-memory `ServiceState`.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::actor_support::ServiceState;
use crate::error::VsmResult;
use crate::prelude::now_json;

pub fn make_decision(state: &mut ServiceState, request: Value) -> Value {
    let options = request
        .get("options")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let criteria = request
        .get("criteria")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let chosen = choose_option(&options, &criteria);
    let decision = json!({
        "id": format!("decision_{}", Uuid::new_v4()),
        "subject": request.get("subject").cloned().unwrap_or(Value::Null),
        "chosen_option": chosen,
        "criteria": criteria,
        "confidence": confidence(&options),
        "made_at": now_json(),
        "status": "active"
    });
    if !state
        .data
        .get("decisions")
        .and_then(Value::as_array)
        .is_some()
    {
        state.data["decisions"] = json!([]);
    }
    if let Some(arr) = state.data["decisions"].as_array_mut() {
        arr.push(decision.clone());
    }
    decision
}

pub fn decision_history(state: &ServiceState, filters: &Value) -> Value {
    let mut decisions = state
        .data
        .get("decisions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Some(subject) = filters.get("subject").and_then(Value::as_str) {
        decisions.retain(|d| {
            d.get("subject")
                .map(|v| v.to_string().contains(subject))
                .unwrap_or(false)
        });
    }
    let count = decisions.len();
    json!({"decisions": decisions, "count": count})
}

pub fn review_decision(state: &mut ServiceState, decision_id: &str, outcome_data: Value) -> Value {
    let review = json!({"decision_id": decision_id, "outcome": outcome_data, "reviewed_at": now_json(), "lessons": []});
    state.record(json!({"event":"decision_review", "review": review.clone()}));
    review
}

pub async fn actor_call(op: &str, payload: Value, state: &mut ServiceState) -> VsmResult<Value> {
    match op {
        "make_decision" | "decide" => Ok(make_decision(state, payload)),
        "history" | "decision_history" => Ok(decision_history(state, &payload)),
        "review" | "review_decision" => Ok(review_decision(
            state,
            payload
                .get("decision_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            payload.get("outcome_data").cloned().unwrap_or(Value::Null),
        )),
        "patterns" => Ok(
            json!({"history_len": state.data.get("decisions").and_then(Value::as_array).map(Vec::len).unwrap_or(0), "learning_entries": state.history.len()}),
        ),
        _ => Ok(json!({"status":"unknown_operation", "op":op})),
    }
}

fn choose_option(options: &[Value], criteria: &[Value]) -> Value {
    options
        .iter()
        .max_by(|a, b| {
            score(a, criteria)
                .partial_cmp(&score(b, criteria))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
        .unwrap_or(Value::Null)
}
fn score(option: &Value, criteria: &[Value]) -> f64 {
    if criteria.is_empty() {
        return option.get("score").and_then(Value::as_f64).unwrap_or(0.5);
    }
    criteria
        .iter()
        .map(|c| {
            let name = c.get("name").and_then(Value::as_str).unwrap_or("");
            let w = c.get("weight").and_then(Value::as_f64).unwrap_or(1.0);
            option.get(name).and_then(Value::as_f64).unwrap_or(0.0) * w
        })
        .sum()
}
fn confidence(options: &[Value]) -> f64 {
    if options.len() <= 1 {
        0.5
    } else {
        0.75
    }
}
