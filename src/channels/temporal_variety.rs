use chrono::Utc;
use ractor::{call_t, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use serde_json::{json, Value};

use crate::channels::temporal::{aggregation, causality, forecasting, patterns, timescales, visualization};
use crate::channels::temporal::timescales::{TemporalMetric, Timescales};
use crate::error::VsmError;
use crate::names;

#[derive(Debug)]
pub enum TemporalVarietyMsg {
    RecordVariety(Value),
    GetVariety(String, RpcReplyPort<Value>),
    GetPatterns(RpcReplyPort<Value>),
    GetForecasts(Vec<usize>, RpcReplyPort<Value>),
    GetCausality(RpcReplyPort<Value>),
    GetSummary(RpcReplyPort<Value>),
    GetVisualizationData(Value, RpcReplyPort<Value>),
    AnalyzePatterns,
    UpdateForecasts,
    AnalyzeCausality,
    CleanupBuffer,
}

#[derive(Debug, Clone)]
pub struct TemporalVarietyArgs { pub config: Value }
impl Default for TemporalVarietyArgs { fn default() -> Self { Self { config: json!({}) } } }

#[derive(Debug)]
pub struct TemporalVarietyState {
    pub timescales: Timescales,
    pub buffer: Vec<Value>,
    pub patterns: Value,
    pub forecasts: Value,
    pub causality: Value,
}

pub struct TemporalVariety;

#[ractor::async_trait]
impl Actor for TemporalVariety {
    type Msg = TemporalVarietyMsg;
    type State = TemporalVarietyState;
    type Arguments = TemporalVarietyArgs;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, args: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("temporal variety channel started");
        Ok(TemporalVarietyState { timescales: timescales::initialize(&args.config), buffer: Vec::new(), patterns: json!({}), forecasts: json!({}), causality: json!({}) })
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            TemporalVarietyMsg::RecordVariety(measurement) => {
                state.buffer.insert(0, measurement.clone());
                state.buffer.truncate(10_000);
                state.timescales = timescales::update(state.timescales.clone(), TemporalMetric { timestamp: Utc::now(), data: measurement });
            }
            TemporalVarietyMsg::GetVariety(scale, reply) => { let _ = reply.send(timescales::get_variety(&state.timescales, &scale)); }
            TemporalVarietyMsg::GetPatterns(reply) => { let _ = reply.send(patterns::analyze(&state.timescales)); }
            TemporalVarietyMsg::GetForecasts(horizons, reply) => { let _ = reply.send(forecasting::generate_forecasts(&state.timescales, &horizons)); }
            TemporalVarietyMsg::GetCausality(reply) => { let _ = reply.send(causality::analyze_correlations(&state.timescales)); }
            TemporalVarietyMsg::GetSummary(reply) => { let _ = reply.send(aggregation::generate_summary(&json!({"stats": timescales::get_statistics(&state.timescales), "buffer_len": state.buffer.len()}))); }
            TemporalVarietyMsg::GetVisualizationData(opts, reply) => { let _ = reply.send(visualization::prepare_data(&json!({"timescales": state.timescales.clone(), "buffer": state.buffer.clone()}), &opts)); }
            TemporalVarietyMsg::AnalyzePatterns => { state.patterns = patterns::analyze(&state.timescales); }
            TemporalVarietyMsg::UpdateForecasts => { state.forecasts = forecasting::generate_forecasts(&state.timescales, &[1, 5, 10]); }
            TemporalVarietyMsg::AnalyzeCausality => { state.causality = causality::analyze_correlations(&state.timescales); }
            TemporalVarietyMsg::CleanupBuffer => { state.buffer.truncate(10_000); }
        }
        Ok(())
    }
}

pub fn actor_ref() -> Result<ActorRef<TemporalVarietyMsg>, VsmError> {
    ActorRef::<TemporalVarietyMsg>::where_is(names::TEMPORAL_VARIETY.to_string()).ok_or_else(|| VsmError::ActorNotFound(names::TEMPORAL_VARIETY.to_string()))
}

pub fn record_variety(measurement: Value) -> Result<(), VsmError> {
    actor_ref()?.send_message(TemporalVarietyMsg::RecordVariety(measurement)).map_err(|_| VsmError::Ractor("failed to record temporal variety".into()))
}

pub async fn get_variety(timescale: impl Into<String>) -> Result<Value, VsmError> {
    call_t!(actor_ref()?, TemporalVarietyMsg::GetVariety, 1_000, timescale.into()).map_err(|err| VsmError::Ractor(err.to_string()))
}

pub async fn get_patterns() -> Result<Value, VsmError> { call_t!(actor_ref()?, TemporalVarietyMsg::GetPatterns, 1_000).map_err(|err| VsmError::Ractor(err.to_string())) }
pub async fn get_forecasts(horizons: Vec<usize>) -> Result<Value, VsmError> { call_t!(actor_ref()?, TemporalVarietyMsg::GetForecasts, 1_000, horizons).map_err(|err| VsmError::Ractor(err.to_string())) }
pub async fn get_causality() -> Result<Value, VsmError> { call_t!(actor_ref()?, TemporalVarietyMsg::GetCausality, 1_000).map_err(|err| VsmError::Ractor(err.to_string())) }
pub async fn get_summary() -> Result<Value, VsmError> { call_t!(actor_ref()?, TemporalVarietyMsg::GetSummary, 1_000).map_err(|err| VsmError::Ractor(err.to_string())) }
pub async fn get_visualization_data(opts: Value) -> Result<Value, VsmError> { call_t!(actor_ref()?, TemporalVarietyMsg::GetVisualizationData, 1_000, opts).map_err(|err| VsmError::Ractor(err.to_string())) }


pub async fn actor_call(op: &str, payload: Value, _state: &mut crate::actor_support::ServiceState) -> crate::error::VsmResult<Value> {
    match op {
        "record" | "record_variety" => { record_variety(payload)?; Ok(json!({"status":"recorded"})) }
        "get_variety" => get_variety(payload.get("timescale").and_then(Value::as_str).unwrap_or("operational").to_string()).await,
        "patterns" | "get_patterns" => get_patterns().await,
        "forecasts" | "get_forecasts" => {
            let horizons = payload.get("horizons").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_u64().map(|u| u as usize)).collect()).unwrap_or_else(|| vec![1,5,10]);
            get_forecasts(horizons).await
        }
        "causality" | "get_causality" => get_causality().await,
        "summary" | "get_summary" => get_summary().await,
        "visualization" => get_visualization_data(payload).await,
        _ => Ok(json!({"status":"unknown_operation", "op":op}))
    }
}
