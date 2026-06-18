//! Application startup and root supervision tree construction.
//!
//! This module creates the default globally named VSM runtime. The root
//! supervisor starts channel infrastructure before Systems 1-5 so actors can
//! subscribe during startup, and it uses one-for-one restart policies throughout
//! the static tree. Starting the default application twice in the same process
//! collides with names from `names`.

use ractor::concurrency::{Duration, JoinHandle};
use ractor::{ActorCell, ActorRef, SpawnErr};
use ractor_supervisor::{
    ChildSpec, DynamicSupervisor, DynamicSupervisorOptions, Restart, SpawnFn, Supervisor,
    SupervisorArguments, SupervisorMsg, SupervisorOptions, SupervisorStrategy,
};

use crate::channels;
use crate::names;
use crate::{system1, system2, system3, system4, system5, telemetry_reporter};

pub struct VsmApplication {
    pub supervisor: ActorRef<SupervisorMsg>,
    pub join_handle: JoinHandle<()>,
}

pub async fn start_vsm_core() -> Result<(ActorRef<SupervisorMsg>, JoinHandle<()>), SpawnErr> {
    tracing::info!("Starting VSM Core Application...");
    let result =
        Supervisor::spawn(names::ROOT_SUPERVISOR.to_string(), root_supervisor_args()).await;
    match &result {
        Ok(_) => tracing::info!("VSM Core Application started successfully"),
        Err(err) => tracing::error!(error = %err, "Failed to start VSM Core Application"),
    }
    result
}

pub async fn start_application() -> Result<VsmApplication, SpawnErr> {
    let (supervisor, join_handle) = start_vsm_core().await?;
    Ok(VsmApplication {
        supervisor,
        join_handle,
    })
}

pub fn root_supervisor_args() -> SupervisorArguments {
    SupervisorArguments {
        child_specs: vec![
            root_dynamic_supervisor_child(),
            static_supervisor_child(
                names::CHANNELS_SUPERVISOR,
                channels::supervisor::supervisor_args,
            ),
            static_supervisor_child(
                names::SYSTEM1_SUPERVISOR,
                system1::supervisor::supervisor_args,
            ),
            static_supervisor_child(
                names::SYSTEM2_SUPERVISOR,
                system2::supervisor::supervisor_args,
            ),
            static_supervisor_child(
                names::SYSTEM3_SUPERVISOR,
                system3::supervisor::supervisor_args,
            ),
            static_supervisor_child(
                names::SYSTEM4_SUPERVISOR,
                system4::supervisor::supervisor_args,
            ),
            static_supervisor_child(
                names::SYSTEM5_SUPERVISOR,
                system5::supervisor::supervisor_args,
            ),
            telemetry_reporter::child_spec(),
        ],
        options: SupervisorOptions {
            strategy: SupervisorStrategy::OneForOne,
            max_restarts: 5,
            max_window: Duration::from_secs(10),
            reset_after: Some(Duration::from_secs(30)),
        },
    }
}

fn root_dynamic_supervisor_child() -> ChildSpec {
    ChildSpec {
        id: names::DYNAMIC_SUPERVISOR.to_string(),
        restart: Restart::Permanent,
        spawn_fn: SpawnFn::new(spawn_dynamic_supervisor),
        backoff_fn: None,
        reset_after: Some(Duration::from_secs(60)),
    }
}

async fn spawn_dynamic_supervisor(
    supervisor_cell: ActorCell,
    child_id: String,
) -> Result<ActorCell, SpawnErr> {
    let args = DynamicSupervisorOptions {
        max_children: None,
        max_restarts: 10,
        max_window: Duration::from_secs(10),
        reset_after: Some(Duration::from_secs(30)),
    };
    let (sup_ref, _join) =
        DynamicSupervisor::spawn_linked(child_id, DynamicSupervisor, args, supervisor_cell).await?;
    Ok(sup_ref.get_cell())
}

fn static_supervisor_child(
    name: &'static str,
    build_args: fn() -> SupervisorArguments,
) -> ChildSpec {
    ChildSpec {
        id: name.to_string(),
        restart: Restart::Permanent,
        spawn_fn: SpawnFn::new(move |supervisor_cell, child_id| {
            spawn_static_supervisor(supervisor_cell, child_id, build_args)
        }),
        backoff_fn: None,
        reset_after: Some(Duration::from_secs(60)),
    }
}

async fn spawn_static_supervisor(
    supervisor_cell: ActorCell,
    child_id: String,
    build_args: fn() -> SupervisorArguments,
) -> Result<ActorCell, SpawnErr> {
    let (sup_ref, _join) =
        Supervisor::spawn_linked(child_id, Supervisor, build_args(), supervisor_cell).await?;
    Ok(sup_ref.get_cell())
}
