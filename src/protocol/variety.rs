//! Typed variety lifecycle records.
//!
//! These records describe runtime-owned variety mechanics: estimates,
//! uncertainty, interventions, and outcomes. Applications supply domain
//! interpretation through variety roles.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::roles::ViableSystem;

use super::algedonic::AlgedonicSnapshot;
use super::envelope::ProtocolMetadata;
use super::temporal::TemporalSnapshot;

/// Direction of a variety trend over a retained window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarietyTrend {
    Increasing,
    Decreasing,
    Stable,
    Unknown,
}

/// Confidence metadata for a variety estimate.
#[derive(Debug, Clone, PartialEq)]
pub struct VarietyUncertainty {
    pub confidence: f64,
    pub method: Option<String>,
    pub notes: BTreeMap<String, String>,
}

impl VarietyUncertainty {
    /// Creates uncertainty metadata with confidence clamped to `0..=1`.
    pub fn new(confidence: f64) -> Self {
        Self {
            confidence: confidence.clamp(0.0, 1.0),
            method: None,
            notes: BTreeMap::new(),
        }
    }
}

impl Default for VarietyUncertainty {
    fn default() -> Self {
        Self::new(1.0)
    }
}

/// Framework-owned variety estimate.
#[derive(Debug, Clone, PartialEq)]
pub struct VarietyEstimate {
    pub metadata: ProtocolMetadata,
    pub estimate_id: String,
    pub input: f64,
    pub output: f64,
    pub ratio: f64,
    pub trend: VarietyTrend,
    pub uncertainty: VarietyUncertainty,
    pub dimensions: BTreeMap<String, String>,
    pub measured_at: DateTime<Utc>,
}

impl VarietyEstimate {
    /// Creates an estimate and derives the output/input ratio.
    pub fn new(input: f64, output: f64) -> Self {
        let input = input.max(0.0);
        let output = output.max(0.0);
        let ratio = if input > 0.0 { output / input } else { 0.0 };

        Self {
            metadata: ProtocolMetadata::new(),
            estimate_id: format!("variety-estimate-{}", Uuid::new_v4()),
            input,
            output,
            ratio,
            trend: VarietyTrend::Unknown,
            uncertainty: VarietyUncertainty::default(),
            dimensions: BTreeMap::new(),
            measured_at: Utc::now(),
        }
    }

    /// Marks the trend associated with this estimate.
    pub fn with_trend(mut self, trend: VarietyTrend) -> Self {
        self.trend = trend;
        self
    }
}

/// Variety observation associated with an optional operational unit.
pub struct VarietyObservation<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub observation_id: String,
    pub source_unit: Option<V::UnitId>,
    pub estimate: VarietyEstimate,
    pub notes: BTreeMap<String, String>,
    pub observed_at: DateTime<Utc>,
}

impl<V> Clone for VarietyObservation<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            observation_id: self.observation_id.clone(),
            source_unit: self.source_unit.clone(),
            estimate: self.estimate.clone(),
            notes: self.notes.clone(),
            observed_at: self.observed_at,
        }
    }
}

impl<V> VarietyObservation<V>
where
    V: ViableSystem,
{
    /// Creates an observation from an estimate.
    pub fn new(estimate: VarietyEstimate) -> Self {
        Self {
            metadata: estimate.metadata.child(),
            observation_id: format!("variety-observation-{}", Uuid::new_v4()),
            source_unit: None,
            estimate,
            notes: BTreeMap::new(),
            observed_at: Utc::now(),
        }
    }

    /// Attaches the source operational unit.
    pub fn from_unit(mut self, unit_id: V::UnitId) -> Self {
        self.source_unit = Some(unit_id);
        self
    }
}

/// Type of variety intervention proposed by an application policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarietyInterventionKind {
    Attenuation,
    Amplification,
    Rebalancing,
    Monitoring,
    Custom(String),
}

/// Intervention proposed to change effective variety.
pub struct VarietyIntervention<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub intervention_id: String,
    pub kind: VarietyInterventionKind,
    pub target_units: Vec<V::UnitId>,
    pub reason: String,
    pub expected_effect: Option<VarietyEstimate>,
    pub requires_ack: bool,
    pub proposed_at: DateTime<Utc>,
}

impl<V> Clone for VarietyIntervention<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            intervention_id: self.intervention_id.clone(),
            kind: self.kind.clone(),
            target_units: self.target_units.clone(),
            reason: self.reason.clone(),
            expected_effect: self.expected_effect.clone(),
            requires_ack: self.requires_ack,
            proposed_at: self.proposed_at,
        }
    }
}

impl<V> VarietyIntervention<V>
where
    V: ViableSystem,
{
    /// Creates a variety intervention.
    pub fn new(kind: VarietyInterventionKind, reason: impl Into<String>) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            intervention_id: format!("variety-intervention-{}", Uuid::new_v4()),
            kind,
            target_units: Vec::new(),
            reason: reason.into(),
            expected_effect: None,
            requires_ack: true,
            proposed_at: Utc::now(),
        }
    }

    /// Adds a target operational unit.
    pub fn target_unit(mut self, unit_id: V::UnitId) -> Self {
        self.target_units.push(unit_id);
        self
    }
}

/// Recorded outcome for a variety intervention.
pub struct VarietyInterventionOutcome<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub intervention_id: String,
    pub target_unit: Option<V::UnitId>,
    pub accepted: bool,
    pub effect: Option<VarietyEstimate>,
    pub reason: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

impl<V> Clone for VarietyInterventionOutcome<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            intervention_id: self.intervention_id.clone(),
            target_unit: self.target_unit.clone(),
            accepted: self.accepted,
            effect: self.effect.clone(),
            reason: self.reason.clone(),
            recorded_at: self.recorded_at,
        }
    }
}

impl<V> VarietyInterventionOutcome<V>
where
    V: ViableSystem,
{
    /// Records an accepted intervention outcome.
    pub fn accepted(intervention: &VarietyIntervention<V>) -> Self {
        Self {
            metadata: intervention.metadata.child(),
            intervention_id: intervention.intervention_id.clone(),
            target_unit: None,
            accepted: true,
            effect: None,
            reason: None,
            recorded_at: Utc::now(),
        }
    }
}

/// One variety engineering cycle.
pub struct VarietyCycle<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub observation: VarietyObservation<V>,
    pub interventions: Vec<VarietyIntervention<V>>,
    pub outcomes: Vec<VarietyInterventionOutcome<V>>,
    pub evaluated_at: DateTime<Utc>,
}

impl<V> Clone for VarietyCycle<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            observation: self.observation.clone(),
            interventions: self.interventions.clone(),
            outcomes: self.outcomes.clone(),
            evaluated_at: self.evaluated_at,
        }
    }
}

/// Snapshot of retained variety lifecycle state.
pub struct VarietySnapshot<V>
where
    V: ViableSystem,
{
    pub observations: Vec<VarietyObservation<V>>,
    pub interventions: Vec<VarietyIntervention<V>>,
    pub outcomes: Vec<VarietyInterventionOutcome<V>>,
    pub last_observed_at: Option<DateTime<Utc>>,
}

/// Snapshot of the combined variety, algedonic, and temporal lifecycle adapter.
pub struct VarietyAlgedonicTemporalSnapshot<V>
where
    V: ViableSystem,
{
    pub variety: VarietySnapshot<V>,
    pub algedonic: AlgedonicSnapshot<V>,
    pub temporal: TemporalSnapshot,
}

impl<V> Clone for VarietyAlgedonicTemporalSnapshot<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            variety: self.variety.clone(),
            algedonic: self.algedonic.clone(),
            temporal: self.temporal.clone(),
        }
    }
}

impl<V> Clone for VarietySnapshot<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            observations: self.observations.clone(),
            interventions: self.interventions.clone(),
            outcomes: self.outcomes.clone(),
            last_observed_at: self.last_observed_at,
        }
    }
}
