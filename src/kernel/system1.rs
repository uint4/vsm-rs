//! Private typed System 1 runtime adapters.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use ractor::{call_t, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

use crate::config::RuntimeConfig;
use crate::error::{FrameworkError, WorkError};
use crate::protocol::events::{RuntimeEvent, RuntimeReport, System1Event, System1Report};
use crate::protocol::snapshot::SnapshotRecord;
use crate::protocol::system1::{
    Acknowledgement, CapacitySnapshot, CoordinationView, PerformanceObservation,
    ResourceShortageRequest, UnitDescriptor, WorkDisposition, WorkOptions, WorkRequest,
    WorkResponse, WorkResult,
};
use crate::protocol::system2::{CoordinationAcknowledgement, CoordinationIntervention};
use crate::protocol::SubsystemRole;
use crate::roles::{BoxOperationalUnit, RoleContext, UnitCandidate, UnitRoleContext, ViableSystem};
use crate::runtime::{
    RegisteredUnit, RuntimePorts, System1RuntimeRoles, UnitAdmissionLimits, UnitRegistration,
    UnitSnapshotConfig,
};

const ACTOR_CALL_TIMEOUT_MS: u64 = 1_000;

pub(crate) struct System1Runtime<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    roles: System1RuntimeRoles<V>,
    ports: RuntimePorts<V>,
    units: Mutex<HashMap<V::UnitId, UnitEntry<V>>>,
    shutdown: AtomicBool,
}

impl<V> System1Runtime<V>
where
    V: ViableSystem,
{
    pub(crate) async fn start(
        config: RuntimeConfig,
        roles: System1RuntimeRoles<V>,
        ports: RuntimePorts<V>,
    ) -> Result<Arc<Self>, FrameworkError> {
        Ok(Arc::new(Self {
            config,
            roles,
            ports,
            units: Mutex::new(HashMap::new()),
            shutdown: AtomicBool::new(false),
        }))
    }

    pub(crate) async fn register_unit(
        &self,
        registration: UnitRegistration<V>,
    ) -> Result<UnitDescriptor<V>, FrameworkError> {
        self.ensure_running()?;
        self.validate_registration_capacity(&registration.descriptor)?;

        let descriptor = registration.descriptor.clone();
        let context = self.role_context();
        let unit = registration
            .factory
            .create_unit(&context, &descriptor)
            .await?;
        let actor_name = self.unit_actor_name(&descriptor);
        let unit_context = UnitRoleContext::new(context, descriptor.unit_id.clone());
        let actor_args = UnitActorArgs {
            unit,
            descriptor: descriptor.clone(),
            context: unit_context,
            admission: registration.admission,
            snapshot: registration.snapshot.clone(),
        };

        let (actor, _join) =
            Actor::spawn(Some(actor_name.clone()), UnitActor::<V>::new(), actor_args)
                .await
                .map_err(|err| FrameworkError::Runtime {
                    reason: format!("failed to spawn typed System 1 unit actor: {err}"),
                })?;

        {
            let mut units = self.units.lock().map_err(poisoned_units)?;
            if units.contains_key(&descriptor.unit_id) {
                actor.stop(Some("duplicate registration".to_string()));
                return Err(FrameworkError::InvalidProtocol {
                    reason: format!("unit already registered: {:?}", descriptor.unit_id),
                });
            }

            units.insert(
                descriptor.unit_id.clone(),
                UnitEntry {
                    descriptor: descriptor.clone(),
                    actor,
                    actor_name,
                    admission: registration.admission,
                    snapshot: registration.snapshot,
                    in_flight: 0,
                    draining: false,
                },
            );
        }

        self.emit_event(RuntimeEvent::System1(Box::new(
            System1Event::UnitRegistered(descriptor.clone()),
        )))
        .await;

        Ok(descriptor)
    }

    pub(crate) fn list_units(&self) -> Result<Vec<RegisteredUnit<V>>, FrameworkError> {
        let units = self.units.lock().map_err(poisoned_units)?;
        Ok(units
            .values()
            .map(|entry| RegisteredUnit {
                descriptor: entry.descriptor.clone(),
                in_flight: entry.in_flight,
                admission: entry.admission,
                draining: entry.draining,
            })
            .collect())
    }

    pub(crate) async fn process(&self, request: WorkRequest<V>) -> WorkResult<V> {
        if let Err(error) = self.ensure_running() {
            return Err(error.into());
        }

        let request = self.with_default_deadline(request);
        let context = self.role_context();

        self.roles
            .work_model()
            .validate_work(&context, request.clone())
            .await?;

        let required_capabilities = self
            .roles
            .work_model()
            .required_capabilities(&context, request.clone())
            .await?;

        let eligible = match self
            .eligible_units(&context, &request, &required_capabilities)
            .await
        {
            Ok(eligible) => eligible,
            Err(error) => return Err(error.into()),
        };

        if eligible.static_eligible_count == 0 {
            self.emit_resource_shortage(&request, required_capabilities)
                .await;
            return Err(FrameworkError::Unavailable {
                target: "system1.unit".to_string(),
            }
            .into());
        }

        if eligible.accepting_candidates.is_empty() {
            return Err(FrameworkError::Backpressured {
                reason: "all eligible System 1 units are at admission capacity".to_string(),
            }
            .into());
        }

        let selected_unit_id = match self
            .roles
            .unit_selection_policy()
            .select_unit(&context, request.clone(), &eligible.accepting_candidates)
            .await
        {
            Ok(Some(unit_id)) => unit_id,
            Ok(None) => {
                return Err(FrameworkError::Unavailable {
                    target: "system1.unit".to_string(),
                }
                .into())
            }
            Err(error) => return Err(error.into()),
        };

        let Some(entry) = self.reserve_unit(&selected_unit_id) else {
            return Err(FrameworkError::Backpressured {
                reason: format!("selected unit is no longer accepting work: {selected_unit_id:?}"),
            }
            .into());
        };

        let result = self.dispatch_to_unit(&entry, request.clone()).await;
        self.release_unit(&selected_unit_id);
        self.record_work_observation(&context, &request, &selected_unit_id, &result)
            .await;

        result
    }

    pub(crate) async fn drain_unit(
        &self,
        unit_id: &V::UnitId,
    ) -> Result<Acknowledgement, FrameworkError> {
        let entry = self.unit_entry(unit_id)?;
        let acknowledgement = call_t!(entry.actor, UnitActorMsg::Drain, ACTOR_CALL_TIMEOUT_MS)
            .map_err(|err| FrameworkError::Runtime {
            reason: format!("failed to drain typed System 1 unit: {err}"),
        })??;

        let mut units = self.units.lock().map_err(poisoned_units)?;
        if let Some(entry) = units.get_mut(unit_id) {
            entry.draining = true;
        }

        Ok(acknowledgement)
    }

    pub(crate) async fn unregister_unit(
        &self,
        unit_id: &V::UnitId,
    ) -> Result<UnitDescriptor<V>, FrameworkError> {
        let entry = {
            let mut units = self.units.lock().map_err(poisoned_units)?;
            units
                .remove(unit_id)
                .ok_or_else(|| FrameworkError::Unavailable {
                    target: format!("system1.unit {unit_id:?}"),
                })?
        };

        if entry.snapshot.save_on_unregister {
            call_t!(
                entry.actor.clone(),
                UnitActorMsg::CaptureSnapshot,
                ACTOR_CALL_TIMEOUT_MS
            )
            .map_err(|err| FrameworkError::Runtime {
                reason: format!("failed to capture typed System 1 unit snapshot: {err}"),
            })??;
        }

        entry
            .actor
            .stop(Some("typed System 1 unit unregistered".to_string()));

        self.emit_event(RuntimeEvent::System1(Box::new(
            System1Event::UnitUnregistered {
                unit_id: unit_id.clone(),
            },
        )))
        .await;

        Ok(entry.descriptor)
    }

    pub(crate) async fn coordination_views(
        &self,
    ) -> Result<Vec<CoordinationView<V>>, FrameworkError> {
        self.ensure_running()?;
        let entries = self.unit_entries()?;
        let mut views = Vec::with_capacity(entries.len());

        for entry in entries {
            let view = call_t!(
                entry.actor,
                UnitActorMsg::CoordinationView,
                ACTOR_CALL_TIMEOUT_MS
            )
            .map_err(|err| FrameworkError::Runtime {
                reason: format!("failed to query typed System 1 coordination view: {err}"),
            })??;
            views.push(view);
        }

        Ok(views)
    }

    pub(crate) async fn apply_coordination_intervention(
        &self,
        intervention: CoordinationIntervention<V>,
    ) -> Vec<CoordinationAcknowledgement<V>> {
        let mut acknowledgements = Vec::with_capacity(intervention.target_units.len());

        for unit_id in intervention.target_units.clone() {
            let acknowledgement = match self.unit_entry(&unit_id) {
                Ok(entry) => call_t!(
                    entry.actor,
                    UnitActorMsg::CoordinationIntervention,
                    ACTOR_CALL_TIMEOUT_MS,
                    Box::new(intervention.clone())
                )
                .map_err(|err| FrameworkError::Runtime {
                    reason: format!("failed to deliver typed System 2 intervention: {err}"),
                })
                .and_then(|result| result)
                .unwrap_or_else(|error| {
                    CoordinationAcknowledgement::failed(
                        &intervention,
                        unit_id.clone(),
                        error.to_string(),
                    )
                }),
                Err(error) => CoordinationAcknowledgement::failed(
                    &intervention,
                    unit_id.clone(),
                    error.to_string(),
                ),
            };
            acknowledgements.push(acknowledgement);
        }

        acknowledgements
    }

    pub(crate) async fn shutdown(&self) -> Result<(), FrameworkError> {
        if self.shutdown.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        let unit_ids = {
            let units = self.units.lock().map_err(poisoned_units)?;
            units.keys().cloned().collect::<Vec<_>>()
        };

        for unit_id in unit_ids {
            let _ = self.unregister_unit(&unit_id).await;
        }

        Ok(())
    }

    fn ensure_running(&self) -> Result<(), FrameworkError> {
        if self.shutdown.load(Ordering::SeqCst) {
            Err(FrameworkError::Shutdown)
        } else {
            Ok(())
        }
    }

    fn validate_registration_capacity(
        &self,
        descriptor: &UnitDescriptor<V>,
    ) -> Result<(), FrameworkError> {
        let units = self.units.lock().map_err(poisoned_units)?;

        if units.contains_key(&descriptor.unit_id) {
            return Err(FrameworkError::InvalidProtocol {
                reason: format!("unit already registered: {:?}", descriptor.unit_id),
            });
        }

        if self
            .config
            .max_registered_units
            .is_some_and(|max_units| units.len() >= max_units)
        {
            return Err(FrameworkError::Backpressured {
                reason: "maximum registered System 1 units reached".to_string(),
            });
        }

        Ok(())
    }

    fn role_context(&self) -> RoleContext<V> {
        self.ports.role_context(
            self.config.runtime_id.clone(),
            self.config.recursion_path.clone(),
            SubsystemRole::System1,
        )
    }

    fn unit_actor_name(&self, descriptor: &UnitDescriptor<V>) -> String {
        let path = if self.config.recursion_path.is_root() {
            "root".to_string()
        } else {
            self.config.recursion_path.segments().join("/")
        };

        format!(
            "{}:{path}:System1:unit:{:?}",
            self.config.runtime_id, descriptor.unit_id
        )
    }

    async fn eligible_units(
        &self,
        context: &RoleContext<V>,
        request: &WorkRequest<V>,
        required_capabilities: &[V::Capability],
    ) -> Result<EligibleUnits<V>, FrameworkError> {
        let entries = self.unit_entries()?;
        let mut static_eligible_count = 0;
        let mut accepting_candidates = Vec::new();

        for entry in entries {
            let mut capacity = call_t!(
                entry.actor.clone(),
                UnitActorMsg::Capacity,
                ACTOR_CALL_TIMEOUT_MS
            )
            .map_err(|err| FrameworkError::Runtime {
                reason: format!("failed to query typed System 1 unit capacity: {err}"),
            })??;

            capacity.in_flight = entry.in_flight;
            capacity.max_in_flight = entry.admission.max_in_flight.or(capacity.max_in_flight);
            let admission_accepting = !entry.draining
                && capacity
                    .max_in_flight
                    .is_none_or(|max| entry.in_flight < max);
            capacity.accepting_work = capacity.accepting_work && admission_accepting;

            let candidate = UnitCandidate::new(entry.descriptor.clone(), capacity);
            if !candidate.advertises_all(required_capabilities) {
                continue;
            }

            static_eligible_count += 1;
            if candidate.capacity.accepting_work {
                accepting_candidates.push(candidate);
            }
        }

        let _ = (context, request);
        Ok(EligibleUnits {
            static_eligible_count,
            accepting_candidates,
        })
    }

    fn unit_entries(&self) -> Result<Vec<UnitEntry<V>>, FrameworkError> {
        let units = self.units.lock().map_err(poisoned_units)?;
        Ok(units.values().cloned().collect())
    }

    fn unit_entry(&self, unit_id: &V::UnitId) -> Result<UnitEntry<V>, FrameworkError> {
        let units = self.units.lock().map_err(poisoned_units)?;
        units
            .get(unit_id)
            .cloned()
            .ok_or_else(|| FrameworkError::Unavailable {
                target: format!("system1.unit {unit_id:?}"),
            })
    }

    fn reserve_unit(&self, unit_id: &V::UnitId) -> Option<UnitEntry<V>> {
        let mut units = self.units.lock().ok()?;
        let entry = units.get_mut(unit_id)?;

        let at_capacity = entry
            .admission
            .max_in_flight
            .is_some_and(|max| entry.in_flight >= max);

        if entry.draining || at_capacity {
            return None;
        }

        entry.in_flight += 1;
        Some(entry.clone())
    }

    fn release_unit(&self, unit_id: &V::UnitId) {
        let Ok(mut units) = self.units.lock() else {
            return;
        };

        if let Some(entry) = units.get_mut(unit_id) {
            entry.in_flight = entry.in_flight.saturating_sub(1);
        }
    }

    async fn dispatch_to_unit(
        &self,
        entry: &UnitEntry<V>,
        request: WorkRequest<V>,
    ) -> WorkResult<V> {
        let timeout_ms =
            call_timeout_ms(request.options.deadline, self.config.default_work_timeout);
        match call_t!(
            entry.actor.clone(),
            UnitActorMsg::HandleWork,
            timeout_ms,
            Box::new(request)
        ) {
            Ok(result) => result,
            Err(_err) => Err(FrameworkError::Timeout {
                operation: format!("typed System 1 unit work via {}", entry.actor_name),
            }
            .into()),
        }
    }

    async fn record_work_observation(
        &self,
        context: &RoleContext<V>,
        request: &WorkRequest<V>,
        unit_id: &V::UnitId,
        result: &WorkResult<V>,
    ) {
        let disposition = match result {
            Ok(outcome) => self
                .roles
                .work_model()
                .classify_outcome(context, request.clone(), outcome.clone())
                .await
                .unwrap_or(WorkDisposition::FrameworkFailed),
            Err(error) => self
                .roles
                .work_model()
                .classify_error(context, request.clone(), error)
                .await
                .unwrap_or(WorkDisposition::FrameworkFailed),
        };

        let observation = PerformanceObservation::<V> {
            metadata: request.metadata.clone(),
            unit_id: unit_id.clone(),
            disposition,
            elapsed: None,
        };

        self.record_report(RuntimeReport::System1(Box::new(
            System1Report::Performance(PerformanceObservation {
                metadata: observation.metadata.clone(),
                unit_id: observation.unit_id.clone(),
                disposition: observation.disposition,
                elapsed: observation.elapsed,
            }),
        )))
        .await;

        let response = match result {
            Ok(outcome) => Some(WorkResponse {
                metadata: request.metadata.clone(),
                result: Ok(outcome.clone()),
            }),
            Err(_) => None,
        };

        let measurements = match &response {
            Some(response) => self
                .roles
                .work_model()
                .measurements(context, request.clone(), response.clone_for_measurement())
                .await
                .unwrap_or_default(),
            None => Vec::new(),
        };

        let Ok(performance) = self
            .roles
            .performance_model()
            .assess_performance(context, &observation, &measurements)
            .await
        else {
            return;
        };

        let _ = self
            .roles
            .variety_model()
            .assess_variety(context, request.clone(), response)
            .await;

        let _ = self
            .roles
            .algedonic_policy()
            .classify_algedonic(context, &observation, &performance)
            .await;
    }

    async fn emit_resource_shortage(
        &self,
        request: &WorkRequest<V>,
        required_capabilities: Vec<V::Capability>,
    ) {
        let shortage = ResourceShortageRequest {
            metadata: request.metadata.clone(),
            required_capabilities,
            work_label: None,
            reason: "no registered System 1 unit advertises every required capability".to_string(),
        };

        self.emit_event(RuntimeEvent::System1(Box::new(
            System1Event::ResourceShortage(Box::new(shortage)),
        )))
        .await;
    }

    async fn emit_event(&self, event: RuntimeEvent<V>) {
        let _ = self.role_context().emit_event(event).await;
    }

    async fn record_report(&self, report: RuntimeReport<V>) {
        let _ = self.role_context().record_report(report).await;
    }

    fn with_default_deadline(&self, mut request: WorkRequest<V>) -> WorkRequest<V> {
        if request.options.deadline.is_none() {
            let deadline = Utc::now()
                + chrono::Duration::from_std(self.config.default_work_timeout)
                    .unwrap_or_else(|_| chrono::Duration::seconds(30));
            request.options = WorkOptions {
                deadline: Some(deadline),
                priority: request.options.priority,
            };
        }

        request
    }
}

struct EligibleUnits<V>
where
    V: ViableSystem,
{
    static_eligible_count: usize,
    accepting_candidates: Vec<UnitCandidate<V>>,
}

struct UnitEntry<V>
where
    V: ViableSystem,
{
    descriptor: UnitDescriptor<V>,
    actor: ActorRef<UnitActorMsg<V>>,
    actor_name: String,
    admission: UnitAdmissionLimits,
    snapshot: UnitSnapshotConfig,
    in_flight: usize,
    draining: bool,
}

impl<V> Clone for UnitEntry<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            descriptor: self.descriptor.clone(),
            actor: self.actor.clone(),
            actor_name: self.actor_name.clone(),
            admission: self.admission,
            snapshot: self.snapshot.clone(),
            in_flight: self.in_flight,
            draining: self.draining,
        }
    }
}

struct UnitActor<V>
where
    V: ViableSystem,
{
    _system: PhantomData<V>,
}

impl<V> UnitActor<V>
where
    V: ViableSystem,
{
    fn new() -> Self {
        Self {
            _system: PhantomData,
        }
    }
}

struct UnitActorArgs<V>
where
    V: ViableSystem,
{
    unit: BoxOperationalUnit<V>,
    descriptor: UnitDescriptor<V>,
    context: UnitRoleContext<V>,
    admission: UnitAdmissionLimits,
    snapshot: UnitSnapshotConfig,
}

struct UnitActorState<V>
where
    V: ViableSystem,
{
    unit: BoxOperationalUnit<V>,
    descriptor: UnitDescriptor<V>,
    context: UnitRoleContext<V>,
    admission: UnitAdmissionLimits,
    snapshot: UnitSnapshotConfig,
    in_flight: usize,
    draining: bool,
}

enum UnitActorMsg<V>
where
    V: ViableSystem,
{
    Capacity(RpcReplyPort<Result<CapacitySnapshot, FrameworkError>>),
    HandleWork(Box<WorkRequest<V>>, RpcReplyPort<WorkResult<V>>),
    CoordinationView(RpcReplyPort<Result<CoordinationView<V>, FrameworkError>>),
    CoordinationIntervention(
        Box<CoordinationIntervention<V>>,
        RpcReplyPort<Result<CoordinationAcknowledgement<V>, FrameworkError>>,
    ),
    Drain(RpcReplyPort<Result<Acknowledgement, FrameworkError>>),
    CaptureSnapshot(RpcReplyPort<Result<(), FrameworkError>>),
}

#[ractor::async_trait]
impl<V> Actor for UnitActor<V>
where
    V: ViableSystem,
{
    type Msg = UnitActorMsg<V>;
    type State = UnitActorState<V>;
    type Arguments = UnitActorArgs<V>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let mut unit = args.unit;

        if let Some(key) = args.snapshot.key.clone() {
            let maybe_snapshot = args
                .context
                .base()
                .state_store()
                .load_unit_snapshot(&key)
                .await
                .map_err(boxed_err)?;

            if let Some(record) = maybe_snapshot {
                if record.version != args.snapshot.version {
                    return Err(boxed_err(FrameworkError::SnapshotIncompatible {
                        key: key.stable_id(),
                        reason: format!(
                            "expected version {:?}, found {:?}",
                            args.snapshot.version, record.version
                        ),
                    }));
                }

                unit.restore(&args.context, record)
                    .await
                    .map_err(boxed_err)?;
            }
        }

        Ok(UnitActorState {
            unit,
            descriptor: args.descriptor,
            context: args.context,
            admission: args.admission,
            snapshot: args.snapshot,
            in_flight: 0,
            draining: false,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            UnitActorMsg::Capacity(reply) => {
                let result = unit_capacity(state).await;
                let _ = reply.send(result);
            }
            UnitActorMsg::HandleWork(request, reply) => {
                let result = unit_work(state, *request).await;
                let _ = reply.send(result);
            }
            UnitActorMsg::CoordinationView(reply) => {
                let result = state.unit.coordination_view(&state.context).await;
                let _ = reply.send(result);
            }
            UnitActorMsg::CoordinationIntervention(intervention, reply) => {
                let result = state
                    .unit
                    .handle_coordination_intervention(&state.context, *intervention)
                    .await;
                let _ = reply.send(result);
            }
            UnitActorMsg::Drain(reply) => {
                state.draining = true;
                let _ = reply.send(Ok(Acknowledgement::accepted(
                    state.context.metadata().clone(),
                )));
            }
            UnitActorMsg::CaptureSnapshot(reply) => {
                let result = capture_snapshot(state).await;
                let _ = reply.send(result);
            }
        }

        Ok(())
    }
}

async fn unit_capacity<V>(state: &mut UnitActorState<V>) -> Result<CapacitySnapshot, FrameworkError>
where
    V: ViableSystem,
{
    let mut capacity = state.unit.capacity(&state.context).await?;
    capacity.in_flight = state.in_flight;
    capacity.max_in_flight = state.admission.max_in_flight.or(capacity.max_in_flight);
    let admission_accepting = !state.draining
        && capacity
            .max_in_flight
            .is_none_or(|max| state.in_flight < max);
    capacity.accepting_work = capacity.accepting_work && admission_accepting;
    Ok(capacity)
}

async fn unit_work<V>(state: &mut UnitActorState<V>, request: WorkRequest<V>) -> WorkResult<V>
where
    V: ViableSystem,
{
    if state.draining {
        return Err(FrameworkError::Backpressured {
            reason: format!("unit is draining: {:?}", state.descriptor.unit_id),
        }
        .into());
    }

    let at_capacity = state
        .admission
        .max_in_flight
        .is_some_and(|max| state.in_flight >= max);
    if at_capacity {
        return Err(FrameworkError::Backpressured {
            reason: format!(
                "unit is at admission capacity: {:?}",
                state.descriptor.unit_id
            ),
        }
        .into());
    }

    let Some(timeout) = request_timeout(&state.context, &request) else {
        return Err(FrameworkError::Timeout {
            operation: "typed System 1 unit work".to_string(),
        }
        .into());
    };

    state.in_flight += 1;
    let result = match tokio::time::timeout(
        timeout,
        state.unit.handle_work(&state.context, request),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(FrameworkError::Timeout {
            operation: "typed System 1 unit work".to_string(),
        }
        .into()),
    };
    state.in_flight = state.in_flight.saturating_sub(1);

    result
}

async fn capture_snapshot<V>(state: &mut UnitActorState<V>) -> Result<(), FrameworkError>
where
    V: ViableSystem,
{
    let Some(key) = state.snapshot.key.clone() else {
        return Ok(());
    };

    let snapshot = state.unit.snapshot(&state.context).await?;
    let record = SnapshotRecord::new(key, state.snapshot.version, snapshot);
    state
        .context
        .base()
        .state_store()
        .save_unit_snapshot(record)
        .await
}

fn request_timeout<V>(context: &UnitRoleContext<V>, request: &WorkRequest<V>) -> Option<Duration>
where
    V: ViableSystem,
{
    let deadline = request.options.deadline?;
    (deadline - context.now()).to_std().ok()
}

fn timeout_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn call_timeout_ms(deadline: Option<chrono::DateTime<Utc>>, default_timeout: Duration) -> u64 {
    let timeout = deadline
        .and_then(|deadline| (deadline - Utc::now()).to_std().ok())
        .unwrap_or(default_timeout);
    timeout_ms(timeout.saturating_add(Duration::from_secs(1)))
}

fn poisoned_units<V>(
    _: std::sync::PoisonError<std::sync::MutexGuard<'_, HashMap<V::UnitId, UnitEntry<V>>>>,
) -> FrameworkError
where
    V: ViableSystem,
{
    FrameworkError::Runtime {
        reason: "typed System 1 unit registry mutex poisoned".to_string(),
    }
}

fn boxed_err<E>(err: E) -> ActorProcessingErr
where
    E: std::error::Error + Send + Sync + 'static,
{
    Box::new(err)
}

trait CloneForMeasurement<V>
where
    V: ViableSystem,
{
    fn clone_for_measurement(&self) -> WorkResponse<V>;
}

impl<V> CloneForMeasurement<V> for WorkResponse<V>
where
    V: ViableSystem,
{
    fn clone_for_measurement(&self) -> WorkResponse<V> {
        let result = match &self.result {
            Ok(outcome) => Ok(outcome.clone()),
            Err(error) => Err(WorkError::Framework(FrameworkError::Runtime {
                reason: format!("measurement cloning skipped failed result: {error}"),
            })),
        };

        WorkResponse {
            metadata: self.metadata.clone(),
            result,
        }
    }
}
