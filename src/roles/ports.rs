//! Runtime ports for state, events, reports, telemetry, alerts, clocks, and IDs.

use std::collections::BTreeMap;
use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use ractor::async_trait;

use crate::error::FrameworkError;
use crate::protocol::{
    CorrelationId, ProtocolMetadata, RuntimeEvent, RuntimeId, RuntimeReport, SnapshotKey,
    SnapshotRecord,
};

use super::ViableSystem;

/// Snapshot store port.
///
/// `NoopStateStore` is the default non-persistent implementation. Durable
/// adapters are intentionally deferred to the persistence milestone.
#[async_trait]
pub trait StateStore<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn load_unit_snapshot(
        &self,
        key: &SnapshotKey,
    ) -> Result<Option<SnapshotRecord<V::UnitSnapshot>>, FrameworkError>;

    async fn save_unit_snapshot(
        &self,
        record: SnapshotRecord<V::UnitSnapshot>,
    ) -> Result<(), FrameworkError>;
}

/// Non-persistent state store that always starts fresh.
#[derive(Debug, Default)]
pub struct NoopStateStore<V>
where
    V: ViableSystem,
{
    _system: PhantomData<V>,
}

impl<V> NoopStateStore<V>
where
    V: ViableSystem,
{
    pub fn new() -> Self {
        Self {
            _system: PhantomData,
        }
    }
}

#[async_trait]
impl<V> StateStore<V> for NoopStateStore<V>
where
    V: ViableSystem,
{
    async fn load_unit_snapshot(
        &self,
        _key: &SnapshotKey,
    ) -> Result<Option<SnapshotRecord<V::UnitSnapshot>>, FrameworkError> {
        Ok(None)
    }

    async fn save_unit_snapshot(
        &self,
        _record: SnapshotRecord<V::UnitSnapshot>,
    ) -> Result<(), FrameworkError> {
        Ok(())
    }
}

/// Observer event sink.
#[async_trait]
pub trait EventSink<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn record_event(&self, event: RuntimeEvent<V>) -> Result<(), FrameworkError>;
}

/// Report sink for cross-system reports.
#[async_trait]
pub trait ReportSink<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn record_report(&self, report: RuntimeReport<V>) -> Result<(), FrameworkError>;
}

/// Event sink that drops all events.
#[derive(Debug, Default)]
pub struct NoopEventSink<V>
where
    V: ViableSystem,
{
    _system: PhantomData<V>,
}

impl<V> NoopEventSink<V>
where
    V: ViableSystem,
{
    pub fn new() -> Self {
        Self {
            _system: PhantomData,
        }
    }
}

#[async_trait]
impl<V> EventSink<V> for NoopEventSink<V>
where
    V: ViableSystem,
{
    async fn record_event(&self, _event: RuntimeEvent<V>) -> Result<(), FrameworkError> {
        Ok(())
    }
}

/// Report sink that drops all reports.
#[derive(Debug, Default)]
pub struct NoopReportSink<V>
where
    V: ViableSystem,
{
    _system: PhantomData<V>,
}

impl<V> NoopReportSink<V>
where
    V: ViableSystem,
{
    pub fn new() -> Self {
        Self {
            _system: PhantomData,
        }
    }
}

#[async_trait]
impl<V> ReportSink<V> for NoopReportSink<V>
where
    V: ViableSystem,
{
    async fn record_report(&self, _report: RuntimeReport<V>) -> Result<(), FrameworkError> {
        Ok(())
    }
}

/// Structured telemetry emitted by runtime adapters and role wrappers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryRecord {
    pub metadata: ProtocolMetadata,
    pub name: String,
    pub fields: BTreeMap<String, String>,
    pub observed_at: DateTime<Utc>,
}

impl TelemetryRecord {
    /// Creates a telemetry record observed at the current wall-clock time.
    pub fn new(metadata: ProtocolMetadata, name: impl Into<String>) -> Self {
        Self {
            metadata,
            name: name.into(),
            fields: BTreeMap::new(),
            observed_at: Utc::now(),
        }
    }
}

/// Port for runtime telemetry export.
#[async_trait]
pub trait TelemetrySink: Send + Sync {
    async fn record_telemetry(&self, record: TelemetryRecord) -> Result<(), FrameworkError>;
}

/// Telemetry sink that drops all records.
#[derive(Debug, Default)]
pub struct NoopTelemetrySink;

#[async_trait]
impl TelemetrySink for NoopTelemetrySink {
    async fn record_telemetry(&self, _record: TelemetryRecord) -> Result<(), FrameworkError> {
        Ok(())
    }
}

/// Framework-level alert severity for external notification ports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Alert intended for an external notification adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertRecord {
    pub metadata: ProtocolMetadata,
    pub severity: AlertSeverity,
    pub message: String,
    pub details: BTreeMap<String, String>,
    pub raised_at: DateTime<Utc>,
}

impl AlertRecord {
    /// Creates an alert record raised at the current wall-clock time.
    pub fn new(
        metadata: ProtocolMetadata,
        severity: AlertSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            metadata,
            severity,
            message: message.into(),
            details: BTreeMap::new(),
            raised_at: Utc::now(),
        }
    }
}

/// Port for external alert delivery.
#[async_trait]
pub trait AlertSink: Send + Sync {
    async fn publish_alert(&self, alert: AlertRecord) -> Result<(), FrameworkError>;
}

/// Alert sink that drops all alerts.
#[derive(Debug, Default)]
pub struct NoopAlertSink;

#[async_trait]
impl AlertSink for NoopAlertSink {
    async fn publish_alert(&self, _alert: AlertRecord) -> Result<(), FrameworkError> {
        Ok(())
    }
}

/// Clock port used by role contexts and deterministic tests.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// Clock implementation backed by [`Utc::now`].
#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// ID generator port for runtime and correlation identifiers.
pub trait IdGenerator: Send + Sync {
    fn new_runtime_id(&self) -> RuntimeId;
    fn new_correlation_id(&self) -> CorrelationId;
}

/// UUID-backed ID generator.
#[derive(Debug, Default)]
pub struct UuidIdGenerator;

impl IdGenerator for UuidIdGenerator {
    fn new_runtime_id(&self) -> RuntimeId {
        RuntimeId::new()
    }

    fn new_correlation_id(&self) -> CorrelationId {
        CorrelationId::new()
    }
}
