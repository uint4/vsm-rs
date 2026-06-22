//! Typed algedonic signal lifecycle records.
//!
//! These records model the framework-owned emergency lifecycle from proposed
//! signal through classification, dispatch, acknowledgement, escalation, and
//! resolution. Domain interpretation remains in roles and System 5 policy.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::roles::ViableSystem;

use super::address::VsmAddress;
use super::envelope::ProtocolMetadata;
use super::system5::CrisisResponse;

/// Algedonic signal category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgedonicSignalKind {
    Pain,
    Pleasure,
    Anomaly,
    Opportunity,
    Emergency,
}

/// Algedonic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlgedonicSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl AlgedonicSeverity {
    /// Returns a generic priority contribution for this severity.
    pub fn score(self) -> f64 {
        match self {
            Self::Low => 0.25,
            Self::Medium => 0.5,
            Self::High => 0.75,
            Self::Critical => 1.0,
        }
    }
}

/// Lifecycle status for an algedonic signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgedonicLifecycleStatus {
    Proposed,
    Classified,
    Dispatched,
    Acknowledged,
    ActedUpon,
    Resolved,
    Escalated,
    Expired,
}

/// Typed algedonic signal record.
pub struct AlgedonicSignalRecord<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub signal_id: String,
    pub kind: AlgedonicSignalKind,
    pub severity: AlgedonicSeverity,
    pub priority: f64,
    pub source: Option<VsmAddress>,
    pub source_label: Option<String>,
    pub unit_id: Option<V::UnitId>,
    pub reason: String,
    pub details: BTreeMap<String, String>,
    pub status: AlgedonicLifecycleStatus,
    pub proposed_at: DateTime<Utc>,
    pub acknowledgement_deadline: Option<DateTime<Utc>>,
    pub dedupe_key: Option<String>,
}

impl<V> Clone for AlgedonicSignalRecord<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            signal_id: self.signal_id.clone(),
            kind: self.kind,
            severity: self.severity,
            priority: self.priority,
            source: self.source.clone(),
            source_label: self.source_label.clone(),
            unit_id: self.unit_id.clone(),
            reason: self.reason.clone(),
            details: self.details.clone(),
            status: self.status,
            proposed_at: self.proposed_at,
            acknowledgement_deadline: self.acknowledgement_deadline,
            dedupe_key: self.dedupe_key.clone(),
        }
    }
}

impl<V> AlgedonicSignalRecord<V>
where
    V: ViableSystem,
{
    /// Creates an algedonic signal record.
    pub fn new(
        kind: AlgedonicSignalKind,
        severity: AlgedonicSeverity,
        reason: impl Into<String>,
    ) -> Self {
        let priority = severity.score();
        Self {
            metadata: ProtocolMetadata::new(),
            signal_id: format!("algedonic-{}", Uuid::new_v4()),
            kind,
            severity,
            priority,
            source: None,
            source_label: None,
            unit_id: None,
            reason: reason.into(),
            details: BTreeMap::new(),
            status: AlgedonicLifecycleStatus::Proposed,
            proposed_at: Utc::now(),
            acknowledgement_deadline: None,
            dedupe_key: None,
        }
    }

    /// Sets a priority score clamped to `0..=1`.
    pub fn with_priority(mut self, priority: f64) -> Self {
        self.priority = priority.clamp(0.0, 1.0);
        self
    }

    /// Attaches a source label.
    pub fn from_source(mut self, source: impl Into<String>) -> Self {
        self.source_label = Some(source.into());
        self
    }

    /// Attaches a unit identity.
    pub fn from_unit(mut self, unit_id: V::UnitId) -> Self {
        self.unit_id = Some(unit_id);
        self
    }

    /// Returns true when the generic priority path should dispatch to System 5.
    pub fn requires_system5_dispatch(&self) -> bool {
        matches!(
            self.severity,
            AlgedonicSeverity::High | AlgedonicSeverity::Critical
        ) || self.priority >= 0.75
    }
}

/// Acknowledgement status for an algedonic signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgedonicAckStatus {
    Accepted,
    Rejected,
    TimedOut,
}

/// Acknowledgement for an algedonic signal lifecycle step.
pub struct AlgedonicAcknowledgement<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub signal_id: String,
    pub status: AlgedonicAckStatus,
    pub acknowledged_by: Option<VsmAddress>,
    pub unit_id: Option<V::UnitId>,
    pub reason: Option<String>,
    pub acknowledged_at: DateTime<Utc>,
}

impl<V> Clone for AlgedonicAcknowledgement<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            signal_id: self.signal_id.clone(),
            status: self.status,
            acknowledged_by: self.acknowledged_by.clone(),
            unit_id: self.unit_id.clone(),
            reason: self.reason.clone(),
            acknowledged_at: self.acknowledged_at,
        }
    }
}

impl<V> AlgedonicAcknowledgement<V>
where
    V: ViableSystem,
{
    /// Records an accepted acknowledgement for a signal.
    pub fn accepted(signal: &AlgedonicSignalRecord<V>) -> Self {
        Self {
            metadata: signal.metadata.child(),
            signal_id: signal.signal_id.clone(),
            status: AlgedonicAckStatus::Accepted,
            acknowledged_by: None,
            unit_id: signal.unit_id.clone(),
            reason: None,
            acknowledged_at: Utc::now(),
        }
    }
}

/// Escalation record for an algedonic signal.
pub struct AlgedonicEscalation<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub signal_id: String,
    pub target: VsmAddress,
    pub unit_id: Option<V::UnitId>,
    pub reason: String,
    pub escalated_at: DateTime<Utc>,
}

impl<V> Clone for AlgedonicEscalation<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            signal_id: self.signal_id.clone(),
            target: self.target.clone(),
            unit_id: self.unit_id.clone(),
            reason: self.reason.clone(),
            escalated_at: self.escalated_at,
        }
    }
}

/// Alert retained by the runtime before optional external delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlgedonicAlert {
    pub metadata: ProtocolMetadata,
    pub signal_id: String,
    pub severity: AlgedonicSeverity,
    pub message: String,
    pub details: BTreeMap<String, String>,
    pub raised_at: DateTime<Utc>,
}

/// One algedonic lifecycle cycle.
pub struct AlgedonicCycle<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub signal: AlgedonicSignalRecord<V>,
    pub acknowledgements: Vec<AlgedonicAcknowledgement<V>>,
    pub escalations: Vec<AlgedonicEscalation<V>>,
    pub crisis_response: Option<CrisisResponse<V>>,
    pub recorded_at: DateTime<Utc>,
}

impl<V> Clone for AlgedonicCycle<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            signal: self.signal.clone(),
            acknowledgements: self.acknowledgements.clone(),
            escalations: self.escalations.clone(),
            crisis_response: self.crisis_response.clone(),
            recorded_at: self.recorded_at,
        }
    }
}

/// Snapshot of retained algedonic lifecycle state.
pub struct AlgedonicSnapshot<V>
where
    V: ViableSystem,
{
    pub signals: Vec<AlgedonicSignalRecord<V>>,
    pub acknowledgements: Vec<AlgedonicAcknowledgement<V>>,
    pub escalations: Vec<AlgedonicEscalation<V>>,
    pub alerts: Vec<AlgedonicAlert>,
    pub last_signal_at: Option<DateTime<Utc>>,
}

impl<V> Clone for AlgedonicSnapshot<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            signals: self.signals.clone(),
            acknowledgements: self.acknowledgements.clone(),
            escalations: self.escalations.clone(),
            alerts: self.alerts.clone(),
            last_signal_at: self.last_signal_at,
        }
    }
}
