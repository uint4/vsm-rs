//! System 5 identity, values, decision, and crisis role contracts.

use std::marker::PhantomData;
use std::sync::Arc;

use ractor::async_trait;

use crate::error::FrameworkError;
use crate::protocol::system5::{
    CrisisResponse, CrisisSignal, DecisionRecord, DecisionRequest, DecisionStatus, IdentityRecord,
    System5DecisionCycle, ValueSet, ValuesEvaluation,
};

use super::{RoleContext, ViableSystem};

/// Shared identity-provider role object.
pub type SharedIdentityProvider<V> = Arc<dyn IdentityProvider<V>>;

/// Shared values-provider role object.
pub type SharedValuesProvider<V> = Arc<dyn ValuesProvider<V>>;

/// Shared values-evaluator role object.
pub type SharedValuesEvaluator<V> = Arc<dyn ValuesEvaluator<V>>;

/// Shared decision-policy role object.
pub type SharedDecisionPolicy<V> = Arc<dyn DecisionPolicy<V>>;

/// Shared crisis-policy role object.
pub type SharedCrisisPolicy<V> = Arc<dyn CrisisPolicy<V>>;

/// Application-owned identity data provider.
#[async_trait]
pub trait IdentityProvider<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn provide_identity(
        &self,
        context: &RoleContext<V>,
    ) -> Result<IdentityRecord, FrameworkError>;
}

/// Application-owned values data provider.
#[async_trait]
pub trait ValuesProvider<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn provide_values(&self, context: &RoleContext<V>) -> Result<ValueSet, FrameworkError>;
}

/// Application-owned identity and values evaluation behavior.
#[async_trait]
pub trait ValuesEvaluator<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn evaluate_values(
        &self,
        context: &RoleContext<V>,
        request: &DecisionRequest<V>,
        identity: &IdentityRecord,
        values: &ValueSet,
    ) -> Result<ValuesEvaluation, FrameworkError>;
}

/// Application-owned strategic decision procedure.
#[async_trait]
pub trait DecisionPolicy<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn decide(
        &self,
        context: &RoleContext<V>,
        request: &DecisionRequest<V>,
        identity: &IdentityRecord,
        values: &ValueSet,
        evaluation: &ValuesEvaluation,
    ) -> Result<DecisionRecord<V>, FrameworkError>;
}

/// Application-owned crisis response behavior.
#[async_trait]
pub trait CrisisPolicy<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn respond_to_crisis(
        &self,
        context: &RoleContext<V>,
        signal: &CrisisSignal,
        identity: &IdentityRecord,
        values: &ValueSet,
    ) -> Result<CrisisResponse<V>, FrameworkError>;
}

/// Static catalog of System 5 roles for one application type family.
pub trait System5Roles<V>: Send + Sync + 'static
where
    V: ViableSystem,
{
    type IdentityProvider: IdentityProvider<V>;
    type ValuesProvider: ValuesProvider<V>;
    type ValuesEvaluator: ValuesEvaluator<V>;
    type DecisionPolicy: DecisionPolicy<V>;
    type CrisisPolicy: CrisisPolicy<V>;
}

/// Opt-in defaults and no-op System 5 policies.
pub mod defaults {
    use super::*;

    /// Identity provider that returns an explicitly unconfigured identity.
    #[derive(Debug, Default)]
    pub struct NoopIdentityProvider<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> NoopIdentityProvider<V>
    where
        V: ViableSystem,
    {
        /// Creates a no-op identity provider.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> IdentityProvider<V> for NoopIdentityProvider<V>
    where
        V: ViableSystem,
    {
        async fn provide_identity(
            &self,
            context: &RoleContext<V>,
        ) -> Result<IdentityRecord, FrameworkError> {
            let _ = context;
            Ok(IdentityRecord::new("unconfigured identity"))
        }
    }

    /// Values provider that returns an empty value set.
    #[derive(Debug, Default)]
    pub struct NoopValuesProvider<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> NoopValuesProvider<V>
    where
        V: ViableSystem,
    {
        /// Creates a no-op values provider.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> ValuesProvider<V> for NoopValuesProvider<V>
    where
        V: ViableSystem,
    {
        async fn provide_values(
            &self,
            context: &RoleContext<V>,
        ) -> Result<ValueSet, FrameworkError> {
            let _ = context;
            Ok(ValueSet::empty())
        }
    }

    /// Values evaluator that applies no normative constraints.
    #[derive(Debug, Default)]
    pub struct NoopValuesEvaluator<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> NoopValuesEvaluator<V>
    where
        V: ViableSystem,
    {
        /// Creates a no-op values evaluator.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> ValuesEvaluator<V> for NoopValuesEvaluator<V>
    where
        V: ViableSystem,
    {
        async fn evaluate_values(
            &self,
            context: &RoleContext<V>,
            request: &DecisionRequest<V>,
            identity: &IdentityRecord,
            values: &ValueSet,
        ) -> Result<ValuesEvaluation, FrameworkError> {
            let _ = (context, request);
            Ok(ValuesEvaluation::neutral(identity, values))
        }
    }

    /// Decision policy that defers every request without issuing directives.
    #[derive(Debug, Default)]
    pub struct NoopDecisionPolicy<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> NoopDecisionPolicy<V>
    where
        V: ViableSystem,
    {
        /// Creates a no-op decision policy.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> DecisionPolicy<V> for NoopDecisionPolicy<V>
    where
        V: ViableSystem,
    {
        async fn decide(
            &self,
            context: &RoleContext<V>,
            request: &DecisionRequest<V>,
            identity: &IdentityRecord,
            values: &ValueSet,
            evaluation: &ValuesEvaluation,
        ) -> Result<DecisionRecord<V>, FrameworkError> {
            let _ = context;
            Ok(DecisionRecord::new(
                request,
                identity,
                values,
                DecisionStatus::Deferred,
                "no System 5 decision policy configured",
            )
            .with_evaluation(evaluation.clone()))
        }
    }

    /// Crisis policy that records the crisis without issuing doctrine.
    #[derive(Debug, Default)]
    pub struct NoopCrisisPolicy<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> NoopCrisisPolicy<V>
    where
        V: ViableSystem,
    {
        /// Creates a no-op crisis policy.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> CrisisPolicy<V> for NoopCrisisPolicy<V>
    where
        V: ViableSystem,
    {
        async fn respond_to_crisis(
            &self,
            context: &RoleContext<V>,
            signal: &CrisisSignal,
            identity: &IdentityRecord,
            values: &ValueSet,
        ) -> Result<CrisisResponse<V>, FrameworkError> {
            let _ = context;
            let mut request =
                DecisionRequest::<V>::new(format!("crisis: {}", signal.summary.clone()));
            request.evidence = signal.evidence.clone();
            let evaluation = ValuesEvaluation::neutral(identity, values);
            let decision = DecisionRecord::new(
                &request,
                identity,
                values,
                DecisionStatus::Crisis,
                "no System 5 crisis policy configured",
            )
            .with_evaluation(evaluation);
            Ok(CrisisResponse::new(signal, decision))
        }
    }

    /// Bundled no-op System 5 roles.
    pub fn noop_roles<V>() -> System5RoleDefaults<V>
    where
        V: ViableSystem,
    {
        System5RoleDefaults {
            identity_provider: NoopIdentityProvider::new(),
            values_provider: NoopValuesProvider::new(),
            values_evaluator: NoopValuesEvaluator::new(),
            decision_policy: NoopDecisionPolicy::new(),
            crisis_policy: NoopCrisisPolicy::new(),
        }
    }

    /// Concrete no-op role bundle for static tests and examples.
    #[derive(Debug, Default)]
    pub struct System5RoleDefaults<V>
    where
        V: ViableSystem,
    {
        pub identity_provider: NoopIdentityProvider<V>,
        pub values_provider: NoopValuesProvider<V>,
        pub values_evaluator: NoopValuesEvaluator<V>,
        pub decision_policy: NoopDecisionPolicy<V>,
        pub crisis_policy: NoopCrisisPolicy<V>,
    }

    impl<V> System5Roles<V> for System5RoleDefaults<V>
    where
        V: ViableSystem,
    {
        type IdentityProvider = NoopIdentityProvider<V>;
        type ValuesProvider = NoopValuesProvider<V>;
        type ValuesEvaluator = NoopValuesEvaluator<V>;
        type DecisionPolicy = NoopDecisionPolicy<V>;
        type CrisisPolicy = NoopCrisisPolicy<V>;
    }

    impl<V> System5RoleDefaults<V>
    where
        V: ViableSystem,
    {
        /// Runs a complete no-op decision cycle for tests.
        pub async fn decide(
            &self,
            context: &RoleContext<V>,
            request: DecisionRequest<V>,
        ) -> Result<System5DecisionCycle<V>, FrameworkError> {
            let identity = self.identity_provider.provide_identity(context).await?;
            let values = self.values_provider.provide_values(context).await?;
            let evaluation = self
                .values_evaluator
                .evaluate_values(context, &request, &identity, &values)
                .await?;
            let decision = self
                .decision_policy
                .decide(context, &request, &identity, &values, &evaluation)
                .await?;
            Ok(System5DecisionCycle {
                metadata: request.metadata.child(),
                request,
                identity,
                values,
                evaluation: evaluation.clone(),
                directive_acknowledgements: Vec::new(),
                escalations: decision.escalations.clone(),
                decided_at: decision.decided_at,
                decision,
            })
        }
    }
}
