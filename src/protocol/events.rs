//! Typed event and report records for observation ports.

use crate::roles::ViableSystem;

use super::envelope::ProtocolMetadata;
use super::system1::{
    AuditEvidence, CoordinationView, PerformanceObservation, ResourceShortageRequest,
    UnitDescriptor,
};
use super::system2::{
    CoordinationAcknowledgement, CoordinationConflict, CoordinationEscalation,
    CoordinationIntervention,
};
use super::system3::{
    AuditFinding, AuditResponse, DirectiveAcknowledgement, OperationalDirective,
    OperationalSummary, RemediationAction, ResourceAllocation, ResourceAllocationAcknowledgement,
    ResourceRequest,
};
use super::system4::{
    AdaptationProposal, EnvironmentSourceStatus, EnvironmentalObservation, Forecast,
    ForecastCalibration, IntelligenceAssessment, InterpretedSignal, Scenario,
};

/// Framework event record.
#[derive(Clone)]
pub struct FrameworkEvent {
    pub metadata: ProtocolMetadata,
    pub kind: String,
}

/// Runtime event stream item.
pub enum RuntimeEvent<V>
where
    V: ViableSystem,
{
    Framework(Box<FrameworkEvent>),
    System1(Box<System1Event<V>>),
    System2(Box<System2Event<V>>),
    System3(Box<System3Event<V>>),
    System4(Box<System4Event>),
}

impl<V> Clone for RuntimeEvent<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        match self {
            Self::Framework(event) => Self::Framework(event.clone()),
            Self::System1(event) => Self::System1(Box::new((**event).clone())),
            Self::System2(event) => Self::System2(Box::new((**event).clone())),
            Self::System3(event) => Self::System3(Box::new((**event).clone())),
            Self::System4(event) => Self::System4(event.clone()),
        }
    }
}

/// System 1 event stream item.
pub enum System1Event<V>
where
    V: ViableSystem,
{
    UnitRegistered(UnitDescriptor<V>),
    UnitUnregistered { unit_id: V::UnitId },
    ResourceShortage(Box<ResourceShortageRequest<V>>),
}

impl<V> Clone for System1Event<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        match self {
            Self::UnitRegistered(descriptor) => Self::UnitRegistered(descriptor.clone()),
            Self::UnitUnregistered { unit_id } => Self::UnitUnregistered {
                unit_id: unit_id.clone(),
            },
            Self::ResourceShortage(shortage) => {
                Self::ResourceShortage(Box::new(ResourceShortageRequest {
                    metadata: shortage.metadata.clone(),
                    required_capabilities: shortage.required_capabilities.clone(),
                    work_label: shortage.work_label.clone(),
                    reason: shortage.reason.clone(),
                }))
            }
        }
    }
}

/// Runtime report stream item.
pub enum RuntimeReport<V>
where
    V: ViableSystem,
{
    System1(Box<System1Report<V>>),
    System2(Box<System2Report<V>>),
    System3(Box<System3Report<V>>),
    System4(Box<System4Report>),
}

/// System 1 report stream item.
pub enum System1Report<V>
where
    V: ViableSystem,
{
    Performance(PerformanceObservation<V>),
    Coordination(CoordinationView<V>),
    Audit(AuditEvidence<V>),
}

/// System 2 event stream item.
pub enum System2Event<V>
where
    V: ViableSystem,
{
    CoordinationCycle {
        conflict_count: usize,
        intervention_count: usize,
    },
    InterventionAcknowledged(Box<CoordinationAcknowledgement<V>>),
    ConflictEscalated(Box<CoordinationEscalation<V>>),
}

impl<V> Clone for System2Event<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        match self {
            Self::CoordinationCycle {
                conflict_count,
                intervention_count,
            } => Self::CoordinationCycle {
                conflict_count: *conflict_count,
                intervention_count: *intervention_count,
            },
            Self::InterventionAcknowledged(acknowledgement) => {
                Self::InterventionAcknowledged(Box::new((**acknowledgement).clone()))
            }
            Self::ConflictEscalated(escalation) => {
                Self::ConflictEscalated(Box::new((**escalation).clone()))
            }
        }
    }
}

/// System 2 report stream item.
pub enum System2Report<V>
where
    V: ViableSystem,
{
    Conflict(Box<CoordinationConflict<V>>),
    Intervention(Box<CoordinationIntervention<V>>),
    Acknowledgement(Box<CoordinationAcknowledgement<V>>),
    Escalation(Box<CoordinationEscalation<V>>),
}

/// System 3 event stream item.
pub enum System3Event<V>
where
    V: ViableSystem,
{
    ResourceCycle {
        request_count: usize,
        allocation_count: usize,
        directive_count: usize,
    },
    AllocationAcknowledged(Box<ResourceAllocationAcknowledgement<V>>),
    DirectiveAcknowledged(Box<DirectiveAcknowledgement<V>>),
    DirectiveAcknowledgementFailed(Box<DirectiveAcknowledgement<V>>),
    AuditCompleted {
        audit_id: String,
        finding_count: usize,
        remediation_count: usize,
    },
}

impl<V> Clone for System3Event<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        match self {
            Self::ResourceCycle {
                request_count,
                allocation_count,
                directive_count,
            } => Self::ResourceCycle {
                request_count: *request_count,
                allocation_count: *allocation_count,
                directive_count: *directive_count,
            },
            Self::AllocationAcknowledged(acknowledgement) => {
                Self::AllocationAcknowledged(Box::new((**acknowledgement).clone()))
            }
            Self::DirectiveAcknowledged(acknowledgement) => {
                Self::DirectiveAcknowledged(Box::new((**acknowledgement).clone()))
            }
            Self::DirectiveAcknowledgementFailed(acknowledgement) => {
                Self::DirectiveAcknowledgementFailed(Box::new((**acknowledgement).clone()))
            }
            Self::AuditCompleted {
                audit_id,
                finding_count,
                remediation_count,
            } => Self::AuditCompleted {
                audit_id: audit_id.clone(),
                finding_count: *finding_count,
                remediation_count: *remediation_count,
            },
        }
    }
}

/// System 3 report stream item.
pub enum System3Report<V>
where
    V: ViableSystem,
{
    ResourceRequest(Box<ResourceRequest<V>>),
    Allocation(Box<ResourceAllocation<V>>),
    AllocationAcknowledgement(Box<ResourceAllocationAcknowledgement<V>>),
    Directive(Box<OperationalDirective<V>>),
    DirectiveAcknowledgement(Box<DirectiveAcknowledgement<V>>),
    OperationalSummary(Box<OperationalSummary<V>>),
    AuditFinding(Box<AuditFinding<V>>),
    Remediation(Box<RemediationAction<V>>),
    AuditResponse(Box<AuditResponse<V>>),
}

/// System 4 event stream item.
#[derive(Clone)]
pub enum System4Event {
    SourceRegistered(Box<EnvironmentSourceStatus>),
    SourceObservationFailed(Box<EnvironmentSourceStatus>),
    ObservationsCollected {
        observation_count: usize,
        stale_source_count: usize,
    },
    IntelligenceCycle {
        observation_count: usize,
        signal_count: usize,
        forecast_count: usize,
        scenario_count: usize,
        proposal_count: usize,
    },
    ForecastCalibrated {
        calibration_count: usize,
    },
    AdaptationProposed(Box<AdaptationProposal>),
}

/// System 4 report stream item.
pub enum System4Report {
    SourceStatus(Box<EnvironmentSourceStatus>),
    Observation(Box<EnvironmentalObservation>),
    Signal(Box<InterpretedSignal>),
    Assessment(Box<IntelligenceAssessment>),
    Forecast(Box<Forecast>),
    Scenario(Box<Scenario>),
    Proposal(Box<AdaptationProposal>),
    Calibration(Box<ForecastCalibration>),
}
