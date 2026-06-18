//! Supervisor specification for channel infrastructure.
//!
//! The channel supervisor starts the broker plus the dedicated algedonic and
//! temporal-variety actors under a one-for-one strategy. Broker restart loses
//! subscriptions and retained channel history; subscribers do not currently
//! re-register automatically.

use ractor::concurrency::Duration;
use ractor::{ActorCell, SpawnErr};
use ractor_supervisor::{
    ChildSpec, Restart, SpawnFn, Supervisor, SupervisorArguments, SupervisorOptions,
    SupervisorStrategy,
};

use crate::channels::algedonic::{Algedonic, AlgedonicArgs};
use crate::channels::broker::ChannelBroker;
use crate::channels::temporal_variety::{TemporalVariety, TemporalVarietyArgs};
use crate::names;

pub fn supervisor_args() -> SupervisorArguments {
    SupervisorArguments {
        child_specs: vec![
            channel_broker_child(),
            algedonic_child(),
            temporal_variety_child(),
        ],
        options: SupervisorOptions {
            strategy: SupervisorStrategy::OneForOne,
            max_restarts: 5,
            max_window: Duration::from_secs(10),
            reset_after: Some(Duration::from_secs(30)),
        },
    }
}

fn channel_broker_child() -> ChildSpec {
    ChildSpec {
        id: names::CHANNEL_BROKER.to_string(),
        restart: Restart::Permanent,
        spawn_fn: SpawnFn::new(spawn_channel_broker),
        backoff_fn: None,
        reset_after: Some(Duration::from_secs(60)),
    }
}

async fn spawn_channel_broker(
    supervisor_cell: ActorCell,
    child_id: String,
) -> Result<ActorCell, SpawnErr> {
    let (actor, _join) =
        Supervisor::spawn_linked(child_id, ChannelBroker, (), supervisor_cell).await?;
    Ok(actor.get_cell())
}

fn algedonic_child() -> ChildSpec {
    ChildSpec {
        id: names::ALGEDONIC.to_string(),
        restart: Restart::Permanent,
        spawn_fn: SpawnFn::new(|supervisor_cell, child_id| async move {
            let (actor, _join) = Supervisor::spawn_linked(
                child_id,
                Algedonic,
                AlgedonicArgs::default(),
                supervisor_cell,
            )
            .await?;
            Ok(actor.get_cell())
        }),
        backoff_fn: None,
        reset_after: Some(Duration::from_secs(60)),
    }
}

fn temporal_variety_child() -> ChildSpec {
    ChildSpec {
        id: names::TEMPORAL_VARIETY.to_string(),
        restart: Restart::Permanent,
        spawn_fn: SpawnFn::new(|supervisor_cell, child_id| async move {
            let (actor, _join) = Supervisor::spawn_linked(
                child_id,
                TemporalVariety,
                TemporalVarietyArgs::default(),
                supervisor_cell,
            )
            .await?;
            Ok(actor.get_cell())
        }),
        backoff_fn: None,
        reset_after: Some(Duration::from_secs(60)),
    }
}
