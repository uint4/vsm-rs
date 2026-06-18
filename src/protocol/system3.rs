//! Typed System 3 control, resource, and audit protocol records.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::roles::ViableSystem;

use super::address::VsmAddress;
use super::envelope::{Priority, ProtocolMetadata};
use super::system1::{AuditEvidence, AuditScope, PerformanceObservation, ResourceShortageRequest};

/// Monotonic version assigned to System 3 directives and allocations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ControlVersion(u64);

impl ControlVersion {
    /// Initial authority version.
    pub const INITIAL: Self = Self(1);

    /// Returns the next version, saturating at `u64::MAX`.
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// Returns the raw version number.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Scope covered by a System 3 authority assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityScope {
    AllOperations,
    ResourceGovernance,
    OperationalControl,
    Audit,
    Custom(String),
}

/// Authority metadata attached to System 3 decisions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlAuthority {
    pub authority_id: String,
    pub issued_by: Option<VsmAddress>,
    pub scope: AuthorityScope,
    pub reason: Option<String>,
}

impl ControlAuthority {
    /// Creates an authority record with generated identity.
    pub fn new(scope: AuthorityScope) -> Self {
        Self {
            authority_id: format!("authority-{}", Uuid::new_v4()),
            issued_by: None,
            scope,
            reason: None,
        }
    }

    /// Adds the issuing framework address.
    pub fn issued_by(mut self, issued_by: VsmAddress) -> Self {
        self.issued_by = Some(issued_by);
        self
    }

    /// Adds a human-readable authority reason.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Typed System 3 resource request.
pub struct ResourceRequest<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub request_id: String,
    pub unit_id: Option<V::UnitId>,
    pub required_capabilities: Vec<V::Capability>,
    pub priority: Priority,
    pub reason: String,
    pub requested_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl<V> Clone for ResourceRequest<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            request_id: self.request_id.clone(),
            unit_id: self.unit_id.clone(),
            required_capabilities: self.required_capabilities.clone(),
            priority: self.priority,
            reason: self.reason.clone(),
            requested_at: self.requested_at,
            expires_at: self.expires_at,
        }
    }
}

impl<V> ResourceRequest<V>
where
    V: ViableSystem,
{
    /// Creates a resource request with generated identity and current timestamp.
    pub fn new(
        required_capabilities: impl IntoIterator<Item = V::Capability>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            request_id: format!("resource-request-{}", Uuid::new_v4()),
            unit_id: None,
            required_capabilities: required_capabilities.into_iter().collect(),
            priority: Priority::Normal,
            reason: reason.into(),
            requested_at: Utc::now(),
            expires_at: None,
        }
    }

    /// Creates a System 3 resource request from a System 1 shortage event.
    pub fn from_shortage(shortage: ResourceShortageRequest<V>) -> Self {
        Self {
            metadata: shortage.metadata,
            request_id: format!("resource-request-{}", Uuid::new_v4()),
            unit_id: None,
            required_capabilities: shortage.required_capabilities,
            priority: Priority::Normal,
            reason: shortage.reason,
            requested_at: Utc::now(),
            expires_at: None,
        }
    }

    /// Sets the unit associated with this request.
    pub fn for_unit(mut self, unit_id: V::UnitId) -> Self {
        self.unit_id = Some(unit_id);
        self
    }

    /// Replaces framework metadata.
    pub fn with_metadata(mut self, metadata: ProtocolMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Resource decision returned by System 3 governance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceDecision {
    Grant,
    Deny,
    CounterOffer,
}

/// Resource allocation or denial issued by System 3.
pub struct ResourceAllocation<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub allocation_id: String,
    pub request_id: String,
    pub target_unit: Option<V::UnitId>,
    pub decision: ResourceDecision,
    pub capabilities: Vec<V::Capability>,
    pub authority: ControlAuthority,
    pub version: ControlVersion,
    pub reason: Option<String>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub requires_ack: bool,
}

impl<V> Clone for ResourceAllocation<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            allocation_id: self.allocation_id.clone(),
            request_id: self.request_id.clone(),
            target_unit: self.target_unit.clone(),
            decision: self.decision,
            capabilities: self.capabilities.clone(),
            authority: self.authority.clone(),
            version: self.version,
            reason: self.reason.clone(),
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            requires_ack: self.requires_ack,
        }
    }
}

impl<V> ResourceAllocation<V>
where
    V: ViableSystem,
{
    /// Creates an allocation decision for a resource request.
    pub fn new(request: &ResourceRequest<V>, decision: ResourceDecision) -> Self {
        Self {
            metadata: request.metadata.child(),
            allocation_id: format!("allocation-{}", Uuid::new_v4()),
            request_id: request.request_id.clone(),
            target_unit: request.unit_id.clone(),
            decision,
            capabilities: request.required_capabilities.clone(),
            authority: ControlAuthority::new(AuthorityScope::ResourceGovernance),
            version: ControlVersion::INITIAL,
            reason: None,
            issued_at: Utc::now(),
            expires_at: request.expires_at,
            requires_ack: true,
        }
    }

    /// Adds a human-readable decision reason.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Marks whether this allocation requires acknowledgement.
    pub fn with_required_ack(mut self, requires_ack: bool) -> Self {
        self.requires_ack = requires_ack;
        self
    }
}

/// Acknowledgement status for System 3 decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAckStatus {
    Accepted,
    Rejected,
    Applied,
    Failed,
    Expired,
}

impl ControlAckStatus {
    /// Returns true when the acknowledgement means the target accepted or applied the decision.
    pub fn is_success(self) -> bool {
        matches!(self, Self::Accepted | Self::Applied)
    }
}

/// Acknowledgement for a resource allocation decision.
pub struct ResourceAllocationAcknowledgement<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub allocation_id: String,
    pub target_unit: Option<V::UnitId>,
    pub status: ControlAckStatus,
    pub reason: Option<String>,
    pub observed_at: DateTime<Utc>,
}

impl<V> Clone for ResourceAllocationAcknowledgement<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            allocation_id: self.allocation_id.clone(),
            target_unit: self.target_unit.clone(),
            status: self.status,
            reason: self.reason.clone(),
            observed_at: self.observed_at,
        }
    }
}

impl<V> ResourceAllocationAcknowledgement<V>
where
    V: ViableSystem,
{
    /// Records an accepted allocation acknowledgement.
    pub fn accepted(allocation: &ResourceAllocation<V>) -> Self {
        Self {
            metadata: allocation.metadata.child(),
            allocation_id: allocation.allocation_id.clone(),
            target_unit: allocation.target_unit.clone(),
            status: ControlAckStatus::Accepted,
            reason: None,
            observed_at: Utc::now(),
        }
    }
}

/// Framework-owned operational directive category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationalDirectiveKind {
    AllocateResources,
    Constrain,
    Drain,
    Resume,
    Stop,
    Remediate,
    Custom(String),
}

/// Operational control directive issued by System 3.
pub struct OperationalDirective<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub directive_id: String,
    pub kind: OperationalDirectiveKind,
    pub target_units: Vec<V::UnitId>,
    pub summary: String,
    pub authority: ControlAuthority,
    pub version: ControlVersion,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub requires_ack: bool,
}

impl<V> Clone for OperationalDirective<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            directive_id: self.directive_id.clone(),
            kind: self.kind.clone(),
            target_units: self.target_units.clone(),
            summary: self.summary.clone(),
            authority: self.authority.clone(),
            version: self.version,
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            requires_ack: self.requires_ack,
        }
    }
}

impl<V> OperationalDirective<V>
where
    V: ViableSystem,
{
    /// Creates a directive with generated identity and acknowledgement required.
    pub fn new(
        kind: OperationalDirectiveKind,
        target_units: impl IntoIterator<Item = V::UnitId>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            directive_id: format!("directive-{}", Uuid::new_v4()),
            kind,
            target_units: target_units.into_iter().collect(),
            summary: summary.into(),
            authority: ControlAuthority::new(AuthorityScope::OperationalControl),
            version: ControlVersion::INITIAL,
            issued_at: Utc::now(),
            expires_at: None,
            requires_ack: true,
        }
    }

    /// Marks whether this directive requires acknowledgement.
    pub fn with_required_ack(mut self, requires_ack: bool) -> Self {
        self.requires_ack = requires_ack;
        self
    }
}

/// Acknowledgement for an operational directive.
pub struct DirectiveAcknowledgement<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub directive_id: String,
    pub unit_id: V::UnitId,
    pub status: ControlAckStatus,
    pub reason: Option<String>,
    pub observed_at: DateTime<Utc>,
}

impl<V> Clone for DirectiveAcknowledgement<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            directive_id: self.directive_id.clone(),
            unit_id: self.unit_id.clone(),
            status: self.status,
            reason: self.reason.clone(),
            observed_at: self.observed_at,
        }
    }
}

impl<V> DirectiveAcknowledgement<V>
where
    V: ViableSystem,
{
    /// Creates an accepted directive acknowledgement.
    pub fn accepted(directive: &OperationalDirective<V>, unit_id: V::UnitId) -> Self {
        Self {
            metadata: directive.metadata.child(),
            directive_id: directive.directive_id.clone(),
            unit_id,
            status: ControlAckStatus::Accepted,
            reason: None,
            observed_at: Utc::now(),
        }
    }

    /// Creates a rejected directive acknowledgement.
    pub fn rejected(
        directive: &OperationalDirective<V>,
        unit_id: V::UnitId,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            metadata: directive.metadata.child(),
            directive_id: directive.directive_id.clone(),
            unit_id,
            status: ControlAckStatus::Rejected,
            reason: Some(reason.into()),
            observed_at: Utc::now(),
        }
    }

    /// Creates a failed directive acknowledgement.
    pub fn failed(
        directive: &OperationalDirective<V>,
        unit_id: V::UnitId,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            metadata: directive.metadata.child(),
            directive_id: directive.directive_id.clone(),
            unit_id,
            status: ControlAckStatus::Failed,
            reason: Some(reason.into()),
            observed_at: Utc::now(),
        }
    }
}

/// Operational summary published by System 3.
pub struct OperationalSummary<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub summary_id: String,
    pub resource_request_count: usize,
    pub allocation_count: usize,
    pub directive_count: usize,
    pub failed_acknowledgement_count: usize,
    pub generated_at: DateTime<Utc>,
    pub affected_units: Vec<V::UnitId>,
}

impl<V> Clone for OperationalSummary<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            summary_id: self.summary_id.clone(),
            resource_request_count: self.resource_request_count,
            allocation_count: self.allocation_count,
            directive_count: self.directive_count,
            failed_acknowledgement_count: self.failed_acknowledgement_count,
            generated_at: self.generated_at,
            affected_units: self.affected_units.clone(),
        }
    }
}

/// One System 3 control cycle.
pub struct System3ControlCycle<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub resource_requests: Vec<ResourceRequest<V>>,
    pub performance: Vec<PerformanceObservation<V>>,
    pub allocations: Vec<ResourceAllocation<V>>,
    pub allocation_acknowledgements: Vec<ResourceAllocationAcknowledgement<V>>,
    pub directives: Vec<OperationalDirective<V>>,
    pub directive_acknowledgements: Vec<DirectiveAcknowledgement<V>>,
    pub summaries: Vec<OperationalSummary<V>>,
}

impl<V> Clone for System3ControlCycle<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            resource_requests: self.resource_requests.clone(),
            performance: self.performance.clone(),
            allocations: self.allocations.clone(),
            allocation_acknowledgements: self.allocation_acknowledgements.clone(),
            directives: self.directives.clone(),
            directive_acknowledgements: self.directive_acknowledgements.clone(),
            summaries: self.summaries.clone(),
        }
    }
}

/// Audit authorization supplied to System 3*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditAuthorization {
    pub authority: ControlAuthority,
    pub approved: bool,
    pub purpose: String,
}

impl AuditAuthorization {
    /// Creates an approved audit authorization.
    pub fn approved(purpose: impl Into<String>) -> Self {
        Self {
            authority: ControlAuthority::new(AuthorityScope::Audit),
            approved: true,
            purpose: purpose.into(),
        }
    }

    /// Creates a rejected audit authorization.
    pub fn rejected(purpose: impl Into<String>) -> Self {
        Self {
            authority: ControlAuthority::new(AuthorityScope::Audit),
            approved: false,
            purpose: purpose.into(),
        }
    }
}

/// Sensitive-data boundary for System 3* evidence collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditDataBoundary {
    pub include_snapshots: bool,
    pub max_evidence_items: Option<usize>,
}

impl AuditDataBoundary {
    /// Creates a boundary that excludes unit snapshots.
    pub fn without_snapshots() -> Self {
        Self {
            include_snapshots: false,
            max_evidence_items: None,
        }
    }

    /// Creates a boundary that allows unit snapshots.
    pub fn with_snapshots() -> Self {
        Self {
            include_snapshots: true,
            max_evidence_items: None,
        }
    }
}

impl Default for AuditDataBoundary {
    fn default() -> Self {
        Self::without_snapshots()
    }
}

/// Typed audit request owned by System 3*.
pub struct System3AuditRequest<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub audit_id: String,
    pub scope: AuditScope<V::UnitId>,
    pub authorization: AuditAuthorization,
    pub boundary: AuditDataBoundary,
    pub requested_at: DateTime<Utc>,
}

impl<V> Clone for System3AuditRequest<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            audit_id: self.audit_id.clone(),
            scope: self.scope.clone(),
            authorization: self.authorization.clone(),
            boundary: self.boundary.clone(),
            requested_at: self.requested_at,
        }
    }
}

impl<V> System3AuditRequest<V>
where
    V: ViableSystem,
{
    /// Creates an authorized audit request.
    pub fn new(scope: AuditScope<V::UnitId>, purpose: impl Into<String>) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            audit_id: format!("audit-{}", Uuid::new_v4()),
            scope,
            authorization: AuditAuthorization::approved(purpose),
            boundary: AuditDataBoundary::default(),
            requested_at: Utc::now(),
        }
    }
}

/// Severity assigned to a System 3* finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// System 3* audit finding.
pub struct AuditFinding<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub finding_id: String,
    pub unit_id: Option<V::UnitId>,
    pub severity: AuditSeverity,
    pub category: String,
    pub summary: String,
    pub observed_at: DateTime<Utc>,
}

impl<V> Clone for AuditFinding<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            finding_id: self.finding_id.clone(),
            unit_id: self.unit_id.clone(),
            severity: self.severity,
            category: self.category.clone(),
            summary: self.summary.clone(),
            observed_at: self.observed_at,
        }
    }
}

impl<V> AuditFinding<V>
where
    V: ViableSystem,
{
    /// Creates a finding with generated identity.
    pub fn new(
        severity: AuditSeverity,
        category: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            finding_id: format!("finding-{}", Uuid::new_v4()),
            unit_id: None,
            severity,
            category: category.into(),
            summary: summary.into(),
            observed_at: Utc::now(),
        }
    }

    /// Associates the finding with a unit.
    pub fn for_unit(mut self, unit_id: V::UnitId) -> Self {
        self.unit_id = Some(unit_id);
        self
    }
}

/// Remediation lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemediationStatus {
    Proposed,
    Accepted,
    InProgress,
    Verified,
    Rejected,
}

/// Remediation proposed by System 3*.
pub struct RemediationAction<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub remediation_id: String,
    pub finding_id: String,
    pub target_units: Vec<V::UnitId>,
    pub summary: String,
    pub status: RemediationStatus,
}

impl<V> Clone for RemediationAction<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            remediation_id: self.remediation_id.clone(),
            finding_id: self.finding_id.clone(),
            target_units: self.target_units.clone(),
            summary: self.summary.clone(),
            status: self.status,
        }
    }
}

impl<V> RemediationAction<V>
where
    V: ViableSystem,
{
    /// Creates a proposed remediation for one finding.
    pub fn new(
        finding_id: impl Into<String>,
        target_units: impl IntoIterator<Item = V::UnitId>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            remediation_id: format!("remediation-{}", Uuid::new_v4()),
            finding_id: finding_id.into(),
            target_units: target_units.into_iter().collect(),
            summary: summary.into(),
            status: RemediationStatus::Proposed,
        }
    }
}

/// System 3* audit response.
pub struct AuditResponse<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub audit_id: String,
    pub findings: Vec<AuditFinding<V>>,
    pub remediations: Vec<RemediationAction<V>>,
    pub completed_at: DateTime<Utc>,
}

impl<V> Clone for AuditResponse<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            audit_id: self.audit_id.clone(),
            findings: self.findings.clone(),
            remediations: self.remediations.clone(),
            completed_at: self.completed_at,
        }
    }
}

impl<V> AuditResponse<V>
where
    V: ViableSystem,
{
    /// Creates an audit response from findings and remediations.
    pub fn new(
        request: &System3AuditRequest<V>,
        findings: Vec<AuditFinding<V>>,
        remediations: Vec<RemediationAction<V>>,
    ) -> Self {
        Self {
            metadata: request.metadata.child(),
            audit_id: request.audit_id.clone(),
            findings,
            remediations,
            completed_at: Utc::now(),
        }
    }
}

/// System 3 runtime snapshot.
pub struct System3Snapshot<V>
where
    V: ViableSystem,
{
    pub resource_requests: Vec<ResourceRequest<V>>,
    pub performance: Vec<PerformanceObservation<V>>,
    pub allocations: Vec<ResourceAllocation<V>>,
    pub allocation_acknowledgements: Vec<ResourceAllocationAcknowledgement<V>>,
    pub directives: Vec<OperationalDirective<V>>,
    pub directive_acknowledgements: Vec<DirectiveAcknowledgement<V>>,
    pub summaries: Vec<OperationalSummary<V>>,
    pub audit_responses: Vec<AuditResponse<V>>,
    pub last_control_cycle_at: Option<DateTime<Utc>>,
    pub last_audit_at: Option<DateTime<Utc>>,
}

impl<V> Clone for System3Snapshot<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            resource_requests: self.resource_requests.clone(),
            performance: self.performance.clone(),
            allocations: self.allocations.clone(),
            allocation_acknowledgements: self.allocation_acknowledgements.clone(),
            directives: self.directives.clone(),
            directive_acknowledgements: self.directive_acknowledgements.clone(),
            summaries: self.summaries.clone(),
            audit_responses: self.audit_responses.clone(),
            last_control_cycle_at: self.last_control_cycle_at,
            last_audit_at: self.last_audit_at,
        }
    }
}

/// Owned audit evidence collected for one System 3* request.
pub struct CollectedAuditEvidence<V>
where
    V: ViableSystem,
{
    pub request: System3AuditRequest<V>,
    pub evidence: Vec<AuditEvidence<V>>,
}
