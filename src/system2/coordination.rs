//! System 2 service operations and typed convenience wrappers.
//!
//! The coordination service delegates to pure scheduler and balancer helpers
//! for `coordinate`, `balance`, and conflict-detection operations. Calls are
//! JSON payload based through `ServiceActor`; unknown operation names return an
//! `unknown_operation` JSON response.

use serde_json::{json, Value};

use crate::actor_support::ServiceState;
use crate::error::VsmResult;

use super::{balancer, scheduler};

pub async fn actor_call(op: &str, payload: Value, state: &mut ServiceState) -> VsmResult<Value> {
    match op {
        "coordinate" => {
            let new = payload
                .get("new_schedules")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let existing = payload
                .get("existing_schedules")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            Ok(scheduler::coordinate(&new, &existing))
        }
        "balance" => {
            let requests = payload
                .get("requests")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let current = payload
                .get("current_allocations")
                .cloned()
                .unwrap_or_else(|| json!({}));
            Ok(balancer::balance(&requests, &current))
        }
        "detect_conflicts" => Ok(scheduler::detect_conflicts(
            &payload.as_array().cloned().unwrap_or_default(),
        )),
        "metrics" | "get_state" => {
            Ok(json!({"state": state.data.clone(), "history_len": state.history.len()}))
        }
        _ => Ok(json!({"status":"unknown_operation", "op": op})),
    }
}

pub async fn coordinate_schedules(
    new_schedules: Vec<Value>,
    existing_schedules: Vec<Value>,
) -> VsmResult<Value> {
    crate::actor_support::call_service(
        crate::names::SYSTEM2_COORDINATION,
        "coordinate",
        json!({"new_schedules": new_schedules, "existing_schedules": existing_schedules}),
    )
    .await
}

pub async fn balance_requests(
    requests: Vec<Value>,
    current_allocations: Value,
) -> VsmResult<Value> {
    crate::actor_support::call_service(
        crate::names::SYSTEM2_COORDINATION,
        "balance",
        json!({"requests": requests, "current_allocations": current_allocations}),
    )
    .await
}

pub async fn get_state() -> VsmResult<Value> {
    let value = crate::actor_support::call_service(
        crate::names::SYSTEM2_COORDINATION,
        "get_state",
        json!({}),
    )
    .await?;
    Ok(value.get("state").cloned().unwrap_or_else(|| value.clone()))
}
