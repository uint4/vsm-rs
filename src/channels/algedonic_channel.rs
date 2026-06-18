//! Broker facade for VSM algedonic messages.
//!
//! This module publishes `VsmMessage` values on the broker's algedonic channel,
//! targeting System 5 by default. It is distinct from `channels::algedonic`,
//! which is a separate typed signal processor that records descriptive routes
//! but does not deliver those routes through the broker.

use crate::channels::broker::VsmActorMsg;
use crate::error::VsmResult;
use crate::shared::message::{ChannelKind, MessageKind, SystemId, VsmMessage};
use ractor::DerivedActorRef;

pub async fn subscribe(
    subsystem_id: impl Into<String>,
    actor: DerivedActorRef<VsmActorMsg>,
) -> VsmResult<()> {
    crate::channels::subscribe(ChannelKind::Algedonic, subsystem_id, actor).await
}

pub async fn unsubscribe(subsystem_id: impl Into<String>) -> VsmResult<()> {
    crate::channels::unsubscribe(ChannelKind::Algedonic, subsystem_id).await
}

pub fn send_message(
    from: SystemId,
    payload: serde_json::Value,
    kind: MessageKind,
) -> VsmResult<()> {
    crate::channels::publish(VsmMessage::new(
        from,
        SystemId::System5,
        ChannelKind::Algedonic,
        kind,
        payload,
    ))
}

pub fn broadcast(from: SystemId, kind: MessageKind, payload: serde_json::Value) -> VsmResult<()> {
    let msg = VsmMessage::new(from, SystemId::All, ChannelKind::Algedonic, kind, payload);
    crate::channels::broadcast(ChannelKind::Algedonic, msg)
}
