//! Demo operational unit actor.
//!
//! A unit owns its config, status, current load, processed count, and arbitrary
//! local JSON state. The built-in implementation validates capability coverage,
//! returns a JSON success envelope for processed transactions, and responds to a
//! simple `status` command. Real applications should replace or extend this
//! actor with domain-specific work.

use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use serde_json::{json, Value};

use crate::system1::transaction::{Transaction, TransactionResult};
use crate::system1::types::{UnitConfig, WorkMigrationDirection};

pub enum UnitMsg {
    Process(Transaction, RpcReplyPort<TransactionResult>),
    CanHandle(Transaction, RpcReplyPort<bool>),
    GetLoad(RpcReplyPort<f64>),
    GetStatus(RpcReplyPort<String>),
    ExecuteCommand(Value),
    GetState(RpcReplyPort<Value>),
    UpdateState(Value),
    MigrateWork(WorkMigrationDirection, f64),
}

pub struct Unit;

#[derive(Debug, Clone)]
pub struct UnitState {
    config: UnitConfig,
    status: String,
    processed_count: u64,
    current_load: f64,
    local_state: Value,
}

impl Unit {
    pub fn actor_name(unit_id: &str) -> String {
        format!("vsm.system1.unit.{unit_id}")
    }
}

#[ractor::async_trait]
impl Actor for Unit {
    type Msg = UnitMsg;
    type State = UnitState;
    type Arguments = UnitConfig;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        config: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!(unit_id = %config.id, "System 1 unit started");

        Ok(UnitState {
            config,
            status: "running".to_string(),
            processed_count: 0,
            current_load: 0.0,
            local_state: json!({}),
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            UnitMsg::Process(transaction, reply) => {
                state.current_load += 1.0;
                state.processed_count += 1;

                let result = TransactionResult::Ok(json!({
                    "unit_id": state.config.id.clone(),
                    "transaction_id": transaction.id,
                    "transaction_type": transaction.kind,
                    "processed_count": state.processed_count,
                    "result": "processed"
                }));

                state.current_load = (state.current_load - 1.0).max(0.0);
                let _ = reply.send(result);
            }

            UnitMsg::CanHandle(transaction, reply) => {
                let can_handle = transaction
                    .required_capabilities
                    .iter()
                    .all(|required| state.config.capabilities.contains(required));
                let _ = reply.send(can_handle);
            }

            UnitMsg::GetLoad(reply) => {
                let _ = reply.send(state.current_load);
            }

            UnitMsg::GetStatus(reply) => {
                let _ = reply.send(state.status.clone());
            }

            UnitMsg::ExecuteCommand(command) => {
                tracing::info!(unit_id = %state.config.id, ?command, "unit executing command");

                if let Some(new_status) = command.get("status").and_then(Value::as_str) {
                    state.status = new_status.to_string();
                }
            }

            UnitMsg::GetState(reply) => {
                let _ = reply.send(state.local_state.clone());
            }

            UnitMsg::UpdateState(new_state) => {
                state.local_state = new_state;
            }

            UnitMsg::MigrateWork(direction, amount) => {
                match direction {
                    WorkMigrationDirection::In => {
                        state.current_load += amount.max(0.0);
                    }
                    WorkMigrationDirection::Out => {
                        state.current_load = (state.current_load - amount.max(0.0)).max(0.0);
                    }
                }
            }
        }

        Ok(())
    }
}
