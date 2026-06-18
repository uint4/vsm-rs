//! Instance-scoped runtime addressing.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identifier for one runtime instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeId(String);

impl RuntimeId {
    /// Creates a fresh runtime ID backed by a UUID string.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Creates a runtime ID from an existing string.
    pub fn from_string(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the runtime ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RuntimeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RuntimeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Path from a root runtime to a nested runtime.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecursionPath(Vec<String>);

impl RecursionPath {
    /// Returns the root recursion path.
    pub fn root() -> Self {
        Self::default()
    }

    /// Creates a path from explicit path segments.
    pub fn from_segments(segments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self(segments.into_iter().map(Into::into).collect())
    }

    /// Returns a child path without mutating the current path.
    pub fn child(&self, segment: impl Into<String>) -> Self {
        let mut segments = self.0.clone();
        segments.push(segment.into());
        Self(segments)
    }

    /// Returns `true` when this path addresses the root runtime.
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the path segments.
    pub fn segments(&self) -> &[String] {
        &self.0
    }
}

/// Runtime role addressed by framework metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubsystemRole {
    System1,
    System2,
    System3,
    System3Star,
    System4,
    System5,
    Algedonic,
    TemporalVariety,
    Telemetry,
    EventSink,
    ReportSink,
    StateStore,
    Custom(String),
}

/// Framework address for a subsystem role and optional entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VsmAddress {
    pub runtime_id: RuntimeId,
    pub recursion_path: RecursionPath,
    pub role: SubsystemRole,
    pub entity: Option<String>,
}

impl VsmAddress {
    /// Creates an address for a subsystem role.
    pub fn new(runtime_id: RuntimeId, recursion_path: RecursionPath, role: SubsystemRole) -> Self {
        Self {
            runtime_id,
            recursion_path,
            role,
            entity: None,
        }
    }

    /// Adds or replaces the entity component.
    pub fn with_entity(mut self, entity: impl Into<String>) -> Self {
        self.entity = Some(entity.into());
        self
    }
}
