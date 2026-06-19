//! Opt-in prototype helpers for System 5 JSON-shaped experiments.
//!
//! These helpers are not used by the typed System 5 runtime and do not define
//! crate-level policy semantics.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::prelude::now_json;

/// Prototype identity document used only by callers that explicitly opt in.
pub fn default_identity() -> Value {
    json!({
        "purpose": "maintain viability",
        "mission": "coordinate adaptive operations through the VSM",
        "vision": "resilient, ethical, learning organization",
        "core_values": ["viability", "autonomy", "coherence", "adaptability"],
        "strategic_focus": ["stability", "adaptation", "learning"],
        "updated_at": now_json()
    })
}

/// Prototype values document used only by callers that explicitly opt in.
pub fn default_values() -> Value {
    json!([
        {"name":"viability", "priority":1.0, "indicators":["sustainable", "resilient", "adaptive"]},
        {"name":"autonomy", "priority":0.85, "indicators":["local", "empowered", "self-managing"]},
        {"name":"coherence", "priority":0.8, "indicators":["aligned", "coordinated", "integrated"]},
        {"name":"ethics", "priority":0.9, "indicators":["fair", "transparent", "responsible"]}
    ])
}

/// Prototype keyword overlap check between an identity document and proposal.
pub fn check_identity_alignment(identity: &Value, proposal: &Value) -> Value {
    let words = crate::prelude::value_keywords(proposal);
    let identity_words = crate::prelude::value_keywords(identity);
    let matches = words
        .iter()
        .filter(|word| identity_words.contains(*word))
        .count();
    let score = if words.is_empty() {
        0.5
    } else {
        matches as f64 / words.len() as f64
    };
    json!({"score": score, "aligned": score >= 0.5, "matched_terms": matches})
}

/// Prototype indicator-overlap values evaluation.
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
        for indicator in &indicators {
            if let Some(indicator) = indicator.as_str() {
                if subject_words
                    .iter()
                    .any(|word| word.contains(indicator) || indicator.contains(word))
                {
                    hits += 1;
                }
            }
        }
        let denom = indicators.len().max(1) as f64;
        scores.push(json!({
            "value": value.get("name").cloned().unwrap_or(Value::Null),
            "score": hits as f64 / denom,
            "priority": value.get("priority").cloned().unwrap_or(json!(1.0))
        }));
    }
    let weighted = weighted_score(&scores);
    json!({"scores": scores, "overall_score": weighted, "aligned": weighted >= 0.5})
}

/// Prototype weighted decision helper over JSON options and criteria.
pub fn make_weighted_decision(request: &Value) -> Value {
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
    json!({
        "id": format!("decision_{}", Uuid::new_v4()),
        "subject": request.get("subject").cloned().unwrap_or(Value::Null),
        "chosen_option": chosen,
        "criteria": criteria,
        "confidence": if options.len() <= 1 { 0.5 } else { 0.75 },
        "made_at": now_json(),
        "status": "active"
    })
}

/// Prototype crisis directives used only by explicit callers.
pub fn generic_crisis_directives() -> Vec<String> {
    vec![
        "protect viability".to_string(),
        "preserve identity".to_string(),
        "communicate clearly".to_string(),
    ]
}

fn weighted_score(scores: &[Value]) -> f64 {
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for score in scores {
        let raw_score = score.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let priority = score.get("priority").and_then(Value::as_f64).unwrap_or(1.0);
        numerator += raw_score * priority;
        denominator += priority;
    }
    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
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
        .map(|criterion| {
            let name = criterion.get("name").and_then(Value::as_str).unwrap_or("");
            let weight = criterion
                .get("weight")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            option.get(name).and_then(Value::as_f64).unwrap_or(0.0) * weight
        })
        .sum()
}
