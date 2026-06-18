//! In-memory channel broker actor.
//!
//! `ChannelBroker` owns subscriber maps and newest-first message history for
//! every `ChannelKind`. Targeted publish validates VSM flow rules, routes to
//! the destination subscriber ID, falls back to broadcast when no target is
//! present, and retains the message. Explicit broadcast sends to every current
//! subscriber and currently bypasses the targeted publish validation path.

use std::collections::HashMap;

use ractor::{Actor, ActorProcessingErr, ActorRef, DerivedActorRef, RpcReplyPort};
use serde::{Deserialize, Serialize};

use crate::error::{VsmError, VsmResult};
use crate::prelude::now_json;
use crate::shared::message::{ChannelKind, VsmMessage};

#[derive(Clone)]
pub enum VsmActorMsg {
    ChannelMessage(VsmMessage),
    AlgedonicSignal(VsmMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStats {
    pub channel: ChannelKind,
    pub subscriber_count: usize,
    pub subscribers: Vec<String>,
    pub active: bool,
    pub retained_message_count: usize,
}

pub enum ChannelBrokerMsg {
    Subscribe(ChannelKind, String, DerivedActorRef<VsmActorMsg>, RpcReplyPort<()>),
    Unsubscribe(ChannelKind, String, RpcReplyPort<()>),
    Publish(VsmMessage),
    Broadcast(ChannelKind, VsmMessage),
    Stats(ChannelKind, RpcReplyPort<ChannelStats>),
    ListChannels(RpcReplyPort<Vec<ChannelKind>>),
    History(ChannelKind, RpcReplyPort<Vec<VsmMessage>>),
}

#[derive(Default)]
pub struct ChannelBroker;

pub struct ChannelBrokerState {
    subscribers: HashMap<ChannelKind, HashMap<String, DerivedActorRef<VsmActorMsg>>>,
    messages: HashMap<ChannelKind, Vec<VsmMessage>>,
}

impl ChannelBrokerState {
    fn new() -> Self {
        let mut subscribers = HashMap::new();
        let mut messages = HashMap::new();
        for channel in ChannelKind::ALL {
            subscribers.insert(channel, HashMap::new());
            messages.insert(channel, Vec::new());
        }
        Self { subscribers, messages }
    }

    fn retain(&mut self, message: VsmMessage) {
        let history = self.messages.entry(message.channel).or_default();
        history.insert(0, message);
        history.truncate(10_000);
    }
}

#[ractor::async_trait]
impl Actor for ChannelBroker {
    type Msg = ChannelBrokerMsg;
    type State = ChannelBrokerState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(ChannelBrokerState::new())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            ChannelBrokerMsg::Subscribe(channel, subscriber_id, actor, reply) => {
                state.subscribers.entry(channel).or_default().insert(subscriber_id, actor);
                if !reply.is_closed() { let _ = reply.send(()); }
            }
            ChannelBrokerMsg::Unsubscribe(channel, subscriber_id, reply) => {
                state.subscribers.entry(channel).or_default().remove(&subscriber_id);
                if !reply.is_closed() { let _ = reply.send(()); }
            }
            ChannelBrokerMsg::Publish(message) => {
                if let Err(err) = validate_for_broker(&message) {
                    tracing::warn!(error = %err, ?message, "rejected invalid VSM message");
                    return Ok(());
                }
                if message.kind.is_high_priority() {
                    tracing::warn!(channel = ?message.channel, kind = ?message.kind, id = %message.id, "high-priority channel message");
                }
                route_message(state, message.clone());
                state.retain(message);
            }
            ChannelBrokerMsg::Broadcast(channel, message) => {
                let outbound = if channel == ChannelKind::Algedonic {
                    VsmActorMsg::AlgedonicSignal(message.clone())
                } else {
                    VsmActorMsg::ChannelMessage(message.clone())
                };
                broadcast_to(state, channel, outbound);
                state.retain(message);
            }
            ChannelBrokerMsg::Stats(channel, reply) => {
                let subscribers = state
                    .subscribers
                    .get(&channel)
                    .map(|m| m.keys().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                let stats = ChannelStats {
                    channel,
                    subscriber_count: subscribers.len(),
                    retained_message_count: state.messages.get(&channel).map(Vec::len).unwrap_or(0),
                    subscribers,
                    active: true,
                };
                if !reply.is_closed() { let _ = reply.send(stats); }
            }
            ChannelBrokerMsg::ListChannels(reply) => {
                if !reply.is_closed() { let _ = reply.send(ChannelKind::ALL.to_vec()); }
            }
            ChannelBrokerMsg::History(channel, reply) => {
                let history = state.messages.get(&channel).cloned().unwrap_or_default();
                if !reply.is_closed() { let _ = reply.send(history); }
            }
        }
        Ok(())
    }
}

fn validate_for_broker(message: &VsmMessage) -> VsmResult<()> {
    match message.validate() {
        Ok(()) => Ok(()),
        Err(err)
            if message.from == crate::shared::message::SystemId::External
                || message.to == crate::shared::message::SystemId::External => Ok(()),
        Err(err) => Err(VsmError::Validation(err)),
    }
}

fn route_message(state: &mut ChannelBrokerState, message: VsmMessage) {
    if message.channel == ChannelKind::Algedonic {
        deliver_to(state, message.channel, "system5", VsmActorMsg::AlgedonicSignal(message));
        return;
    }
    let target = message.to.subscriber_id().to_string();
    if !deliver_to(state, message.channel, &target, VsmActorMsg::ChannelMessage(message.clone())) {
        // Fall back to broadcast for channels where Elixir used Registry.dispatch.
        broadcast_to(state, message.channel, VsmActorMsg::ChannelMessage(message));
    }
}

fn deliver_to(
    state: &mut ChannelBrokerState,
    channel: ChannelKind,
    target: &str,
    msg: VsmActorMsg,
) -> bool {
    let Some(subscribers) = state.subscribers.get_mut(&channel) else { return false; };
    let Some(actor) = subscribers.get(target) else { return false; };
    if actor.send_message(msg).is_err() {
        subscribers.remove(target);
        return false;
    }
    true
}

fn broadcast_to(state: &mut ChannelBrokerState, channel: ChannelKind, msg: VsmActorMsg) {
    let Some(subscribers) = state.subscribers.get_mut(&channel) else { return; };
    let mut dead = Vec::new();
    for (id, actor) in subscribers.iter() {
        if actor.send_message(msg.clone()).is_err() {
            dead.push(id.clone());
        }
    }
    for id in dead {
        subscribers.remove(&id);
    }
}
