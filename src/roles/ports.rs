//! Runtime ports for state, events, and reports.

use std::marker::PhantomData;

use ractor::async_trait;

use crate::error::FrameworkError;
use crate::protocol::{RuntimeEvent, RuntimeReport, SnapshotKey, SnapshotRecord};

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
