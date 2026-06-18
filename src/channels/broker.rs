//! In-memory channel broker actor.
//!
//! `ChannelBroker` owns subscriber maps and newest-first message history for
//! every `ChannelKind`. Targeted publish validates VSM flow rules, routes to
//! the destination subscriber ID, and reports missing targets explicitly.
//! Explicit broadcast is reserved for `SystemId::All` messages and uses the
//! same validation boundary as targeted publish.

use std::collections::HashMap;

use ractor::{Actor, ActorProcessingErr, ActorRef, DerivedActorRef, RpcReplyPort};
use serde::{Deserialize, Serialize};

use crate::error::{VsmError, VsmResult};
use crate::protocol::{DeliveryMetrics, DeliveryStatus};
use crate::shared::message::{ChannelKind, VsmMessage};

#[derive(Clone)]
pub enum VsmActorMsg {
    ChannelMessage(VsmMessage),
    AlgedonicSignal(VsmMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryOutcome {
    pub status: DeliveryStatus,
    pub channel: ChannelKind,
    pub message_id: String,
    pub target: Option<String>,
    pub delivered_to: usize,
    pub reason: Option<String>,
}

impl DeliveryOutcome {
    pub fn delivered(
        channel: ChannelKind,
        message_id: impl Into<String>,
        target: Option<String>,
        delivered_to: usize,
    ) -> Self {
        Self {
            status: DeliveryStatus::Delivered,
            channel,
            message_id: message_id.into(),
            target,
            delivered_to,
            reason: None,
        }
    }

    pub fn failed(
        status: DeliveryStatus,
        channel: ChannelKind,
        message_id: impl Into<String>,
        target: Option<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            status,
            channel,
            message_id: message_id.into(),
            target,
            delivered_to: 0,
            reason: Some(reason.into()),
        }
    }

    pub fn is_delivered(&self) -> bool {
        self.status.is_delivered()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndeliverableMessage {
    pub message: VsmMessage,
    pub outcome: DeliveryOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStats {
    pub channel: ChannelKind,
    pub subscriber_count: usize,
    pub subscribers: Vec<String>,
    pub active: bool,
    pub retained_message_count: usize,
    pub undeliverable_count: usize,
    pub delivery_metrics: DeliveryMetrics,
}

pub enum ChannelBrokerMsg {
    Subscribe(
        ChannelKind,
        String,
        DerivedActorRef<VsmActorMsg>,
        RpcReplyPort<()>,
    ),
    Unsubscribe(ChannelKind, String, RpcReplyPort<()>),
    Publish(VsmMessage),
    PublishWithOutcome(VsmMessage, RpcReplyPort<DeliveryOutcome>),
    Broadcast(ChannelKind, VsmMessage),
    BroadcastWithOutcome(ChannelKind, VsmMessage, RpcReplyPort<DeliveryOutcome>),
    Stats(ChannelKind, RpcReplyPort<ChannelStats>),
    ListChannels(RpcReplyPort<Vec<ChannelKind>>),
    History(ChannelKind, RpcReplyPort<Vec<VsmMessage>>),
    DeadLetters(ChannelKind, RpcReplyPort<Vec<UndeliverableMessage>>),
}

#[derive(Default)]
pub struct ChannelBroker;

pub struct ChannelBrokerState {
    subscribers: HashMap<ChannelKind, HashMap<String, DerivedActorRef<VsmActorMsg>>>,
    messages: HashMap<ChannelKind, Vec<VsmMessage>>,
    dead_letters: HashMap<ChannelKind, Vec<UndeliverableMessage>>,
    delivery_metrics: HashMap<ChannelKind, DeliveryMetrics>,
}

impl ChannelBrokerState {
    fn new() -> Self {
        let mut subscribers = HashMap::new();
        let mut messages = HashMap::new();
        let mut dead_letters = HashMap::new();
        let mut delivery_metrics = HashMap::new();
        for channel in ChannelKind::ALL {
            subscribers.insert(channel, HashMap::new());
            messages.insert(channel, Vec::new());
            dead_letters.insert(channel, Vec::new());
            delivery_metrics.insert(channel, DeliveryMetrics::default());
        }
        Self {
            subscribers,
            messages,
            dead_letters,
            delivery_metrics,
        }
    }

    fn retain(&mut self, message: VsmMessage) {
        let history = self.messages.entry(message.channel).or_default();
        history.insert(0, message);
        history.truncate(10_000);
    }

    fn record_outcome(&mut self, outcome: &DeliveryOutcome) {
        self.delivery_metrics
            .entry(outcome.channel)
            .or_default()
            .record(outcome.status);
    }

    fn retain_dead_letter(&mut self, message: VsmMessage, outcome: DeliveryOutcome) {
        let dead_letters = self.dead_letters.entry(message.channel).or_default();
        dead_letters.insert(0, UndeliverableMessage { message, outcome });
        dead_letters.truncate(10_000);
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
                state
                    .subscribers
                    .entry(channel)
                    .or_default()
                    .insert(subscriber_id, actor);
                if !reply.is_closed() {
                    let _ = reply.send(());
                }
            }
            ChannelBrokerMsg::Unsubscribe(channel, subscriber_id, reply) => {
                state
                    .subscribers
                    .entry(channel)
                    .or_default()
                    .remove(&subscriber_id);
                if !reply.is_closed() {
                    let _ = reply.send(());
                }
            }
            ChannelBrokerMsg::Publish(message) => {
                let outcome = publish_message(state, message);
                if !outcome.is_delivered() {
                    tracing::warn!(?outcome, "channel publish did not deliver");
                }
            }
            ChannelBrokerMsg::PublishWithOutcome(message, reply) => {
                let outcome = publish_message(state, message);
                if !reply.is_closed() {
                    let _ = reply.send(outcome);
                }
            }
            ChannelBrokerMsg::Broadcast(channel, message) => {
                let outcome = broadcast_message(state, channel, message);
                if !outcome.is_delivered() {
                    tracing::warn!(?outcome, "channel broadcast did not deliver");
                }
            }
            ChannelBrokerMsg::BroadcastWithOutcome(channel, message, reply) => {
                let outcome = broadcast_message(state, channel, message);
                if !reply.is_closed() {
                    let _ = reply.send(outcome);
                }
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
                    undeliverable_count: state
                        .dead_letters
                        .get(&channel)
                        .map(Vec::len)
                        .unwrap_or(0),
                    delivery_metrics: state
                        .delivery_metrics
                        .get(&channel)
                        .cloned()
                        .unwrap_or_default(),
                    subscribers,
                    active: true,
                };
                if !reply.is_closed() {
                    let _ = reply.send(stats);
                }
            }
            ChannelBrokerMsg::ListChannels(reply) => {
                if !reply.is_closed() {
                    let _ = reply.send(ChannelKind::ALL.to_vec());
                }
            }
            ChannelBrokerMsg::History(channel, reply) => {
                let history = state.messages.get(&channel).cloned().unwrap_or_default();
                if !reply.is_closed() {
                    let _ = reply.send(history);
                }
            }
            ChannelBrokerMsg::DeadLetters(channel, reply) => {
                let dead_letters = state
                    .dead_letters
                    .get(&channel)
                    .cloned()
                    .unwrap_or_default();
                if !reply.is_closed() {
                    let _ = reply.send(dead_letters);
                }
            }
        }
        Ok(())
    }
}

fn validate_for_broker(message: &VsmMessage) -> VsmResult<()> {
    match message.validate() {
        Ok(()) => Ok(()),
        Err(_)
            if message.from == crate::shared::message::SystemId::External
                || message.to == crate::shared::message::SystemId::External =>
        {
            Ok(())
        }
        Err(err) => Err(VsmError::Validation(err)),
    }
}

fn validate_for_broadcast(channel: ChannelKind, message: &VsmMessage) -> VsmResult<()> {
    if message.channel != channel {
        return Err(VsmError::Validation(format!(
            "broadcast channel mismatch: envelope={:?}, requested={channel:?}",
            message.channel
        )));
    }

    if message.to != crate::shared::message::SystemId::All {
        return Err(VsmError::Validation(format!(
            "broadcast target must be SystemId::All, found {:?}",
            message.to
        )));
    }

    validate_for_broker(message)
}

fn publish_message(state: &mut ChannelBrokerState, message: VsmMessage) -> DeliveryOutcome {
    let outcome = match validate_for_broker(&message) {
        Ok(()) => {
            if message.kind.is_high_priority() {
                tracing::warn!(channel = ?message.channel, kind = ?message.kind, id = %message.id, "high-priority channel message");
            }
            route_message(state, message.clone())
        }
        Err(err) => DeliveryOutcome::failed(
            DeliveryStatus::RejectedByProtocol,
            message.channel,
            message.id.clone(),
            Some(message.to.subscriber_id().to_string()),
            err.to_string(),
        ),
    };

    state.record_outcome(&outcome);
    if outcome.is_delivered() {
        state.retain(message);
    } else {
        state.retain_dead_letter(message, outcome.clone());
    }
    outcome
}

fn broadcast_message(
    state: &mut ChannelBrokerState,
    channel: ChannelKind,
    message: VsmMessage,
) -> DeliveryOutcome {
    let outcome = match validate_for_broadcast(channel, &message) {
        Ok(()) => {
            let outbound = if channel == ChannelKind::Algedonic {
                VsmActorMsg::AlgedonicSignal(message.clone())
            } else {
                VsmActorMsg::ChannelMessage(message.clone())
            };
            let delivered_to = broadcast_to(state, channel, outbound);
            if delivered_to == 0 {
                DeliveryOutcome::failed(
                    DeliveryStatus::TargetUnavailable,
                    channel,
                    message.id.clone(),
                    Some("broadcast".to_string()),
                    "no subscribers are registered for broadcast",
                )
            } else {
                DeliveryOutcome::delivered(
                    channel,
                    message.id.clone(),
                    Some("broadcast".to_string()),
                    delivered_to,
                )
            }
        }
        Err(err) => DeliveryOutcome::failed(
            DeliveryStatus::RejectedByProtocol,
            channel,
            message.id.clone(),
            Some("broadcast".to_string()),
            err.to_string(),
        ),
    };

    state.record_outcome(&outcome);
    if outcome.is_delivered() {
        state.retain(message);
    } else {
        state.retain_dead_letter(message, outcome.clone());
    }
    outcome
}

fn route_message(state: &mut ChannelBrokerState, message: VsmMessage) -> DeliveryOutcome {
    if message.channel == ChannelKind::Algedonic {
        return deliver_to(
            state,
            message.channel,
            "system5",
            VsmActorMsg::AlgedonicSignal(message),
        );
    }
    let target = message.to.subscriber_id().to_string();
    deliver_to(
        state,
        message.channel,
        &target,
        VsmActorMsg::ChannelMessage(message.clone()),
    )
}

fn deliver_to(
    state: &mut ChannelBrokerState,
    channel: ChannelKind,
    target: &str,
    msg: VsmActorMsg,
) -> DeliveryOutcome {
    let message_id = msg.message_id().to_string();
    let Some(subscribers) = state.subscribers.get_mut(&channel) else {
        return DeliveryOutcome::failed(
            DeliveryStatus::TargetUnavailable,
            channel,
            message_id,
            Some(target.to_string()),
            "channel has no subscriber registry",
        );
    };
    let Some(actor) = subscribers.get(target) else {
        return DeliveryOutcome::failed(
            DeliveryStatus::TargetUnavailable,
            channel,
            message_id,
            Some(target.to_string()),
            "target subscriber is not registered",
        );
    };
    if actor.send_message(msg).is_err() {
        subscribers.remove(target);
        return DeliveryOutcome::failed(
            DeliveryStatus::TargetUnavailable,
            channel,
            message_id,
            Some(target.to_string()),
            "target subscriber actor is unavailable",
        );
    }
    DeliveryOutcome::delivered(channel, message_id, Some(target.to_string()), 1)
}

fn broadcast_to(state: &mut ChannelBrokerState, channel: ChannelKind, msg: VsmActorMsg) -> usize {
    let Some(subscribers) = state.subscribers.get_mut(&channel) else {
        return 0;
    };
    let mut dead = Vec::new();
    let mut delivered = 0;
    for (id, actor) in subscribers.iter() {
        match actor.send_message(msg.clone()) {
            Ok(()) => delivered += 1,
            Err(_) => dead.push(id.clone()),
        }
    }
    for id in dead {
        subscribers.remove(&id);
    }
    delivered
}

impl VsmActorMsg {
    fn message_id(&self) -> &str {
        match self {
            Self::ChannelMessage(message) | Self::AlgedonicSignal(message) => &message.id,
        }
    }
}
