//! Typed temporal analysis records.
//!
//! These records keep generic timescale, aggregation, pattern, forecast, and
//! causal-hypothesis mechanics in the framework while applications provide
//! interpretation through temporal roles.

use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::envelope::ProtocolMetadata;

/// Named temporal scale and retention window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalScale {
    pub name: String,
    pub window: Duration,
    pub schema_version: u64,
}

impl TemporalScale {
    /// Creates a temporal scale.
    pub fn new(name: impl Into<String>, window: Duration) -> Self {
        Self {
            name: name.into(),
            window,
            schema_version: 1,
        }
    }
}

/// One numeric sample in a temporal window.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalSample {
    pub metadata: ProtocolMetadata,
    pub sample_id: String,
    pub scale: String,
    pub value: f64,
    pub dimensions: BTreeMap<String, String>,
    pub observed_at: DateTime<Utc>,
}

impl TemporalSample {
    /// Creates a temporal sample.
    pub fn new(scale: impl Into<String>, value: f64) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            sample_id: format!("temporal-sample-{}", Uuid::new_v4()),
            scale: scale.into(),
            value,
            dimensions: BTreeMap::new(),
            observed_at: Utc::now(),
        }
    }
}

/// Aggregate values for one temporal scale.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalAggregate {
    pub metadata: ProtocolMetadata,
    pub scale: String,
    pub count: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub latest_at: Option<DateTime<Utc>>,
}

impl TemporalAggregate {
    /// Aggregates samples for one scale.
    pub fn from_samples(scale: impl Into<String>, samples: &[TemporalSample]) -> Self {
        let scale = scale.into();
        let count = samples.len();
        let (min, max, sum) = samples.iter().fold(
            (f64::INFINITY, f64::NEG_INFINITY, 0.0),
            |(min, max, sum), sample| {
                (
                    min.min(sample.value),
                    max.max(sample.value),
                    sum + sample.value,
                )
            },
        );
        let latest_at = samples.iter().map(|sample| sample.observed_at).max();

        Self {
            metadata: ProtocolMetadata::new(),
            scale,
            count,
            min: if count == 0 { 0.0 } else { min },
            max: if count == 0 { 0.0 } else { max },
            mean: if count == 0 { 0.0 } else { sum / count as f64 },
            latest_at,
        }
    }
}

/// Category for a temporal pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalPatternKind {
    Trend,
    Cycle,
    Seasonality,
    Anomaly,
    Custom(String),
}

/// Replaceable temporal pattern record.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalPattern {
    pub metadata: ProtocolMetadata,
    pub pattern_id: String,
    pub kind: TemporalPatternKind,
    pub scale: String,
    pub confidence: f64,
    pub description: String,
    pub detected_at: DateTime<Utc>,
}

impl TemporalPattern {
    /// Creates a temporal pattern.
    pub fn new(
        kind: TemporalPatternKind,
        scale: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            pattern_id: format!("temporal-pattern-{}", Uuid::new_v4()),
            kind,
            scale: scale.into(),
            confidence: 1.0,
            description: description.into(),
            detected_at: Utc::now(),
        }
    }
}

/// One forecast point.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalForecastPoint {
    pub offset: Duration,
    pub value: f64,
    pub confidence: f64,
}

/// Replaceable temporal forecast record.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalForecast {
    pub metadata: ProtocolMetadata,
    pub forecast_id: String,
    pub scale: String,
    pub points: Vec<TemporalForecastPoint>,
    pub generated_at: DateTime<Utc>,
}

impl TemporalForecast {
    /// Creates an empty forecast for a scale.
    pub fn new(scale: impl Into<String>) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            forecast_id: format!("temporal-forecast-{}", Uuid::new_v4()),
            scale: scale.into(),
            points: Vec::new(),
            generated_at: Utc::now(),
        }
    }
}

/// Replaceable causal hypothesis record.
#[derive(Debug, Clone, PartialEq)]
pub struct CausalHypothesis {
    pub metadata: ProtocolMetadata,
    pub hypothesis_id: String,
    pub cause: String,
    pub effect: String,
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub detected_at: DateTime<Utc>,
}

impl CausalHypothesis {
    /// Creates a causal hypothesis.
    pub fn new(cause: impl Into<String>, effect: impl Into<String>, confidence: f64) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            hypothesis_id: format!("temporal-causal-{}", Uuid::new_v4()),
            cause: cause.into(),
            effect: effect.into(),
            confidence: confidence.clamp(0.0, 1.0),
            evidence: Vec::new(),
            detected_at: Utc::now(),
        }
    }
}

/// Temporal analysis returned by an application strategy role.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalAnalysis {
    pub metadata: ProtocolMetadata,
    pub analysis_id: String,
    pub aggregates: Vec<TemporalAggregate>,
    pub patterns: Vec<TemporalPattern>,
    pub forecasts: Vec<TemporalForecast>,
    pub causal_hypotheses: Vec<CausalHypothesis>,
    pub analyzed_at: DateTime<Utc>,
}

impl TemporalAnalysis {
    /// Creates an empty analysis for current aggregates.
    pub fn empty(aggregates: Vec<TemporalAggregate>) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            analysis_id: format!("temporal-analysis-{}", Uuid::new_v4()),
            aggregates,
            patterns: Vec::new(),
            forecasts: Vec::new(),
            causal_hypotheses: Vec::new(),
            analyzed_at: Utc::now(),
        }
    }
}

/// Snapshot of retained temporal samples and analyses.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalSnapshot {
    pub samples: Vec<TemporalSample>,
    pub aggregates: Vec<TemporalAggregate>,
    pub analyses: Vec<TemporalAnalysis>,
    pub last_sample_at: Option<DateTime<Utc>>,
}
