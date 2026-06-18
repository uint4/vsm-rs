//! System 3 control service operations and typed wrappers.
//!
//! The control service accepts JSON operations for resource allocation, audit,
//! and state inspection. It delegates calculations to pure modules and stores
//! service data/history in memory through `ServiceActor`.

use serde_json::{json, Value};

use crate::actor_support::ServiceState;
use crate::error::VsmResult;

use super::{audit, resources};

pub async fn actor_call(op: &str, payload: Value, state: &mut ServiceState) -> VsmResult<Value> {
    match op {
        "allocate_resources" | "allocate" => {
            let requests = payload
                .get("requests")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let available = payload
                .get("available")
                .cloned()
                .unwrap_or_else(|| json!({"capacity":100.0}));
            let performance = payload
                .get("performance_data")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let policies = payload
                .get("policies")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            Ok(resources::allocate(
                &requests,
                &available,
                &performance,
                &policies,
            ))
        }
        "audit" => {
            let units: Vec<String> = payload
                .get("unit_ids")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect();
            let audit_type = payload
                .get("audit_type")
                .and_then(Value::as_str)
                .unwrap_or("focused");
            let system_state = payload
                .get("system_state")
                .cloned()
                .unwrap_or_else(|| json!({}));
            Ok(audit::perform_audit(&units, audit_type, &system_state))
        }
        "state" | "get_state" => {
            Ok(json!({"state": state.data.clone(), "history_len": state.history.len()}))
        }
        _ => Ok(json!({"status":"unknown_operation", "op": op})),
    }
}

pub async fn allocate_resources(
    requests: Vec<Value>,
    available: Value,
    performance_data: Value,
    policies: Vec<Value>,
) -> VsmResult<Value> {
    crate::actor_support::call_service(
        crate::names::SYSTEM3_CONTROL,
        "allocate_resources",
        json!({"requests": requests, "available": available, "performance_data": performance_data, "policies": policies}),
    ).await
}

pub async fn perform_audit(
    unit_ids: Vec<String>,
    audit_type: impl Into<String>,
    system_state: Value,
) -> VsmResult<Value> {
    crate::actor_support::call_service(
        crate::names::SYSTEM3_CONTROL,
        "audit",
        json!({"unit_ids": unit_ids, "audit_type": audit_type.into(), "system_state": system_state}),
    ).await
}

pub async fn get_state() -> VsmResult<Value> {
    let value =
        crate::actor_support::call_service(crate::names::SYSTEM3_CONTROL, "get_state", json!({}))
            .await?;
    Ok(value.get("state").cloned().unwrap_or_else(|| value.clone()))
}
