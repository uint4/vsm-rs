//! Supervised telemetry reporter facade.
//!
//! The current telemetry reporter is a `ServiceActor` child that answers a
//! health-style service call and exposes a nominal reporting interval. It does
//! not yet export metrics, schedule periodic reports, or expose restart/queue
//! telemetry; those are documented as future hardening work.

use ractor::concurrency::Duration;
use ractor_supervisor::ChildSpec;
use serde_json::json;

use crate::actor_support::{call_service, service_child, ServiceKind};
use crate::error::VsmResult;
use crate::names;

pub fn child_spec() -> ChildSpec {
    service_child(names::TELEMETRY_REPORTER, ServiceKind::TelemetryReporter, json!({"role":"telemetry_reporter"}))
}

pub async fn health() -> VsmResult<serde_json::Value> {
    call_service(names::TELEMETRY_REPORTER, "health", json!({})).await
}

pub fn interval() -> Duration { Duration::from_secs(60) }
