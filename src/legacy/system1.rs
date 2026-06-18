//! Legacy System 1 JSON adapters.

use serde_json::{json, Value};
use thiserror::Error;

use crate::error::{ApplicationFailure, FrameworkError, WorkError};
use crate::protocol::system1::{
    CapabilityDescription, ResourceShortageRequest, UnitDescriptor, WorkRequest, WorkResponse,
};
use crate::roles::ViableSystem;
use crate::system1::{Transaction, TransactionResult, UnitConfig};
use crate::{ChannelKind, MessageKind, SystemId, VsmMessage};

/// Type family for the current JSON-backed System 1 facade.
#[derive(Debug, Clone)]
pub struct LegacyJsonSystem;

/// Application error wrapper for legacy JSON outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct LegacyAppError {
    pub message: String,
}

impl LegacyAppError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl ViableSystem for LegacyJsonSystem {
    type Work = Transaction;
    type Outcome = Value;
    type AppError = LegacyAppError;
    type Capability = String;
    type UnitId = String;
    type UnitSnapshot = Value;
}

/// Converts a current `Transaction` into a typed work request.
pub fn transaction_to_work_request(transaction: Transaction) -> WorkRequest<LegacyJsonSystem> {
    WorkRequest::new(transaction)
}

/// Converts a typed work request back into the current `Transaction`.
pub fn work_request_to_transaction(request: WorkRequest<LegacyJsonSystem>) -> Transaction {
    request.work
}

/// Converts a current `TransactionResult` into a typed work response.
pub fn transaction_result_to_work_response(
    result: TransactionResult,
) -> WorkResponse<LegacyJsonSystem> {
    let result = match result {
        TransactionResult::Ok(value) => Ok(value),
        TransactionResult::InvalidTransaction(reason) => Err(WorkError::Application(
            ApplicationFailure::Rejected(LegacyAppError::new(reason)),
        )),
        TransactionResult::NoSuitableUnit => {
            Err(WorkError::Framework(FrameworkError::Unavailable {
                target: "suitable System 1 unit".to_string(),
            }))
        }
        TransactionResult::UnitUnavailable(unit) => {
            Err(WorkError::Framework(FrameworkError::Unavailable {
                target: unit,
            }))
        }
        TransactionResult::UnitError(message) => Err(WorkError::Application(
            ApplicationFailure::Failed(LegacyAppError::new(message)),
        )),
    };

    WorkResponse {
        metadata: crate::protocol::ProtocolMetadata::new(),
        result,
    }
}

/// Converts a typed work response into the current `TransactionResult`.
pub fn work_response_to_transaction_result(
    response: WorkResponse<LegacyJsonSystem>,
) -> TransactionResult {
    match response.result {
        Ok(value) => TransactionResult::Ok(value),
        Err(WorkError::Application(ApplicationFailure::Rejected(error))) => {
            TransactionResult::InvalidTransaction(error.message)
        }
        Err(WorkError::Application(ApplicationFailure::Failed(error))) => {
            TransactionResult::UnitError(error.message)
        }
        Err(WorkError::Framework(FrameworkError::Unavailable { target })) => {
            TransactionResult::UnitUnavailable(target)
        }
        Err(WorkError::Framework(error)) => TransactionResult::UnitError(error.to_string()),
    }
}

/// Converts current unit configuration into a typed unit descriptor.
pub fn unit_config_to_descriptor(config: UnitConfig) -> UnitDescriptor<LegacyJsonSystem> {
    UnitDescriptor {
        unit_id: config.id,
        capabilities: config
            .capabilities
            .into_iter()
            .map(CapabilityDescription::new)
            .collect(),
        labels: Default::default(),
    }
}

/// Converts a typed unit descriptor into current unit configuration.
pub fn descriptor_to_unit_config(descriptor: UnitDescriptor<LegacyJsonSystem>) -> UnitConfig {
    UnitConfig {
        id: descriptor.unit_id,
        capabilities: descriptor
            .capabilities
            .into_iter()
            .map(|description| description.capability)
            .collect(),
        auto_restart: true,
        metadata: Value::Null,
    }
}

/// Converts a current resource-shortage message into a typed request.
pub fn resource_shortage_from_message(
    message: &VsmMessage,
) -> Result<ResourceShortageRequest<LegacyJsonSystem>, FrameworkError> {
    if message.channel != ChannelKind::ResourceBargain || message.kind != MessageKind::UnitRequest {
        return Err(FrameworkError::InvalidProtocol {
            reason: "expected ResourceBargain/UnitRequest message".to_string(),
        });
    }

    let required_capabilities = message
        .payload
        .get("required_capabilities")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let work_label = message
        .payload
        .get("transaction_type")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let reason = work_label
        .as_ref()
        .map(|kind| format!("no suitable unit for transaction type {kind}"))
        .unwrap_or_else(|| "no suitable unit".to_string());

    Ok(ResourceShortageRequest {
        metadata: crate::protocol::ProtocolMetadata::new(),
        required_capabilities,
        work_label,
        reason,
    })
}

/// Converts a typed resource-shortage request into the current channel message.
pub fn resource_shortage_to_message(
    request: ResourceShortageRequest<LegacyJsonSystem>,
) -> VsmMessage {
    VsmMessage::new(
        SystemId::System1,
        SystemId::System3,
        ChannelKind::ResourceBargain,
        MessageKind::UnitRequest,
        json!({
            "transaction_type": request.work_label,
            "required_capabilities": request.required_capabilities,
            "reason": request.reason,
        }),
    )
}
