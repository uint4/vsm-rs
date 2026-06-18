//! Typed control-bus records and delivery outcomes.

use serde::{Deserialize, Serialize};

use crate::roles::ViableSystem;

use super::system1::{AuditRequest, UnitCommand, WorkRequest};
use super::system2::{CoordinationAcknowledgement, CoordinationViewRecord};

/// Delivery result for a typed control-path message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Delivered,
    TargetUnavailable,
    RejectedByProtocol,
    DeadlineExpired,
    Backpressured,
    RuntimeShuttingDown,
}

impl DeliveryStatus {
    /// Returns true when the target accepted the message for processing.
    pub fn is_delivered(self) -> bool {
        self == Self::Delivered
    }
}

/// Cumulative delivery counters for a bus or channel boundary.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryMetrics {
    pub delivered: usize,
    pub target_unavailable: usize,
    pub rejected_by_protocol: usize,
    pub deadline_expired: usize,
    pub backpressured: usize,
    pub runtime_shutting_down: usize,
}

impl DeliveryMetrics {
    /// Records one delivery status.
    pub fn record(&mut self, status: DeliveryStatus) {
        match status {
            DeliveryStatus::Delivered => self.delivered += 1,
            DeliveryStatus::TargetUnavailable => self.target_unavailable += 1,
            DeliveryStatus::RejectedByProtocol => self.rejected_by_protocol += 1,
            DeliveryStatus::DeadlineExpired => self.deadline_expired += 1,
            DeliveryStatus::Backpressured => self.backpressured += 1,
            DeliveryStatus::RuntimeShuttingDown => self.runtime_shutting_down += 1,
        }
    }
}

/// Canonical typed control message family for runtime-owned protocols.
pub enum RuntimeControlMessage<V>
where
    V: ViableSystem,
{
    System1(Box<System1ControlMessage<V>>),
    System2(Box<System2ControlMessage<V>>),
}

/// Typed System 1 control messages.
pub enum System1ControlMessage<V>
where
    V: ViableSystem,
{
    Work(Box<WorkRequest<V>>),
    UnitCommand(UnitCommand<V>),
    AuditRequest(AuditRequest<V>),
}

/// Typed System 2 control messages.
pub enum System2ControlMessage<V>
where
    V: ViableSystem,
{
    CoordinationViews(Vec<CoordinationViewRecord<V>>),
    InterventionAcknowledgement(Box<CoordinationAcknowledgement<V>>),
}
