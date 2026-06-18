//! Role contexts exposed to application-owned behavior.

use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::cancellation::CancellationToken;
use crate::error::FrameworkError;
use crate::protocol::{
    ProtocolMetadata, RecursionPath, RuntimeEvent, RuntimeId, RuntimeReport, SubsystemRole,
};

use super::{
    Clock, EventSink, NoopEventSink, NoopReportSink, NoopStateStore, ReportSink, StateStore,
    SystemClock, ViableSystem,
};

/// Shared context passed to application role implementations.
///
/// The context exposes framework-owned identity, correlation, deadline,
/// cancellation, time, events, reports, and explicitly allowed state storage.
/// It intentionally does not expose actor references, global names, channel
/// publishing, supervisor controls, or mutable state owned by another role.
pub struct RoleContext<V>
where
    V: ViableSystem,
{
    runtime_id: RuntimeId,
    recursion_path: RecursionPath,
    role: SubsystemRole,
    metadata: ProtocolMetadata,
    cancellation: CancellationToken,
    clock: Arc<dyn Clock>,
    event_sink: Arc<dyn EventSink<V>>,
    report_sink: Arc<dyn ReportSink<V>>,
    state_store: Arc<dyn StateStore<V>>,
}

impl<V> Clone for RoleContext<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            runtime_id: self.runtime_id.clone(),
            recursion_path: self.recursion_path.clone(),
            role: self.role.clone(),
            metadata: self.metadata.clone(),
            cancellation: self.cancellation.clone(),
            clock: Arc::clone(&self.clock),
            event_sink: Arc::clone(&self.event_sink),
            report_sink: Arc::clone(&self.report_sink),
            state_store: Arc::clone(&self.state_store),
        }
    }
}

impl<V> RoleContext<V>
where
    V: ViableSystem,
{
    /// Creates a role context with no-op ports and a system clock.
    pub fn new(runtime_id: RuntimeId, recursion_path: RecursionPath, role: SubsystemRole) -> Self {
        Self {
            runtime_id,
            recursion_path,
            role,
            metadata: ProtocolMetadata::new(),
            cancellation: CancellationToken::new(),
            clock: Arc::new(SystemClock),
            event_sink: Arc::new(NoopEventSink::<V>::new()),
            report_sink: Arc::new(NoopReportSink::<V>::new()),
            state_store: Arc::new(NoopStateStore::<V>::new()),
        }
    }

    /// Replaces the framework metadata carried by this context.
    pub fn with_metadata(mut self, metadata: ProtocolMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Replaces the cooperative cancellation token.
    pub fn with_cancellation(mut self, cancellation: CancellationToken) -> Self {
        self.cancellation = cancellation;
        self
    }

    /// Replaces the clock port.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Replaces the observer event sink.
    pub fn with_event_sink(mut self, event_sink: Arc<dyn EventSink<V>>) -> Self {
        self.event_sink = event_sink;
        self
    }

    /// Replaces the report sink.
    pub fn with_report_sink(mut self, report_sink: Arc<dyn ReportSink<V>>) -> Self {
        self.report_sink = report_sink;
        self
    }

    /// Replaces the state store available to this role.
    pub fn with_state_store(mut self, state_store: Arc<dyn StateStore<V>>) -> Self {
        self.state_store = state_store;
        self
    }

    /// Returns the runtime instance identity.
    pub fn runtime_id(&self) -> &RuntimeId {
        &self.runtime_id
    }

    /// Returns the recursion path for this role invocation.
    pub fn recursion_path(&self) -> &RecursionPath {
        &self.recursion_path
    }

    /// Returns the subsystem role identity.
    pub fn role(&self) -> &SubsystemRole {
        &self.role
    }

    /// Returns the framework metadata.
    pub fn metadata(&self) -> &ProtocolMetadata {
        &self.metadata
    }

    /// Returns the deadline inherited by this role invocation, when present.
    pub fn deadline(&self) -> Option<DateTime<Utc>> {
        self.metadata.deadline
    }

    /// Returns the cooperative cancellation token.
    pub fn cancellation(&self) -> &CancellationToken {
        &self.cancellation
    }

    /// Returns the current time from the context clock.
    pub fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Returns the state store explicitly allowed for this role.
    pub fn state_store(&self) -> &(dyn StateStore<V> + '_) {
        self.state_store.as_ref()
    }

    /// Records an observer event through the configured sink.
    pub async fn emit_event(&self, event: RuntimeEvent<V>) -> Result<(), FrameworkError> {
        self.event_sink.record_event(event).await
    }

    /// Records a cross-system report through the configured sink.
    pub async fn record_report(&self, report: RuntimeReport<V>) -> Result<(), FrameworkError> {
        self.report_sink.record_report(report).await
    }
}

/// Role context for one operational unit.
pub struct UnitRoleContext<V>
where
    V: ViableSystem,
{
    base: RoleContext<V>,
    unit_id: V::UnitId,
}

impl<V> Clone for UnitRoleContext<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            unit_id: self.unit_id.clone(),
        }
    }
}

impl<V> UnitRoleContext<V>
where
    V: ViableSystem,
{
    /// Creates a context for one operational unit.
    pub fn new(base: RoleContext<V>, unit_id: V::UnitId) -> Self {
        Self { base, unit_id }
    }

    /// Returns the shared role context.
    pub fn base(&self) -> &RoleContext<V> {
        &self.base
    }

    /// Returns the unit identity for this context.
    pub fn unit_id(&self) -> &V::UnitId {
        &self.unit_id
    }

    /// Returns the runtime instance identity.
    pub fn runtime_id(&self) -> &RuntimeId {
        self.base.runtime_id()
    }

    /// Returns the recursion path for this unit.
    pub fn recursion_path(&self) -> &RecursionPath {
        self.base.recursion_path()
    }

    /// Returns the framework metadata.
    pub fn metadata(&self) -> &ProtocolMetadata {
        self.base.metadata()
    }

    /// Returns the deadline inherited by this unit invocation, when present.
    pub fn deadline(&self) -> Option<DateTime<Utc>> {
        self.base.deadline()
    }

    /// Returns the cooperative cancellation token.
    pub fn cancellation(&self) -> &CancellationToken {
        self.base.cancellation()
    }

    /// Returns the current time from the context clock.
    pub fn now(&self) -> DateTime<Utc> {
        self.base.now()
    }
}
