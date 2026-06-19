//! Typed System 5 policy, identity, values, decision, and crisis records.
//!
//! These records are framework-owned. Applications supply organizational
//! meaning through System 5 roles rather than through crate-provided missions,
//! values, scoring rules, or crisis doctrine.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::roles::ViableSystem;

use super::address::VsmAddress;
use super::envelope::{Priority, ProtocolMetadata};
use super::system3::OperationalSummary;
use super::system4::AdaptationProposal;

/// Monotonic policy version used in System 5 decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PolicyVersion(u64);

impl PolicyVersion {
    /// Initial policy version.
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

/// Monotonic identity document version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdentityVersion(u64);

impl IdentityVersion {
    /// Initial identity version.
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

/// Monotonic values version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValuesVersion(u64);

impl ValuesVersion {
    /// Initial values version.
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

/// Identity document supplied by an application provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityRecord {
    pub metadata: ProtocolMetadata,
    pub identity_id: String,
    pub version: IdentityVersion,
    pub label: String,
    pub purpose: Option<String>,
    pub commitments: Vec<String>,
    pub provenance: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

impl IdentityRecord {
    /// Creates an identity record with generated identity and no mission text.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            identity_id: format!("identity-{}", Uuid::new_v4()),
            version: IdentityVersion::INITIAL,
            label: label.into(),
            purpose: None,
            commitments: Vec::new(),
            provenance: Vec::new(),
            updated_at: Utc::now(),
        }
    }

    /// Adds a purpose supplied by the application.
    pub fn with_purpose(mut self, purpose: impl Into<String>) -> Self {
        self.purpose = Some(purpose.into());
        self
    }

    /// Replaces the identity version.
    pub fn with_version(mut self, version: IdentityVersion) -> Self {
        self.version = version;
        self
    }
}

/// One named value statement supplied by an application provider.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueStatement {
    pub name: String,
    pub priority: f64,
    pub description: Option<String>,
    pub indicators: Vec<String>,
}

impl ValueStatement {
    /// Creates a value statement.
    pub fn new(name: impl Into<String>, priority: f64) -> Self {
        Self {
            name: name.into(),
            priority: priority.clamp(0.0, 1.0),
            description: None,
            indicators: Vec::new(),
        }
    }

    /// Adds a human-readable description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Values document supplied by an application provider.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueSet {
    pub metadata: ProtocolMetadata,
    pub values_id: String,
    pub version: ValuesVersion,
    pub values: Vec<ValueStatement>,
    pub provenance: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

impl ValueSet {
    /// Creates a values document from application-supplied statements.
    pub fn new(values: impl IntoIterator<Item = ValueStatement>) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            values_id: format!("values-{}", Uuid::new_v4()),
            version: ValuesVersion::INITIAL,
            values: values.into_iter().collect(),
            provenance: Vec::new(),
            updated_at: Utc::now(),
        }
    }

    /// Creates an empty values document.
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    /// Replaces the values version.
    pub fn with_version(mut self, version: ValuesVersion) -> Self {
        self.version = version;
        self
    }
}

/// Scope for a System 5 authority assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyAuthorityScope {
    Identity,
    Values,
    Strategy,
    Crisis,
    Directive,
    Escalation,
    Custom(String),
}

/// Authority metadata attached to System 5 decisions and directives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyAuthority {
    pub authority_id: String,
    pub issued_by: Option<VsmAddress>,
    pub scope: PolicyAuthorityScope,
    pub policy_version: PolicyVersion,
    pub rationale: Option<String>,
}

impl PolicyAuthority {
    /// Creates an authority record with generated identity.
    pub fn new(scope: PolicyAuthorityScope) -> Self {
        Self {
            authority_id: format!("policy-authority-{}", Uuid::new_v4()),
            issued_by: None,
            scope,
            policy_version: PolicyVersion::INITIAL,
            rationale: None,
        }
    }

    /// Adds the issuing framework address.
    pub fn issued_by(mut self, issued_by: VsmAddress) -> Self {
        self.issued_by = Some(issued_by);
        self
    }

    /// Adds an authority rationale.
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }
}

/// Policy document metadata associated with decisions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyRecord {
    pub metadata: ProtocolMetadata,
    pub policy_id: String,
    pub version: PolicyVersion,
    pub title: String,
    pub summary: Option<String>,
    pub authority: PolicyAuthority,
    pub updated_at: DateTime<Utc>,
    pub review_at: Option<DateTime<Utc>>,
    pub provenance: Vec<String>,
}

impl PolicyRecord {
    /// Creates a policy record.
    pub fn new(title: impl Into<String>, scope: PolicyAuthorityScope) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            policy_id: format!("policy-{}", Uuid::new_v4()),
            version: PolicyVersion::INITIAL,
            title: title.into(),
            summary: None,
            authority: PolicyAuthority::new(scope),
            updated_at: Utc::now(),
            review_at: None,
            provenance: Vec::new(),
        }
    }
}

/// Evidence category recorded in a decision audit trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionEvidenceKind {
    Identity,
    Values,
    System3Operational,
    System4Future,
    Crisis,
    Application,
    Custom(String),
}

/// Decision evidence with provenance and confidence.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionEvidence {
    pub metadata: ProtocolMetadata,
    pub evidence_id: String,
    pub kind: DecisionEvidenceKind,
    pub summary: String,
    pub provenance: Vec<String>,
    pub confidence: f64,
    pub observed_at: DateTime<Utc>,
}

impl DecisionEvidence {
    /// Creates decision evidence.
    pub fn new(kind: DecisionEvidenceKind, summary: impl Into<String>) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            evidence_id: format!("decision-evidence-{}", Uuid::new_v4()),
            kind,
            summary: summary.into(),
            provenance: Vec::new(),
            confidence: 1.0,
            observed_at: Utc::now(),
        }
    }

    /// Creates evidence from an operational summary.
    pub fn from_operational_summary<V>(summary: &OperationalSummary<V>) -> Self
    where
        V: ViableSystem,
    {
        Self {
            metadata: summary.metadata.child(),
            evidence_id: format!("decision-evidence-{}", Uuid::new_v4()),
            kind: DecisionEvidenceKind::System3Operational,
            summary: format!(
                "System 3 summary {}: {} requests, {} allocations, {} directives, {} failed acknowledgements",
                summary.summary_id,
                summary.resource_request_count,
                summary.allocation_count,
                summary.directive_count,
                summary.failed_acknowledgement_count
            ),
            provenance: vec![format!("system3-summary:{}", summary.summary_id)],
            confidence: 1.0,
            observed_at: summary.generated_at,
        }
    }

    /// Creates evidence from a System 4 adaptation proposal.
    pub fn from_adaptation_proposal(proposal: &AdaptationProposal) -> Self {
        Self {
            metadata: proposal.metadata.child(),
            evidence_id: format!("decision-evidence-{}", Uuid::new_v4()),
            kind: DecisionEvidenceKind::System4Future,
            summary: format!(
                "System 4 proposal {}: {}",
                proposal.proposal_id, proposal.title
            ),
            provenance: proposal.provenance.clone(),
            confidence: (1.0 - proposal.uncertainty).clamp(0.0, 1.0),
            observed_at: proposal.generated_at,
        }
    }

    /// Sets evidence confidence.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

/// Kind of directive issued by System 5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDirectiveKind {
    Strategic,
    OperationalConstraint,
    CrisisResponse,
    Review,
    Escalation,
    Custom(String),
}

/// Directive issued by System 5 policy.
pub struct PolicyDirective<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub directive_id: String,
    pub kind: PolicyDirectiveKind,
    pub target_units: Vec<V::UnitId>,
    pub target: Option<VsmAddress>,
    pub summary: String,
    pub authority: PolicyAuthority,
    pub version: PolicyVersion,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub requires_ack: bool,
}

impl<V> Clone for PolicyDirective<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            directive_id: self.directive_id.clone(),
            kind: self.kind.clone(),
            target_units: self.target_units.clone(),
            target: self.target.clone(),
            summary: self.summary.clone(),
            authority: self.authority.clone(),
            version: self.version,
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            requires_ack: self.requires_ack,
        }
    }
}

impl<V> PolicyDirective<V>
where
    V: ViableSystem,
{
    /// Creates a directive with acknowledgement required.
    pub fn new(kind: PolicyDirectiveKind, summary: impl Into<String>) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            directive_id: format!("policy-directive-{}", Uuid::new_v4()),
            kind,
            target_units: Vec::new(),
            target: None,
            summary: summary.into(),
            authority: PolicyAuthority::new(PolicyAuthorityScope::Directive),
            version: PolicyVersion::INITIAL,
            issued_at: Utc::now(),
            expires_at: None,
            requires_ack: true,
        }
    }

    /// Sets target units.
    pub fn with_target_units(mut self, units: impl IntoIterator<Item = V::UnitId>) -> Self {
        self.target_units = units.into_iter().collect();
        self
    }

    /// Sets target runtime address.
    pub fn with_target(mut self, target: VsmAddress) -> Self {
        self.target = Some(target);
        self
    }

    /// Marks whether this directive requires acknowledgement.
    pub fn with_required_ack(mut self, requires_ack: bool) -> Self {
        self.requires_ack = requires_ack;
        self
    }
}

/// Acknowledgement status for System 5 directives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyAckStatus {
    Accepted,
    Rejected,
    Applied,
    Failed,
    Expired,
}

impl PolicyAckStatus {
    /// Returns true when the acknowledgement means the directive was accepted or applied.
    pub fn is_success(self) -> bool {
        matches!(self, Self::Accepted | Self::Applied)
    }
}

/// Acknowledgement for a System 5 directive.
pub struct PolicyDirectiveAcknowledgement<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub directive_id: String,
    pub target_unit: Option<V::UnitId>,
    pub target: Option<VsmAddress>,
    pub status: PolicyAckStatus,
    pub reason: Option<String>,
    pub observed_at: DateTime<Utc>,
}

impl<V> Clone for PolicyDirectiveAcknowledgement<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            directive_id: self.directive_id.clone(),
            target_unit: self.target_unit.clone(),
            target: self.target.clone(),
            status: self.status,
            reason: self.reason.clone(),
            observed_at: self.observed_at,
        }
    }
}

impl<V> PolicyDirectiveAcknowledgement<V>
where
    V: ViableSystem,
{
    /// Creates an accepted directive acknowledgement.
    pub fn accepted(directive: &PolicyDirective<V>) -> Self {
        Self {
            metadata: directive.metadata.child(),
            directive_id: directive.directive_id.clone(),
            target_unit: directive.target_units.first().cloned(),
            target: directive.target.clone(),
            status: PolicyAckStatus::Accepted,
            reason: None,
            observed_at: Utc::now(),
        }
    }

    /// Creates a failed directive acknowledgement.
    pub fn failed(directive: &PolicyDirective<V>, reason: impl Into<String>) -> Self {
        Self {
            metadata: directive.metadata.child(),
            directive_id: directive.directive_id.clone(),
            target_unit: directive.target_units.first().cloned(),
            target: directive.target.clone(),
            status: PolicyAckStatus::Failed,
            reason: Some(reason.into()),
            observed_at: Utc::now(),
        }
    }
}

/// One alternative considered by a decision policy.
pub struct DecisionAlternative<V>
where
    V: ViableSystem,
{
    pub alternative_id: String,
    pub summary: String,
    pub expected_capabilities: Vec<V::Capability>,
    pub target_units: Vec<V::UnitId>,
    pub directives: Vec<PolicyDirective<V>>,
    pub confidence: f64,
    pub rationale: Option<String>,
}

impl<V> Clone for DecisionAlternative<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            alternative_id: self.alternative_id.clone(),
            summary: self.summary.clone(),
            expected_capabilities: self.expected_capabilities.clone(),
            target_units: self.target_units.clone(),
            directives: self.directives.clone(),
            confidence: self.confidence,
            rationale: self.rationale.clone(),
        }
    }
}

impl<V> DecisionAlternative<V>
where
    V: ViableSystem,
{
    /// Creates a decision alternative.
    pub fn new(summary: impl Into<String>) -> Self {
        Self {
            alternative_id: format!("decision-alternative-{}", Uuid::new_v4()),
            summary: summary.into(),
            expected_capabilities: Vec::new(),
            target_units: Vec::new(),
            directives: Vec::new(),
            confidence: 1.0,
            rationale: None,
        }
    }

    /// Adds a directive associated with this alternative.
    pub fn with_directive(mut self, directive: PolicyDirective<V>) -> Self {
        self.directives.push(directive);
        self
    }

    /// Adds a rationale.
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }
}

/// Result of evaluating identity and values for a decision request.
#[derive(Debug, Clone, PartialEq)]
pub struct ValuesEvaluation {
    pub metadata: ProtocolMetadata,
    pub evaluation_id: String,
    pub identity_version: IdentityVersion,
    pub values_version: ValuesVersion,
    pub score: f64,
    pub aligned: bool,
    pub findings: Vec<String>,
    pub rationale: Option<String>,
    pub evaluated_at: DateTime<Utc>,
}

impl ValuesEvaluation {
    /// Creates a neutral no-op evaluation.
    pub fn neutral(identity: &IdentityRecord, values: &ValueSet) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            evaluation_id: format!("values-evaluation-{}", Uuid::new_v4()),
            identity_version: identity.version,
            values_version: values.version,
            score: 1.0,
            aligned: true,
            findings: Vec::new(),
            rationale: Some("no values evaluator configured".to_string()),
            evaluated_at: Utc::now(),
        }
    }
}

/// Decision request sent to System 5.
pub struct DecisionRequest<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub request_id: String,
    pub subject: String,
    pub summary: Option<String>,
    pub policy_area: Option<String>,
    pub priority: Priority,
    pub alternatives: Vec<DecisionAlternative<V>>,
    pub evidence: Vec<DecisionEvidence>,
    pub operational_summaries: Vec<OperationalSummary<V>>,
    pub adaptation_proposals: Vec<AdaptationProposal>,
    pub requested_at: DateTime<Utc>,
    pub review_at: Option<DateTime<Utc>>,
}

impl<V> Clone for DecisionRequest<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            request_id: self.request_id.clone(),
            subject: self.subject.clone(),
            summary: self.summary.clone(),
            policy_area: self.policy_area.clone(),
            priority: self.priority,
            alternatives: self.alternatives.clone(),
            evidence: self.evidence.clone(),
            operational_summaries: self.operational_summaries.clone(),
            adaptation_proposals: self.adaptation_proposals.clone(),
            requested_at: self.requested_at,
            review_at: self.review_at,
        }
    }
}

impl<V> DecisionRequest<V>
where
    V: ViableSystem,
{
    /// Creates a decision request.
    pub fn new(subject: impl Into<String>) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            request_id: format!("decision-request-{}", Uuid::new_v4()),
            subject: subject.into(),
            summary: None,
            policy_area: None,
            priority: Priority::Normal,
            alternatives: Vec::new(),
            evidence: Vec::new(),
            operational_summaries: Vec::new(),
            adaptation_proposals: Vec::new(),
            requested_at: Utc::now(),
            review_at: None,
        }
    }

    /// Adds an alternative.
    pub fn with_alternative(mut self, alternative: DecisionAlternative<V>) -> Self {
        self.alternatives.push(alternative);
        self
    }

    /// Adds decision evidence.
    pub fn with_evidence(mut self, evidence: DecisionEvidence) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Adds a System 3 operational summary.
    pub fn with_operational_summary(mut self, summary: OperationalSummary<V>) -> Self {
        self.operational_summaries.push(summary);
        self
    }

    /// Adds a System 4 adaptation proposal.
    pub fn with_adaptation_proposal(mut self, proposal: AdaptationProposal) -> Self {
        self.adaptation_proposals.push(proposal);
        self
    }
}

/// Decision lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionStatus {
    Proposed,
    Approved,
    Rejected,
    Deferred,
    Escalated,
    Crisis,
}

/// Escalation record for decisions outside local authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyEscalation {
    pub metadata: ProtocolMetadata,
    pub escalation_id: String,
    pub reason: String,
    pub target: Option<VsmAddress>,
    pub requires_parent: bool,
    pub created_at: DateTime<Utc>,
}

impl PolicyEscalation {
    /// Creates an escalation record.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            escalation_id: format!("policy-escalation-{}", Uuid::new_v4()),
            reason: reason.into(),
            target: None,
            requires_parent: false,
            created_at: Utc::now(),
        }
    }

    /// Marks the escalation as requiring a parent recursion boundary.
    pub fn to_parent(mut self, target: VsmAddress) -> Self {
        self.target = Some(target);
        self.requires_parent = true;
        self
    }
}

/// Complete decision audit record.
pub struct DecisionRecord<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub decision_id: String,
    pub request_id: String,
    pub subject: String,
    pub status: DecisionStatus,
    pub selected: Option<DecisionAlternative<V>>,
    pub alternatives: Vec<DecisionAlternative<V>>,
    pub evidence: Vec<DecisionEvidence>,
    pub evaluation: Option<ValuesEvaluation>,
    pub authority: PolicyAuthority,
    pub rationale: String,
    pub directives: Vec<PolicyDirective<V>>,
    pub escalations: Vec<PolicyEscalation>,
    pub decided_at: DateTime<Utc>,
    pub review_at: Option<DateTime<Utc>>,
    pub identity_version: IdentityVersion,
    pub values_version: ValuesVersion,
    pub policy_version: PolicyVersion,
}

impl<V> Clone for DecisionRecord<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            decision_id: self.decision_id.clone(),
            request_id: self.request_id.clone(),
            subject: self.subject.clone(),
            status: self.status,
            selected: self.selected.clone(),
            alternatives: self.alternatives.clone(),
            evidence: self.evidence.clone(),
            evaluation: self.evaluation.clone(),
            authority: self.authority.clone(),
            rationale: self.rationale.clone(),
            directives: self.directives.clone(),
            escalations: self.escalations.clone(),
            decided_at: self.decided_at,
            review_at: self.review_at,
            identity_version: self.identity_version,
            values_version: self.values_version,
            policy_version: self.policy_version,
        }
    }
}

impl<V> DecisionRecord<V>
where
    V: ViableSystem,
{
    /// Creates a decision record for a request.
    pub fn new(
        request: &DecisionRequest<V>,
        identity: &IdentityRecord,
        values: &ValueSet,
        status: DecisionStatus,
        rationale: impl Into<String>,
    ) -> Self {
        let authority = PolicyAuthority::new(PolicyAuthorityScope::Strategy);
        Self {
            metadata: request.metadata.child(),
            decision_id: format!("decision-{}", Uuid::new_v4()),
            request_id: request.request_id.clone(),
            subject: request.subject.clone(),
            status,
            selected: None,
            alternatives: request.alternatives.clone(),
            evidence: request.evidence.clone(),
            evaluation: None,
            authority: authority.clone(),
            rationale: rationale.into(),
            directives: Vec::new(),
            escalations: Vec::new(),
            decided_at: Utc::now(),
            review_at: request.review_at,
            identity_version: identity.version,
            values_version: values.version,
            policy_version: authority.policy_version,
        }
    }

    /// Adds an evaluation to the decision record.
    pub fn with_evaluation(mut self, evaluation: ValuesEvaluation) -> Self {
        self.identity_version = evaluation.identity_version;
        self.values_version = evaluation.values_version;
        self.evaluation = Some(evaluation);
        self
    }

    /// Selects an alternative and copies its directives.
    pub fn with_selected(mut self, selected: DecisionAlternative<V>) -> Self {
        self.directives = selected.directives.clone();
        self.selected = Some(selected);
        self
    }
}

/// Severity of a crisis signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrisisSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Crisis signal delivered to System 5.
#[derive(Debug, Clone, PartialEq)]
pub struct CrisisSignal {
    pub metadata: ProtocolMetadata,
    pub signal_id: String,
    pub severity: CrisisSeverity,
    pub source: Option<VsmAddress>,
    pub summary: String,
    pub evidence: Vec<DecisionEvidence>,
    pub raised_at: DateTime<Utc>,
}

impl CrisisSignal {
    /// Creates a crisis signal.
    pub fn new(severity: CrisisSeverity, summary: impl Into<String>) -> Self {
        Self {
            metadata: ProtocolMetadata::new(),
            signal_id: format!("crisis-signal-{}", Uuid::new_v4()),
            severity,
            source: None,
            summary: summary.into(),
            evidence: Vec::new(),
            raised_at: Utc::now(),
        }
    }

    /// Marks the signal source.
    pub fn from_source(mut self, source: VsmAddress) -> Self {
        self.source = Some(source);
        self
    }

    /// Adds evidence.
    pub fn with_evidence(mut self, evidence: DecisionEvidence) -> Self {
        self.evidence.push(evidence);
        self
    }
}

/// Crisis response produced by System 5.
pub struct CrisisResponse<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub response_id: String,
    pub signal_id: String,
    pub decision: DecisionRecord<V>,
    pub directives: Vec<PolicyDirective<V>>,
    pub escalations: Vec<PolicyEscalation>,
    pub responded_at: DateTime<Utc>,
}

impl<V> Clone for CrisisResponse<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            response_id: self.response_id.clone(),
            signal_id: self.signal_id.clone(),
            decision: self.decision.clone(),
            directives: self.directives.clone(),
            escalations: self.escalations.clone(),
            responded_at: self.responded_at,
        }
    }
}

impl<V> CrisisResponse<V>
where
    V: ViableSystem,
{
    /// Creates a crisis response.
    pub fn new(signal: &CrisisSignal, decision: DecisionRecord<V>) -> Self {
        Self {
            metadata: signal.metadata.child(),
            response_id: format!("crisis-response-{}", Uuid::new_v4()),
            signal_id: signal.signal_id.clone(),
            directives: decision.directives.clone(),
            escalations: decision.escalations.clone(),
            decision,
            responded_at: Utc::now(),
        }
    }
}

/// Result of one System 5 decision cycle.
pub struct System5DecisionCycle<V>
where
    V: ViableSystem,
{
    pub metadata: ProtocolMetadata,
    pub request: DecisionRequest<V>,
    pub identity: IdentityRecord,
    pub values: ValueSet,
    pub evaluation: ValuesEvaluation,
    pub decision: DecisionRecord<V>,
    pub directive_acknowledgements: Vec<PolicyDirectiveAcknowledgement<V>>,
    pub escalations: Vec<PolicyEscalation>,
    pub decided_at: DateTime<Utc>,
}

impl<V> Clone for System5DecisionCycle<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            request: self.request.clone(),
            identity: self.identity.clone(),
            values: self.values.clone(),
            evaluation: self.evaluation.clone(),
            decision: self.decision.clone(),
            directive_acknowledgements: self.directive_acknowledgements.clone(),
            escalations: self.escalations.clone(),
            decided_at: self.decided_at,
        }
    }
}

/// Snapshot of the typed System 5 runtime.
pub struct System5Snapshot<V>
where
    V: ViableSystem,
{
    pub identity: Option<IdentityRecord>,
    pub values: Option<ValueSet>,
    pub decisions: Vec<DecisionRecord<V>>,
    pub directives: Vec<PolicyDirective<V>>,
    pub directive_acknowledgements: Vec<PolicyDirectiveAcknowledgement<V>>,
    pub crises: Vec<CrisisResponse<V>>,
    pub escalations: Vec<PolicyEscalation>,
    pub last_decision_at: Option<DateTime<Utc>>,
}

impl<V> Clone for System5Snapshot<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            identity: self.identity.clone(),
            values: self.values.clone(),
            decisions: self.decisions.clone(),
            directives: self.directives.clone(),
            directive_acknowledgements: self.directive_acknowledgements.clone(),
            crises: self.crises.clone(),
            escalations: self.escalations.clone(),
            last_decision_at: self.last_decision_at,
        }
    }
}
