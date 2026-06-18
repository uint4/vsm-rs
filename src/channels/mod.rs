//! Brokered channels for inter- and intra-subsystem communication.
//!
//! The channel layer stores in-memory subscriptions and message history, routes
//! `VsmMessage` values by `ChannelKind` and subscriber ID, and provides thin
//! channel-specific facades. Delivery is asynchronous, best effort, and not
//! acknowledged by receivers. Systems 2-5 currently record channel events in
//! service history, while System 1 handles selected command, coordination, and
//! audit messages.

pub mod algedonic;
pub mod algedonic_channel;
pub mod audit_channel;
pub mod broker;
pub mod command_channel;
pub mod coordination_channel;
pub mod supervisor;
pub mod temporal;
pub mod temporal_variety;

use ractor::{call_t, ActorRef, DerivedActorRef};
use serde_json::Value;

use crate::channels::broker::{ChannelBrokerMsg, ChannelStats, VsmActorMsg};
use crate::error::{VsmError, VsmResult};
use crate::names;
use crate::shared::message::{ChannelKind, VsmMessage};

pub fn broker_ref() -> VsmResult<ActorRef<ChannelBrokerMsg>> {
    ActorRef::<ChannelBrokerMsg>::where_is(names::CHANNEL_BROKER.to_string())
        .ok_or_else(|| VsmError::ActorUnavailable(names::CHANNEL_BROKER.to_string()))
}

pub async fn subscribe(
    channel: ChannelKind,
    subscriber_id: impl Into<String>,
    actor: DerivedActorRef<VsmActorMsg>,
) -> VsmResult<()> {
    let broker = broker_ref()?;
    call_t!(
        broker,
        ChannelBrokerMsg::Subscribe,
        2_000,
        channel,
        subscriber_id.into(),
        actor
    )
    .map_err(|err| VsmError::Channel(err.to_string()))?;
    Ok(())
}

pub async fn unsubscribe(channel: ChannelKind, subscriber_id: impl Into<String>) -> VsmResult<()> {
    let broker = broker_ref()?;
    call_t!(
        broker,
        ChannelBrokerMsg::Unsubscribe,
        2_000,
        channel,
        subscriber_id.into()
    )
    .map_err(|err| VsmError::Channel(err.to_string()))?;
    Ok(())
}

pub fn publish(message: VsmMessage) -> VsmResult<()> {
    broker_ref()?
        .send_message(ChannelBrokerMsg::Publish(message))
        .map_err(|err| VsmError::Channel(err.to_string()))
}

pub fn broadcast(channel: ChannelKind, message: VsmMessage) -> VsmResult<()> {
    broker_ref()?
        .send_message(ChannelBrokerMsg::Broadcast(channel, message))
        .map_err(|err| VsmError::Channel(err.to_string()))
}

pub async fn stats(channel: ChannelKind) -> VsmResult<ChannelStats> {
    let broker = broker_ref()?;
    call_t!(broker, ChannelBrokerMsg::Stats, 2_000, channel)
        .map_err(|err| VsmError::Channel(err.to_string()))
}

pub async fn subscribers(channel: ChannelKind) -> VsmResult<Vec<String>> {
    Ok(stats(channel).await?.subscribers)
}

pub async fn list_channels() -> VsmResult<Vec<ChannelKind>> {
    let broker = broker_ref()?;
    call_t!(broker, ChannelBrokerMsg::ListChannels, 2_000)
        .map_err(|err| VsmError::Channel(err.to_string()))
}

pub async fn history(channel: ChannelKind) -> VsmResult<Vec<VsmMessage>> {
    let broker = broker_ref()?;
    call_t!(broker, ChannelBrokerMsg::History, 2_000, channel)
        .map_err(|err| VsmError::Channel(err.to_string()))
}

pub fn json_message(channel: ChannelKind, payload: Value) -> VsmMessage {
    use crate::shared::message::{MessageKind, SystemId};
    VsmMessage::new(
        SystemId::External,
        SystemId::External,
        channel,
        MessageKind::Other("json".into()),
        payload,
    )
}
