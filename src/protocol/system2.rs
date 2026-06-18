//! Typed System 2 coordination protocol records.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::roles::ViableSystem;

use super::envelope::ProtocolMetadata;
use super::system1::CoordinationView;

/// Monotonic version assigned when System 2 receives a unit coordination view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CoordinationViewVersion(u64);

impl CoordinationViewVersion {
    /// Initial version assigned to the first view received for a unit.
    pub const INITIAL: Self = Self(1);

    /// Returns the next view version, saturating at `u64::MAX`.
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// Returns the raw version number.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// System 2's stored view plus freshness metadata.
#[derive(Debug)]
pub struct CoordinationViewRecord<V>
where
    V: ViableSystem,
{
    pub view: CoordinationView<V>,
    pub version: CoordinationViewVersion,
    pub received_at: DateTime<Utc>,
}

impl<V> Clone for CoordinationViewRecord<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            view: self.view.clone(),
            version: self.version,
            received_at: self.received_at,
        }
    }
}

/// Severity assigned by coordination policy to a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinationSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// Framework-owned conflict category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinationConflictKind {
    Oscillation,
    Dependency,
    CapabilityOverlap,
    CapacityPressure,
    StaleView,
    Custom(String),
}

/// Conflict detected across one or more coordination views.
#[derive(Debug)]
pub struct CoordinationConflict<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub conflict_id: String,
    pub kind: CoordinationConflictKind,
    pub affected_units: Vec<V::UnitId>,
    pub severity: CoordinationSeverity,
    pub summary: String,
    pub detected_at: DateTime<Utc>,
}

impl<V> Clone for CoordinationConflict<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            conflict_id: self.conflict_id.clone(),
            kind: self.kind.clone(),
            affected_units: self.affected_units.clone(),
            severity: self.severity,
            summary: self.summary.clone(),
            detected_at: self.detected_at,
        }
    }
}

impl<V> CoordinationConflict<V>
where
    V: ViableSystem,
{
    /// Creates a conflict with generated identity and current timestamp.
    pub fn new(
        kind: CoordinationConflictKind,
        affected_units: impl IntoIterator<Item = V::UnitId>,
        severity: CoordinationSeverity,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            conflict_id: format!("conflict-{}", Uuid::new_v4()),
            kind,
            affected_units: affected_units.into_iter().collect(),
            severity,
            summary: summary.into(),
            detected_at: Utc::now(),
        }
    }

    /// Replaces framework metadata.
    pub fn with_metadata(mut self, metadata: ProtocolMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Framework-owned intervention category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinationInterventionKind {
    Recommendation,
    Constraint,
    Throttle,
    Drain,
    Resume,
    Custom(String),
}

/// Recommendation or constraint sent by System 2 to affected units.
#[derive(Debug)]
pub struct CoordinationIntervention<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub intervention_id: String,
    pub conflict_id: Option<String>,
    pub kind: CoordinationInterventionKind,
    pub target_units: Vec<V::UnitId>,
    pub summary: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub requires_ack: bool,
}

impl<V> Clone for CoordinationIntervention<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            intervention_id: self.intervention_id.clone(),
            conflict_id: self.conflict_id.clone(),
            kind: self.kind.clone(),
            target_units: self.target_units.clone(),
            summary: self.summary.clone(),
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            requires_ack: self.requires_ack,
        }
    }
}

impl<V> CoordinationIntervention<V>
where
    V: ViableSystem,
{
    /// Creates an intervention with generated identity and acknowledgement required.
    pub fn new(
        kind: CoordinationInterventionKind,
        target_units: impl IntoIterator<Item = V::UnitId>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            intervention_id: format!("intervention-{}", Uuid::new_v4()),
            conflict_id: None,
            kind,
            target_units: target_units.into_iter().collect(),
            summary: summary.into(),
            issued_at: Utc::now(),
            expires_at: None,
            requires_ack: true,
        }
    }

    /// Associates this intervention with a detected conflict.
    pub fn for_conflict(mut self, conflict_id: impl Into<String>) -> Self {
        self.conflict_id = Some(conflict_id.into());
        self
    }

    /// Replaces framework metadata.
    pub fn with_metadata(mut self, metadata: ProtocolMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Marks whether target units must acknowledge this intervention.
    pub fn with_required_ack(mut self, requires_ack: bool) -> Self {
        self.requires_ack = requires_ack;
        self
    }
}

/// Status returned by a unit after receiving a System 2 intervention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinationAckStatus {
    Accepted,
    Rejected,
    Applied,
    Failed,
}

impl CoordinationAckStatus {
    /// Returns true when the acknowledgement indicates the unit accepted or applied the intervention.
    pub fn is_success(self) -> bool {
        matches!(self, Self::Accepted | Self::Applied)
    }
}

/// Unit acknowledgement for an intervention.
#[derive(Debug)]
pub struct CoordinationAcknowledgement<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub intervention_id: String,
    pub unit_id: V::UnitId,
    pub status: CoordinationAckStatus,
    pub reason: Option<String>,
    pub observed_at: DateTime<Utc>,
}

impl<V> Clone for CoordinationAcknowledgement<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            intervention_id: self.intervention_id.clone(),
            unit_id: self.unit_id.clone(),
            status: self.status,
            reason: self.reason.clone(),
            observed_at: self.observed_at,
        }
    }
}

impl<V> CoordinationAcknowledgement<V>
where
    V: ViableSystem,
{
    /// Creates a successful acknowledgement for an intervention.
    pub fn accepted(intervention: &CoordinationIntervention<V>, unit_id: V::UnitId) -> Self {
        Self::new(intervention, unit_id, CoordinationAckStatus::Accepted, None)
    }

    /// Creates an applied acknowledgement for an intervention.
    pub fn applied(intervention: &CoordinationIntervention<V>, unit_id: V::UnitId) -> Self {
        Self::new(intervention, unit_id, CoordinationAckStatus::Applied, None)
    }

    /// Creates a rejected acknowledgement for an intervention.
    pub fn rejected(
        intervention: &CoordinationIntervention<V>,
        unit_id: V::UnitId,
        reason: impl Into<String>,
    ) -> Self {
        Self::new(
            intervention,
            unit_id,
            CoordinationAckStatus::Rejected,
            Some(reason.into()),
        )
    }

    /// Creates a failed acknowledgement for an intervention.
    pub fn failed(
        intervention: &CoordinationIntervention<V>,
        unit_id: V::UnitId,
        reason: impl Into<String>,
    ) -> Self {
        Self::new(
            intervention,
            unit_id,
            CoordinationAckStatus::Failed,
            Some(reason.into()),
        )
    }

    fn new(
        intervention: &CoordinationIntervention<V>,
        unit_id: V::UnitId,
        status: CoordinationAckStatus,
        reason: Option<String>,
    ) -> Self {
        Self {
            metadata: intervention.metadata.child(),
            intervention_id: intervention.intervention_id.clone(),
            unit_id,
            status,
            reason,
            observed_at: Utc::now(),
        }
    }
}

/// Escalation record for unresolved coordination conflicts.
#[derive(Debug)]
pub struct CoordinationEscalation<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub conflict: CoordinationConflict<V>,
    pub intervention_id: String,
    pub reason: String,
    pub escalated_at: DateTime<Utc>,
}

impl<V> Clone for CoordinationEscalation<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            conflict: self.conflict.clone(),
            intervention_id: self.intervention_id.clone(),
            reason: self.reason.clone(),
            escalated_at: self.escalated_at,
        }
    }
}

impl<V> CoordinationEscalation<V>
where
    V: ViableSystem,
{
    /// Creates an escalation record for a failed or rejected acknowledgement.
    pub fn new(
        metadata: ProtocolMetadata,
        conflict: CoordinationConflict<V>,
        intervention_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            metadata,
            conflict,
            intervention_id: intervention_id.into(),
            reason: reason.into(),
            escalated_at: Utc::now(),
        }
    }
}

/// One completed System 2 policy cycle.
#[derive(Debug)]
pub struct CoordinationCycle<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub views: Vec<CoordinationViewRecord<V>>,
    pub conflicts: Vec<CoordinationConflict<V>>,
    pub interventions: Vec<CoordinationIntervention<V>>,
    pub acknowledgements: Vec<CoordinationAcknowledgement<V>>,
    pub escalations: Vec<CoordinationEscalation<V>>,
}

impl<V> Clone for CoordinationCycle<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            views: self.views.clone(),
            conflicts: self.conflicts.clone(),
            interventions: self.interventions.clone(),
            acknowledgements: self.acknowledgements.clone(),
            escalations: self.escalations.clone(),
        }
    }
}

impl<V> CoordinationCycle<V>
where
    V: ViableSystem,
{
    /// Creates an empty cycle.
    pub fn empty(metadata: ProtocolMetadata) -> Self {
        Self {
            metadata,
            views: Vec::new(),
            conflicts: Vec::new(),
            interventions: Vec::new(),
            acknowledgements: Vec::new(),
            escalations: Vec::new(),
        }
    }
}

/// Point-in-time snapshot of the typed System 2 runtime.
#[derive(Debug)]
pub struct System2Snapshot<V>
where
    V: ViableSystem,
{
    pub views: Vec<CoordinationViewRecord<V>>,
    pub pending_interventions: Vec<CoordinationIntervention<V>>,
    pub acknowledgements: Vec<CoordinationAcknowledgement<V>>,
    pub escalations: Vec<CoordinationEscalation<V>>,
    pub last_cycle_at: Option<DateTime<Utc>>,
}

impl<V> Clone for System2Snapshot<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            views: self.views.clone(),
            pending_interventions: self.pending_interventions.clone(),
            acknowledgements: self.acknowledgements.clone(),
            escalations: self.escalations.clone(),
            last_cycle_at: self.last_cycle_at,
        }
    }
}
