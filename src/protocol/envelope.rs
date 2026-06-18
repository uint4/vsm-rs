//! Framework-owned envelope metadata.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::address::VsmAddress;

/// Correlation or causation identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId(String);

impl CorrelationId {
    /// Creates a fresh correlation ID backed by a UUID string.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Creates an ID from an existing string.
    pub fn from_string(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Protocol version carried by framework metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    /// Current protocol version for the foundational typed records.
    pub const CURRENT: Self = Self { major: 0, minor: 1 };
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}

/// Framework priority used by admission and routing policies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

/// Trace context propagated by adapters and runtime handles.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceContext {
    pub fields: BTreeMap<String, String>,
}

impl TraceContext {
    /// Creates an empty trace context.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Framework metadata shared by typed protocol records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolMetadata {
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CorrelationId>,
    pub deadline: Option<DateTime<Utc>>,
    pub priority: Priority,
    pub protocol_version: ProtocolVersion,
    pub trace_context: TraceContext,
    pub source: Option<VsmAddress>,
    pub destination: Option<VsmAddress>,
}

impl ProtocolMetadata {
    /// Creates metadata with a fresh correlation ID and default framework values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a child metadata value with causation pointing at this metadata.
    pub fn child(&self) -> Self {
        Self {
            correlation_id: self.correlation_id.clone(),
            causation_id: Some(self.correlation_id.clone()),
            deadline: self.deadline,
            priority: self.priority,
            protocol_version: self.protocol_version,
            trace_context: self.trace_context.clone(),
            source: self.source.clone(),
            destination: self.destination.clone(),
        }
    }
}

impl Default for ProtocolMetadata {
    fn default() -> Self {
        Self {
            correlation_id: CorrelationId::new(),
            causation_id: None,
            deadline: None,
            priority: Priority::Normal,
            protocol_version: ProtocolVersion::CURRENT,
            trace_context: TraceContext::new(),
            source: None,
            destination: None,
        }
    }
}
