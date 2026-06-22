//! Private operational-recursion runtime manager.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::Utc;
use uuid::Uuid;

use crate::config::RuntimeConfig;
use crate::error::FrameworkError;
use crate::protocol::algedonic::AlgedonicSignalRecord;
use crate::protocol::recursion::{
    ChildRuntimeSnapshot, ChildRuntimeStatus, DelegatedWork, RecursionAlgedonicEscalation,
    RecursionBoundaryDecision, RecursionIntelligenceSummary, RecursionPerformanceSummary,
    RecursionPolicyDirective, RecursionResourceEscalation, RecursionSnapshot,
};
use crate::protocol::system1::{
    CapacitySnapshot, ResourceShortageRequest, WorkRequest, WorkResponse,
};
use crate::protocol::system3::{OperationalDirective, ResourceRequest};
use crate::protocol::system4::System4IntelligenceCycle;
use crate::protocol::SubsystemRole;
use crate::roles::{RoleContext, ViableSystem};
use crate::runtime::{ChildRuntimeRegistration, RecursionRuntimeRoles, RuntimePorts, VsmRuntime};

pub(crate) struct RecursionRuntime<V>
where
    V: ViableSystem,
{
    config: RuntimeConfig,
    roles: RecursionRuntimeRoles<V>,
    ports: RuntimePorts<V>,
    children: Mutex<HashMap<String, ChildEntry<V>>>,
    resource_escalations: Mutex<Vec<RecursionResourceEscalation<V>>>,
    algedonic_escalations: Mutex<Vec<RecursionAlgedonicEscalation<V>>>,
    policy_directives: Mutex<Vec<RecursionPolicyDirective<V>>>,
    intelligence_summaries: Mutex<Vec<RecursionIntelligenceSummary>>,
    performance_summaries: Mutex<Vec<RecursionPerformanceSummary<V>>>,
    shutdown: AtomicBool,
}

impl<V> RecursionRuntime<V>
where
    V: ViableSystem,
{
    pub(crate) async fn start(
        config: RuntimeConfig,
        roles: RecursionRuntimeRoles<V>,
        ports: RuntimePorts<V>,
    ) -> Result<Arc<Self>, FrameworkError> {
        Ok(Arc::new(Self {
            config,
            roles,
            ports,
            children: Mutex::new(HashMap::new()),
            resource_escalations: Mutex::new(Vec::new()),
            algedonic_escalations: Mutex::new(Vec::new()),
            policy_directives: Mutex::new(Vec::new()),
            intelligence_summaries: Mutex::new(Vec::new()),
            performance_summaries: Mutex::new(Vec::new()),
            shutdown: AtomicBool::new(false),
        }))
    }

    pub(crate) async fn register_child(
        &self,
        registration: ChildRuntimeRegistration<V>,
    ) -> Result<ChildRuntimeSnapshot<V>, FrameworkError> {
        self.ensure_running()?;

        let context = self.role_context();
        let descriptor = registration.descriptor.clone();
        let decision = self
            .roles
            .recursion_transducer()
            .authorize_child_registration(&context, &descriptor)
            .await?;
        require_allowed(decision)?;

        {
            let children = self.children.lock().map_err(poisoned_children)?;
            if children.contains_key(&descriptor.child_id) {
                return Err(FrameworkError::InvalidProtocol {
                    reason: format!("child runtime already registered: {}", descriptor.child_id),
                });
            }
        }

        let runtime = Arc::new(
            registration
                .factory
                .start_child_runtime(&context, &descriptor)
                .await?,
        );
        let registered_units = runtime.system1().list_units()?.len();
        let snapshot = ChildRuntimeSnapshot {
            descriptor: descriptor.clone(),
            status: ChildRuntimeStatus::Running,
            registered_units,
        };

        let mut children = self.children.lock().map_err(poisoned_children)?;
        if children.contains_key(&descriptor.child_id) {
            return Err(FrameworkError::InvalidProtocol {
                reason: format!("child runtime already registered: {}", descriptor.child_id),
            });
        }
        children.insert(
            descriptor.child_id.clone(),
            ChildEntry {
                descriptor,
                runtime,
                status: ChildRuntimeStatus::Running,
            },
        );

        Ok(snapshot)
    }

    pub(crate) fn list_children(&self) -> Result<Vec<ChildRuntimeSnapshot<V>>, FrameworkError> {
        let children = self.children.lock().map_err(poisoned_children)?;
        Ok(children.values().map(ChildEntry::snapshot).collect())
    }

    pub(crate) fn child_runtime(
        &self,
        child_id: &str,
    ) -> Result<Arc<VsmRuntime<V>>, FrameworkError> {
        Ok(self.child_entry(child_id)?.runtime)
    }

    pub(crate) async fn child_capacity(
        &self,
        child_id: &str,
    ) -> Result<CapacitySnapshot, FrameworkError> {
        Ok(self.child_entry(child_id)?.descriptor.capacity)
    }

    pub(crate) async fn delegate_work(
        &self,
        child_id: &str,
        request: WorkRequest<V>,
    ) -> Result<WorkResponse<V>, FrameworkError> {
        self.ensure_running()?;
        let entry = self.child_entry(child_id)?;
        let context = self.role_context();
        let delegation = DelegatedWork::new(child_id.to_string(), request);
        let request = self
            .roles
            .recursion_transducer()
            .translate_work_to_child(&context, &entry.descriptor, delegation.request.clone())
            .await?;
        let response = entry.runtime.system1().process_response(request).await;
        self.roles
            .recursion_transducer()
            .translate_work_from_child(&context, &entry.descriptor, response)
            .await
    }

    pub(crate) async fn translate_resource_escalation(
        &self,
        child_id: &str,
        shortage: ResourceShortageRequest<V>,
    ) -> Result<ResourceRequest<V>, FrameworkError> {
        self.ensure_running()?;
        let entry = self.child_entry(child_id)?;
        let context = self.role_context();
        let (decision, parent_request) = self
            .roles
            .recursion_transducer()
            .translate_resource_escalation(&context, &entry.descriptor, shortage.clone())
            .await?;
        let record = RecursionResourceEscalation {
            metadata: shortage.metadata.child(),
            escalation_id: format!("recursion-resource-{}", Uuid::new_v4()),
            child_id: child_id.to_string(),
            shortage,
            parent_request: parent_request.clone(),
            decision: decision.clone(),
            escalated_at: Utc::now(),
        };
        self.resource_escalations
            .lock()
            .map_err(poisoned_resource_escalations)?
            .push(record);
        require_allowed(decision)?;
        Ok(parent_request)
    }

    pub(crate) async fn translate_algedonic_escalation(
        &self,
        child_id: &str,
        signal: AlgedonicSignalRecord<V>,
    ) -> Result<AlgedonicSignalRecord<V>, FrameworkError> {
        self.ensure_running()?;
        let entry = self.child_entry(child_id)?;
        let context = self.role_context();
        let (decision, parent_signal) = self
            .roles
            .recursion_transducer()
            .disclose_algedonic_escalation(&context, &entry.descriptor, signal)
            .await?;
        let record = RecursionAlgedonicEscalation {
            metadata: parent_signal.metadata.child(),
            escalation_id: format!("recursion-algedonic-{}", Uuid::new_v4()),
            child_id: child_id.to_string(),
            signal: parent_signal.clone(),
            decision: decision.clone(),
            escalated_at: Utc::now(),
        };
        self.algedonic_escalations
            .lock()
            .map_err(poisoned_algedonic_escalations)?
            .push(record);
        require_allowed(decision)?;
        Ok(parent_signal)
    }

    pub(crate) async fn transduce_policy_directive(
        &self,
        child_id: &str,
        directive: OperationalDirective<V>,
    ) -> Result<Option<OperationalDirective<V>>, FrameworkError> {
        self.ensure_running()?;
        let entry = self.child_entry(child_id)?;
        let context = self.role_context();
        let (decision, child_directive) = self
            .roles
            .recursion_transducer()
            .transduce_policy_directive(&context, &entry.descriptor, directive.clone())
            .await?;
        let record = RecursionPolicyDirective {
            metadata: directive.metadata.child(),
            directive_id: format!("recursion-policy-{}", Uuid::new_v4()),
            child_id: child_id.to_string(),
            parent_directive: directive,
            child_directive: child_directive.clone(),
            decision: decision.clone(),
            transduced_at: Utc::now(),
        };
        self.policy_directives
            .lock()
            .map_err(poisoned_policy_directives)?
            .push(record);

        if decision.is_allowed() {
            Ok(child_directive)
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn record_intelligence_summary(
        &self,
        child_id: &str,
        cycle: &System4IntelligenceCycle,
    ) -> Result<RecursionIntelligenceSummary, FrameworkError> {
        self.ensure_running()?;
        let _ = self.child_entry(child_id)?;
        let summary = RecursionIntelligenceSummary::from_cycle(child_id.to_string(), cycle);
        self.intelligence_summaries
            .lock()
            .map_err(poisoned_intelligence_summaries)?
            .push(summary.clone());
        Ok(summary)
    }

    pub(crate) fn snapshot(&self) -> Result<RecursionSnapshot<V>, FrameworkError> {
        Ok(RecursionSnapshot {
            children: self.list_children()?,
            resource_escalations: self
                .resource_escalations
                .lock()
                .map_err(poisoned_resource_escalations)?
                .clone(),
            algedonic_escalations: self
                .algedonic_escalations
                .lock()
                .map_err(poisoned_algedonic_escalations)?
                .clone(),
            policy_directives: self
                .policy_directives
                .lock()
                .map_err(poisoned_policy_directives)?
                .clone(),
            intelligence_summaries: self
                .intelligence_summaries
                .lock()
                .map_err(poisoned_intelligence_summaries)?
                .clone(),
            performance_summaries: self
                .performance_summaries
                .lock()
                .map_err(poisoned_performance_summaries)?
                .clone(),
            captured_at: Utc::now(),
        })
    }

    pub(crate) async fn shutdown(&self) -> Result<(), FrameworkError> {
        if self.shutdown.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        let runtimes = {
            let mut children = self.children.lock().map_err(poisoned_children)?;
            for entry in children.values_mut() {
                entry.status = ChildRuntimeStatus::Stopped;
            }
            children
                .values()
                .map(|entry| Arc::clone(&entry.runtime))
                .collect::<Vec<_>>()
        };

        for runtime in runtimes {
            runtime.shutdown_without_children().await?;
        }

        Ok(())
    }

    fn ensure_running(&self) -> Result<(), FrameworkError> {
        if self.shutdown.load(Ordering::SeqCst) {
            Err(FrameworkError::Shutdown)
        } else {
            Ok(())
        }
    }

    fn child_entry(&self, child_id: &str) -> Result<ChildEntry<V>, FrameworkError> {
        let children = self.children.lock().map_err(poisoned_children)?;
        children
            .get(child_id)
            .cloned()
            .ok_or_else(|| FrameworkError::Unavailable {
                target: format!("child runtime {child_id}"),
            })
    }

    fn role_context(&self) -> RoleContext<V> {
        self.ports.role_context(
            self.config.runtime_id.clone(),
            self.config.recursion_path.clone(),
            SubsystemRole::Custom("recursion".to_string()),
        )
    }
}

struct ChildEntry<V>
where
    V: ViableSystem,
{
    descriptor: crate::protocol::recursion::ChildRuntimeDescriptor<V>,
    runtime: Arc<VsmRuntime<V>>,
    status: ChildRuntimeStatus,
}

impl<V> Clone for ChildEntry<V>
where
    V: ViableSystem,
{
    fn clone(&self) -> Self {
        Self {
            descriptor: self.descriptor.clone(),
            runtime: Arc::clone(&self.runtime),
            status: self.status,
        }
    }
}

impl<V> ChildEntry<V>
where
    V: ViableSystem,
{
    fn snapshot(&self) -> ChildRuntimeSnapshot<V> {
        let registered_units = self
            .runtime
            .system1()
            .list_units()
            .map(|units| units.len())
            .unwrap_or_default();
        ChildRuntimeSnapshot {
            descriptor: self.descriptor.clone(),
            status: self.status,
            registered_units,
        }
    }
}

fn require_allowed(decision: RecursionBoundaryDecision) -> Result<(), FrameworkError> {
    match decision {
        RecursionBoundaryDecision::Allow => Ok(()),
        RecursionBoundaryDecision::Deny { reason } => Err(FrameworkError::InvalidProtocol {
            reason: format!("recursion boundary denied operation: {reason}"),
        }),
    }
}

fn poisoned_children<V>(
    _: std::sync::PoisonError<std::sync::MutexGuard<'_, HashMap<String, ChildEntry<V>>>>,
) -> FrameworkError
where
    V: ViableSystem,
{
    FrameworkError::Runtime {
        reason: "recursion child registry mutex poisoned".to_string(),
    }
}

fn poisoned_resource_escalations<V>(
    _: std::sync::PoisonError<std::sync::MutexGuard<'_, Vec<RecursionResourceEscalation<V>>>>,
) -> FrameworkError
where
    V: ViableSystem,
{
    FrameworkError::Runtime {
        reason: "recursion resource escalation mutex poisoned".to_string(),
    }
}

fn poisoned_algedonic_escalations<V>(
    _: std::sync::PoisonError<std::sync::MutexGuard<'_, Vec<RecursionAlgedonicEscalation<V>>>>,
) -> FrameworkError
where
    V: ViableSystem,
{
    FrameworkError::Runtime {
        reason: "recursion algedonic escalation mutex poisoned".to_string(),
    }
}

fn poisoned_policy_directives<V>(
    _: std::sync::PoisonError<std::sync::MutexGuard<'_, Vec<RecursionPolicyDirective<V>>>>,
) -> FrameworkError
where
    V: ViableSystem,
{
    FrameworkError::Runtime {
        reason: "recursion policy directive mutex poisoned".to_string(),
    }
}

fn poisoned_intelligence_summaries(
    _: std::sync::PoisonError<std::sync::MutexGuard<'_, Vec<RecursionIntelligenceSummary>>>,
) -> FrameworkError {
    FrameworkError::Runtime {
        reason: "recursion intelligence summary mutex poisoned".to_string(),
    }
}

fn poisoned_performance_summaries<V>(
    _: std::sync::PoisonError<std::sync::MutexGuard<'_, Vec<RecursionPerformanceSummary<V>>>>,
) -> FrameworkError
where
    V: ViableSystem,
{
    FrameworkError::Runtime {
        reason: "recursion performance summary mutex poisoned".to_string(),
    }
}
