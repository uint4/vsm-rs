//! Broker facade for coordination-channel messages.
//!
//! Coordination traffic is valid between System 1 and System 2. System 1
//! currently handles typed coordination payloads for state synchronization and
//! load balancing; System 2 records incoming channel messages in service
//! history unless called through its explicit service API.

use ractor::DerivedActorRef;
use crate::channels::broker::VsmActorMsg;
use crate::error::VsmResult;
use crate::shared::message::{ChannelKind, MessageKind, SystemId, VsmMessage};

pub async fn subscribe(subsystem_id: impl Into<String>, actor: DerivedActorRef<VsmActorMsg>) -> VsmResult<()> {
    crate::channels::subscribe(ChannelKind::Coordination, subsystem_id, actor).await
}

pub async fn unsubscribe(subsystem_id: impl Into<String>) -> VsmResult<()> {
    crate::channels::unsubscribe(ChannelKind::Coordination, subsystem_id).await
}

pub fn send_message(from: SystemId, to: SystemId, kind: MessageKind, payload: serde_json::Value) -> VsmResult<()> {
    crate::channels::publish(VsmMessage::coordination(from, to, kind, payload))
}

pub fn broadcast(from: SystemId, kind: MessageKind, payload: serde_json::Value) -> VsmResult<()> {
    let msg = VsmMessage::coordination(from, SystemId::External, kind, payload);
    crate::channels::broadcast(ChannelKind::Coordination, msg)
}
