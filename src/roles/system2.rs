//! System 2 coordination role contracts.

use std::marker::PhantomData;
use std::sync::Arc;

use ractor::async_trait;

use crate::error::FrameworkError;
use crate::protocol::system2::{
    CoordinationConflict, CoordinationIntervention, CoordinationViewRecord,
};

use super::{RoleContext, ViableSystem};

/// Shared coordination policy object.
pub type SharedCoordinationPolicy<V> = Arc<dyn CoordinationPolicy<V>>;

/// Application-owned System 2 coordination policy.
///
/// The policy sees typed System 1 coordination views and returns generic
/// framework-owned conflicts and interventions. Domain scheduling, dependency,
/// and resource meanings belong in the policy implementation rather than in
/// System 2 core.
#[async_trait]
pub trait CoordinationPolicy<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn detect_conflicts(
        &self,
        context: &RoleContext<V>,
        views: &[CoordinationViewRecord<V>],
    ) -> Result<Vec<CoordinationConflict<V>>, FrameworkError>;

    async fn plan_interventions(
        &self,
        context: &RoleContext<V>,
        conflicts: &[CoordinationConflict<V>],
        views: &[CoordinationViewRecord<V>],
    ) -> Result<Vec<CoordinationIntervention<V>>, FrameworkError>;
}

/// Static catalog of System 2 roles for one application type family.
pub trait System2Roles<V>: Send + Sync + 'static
where
    V: ViableSystem,
{
    type CoordinationPolicy: CoordinationPolicy<V>;
}

/// Opt-in default and no-op System 2 policies.
pub mod defaults {
    use super::*;

    /// Coordination policy that reports no conflicts and no interventions.
    #[derive(Debug, Default)]
    pub struct NoopCoordinationPolicy<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> NoopCoordinationPolicy<V>
    where
        V: ViableSystem,
    {
        /// Creates a no-op coordination policy.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> CoordinationPolicy<V> for NoopCoordinationPolicy<V>
    where
        V: ViableSystem,
    {
        async fn detect_conflicts(
            &self,
            context: &RoleContext<V>,
            views: &[CoordinationViewRecord<V>],
        ) -> Result<Vec<CoordinationConflict<V>>, FrameworkError> {
            let _ = (context, views);
            Ok(Vec::new())
        }

        async fn plan_interventions(
            &self,
            context: &RoleContext<V>,
            conflicts: &[CoordinationConflict<V>],
            views: &[CoordinationViewRecord<V>],
        ) -> Result<Vec<CoordinationIntervention<V>>, FrameworkError> {
            let _ = (context, conflicts, views);
            Ok(Vec::new())
        }
    }
}
