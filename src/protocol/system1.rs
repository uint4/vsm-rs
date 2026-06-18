//! Typed System 1 protocol records.

use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::error::{FrameworkError, WorkError};
use crate::roles::ViableSystem;

use super::envelope::{Priority, ProtocolMetadata};
use super::snapshot::SnapshotVersion;

/// Options supplied with a work request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkOptions {
    pub deadline: Option<DateTime<Utc>>,
    pub priority: Priority,
}

impl Default for WorkOptions {
    fn default() -> Self {
        Self {
            deadline: None,
            priority: Priority::Normal,
        }
    }
}

/// Typed work request owned by the application type family.
#[derive(Debug)]
pub struct WorkRequest<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub work: V::Work,
    pub options: WorkOptions,
}

impl<V> Clone for WorkRequest<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            work: self.work.clone(),
            options: self.options.clone(),
        }
    }
}

impl<V> WorkRequest<V>
where
    V: ViableSystem,
{
    /// Creates a work request with default framework metadata and options.
    pub fn new(work: V::Work) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            work,
            options: WorkOptions::default(),
        }
    }

    /// Replaces the metadata.
    pub fn with_metadata(mut self, metadata: ProtocolMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Replaces the work options.
    pub fn with_options(mut self, options: WorkOptions) -> Self {
        self.options = options;
        self
    }
}

/// Result returned by an operational unit.
pub type WorkResult<V> =
    Result<<V as ViableSystem>::Outcome, WorkError<<V as ViableSystem>::AppError>>;

/// Typed work response plus framework metadata.
pub struct WorkResponse<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub result: WorkResult<V>,
}

/// Static description of one application capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDescription<C> {
    pub capability: C,
    pub description: Option<String>,
}

impl<C> CapabilityDescription<C> {
    /// Creates a capability description without optional text.
    pub fn new(capability: C) -> Self {
        Self {
            capability,
            description: None,
        }
    }

    /// Adds human-readable descriptive text.
    pub fn describe(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Runtime capacity and load reported by a unit.
#[derive(Debug, Clone, PartialEq)]
pub struct CapacitySnapshot {
    pub observed_at: DateTime<Utc>,
    pub in_flight: usize,
    pub max_in_flight: Option<usize>,
    pub load: f64,
    pub accepting_work: bool,
}

impl CapacitySnapshot {
    /// Creates a capacity snapshot with load clamped to the inclusive 0-1 range.
    pub fn new(in_flight: usize, max_in_flight: Option<usize>, load: f64) -> Self {
        Self {
            observed_at: Utc::now(),
            in_flight,
            max_in_flight,
            load: load.clamp(0.0, 1.0),
            accepting_work: max_in_flight.is_none_or(|max| in_flight < max),
        }
    }
}

/// Static descriptor used to register or restore an operational unit.
#[derive(Debug, PartialEq, Eq)]
pub struct UnitDescriptor<V>
where
    V: ViableSystem,
{
    pub unit_id: V::UnitId,
    pub capabilities: Vec<CapabilityDescription<V::Capability>>,
    pub labels: BTreeMap<String, String>,
}

impl<V> Clone for UnitDescriptor<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            unit_id: self.unit_id.clone(),
            capabilities: self.capabilities.clone(),
            labels: self.labels.clone(),
        }
    }
}

impl<V> UnitDescriptor<V>
where
    V: ViableSystem,
{
    /// Creates a descriptor with no labels.
    pub fn new(unit_id: V::UnitId, capabilities: impl IntoIterator<Item = V::Capability>) -> Self {
        Self {
            unit_id,
            capabilities: capabilities
                .into_iter()
                .map(CapabilityDescription::new)
                .collect(),
            labels: BTreeMap::new(),
        }
    }
}

/// Framework-level unit command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitCommandKind {
    Drain,
    Resume,
    Stop,
    CaptureSnapshot,
    Custom(String),
}

/// Command sent to one operational unit.
#[derive(Debug, Clone)]
pub struct UnitCommand<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub unit_id: V::UnitId,
    pub kind: UnitCommandKind,
}

/// Acknowledgement for commands and control messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acknowledgement {
    pub metadata: ProtocolMetadata,
    pub accepted: bool,
    pub reason: Option<String>,
}

impl Acknowledgement {
    /// Creates an accepted acknowledgement.
    pub fn accepted(metadata: ProtocolMetadata) -> Self {
        Self {
            metadata,
            accepted: true,
            reason: None,
        }
    }

    /// Creates a rejected acknowledgement.
    pub fn rejected(metadata: ProtocolMetadata, reason: impl Into<String>) -> Self {
        Self {
            metadata,
            accepted: false,
            reason: Some(reason.into()),
        }
    }
}

/// Classification of a completed work attempt from the framework perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkDisposition {
    Completed,
    ApplicationRejected,
    ApplicationFailed,
    TimedOut,
    Cancelled,
    Backpressured,
    FrameworkFailed,
}

/// Generic performance observation emitted by System 1.
#[derive(Debug)]
pub struct PerformanceObservation<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub unit_id: V::UnitId,
    pub disposition: WorkDisposition,
    pub elapsed: Option<Duration>,
}

impl<V> Clone for PerformanceObservation<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            unit_id: self.unit_id.clone(),
            disposition: self.disposition,
            elapsed: self.elapsed,
        }
    }
}

/// Request emitted when no registered unit can accept required capabilities.
#[derive(Debug, Clone)]
pub struct ResourceShortageRequest<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub required_capabilities: Vec<V::Capability>,
    pub work_label: Option<String>,
    pub reason: String,
}

/// Scope of an operational audit request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditScope<U> {
    AllUnits,
    Units(Vec<U>),
    Custom(String),
}

/// Typed audit request for System 1.
#[derive(Debug)]
pub struct AuditRequest<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub scope: AuditScope<V::UnitId>,
}

impl<V> Clone for AuditRequest<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            scope: self.scope.clone(),
        }
    }
}

/// Evidence returned by an operational unit for an audit.
pub struct AuditEvidence<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub unit_id: V::UnitId,
    pub capabilities: Vec<CapabilityDescription<V::Capability>>,
    pub capacity: CapacitySnapshot,
    pub snapshot_version: Option<SnapshotVersion>,
    pub snapshot: Option<V::UnitSnapshot>,
}

/// Coordination view shared by a unit without exposing raw mutable state.
#[derive(Debug)]
pub struct CoordinationView<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub unit_id: V::UnitId,
    pub capabilities: Vec<CapabilityDescription<V::Capability>>,
    pub capacity: CapacitySnapshot,
    pub snapshot_version: Option<SnapshotVersion>,
}

impl<V> Clone for CoordinationView<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            unit_id: self.unit_id.clone(),
            capabilities: self.capabilities.clone(),
            capacity: self.capacity.clone(),
            snapshot_version: self.snapshot_version,
        }
    }
}

impl<E> From<&WorkError<E>> for WorkDisposition
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(error: &WorkError<E>) -> Self {
        match error {
            WorkError::Application(crate::error::ApplicationFailure::Rejected(_)) => {
                Self::ApplicationRejected
            }
            WorkError::Application(crate::error::ApplicationFailure::Failed(_)) => {
                Self::ApplicationFailed
            }
            WorkError::Framework(FrameworkError::Timeout { .. }) => Self::TimedOut,
            WorkError::Framework(FrameworkError::Cancelled) => Self::Cancelled,
            WorkError::Framework(FrameworkError::Backpressured { .. }) => Self::Backpressured,
            WorkError::Framework(_) => Self::FrameworkFailed,
        }
    }
}
