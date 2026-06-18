//! Private typed System 3 runtime adapters.

use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use ractor::{call_t, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

use crate::config::RuntimeConfig;
use crate::error::FrameworkError;
use crate::protocol::events::{RuntimeEvent, RuntimeReport, System3Event, System3Report};
use crate::protocol::system1::{AuditEvidence, PerformanceObservation};
use crate::protocol::system3::{
    AuditResponse, AuthorityScope, ControlAuthority, DirectiveAcknowledgement,
    OperationalDirective, OperationalSummary, ResourceAllocation,
    ResourceAllocationAcknowledgement, ResourceRequest, System3AuditRequest, System3ControlCycle,
    System3Snapshot,
};
use crate::protocol::{ProtocolMetadata, SubsystemRole, VsmAddress};
use crate::roles::{RoleContext, ViableSystem};
use crate::runtime::{RuntimePorts, System3RuntimeRoles};

const ACTOR_CALL_TIMEOUT_MS: u64 = 1_000;

type AuditSnapshotResult<V> =
    Result<(Vec<AuditResponse<V>>, Option<DateTime<Utc>>), FrameworkError>;

pub(crate) struct System3Runtime<V>
where
    V: ViableSystem,
{
    control_actor: ActorRef<ControlActorMsg<V>>,
    audit_actor: ActorRef<AuditActorMsg<V>>,
    shutdown: AtomicBool,
}

impl<V> System3Runtime<V>
where
    V: ViableSystem,
{
    pub(crate) async fn start(
        config: RuntimeConfig,
        roles: System3RuntimeRoles<V>,
        ports: RuntimePorts<V>,
    ) -> Result<Arc<Self>, FrameworkError> {
        let control_context = ports.role_context(
            config.runtime_id.clone(),
            config.recursion_path.clone(),
            SubsystemRole::System3,
        );
        let audit_context = ports.role_context(
            config.runtime_id.clone(),
            config.recursion_path.clone(),
            SubsystemRole::System3Star,
        );

        let (control_actor, _control_join) = Actor::spawn(
            Some(system3_actor_name(&config, "control")),
            ControlActor::<V>::new(),
            ControlActorArgs {
                roles: roles.clone(),
                context: control_context,
            },
        )
        .await
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to spawn typed System 3 control actor: {err}"),
        })?;

        let (audit_actor, _audit_join) = Actor::spawn(
            Some(system3_actor_name(&config, "audit")),
            AuditActor::<V>::new(),
            AuditActorArgs {
                roles,
                context: audit_context,
            },
        )
        .await
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to spawn typed System 3* audit actor: {err}"),
        })?;

        Ok(Arc::new(Self {
            control_actor,
            audit_actor,
            shutdown: AtomicBool::new(false),
        }))
    }

    pub(crate) async fn govern_resources(
        &self,
        requests: Vec<ResourceRequest<V>>,
        performance: Vec<PerformanceObservation<V>>,
    ) -> Result<System3ControlCycle<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.control_actor,
            ControlActorMsg::GovernResources,
            ACTOR_CALL_TIMEOUT_MS,
            requests,
            performance
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to run typed System 3 resource governance: {err}"),
        })?
    }

    pub(crate) async fn record_directive_acknowledgements(
        &self,
        acknowledgements: Vec<DirectiveAcknowledgement<V>>,
    ) -> Result<System3ControlCycle<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.control_actor,
            ControlActorMsg::RecordDirectiveAcknowledgements,
            ACTOR_CALL_TIMEOUT_MS,
            acknowledgements
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to record typed System 3 directive acknowledgements: {err}"),
        })?
    }

    pub(crate) async fn perform_audit(
        &self,
        request: System3AuditRequest<V>,
        evidence: Vec<AuditEvidence<V>>,
    ) -> Result<AuditResponse<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.audit_actor,
            AuditActorMsg::Audit,
            ACTOR_CALL_TIMEOUT_MS,
            Box::new(request),
            evidence
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to run typed System 3* audit: {err}"),
        })?
    }

    pub(crate) async fn snapshot(&self) -> Result<System3Snapshot<V>, FrameworkError> {
        self.ensure_running()?;
        let mut snapshot = call_t!(
            self.control_actor,
            ControlActorMsg::Snapshot,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to read typed System 3 control snapshot: {err}"),
        })??;
        let (audit_responses, last_audit_at) = call_t!(
            self.audit_actor,
            AuditActorMsg::Snapshot,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to read typed System 3* audit snapshot: {err}"),
        })??;
        snapshot.audit_responses = audit_responses;
        snapshot.last_audit_at = last_audit_at;
        Ok(snapshot)
    }

    pub(crate) async fn shutdown(&self) -> Result<(), FrameworkError> {
        if self.shutdown.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        call_t!(
            self.audit_actor,
            AuditActorMsg::Shutdown,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to shut down typed System 3* runtime: {err}"),
        })??;
        self.audit_actor
            .stop(Some("typed System 3* runtime shutdown".to_string()));

        call_t!(
            self.control_actor,
            ControlActorMsg::Shutdown,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to shut down typed System 3 runtime: {err}"),
        })??;
        self.control_actor
            .stop(Some("typed System 3 runtime shutdown".to_string()));

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

struct ControlActor<V>
where
    V: ViableSystem,
{
    _system: PhantomData<V>,
}

impl<V> ControlActor<V>
where
    V: ViableSystem,
{
    fn new() -> Self {
        Self {
            _system: PhantomData,
        }
    }
}

struct ControlActorArgs<V>
where
    V: ViableSystem,
{
    roles: System3RuntimeRoles<V>,
    context: RoleContext<V>,
}

struct ControlActorState<V>
where
    V: ViableSystem,
{
    roles: System3RuntimeRoles<V>,
    context: RoleContext<V>,
    resource_requests: Vec<ResourceRequest<V>>,
    performance: Vec<PerformanceObservation<V>>,
    allocations: Vec<ResourceAllocation<V>>,
    allocation_acknowledgements: Vec<ResourceAllocationAcknowledgement<V>>,
    directives: Vec<OperationalDirective<V>>,
    directive_acknowledgements: Vec<DirectiveAcknowledgement<V>>,
    summaries: Vec<OperationalSummary<V>>,
    last_cycle_at: Option<DateTime<Utc>>,
}

enum ControlActorMsg<V>
where
    V: ViableSystem,
{
    GovernResources(
        Vec<ResourceRequest<V>>,
        Vec<PerformanceObservation<V>>,
        RpcReplyPort<Result<System3ControlCycle<V>, FrameworkError>>,
    ),
    RecordDirectiveAcknowledgements(
        Vec<DirectiveAcknowledgement<V>>,
        RpcReplyPort<Result<System3ControlCycle<V>, FrameworkError>>,
    ),
    Snapshot(RpcReplyPort<Result<System3Snapshot<V>, FrameworkError>>),
    Shutdown(RpcReplyPort<Result<(), FrameworkError>>),
}

#[ractor::async_trait]
impl<V> Actor for ControlActor<V>
where
    V: ViableSystem,
{
    type Msg = ControlActorMsg<V>;
    type State = ControlActorState<V>;
    type Arguments = ControlActorArgs<V>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(ControlActorState {
            roles: args.roles,
            context: args.context,
            resource_requests: Vec::new(),
            performance: Vec::new(),
            allocations: Vec::new(),
            allocation_acknowledgements: Vec::new(),
            directives: Vec::new(),
            directive_acknowledgements: Vec::new(),
            summaries: Vec::new(),
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
            ControlActorMsg::GovernResources(requests, performance, reply) => {
                let result = govern_resources(state, requests, performance).await;
                let _ = reply.send(result);
            }
            ControlActorMsg::RecordDirectiveAcknowledgements(acknowledgements, reply) => {
                let result = record_directive_acknowledgements(state, acknowledgements).await;
                let _ = reply.send(result);
            }
            ControlActorMsg::Snapshot(reply) => {
                let _ = reply.send(Ok(control_snapshot(state)));
            }
            ControlActorMsg::Shutdown(reply) => {
                let _ = reply.send(Ok(()));
            }
        }

        Ok(())
    }
}

async fn govern_resources<V>(
    state: &mut ControlActorState<V>,
    mut requests: Vec<ResourceRequest<V>>,
    performance: Vec<PerformanceObservation<V>>,
) -> Result<System3ControlCycle<V>, FrameworkError>
where
    V: ViableSystem,
{
    for request in &mut requests {
        set_system3_metadata(&mut request.metadata, &state.context);
    }

    let mut allocations = state
        .roles
        .resource_governance()
        .allocate_resources(&state.context, &requests, &performance)
        .await?;
    for allocation in &mut allocations {
        set_system3_metadata(&mut allocation.metadata, &state.context);
        ensure_authority(
            &mut allocation.authority,
            &state.context,
            AuthorityScope::ResourceGovernance,
        );
    }

    let allocation_acknowledgements = allocations
        .iter()
        .filter(|allocation| allocation.requires_ack)
        .map(ResourceAllocationAcknowledgement::accepted)
        .collect::<Vec<_>>();

    let mut directives = state
        .roles
        .operational_control_policy()
        .plan_directives(&state.context, &allocations, &performance)
        .await?;
    for directive in &mut directives {
        set_system3_metadata(&mut directive.metadata, &state.context);
        ensure_authority(
            &mut directive.authority,
            &state.context,
            AuthorityScope::OperationalControl,
        );
    }

    let summary = operational_summary(&state.context, &requests, &allocations, &directives, &[]);

    state.resource_requests.extend(requests.iter().cloned());
    state.performance.extend(performance.iter().cloned());
    state.allocations.extend(allocations.iter().cloned());
    state
        .allocation_acknowledgements
        .extend(allocation_acknowledgements.iter().cloned());
    state.directives.extend(directives.iter().cloned());
    state.summaries.push(summary.clone());
    state.last_cycle_at = Some(Utc::now());

    for request in &requests {
        record_report(
            state,
            System3Report::ResourceRequest(Box::new(request.clone())),
        )
        .await;
    }
    for allocation in &allocations {
        record_report(
            state,
            System3Report::Allocation(Box::new(allocation.clone())),
        )
        .await;
    }
    for acknowledgement in &allocation_acknowledgements {
        record_report(
            state,
            System3Report::AllocationAcknowledgement(Box::new(acknowledgement.clone())),
        )
        .await;
        emit_event(
            state,
            System3Event::AllocationAcknowledged(Box::new(acknowledgement.clone())),
        )
        .await;
    }
    for directive in &directives {
        record_report(state, System3Report::Directive(Box::new(directive.clone()))).await;
    }
    record_report(
        state,
        System3Report::OperationalSummary(Box::new(summary.clone())),
    )
    .await;
    emit_event(
        state,
        System3Event::ResourceCycle {
            request_count: requests.len(),
            allocation_count: allocations.len(),
            directive_count: directives.len(),
        },
    )
    .await;

    Ok(System3ControlCycle {
        metadata: state.context.metadata().clone(),
        resource_requests: requests,
        performance,
        allocations,
        allocation_acknowledgements,
        directives,
        directive_acknowledgements: Vec::new(),
        summaries: vec![summary],
    })
}

async fn record_directive_acknowledgements<V>(
    state: &mut ControlActorState<V>,
    acknowledgements: Vec<DirectiveAcknowledgement<V>>,
) -> Result<System3ControlCycle<V>, FrameworkError>
where
    V: ViableSystem,
{
    for acknowledgement in &acknowledgements {
        record_report(
            state,
            System3Report::DirectiveAcknowledgement(Box::new(acknowledgement.clone())),
        )
        .await;
        if acknowledgement.status.is_success() {
            emit_event(
                state,
                System3Event::DirectiveAcknowledged(Box::new(acknowledgement.clone())),
            )
            .await;
        } else {
            emit_event(
                state,
                System3Event::DirectiveAcknowledgementFailed(Box::new(acknowledgement.clone())),
            )
            .await;
        }
    }

    state
        .directive_acknowledgements
        .extend(acknowledgements.iter().cloned());
    let summary = operational_summary(&state.context, &[], &[], &[], &acknowledgements);
    state.summaries.push(summary.clone());
    record_report(
        state,
        System3Report::OperationalSummary(Box::new(summary.clone())),
    )
    .await;

    Ok(System3ControlCycle {
        metadata: state.context.metadata().clone(),
        resource_requests: Vec::new(),
        performance: Vec::new(),
        allocations: Vec::new(),
        allocation_acknowledgements: Vec::new(),
        directives: Vec::new(),
        directive_acknowledgements: acknowledgements,
        summaries: vec![summary],
    })
}

fn operational_summary<V>(
    context: &RoleContext<V>,
    requests: &[ResourceRequest<V>],
    allocations: &[ResourceAllocation<V>],
    directives: &[OperationalDirective<V>],
    acknowledgements: &[DirectiveAcknowledgement<V>],
) -> OperationalSummary<V>
where
    V: ViableSystem,
{
    let mut affected_units = allocations
        .iter()
        .filter_map(|allocation| allocation.target_unit.clone())
        .collect::<Vec<_>>();
    affected_units.extend(
        directives
            .iter()
            .flat_map(|directive| directive.target_units.iter().cloned()),
    );
    affected_units.extend(
        acknowledgements
            .iter()
            .map(|acknowledgement| acknowledgement.unit_id.clone()),
    );

    OperationalSummary {
        metadata: context.metadata().clone(),
        summary_id: format!("summary-{}", uuid::Uuid::new_v4()),
        resource_request_count: requests.len(),
        allocation_count: allocations.len(),
        directive_count: directives.len(),
        failed_acknowledgement_count: acknowledgements
            .iter()
            .filter(|acknowledgement| !acknowledgement.status.is_success())
            .count(),
        generated_at: context.now(),
        affected_units,
    }
}

fn control_snapshot<V>(state: &ControlActorState<V>) -> System3Snapshot<V>
where
    V: ViableSystem,
{
    System3Snapshot {
        resource_requests: state.resource_requests.clone(),
        performance: state.performance.clone(),
        allocations: state.allocations.clone(),
        allocation_acknowledgements: state.allocation_acknowledgements.clone(),
        directives: state.directives.clone(),
        directive_acknowledgements: state.directive_acknowledgements.clone(),
        summaries: state.summaries.clone(),
        audit_responses: Vec::new(),
        last_control_cycle_at: state.last_cycle_at,
        last_audit_at: None,
    }
}

struct AuditActor<V>
where
    V: ViableSystem,
{
    _system: PhantomData<V>,
}

impl<V> AuditActor<V>
where
    V: ViableSystem,
{
    fn new() -> Self {
        Self {
            _system: PhantomData,
        }
    }
}

struct AuditActorArgs<V>
where
    V: ViableSystem,
{
    roles: System3RuntimeRoles<V>,
    context: RoleContext<V>,
}

struct AuditActorState<V>
where
    V: ViableSystem,
{
    roles: System3RuntimeRoles<V>,
    context: RoleContext<V>,
    responses: Vec<AuditResponse<V>>,
    last_audit_at: Option<DateTime<Utc>>,
}

enum AuditActorMsg<V>
where
    V: ViableSystem,
{
    Audit(
        Box<System3AuditRequest<V>>,
        Vec<AuditEvidence<V>>,
        RpcReplyPort<Result<AuditResponse<V>, FrameworkError>>,
    ),
    Snapshot(RpcReplyPort<AuditSnapshotResult<V>>),
    Shutdown(RpcReplyPort<Result<(), FrameworkError>>),
}

#[ractor::async_trait]
impl<V> Actor for AuditActor<V>
where
    V: ViableSystem,
{
    type Msg = AuditActorMsg<V>;
    type State = AuditActorState<V>;
    type Arguments = AuditActorArgs<V>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(AuditActorState {
            roles: args.roles,
            context: args.context,
            responses: Vec::new(),
            last_audit_at: None,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            AuditActorMsg::Audit(request, evidence, reply) => {
                let result = perform_audit(state, *request, evidence).await;
                let _ = reply.send(result);
            }
            AuditActorMsg::Snapshot(reply) => {
                let _ = reply.send(Ok((state.responses.clone(), state.last_audit_at)));
            }
            AuditActorMsg::Shutdown(reply) => {
                let _ = reply.send(Ok(()));
            }
        }

        Ok(())
    }
}

async fn perform_audit<V>(
    state: &mut AuditActorState<V>,
    mut request: System3AuditRequest<V>,
    evidence: Vec<AuditEvidence<V>>,
) -> Result<AuditResponse<V>, FrameworkError>
where
    V: ViableSystem,
{
    if !request.authorization.approved {
        return Err(FrameworkError::InvalidProtocol {
            reason: format!("audit request is not authorized: {}", request.audit_id),
        });
    }

    set_system3_star_metadata(&mut request.metadata, &state.context);
    ensure_authority(
        &mut request.authorization.authority,
        &state.context,
        AuthorityScope::Audit,
    );

    let mut response = state
        .roles
        .auditor()
        .audit(&state.context, &request, evidence)
        .await?;
    set_system3_star_metadata(&mut response.metadata, &state.context);
    for finding in &mut response.findings {
        set_system3_star_metadata(&mut finding.metadata, &state.context);
    }
    for remediation in &mut response.remediations {
        set_system3_star_metadata(&mut remediation.metadata, &state.context);
    }

    for finding in &response.findings {
        record_audit_report(
            state,
            System3Report::AuditFinding(Box::new(finding.clone())),
        )
        .await;
    }
    for remediation in &response.remediations {
        record_audit_report(
            state,
            System3Report::Remediation(Box::new(remediation.clone())),
        )
        .await;
    }
    record_audit_report(
        state,
        System3Report::AuditResponse(Box::new(response.clone())),
    )
    .await;
    emit_audit_event(
        state,
        System3Event::AuditCompleted {
            audit_id: response.audit_id.clone(),
            finding_count: response.findings.len(),
            remediation_count: response.remediations.len(),
        },
    )
    .await;

    state.last_audit_at = Some(Utc::now());
    state.responses.push(response.clone());
    Ok(response)
}

fn set_system3_metadata<V>(metadata: &mut ProtocolMetadata, context: &RoleContext<V>)
where
    V: ViableSystem,
{
    metadata.source = Some(VsmAddress::new(
        context.runtime_id().clone(),
        context.recursion_path().clone(),
        SubsystemRole::System3,
    ));
}

fn set_system3_star_metadata<V>(metadata: &mut ProtocolMetadata, context: &RoleContext<V>)
where
    V: ViableSystem,
{
    metadata.source = Some(VsmAddress::new(
        context.runtime_id().clone(),
        context.recursion_path().clone(),
        SubsystemRole::System3Star,
    ));
}

fn ensure_authority<V>(
    authority: &mut ControlAuthority,
    context: &RoleContext<V>,
    scope: AuthorityScope,
) where
    V: ViableSystem,
{
    if authority.issued_by.is_none() {
        authority.issued_by = Some(VsmAddress::new(
            context.runtime_id().clone(),
            context.recursion_path().clone(),
            context.role().clone(),
        ));
    }
    authority.scope = scope;
}

async fn emit_event<V>(state: &ControlActorState<V>, event: System3Event<V>)
where
    V: ViableSystem,
{
    let _ = state
        .context
        .emit_event(RuntimeEvent::System3(Box::new(event)))
        .await;
}

async fn record_report<V>(state: &ControlActorState<V>, report: System3Report<V>)
where
    V: ViableSystem,
{
    let _ = state
        .context
        .record_report(RuntimeReport::System3(Box::new(report)))
        .await;
}

async fn emit_audit_event<V>(state: &AuditActorState<V>, event: System3Event<V>)
where
    V: ViableSystem,
{
    let _ = state
        .context
        .emit_event(RuntimeEvent::System3(Box::new(event)))
        .await;
}

async fn record_audit_report<V>(state: &AuditActorState<V>, report: System3Report<V>)
where
    V: ViableSystem,
{
    let _ = state
        .context
        .record_report(RuntimeReport::System3(Box::new(report)))
        .await;
}

fn system3_actor_name(config: &RuntimeConfig, actor: &str) -> String {
    let path = if config.recursion_path.is_root() {
        "root".to_string()
    } else {
        config.recursion_path.segments().join("/")
    };

    format!("{}:{path}:System3:{actor}", config.runtime_id)
}
