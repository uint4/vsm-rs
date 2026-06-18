use serde_json::{json, Value};

use crate::actor_support::ServiceState;
use crate::error::VsmResult;

use super::{analytics, forecasting, scanner};

pub async fn actor_call(op: &str, payload: Value, state: &mut ServiceState) -> VsmResult<Value> {
    match op {
        "environmental_scan" | "scan" => scanner::actor_call("scan", payload, state).await,
        "analyze" => analytics::actor_call("analyze", payload, state).await,
        "forecast" => forecasting::actor_call("forecast", payload, state).await,
        "intelligence_report" => {
            let sources = payload.get("sources").and_then(Value::as_array).cloned().unwrap_or_default();
            let scan = scanner::scan_environment(&sources, &payload);
            let signals = scan.get("signals").and_then(Value::as_array).cloned().unwrap_or_default();
            let insights = analytics::generate_insights(&signals, &payload);
            Ok(json!({"scan": scan, "insights": insights, "history_len": state.history.len()}))
        }
        _ => Ok(json!({"status":"unknown_operation", "op":op}))
    }
}

pub async fn get_intelligence_report() -> VsmResult<Value> {
    crate::actor_support::call_service(crate::names::SYSTEM4_INTELLIGENCE, "intelligence_report", json!({"sources": []})).await
}

pub async fn environmental_scan(sources: Vec<Value>, options: Value) -> VsmResult<Value> {
    crate::actor_support::call_service(
        crate::names::SYSTEM4_INTELLIGENCE,
        "environmental_scan",
        json!({"sources": sources, "options": options}),
    ).await
}
