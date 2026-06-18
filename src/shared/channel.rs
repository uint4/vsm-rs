use ractor::DerivedActorRef;

use crate::channels;
use crate::channels::broker::{ChannelStats, VsmActorMsg};
use crate::domain::{ChannelKind, VsmMessage};
use crate::error::VsmError;

pub async fn subscribe(channel: ChannelKind, subscriber_id: impl Into<String>, subscriber: DerivedActorRef<VsmActorMsg>) -> Result<(), VsmError> { channels::subscribe(channel, subscriber_id, subscriber).await }
pub async fn unsubscribe(channel: ChannelKind, subscriber_id: impl Into<String>) -> Result<(), VsmError> { channels::unsubscribe(channel, subscriber_id).await }
pub fn publish(channel: ChannelKind, mut message: VsmMessage) -> Result<(), VsmError> { message.channel = channel; channels::publish(message) }
pub async fn stats(channel: ChannelKind) -> Result<ChannelStats, VsmError> { channels::stats(channel).await }
pub async fn list_channels() -> Result<Vec<ChannelKind>, VsmError> { channels::list_channels().await }
