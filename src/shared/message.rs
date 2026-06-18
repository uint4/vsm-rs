//! Convenience constructors and serialization helpers for VSM messages.
//!
//! The helpers wrap `VsmMessage` construction, validation, reply generation,
//! JSON serialization, and broker publication. `send` publishes asynchronously
//! through the broker and returns the message that was enqueued, not proof that
//! a subscriber processed it.

use serde_json::Value;

use crate::channels;
pub use crate::domain::{ChannelKind, MessageKind, SystemId, VsmMessage};
use crate::error::VsmError;

pub fn new(
    from: SystemId,
    to: SystemId,
    channel: ChannelKind,
    kind: MessageKind,
    payload: Value,
) -> VsmMessage {
    VsmMessage::new(from, to, channel, kind, payload)
}

pub fn command(from: SystemId, to: SystemId, kind: MessageKind, payload: Value) -> VsmMessage {
    VsmMessage::command(from, to, kind, payload)
}
pub fn algedonic(from: SystemId, payload: Value) -> VsmMessage {
    VsmMessage::algedonic(from, payload)
}
pub fn coordination(from: SystemId, to: SystemId, kind: MessageKind, payload: Value) -> VsmMessage {
    VsmMessage::coordination(from, to, kind, payload)
}
pub fn audit(from: SystemId, to: SystemId, kind: MessageKind, payload: Value) -> VsmMessage {
    VsmMessage::audit(from, to, kind, payload)
}
pub fn resource_bargain(
    from: SystemId,
    to: SystemId,
    kind: MessageKind,
    payload: Value,
) -> VsmMessage {
    VsmMessage::resource_bargain(from, to, kind, payload)
}

pub fn send(
    from: SystemId,
    to: SystemId,
    channel: ChannelKind,
    kind: MessageKind,
    payload: Value,
) -> Result<VsmMessage, VsmError> {
    let message = VsmMessage::new(from, to, channel, kind, payload);
    channels::publish(message.clone())?;
    Ok(message)
}

pub fn reply(original: &VsmMessage, kind: MessageKind, payload: Value) -> VsmMessage {
    original.reply(kind, payload)
}
pub fn serialize(message: &VsmMessage) -> Result<Value, VsmError> {
    serde_json::to_value(message).map_err(VsmError::from)
}
pub fn deserialize(data: Value) -> Result<VsmMessage, VsmError> {
    serde_json::from_value(data).map_err(VsmError::from)
}
pub fn valid(message: &VsmMessage) -> bool {
    message.validate_basic_flow().is_ok()
}
pub fn validate(message: &VsmMessage) -> Result<(), VsmError> {
    message.validate_basic_flow().map_err(VsmError::Validation)
}
