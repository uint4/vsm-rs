use serde_json::{json, Value};

use crate::actor_support::ServiceState;
use crate::error::VsmResult;
use crate::prelude::now_json;

use super::{decisions, identity, values};

pub fn set_policy(state: &mut ServiceState, policy_area: &str, policy_details: Value) -> Value {
    if !state.data.get("policies").and_then(Value::as_object).is_some() {
        state.data["policies"] = json!({});
    }
    state.data["policies"][policy_area] = policy_details.clone();
    state.data["policies_updated_at"] = now_json();
    json!({"policy_area": policy_area, "policy": policy_details, "status":"set"})
}

pub fn evaluate_alignment(state: &ServiceState, proposal: &Value) -> Value {
    let id = state.data.get("identity").cloned().unwrap_or_else(identity::default_identity);
    let vals = state.data.get("values").cloned().unwrap_or_else(values::default_values);
    json!({"identity_alignment": identity::check_alignment(&id, proposal), "values_alignment": values::evaluate_against_values(&vals, proposal)})
}

pub fn handle_crisis(state: &mut ServiceState, crisis_info: Value) -> Value {
    let severity = crisis_info.get("severity").and_then(Value::as_str).unwrap_or("medium").to_string();
    let decision = decisions::make_decision(state, json!({"subject":"crisis_response", "options":[{"name":"stabilize", "score":0.9},{"name":"monitor", "score":0.5}], "criteria":[{"name":"score", "weight":1.0}], "crisis": crisis_info}));
    json!({"severity": severity, "decision": decision, "directives": ["protect viability", "preserve identity", "communicate clearly"]})
}

pub async fn actor_call(op: &str, payload: Value, state: &mut ServiceState) -> VsmResult<Value> {
    match op {
        "set_identity" => Ok(identity::set_identity(state, payload)),
        "define_values" => Ok(values::define_values(state, payload)),
        "make_decision" => Ok(decisions::make_decision(state, payload)),
        "set_policy" => {
            let policy_area = payload.get("policy_area").and_then(Value::as_str).unwrap_or("general").to_string();
            let policy_details = payload.get("policy_details").cloned().unwrap_or_else(|| payload.clone());
            Ok(set_policy(state, &policy_area, policy_details))
        }
        "evaluate_alignment" => Ok(evaluate_alignment(state, &payload)),
        "handle_crisis" => Ok(handle_crisis(state, payload)),
        "get_organizational_state" | "state" => Ok(json!({"state": state.data.clone(), "history_len": state.history.len()})),
        _ => Ok(json!({"status":"unknown_operation", "op":op}))
    }
}

pub async fn make_decision(request: Value) -> VsmResult<Value> {
    let decision = crate::actor_support::call_service(crate::names::SYSTEM5_POLICY, "make_decision", request).await?;
    let mut out = decision.clone();
    if let Value::Object(map) = &mut out {
        map.insert("decision".to_string(), decision);
    }
    Ok(out)
}

pub async fn get_organizational_state() -> VsmResult<Value> {
    let value = crate::actor_support::call_service(crate::names::SYSTEM5_POLICY, "get_organizational_state", json!({})).await?;
    Ok(value.get("state").cloned().unwrap_or_else(|| value.clone()))
}

pub async fn set_policy_area(policy_area: impl Into<String>, policy_details: Value) -> VsmResult<Value> {
    crate::actor_support::call_service(
        crate::names::SYSTEM5_POLICY,
        "set_policy",
        json!({"policy_area": policy_area.into(), "policy_details": policy_details}),
    ).await
}
