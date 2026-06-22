//! Variety, algedonic, and temporal strategy role contracts.

use std::marker::PhantomData;
use std::sync::Arc;

use ractor::async_trait;

use crate::error::FrameworkError;
use crate::protocol::algedonic::{
    AlgedonicLifecycleStatus, AlgedonicSignalRecord, AlgedonicSnapshot,
};
use crate::protocol::temporal::{TemporalAggregate, TemporalAnalysis, TemporalSample};
use crate::protocol::variety::{
    VarietyCycle, VarietyIntervention, VarietyInterventionOutcome, VarietyObservation,
};

use super::{RoleContext, ViableSystem};

/// Shared variety-engineering policy object.
pub type SharedVarietyEngineeringPolicy<V> = Arc<dyn VarietyEngineeringPolicy<V>>;

/// Shared algedonic lifecycle policy object.
pub type SharedAlgedonicLifecyclePolicy<V> = Arc<dyn AlgedonicLifecyclePolicy<V>>;

/// Shared temporal analysis policy object.
pub type SharedTemporalAnalysisPolicy<V> = Arc<dyn TemporalAnalysisPolicy<V>>;

/// Application-owned policy for attenuation/amplification choices.
#[async_trait]
pub trait VarietyEngineeringPolicy<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn plan_interventions(
        &self,
        context: &RoleContext<V>,
        observation: &VarietyObservation<V>,
    ) -> Result<Vec<VarietyIntervention<V>>, FrameworkError>;

    async fn evaluate_outcomes(
        &self,
        context: &RoleContext<V>,
        cycle: &VarietyCycle<V>,
        outcomes: Vec<VarietyInterventionOutcome<V>>,
    ) -> Result<Vec<VarietyInterventionOutcome<V>>, FrameworkError> {
        let _ = (context, cycle);
        Ok(outcomes)
    }
}

/// Application-owned policy for lifecycle classification and escalation.
#[async_trait]
pub trait AlgedonicLifecyclePolicy<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn classify_signal(
        &self,
        context: &RoleContext<V>,
        signal: AlgedonicSignalRecord<V>,
    ) -> Result<AlgedonicSignalRecord<V>, FrameworkError>;

    async fn escalate_expired(
        &self,
        context: &RoleContext<V>,
        snapshot: &AlgedonicSnapshot<V>,
    ) -> Result<Vec<crate::protocol::algedonic::AlgedonicEscalation<V>>, FrameworkError> {
        let _ = (context, snapshot);
        Ok(Vec::new())
    }
}

/// Application-owned temporal pattern, forecast, and causality strategy.
#[async_trait]
pub trait TemporalAnalysisPolicy<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn analyze_temporal(
        &self,
        context: &RoleContext<V>,
        samples: &[TemporalSample],
        aggregates: &[TemporalAggregate],
    ) -> Result<TemporalAnalysis, FrameworkError>;
}

/// Static catalog for variety, algedonic, and temporal roles.
pub trait VarietyAlgedonicTemporalRoles<V>: Send + Sync + 'static
where
    V: ViableSystem,
{
    type VarietyEngineeringPolicy: VarietyEngineeringPolicy<V>;
    type AlgedonicLifecyclePolicy: AlgedonicLifecyclePolicy<V>;
    type TemporalAnalysisPolicy: TemporalAnalysisPolicy<V>;
}

/// Opt-in defaults and no-op policies.
pub mod defaults {
    use super::*;

    /// Variety policy that proposes no interventions.
    #[derive(Debug, Default)]
    pub struct NoopVarietyEngineeringPolicy<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> NoopVarietyEngineeringPolicy<V>
    where
        V: ViableSystem,
    {
        /// Creates a no-op variety engineering policy.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> VarietyEngineeringPolicy<V> for NoopVarietyEngineeringPolicy<V>
    where
        V: ViableSystem,
    {
        async fn plan_interventions(
            &self,
            context: &RoleContext<V>,
            observation: &VarietyObservation<V>,
        ) -> Result<Vec<VarietyIntervention<V>>, FrameworkError> {
            let _ = (context, observation);
            Ok(Vec::new())
        }
    }

    /// Algedonic policy that only marks signals as classified.
    #[derive(Debug, Default)]
    pub struct DefaultAlgedonicLifecyclePolicy<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> DefaultAlgedonicLifecyclePolicy<V>
    where
        V: ViableSystem,
    {
        /// Creates the default lifecycle policy.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> AlgedonicLifecyclePolicy<V> for DefaultAlgedonicLifecyclePolicy<V>
    where
        V: ViableSystem,
    {
        async fn classify_signal(
            &self,
            context: &RoleContext<V>,
            mut signal: AlgedonicSignalRecord<V>,
        ) -> Result<AlgedonicSignalRecord<V>, FrameworkError> {
            let _ = context;
            if signal.status == AlgedonicLifecycleStatus::Proposed {
                signal.status = AlgedonicLifecycleStatus::Classified;
            }
            Ok(signal)
        }
    }

    /// Temporal policy that returns aggregates with no patterns or forecasts.
    #[derive(Debug, Default)]
    pub struct NoopTemporalAnalysisPolicy<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> NoopTemporalAnalysisPolicy<V>
    where
        V: ViableSystem,
    {
        /// Creates a no-op temporal analysis policy.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> TemporalAnalysisPolicy<V> for NoopTemporalAnalysisPolicy<V>
    where
        V: ViableSystem,
    {
        async fn analyze_temporal(
            &self,
            context: &RoleContext<V>,
            samples: &[TemporalSample],
            aggregates: &[TemporalAggregate],
        ) -> Result<TemporalAnalysis, FrameworkError> {
            let _ = (context, samples);
            Ok(TemporalAnalysis::empty(aggregates.to_vec()))
        }
    }
}
