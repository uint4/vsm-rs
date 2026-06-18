//! Typed event and report records for observation ports.

use crate::roles::ViableSystem;

use super::envelope::ProtocolMetadata;
use super::system1::{
    AuditEvidence, CoordinationView, PerformanceObservation, ResourceShortageRequest,
    UnitDescriptor,
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
}

impl<V> Clone for RuntimeEvent<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        match self {
            Self::Framework(event) => Self::Framework(event.clone()),
            Self::System1(event) => Self::System1(Box::new((**event).clone())),
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
