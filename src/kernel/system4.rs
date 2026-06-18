//! Private typed System 4 runtime adapters.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use ractor::{call_t, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

use crate::config::RuntimeConfig;
use crate::error::FrameworkError;
use crate::protocol::events::{RuntimeEvent, RuntimeReport, System4Event, System4Report};
use crate::protocol::system4::{
    AdaptationProposal, EnvironmentSourceDescriptor, EnvironmentSourceStatus,
    EnvironmentalObservation, ForecastCalibration, FreshnessStatus, IntelligenceAssessment,
    InterpretedSignal, OperationalFeasibilityInfo, System4IntelligenceCycle, System4Snapshot,
};
use crate::protocol::{ProtocolMetadata, SubsystemRole, VsmAddress};
use crate::roles::{BoxEnvironmentalSource, RoleContext, ViableSystem};
use crate::runtime::{RuntimePorts, System4RuntimeRoles};

const ACTOR_CALL_TIMEOUT_MS: u64 = 1_000;

pub(crate) struct System4Runtime<V>
where
    V: ViableSystem,
{
    intelligence_actor: ActorRef<IntelligenceActorMsg>,
    shutdown: AtomicBool,
    _system: PhantomData<fn() -> V>,
}

impl<V> System4Runtime<V>
where
    V: ViableSystem,
{
    pub(crate) async fn start(
        config: RuntimeConfig,
        roles: System4RuntimeRoles<V>,
        ports: RuntimePorts<V>,
    ) -> Result<Arc<Self>, FrameworkError> {
        let context = ports.role_context(
            config.runtime_id.clone(),
            config.recursion_path.clone(),
            SubsystemRole::System4,
        );

        let (intelligence_actor, _join) = Actor::spawn(
            Some(system4_actor_name(&config, "intelligence")),
            IntelligenceActor::<V>::new(),
            IntelligenceActorArgs {
                config,
                roles,
                context,
            },
        )
        .await
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to spawn typed System 4 intelligence actor: {err}"),
        })?;

        Ok(Arc::new(Self {
            intelligence_actor,
            shutdown: AtomicBool::new(false),
            _system: PhantomData,
        }))
    }

    pub(crate) async fn register_source(
        &self,
        descriptor: EnvironmentSourceDescriptor,
    ) -> Result<EnvironmentSourceStatus, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.intelligence_actor,
            IntelligenceActorMsg::RegisterSource,
            ACTOR_CALL_TIMEOUT_MS,
            descriptor
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to register typed System 4 source: {err}"),
        })?
    }

    pub(crate) async fn list_sources(
        &self,
    ) -> Result<Vec<EnvironmentSourceStatus>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.intelligence_actor,
            IntelligenceActorMsg::ListSources,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to list typed System 4 sources: {err}"),
        })?
    }

    pub(crate) async fn collect_observations(
        &self,
    ) -> Result<Vec<EnvironmentalObservation>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.intelligence_actor,
            IntelligenceActorMsg::CollectObservations,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to collect typed System 4 observations: {err}"),
        })?
    }

    pub(crate) async fn run_cycle(&self) -> Result<System4IntelligenceCycle, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.intelligence_actor,
            IntelligenceActorMsg::RunCycle,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to run typed System 4 intelligence cycle: {err}"),
        })?
    }

    pub(crate) async fn calibrate(
        &self,
        actuals: Vec<EnvironmentalObservation>,
    ) -> Result<Vec<ForecastCalibration>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.intelligence_actor,
            IntelligenceActorMsg::Calibrate,
            ACTOR_CALL_TIMEOUT_MS,
            actuals
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to calibrate typed System 4 forecasts: {err}"),
        })?
    }

    pub(crate) async fn record_proposals(
        &self,
        proposals: Vec<AdaptationProposal>,
    ) -> Result<(), FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.intelligence_actor,
            IntelligenceActorMsg::RecordProposals,
            ACTOR_CALL_TIMEOUT_MS,
            proposals
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to record typed System 4 proposals: {err}"),
        })?
    }

    pub(crate) async fn snapshot(&self) -> Result<System4Snapshot, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.intelligence_actor,
            IntelligenceActorMsg::Snapshot,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to read typed System 4 snapshot: {err}"),
        })?
    }

    pub(crate) async fn shutdown(&self) -> Result<(), FrameworkError> {
        if self.shutdown.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        call_t!(
            self.intelligence_actor,
            IntelligenceActorMsg::Shutdown,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to shut down typed System 4 runtime: {err}"),
        })??;
        self.intelligence_actor
            .stop(Some("typed System 4 runtime shutdown".to_string()));

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

struct IntelligenceActor<V>
where
    V: ViableSystem,
{
    _system: PhantomData<V>,
}

impl<V> IntelligenceActor<V>
where
    V: ViableSystem,
{
    fn new() -> Self {
        Self {
            _system: PhantomData,
        }
    }
}

struct IntelligenceActorArgs<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    roles: System4RuntimeRoles<V>,
    context: RoleContext<V>,
}

struct RegisteredSource {
    actor: ActorRef<SourceActorMsg>,
    status: EnvironmentSourceStatus,
}

struct IntelligenceActorState<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    roles: System4RuntimeRoles<V>,
    context: RoleContext<V>,
    sources: HashMap<String, RegisteredSource>,
    observations: Vec<EnvironmentalObservation>,
    signals: Vec<InterpretedSignal>,
    assessments: Vec<IntelligenceAssessment>,
    forecasts: Vec<crate::protocol::system4::Forecast>,
    scenarios: Vec<crate::protocol::system4::Scenario>,
    proposals: Vec<AdaptationProposal>,
    calibrations: Vec<ForecastCalibration>,
    last_cycle_at: Option<DateTime<Utc>>,
}

enum IntelligenceActorMsg {
    RegisterSource(
        EnvironmentSourceDescriptor,
        RpcReplyPort<Result<EnvironmentSourceStatus, FrameworkError>>,
    ),
    ListSources(RpcReplyPort<Result<Vec<EnvironmentSourceStatus>, FrameworkError>>),
    CollectObservations(RpcReplyPort<Result<Vec<EnvironmentalObservation>, FrameworkError>>),
    RunCycle(RpcReplyPort<Result<System4IntelligenceCycle, FrameworkError>>),
    Calibrate(
        Vec<EnvironmentalObservation>,
        RpcReplyPort<Result<Vec<ForecastCalibration>, FrameworkError>>,
    ),
    RecordProposals(
        Vec<AdaptationProposal>,
        RpcReplyPort<Result<(), FrameworkError>>,
    ),
    Snapshot(RpcReplyPort<Result<System4Snapshot, FrameworkError>>),
    Shutdown(RpcReplyPort<Result<(), FrameworkError>>),
}

#[ractor::async_trait]
impl<V> Actor for IntelligenceActor<V>
where
    V: ViableSystem,
{
    type Msg = IntelligenceActorMsg;
    type State = IntelligenceActorState<V>;
    type Arguments = IntelligenceActorArgs<V>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(IntelligenceActorState {
            config: args.config,
            roles: args.roles,
            context: args.context,
            sources: HashMap::new(),
            observations: Vec::new(),
            signals: Vec::new(),
            assessments: Vec::new(),
            forecasts: Vec::new(),
            scenarios: Vec::new(),
            proposals: Vec::new(),
            calibrations: Vec::new(),
            last_cycle_at: None,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            IntelligenceActorMsg::RegisterSource(descriptor, reply) => {
                let result = register_source(state, descriptor).await;
                let _ = reply.send(result);
            }
            IntelligenceActorMsg::ListSources(reply) => {
                let _ = reply.send(Ok(source_statuses(state)));
            }
            IntelligenceActorMsg::CollectObservations(reply) => {
                let result = collect_observations(state).await;
                let _ = reply.send(result);
            }
            IntelligenceActorMsg::RunCycle(reply) => {
                let result = run_cycle(state).await;
                let _ = reply.send(result);
            }
            IntelligenceActorMsg::Calibrate(actuals, reply) => {
                let result = calibrate_forecasts(state, actuals).await;
                let _ = reply.send(result);
            }
            IntelligenceActorMsg::RecordProposals(proposals, reply) => {
                state.proposals = proposals;
                let _ = reply.send(Ok(()));
            }
            IntelligenceActorMsg::Snapshot(reply) => {
                let _ = reply.send(Ok(snapshot(state)));
            }
            IntelligenceActorMsg::Shutdown(reply) => {
                shutdown_sources(state).await;
                let _ = reply.send(Ok(()));
            }
        }

        Ok(())
    }
}

async fn register_source<V>(
    state: &mut IntelligenceActorState<V>,
    descriptor: EnvironmentSourceDescriptor,
) -> Result<EnvironmentSourceStatus, FrameworkError>
where
    V: ViableSystem,
{
    if state.sources.contains_key(&descriptor.source_id) {
        return Err(FrameworkError::InvalidProtocol {
            reason: format!(
                "System 4 source already registered: {}",
                descriptor.source_id
            ),
        });
    }

    let source_id = descriptor.source_id.clone();
    let source_context = source_context(&state.context, &source_id);
    let (actor, _join) = Actor::spawn(
        Some(system4_source_actor_name(&state.config, &source_id)),
        SourceActor::<V>::new(),
        SourceActorArgs {
            factory: state.roles.environmental_source_factory(),
            context: source_context,
            descriptor: descriptor.clone(),
        },
    )
    .await
    .map_err(|err| FrameworkError::Runtime {
        reason: format!("failed to spawn typed System 4 source actor {source_id}: {err}"),
    })?;

    let status = EnvironmentSourceStatus::new(descriptor.clone());
    state.sources.insert(
        source_id,
        RegisteredSource {
            actor,
            status: status.clone(),
        },
    );

    record_report(state, System4Report::SourceStatus(Box::new(status.clone()))).await;
    emit_event(
        state,
        System4Event::SourceRegistered(Box::new(status.clone())),
    )
    .await;

    Ok(status)
}

async fn collect_observations<V>(
    state: &mut IntelligenceActorState<V>,
) -> Result<Vec<EnvironmentalObservation>, FrameworkError>
where
    V: ViableSystem,
{
    let source_ids = state.sources.keys().cloned().collect::<Vec<_>>();
    let mut observations = Vec::new();

    for source_id in source_ids {
        let Some(registered) = state.sources.get(&source_id) else {
            continue;
        };
        let actor = registered.actor.clone();
        match call_t!(actor, SourceActorMsg::Observe, ACTOR_CALL_TIMEOUT_MS) {
            Ok(batch) => {
                state.sources.insert(
                    source_id.clone(),
                    RegisteredSource {
                        actor,
                        status: batch.status.clone(),
                    },
                );
                if batch.error.is_some() {
                    emit_event(
                        state,
                        System4Event::SourceObservationFailed(Box::new(batch.status.clone())),
                    )
                    .await;
                }
                record_report(
                    state,
                    System4Report::SourceStatus(Box::new(batch.status.clone())),
                )
                .await;
                for observation in &batch.observations {
                    record_report(
                        state,
                        System4Report::Observation(Box::new(observation.clone())),
                    )
                    .await;
                }
                observations.extend(batch.observations);
            }
            Err(err) => {
                if let Some(registered) = state.sources.get_mut(&source_id) {
                    registered.status.last_error = Some(err.to_string());
                    registered.status.freshness = FreshnessStatus::Stale;
                    let status = registered.status.clone();
                    emit_event(
                        state,
                        System4Event::SourceObservationFailed(Box::new(status.clone())),
                    )
                    .await;
                    record_report(state, System4Report::SourceStatus(Box::new(status))).await;
                }
            }
        }
    }

    state.observations.extend(observations.iter().cloned());
    let stale_source_count = source_statuses(state)
        .into_iter()
        .filter(|status| status.freshness != FreshnessStatus::Fresh)
        .count();
    emit_event(
        state,
        System4Event::ObservationsCollected {
            observation_count: observations.len(),
            stale_source_count,
        },
    )
    .await;

    Ok(observations)
}

async fn run_cycle<V>(
    state: &mut IntelligenceActorState<V>,
) -> Result<System4IntelligenceCycle, FrameworkError>
where
    V: ViableSystem,
{
    let observations = collect_observations(state).await?;
    let mut signals = state
        .roles
        .signal_interpreter()
        .interpret(&state.context, &observations)
        .await?;
    for signal in &mut signals {
        set_system4_metadata(&mut signal.metadata, &state.context);
    }

    let mut assessment = state
        .roles
        .intelligence_model()
        .assess(&state.context, &signals)
        .await?;
    set_system4_metadata(&mut assessment.metadata, &state.context);

    let mut forecasts = state
        .roles
        .forecaster()
        .forecast(&state.context, &assessment, &signals)
        .await?;
    for forecast in &mut forecasts {
        set_system4_metadata(&mut forecast.metadata, &state.context);
    }

    let mut scenarios = state
        .roles
        .forecaster()
        .plan_scenarios(&state.context, &assessment, &forecasts)
        .await?;
    for scenario in &mut scenarios {
        set_system4_metadata(&mut scenario.metadata, &state.context);
        if scenario.provenance.is_empty() {
            scenario
                .provenance
                .push(format!("assessment:{}", assessment.assessment_id));
            if let Some(forecast_id) = &scenario.forecast_id {
                scenario.provenance.push(format!("forecast:{forecast_id}"));
            }
        }
    }

    let mut proposals = state
        .roles
        .forecaster()
        .propose_adaptations(&state.context, &assessment, &forecasts, &scenarios)
        .await?;
    for proposal in &mut proposals {
        set_system4_metadata(&mut proposal.metadata, &state.context);
        if proposal.destination.is_none() {
            proposal.destination = Some(VsmAddress::new(
                state.context.runtime_id().clone(),
                state.context.recursion_path().clone(),
                SubsystemRole::System5,
            ));
        }
        if proposal.provenance.is_empty() {
            proposal
                .provenance
                .push(format!("assessment:{}", assessment.assessment_id));
            if let Some(scenario_id) = &proposal.scenario_id {
                proposal.provenance.push(format!("scenario:{scenario_id}"));
            }
        }
        if proposal.uncertainty == 0.0 {
            proposal.uncertainty = proposal
                .scenario_id
                .as_ref()
                .and_then(|scenario_id| {
                    scenarios
                        .iter()
                        .find(|scenario| &scenario.scenario_id == scenario_id)
                })
                .map(|scenario| scenario.uncertainty)
                .unwrap_or(assessment.uncertainty);
        }
    }

    let stale_sources = source_statuses(state)
        .into_iter()
        .filter(|status| status.freshness != FreshnessStatus::Fresh)
        .collect::<Vec<_>>();
    let cycle = System4IntelligenceCycle {
        metadata: state.context.metadata().clone(),
        observations: observations.clone(),
        signals: signals.clone(),
        assessment: assessment.clone(),
        forecasts: forecasts.clone(),
        scenarios: scenarios.clone(),
        proposals: proposals.clone(),
        stale_sources,
        generated_at: state.context.now(),
    };

    state.signals.extend(signals.iter().cloned());
    state.assessments.push(assessment.clone());
    state.forecasts.extend(forecasts.iter().cloned());
    state.scenarios.extend(scenarios.iter().cloned());
    state.proposals.extend(proposals.iter().cloned());
    state.last_cycle_at = Some(cycle.generated_at);

    for signal in &signals {
        record_report(state, System4Report::Signal(Box::new(signal.clone()))).await;
    }
    record_report(
        state,
        System4Report::Assessment(Box::new(assessment.clone())),
    )
    .await;
    for forecast in &forecasts {
        record_report(state, System4Report::Forecast(Box::new(forecast.clone()))).await;
    }
    for scenario in &scenarios {
        record_report(state, System4Report::Scenario(Box::new(scenario.clone()))).await;
    }
    for proposal in &proposals {
        record_report(state, System4Report::Proposal(Box::new(proposal.clone()))).await;
        emit_event(
            state,
            System4Event::AdaptationProposed(Box::new(proposal.clone())),
        )
        .await;
    }
    emit_event(
        state,
        System4Event::IntelligenceCycle {
            observation_count: observations.len(),
            signal_count: signals.len(),
            forecast_count: forecasts.len(),
            scenario_count: scenarios.len(),
            proposal_count: proposals.len(),
        },
    )
    .await;

    Ok(cycle)
}

async fn calibrate_forecasts<V>(
    state: &mut IntelligenceActorState<V>,
    actuals: Vec<EnvironmentalObservation>,
) -> Result<Vec<ForecastCalibration>, FrameworkError>
where
    V: ViableSystem,
{
    let mut calibrations = state
        .roles
        .forecaster()
        .calibrate(&state.context, &state.forecasts, &actuals)
        .await?;
    for calibration in &mut calibrations {
        set_system4_metadata(&mut calibration.metadata, &state.context);
        record_report(
            state,
            System4Report::Calibration(Box::new(calibration.clone())),
        )
        .await;
    }
    state.calibrations.extend(calibrations.iter().cloned());
    emit_event(
        state,
        System4Event::ForecastCalibrated {
            calibration_count: calibrations.len(),
        },
    )
    .await;
    Ok(calibrations)
}

fn source_statuses<V>(state: &IntelligenceActorState<V>) -> Vec<EnvironmentSourceStatus>
where
    V: ViableSystem,
{
    state
        .sources
        .values()
        .map(|registered| registered.status.clone())
        .collect()
}

fn snapshot<V>(state: &IntelligenceActorState<V>) -> System4Snapshot
where
    V: ViableSystem,
{
    System4Snapshot {
        sources: source_statuses(state),
        observations: state.observations.clone(),
        signals: state.signals.clone(),
        assessments: state.assessments.clone(),
        forecasts: state.forecasts.clone(),
        scenarios: state.scenarios.clone(),
        proposals: state.proposals.clone(),
        calibrations: state.calibrations.clone(),
        last_cycle_at: state.last_cycle_at,
    }
}

async fn shutdown_sources<V>(state: &mut IntelligenceActorState<V>)
where
    V: ViableSystem,
{
    for source in state.sources.values() {
        let _ = call_t!(
            source.actor,
            SourceActorMsg::Shutdown,
            ACTOR_CALL_TIMEOUT_MS
        );
        source
            .actor
            .stop(Some("typed System 4 source shutdown".to_string()));
    }
}

struct SourceActor<V>
where
    V: ViableSystem,
{
    _system: PhantomData<V>,
}

impl<V> SourceActor<V>
where
    V: ViableSystem,
{
    fn new() -> Self {
        Self {
            _system: PhantomData,
        }
    }
}

struct SourceActorArgs<V>
where
    V: ViableSystem,
{
    factory: crate::roles::SharedEnvironmentalSourceFactory<V>,
    context: RoleContext<V>,
    descriptor: EnvironmentSourceDescriptor,
}

struct SourceActorState<V>
where
    V: ViableSystem,
{
    factory: crate::roles::SharedEnvironmentalSourceFactory<V>,
    context: RoleContext<V>,
    descriptor: EnvironmentSourceDescriptor,
    source: BoxEnvironmentalSource<V>,
    status: EnvironmentSourceStatus,
}

enum SourceActorMsg {
    Observe(RpcReplyPort<SourceObservationBatch>),
    Shutdown(RpcReplyPort<Result<(), FrameworkError>>),
}

struct SourceObservationBatch {
    observations: Vec<EnvironmentalObservation>,
    status: EnvironmentSourceStatus,
    error: Option<String>,
}

#[ractor::async_trait]
impl<V> Actor for SourceActor<V>
where
    V: ViableSystem,
{
    type Msg = SourceActorMsg;
    type State = SourceActorState<V>;
    type Arguments = SourceActorArgs<V>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let source = args
            .factory
            .create_source(&args.context, &args.descriptor)
            .await
            .map_err(|err| -> ActorProcessingErr { err.into() })?;
        Ok(SourceActorState {
            factory: args.factory,
            context: args.context,
            status: EnvironmentSourceStatus::new(args.descriptor.clone()),
            descriptor: args.descriptor,
            source,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            SourceActorMsg::Observe(reply) => {
                let result = observe_source(state).await;
                let _ = reply.send(result);
            }
            SourceActorMsg::Shutdown(reply) => {
                let _ = reply.send(Ok(()));
            }
        }

        Ok(())
    }
}

async fn observe_source<V>(state: &mut SourceActorState<V>) -> SourceObservationBatch
where
    V: ViableSystem,
{
    match state
        .source
        .observe(&state.context, &state.descriptor)
        .await
    {
        Ok(mut observations) => {
            for observation in &mut observations {
                normalize_observation(observation, &state.context, &state.descriptor);
            }
            state.status.observation_count += observations.len();
            state.status.last_observed_at = observations
                .iter()
                .map(|observation| observation.observed_at)
                .max()
                .or_else(|| Some(state.context.now()));
            state.status.last_error = None;
            refresh_source_freshness(&mut state.status, state.context.now());
            SourceObservationBatch {
                observations,
                status: state.status.clone(),
                error: None,
            }
        }
        Err(err) => {
            let error = err.to_string();
            state.status.last_error = Some(error.clone());
            state.status.freshness = FreshnessStatus::Stale;
            state.status.restart_count += 1;
            match state
                .factory
                .create_source(&state.context, &state.descriptor)
                .await
            {
                Ok(source) => {
                    state.source = source;
                }
                Err(restart_err) => {
                    state.status.last_error =
                        Some(format!("{error}; restart failed: {restart_err}"));
                }
            }
            SourceObservationBatch {
                observations: Vec::new(),
                status: state.status.clone(),
                error: Some(error),
            }
        }
    }
}

fn normalize_observation<V>(
    observation: &mut EnvironmentalObservation,
    context: &RoleContext<V>,
    descriptor: &EnvironmentSourceDescriptor,
) where
    V: ViableSystem,
{
    observation.source_id = descriptor.source_id.clone();
    observation.received_at = context.now();
    set_system4_metadata(&mut observation.metadata, context);
    observation.metadata.source = Some(
        VsmAddress::new(
            context.runtime_id().clone(),
            context.recursion_path().clone(),
            SubsystemRole::System4,
        )
        .with_entity(descriptor.source_id.clone()),
    );
    if observation.provenance.is_empty() {
        observation.provenance = descriptor.provenance.clone();
    }
    observation.freshness =
        observation_freshness(observation.observed_at, context.now(), descriptor);
}

fn observation_freshness(
    observed_at: DateTime<Utc>,
    now: DateTime<Utc>,
    descriptor: &EnvironmentSourceDescriptor,
) -> FreshnessStatus {
    let Some(stale_after) = descriptor.stale_after else {
        return FreshnessStatus::Fresh;
    };
    let Ok(age) = (now - observed_at).to_std() else {
        return FreshnessStatus::Fresh;
    };
    if age <= stale_after {
        FreshnessStatus::Fresh
    } else if age <= stale_after.checked_mul(2).unwrap_or(stale_after) {
        FreshnessStatus::Stale
    } else {
        FreshnessStatus::Expired
    }
}

fn refresh_source_freshness(status: &mut EnvironmentSourceStatus, now: DateTime<Utc>) {
    let freshness = status
        .last_observed_at
        .map(|observed_at| observation_freshness(observed_at, now, &status.descriptor))
        .unwrap_or(FreshnessStatus::Fresh);
    status.freshness = freshness;
}

fn set_system4_metadata<V>(metadata: &mut ProtocolMetadata, context: &RoleContext<V>)
where
    V: ViableSystem,
{
    metadata.source = Some(VsmAddress::new(
        context.runtime_id().clone(),
        context.recursion_path().clone(),
        SubsystemRole::System4,
    ));
}

fn source_context<V>(context: &RoleContext<V>, source_id: &str) -> RoleContext<V>
where
    V: ViableSystem,
{
    let mut metadata = context.metadata().child();
    metadata.source = Some(
        VsmAddress::new(
            context.runtime_id().clone(),
            context.recursion_path().clone(),
            SubsystemRole::System4,
        )
        .with_entity(source_id.to_string()),
    );
    context.clone().with_metadata(metadata)
}

pub(crate) fn feasibility_from_system3_snapshot<V>(
    context: &RoleContext<V>,
    snapshot: Option<&crate::protocol::system3::System3Snapshot<V>>,
    unavailable_reason: Option<String>,
) -> OperationalFeasibilityInfo
where
    V: ViableSystem,
{
    let assessed_by = Some(VsmAddress::new(
        context.runtime_id().clone(),
        context.recursion_path().clone(),
        SubsystemRole::System3,
    ));

    match snapshot {
        Some(snapshot) => OperationalFeasibilityInfo {
            requested_at: context.now(),
            assessed_by,
            summary: format!(
                "System 3 snapshot observed: {} allocations, {} directives, {} audit responses",
                snapshot.allocations.len(),
                snapshot.directives.len(),
                snapshot.audit_responses.len()
            ),
            constraints: snapshot
                .summaries
                .iter()
                .map(|summary| {
                    format!(
                        "{} affected units, {} failed acknowledgements",
                        summary.affected_units.len(),
                        summary.failed_acknowledgement_count
                    )
                })
                .collect(),
            confidence: 1.0,
        },
        None => OperationalFeasibilityInfo {
            requested_at: context.now(),
            assessed_by,
            summary: unavailable_reason
                .unwrap_or_else(|| "System 3 snapshot unavailable".to_string()),
            constraints: Vec::new(),
            confidence: 0.0,
        },
    }
}

async fn emit_event<V>(state: &IntelligenceActorState<V>, event: System4Event)
where
    V: ViableSystem,
{
    let _ = state
        .context
        .emit_event(RuntimeEvent::System4(Box::new(event)))
        .await;
}

async fn record_report<V>(state: &IntelligenceActorState<V>, report: System4Report)
where
    V: ViableSystem,
{
    let _ = state
        .context
        .record_report(RuntimeReport::System4(Box::new(report)))
        .await;
}

fn system4_actor_name(config: &RuntimeConfig, role: &str) -> String {
    format!("typed.{}.system4.{role}", config.runtime_id.as_str())
}

fn system4_source_actor_name(config: &RuntimeConfig, source_id: &str) -> String {
    format!(
        "typed.{}.system4.source.{}",
        config.runtime_id.as_str(),
        source_id
    )
}
