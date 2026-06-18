//! Core VSM message, channel, and subsystem vocabulary.
//!
//! `VsmMessage` is the brokered inter-system envelope used by channel facades
//! and actors. The domain model preserves the Elixir-compatible serialized
//! `type` field for message kind and enforces the current internal flow matrix
//! for command, coordination, audit, resource-bargain, algedonic, and temporal
//! variety messages. External messages and broadcasts are intentionally more
//! permissive.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemId {
    All,
    External,
    System1,
    System2,
    System3,
    System3Star,
    System4,
    System5,
    RateLimiter,
    Telemetry,
    TemporalVariety,
    Algedonic,
    Goldrush,
    StarterSystem,
    EcosystemTest,
}

impl SystemId {
    pub fn subscriber_id(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::External => "external",
            Self::System1 => "system1",
            Self::System2 => "system2",
            Self::System3 => "system3",
            Self::System3Star => "system3_star",
            Self::System4 => "system4",
            Self::System5 => "system5",
            Self::RateLimiter => "rate_limiter",
            Self::Telemetry => "telemetry",
            Self::TemporalVariety => "temporal_variety",
            Self::Algedonic => "algedonic",
            Self::Goldrush => "goldrush",
            Self::StarterSystem => "starter_system",
            Self::EcosystemTest => "ecosystem_test",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    Command,
    Coordination,
    Audit,
    Algedonic,
    ResourceBargain,
    TemporalVariety,
}

impl ChannelKind {
    pub const ALL: [ChannelKind; 6] = [
        ChannelKind::Command,
        ChannelKind::Coordination,
        ChannelKind::Audit,
        ChannelKind::Algedonic,
        ChannelKind::ResourceBargain,
        ChannelKind::TemporalVariety,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Command => "command_channel",
            Self::Coordination => "coordination_channel",
            Self::Audit => "audit_channel",
            Self::Algedonic => "algedonic_channel",
            Self::ResourceBargain => "resource_bargain_channel",
            Self::TemporalVariety => "temporal_variety_channel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    Execute,
    Coordinate,
    UnitRegistered,
    UnitRequest,
    AuditRequest,
    AuditResponse,
    IdentityUpdate,
    PolicyUpdate,
    ValuesUpdate,
    ResourceRequest,
    ResourceAllocation,
    EnvironmentalUpdate,
    IntelligenceRequest,
    ModelUpdate,
    Forecast,
    StrategicChange,
    DecisionRequest,
    DecisionResponse,
    Alert,
    Emergency,
    Critical,
    EmergencySignal,
    PainSignal,
    PleasureSignal,
    Other(String),
}

impl MessageKind {
    pub fn is_high_priority(&self) -> bool {
        matches!(self, Self::Alert | Self::Emergency | Self::Critical | Self::EmergencySignal | Self::PainSignal)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsmMessage {
    pub id: String,
    pub from: SystemId,
    pub to: SystemId,
    pub channel: ChannelKind,

    #[serde(rename = "type")]
    pub kind: MessageKind,

    pub payload: Value,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<Value>,
    pub correlation_id: Option<String>,
    pub reply_to: Option<String>,
}

impl VsmMessage {
    pub fn new(from: SystemId, to: SystemId, channel: ChannelKind, kind: MessageKind, payload: Value) -> Self {
        Self {
            id: format!("msg_{}", Uuid::new_v4()),
            from,
            to,
            channel,
            kind,
            payload,
            timestamp: Utc::now(),
            metadata: None,
            correlation_id: None,
            reply_to: None,
        }
    }

    pub fn command(from: SystemId, to: SystemId, kind: MessageKind, payload: Value) -> Self {
        Self::new(from, to, ChannelKind::Command, kind, payload)
    }

    pub fn coordination(from: SystemId, to: SystemId, kind: MessageKind, payload: Value) -> Self {
        Self::new(from, to, ChannelKind::Coordination, kind, payload)
    }

    pub fn audit(from: SystemId, to: SystemId, kind: MessageKind, payload: Value) -> Self {
        Self::new(from, to, ChannelKind::Audit, kind, payload)
    }

    pub fn resource_bargain(from: SystemId, to: SystemId, kind: MessageKind, payload: Value) -> Self {
        Self::new(from, to, ChannelKind::ResourceBargain, kind, payload)
    }

    pub fn algedonic(from: SystemId, payload: Value) -> Self {
        Self::new(from, SystemId::System5, ChannelKind::Algedonic, MessageKind::EmergencySignal, payload)
    }

    pub fn reply(&self, kind: MessageKind, payload: Value) -> Self {
        let mut reply = Self::new(self.to, self.from, self.channel, kind, payload);
        reply.correlation_id = Some(self.correlation_id.clone().unwrap_or_else(|| self.id.clone()));
        reply.reply_to = Some(self.id.clone());
        reply
    }

    pub fn serialize(&self) -> serde_json::Result<Value> {
        serde_json::to_value(self)
    }

    pub fn validate_basic_flow(&self) -> Result<(), String> {
        if self.from == SystemId::External || self.to == SystemId::External || self.to == SystemId::All {
            return Ok(());
        }
        match self.channel {
            ChannelKind::Command => {
                if matches!((self.from, self.to),
                    (SystemId::System5, SystemId::System4)
                    | (SystemId::System5, SystemId::System3)
                    | (SystemId::System4, SystemId::System3)
                    | (SystemId::System3, SystemId::System1)
                    | (SystemId::System5, SystemId::System1)
                    | (SystemId::System4, SystemId::System1)
                ) { Ok(()) } else { Err(format!("invalid command flow: {:?} -> {:?}", self.from, self.to)) }
            }
            ChannelKind::Coordination => {
                if matches!((self.from, self.to), (SystemId::System1, SystemId::System2) | (SystemId::System2, SystemId::System1)) {
                    Ok(())
                } else {
                    Err(format!("invalid coordination flow: {:?} -> {:?}", self.from, self.to))
                }
            }
            ChannelKind::Audit => {
                if matches!((self.from, self.to), (SystemId::System3Star, SystemId::System1) | (SystemId::System1, SystemId::System3Star) | (SystemId::System3, SystemId::System1) | (SystemId::System1, SystemId::System3)) {
                    Ok(())
                } else {
                    Err(format!("invalid audit flow: {:?} -> {:?}", self.from, self.to))
                }
            }
            ChannelKind::Algedonic => {
                if self.to == SystemId::System5 || self.to == SystemId::Algedonic { Ok(()) } else { Err(format!("invalid algedonic destination: {:?}", self.to)) }
            }
            ChannelKind::ResourceBargain => {
                if matches!((self.from, self.to), (SystemId::System1, SystemId::System3) | (SystemId::System3, SystemId::System1)) {
                    Ok(())
                } else {
                    Err(format!("invalid resource bargain flow: {:?} -> {:?}", self.from, self.to))
                }
            }
            ChannelKind::TemporalVariety => Ok(()),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_basic_flow()
    }

}
