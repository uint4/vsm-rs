//! Typed System 4 environmental intelligence protocol records.
//!
//! These records are framework-owned. Applications provide meaning through
//! System 4 roles rather than by forcing environmental payloads into the core
//! type family.

use std::time::Duration;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::address::VsmAddress;
use super::envelope::ProtocolMetadata;

/// Freshness state for environmental observations and sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshnessStatus {
    Fresh,
    Stale,
    Expired,
}

/// Descriptor for a dynamically registered environmental source.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentSourceDescriptor {
    pub source_id: String,
    pub label: String,
    pub description: Option<String>,
    pub provenance: Vec<String>,
    pub stale_after: Option<Duration>,
    pub tags: Vec<String>,
}

impl EnvironmentSourceDescriptor {
    /// Creates a source descriptor with a stable source identity.
    pub fn new(source_id: impl Into<String>) -> Self {
        let source_id = source_id.into();
        Self {
            label: source_id.clone(),
            source_id,
            description: None,
            provenance: Vec::new(),
            stale_after: None,
            tags: Vec::new(),
        }
    }

    /// Sets the human-readable source label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets source freshness tolerance.
    pub fn with_stale_after(mut self, stale_after: Duration) -> Self {
        self.stale_after = Some(stale_after);
        self
    }

    /// Sets source provenance hints.
    pub fn with_provenance(
        mut self,
        provenance: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.provenance = provenance.into_iter().map(Into::into).collect();
        self
    }
}

/// Runtime status for one environmental source.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentSourceStatus {
    pub descriptor: EnvironmentSourceDescriptor,
    pub observation_count: usize,
    pub restart_count: usize,
    pub last_observed_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub freshness: FreshnessStatus,
}

impl EnvironmentSourceStatus {
    /// Creates a running status snapshot for a source descriptor.
    pub fn new(descriptor: EnvironmentSourceDescriptor) -> Self {
        Self {
            descriptor,
            observation_count: 0,
            restart_count: 0,
            last_observed_at: None,
            last_error: None,
            freshness: FreshnessStatus::Fresh,
        }
    }
}

/// Numeric environmental measurement normalized by the framework boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentalMeasurement {
    pub name: String,
    pub value: f64,
    pub unit: Option<String>,
}

impl EnvironmentalMeasurement {
    /// Creates a named numeric measurement.
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
            unit: None,
        }
    }

    /// Adds a unit label.
    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }
}

/// Normalized observation emitted by an environmental source.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentalObservation {
    pub metadata: ProtocolMetadata,
    pub observation_id: String,
    pub source_id: String,
    pub observed_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub provenance: Vec<String>,
    pub confidence: f64,
    pub freshness: FreshnessStatus,
    pub summary: Option<String>,
    pub measurements: Vec<EnvironmentalMeasurement>,
    pub tags: Vec<String>,
}

impl EnvironmentalObservation {
    /// Creates an observation for a source.
    pub fn new(source_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            metadata: ProtocolMetadata::new(),
            observation_id: format!("observation-{}", Uuid::new_v4()),
            source_id: source_id.into(),
            observed_at: now,
            received_at: now,
            provenance: Vec::new(),
            confidence: 1.0,
            freshness: FreshnessStatus::Fresh,
            summary: None,
            measurements: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Adds a numeric measurement.
    pub fn with_measurement(mut self, measurement: EnvironmentalMeasurement) -> Self {
        self.measurements.push(measurement);
        self
    }

    /// Sets confidence, clamped into the inclusive `0.0..=1.0` range.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = clamp_unit(confidence);
        self
    }

    /// Sets a human-readable observation summary.
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }
}

/// Application-interpreted environmental signal class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalKind {
    Opportunity,
    Threat,
    WeakSignal,
    Anomaly,
    Custom(String),
}

/// Signal interpreted from one or more environmental observations.
#[derive(Debug, Clone, PartialEq)]
pub struct InterpretedSignal {
    pub metadata: ProtocolMetadata,
    pub signal_id: String,
    pub source_id: Option<String>,
    pub observation_id: Option<String>,
    pub kind: SignalKind,
    pub strength: f64,
    pub confidence: f64,
    pub uncertainty: f64,
    pub detected_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub rationale: Option<String>,
    pub provenance: Vec<String>,
}

impl InterpretedSignal {
    /// Creates an interpreted signal.
    pub fn new(kind: SignalKind) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            signal_id: format!("signal-{}", Uuid::new_v4()),
            source_id: None,
            observation_id: None,
            kind,
            strength: 0.0,
            confidence: 1.0,
            uncertainty: 0.0,
            detected_at: Utc::now(),
            expires_at: None,
            rationale: None,
            provenance: Vec::new(),
        }
    }

    /// Links the signal to an observation.
    pub fn from_observation(mut self, observation: &EnvironmentalObservation) -> Self {
        self.source_id = Some(observation.source_id.clone());
        self.observation_id = Some(observation.observation_id.clone());
        self.metadata = observation.metadata.child();
        self.provenance = observation.provenance.clone();
        self.confidence = observation.confidence;
        self
    }

    /// Sets signal strength, clamped into the inclusive `-1.0..=1.0` range.
    pub fn with_strength(mut self, strength: f64) -> Self {
        self.strength = strength.clamp(-1.0, 1.0);
        self
    }

    /// Sets confidence, clamped into the inclusive `0.0..=1.0` range.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = clamp_unit(confidence);
        self
    }

    /// Sets uncertainty, clamped into the inclusive `0.0..=1.0` range.
    pub fn with_uncertainty(mut self, uncertainty: f64) -> Self {
        self.uncertainty = clamp_unit(uncertainty);
        self
    }

    /// Adds an interpretation rationale.
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }
}

/// Intelligence assessment produced from interpreted signals.
#[derive(Debug, Clone, PartialEq)]
pub struct IntelligenceAssessment {
    pub metadata: ProtocolMetadata,
    pub assessment_id: String,
    pub generated_at: DateTime<Utc>,
    pub signals: Vec<InterpretedSignal>,
    pub summary: Option<String>,
    pub risk_score: f64,
    pub opportunity_score: f64,
    pub uncertainty: f64,
    pub recommendations: Vec<String>,
}

impl IntelligenceAssessment {
    /// Creates an assessment from interpreted signals.
    pub fn new(signals: Vec<InterpretedSignal>) -> Self {
        let risk_score = signals
            .iter()
            .filter(|signal| signal.kind == SignalKind::Threat)
            .map(|signal| signal.strength.abs())
            .fold(0.0, f64::max);
        let opportunity_score = signals
            .iter()
            .filter(|signal| signal.kind == SignalKind::Opportunity)
            .map(|signal| signal.strength.max(0.0))
            .fold(0.0, f64::max);
        let uncertainty = if signals.is_empty() {
            0.0
        } else {
            signals.iter().map(|signal| signal.uncertainty).sum::<f64>() / signals.len() as f64
        };

        Self {
            metadata: ProtocolMetadata::new(),
            assessment_id: format!("assessment-{}", Uuid::new_v4()),
            generated_at: Utc::now(),
            signals,
            summary: None,
            risk_score,
            opportunity_score,
            uncertainty: clamp_unit(uncertainty),
            recommendations: Vec::new(),
        }
    }

    /// Creates an empty assessment.
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }
}

/// One point in a forecast horizon.
#[derive(Debug, Clone, PartialEq)]
pub struct ForecastPoint {
    pub offset: Duration,
    pub value: f64,
    pub confidence: f64,
}

impl ForecastPoint {
    /// Creates a forecast point.
    pub fn new(offset: Duration, value: f64, confidence: f64) -> Self {
        Self {
            offset,
            value,
            confidence: clamp_unit(confidence),
        }
    }
}

/// Forecast produced by a System 4 role.
#[derive(Debug, Clone, PartialEq)]
pub struct Forecast {
    pub metadata: ProtocolMetadata,
    pub forecast_id: String,
    pub assessment_id: Option<String>,
    pub generated_at: DateTime<Utc>,
    pub horizon: Duration,
    pub model: Option<String>,
    pub confidence: f64,
    pub uncertainty: f64,
    pub points: Vec<ForecastPoint>,
    pub provenance: Vec<String>,
}

impl Forecast {
    /// Creates a forecast for an assessment and horizon.
    pub fn new(assessment: &IntelligenceAssessment, horizon: Duration) -> Self {
        Self {
            metadata: assessment.metadata.child(),
            forecast_id: format!("forecast-{}", Uuid::new_v4()),
            assessment_id: Some(assessment.assessment_id.clone()),
            generated_at: Utc::now(),
            horizon,
            model: None,
            confidence: 1.0,
            uncertainty: assessment.uncertainty,
            points: Vec::new(),
            provenance: Vec::new(),
        }
    }
}

/// Scenario derived from forecast and intelligence records.
#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    pub metadata: ProtocolMetadata,
    pub scenario_id: String,
    pub forecast_id: Option<String>,
    pub title: String,
    pub generated_at: DateTime<Utc>,
    pub probability: f64,
    pub impact: f64,
    pub uncertainty: f64,
    pub rationale: Option<String>,
    pub provenance: Vec<String>,
}

impl Scenario {
    /// Creates a scenario with generated identity.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            scenario_id: format!("scenario-{}", Uuid::new_v4()),
            forecast_id: None,
            title: title.into(),
            generated_at: Utc::now(),
            probability: 0.0,
            impact: 0.0,
            uncertainty: 0.0,
            rationale: None,
            provenance: Vec::new(),
        }
    }
}

/// System 3 feasibility information attached to an adaptation proposal.
#[derive(Debug, Clone, PartialEq)]
pub struct OperationalFeasibilityInfo {
    pub requested_at: DateTime<Utc>,
    pub assessed_by: Option<VsmAddress>,
    pub summary: String,
    pub constraints: Vec<String>,
    pub confidence: f64,
}

/// Adaptation proposal intended for governance by System 5.
#[derive(Debug, Clone, PartialEq)]
pub struct AdaptationProposal {
    pub metadata: ProtocolMetadata,
    pub proposal_id: String,
    pub scenario_id: Option<String>,
    pub title: String,
    pub rationale: String,
    pub expected_benefit: f64,
    pub urgency: f64,
    pub uncertainty: f64,
    pub generated_at: DateTime<Utc>,
    pub feasibility: Option<OperationalFeasibilityInfo>,
    pub destination: Option<VsmAddress>,
    pub provenance: Vec<String>,
}

impl AdaptationProposal {
    /// Creates an adaptation proposal with generated identity.
    pub fn new(title: impl Into<String>, rationale: impl Into<String>) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            proposal_id: format!("adaptation-proposal-{}", Uuid::new_v4()),
            scenario_id: None,
            title: title.into(),
            rationale: rationale.into(),
            expected_benefit: 0.0,
            urgency: 0.0,
            uncertainty: 0.0,
            generated_at: Utc::now(),
            feasibility: None,
            destination: None,
            provenance: Vec::new(),
        }
    }
}

/// Calibration result comparing a forecast with observed outcomes.
#[derive(Debug, Clone, PartialEq)]
pub struct ForecastCalibration {
    pub metadata: ProtocolMetadata,
    pub calibration_id: String,
    pub forecast_id: String,
    pub calibrated_at: DateTime<Utc>,
    pub sample_size: usize,
    pub mean_absolute_error: f64,
    pub uncertainty_after: f64,
    pub notes: Vec<String>,
}

impl ForecastCalibration {
    /// Creates a calibration record.
    pub fn new(forecast_id: impl Into<String>, sample_size: usize) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            calibration_id: format!("forecast-calibration-{}", Uuid::new_v4()),
            forecast_id: forecast_id.into(),
            calibrated_at: Utc::now(),
            sample_size,
            mean_absolute_error: 0.0,
            uncertainty_after: 0.0,
            notes: Vec::new(),
        }
    }
}

/// Result of one System 4 intelligence cycle.
#[derive(Debug, Clone, PartialEq)]
pub struct System4IntelligenceCycle {
    pub metadata: ProtocolMetadata,
    pub observations: Vec<EnvironmentalObservation>,
    pub signals: Vec<InterpretedSignal>,
    pub assessment: IntelligenceAssessment,
    pub forecasts: Vec<Forecast>,
    pub scenarios: Vec<Scenario>,
    pub proposals: Vec<AdaptationProposal>,
    pub stale_sources: Vec<EnvironmentSourceStatus>,
    pub generated_at: DateTime<Utc>,
}

/// Snapshot of the typed System 4 runtime.
#[derive(Debug, Clone, PartialEq)]
pub struct System4Snapshot {
    pub sources: Vec<EnvironmentSourceStatus>,
    pub observations: Vec<EnvironmentalObservation>,
    pub signals: Vec<InterpretedSignal>,
    pub assessments: Vec<IntelligenceAssessment>,
    pub forecasts: Vec<Forecast>,
    pub scenarios: Vec<Scenario>,
    pub proposals: Vec<AdaptationProposal>,
    pub calibrations: Vec<ForecastCalibration>,
    pub last_cycle_at: Option<DateTime<Utc>>,
}

fn clamp_unit(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}
