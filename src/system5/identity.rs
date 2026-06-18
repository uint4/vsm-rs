//! Identity state and alignment helpers for System 5.
//!
//! Identity is represented as JSON and updated with deep-merge semantics.
//! Alignment and relevant-aspect checks use keyword extraction heuristics rather
//! than semantic models. When called through the standalone Identity actor,
//! changes do not update the Policy actor's identity.

use serde_json::{json, Value};

use crate::actor_support::ServiceState;
use crate::error::VsmResult;
use crate::prelude::now_json;

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

pub fn set_identity(state: &mut ServiceState, identity_config: Value) -> Value {
    let mut identity = state.data.get("identity").cloned().unwrap_or_else(default_identity);
    crate::util::deep_merge(&mut identity, &identity_config);
    identity["updated_at"] = now_json();
    state.data["identity"] = identity.clone();
    state.record(json!({"event":"identity_updated", "identity": identity.clone(), "timestamp": now_json()}));
    identity
}

pub fn check_alignment(identity: &Value, proposal: &Value) -> Value {
    let words = crate::prelude::value_keywords(proposal);
    let identity_words = crate::prelude::value_keywords(identity);
    let matches = words.iter().filter(|w| identity_words.contains(*w)).count();
    let score = if words.is_empty() { 0.5 } else { matches as f64 / words.len() as f64 };
    json!({"score": score, "aligned": score >= 0.5, "matched_terms": matches})
}

pub fn relevant_aspects(identity: &Value, context: &Value) -> Value {
    let keywords = crate::prelude::value_keywords(context);
    let mut aspects = serde_json::Map::new();
    if let Some(obj) = identity.as_object() {
        for (k, v) in obj { if keywords.iter().any(|kw| k.contains(kw) || v.to_string().contains(kw)) { aspects.insert(k.clone(), v.clone()); } }
    }
    Value::Object(aspects)
}

pub async fn actor_call(op: &str, payload: Value, state: &mut ServiceState) -> VsmResult<Value> {
    match op {
        "set_identity" => Ok(set_identity(state, payload)),
        "get_current_identity" | "identity" => Ok(state.data.get("identity").cloned().unwrap_or_else(default_identity)),
        "check_alignment" => Ok(check_alignment(&state.data.get("identity").cloned().unwrap_or_else(default_identity), &payload)),
        "get_relevant_aspects" => Ok(relevant_aspects(&state.data.get("identity").cloned().unwrap_or_else(default_identity), &payload)),
        "update_aspect" => { let aspect=payload.get("aspect").and_then(Value::as_str).unwrap_or("metadata"); let value=payload.get("value").cloned().unwrap_or(Value::Null); let mut id=state.data.get("identity").cloned().unwrap_or_else(default_identity); id[aspect]=value; Ok(set_identity(state, id)) }
        "evolve_identity" => { let mut id=state.data.get("identity").cloned().unwrap_or_else(default_identity); crate::util::deep_merge(&mut id, &payload); id["evolved_at"]=now_json(); Ok(set_identity(state, id)) }
        _ => Ok(json!({"status":"unknown_operation", "op":op}))
    }
}
