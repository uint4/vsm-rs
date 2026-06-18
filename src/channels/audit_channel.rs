use ractor::DerivedActorRef;
use crate::channels::broker::VsmActorMsg;
use crate::error::VsmResult;
use crate::shared::message::{ChannelKind, MessageKind, SystemId, VsmMessage};

pub async fn subscribe(subsystem_id: impl Into<String>, actor: DerivedActorRef<VsmActorMsg>) -> VsmResult<()> {
    crate::channels::subscribe(ChannelKind::Audit, subsystem_id, actor).await
}

pub async fn unsubscribe(subsystem_id: impl Into<String>) -> VsmResult<()> {
    crate::channels::unsubscribe(ChannelKind::Audit, subsystem_id).await
}

pub fn send_message(from: SystemId, to: SystemId, kind: MessageKind, payload: serde_json::Value) -> VsmResult<()> {
    crate::channels::publish(VsmMessage::audit(from, to, kind, payload))
}

pub fn broadcast(from: SystemId, kind: MessageKind, payload: serde_json::Value) -> VsmResult<()> {
    let msg = VsmMessage::audit(from, SystemId::External, kind, payload);
    crate::channels::broadcast(ChannelKind::Audit, msg)
}
