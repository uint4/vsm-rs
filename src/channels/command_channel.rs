//! Broker facade for command-channel messages.
//!
//! Command messages model direction from policy, intelligence, or control
//! toward lower-level operations. In the current runtime System 1 handles
//! `Execute` commands by forwarding the payload to all registered units.

use crate::channels::broker::VsmActorMsg;
use crate::error::VsmResult;
use crate::shared::message::{ChannelKind, MessageKind, SystemId, VsmMessage};
use ractor::DerivedActorRef;

pub async fn subscribe(
    subsystem_id: impl Into<String>,
    actor: DerivedActorRef<VsmActorMsg>,
) -> VsmResult<()> {
    crate::channels::subscribe(ChannelKind::Command, subsystem_id, actor).await
}

pub async fn unsubscribe(subsystem_id: impl Into<String>) -> VsmResult<()> {
    crate::channels::unsubscribe(ChannelKind::Command, subsystem_id).await
}

pub fn send_message(
    from: SystemId,
    to: SystemId,
    kind: MessageKind,
    payload: serde_json::Value,
) -> VsmResult<()> {
    crate::channels::publish(VsmMessage::command(from, to, kind, payload))
}

pub fn broadcast(from: SystemId, kind: MessageKind, payload: serde_json::Value) -> VsmResult<()> {
    let msg = VsmMessage::command(from, SystemId::External, kind, payload);
    crate::channels::broadcast(ChannelKind::Command, msg)
}
