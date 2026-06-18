//! Snapshot metadata and records.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::address::{RecursionPath, RuntimeId, SubsystemRole};

/// Version marker for a typed snapshot payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SnapshotVersion(pub u64);

impl SnapshotVersion {
    /// Initial snapshot version.
    pub const INITIAL: Self = Self(1);
}

impl Default for SnapshotVersion {
    fn default() -> Self {
        Self::INITIAL
    }
}

/// Framework-owned snapshot key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotKey {
    pub runtime_id: RuntimeId,
    pub recursion_path: RecursionPath,
    pub role: SubsystemRole,
    pub entity: Option<String>,
}

impl SnapshotKey {
    /// Creates a key for a role-level snapshot.
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

    /// Returns a stable string suitable for logs and store diagnostics.
    pub fn stable_id(&self) -> String {
        let path = if self.recursion_path.is_root() {
            "root".to_string()
        } else {
            self.recursion_path.segments().join("/")
        };
        let entity = self.entity.as_deref().unwrap_or("-");
        format!("{}:{path}:{:?}:{entity}", self.runtime_id, self.role)
    }
}

/// A typed snapshot payload plus framework metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRecord<S> {
    pub key: SnapshotKey,
    pub version: SnapshotVersion,
    pub captured_at: DateTime<Utc>,
    pub snapshot: S,
}

impl<S> SnapshotRecord<S> {
    /// Creates a snapshot record captured at the current time.
    pub fn new(key: SnapshotKey, version: SnapshotVersion, snapshot: S) -> Self {
        Self {
            key,
            version,
            captured_at: Utc::now(),
            snapshot,
        }
    }
}
