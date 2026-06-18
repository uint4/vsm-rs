//! Supervisor specification for System 1.
//!
//! System 1 starts a dynamic unit supervisor before the Operations actor so
//! Operations can resolve it during startup. Registered units are spawned under
//! that dynamic supervisor with either permanent or temporary restart behavior
//! based on `UnitConfig::auto_restart`.

use ractor::{ActorCell, SpawnErr};
use ractor::concurrency::Duration;
use ractor_supervisor::{
    ChildSpec, DynamicSupervisor, DynamicSupervisorOptions, Restart, SpawnFn, Supervisor,
    SupervisorArguments, SupervisorOptions, SupervisorStrategy,
};
use serde_json::json;

use crate::names;
use crate::system1::operations::{Operations, OperationsArgs};

pub fn supervisor_args() -> SupervisorArguments {
    SupervisorArguments {
        child_specs: vec![unit_supervisor_child(), operations_child()],
        options: SupervisorOptions {
            strategy: SupervisorStrategy::OneForOne,
            max_restarts: 5,
            max_window: Duration::from_secs(10),
            reset_after: Some(Duration::from_secs(30)),
        },
    }
}

fn unit_supervisor_child() -> ChildSpec {
    ChildSpec {
        id: names::SYSTEM1_UNIT_SUPERVISOR.to_string(),
        restart: Restart::Permanent,
        spawn_fn: SpawnFn::new(|supervisor_cell, child_id| spawn_unit_supervisor(supervisor_cell, child_id)),
        backoff_fn: None,
        reset_after: Some(Duration::from_secs(60)),
    }
}

async fn spawn_unit_supervisor(
    supervisor_cell: ActorCell,
    child_id: String,
) -> Result<ActorCell, SpawnErr> {
    let args = DynamicSupervisorOptions {
        max_children: None,
        max_restarts: 10,
        max_window: Duration::from_secs(10),
        reset_after: Some(Duration::from_secs(30)),
    };

    let (sup_ref, _join) = DynamicSupervisor::spawn_linked(
        child_id,
        DynamicSupervisor,
        args,
        supervisor_cell,
    )
    .await?;

    Ok(sup_ref.get_cell())
}

fn operations_child() -> ChildSpec {
    ChildSpec {
        id: names::SYSTEM1_OPERATIONS.to_string(),
        restart: Restart::Permanent,
        spawn_fn: SpawnFn::new(|supervisor_cell, child_id| spawn_operations(supervisor_cell, child_id)),
        backoff_fn: None,
        reset_after: Some(Duration::from_secs(60)),
    }
}

async fn spawn_operations(
    supervisor_cell: ActorCell,
    child_id: String,
) -> Result<ActorCell, SpawnErr> {
    let args = OperationsArgs {
        config: json!({
            "subsystem": "system1",
            "role": "operations"
        }),
    };

    let (ops_ref, _join) = Supervisor::spawn_linked(
        child_id,
        Operations,
        args,
        supervisor_cell,
    )
    .await?;

    Ok(ops_ref.get_cell())
}
