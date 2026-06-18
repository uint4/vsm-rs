//! Private typed System 2 runtime adapter.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::Utc;
use ractor::{call_t, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

use crate::config::RuntimeConfig;
use crate::error::FrameworkError;
use crate::protocol::events::{RuntimeEvent, RuntimeReport, System2Event, System2Report};
use crate::protocol::system1::CoordinationView;
use crate::protocol::system2::{
    CoordinationAckStatus, CoordinationAcknowledgement, CoordinationConflict, CoordinationCycle,
    CoordinationEscalation, CoordinationIntervention, CoordinationViewRecord,
    CoordinationViewVersion, System2Snapshot,
};
use crate::protocol::{ProtocolMetadata, SubsystemRole, VsmAddress};
use crate::roles::{RoleContext, ViableSystem};
use crate::runtime::{RuntimePorts, System2RuntimeRoles};

const ACTOR_CALL_TIMEOUT_MS: u64 = 1_000;

pub(crate) struct System2Runtime<V>
where
    V: ViableSystem,
{
    actor: ActorRef<CoordinationActorMsg<V>>,
    shutdown: AtomicBool,
}

impl<V> System2Runtime<V>
where
    V: ViableSystem,
{
    pub(crate) async fn start(
        config: RuntimeConfig,
        roles: System2RuntimeRoles<V>,
        ports: RuntimePorts<V>,
    ) -> Result<Arc<Self>, FrameworkError> {
        let context = ports.role_context(
            config.runtime_id.clone(),
            config.recursion_path.clone(),
            SubsystemRole::System2,
        );
        let actor_name = coordination_actor_name(&config);
        let actor_args = CoordinationActorArgs { roles, context };
        let (actor, _join) =
            Actor::spawn(Some(actor_name), CoordinationActor::<V>::new(), actor_args)
                .await
                .map_err(|err| FrameworkError::Runtime {
                    reason: format!("failed to spawn typed System 2 coordination actor: {err}"),
                })?;

        Ok(Arc::new(Self {
            actor,
            shutdown: AtomicBool::new(false),
        }))
    }

    pub(crate) async fn coordinate_views(
        &self,
        views: Vec<CoordinationView<V>>,
    ) -> Result<CoordinationCycle<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.actor,
            CoordinationActorMsg::CoordinateViews,
            ACTOR_CALL_TIMEOUT_MS,
            views
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to coordinate typed System 2 views: {err}"),
        })?
    }

    pub(crate) async fn record_acknowledgements(
        &self,
        acknowledgements: Vec<CoordinationAcknowledgement<V>>,
    ) -> Result<CoordinationCycle<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.actor,
            CoordinationActorMsg::RecordAcknowledgements,
            ACTOR_CALL_TIMEOUT_MS,
            acknowledgements
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to record typed System 2 acknowledgements: {err}"),
        })?
    }

    pub(crate) async fn snapshot(&self) -> Result<System2Snapshot<V>, FrameworkError> {
        self.ensure_running()?;
        call_t!(
            self.actor,
            CoordinationActorMsg::Snapshot,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to read typed System 2 snapshot: {err}"),
        })?
    }

    pub(crate) async fn shutdown(&self) -> Result<(), FrameworkError> {
        if self.shutdown.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        call_t!(
            self.actor,
            CoordinationActorMsg::Shutdown,
            ACTOR_CALL_TIMEOUT_MS
        )
        .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to shut down typed System 2 runtime: {err}"),
        })??;
        self.actor
            .stop(Some("typed System 2 runtime shutdown".to_string()));
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

struct CoordinationActor<V>
where
    V: ViableSystem,
{
    _system: PhantomData<V>,
}

impl<V> CoordinationActor<V>
where
    V: ViableSystem,
{
    fn new() -> Self {
        Self {
            _system: PhantomData,
        }
    }
}

struct CoordinationActorArgs<V>
where
    V: ViableSystem,
{
    roles: System2RuntimeRoles<V>,
    context: RoleContext<V>,
}

struct CoordinationActorState<V>
where
    V: ViableSystem,
{
    roles: System2RuntimeRoles<V>,
    context: RoleContext<V>,
    views: HashMap<V::UnitId, CoordinationViewRecord<V>>,
    conflicts: Vec<CoordinationConflict<V>>,
    pending_interventions: Vec<CoordinationIntervention<V>>,
    acknowledgements: Vec<CoordinationAcknowledgement<V>>,
    escalations: Vec<CoordinationEscalation<V>>,
    last_cycle_at: Option<chrono::DateTime<Utc>>,
}

enum CoordinationActorMsg<V>
where
    V: ViableSystem,
{
    CoordinateViews(
        Vec<CoordinationView<V>>,
        RpcReplyPort<Result<CoordinationCycle<V>, FrameworkError>>,
    ),
    RecordAcknowledgements(
        Vec<CoordinationAcknowledgement<V>>,
        RpcReplyPort<Result<CoordinationCycle<V>, FrameworkError>>,
    ),
    Snapshot(RpcReplyPort<Result<System2Snapshot<V>, FrameworkError>>),
    Shutdown(RpcReplyPort<Result<(), FrameworkError>>),
}

#[ractor::async_trait]
impl<V> Actor for CoordinationActor<V>
where
    V: ViableSystem,
{
    type Msg = CoordinationActorMsg<V>;
    type State = CoordinationActorState<V>;
    type Arguments = CoordinationActorArgs<V>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(CoordinationActorState {
            roles: args.roles,
            context: args.context,
            views: HashMap::new(),
            conflicts: Vec::new(),
            pending_interventions: Vec::new(),
            acknowledgements: Vec::new(),
            escalations: Vec::new(),
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
            CoordinationActorMsg::CoordinateViews(views, reply) => {
                let result = coordinate_views(state, views).await;
                let _ = reply.send(result);
            }
            CoordinationActorMsg::RecordAcknowledgements(acknowledgements, reply) => {
                let result = record_acknowledgements(state, acknowledgements).await;
                let _ = reply.send(result);
            }
            CoordinationActorMsg::Snapshot(reply) => {
                let _ = reply.send(Ok(snapshot(state)));
            }
            CoordinationActorMsg::Shutdown(reply) => {
                let _ = reply.send(Ok(()));
            }
        }

        Ok(())
    }
}

async fn coordinate_views<V>(
    state: &mut CoordinationActorState<V>,
    views: Vec<CoordinationView<V>>,
) -> Result<CoordinationCycle<V>, FrameworkError>
where
    V: ViableSystem,
{
    for view in views {
        record_view(state, view);
    }

    let views = current_views(state);
    let mut conflicts = state
        .roles
        .coordination_policy()
        .detect_conflicts(&state.context, &views)
        .await?;
    for conflict in &mut conflicts {
        set_system2_metadata(&mut conflict.metadata, &state.context);
    }

    let mut interventions = state
        .roles
        .coordination_policy()
        .plan_interventions(&state.context, &conflicts, &views)
        .await?;
    for intervention in &mut interventions {
        set_system2_metadata(&mut intervention.metadata, &state.context);
    }

    state.conflicts = conflicts.clone();
    state
        .pending_interventions
        .extend(interventions.iter().cloned());
    state.last_cycle_at = Some(Utc::now());

    for conflict in &conflicts {
        record_report(state, System2Report::Conflict(Box::new(conflict.clone()))).await;
    }
    for intervention in &interventions {
        record_report(
            state,
            System2Report::Intervention(Box::new(intervention.clone())),
        )
        .await;
    }
    emit_event(
        state,
        System2Event::CoordinationCycle {
            conflict_count: conflicts.len(),
            intervention_count: interventions.len(),
        },
    )
    .await;

    Ok(CoordinationCycle {
        metadata: state.context.metadata().clone(),
        views,
        conflicts,
        interventions,
        acknowledgements: Vec::new(),
        escalations: Vec::new(),
    })
}

async fn record_acknowledgements<V>(
    state: &mut CoordinationActorState<V>,
    acknowledgements: Vec<CoordinationAcknowledgement<V>>,
) -> Result<CoordinationCycle<V>, FrameworkError>
where
    V: ViableSystem,
{
    let mut escalations = Vec::new();
    for acknowledgement in &acknowledgements {
        record_report(
            state,
            System2Report::Acknowledgement(Box::new(acknowledgement.clone())),
        )
        .await;
        emit_event(
            state,
            System2Event::InterventionAcknowledged(Box::new(acknowledgement.clone())),
        )
        .await;

        if !acknowledgement.status.is_success() {
            if let Some(escalation) = escalation_for_acknowledgement(state, acknowledgement) {
                record_report(
                    state,
                    System2Report::Escalation(Box::new(escalation.clone())),
                )
                .await;
                emit_event(
                    state,
                    System2Event::ConflictEscalated(Box::new(escalation.clone())),
                )
                .await;
                escalations.push(escalation);
            }
        }
    }

    state
        .acknowledgements
        .extend(acknowledgements.iter().cloned());
    state.escalations.extend(escalations.iter().cloned());
    retain_unacknowledged_interventions(state);

    Ok(CoordinationCycle {
        metadata: state.context.metadata().clone(),
        views: current_views(state),
        conflicts: state.conflicts.clone(),
        interventions: Vec::new(),
        acknowledgements,
        escalations,
    })
}

fn record_view<V>(state: &mut CoordinationActorState<V>, view: CoordinationView<V>)
where
    V: ViableSystem,
{
    let version = state
        .views
        .get(&view.unit_id)
        .map(|record| record.version.next())
        .unwrap_or(CoordinationViewVersion::INITIAL);
    let received_at = state.context.now();
    state.views.insert(
        view.unit_id.clone(),
        CoordinationViewRecord {
            view,
            version,
            received_at,
        },
    );
}

fn current_views<V>(state: &CoordinationActorState<V>) -> Vec<CoordinationViewRecord<V>>
where
    V: ViableSystem,
{
    state.views.values().cloned().collect()
}

fn snapshot<V>(state: &CoordinationActorState<V>) -> System2Snapshot<V>
where
    V: ViableSystem,
{
    System2Snapshot {
        views: current_views(state),
        pending_interventions: state.pending_interventions.clone(),
        acknowledgements: state.acknowledgements.clone(),
        escalations: state.escalations.clone(),
        last_cycle_at: state.last_cycle_at,
    }
}

fn escalation_for_acknowledgement<V>(
    state: &CoordinationActorState<V>,
    acknowledgement: &CoordinationAcknowledgement<V>,
) -> Option<CoordinationEscalation<V>>
where
    V: ViableSystem,
{
    let intervention = state
        .pending_interventions
        .iter()
        .find(|intervention| intervention.intervention_id == acknowledgement.intervention_id)?;
    let conflict_id = intervention.conflict_id.as_ref()?;
    let conflict = state
        .conflicts
        .iter()
        .find(|conflict| &conflict.conflict_id == conflict_id)?
        .clone();
    let mut metadata = acknowledgement.metadata.child();
    metadata.source = Some(VsmAddress::new(
        state.context.runtime_id().clone(),
        state.context.recursion_path().clone(),
        SubsystemRole::System2,
    ));
    metadata.destination = Some(VsmAddress::new(
        state.context.runtime_id().clone(),
        state.context.recursion_path().clone(),
        SubsystemRole::System3,
    ));
    Some(CoordinationEscalation::new(
        metadata,
        conflict,
        acknowledgement.intervention_id.clone(),
        acknowledgement
            .reason
            .clone()
            .unwrap_or_else(|| status_reason(acknowledgement.status)),
    ))
}

fn retain_unacknowledged_interventions<V>(state: &mut CoordinationActorState<V>)
where
    V: ViableSystem,
{
    let acknowledgements = &state.acknowledgements;
    state.pending_interventions.retain(|intervention| {
        intervention.requires_ack && !intervention_is_satisfied(intervention, acknowledgements)
    });
}

fn intervention_is_satisfied<V>(
    intervention: &CoordinationIntervention<V>,
    acknowledgements: &[CoordinationAcknowledgement<V>],
) -> bool
where
    V: ViableSystem,
{
    intervention.target_units.iter().all(|target| {
        acknowledgements.iter().any(|acknowledgement| {
            acknowledgement.intervention_id == intervention.intervention_id
                && &acknowledgement.unit_id == target
                && acknowledgement.status.is_success()
        })
    })
}

fn set_system2_metadata<V>(metadata: &mut ProtocolMetadata, context: &RoleContext<V>)
where
    V: ViableSystem,
{
    metadata.source = Some(VsmAddress::new(
        context.runtime_id().clone(),
        context.recursion_path().clone(),
        SubsystemRole::System2,
    ));
}

async fn emit_event<V>(state: &CoordinationActorState<V>, event: System2Event<V>)
where
    V: ViableSystem,
{
    let _ = state
        .context
        .emit_event(RuntimeEvent::System2(Box::new(event)))
        .await;
}

async fn record_report<V>(state: &CoordinationActorState<V>, report: System2Report<V>)
where
    V: ViableSystem,
{
    let _ = state
        .context
        .record_report(RuntimeReport::System2(Box::new(report)))
        .await;
}

fn status_reason(status: CoordinationAckStatus) -> String {
    match status {
        CoordinationAckStatus::Accepted => "accepted".to_string(),
        CoordinationAckStatus::Rejected => "rejected".to_string(),
        CoordinationAckStatus::Applied => "applied".to_string(),
        CoordinationAckStatus::Failed => "failed".to_string(),
    }
}

fn coordination_actor_name(config: &RuntimeConfig) -> String {
    let path = if config.recursion_path.is_root() {
        "root".to_string()
    } else {
        config.recursion_path.segments().join("/")
    };

    format!("{}:{path}:System2:coordination", config.runtime_id)
}
