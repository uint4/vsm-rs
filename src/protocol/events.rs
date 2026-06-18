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
