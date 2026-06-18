//! System 3 control, resource governance, and System 3* audit role contracts.

use std::marker::PhantomData;
use std::sync::Arc;

use ractor::async_trait;

use crate::error::FrameworkError;
use crate::protocol::system1::{AuditEvidence, PerformanceObservation};
use crate::protocol::system3::{
    AuditResponse, OperationalDirective, ResourceAllocation, ResourceRequest, System3AuditRequest,
};

use super::{RoleContext, ViableSystem};

/// Shared resource-governance role object.
pub type SharedResourceGovernance<V> = Arc<dyn ResourceGovernance<V>>;

/// Shared operational-control role object.
pub type SharedOperationalControlPolicy<V> = Arc<dyn OperationalControlPolicy<V>>;

/// Shared System 3* auditor role object.
pub type SharedAuditor<V> = Arc<dyn Auditor<V>>;

/// Application-owned resource governance policy for System 3.
///
/// The role receives framework-owned resource requests and performance
/// observations, then returns framework-owned allocation decisions. Domain
/// resource semantics belong in this role implementation, not in core.
#[async_trait]
pub trait ResourceGovernance<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn allocate_resources(
        &self,
        context: &RoleContext<V>,
        requests: &[ResourceRequest<V>],
        performance: &[PerformanceObservation<V>],
    ) -> Result<Vec<ResourceAllocation<V>>, FrameworkError>;
}

/// Application-owned operational control policy for System 3 directives.
#[async_trait]
pub trait OperationalControlPolicy<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn plan_directives(
        &self,
        context: &RoleContext<V>,
        allocations: &[ResourceAllocation<V>],
        performance: &[PerformanceObservation<V>],
    ) -> Result<Vec<OperationalDirective<V>>, FrameworkError>;
}

/// Application-owned System 3* auditor.
///
/// Evidence is supplied through an explicit System 3* audit path rather than
/// through ordinary System 1 report history.
#[async_trait]
pub trait Auditor<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn audit(
        &self,
        context: &RoleContext<V>,
        request: &System3AuditRequest<V>,
        evidence: Vec<AuditEvidence<V>>,
    ) -> Result<AuditResponse<V>, FrameworkError>;
}

/// Static catalog of System 3 roles for one application type family.
pub trait System3Roles<V>: Send + Sync + 'static
where
    V: ViableSystem,
{
    type ResourceGovernance: ResourceGovernance<V>;
    type OperationalControlPolicy: OperationalControlPolicy<V>;
    type Auditor: Auditor<V>;
}

/// Opt-in defaults and no-op System 3 policies.
pub mod defaults {
    use crate::protocol::system3::ResourceDecision;

    use super::*;

    /// Resource governance policy that denies every request explicitly.
    #[derive(Debug, Default)]
    pub struct DenyAllResourceGovernance<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> DenyAllResourceGovernance<V>
    where
        V: ViableSystem,
    {
        /// Creates a deny-all resource governance policy.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> ResourceGovernance<V> for DenyAllResourceGovernance<V>
    where
        V: ViableSystem,
    {
        async fn allocate_resources(
            &self,
            context: &RoleContext<V>,
            requests: &[ResourceRequest<V>],
            performance: &[PerformanceObservation<V>],
        ) -> Result<Vec<ResourceAllocation<V>>, FrameworkError> {
            let _ = (context, performance);
            Ok(requests
                .iter()
                .map(|request| {
                    ResourceAllocation::new(request, ResourceDecision::Deny)
                        .with_reason("no resource governance policy configured")
                })
                .collect())
        }
    }

    /// Operational control policy that emits no directives.
    #[derive(Debug, Default)]
    pub struct NoopOperationalControlPolicy<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> NoopOperationalControlPolicy<V>
    where
        V: ViableSystem,
    {
        /// Creates a no-op control policy.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> OperationalControlPolicy<V> for NoopOperationalControlPolicy<V>
    where
        V: ViableSystem,
    {
        async fn plan_directives(
            &self,
            context: &RoleContext<V>,
            allocations: &[ResourceAllocation<V>],
            performance: &[PerformanceObservation<V>],
        ) -> Result<Vec<OperationalDirective<V>>, FrameworkError> {
            let _ = (context, allocations, performance);
            Ok(Vec::new())
        }
    }

    /// Auditor that returns an empty audit response.
    #[derive(Debug, Default)]
    pub struct NoopAuditor<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> NoopAuditor<V>
    where
        V: ViableSystem,
    {
        /// Creates a no-op auditor.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> Auditor<V> for NoopAuditor<V>
    where
        V: ViableSystem,
    {
        async fn audit(
            &self,
            context: &RoleContext<V>,
            request: &System3AuditRequest<V>,
            evidence: Vec<AuditEvidence<V>>,
        ) -> Result<AuditResponse<V>, FrameworkError> {
            let _ = (context, evidence);
            Ok(AuditResponse::new(request, Vec::new(), Vec::new()))
        }
    }
}
