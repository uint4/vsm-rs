//! Dedicated algedonic signal processor.
//!
//! This actor accepts typed pain and pleasure signals, applies configurable
//! filters, calculates descriptive routes, records alert history, and exposes
//! active-signal and metrics queries. It does not publish calculated routes to
//! the broker or invoke System 3/System 5 actors; callers that need actor
//! delivery should also publish a `VsmMessage`.

pub mod alerting;
pub mod correlation;
pub mod filtering;
pub mod routing;
pub mod signals;

use ractor::{call_t, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use serde_json::{json, Value};

use crate::error::VsmError;
use crate::names;

use alerting::{send_alert, send_critical_alert};
use filtering::{apply_filters, default_filters, validate_filters, Filter};
use routing::{default_rules, route_signal, RouteInfo, RoutingRule};
use signals::{create_signal, AlgedonicSignal, Severity, SignalKind};

#[derive(Debug)]
pub enum AlgedonicMsg {
    Signal(AlgedonicSignal),
    GetActiveSignals(RpcReplyPort<Vec<AlgedonicSignal>>),
    ConfigureFilters(Vec<Filter>, RpcReplyPort<Result<(), VsmError>>),
    GetMetrics(RpcReplyPort<Value>),
    ProcessSignals,
    AnalyzeCorrelations,
    CollectMetrics,
}

#[derive(Debug, Clone)]
pub struct AlgedonicArgs {
    pub filters: Vec<Filter>,
    pub routing_rules: Vec<RoutingRule>,
}

impl Default for AlgedonicArgs {
    fn default() -> Self {
        Self {
            filters: default_filters(),
            routing_rules: default_rules(),
        }
    }
}

#[derive(Debug)]
pub struct AlgedonicState {
    active_signals: Vec<AlgedonicSignal>,
    filters: Vec<Filter>,
    routing_rules: Vec<RoutingRule>,
    routes: Vec<RouteInfo>,
    accepted: u64,
    rejected: u64,
}

pub struct Algedonic;

#[ractor::async_trait]
impl Actor for Algedonic {
    type Msg = AlgedonicMsg;
    type State = AlgedonicState;
    type Arguments = AlgedonicArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("algedonic channel started");
        Ok(AlgedonicState {
            active_signals: Vec::new(),
            filters: args.filters,
            routing_rules: args.routing_rules,
            routes: Vec::new(),
            accepted: 0,
            rejected: 0,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            AlgedonicMsg::Signal(signal) => {
                if apply_filters(&signal, &state.filters) {
                    let route = route_signal(&signal, &state.routing_rules);
                    if signal.priority >= 0.9 {
                        send_critical_alert(signal.clone(), route.clone());
                    } else {
                        send_alert(signal.clone(), route.clone());
                    }
                    state.routes.push(route);
                    state.active_signals.push(signal);
                    state.active_signals.truncate(1000);
                    state.accepted += 1;
                } else {
                    state.rejected += 1;
                }
            }
            AlgedonicMsg::GetActiveSignals(reply) => {
                let _ = reply.send(state.active_signals.clone());
            }
            AlgedonicMsg::ConfigureFilters(filters, reply) => {
                let result = validate_filters(&filters).map_err(VsmError::Validation);
                if result.is_ok() {
                    state.filters = filters;
                }
                let _ = reply.send(result);
            }
            AlgedonicMsg::GetMetrics(reply) => {
                let _ = reply.send(json!({
                    "active_signals": state.active_signals.len(),
                    "accepted": state.accepted,
                    "rejected": state.rejected,
                    "routes": state.routes.len(),
                    "correlations": correlation::analyze_patterns(&state.active_signals, &json!({}))
                }));
            }
            AlgedonicMsg::ProcessSignals
            | AlgedonicMsg::AnalyzeCorrelations
            | AlgedonicMsg::CollectMetrics => {}
        }
        Ok(())
    }
}

pub fn actor_ref() -> Result<ActorRef<AlgedonicMsg>, VsmError> {
    ActorRef::<AlgedonicMsg>::where_is(names::ALGEDONIC.to_string())
        .ok_or_else(|| VsmError::ActorNotFound(names::ALGEDONIC.to_string()))
}

pub fn send_pain_signal(
    source: impl Into<String>,
    data: Value,
    severity: Severity,
) -> Result<(), VsmError> {
    let signal = create_signal(SignalKind::Pain, source, data, severity);
    actor_ref()?
        .send_message(AlgedonicMsg::Signal(signal))
        .map_err(|_| VsmError::Ractor("failed to send pain signal".into()))
}

pub fn send_pleasure_signal(
    source: impl Into<String>,
    data: Value,
    severity: Severity,
) -> Result<(), VsmError> {
    let signal = create_signal(SignalKind::Pleasure, source, data, severity);
    actor_ref()?
        .send_message(AlgedonicMsg::Signal(signal))
        .map_err(|_| VsmError::Ractor("failed to send pleasure signal".into()))
}

pub async fn get_active_signals() -> Result<Vec<AlgedonicSignal>, VsmError> {
    call_t!(actor_ref()?, AlgedonicMsg::GetActiveSignals, 1_000)
        .map_err(|err| VsmError::Ractor(err.to_string()))
}

pub async fn configure_filters(filters: Vec<Filter>) -> Result<(), VsmError> {
    call_t!(actor_ref()?, AlgedonicMsg::ConfigureFilters, 1_000, filters)
        .map_err(|err| VsmError::Ractor(err.to_string()))?
}

pub async fn get_metrics() -> Result<Value, VsmError> {
    call_t!(actor_ref()?, AlgedonicMsg::GetMetrics, 1_000)
        .map_err(|err| VsmError::Ractor(err.to_string()))
}

pub async fn actor_call(
    op: &str,
    payload: Value,
    _state: &mut crate::actor_support::ServiceState,
) -> crate::error::VsmResult<Value> {
    match op {
        "pain" | "send_pain_signal" => {
            let severity = signals::parse_severity(
                payload
                    .get("severity")
                    .and_then(Value::as_str)
                    .unwrap_or("medium"),
            );
            let source = payload
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("external")
                .to_string();
            let data = payload
                .get("data")
                .cloned()
                .unwrap_or_else(|| payload.clone());
            send_pain_signal(source, data, severity)?;
            Ok(json!({"status":"sent"}))
        }
        "pleasure" | "send_pleasure_signal" => {
            let severity = signals::parse_severity(
                payload
                    .get("severity")
                    .and_then(Value::as_str)
                    .unwrap_or("medium"),
            );
            let source = payload
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("external")
                .to_string();
            let data = payload
                .get("data")
                .cloned()
                .unwrap_or_else(|| payload.clone());
            send_pleasure_signal(source, data, severity)?;
            Ok(json!({"status":"sent"}))
        }
        "active" | "get_active_signals" => Ok(serde_json::to_value(get_active_signals().await?)?),
        "metrics" | "get_metrics" => get_metrics().await,
        _ => Ok(json!({"status":"unknown_operation", "op":op})),
    }
}
