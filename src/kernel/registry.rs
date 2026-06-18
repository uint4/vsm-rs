//! Instance-scoped runtime component registry.

use std::collections::BTreeMap;

use crate::protocol::{RecursionPath, RuntimeId, SubsystemRole, VsmAddress};
use crate::runtime::{RuntimeComponentSnapshot, RuntimeComponentStatus, RuntimeDirectorySnapshot};

#[derive(Debug, Default)]
pub(crate) struct RuntimeDirectory {
    components: BTreeMap<String, RuntimeComponentSnapshot>,
}

impl RuntimeDirectory {
    pub(crate) fn new() -> Self {
        Self {
            components: BTreeMap::new(),
        }
    }

    pub(crate) fn register(
        &mut self,
        runtime_id: &RuntimeId,
        recursion_path: &RecursionPath,
        role: SubsystemRole,
        entity: impl Into<String>,
        status: RuntimeComponentStatus,
    ) {
        let entity = entity.into();
        let address = VsmAddress::new(runtime_id.clone(), recursion_path.clone(), role)
            .with_entity(entity.clone());
        let internal_name = internal_component_name(runtime_id, recursion_path, &address, &entity);

        self.components.insert(
            internal_name.clone(),
            RuntimeComponentSnapshot {
                internal_name,
                address,
                status,
            },
        );
    }

    pub(crate) fn mark_all_shutdown(&mut self) {
        for component in self.components.values_mut() {
            component.status = RuntimeComponentStatus::Shutdown;
        }
    }

    pub(crate) fn snapshot(&self) -> RuntimeDirectorySnapshot {
        RuntimeDirectorySnapshot {
            components: self.components.values().cloned().collect(),
        }
    }
}

fn internal_component_name(
    runtime_id: &RuntimeId,
    recursion_path: &RecursionPath,
    address: &VsmAddress,
    entity: &str,
) -> String {
    let path = if recursion_path.is_root() {
        "root".to_string()
    } else {
        recursion_path.segments().join("/")
    };

    format!("{runtime_id}:{path}:{role:?}:{entity}", role = address.role)
}
