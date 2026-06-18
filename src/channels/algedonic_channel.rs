use ractor::DerivedActorRef;
use crate::channels::broker::VsmActorMsg;
use crate::error::VsmResult;
use crate::shared::message::{ChannelKind, MessageKind, SystemId, VsmMessage};

pub async fn subscribe(subsystem_id: impl Into<String>, actor: DerivedActorRef<VsmActorMsg>) -> VsmResult<()> {
    crate::channels::subscribe(ChannelKind::Algedonic, subsystem_id, actor).await
}

pub async fn unsubscribe(subsystem_id: impl Into<String>) -> VsmResult<()> {
    crate::channels::unsubscribe(ChannelKind::Algedonic, subsystem_id).await
}

pub fn send_message(from: SystemId, payload: serde_json::Value, kind: MessageKind) -> VsmResult<()> {
    crate::channels::publish(VsmMessage::new(from, SystemId::System5, ChannelKind::Algedonic, kind, payload))
}

pub fn broadcast(from: SystemId, kind: MessageKind, payload: serde_json::Value) -> VsmResult<()> {
    let msg = VsmMessage::new(from, SystemId::External, ChannelKind::Algedonic, kind, payload);
    crate::channels::broadcast(ChannelKind::Algedonic, msg)
}
