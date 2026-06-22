//! Typed operational-recursion protocol records.
//!
//! These records describe the framework-owned boundary between a parent VSM
//! runtime and child VSM runtimes. Application meaning stays in recursion roles
//! rather than becoming new required `ViableSystem` associated types.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::roles::ViableSystem;

use super::address::{RecursionPath, RuntimeId, VsmAddress};
use super::algedonic::AlgedonicSignalRecord;
use super::envelope::ProtocolMetadata;
use super::system1::{
    CapacitySnapshot, ResourceShortageRequest, UnitDescriptor, WorkRequest, WorkResponse,
};
use super::system3::{OperationalDirective, ResourceRequest, System3ControlCycle};
use super::system4::System4IntelligenceCycle;
use super::variety::VarietyCycle;

/// Framework-level decision for information crossing a recursion boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecursionBoundaryDecision {
    Allow,
    Deny { reason: String },
}

impl RecursionBoundaryDecision {
    /// Creates an allow decision.
    pub fn allow() -> Self {
        Self::Allow
    }

    /// Creates a deny decision with a human-readable reason.
    pub fn deny(reason: impl Into<String>) -> Self {
        Self::Deny {
            reason: reason.into(),
        }
    }

    /// Returns true when the boundary decision allows the action.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }
}

/// Lifecycle status retained for one child runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildRuntimeStatus {
    Starting,
    Running,
    Draining,
    Stopped,
}

/// Descriptor for one runtime registered below its parent.
pub struct ChildRuntimeDescriptor<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub child_id: String,
    pub runtime_id: RuntimeId,
    pub recursion_path: RecursionPath,
    pub unit_descriptor: UnitDescriptor<V>,
    pub capacity: CapacitySnapshot,
    pub registered_at: DateTime<Utc>,
}

impl<V> Clone for ChildRuntimeDescriptor<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            child_id: self.child_id.clone(),
            runtime_id: self.runtime_id.clone(),
            recursion_path: self.recursion_path.clone(),
            unit_descriptor: self.unit_descriptor.clone(),
            capacity: self.capacity.clone(),
            registered_at: self.registered_at,
        }
    }
}

impl<V> ChildRuntimeDescriptor<V>
where
    V: ViableSystem,
{
    /// Creates a descriptor for a child runtime bridge unit.
    pub fn new(
        child_id: impl Into<String>,
        runtime_id: RuntimeId,
        recursion_path: RecursionPath,
        unit_descriptor: UnitDescriptor<V>,
        capacity: CapacitySnapshot,
    ) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            child_id: child_id.into(),
            runtime_id,
            recursion_path,
            unit_descriptor,
            capacity,
            registered_at: Utc::now(),
        }
    }
}

/// Snapshot of one retained child runtime registration.
pub struct ChildRuntimeSnapshot<V>
where
    V: ViableSystem,
{
    pub descriptor: ChildRuntimeDescriptor<V>,
    pub status: ChildRuntimeStatus,
    pub registered_units: usize,
}

impl<V> Clone for ChildRuntimeSnapshot<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            descriptor: self.descriptor.clone(),
            status: self.status,
            registered_units: self.registered_units,
        }
    }
}

/// Work delegated from a parent runtime to a child runtime.
pub struct DelegatedWork<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub delegation_id: String,
    pub child_id: String,
    pub request: WorkRequest<V>,
    pub delegated_at: DateTime<Utc>,
}

impl<V> Clone for DelegatedWork<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            delegation_id: self.delegation_id.clone(),
            child_id: self.child_id.clone(),
            request: self.request.clone(),
            delegated_at: self.delegated_at,
        }
    }
}

impl<V> DelegatedWork<V>
where
    V: ViableSystem,
{
    /// Creates a delegated-work record for a child runtime.
    pub fn new(child_id: impl Into<String>, request: WorkRequest<V>) -> Self {
        Self {
            metadata: request.metadata.child(),
            delegation_id: format!("delegated-work-{}", Uuid::new_v4()),
            child_id: child_id.into(),
            request,
            delegated_at: Utc::now(),
        }
    }
}

/// Response returned from a delegated child-runtime work attempt.
pub struct DelegatedWorkOutcome<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub delegation_id: String,
    pub child_id: String,
    pub response: WorkResponse<V>,
    pub completed_at: DateTime<Utc>,
}

impl<V> DelegatedWorkOutcome<V>
where
    V: ViableSystem,
{
    /// Creates a delegated-work outcome.
    pub fn new(delegation: &DelegatedWork<V>, response: WorkResponse<V>) -> Self {
        Self {
            metadata: response.metadata.child(),
            delegation_id: delegation.delegation_id.clone(),
            child_id: delegation.child_id.clone(),
            response,
            completed_at: Utc::now(),
        }
    }
}

/// Resource escalation crossing from child to parent.
pub struct RecursionResourceEscalation<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub escalation_id: String,
    pub child_id: String,
    pub shortage: ResourceShortageRequest<V>,
    pub parent_request: ResourceRequest<V>,
    pub decision: RecursionBoundaryDecision,
    pub escalated_at: DateTime<Utc>,
}

impl<V> Clone for RecursionResourceEscalation<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            escalation_id: self.escalation_id.clone(),
            child_id: self.child_id.clone(),
            shortage: self.shortage.clone(),
            parent_request: self.parent_request.clone(),
            decision: self.decision.clone(),
            escalated_at: self.escalated_at,
        }
    }
}

/// Algedonic escalation crossing from child to parent.
pub struct RecursionAlgedonicEscalation<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub escalation_id: String,
    pub child_id: String,
    pub signal: AlgedonicSignalRecord<V>,
    pub decision: RecursionBoundaryDecision,
    pub escalated_at: DateTime<Utc>,
}

impl<V> Clone for RecursionAlgedonicEscalation<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            escalation_id: self.escalation_id.clone(),
            child_id: self.child_id.clone(),
            signal: self.signal.clone(),
            decision: self.decision.clone(),
            escalated_at: self.escalated_at,
        }
    }
}

/// Policy directive transduced across a parent/child boundary.
pub struct RecursionPolicyDirective<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub directive_id: String,
    pub child_id: String,
    pub parent_directive: OperationalDirective<V>,
    pub child_directive: Option<OperationalDirective<V>>,
    pub decision: RecursionBoundaryDecision,
    pub transduced_at: DateTime<Utc>,
}

impl<V> Clone for RecursionPolicyDirective<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            directive_id: self.directive_id.clone(),
            child_id: self.child_id.clone(),
            parent_directive: self.parent_directive.clone(),
            child_directive: self.child_directive.clone(),
            decision: self.decision.clone(),
            transduced_at: self.transduced_at,
        }
    }
}

/// Generic intelligence summary retained at a recursion boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecursionIntelligenceSummary {
    pub metadata: ProtocolMetadata,
    pub summary_id: String,
    pub child_id: String,
    pub observation_count: usize,
    pub signal_count: usize,
    pub proposal_count: usize,
    pub summary: Option<String>,
    pub summarized_at: DateTime<Utc>,
}

impl RecursionIntelligenceSummary {
    /// Summarizes a typed System 4 intelligence cycle for a recursion boundary.
    pub fn from_cycle(child_id: impl Into<String>, cycle: &System4IntelligenceCycle) -> Self {
        Self {
            metadata: cycle.metadata.child(),
            summary_id: format!("recursion-intelligence-{}", Uuid::new_v4()),
            child_id: child_id.into(),
            observation_count: cycle.observations.len(),
            signal_count: cycle.signals.len(),
            proposal_count: cycle.proposals.len(),
            summary: cycle.assessment.summary.clone(),
            summarized_at: Utc::now(),
        }
    }
}

/// Performance summary retained for a child runtime boundary.
pub struct RecursionPerformanceSummary<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub child_id: String,
    pub capacity: CapacitySnapshot,
    pub system3_cycle: Option<System3ControlCycle<V>>,
    pub variety_cycle: Option<VarietyCycle<V>>,
    pub summarized_at: DateTime<Utc>,
}

impl<V> Clone for RecursionPerformanceSummary<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            child_id: self.child_id.clone(),
            capacity: self.capacity.clone(),
            system3_cycle: self.system3_cycle.clone(),
            variety_cycle: self.variety_cycle.clone(),
            summarized_at: self.summarized_at,
        }
    }
}

/// Snapshot of the operational-recursion manager.
pub struct RecursionSnapshot<V>
where
    V: ViableSystem,
{
    pub children: Vec<ChildRuntimeSnapshot<V>>,
    pub resource_escalations: Vec<RecursionResourceEscalation<V>>,
    pub algedonic_escalations: Vec<RecursionAlgedonicEscalation<V>>,
    pub policy_directives: Vec<RecursionPolicyDirective<V>>,
    pub intelligence_summaries: Vec<RecursionIntelligenceSummary>,
    pub performance_summaries: Vec<RecursionPerformanceSummary<V>>,
    pub captured_at: DateTime<Utc>,
}

impl<V> Clone for RecursionSnapshot<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            children: self.children.clone(),
            resource_escalations: self.resource_escalations.clone(),
            algedonic_escalations: self.algedonic_escalations.clone(),
            policy_directives: self.policy_directives.clone(),
            intelligence_summaries: self.intelligence_summaries.clone(),
            performance_summaries: self.performance_summaries.clone(),
            captured_at: self.captured_at,
        }
    }
}

/// Converts a child runtime destination into framework metadata.
pub fn child_destination(
    runtime_id: RuntimeId,
    recursion_path: RecursionPath,
    role: super::SubsystemRole,
) -> VsmAddress {
    VsmAddress::new(runtime_id, recursion_path, role)
}
