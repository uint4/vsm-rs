//! Broker facade for audit-channel messages.
//!
//! Audit traffic connects System 1 with System 3 or System 3*. System 1 handles
//! audit requests by publishing a response containing unit IDs, metrics,
//! operational variety, timestamp, and its startup config. Delivery still uses
//! the broker's best-effort semantics.

use crate::channels::broker::VsmActorMsg;
use crate::error::VsmResult;
use crate::shared::message::{ChannelKind, MessageKind, SystemId, VsmMessage};
use ractor::DerivedActorRef;

pub async fn subscribe(
    subsystem_id: impl Into<String>,
    actor: DerivedActorRef<VsmActorMsg>,
) -> VsmResult<()> {
    crate::channels::subscribe(ChannelKind::Audit, subsystem_id, actor).await
}

pub async fn unsubscribe(subsystem_id: impl Into<String>) -> VsmResult<()> {
    crate::channels::unsubscribe(ChannelKind::Audit, subsystem_id).await
}

pub fn send_message(
    from: SystemId,
    to: SystemId,
    kind: MessageKind,
    payload: serde_json::Value,
) -> VsmResult<()> {
    crate::channels::publish(VsmMessage::audit(from, to, kind, payload))
}

pub fn broadcast(from: SystemId, kind: MessageKind, payload: serde_json::Value) -> VsmResult<()> {
    let msg = VsmMessage::audit(from, SystemId::All, kind, payload);
    crate::channels::broadcast(ChannelKind::Audit, msg)
}
