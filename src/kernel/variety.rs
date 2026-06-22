//! Private variety, algedonic, and temporal runtime adapter.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use ractor::{call_t, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

use crate::config::RuntimeConfig;
use crate::error::FrameworkError;
use crate::protocol::algedonic::{
    AlgedonicAcknowledgement, AlgedonicAlert, AlgedonicCycle, AlgedonicEscalation,
    AlgedonicLifecycleStatus, AlgedonicSeverity, AlgedonicSignalRecord, AlgedonicSnapshot,
};
use crate::protocol::events::{
    AlgedonicEvent, AlgedonicReport, RuntimeEvent, RuntimeReport, TemporalEvent, TemporalReport,
    VarietyEvent, VarietyReport,
};
use crate::protocol::system5::CrisisResponse;
use crate::protocol::temporal::{
    TemporalAggregate, TemporalAnalysis, TemporalSample, TemporalSnapshot,
};
use crate::protocol::variety::{
    VarietyAlgedonicTemporalSnapshot, VarietyCycle, VarietyIntervention,
    VarietyInterventionOutcome, VarietyObservation, VarietySnapshot,
};
use crate::protocol::{ProtocolMetadata, SubsystemRole, VsmAddress};
use crate::roles::{AlertRecord, AlertSeverity, AlertSink, RoleContext, ViableSystem};
use crate::runtime::{RuntimePorts, VarietyRuntimeRoles};

const ACTOR_CALL_TIMEOUT_MS: u64 = 1_000;
const RETAINED_RECORDS: usize = 1_000;

pub(crate) struct VarietyRuntime<V>
where
    V: ViableSystem,
{
    actor: ActorRef<VarietyActorMsg<V>>,
    shutdown: AtomicBool,
    _system: PhantomData<fn() -> V>,
}

impl<V> VarietyRuntime<V>
where
    V: ViableSystem,
{
    pub(crate) async fn start(
        config: RuntimeConfig,
        roles: VarietyRuntimeRoles<V>,
        ports: RuntimePorts<V>,
    ) -> Result<Arc<Self>, FrameworkError> {
        let context = ports.role_context(
            config.runtime_id.clone(),
            config.recursion_path.clone(),
            SubsystemRole::Variety,
        );

        let (actor, _join) = Actor::spawn(
            Some(variety_actor_name(&config)),
            VarietyActor::<V>::new(),
            VarietyActorArgs {
                config,
                roles,
                context,
                alert_sink: ports.alert_sink(),
            },
        )
        .await
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to spawn typed variety/algedonic/temporal actor: {err}"),
        })?;

        Ok(Arc::new(Self {
            actor,
            shutdown: AtomicBool::new(false),
            _system: PhantomData,
        }))
    }

    pub(crate) async fn record_variety(
        &self,
        observation: VarietyObservation<V>,
    ) -> Result<VarietyCycle<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.actor,
            VarietyActorMsg::RecordVariety,
            ACTOR_CALL_TIMEOUT_MS,
            Box::new(observation)
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to record typed variety observation: {err}"),
        })?
    }

    pub(crate) async fn record_variety_outcomes(
        &self,
        outcomes: Vec<VarietyInterventionOutcome<V>>,
    ) -> Result<VarietyAlgedonicTemporalSnapshot<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.actor,
            VarietyActorMsg::RecordVarietyOutcomes,
            ACTOR_CALL_TIMEOUT_MS,
            outcomes
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to record typed variety outcomes: {err}"),
        })?
    }

    pub(crate) async fn process_algedonic(
        &self,
        signal: AlgedonicSignalRecord<V>,
    ) -> Result<AlgedonicCycle<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.actor,
            VarietyActorMsg::ProcessAlgedonic,
            ACTOR_CALL_TIMEOUT_MS,
            Box::new(signal)
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to process typed algedonic signal: {err}"),
        })?
    }

    pub(crate) async fn record_system5_dispatch(
        &self,
        signal_id: String,
        response: CrisisResponse<V>,
    ) -> Result<AlgedonicCycle<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.actor,
            VarietyActorMsg::RecordSystem5Dispatch,
            ACTOR_CALL_TIMEOUT_MS,
            signal_id,
            Box::new(response)
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to record typed algedonic System 5 dispatch: {err}"),
        })?
    }

    pub(crate) async fn acknowledge_algedonic(
        &self,
        acknowledgements: Vec<AlgedonicAcknowledgement<V>>,
    ) -> Result<VarietyAlgedonicTemporalSnapshot<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.actor,
            VarietyActorMsg::AcknowledgeAlgedonic,
            ACTOR_CALL_TIMEOUT_MS,
            acknowledgements
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to acknowledge typed algedonic signals: {err}"),
        })?
    }

    pub(crate) async fn expire_algedonic(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<AlgedonicEscalation<V>>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.actor,
            VarietyActorMsg::ExpireAlgedonic,
            ACTOR_CALL_TIMEOUT_MS,
            now
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to expire typed algedonic signals: {err}"),
        })?
    }

    pub(crate) async fn record_temporal_sample(
        &self,
        sample: TemporalSample,
    ) -> Result<TemporalSnapshot, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.actor,
            VarietyActorMsg::RecordTemporalSample,
            ACTOR_CALL_TIMEOUT_MS,
            Box::new(sample)
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to record typed temporal sample: {err}"),
        })?
    }

    pub(crate) async fn analyze_temporal(&self) -> Result<TemporalAnalysis, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.actor,
            VarietyActorMsg::AnalyzeTemporal,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to analyze typed temporal samples: {err}"),
        })?
    }

    pub(crate) async fn snapshot(
        &self,
    ) -> Result<VarietyAlgedonicTemporalSnapshot<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(self.actor, VarietyActorMsg::Snapshot, ACTOR_CALL_TIMEOUT_MS).map_err(|err| {
            FrameworkError::Runtime {
                reason: format!("failed to read typed variety/algedonic/temporal snapshot: {err}"),
            }
        })?
    }

    pub(crate) async fn shutdown(&self) -> Result<(), FrameworkError> {
        if self.shutdown.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        call_t!(self.actor, VarietyActorMsg::Shutdown, ACTOR_CALL_TIMEOUT_MS).map_err(
            |err| FrameworkError::Runtime {
                reason: format!(
                    "failed to shut down typed variety/algedonic/temporal runtime: {err}"
                ),
            },
        )??;
        self.actor.stop(Some(
            "typed variety/algedonic/temporal runtime shutdown".to_string(),
        ));
        Ok(())
    }

    fn ensure_running(&self) -> Result<(), FrameworkError> {
        if self.shutdown.load(Ordering::SeqCst) {
            Err(FrameworkError::Shutdown)
        } else {
            Ok(())
        }
    }
}

struct VarietyActor<V>
where
    V: ViableSystem,
{
    _system: PhantomData<V>,
}

impl<V> VarietyActor<V>
where
    V: ViableSystem,
{
    fn new() -> Self {
        Self {
            _system: PhantomData,
        }
    }
}

struct VarietyActorArgs<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    roles: VarietyRuntimeRoles<V>,
    context: RoleContext<V>,
    alert_sink: Arc<dyn AlertSink>,
}

struct VarietyActorState<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    roles: VarietyRuntimeRoles<V>,
    context: RoleContext<V>,
    alert_sink: Arc<dyn AlertSink>,
    observations: Vec<VarietyObservation<V>>,
    interventions: Vec<VarietyIntervention<V>>,
    outcomes: Vec<VarietyInterventionOutcome<V>>,
    signals: Vec<AlgedonicSignalRecord<V>>,
    acknowledgements: Vec<AlgedonicAcknowledgement<V>>,
    escalations: Vec<AlgedonicEscalation<V>>,
    alerts: Vec<AlgedonicAlert>,
    crisis_responses: HashMap<String, CrisisResponse<V>>,
    temporal_samples: Vec<TemporalSample>,
    temporal_aggregates: Vec<TemporalAggregate>,
    temporal_analyses: Vec<TemporalAnalysis>,
}

enum VarietyActorMsg<V>
where
    V: ViableSystem,
{
    RecordVariety(
        Box<VarietyObservation<V>>,
        RpcReplyPort<Result<VarietyCycle<V>, FrameworkError>>,
    ),
    RecordVarietyOutcomes(
        Vec<VarietyInterventionOutcome<V>>,
        RpcReplyPort<Result<VarietyAlgedonicTemporalSnapshot<V>, FrameworkError>>,
    ),
    ProcessAlgedonic(
        Box<AlgedonicSignalRecord<V>>,
        RpcReplyPort<Result<AlgedonicCycle<V>, FrameworkError>>,
    ),
    RecordSystem5Dispatch(
        String,
        Box<CrisisResponse<V>>,
        RpcReplyPort<Result<AlgedonicCycle<V>, FrameworkError>>,
    ),
    AcknowledgeAlgedonic(
        Vec<AlgedonicAcknowledgement<V>>,
        RpcReplyPort<Result<VarietyAlgedonicTemporalSnapshot<V>, FrameworkError>>,
    ),
    ExpireAlgedonic(
        DateTime<Utc>,
        RpcReplyPort<Result<Vec<AlgedonicEscalation<V>>, FrameworkError>>,
    ),
    RecordTemporalSample(
        Box<TemporalSample>,
        RpcReplyPort<Result<TemporalSnapshot, FrameworkError>>,
    ),
    AnalyzeTemporal(RpcReplyPort<Result<TemporalAnalysis, FrameworkError>>),
    Snapshot(RpcReplyPort<Result<VarietyAlgedonicTemporalSnapshot<V>, FrameworkError>>),
    Shutdown(RpcReplyPort<Result<(), FrameworkError>>),
}

#[ractor::async_trait]
impl<V> Actor for VarietyActor<V>
where
    V: ViableSystem,
{
    type Msg = VarietyActorMsg<V>;
    type State = VarietyActorState<V>;
    type Arguments = VarietyActorArgs<V>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(VarietyActorState {
            config: args.config,
            roles: args.roles,
            context: args.context,
            alert_sink: args.alert_sink,
            observations: Vec::new(),
            interventions: Vec::new(),
            outcomes: Vec::new(),
            signals: Vec::new(),
            acknowledgements: Vec::new(),
            escalations: Vec::new(),
            alerts: Vec::new(),
            crisis_responses: HashMap::new(),
            temporal_samples: Vec::new(),
            temporal_aggregates: Vec::new(),
            temporal_analyses: Vec::new(),
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            VarietyActorMsg::RecordVariety(observation, reply) => {
                let result = record_variety(state, *observation).await;
                let _ = reply.send(result);
            }
            VarietyActorMsg::RecordVarietyOutcomes(outcomes, reply) => {
                let result = record_variety_outcomes(state, outcomes).await;
                let _ = reply.send(result);
            }
            VarietyActorMsg::ProcessAlgedonic(signal, reply) => {
                let result = process_algedonic(state, *signal).await;
                let _ = reply.send(result);
            }
            VarietyActorMsg::RecordSystem5Dispatch(signal_id, response, reply) => {
                let result = record_system5_dispatch(state, signal_id, *response).await;
                let _ = reply.send(result);
            }
            VarietyActorMsg::AcknowledgeAlgedonic(acknowledgements, reply) => {
                let result = acknowledge_algedonic(state, acknowledgements).await;
                let _ = reply.send(result);
            }
            VarietyActorMsg::ExpireAlgedonic(now, reply) => {
                let result = expire_algedonic(state, now).await;
                let _ = reply.send(result);
            }
            VarietyActorMsg::RecordTemporalSample(sample, reply) => {
                let result = record_temporal_sample(state, *sample).await;
                let _ = reply.send(result);
            }
            VarietyActorMsg::AnalyzeTemporal(reply) => {
                let result = analyze_temporal(state).await;
                let _ = reply.send(result);
            }
            VarietyActorMsg::Snapshot(reply) => {
                let _ = reply.send(Ok(snapshot(state)));
            }
            VarietyActorMsg::Shutdown(reply) => {
                let _ = reply.send(Ok(()));
            }
        }
        Ok(())
    }
}

async fn record_variety<V>(
    state: &mut VarietyActorState<V>,
    mut observation: VarietyObservation<V>,
) -> Result<VarietyCycle<V>, FrameworkError>
where
    V: ViableSystem,
{
    set_metadata(&mut observation.metadata, &state.context);
    set_metadata(&mut observation.estimate.metadata, &state.context);

    let mut interventions = state
        .roles
        .variety_engineering_policy()
        .plan_interventions(&state.context, &observation)
        .await?;
    for intervention in &mut interventions {
        set_metadata(&mut intervention.metadata, &state.context);
    }

    state.observations.push(observation.clone());
    state.interventions.extend(interventions.iter().cloned());
    retain(&mut state.observations);
    retain(&mut state.interventions);

    record_report(
        state,
        RuntimeReport::Variety(Box::new(VarietyReport::Observation(Box::new(
            observation.clone(),
        )))),
    )
    .await;
    for intervention in &interventions {
        record_report(
            state,
            RuntimeReport::Variety(Box::new(VarietyReport::Intervention(Box::new(
                intervention.clone(),
            )))),
        )
        .await;
    }
    emit_event(
        state,
        RuntimeEvent::Variety(Box::new(VarietyEvent::ObservationRecorded(Box::new(
            observation.clone(),
        )))),
    )
    .await;
    emit_event(
        state,
        RuntimeEvent::Variety(Box::new(VarietyEvent::InterventionsProposed {
            intervention_count: interventions.len(),
        })),
    )
    .await;

    Ok(VarietyCycle {
        metadata: observation.metadata.child(),
        observation,
        interventions,
        outcomes: Vec::new(),
        evaluated_at: Utc::now(),
    })
}

async fn record_variety_outcomes<V>(
    state: &mut VarietyActorState<V>,
    outcomes: Vec<VarietyInterventionOutcome<V>>,
) -> Result<VarietyAlgedonicTemporalSnapshot<V>, FrameworkError>
where
    V: ViableSystem,
{
    for mut outcome in outcomes {
        set_metadata(&mut outcome.metadata, &state.context);
        record_report(
            state,
            RuntimeReport::Variety(Box::new(VarietyReport::Outcome(Box::new(outcome.clone())))),
        )
        .await;
        state.outcomes.push(outcome);
    }
    retain(&mut state.outcomes);
    emit_event(
        state,
        RuntimeEvent::Variety(Box::new(VarietyEvent::OutcomesRecorded {
            outcome_count: state.outcomes.len(),
        })),
    )
    .await;
    Ok(snapshot(state))
}

async fn process_algedonic<V>(
    state: &mut VarietyActorState<V>,
    mut signal: AlgedonicSignalRecord<V>,
) -> Result<AlgedonicCycle<V>, FrameworkError>
where
    V: ViableSystem,
{
    set_metadata(&mut signal.metadata, &state.context);
    if signal.source.is_none() {
        signal.source = Some(address_for(state, SubsystemRole::Algedonic));
    }

    let mut signal = state
        .roles
        .algedonic_lifecycle_policy()
        .classify_signal(&state.context, signal)
        .await?;
    set_metadata(&mut signal.metadata, &state.context);
    if signal.status == AlgedonicLifecycleStatus::Proposed {
        signal.status = AlgedonicLifecycleStatus::Classified;
    }

    maybe_raise_alert(state, &signal).await;
    state.signals.push(signal.clone());
    retain(&mut state.signals);

    record_report(
        state,
        RuntimeReport::Algedonic(Box::new(AlgedonicReport::Signal(Box::new(signal.clone())))),
    )
    .await;
    emit_event(
        state,
        RuntimeEvent::Algedonic(Box::new(AlgedonicEvent::SignalRecorded(Box::new(
            signal.clone(),
        )))),
    )
    .await;

    Ok(cycle_for_signal(state, signal))
}

async fn record_system5_dispatch<V>(
    state: &mut VarietyActorState<V>,
    signal_id: String,
    response: CrisisResponse<V>,
) -> Result<AlgedonicCycle<V>, FrameworkError>
where
    V: ViableSystem,
{
    let Some(index) = state
        .signals
        .iter()
        .position(|signal| signal.signal_id == signal_id)
    else {
        return Err(FrameworkError::Unavailable {
            target: format!("algedonic signal {signal_id}"),
        });
    };

    state.signals[index].status = AlgedonicLifecycleStatus::Dispatched;
    state
        .crisis_responses
        .insert(signal_id.clone(), response.clone());
    record_report(
        state,
        RuntimeReport::Algedonic(Box::new(AlgedonicReport::CrisisResponse(Box::new(
            response,
        )))),
    )
    .await;
    emit_event(
        state,
        RuntimeEvent::Algedonic(Box::new(AlgedonicEvent::SignalDispatched {
            signal_id: signal_id.clone(),
            destination: SubsystemRole::System5,
        })),
    )
    .await;

    Ok(cycle_for_signal(state, state.signals[index].clone()))
}

async fn acknowledge_algedonic<V>(
    state: &mut VarietyActorState<V>,
    acknowledgements: Vec<AlgedonicAcknowledgement<V>>,
) -> Result<VarietyAlgedonicTemporalSnapshot<V>, FrameworkError>
where
    V: ViableSystem,
{
    for mut acknowledgement in acknowledgements {
        set_metadata(&mut acknowledgement.metadata, &state.context);
        if let Some(signal) = state
            .signals
            .iter_mut()
            .find(|signal| signal.signal_id == acknowledgement.signal_id)
        {
            signal.status = AlgedonicLifecycleStatus::Acknowledged;
        }
        record_report(
            state,
            RuntimeReport::Algedonic(Box::new(AlgedonicReport::Acknowledgement(Box::new(
                acknowledgement.clone(),
            )))),
        )
        .await;
        emit_event(
            state,
            RuntimeEvent::Algedonic(Box::new(AlgedonicEvent::SignalAcknowledged(Box::new(
                acknowledgement.clone(),
            )))),
        )
        .await;
        state.acknowledgements.push(acknowledgement);
    }
    retain(&mut state.acknowledgements);
    Ok(snapshot(state))
}

async fn expire_algedonic<V>(
    state: &mut VarietyActorState<V>,
    now: DateTime<Utc>,
) -> Result<Vec<AlgedonicEscalation<V>>, FrameworkError>
where
    V: ViableSystem,
{
    let mut escalations = Vec::new();
    let target = address_for(state, SubsystemRole::System5);

    for signal in &mut state.signals {
        let Some(deadline) = signal.acknowledgement_deadline else {
            continue;
        };
        if deadline > now || is_final_algedonic_status(signal.status) {
            continue;
        }

        signal.status = AlgedonicLifecycleStatus::Expired;
        escalations.push(AlgedonicEscalation {
            metadata: signal.metadata.child(),
            signal_id: signal.signal_id.clone(),
            target: target.clone(),
            unit_id: signal.unit_id.clone(),
            reason: "algedonic acknowledgement deadline expired".to_string(),
            escalated_at: now,
        });
    }

    let role_escalations = state
        .roles
        .algedonic_lifecycle_policy()
        .escalate_expired(&state.context, &algedonic_snapshot(state))
        .await?;
    escalations.extend(role_escalations);

    for mut escalation in escalations.clone() {
        set_metadata(&mut escalation.metadata, &state.context);
        record_report(
            state,
            RuntimeReport::Algedonic(Box::new(AlgedonicReport::Escalation(Box::new(
                escalation.clone(),
            )))),
        )
        .await;
        emit_event(
            state,
            RuntimeEvent::Algedonic(Box::new(AlgedonicEvent::SignalEscalated(Box::new(
                escalation.clone(),
            )))),
        )
        .await;
        state.escalations.push(escalation);
    }
    retain(&mut state.escalations);

    Ok(escalations)
}

async fn record_temporal_sample<V>(
    state: &mut VarietyActorState<V>,
    mut sample: TemporalSample,
) -> Result<TemporalSnapshot, FrameworkError>
where
    V: ViableSystem,
{
    set_metadata(&mut sample.metadata, &state.context);
    state.temporal_samples.push(sample.clone());
    retain(&mut state.temporal_samples);
    state.temporal_aggregates = aggregate_samples(&state.temporal_samples);

    record_report(
        state,
        RuntimeReport::Temporal(Box::new(TemporalReport::Sample(Box::new(sample.clone())))),
    )
    .await;
    emit_event(
        state,
        RuntimeEvent::Temporal(Box::new(TemporalEvent::SampleRecorded {
            scale: sample.scale.clone(),
        })),
    )
    .await;

    Ok(temporal_snapshot(state))
}

async fn analyze_temporal<V>(
    state: &mut VarietyActorState<V>,
) -> Result<TemporalAnalysis, FrameworkError>
where
    V: ViableSystem,
{
    state.temporal_aggregates = aggregate_samples(&state.temporal_samples);
    let mut analysis = state
        .roles
        .temporal_analysis_policy()
        .analyze_temporal(
            &state.context,
            &state.temporal_samples,
            &state.temporal_aggregates,
        )
        .await?;
    set_metadata(&mut analysis.metadata, &state.context);

    for aggregate in &analysis.aggregates {
        record_report(
            state,
            RuntimeReport::Temporal(Box::new(TemporalReport::Aggregate(Box::new(
                aggregate.clone(),
            )))),
        )
        .await;
    }
    record_report(
        state,
        RuntimeReport::Temporal(Box::new(TemporalReport::Analysis(Box::new(
            analysis.clone(),
        )))),
    )
    .await;
    emit_event(
        state,
        RuntimeEvent::Temporal(Box::new(TemporalEvent::AnalysisCompleted {
            aggregate_count: analysis.aggregates.len(),
            pattern_count: analysis.patterns.len(),
            forecast_count: analysis.forecasts.len(),
            causal_hypothesis_count: analysis.causal_hypotheses.len(),
        })),
    )
    .await;

    state.temporal_analyses.push(analysis.clone());
    retain(&mut state.temporal_analyses);
    Ok(analysis)
}

async fn maybe_raise_alert<V>(state: &mut VarietyActorState<V>, signal: &AlgedonicSignalRecord<V>)
where
    V: ViableSystem,
{
    if !signal.requires_system5_dispatch() {
        return;
    }

    let alert = AlgedonicAlert {
        metadata: signal.metadata.child(),
        signal_id: signal.signal_id.clone(),
        severity: signal.severity,
        message: signal.reason.clone(),
        details: signal.details.clone(),
        raised_at: Utc::now(),
    };
    state.alerts.push(alert.clone());
    retain(&mut state.alerts);

    let _ = state
        .alert_sink
        .publish_alert(AlertRecord {
            metadata: alert.metadata.clone(),
            severity: alert_severity(alert.severity),
            message: alert.message.clone(),
            details: alert.details.clone(),
            raised_at: alert.raised_at,
        })
        .await;

    record_report(
        state,
        RuntimeReport::Algedonic(Box::new(AlgedonicReport::Alert(Box::new(alert.clone())))),
    )
    .await;
    emit_event(
        state,
        RuntimeEvent::Algedonic(Box::new(AlgedonicEvent::AlertRaised(Box::new(alert)))),
    )
    .await;
}

fn cycle_for_signal<V>(
    state: &VarietyActorState<V>,
    signal: AlgedonicSignalRecord<V>,
) -> AlgedonicCycle<V>
where
    V: ViableSystem,
{
    AlgedonicCycle {
        metadata: signal.metadata.child(),
        acknowledgements: state
            .acknowledgements
            .iter()
            .filter(|acknowledgement| acknowledgement.signal_id == signal.signal_id)
            .cloned()
            .collect(),
        escalations: state
            .escalations
            .iter()
            .filter(|escalation| escalation.signal_id == signal.signal_id)
            .cloned()
            .collect(),
        crisis_response: state.crisis_responses.get(&signal.signal_id).cloned(),
        signal,
        recorded_at: Utc::now(),
    }
}

fn aggregate_samples(samples: &[TemporalSample]) -> Vec<TemporalAggregate> {
    let mut scales = samples
        .iter()
        .map(|sample| sample.scale.clone())
        .collect::<Vec<_>>();
    scales.sort();
    scales.dedup();
    scales
        .into_iter()
        .map(|scale| {
            let scale_samples = samples
                .iter()
                .filter(|sample| sample.scale == scale)
                .cloned()
                .collect::<Vec<_>>();
            TemporalAggregate::from_samples(scale, &scale_samples)
        })
        .collect()
}

fn snapshot<V>(state: &VarietyActorState<V>) -> VarietyAlgedonicTemporalSnapshot<V>
where
    V: ViableSystem,
{
    VarietyAlgedonicTemporalSnapshot {
        variety: variety_snapshot(state),
        algedonic: algedonic_snapshot(state),
        temporal: temporal_snapshot(state),
    }
}

fn variety_snapshot<V>(state: &VarietyActorState<V>) -> VarietySnapshot<V>
where
    V: ViableSystem,
{
    VarietySnapshot {
        observations: state.observations.clone(),
        interventions: state.interventions.clone(),
        outcomes: state.outcomes.clone(),
        last_observed_at: state
            .observations
            .last()
            .map(|observation| observation.observed_at),
    }
}

fn algedonic_snapshot<V>(state: &VarietyActorState<V>) -> AlgedonicSnapshot<V>
where
    V: ViableSystem,
{
    AlgedonicSnapshot {
        signals: state.signals.clone(),
        acknowledgements: state.acknowledgements.clone(),
        escalations: state.escalations.clone(),
        alerts: state.alerts.clone(),
        last_signal_at: state.signals.last().map(|signal| signal.proposed_at),
    }
}

fn temporal_snapshot<V>(state: &VarietyActorState<V>) -> TemporalSnapshot
where
    V: ViableSystem,
{
    TemporalSnapshot {
        samples: state.temporal_samples.clone(),
        aggregates: state.temporal_aggregates.clone(),
        analyses: state.temporal_analyses.clone(),
        last_sample_at: state
            .temporal_samples
            .last()
            .map(|sample| sample.observed_at),
    }
}

async fn emit_event<V>(state: &VarietyActorState<V>, event: RuntimeEvent<V>)
where
    V: ViableSystem,
{
    let _ = state.context.emit_event(event).await;
}

async fn record_report<V>(state: &VarietyActorState<V>, report: RuntimeReport<V>)
where
    V: ViableSystem,
{
    let _ = state.context.record_report(report).await;
}

fn set_metadata<V>(metadata: &mut ProtocolMetadata, context: &RoleContext<V>)
where
    V: ViableSystem,
{
    metadata.source = Some(address_for_context(context, SubsystemRole::Variety));
}

fn address_for<V>(state: &VarietyActorState<V>, role: SubsystemRole) -> VsmAddress
where
    V: ViableSystem,
{
    VsmAddress::new(
        state.config.runtime_id.clone(),
        state.config.recursion_path.clone(),
        role,
    )
}

fn address_for_context<V>(context: &RoleContext<V>, role: SubsystemRole) -> VsmAddress
where
    V: ViableSystem,
{
    VsmAddress::new(
        context.runtime_id().clone(),
        context.recursion_path().clone(),
        role,
    )
}

fn alert_severity(severity: AlgedonicSeverity) -> AlertSeverity {
    match severity {
        AlgedonicSeverity::Low => AlertSeverity::Low,
        AlgedonicSeverity::Medium => AlertSeverity::Medium,
        AlgedonicSeverity::High => AlertSeverity::High,
        AlgedonicSeverity::Critical => AlertSeverity::Critical,
    }
}

fn is_final_algedonic_status(status: AlgedonicLifecycleStatus) -> bool {
    matches!(
        status,
        AlgedonicLifecycleStatus::Acknowledged
            | AlgedonicLifecycleStatus::ActedUpon
            | AlgedonicLifecycleStatus::Resolved
            | AlgedonicLifecycleStatus::Escalated
            | AlgedonicLifecycleStatus::Expired
    )
}

fn retain<T>(items: &mut Vec<T>) {
    if items.len() > RETAINED_RECORDS {
        let excess = items.len() - RETAINED_RECORDS;
        items.drain(0..excess);
    }
}

fn variety_actor_name(config: &RuntimeConfig) -> String {
    let path = if config.recursion_path.is_root() {
        "root".to_string()
    } else {
        config.recursion_path.segments().join("/")
    };
    format!(
        "{}:{path}:variety-algedonic-temporal:lifecycle",
        config.runtime_id
    )
}
