//! Typed event and report records for observation ports.

use crate::roles::ViableSystem;

use super::envelope::ProtocolMetadata;
use super::system1::{
    AuditEvidence, CoordinationView, PerformanceObservation, ResourceShortageRequest,
    UnitDescriptor,
};

/// Framework event record.
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

/// System 1 event stream item.
pub enum System1Event<V>
where
    V: ViableSystem,
{
    UnitRegistered(UnitDescriptor<V>),
    UnitUnregistered { unit_id: V::UnitId },
    ResourceShortage(Box<ResourceShortageRequest<V>>),
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
