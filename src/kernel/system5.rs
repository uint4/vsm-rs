//! Private typed System 5 runtime adapter.

use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ractor::{call_t, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

use crate::config::RuntimeConfig;
use crate::error::FrameworkError;
use crate::protocol::events::{RuntimeEvent, RuntimeReport, System5Event, System5Report};
use crate::protocol::system5::{
    CrisisResponse, CrisisSignal, DecisionEvidence, DecisionRecord, DecisionRequest,
    IdentityRecord, PolicyDirective, PolicyDirectiveAcknowledgement, PolicyEscalation,
    System5DecisionCycle, System5Snapshot, ValueSet, ValuesEvaluation,
};
use crate::protocol::{ProtocolMetadata, SubsystemRole, VsmAddress};
use crate::roles::{RoleContext, ViableSystem};
use crate::runtime::{RuntimePorts, System5RuntimeRoles};

const ACTOR_CALL_TIMEOUT_MS: u64 = 1_000;

pub(crate) struct System5Runtime<V>
where
    V: ViableSystem,
{
    policy_actor: ActorRef<PolicyActorMsg<V>>,
    shutdown: AtomicBool,
    _system: PhantomData<fn() -> V>,
}

impl<V> System5Runtime<V>
where
    V: ViableSystem,
{
    pub(crate) async fn start(
        config: RuntimeConfig,
        roles: System5RuntimeRoles<V>,
        ports: RuntimePorts<V>,
    ) -> Result<Arc<Self>, FrameworkError> {
        let context = ports.role_context(
            config.runtime_id.clone(),
            config.recursion_path.clone(),
            SubsystemRole::System5,
        );

        let (policy_actor, _join) = Actor::spawn(
            Some(system5_actor_name(&config, "policy")),
            PolicyActor::<V>::new(),
            PolicyActorArgs {
                config,
                roles,
                context,
            },
        )
        .await
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to spawn typed System 5 policy actor: {err}"),
        })?;

        Ok(Arc::new(Self {
            policy_actor,
            shutdown: AtomicBool::new(false),
            _system: PhantomData,
        }))
    }

    pub(crate) async fn identity(&self) -> Result<IdentityRecord, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.policy_actor,
            PolicyActorMsg::Identity,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to read typed System 5 identity: {err}"),
        })?
    }

    pub(crate) async fn values(&self) -> Result<ValueSet, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.policy_actor,
            PolicyActorMsg::Values,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to read typed System 5 values: {err}"),
        })?
    }

    pub(crate) async fn decide(
        &self,
        request: DecisionRequest<V>,
    ) -> Result<System5DecisionCycle<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.policy_actor,
            PolicyActorMsg::Decide,
            ACTOR_CALL_TIMEOUT_MS,
            request
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to run typed System 5 decision cycle: {err}"),
        })?
    }

    pub(crate) async fn handle_crisis(
        &self,
        signal: CrisisSignal,
    ) -> Result<CrisisResponse<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.policy_actor,
            PolicyActorMsg::HandleCrisis,
            ACTOR_CALL_TIMEOUT_MS,
            signal
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to run typed System 5 crisis policy: {err}"),
        })?
    }

    pub(crate) async fn acknowledge_directives(
        &self,
        acknowledgements: Vec<PolicyDirectiveAcknowledgement<V>>,
    ) -> Result<System5Snapshot<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.policy_actor,
            PolicyActorMsg::AcknowledgeDirectives,
            ACTOR_CALL_TIMEOUT_MS,
            acknowledgements
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to record typed System 5 directive acknowledgements: {err}"),
        })?
    }

    pub(crate) async fn snapshot(&self) -> Result<System5Snapshot<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.policy_actor,
            PolicyActorMsg::Snapshot,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to read typed System 5 snapshot: {err}"),
        })?
    }

    pub(crate) async fn shutdown(&self) -> Result<(), FrameworkError> {
        if self.shutdown.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        call_t!(
            self.policy_actor,
            PolicyActorMsg::Shutdown,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to shut down typed System 5 runtime: {err}"),
        })??;
        self.policy_actor
            .stop(Some("typed System 5 runtime shutdown".to_string()));

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

struct PolicyActor<V>
where
    V: ViableSystem,
{
    _system: PhantomData<V>,
}

impl<V> PolicyActor<V>
where
    V: ViableSystem,
{
    fn new() -> Self {
        Self {
            _system: PhantomData,
        }
    }
}

struct PolicyActorArgs<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    roles: System5RuntimeRoles<V>,
    context: RoleContext<V>,
}

struct PolicyActorState<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    roles: System5RuntimeRoles<V>,
    context: RoleContext<V>,
    identity: Option<IdentityRecord>,
    values: Option<ValueSet>,
    decisions: Vec<DecisionRecord<V>>,
    directives: Vec<PolicyDirective<V>>,
    directive_acknowledgements: Vec<PolicyDirectiveAcknowledgement<V>>,
    crises: Vec<CrisisResponse<V>>,
    escalations: Vec<PolicyEscalation>,
}

enum PolicyActorMsg<V>
where
    V: ViableSystem,
{
    Identity(RpcReplyPort<Result<IdentityRecord, FrameworkError>>),
    Values(RpcReplyPort<Result<ValueSet, FrameworkError>>),
    Decide(
        DecisionRequest<V>,
        RpcReplyPort<Result<System5DecisionCycle<V>, FrameworkError>>,
    ),
    HandleCrisis(
        CrisisSignal,
        RpcReplyPort<Result<CrisisResponse<V>, FrameworkError>>,
    ),
    AcknowledgeDirectives(
        Vec<PolicyDirectiveAcknowledgement<V>>,
        RpcReplyPort<Result<System5Snapshot<V>, FrameworkError>>,
    ),
    Snapshot(RpcReplyPort<Result<System5Snapshot<V>, FrameworkError>>),
    Shutdown(RpcReplyPort<Result<(), FrameworkError>>),
}

#[ractor::async_trait]
impl<V> Actor for PolicyActor<V>
where
    V: ViableSystem,
{
    type Msg = PolicyActorMsg<V>;
    type State = PolicyActorState<V>;
    type Arguments = PolicyActorArgs<V>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(PolicyActorState {
            config: args.config,
            roles: args.roles,
            context: args.context,
            identity: None,
            values: None,
            decisions: Vec::new(),
            directives: Vec::new(),
            directive_acknowledgements: Vec::new(),
            crises: Vec::new(),
            escalations: Vec::new(),
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            PolicyActorMsg::Identity(reply) => {
                let result = provide_identity(state).await;
                let _ = reply.send(result);
            }
            PolicyActorMsg::Values(reply) => {
                let result = provide_values(state).await;
                let _ = reply.send(result);
            }
            PolicyActorMsg::Decide(request, reply) => {
                let result = decide(state, request).await;
                let _ = reply.send(result);
            }
            PolicyActorMsg::HandleCrisis(signal, reply) => {
                let result = handle_crisis(state, signal).await;
                let _ = reply.send(result);
            }
            PolicyActorMsg::AcknowledgeDirectives(acknowledgements, reply) => {
                let result = acknowledge_directives(state, acknowledgements).await;
                let _ = reply.send(result);
            }
            PolicyActorMsg::Snapshot(reply) => {
                let _ = reply.send(Ok(snapshot(state)));
            }
            PolicyActorMsg::Shutdown(reply) => {
                let _ = reply.send(Ok(()));
            }
        }

        Ok(())
    }
}

async fn provide_identity<V>(
    state: &mut PolicyActorState<V>,
) -> Result<IdentityRecord, FrameworkError>
where
    V: ViableSystem,
{
    let mut identity = state
        .roles
        .identity_provider()
        .provide_identity(&state.context)
        .await?;
    set_system5_metadata(&mut identity.metadata, &state.context);
    state.identity = Some(identity.clone());
    record_report(state, System5Report::Identity(Box::new(identity.clone()))).await;
    Ok(identity)
}

async fn provide_values<V>(state: &mut PolicyActorState<V>) -> Result<ValueSet, FrameworkError>
where
    V: ViableSystem,
{
    let mut values = state
        .roles
        .values_provider()
        .provide_values(&state.context)
        .await?;
    set_system5_metadata(&mut values.metadata, &state.context);
    state.values = Some(values.clone());
    record_report(state, System5Report::Values(Box::new(values.clone()))).await;
    Ok(values)
}

async fn decide<V>(
    state: &mut PolicyActorState<V>,
    mut request: DecisionRequest<V>,
) -> Result<System5DecisionCycle<V>, FrameworkError>
where
    V: ViableSystem,
{
    set_system5_metadata(&mut request.metadata, &state.context);
    enrich_request_with_context(&mut request);

    let identity = provide_identity(state).await?;
    let values = provide_values(state).await?;
    let mut evaluation = state
        .roles
        .values_evaluator()
        .evaluate_values(&state.context, &request, &identity, &values)
        .await?;
    set_system5_metadata(&mut evaluation.metadata, &state.context);

    let mut decision = state
        .roles
        .decision_policy()
        .decide(&state.context, &request, &identity, &values, &evaluation)
        .await?;
    normalize_decision(&mut decision, &request, &evaluation, &state.context);

    record_decision(state, &decision).await;
    let cycle = System5DecisionCycle {
        metadata: request.metadata.child(),
        request,
        identity,
        values,
        evaluation: evaluation.clone(),
        directive_acknowledgements: state.directive_acknowledgements.clone(),
        escalations: decision.escalations.clone(),
        decided_at: decision.decided_at,
        decision,
    };

    emit_event(
        state,
        System5Event::DecisionRecorded {
            decision_id: cycle.decision.decision_id.clone(),
            directive_count: cycle.decision.directives.len(),
            escalation_count: cycle.decision.escalations.len(),
        },
    )
    .await;

    Ok(cycle)
}

async fn handle_crisis<V>(
    state: &mut PolicyActorState<V>,
    mut signal: CrisisSignal,
) -> Result<CrisisResponse<V>, FrameworkError>
where
    V: ViableSystem,
{
    set_system5_metadata(&mut signal.metadata, &state.context);
    if signal.source.is_none() {
        signal.source = Some(VsmAddress::new(
            state.config.runtime_id.clone(),
            state.config.recursion_path.clone(),
            SubsystemRole::Algedonic,
        ));
    }

    let identity = provide_identity(state).await?;
    let values = provide_values(state).await?;
    let mut response = state
        .roles
        .crisis_policy()
        .respond_to_crisis(&state.context, &signal, &identity, &values)
        .await?;
    normalize_crisis_response(&mut response, &signal, &state.context);

    record_decision(state, &response.decision).await;
    for escalation in &response.escalations {
        record_report(
            state,
            System5Report::Escalation(Box::new(escalation.clone())),
        )
        .await;
    }
    record_report(
        state,
        System5Report::CrisisResponse(Box::new(response.clone())),
    )
    .await;
    state.crises.push(response.clone());

    emit_event(
        state,
        System5Event::CrisisHandled {
            signal_id: signal.signal_id.clone(),
            directive_count: response.directives.len(),
            escalation_count: response.escalations.len(),
        },
    )
    .await;

    Ok(response)
}

async fn acknowledge_directives<V>(
    state: &mut PolicyActorState<V>,
    acknowledgements: Vec<PolicyDirectiveAcknowledgement<V>>,
) -> Result<System5Snapshot<V>, FrameworkError>
where
    V: ViableSystem,
{
    for acknowledgement in acknowledgements {
        let success = acknowledgement.status.is_success();
        record_report(
            state,
            System5Report::DirectiveAcknowledgement(Box::new(acknowledgement.clone())),
        )
        .await;
        emit_event(
            state,
            System5Event::DirectiveAcknowledged {
                directive_id: acknowledgement.directive_id.clone(),
                status: acknowledgement.status,
                success,
            },
        )
        .await;
        state.directive_acknowledgements.push(acknowledgement);
    }

    Ok(snapshot(state))
}

async fn record_decision<V>(state: &mut PolicyActorState<V>, decision: &DecisionRecord<V>)
where
    V: ViableSystem,
{
    state.directives.extend(decision.directives.iter().cloned());
    state
        .escalations
        .extend(decision.escalations.iter().cloned());
    state.decisions.push(decision.clone());

    record_report(state, System5Report::Decision(Box::new(decision.clone()))).await;
    for directive in &decision.directives {
        record_report(state, System5Report::Directive(Box::new(directive.clone()))).await;
        emit_event(
            state,
            System5Event::DirectiveIssued {
                directive_id: directive.directive_id.clone(),
                requires_ack: directive.requires_ack,
            },
        )
        .await;
    }
    for escalation in &decision.escalations {
        record_report(
            state,
            System5Report::Escalation(Box::new(escalation.clone())),
        )
        .await;
    }
}

fn enrich_request_with_context<V>(request: &mut DecisionRequest<V>)
where
    V: ViableSystem,
{
    let operational_evidence = request
        .operational_summaries
        .iter()
        .map(DecisionEvidence::from_operational_summary);
    let proposal_evidence = request
        .adaptation_proposals
        .iter()
        .map(DecisionEvidence::from_adaptation_proposal);
    request.evidence.extend(operational_evidence);
    request.evidence.extend(proposal_evidence);
}

fn normalize_decision<V>(
    decision: &mut DecisionRecord<V>,
    request: &DecisionRequest<V>,
    evaluation: &ValuesEvaluation,
    context: &RoleContext<V>,
) where
    V: ViableSystem,
{
    set_system5_metadata(&mut decision.metadata, context);
    if decision.request_id.is_empty() {
        decision.request_id = request.request_id.clone();
    }
    if decision.subject.is_empty() {
        decision.subject = request.subject.clone();
    }
    if decision.evidence.is_empty() {
        decision.evidence = request.evidence.clone();
    }
    if decision.evaluation.is_none() {
        decision.evaluation = Some(evaluation.clone());
    }
    decision.identity_version = evaluation.identity_version;
    decision.values_version = evaluation.values_version;
    decision.policy_version = decision.authority.policy_version;
    if decision.authority.issued_by.is_none() {
        decision.authority.issued_by = Some(system5_address(context));
    }

    for directive in &mut decision.directives {
        normalize_directive(directive, context);
    }
    for escalation in &mut decision.escalations {
        set_system5_metadata(&mut escalation.metadata, context);
    }
}

fn normalize_crisis_response<V>(
    response: &mut CrisisResponse<V>,
    signal: &CrisisSignal,
    context: &RoleContext<V>,
) where
    V: ViableSystem,
{
    set_system5_metadata(&mut response.metadata, context);
    response.signal_id = signal.signal_id.clone();
    set_system5_metadata(&mut response.decision.metadata, context);
    if response.decision.authority.issued_by.is_none() {
        response.decision.authority.issued_by = Some(system5_address(context));
    }
    response.decision.policy_version = response.decision.authority.policy_version;
    for directive in &mut response.decision.directives {
        normalize_directive(directive, context);
    }
    for escalation in &mut response.decision.escalations {
        set_system5_metadata(&mut escalation.metadata, context);
    }
    response.directives = response.decision.directives.clone();
    response.escalations = response.decision.escalations.clone();
    for directive in &mut response.directives {
        normalize_directive(directive, context);
    }
    for escalation in &mut response.escalations {
        set_system5_metadata(&mut escalation.metadata, context);
    }
}

fn normalize_directive<V>(directive: &mut PolicyDirective<V>, context: &RoleContext<V>)
where
    V: ViableSystem,
{
    set_system5_metadata(&mut directive.metadata, context);
    if directive.authority.issued_by.is_none() {
        directive.authority.issued_by = Some(system5_address(context));
    }
    directive.version = directive.authority.policy_version;
}

fn snapshot<V>(state: &PolicyActorState<V>) -> System5Snapshot<V>
where
    V: ViableSystem,
{
    System5Snapshot {
        identity: state.identity.clone(),
        values: state.values.clone(),
        decisions: state.decisions.clone(),
        directives: state.directives.clone(),
        directive_acknowledgements: state.directive_acknowledgements.clone(),
        crises: state.crises.clone(),
        escalations: state.escalations.clone(),
        last_decision_at: state.decisions.last().map(|decision| decision.decided_at),
    }
}

fn set_system5_metadata<V>(metadata: &mut ProtocolMetadata, context: &RoleContext<V>)
where
    V: ViableSystem,
{
    metadata.source = Some(system5_address(context));
}

fn system5_address<V>(context: &RoleContext<V>) -> VsmAddress
where
    V: ViableSystem,
{
    VsmAddress::new(
        context.runtime_id().clone(),
        context.recursion_path().clone(),
        SubsystemRole::System5,
    )
}

async fn emit_event<V>(state: &PolicyActorState<V>, event: System5Event)
where
    V: ViableSystem,
{
    let _ = state
        .context
        .emit_event(RuntimeEvent::System5(Box::new(event)))
        .await;
}

async fn record_report<V>(state: &PolicyActorState<V>, report: System5Report<V>)
where
    V: ViableSystem,
{
    let _ = state
        .context
        .record_report(RuntimeReport::System5(Box::new(report)))
        .await;
}

fn system5_actor_name(config: &RuntimeConfig, role: &str) -> String {
    format!("typed.{}.system5.{role}", config.runtime_id.as_str())
}
