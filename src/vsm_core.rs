//! High-level lifecycle, health, and smoke-test facade.
//!
//! This module wraps `app` startup/shutdown and aggregates runtime inspection
//! from channels, telemetry, and Systems 2-5. Health and status calls are
//! best-effort snapshots over in-memory actor state; they are not readiness
//! barriers and may omit subsystem state when a service call fails.

use ractor::ActorRef;
use ractor_supervisor::SupervisorMsg;
use serde_json::{json, Value};

use crate::actor_support::call_service;
use crate::app::{self, VsmApplication};
use crate::channels;
use crate::error::{VsmError, VsmResult};
use crate::names;
use crate::shared::message::{ChannelKind, MessageKind, SystemId, VsmMessage};
use crate::system1::{self, Transaction};

pub async fn start() -> Result<VsmApplication, ractor::SpawnErr> {
    app::start_application().await
}

pub async fn stop() -> VsmResult<()> {
    if let Some(root) = ActorRef::<SupervisorMsg>::where_is(names::ROOT_SUPERVISOR.to_string()) {
        root.stop(Some("shutdown requested".to_string()));
    }
    Ok(())
}

pub async fn health() -> VsmResult<Value> {
    let mut channel_stats = Vec::new();
    for channel in ChannelKind::ALL {
        if let Ok(stats) = channels::stats(channel).await {
            channel_stats.push(serde_json::to_value(stats)?);
        }
    }

    let telemetry = crate::telemetry_reporter::health()
        .await
        .unwrap_or_else(|err| json!({"status":"unavailable", "error": err.to_string()}));

    Ok(json!({
        "status":"running",
        "root_supervisor": ActorRef::<SupervisorMsg>::where_is(names::ROOT_SUPERVISOR.to_string()).is_some(),
        "channels": channel_stats,
        "telemetry": telemetry,
    }))
}

pub async fn subsystem_state() -> VsmResult<Value> {
    let mut state = serde_json::Map::new();
    for (name, op) in [
        (names::SYSTEM2_COORDINATION, "get_state"),
        (names::SYSTEM3_CONTROL, "get_state"),
        (names::SYSTEM4_INTELLIGENCE, "intelligence_report"),
        (names::SYSTEM5_POLICY, "get_organizational_state"),
    ] {
        if let Ok(value) = call_service(name, op, json!({})).await {
            state.insert(name.to_string(), value);
        }
    }
    Ok(Value::Object(state))
}

pub fn send_test_signal(payload: Value) -> VsmResult<()> {
    channels::publish(VsmMessage::new(
        SystemId::External,
        SystemId::System5,
        ChannelKind::Algedonic,
        MessageKind::Alert,
        payload,
    ))
}

pub async fn test_signal() -> VsmResult<Value> {
    let started = std::time::Instant::now();
    send_test_signal(json!({"message":"port smoke-test", "severity":"medium"}))?;

    let tx = Transaction::new(
        "test_signal",
        vec!["test".to_string()],
        json!({"source":"vsm_core::test_signal"}),
    );

    let transaction = match system1::process_transaction(tx).await {
        Ok(value) => serde_json::to_value(value)?,
        Err(err) => json!({"error": err.to_string()}),
    };

    Ok(json!({
        "status": "sent",
        "latency_ms": started.elapsed().as_millis(),
        "transaction": transaction,
    }))
}

pub async fn status() -> VsmResult<Value> {
    Ok(json!({
        "health": health().await?,
        "subsystems": subsystem_state().await?,
    }))
}

pub fn require_running() -> VsmResult<()> {
    if ActorRef::<SupervisorMsg>::where_is(names::ROOT_SUPERVISOR.to_string()).is_some() {
        Ok(())
    } else {
        Err(VsmError::ActorNotFound(names::ROOT_SUPERVISOR.to_string()))
    }
}
