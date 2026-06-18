//! Private observer event bus for typed runtime handles.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use ractor::async_trait;
use tokio::sync::mpsc;

use crate::error::FrameworkError;
use crate::protocol::RuntimeEvent;
use crate::roles::{EventSink, ViableSystem};

/// Stable observer subscription identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObserverId(String);

impl ObserverId {
    /// Creates an observer ID from an existing string.
    pub fn from_string(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the observer ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ObserverId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Runtime event subscription returned to observers.
pub struct ObserverSubscription<V>
where
    V: ViableSystem,
{
    observer_id: ObserverId,
    receiver: mpsc::Receiver<RuntimeEvent<V>>,
}

impl<V> ObserverSubscription<V>
where
    V: ViableSystem,
{
    /// Returns the subscription identity.
    pub fn observer_id(&self) -> &ObserverId {
        &self.observer_id
    }

    /// Waits for the next observer event.
    pub async fn recv(&mut self) -> Option<RuntimeEvent<V>> {
        self.receiver.recv().await
    }

    /// Attempts to receive the next observer event without waiting.
    pub fn try_recv(&mut self) -> Result<RuntimeEvent<V>, mpsc::error::TryRecvError> {
        self.receiver.try_recv()
    }
}

/// Point-in-time observer bus metrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObserverBusSnapshot {
    pub subscriber_count: usize,
    pub retained_event_count: usize,
    pub dropped_event_count: usize,
    pub closed_subscriber_count: usize,
    pub sink_error_count: usize,
}

#[derive(Debug, Default)]
struct ObserverBusMetrics {
    dropped_event_count: usize,
    closed_subscriber_count: usize,
    sink_error_count: usize,
}

pub(crate) struct ObserverEventBus<V>
where
    V: ViableSystem,
{
    downstream: Arc<dyn EventSink<V>>,
    subscribers: Mutex<HashMap<ObserverId, mpsc::Sender<RuntimeEvent<V>>>>,
    history: Mutex<VecDeque<RuntimeEvent<V>>>,
    metrics: Mutex<ObserverBusMetrics>,
    retained_capacity: usize,
    subscriber_capacity: usize,
}

impl<V> ObserverEventBus<V>
where
    V: ViableSystem,
{
    pub(crate) fn new(downstream: Arc<dyn EventSink<V>>, retained_capacity: usize) -> Self {
        Self {
            downstream,
            subscribers: Mutex::new(HashMap::new()),
            history: Mutex::new(VecDeque::new()),
            metrics: Mutex::new(ObserverBusMetrics::default()),
            retained_capacity,
            subscriber_capacity: retained_capacity.max(1),
        }
    }

    pub(crate) fn subscribe(
        &self,
        observer_id: ObserverId,
    ) -> Result<ObserverSubscription<V>, FrameworkError> {
        let (sender, receiver) = mpsc::channel(self.subscriber_capacity);
        self.subscribers
            .lock()
            .map_err(poisoned_subscribers)?
            .insert(observer_id.clone(), sender);

        Ok(ObserverSubscription {
            observer_id,
            receiver,
        })
    }

    pub(crate) fn history(&self) -> Result<Vec<RuntimeEvent<V>>, FrameworkError> {
        Ok(self
            .history
            .lock()
            .map_err(poisoned_history)?
            .iter()
            .cloned()
            .collect())
    }

    pub(crate) fn snapshot(&self) -> Result<ObserverBusSnapshot, FrameworkError> {
        let subscriber_count = self.subscribers.lock().map_err(poisoned_subscribers)?.len();
        let retained_event_count = self.history.lock().map_err(poisoned_history)?.len();
        let metrics = self.metrics.lock().map_err(poisoned_metrics)?;

        Ok(ObserverBusSnapshot {
            subscriber_count,
            retained_event_count,
            dropped_event_count: metrics.dropped_event_count,
            closed_subscriber_count: metrics.closed_subscriber_count,
            sink_error_count: metrics.sink_error_count,
        })
    }

    fn retain(&self, event: RuntimeEvent<V>) -> Result<(), FrameworkError> {
        if self.retained_capacity == 0 {
            return Ok(());
        }

        let mut history = self.history.lock().map_err(poisoned_history)?;
        history.push_front(event);
        history.truncate(self.retained_capacity);
        Ok(())
    }

    fn deliver_to_subscribers(&self, event: RuntimeEvent<V>) -> Result<(), FrameworkError> {
        let mut subscribers = self.subscribers.lock().map_err(poisoned_subscribers)?;
        let mut closed = Vec::new();
        let mut dropped = 0;

        for (observer_id, subscriber) in subscribers.iter() {
            match subscriber.try_send(event.clone()) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => dropped += 1,
                Err(mpsc::error::TrySendError::Closed(_)) => closed.push(observer_id.clone()),
            }
        }

        for observer_id in &closed {
            subscribers.remove(observer_id);
        }
        drop(subscribers);

        if dropped > 0 || !closed.is_empty() {
            let mut metrics = self.metrics.lock().map_err(poisoned_metrics)?;
            metrics.dropped_event_count += dropped;
            metrics.closed_subscriber_count += closed.len();
        }

        Ok(())
    }

    fn record_sink_error(&self) -> Result<(), FrameworkError> {
        self.metrics
            .lock()
            .map_err(poisoned_metrics)?
            .sink_error_count += 1;
        Ok(())
    }
}

#[async_trait]
impl<V> EventSink<V> for ObserverEventBus<V>
where
    V: ViableSystem,
{
    async fn record_event(&self, event: RuntimeEvent<V>) -> Result<(), FrameworkError> {
        self.retain(event.clone())?;
        self.deliver_to_subscribers(event.clone())?;

        if self.downstream.record_event(event).await.is_err() {
            self.record_sink_error()?;
        }

        Ok(())
    }
}

fn poisoned_subscribers<T>(_: std::sync::PoisonError<T>) -> FrameworkError {
    FrameworkError::Runtime {
        reason: "observer event subscriber registry mutex poisoned".to_string(),
    }
}

fn poisoned_history<T>(_: std::sync::PoisonError<T>) -> FrameworkError {
    FrameworkError::Runtime {
        reason: "observer event history mutex poisoned".to_string(),
    }
}

fn poisoned_metrics<T>(_: std::sync::PoisonError<T>) -> FrameworkError {
    FrameworkError::Runtime {
        reason: "observer event metrics mutex poisoned".to_string(),
    }
}
