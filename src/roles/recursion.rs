//! Operational recursion role contracts.

use std::marker::PhantomData;
use std::sync::Arc;

use ractor::async_trait;

use crate::error::FrameworkError;
use crate::protocol::algedonic::AlgedonicSignalRecord;
use crate::protocol::recursion::{ChildRuntimeDescriptor, RecursionBoundaryDecision};
use crate::protocol::system1::{ResourceShortageRequest, WorkRequest, WorkResponse};
use crate::protocol::system3::{OperationalDirective, ResourceRequest};

use super::{RoleContext, ViableSystem};

/// Shared recursion transducer object.
pub type SharedRecursionTransducer<V> = Arc<dyn RecursionTransducer<V>>;

/// Application-owned parent/child recursion boundary behavior.
///
/// The default implementation is transparent and permissive. Applications can
/// replace it to enforce authority, redact disclosure, or translate protocol
/// records while keeping core recursion mechanics framework-owned.
#[async_trait]
pub trait RecursionTransducer<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn authorize_child_registration(
        &self,
        context: &RoleContext<V>,
        descriptor: &ChildRuntimeDescriptor<V>,
    ) -> Result<RecursionBoundaryDecision, FrameworkError> {
        let _ = (context, descriptor);
        Ok(RecursionBoundaryDecision::allow())
    }

    async fn translate_work_to_child(
        &self,
        context: &RoleContext<V>,
        child: &ChildRuntimeDescriptor<V>,
        request: WorkRequest<V>,
    ) -> Result<WorkRequest<V>, FrameworkError> {
        let _ = (context, child);
        Ok(request)
    }

    async fn translate_work_from_child(
        &self,
        context: &RoleContext<V>,
        child: &ChildRuntimeDescriptor<V>,
        response: WorkResponse<V>,
    ) -> Result<WorkResponse<V>, FrameworkError> {
        let _ = (context, child);
        Ok(response)
    }

    async fn translate_resource_escalation(
        &self,
        context: &RoleContext<V>,
        child: &ChildRuntimeDescriptor<V>,
        shortage: ResourceShortageRequest<V>,
    ) -> Result<(RecursionBoundaryDecision, ResourceRequest<V>), FrameworkError> {
        let _ = (context, child);
        let parent_request = ResourceRequest::from_shortage(shortage.clone());
        Ok((RecursionBoundaryDecision::allow(), parent_request))
    }

    async fn disclose_algedonic_escalation(
        &self,
        context: &RoleContext<V>,
        child: &ChildRuntimeDescriptor<V>,
        signal: AlgedonicSignalRecord<V>,
    ) -> Result<(RecursionBoundaryDecision, AlgedonicSignalRecord<V>), FrameworkError> {
        let _ = (context, child);
        Ok((RecursionBoundaryDecision::allow(), signal))
    }

    async fn transduce_policy_directive(
        &self,
        context: &RoleContext<V>,
        child: &ChildRuntimeDescriptor<V>,
        directive: OperationalDirective<V>,
    ) -> Result<(RecursionBoundaryDecision, Option<OperationalDirective<V>>), FrameworkError> {
        let _ = (context, child);
        Ok((RecursionBoundaryDecision::allow(), Some(directive)))
    }
}

/// Static catalog of recursion roles for one application type family.
pub trait RecursionRoles<V>: Send + Sync + 'static
where
    V: ViableSystem,
{
    type RecursionTransducer: RecursionTransducer<V>;
}

/// Opt-in defaults for operational recursion.
pub mod defaults {
    use super::*;

    /// Transparent recursion transducer that allows every boundary operation.
    #[derive(Debug, Default)]
    pub struct AllowAllRecursionTransducer<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> AllowAllRecursionTransducer<V>
    where
        V: ViableSystem,
    {
        /// Creates an allow-all recursion transducer.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> RecursionTransducer<V> for AllowAllRecursionTransducer<V> where V: ViableSystem {}
}
