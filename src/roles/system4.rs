//! System 4 environmental source, intelligence, and forecasting role contracts.

use std::marker::PhantomData;
use std::sync::Arc;

use ractor::async_trait;

use crate::error::FrameworkError;
use crate::protocol::system4::{
    AdaptationProposal, EnvironmentSourceDescriptor, EnvironmentalObservation, Forecast,
    ForecastCalibration, IntelligenceAssessment, InterpretedSignal, Scenario,
};

use super::{RoleContext, ViableSystem};

/// Boxed environmental source instance owned by one source actor.
pub type BoxEnvironmentalSource<V> = Box<dyn EnvironmentalSource<V>>;

/// Shared environmental source factory role object.
pub type SharedEnvironmentalSourceFactory<V> = Arc<dyn EnvironmentalSourceFactory<V>>;

/// Shared signal-interpreter role object.
pub type SharedSignalInterpreter<V> = Arc<dyn SignalInterpreter<V>>;

/// Shared intelligence-model role object.
pub type SharedIntelligenceModel<V> = Arc<dyn IntelligenceModel<V>>;

/// Shared forecaster role object.
pub type SharedForecaster<V> = Arc<dyn Forecaster<V>>;

/// Application-owned environmental source instance.
///
/// The runtime owns polling and supervision. The source role owns how external
/// environmental information is gathered and converted into normalized
/// observations.
#[async_trait]
pub trait EnvironmentalSource<V>: Send
where
    V: ViableSystem,
{
    async fn observe(
        &mut self,
        context: &RoleContext<V>,
        descriptor: &EnvironmentSourceDescriptor,
    ) -> Result<Vec<EnvironmentalObservation>, FrameworkError>;
}

/// Shared async factory that creates fresh source instances.
#[async_trait]
pub trait EnvironmentalSourceFactory<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn create_source(
        &self,
        context: &RoleContext<V>,
        descriptor: &EnvironmentSourceDescriptor,
    ) -> Result<BoxEnvironmentalSource<V>, FrameworkError>;
}

/// Application-owned signal interpretation policy.
#[async_trait]
pub trait SignalInterpreter<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn interpret(
        &self,
        context: &RoleContext<V>,
        observations: &[EnvironmentalObservation],
    ) -> Result<Vec<InterpretedSignal>, FrameworkError>;
}

/// Application-owned intelligence assessment model.
#[async_trait]
pub trait IntelligenceModel<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn assess(
        &self,
        context: &RoleContext<V>,
        signals: &[InterpretedSignal],
    ) -> Result<IntelligenceAssessment, FrameworkError>;
}

/// Application-owned forecasting and scenario-planning policy.
#[async_trait]
pub trait Forecaster<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn forecast(
        &self,
        context: &RoleContext<V>,
        assessment: &IntelligenceAssessment,
        signals: &[InterpretedSignal],
    ) -> Result<Vec<Forecast>, FrameworkError>;

    async fn plan_scenarios(
        &self,
        context: &RoleContext<V>,
        assessment: &IntelligenceAssessment,
        forecasts: &[Forecast],
    ) -> Result<Vec<Scenario>, FrameworkError>;

    async fn propose_adaptations(
        &self,
        context: &RoleContext<V>,
        assessment: &IntelligenceAssessment,
        forecasts: &[Forecast],
        scenarios: &[Scenario],
    ) -> Result<Vec<AdaptationProposal>, FrameworkError>;

    async fn calibrate(
        &self,
        context: &RoleContext<V>,
        forecasts: &[Forecast],
        actuals: &[EnvironmentalObservation],
    ) -> Result<Vec<ForecastCalibration>, FrameworkError>;
}

/// Static catalog of System 4 roles for one application type family.
pub trait System4Roles<V>: Send + Sync + 'static
where
    V: ViableSystem,
{
    type EnvironmentalSourceFactory: EnvironmentalSourceFactory<V>;
    type SignalInterpreter: SignalInterpreter<V>;
    type IntelligenceModel: IntelligenceModel<V>;
    type Forecaster: Forecaster<V>;
}

/// Opt-in defaults and no-op System 4 policies.
pub mod defaults {
    use super::*;

    /// Environmental source factory that creates no-op sources.
    #[derive(Debug, Default)]
    pub struct NoopEnvironmentalSourceFactory<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> NoopEnvironmentalSourceFactory<V>
    where
        V: ViableSystem,
    {
        /// Creates a no-op source factory.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> EnvironmentalSourceFactory<V> for NoopEnvironmentalSourceFactory<V>
    where
        V: ViableSystem,
    {
        async fn create_source(
            &self,
            context: &RoleContext<V>,
            descriptor: &EnvironmentSourceDescriptor,
        ) -> Result<BoxEnvironmentalSource<V>, FrameworkError> {
            let _ = (context, descriptor);
            Ok(Box::new(NoopEnvironmentalSource::<V>::new()))
        }
    }

    /// Environmental source that emits no observations.
    #[derive(Debug, Default)]
    pub struct NoopEnvironmentalSource<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> NoopEnvironmentalSource<V>
    where
        V: ViableSystem,
    {
        /// Creates a no-op environmental source.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> EnvironmentalSource<V> for NoopEnvironmentalSource<V>
    where
        V: ViableSystem,
    {
        async fn observe(
            &mut self,
            context: &RoleContext<V>,
            descriptor: &EnvironmentSourceDescriptor,
        ) -> Result<Vec<EnvironmentalObservation>, FrameworkError> {
            let _ = (context, descriptor);
            Ok(Vec::new())
        }
    }

    /// Signal interpreter that emits no interpreted signals.
    #[derive(Debug, Default)]
    pub struct NoopSignalInterpreter<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> NoopSignalInterpreter<V>
    where
        V: ViableSystem,
    {
        /// Creates a no-op signal interpreter.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> SignalInterpreter<V> for NoopSignalInterpreter<V>
    where
        V: ViableSystem,
    {
        async fn interpret(
            &self,
            context: &RoleContext<V>,
            observations: &[EnvironmentalObservation],
        ) -> Result<Vec<InterpretedSignal>, FrameworkError> {
            let _ = (context, observations);
            Ok(Vec::new())
        }
    }

    /// Intelligence model that returns an empty assessment.
    #[derive(Debug, Default)]
    pub struct NoopIntelligenceModel<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> NoopIntelligenceModel<V>
    where
        V: ViableSystem,
    {
        /// Creates a no-op intelligence model.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> IntelligenceModel<V> for NoopIntelligenceModel<V>
    where
        V: ViableSystem,
    {
        async fn assess(
            &self,
            context: &RoleContext<V>,
            signals: &[InterpretedSignal],
        ) -> Result<IntelligenceAssessment, FrameworkError> {
            let _ = (context, signals);
            Ok(IntelligenceAssessment::empty())
        }
    }

    /// Forecaster that emits no forecasts, scenarios, proposals, or calibrations.
    #[derive(Debug, Default)]
    pub struct NoopForecaster<V>
    where
        V: ViableSystem,
    {
        _system: PhantomData<V>,
    }

    impl<V> NoopForecaster<V>
    where
        V: ViableSystem,
    {
        /// Creates a no-op forecaster.
        pub fn new() -> Self {
            Self {
                _system: PhantomData,
            }
        }
    }

    #[async_trait]
    impl<V> Forecaster<V> for NoopForecaster<V>
    where
        V: ViableSystem,
    {
        async fn forecast(
            &self,
            context: &RoleContext<V>,
            assessment: &IntelligenceAssessment,
            signals: &[InterpretedSignal],
        ) -> Result<Vec<Forecast>, FrameworkError> {
            let _ = (context, assessment, signals);
            Ok(Vec::new())
        }

        async fn plan_scenarios(
            &self,
            context: &RoleContext<V>,
            assessment: &IntelligenceAssessment,
            forecasts: &[Forecast],
        ) -> Result<Vec<Scenario>, FrameworkError> {
            let _ = (context, assessment, forecasts);
            Ok(Vec::new())
        }

        async fn propose_adaptations(
            &self,
            context: &RoleContext<V>,
            assessment: &IntelligenceAssessment,
            forecasts: &[Forecast],
            scenarios: &[Scenario],
        ) -> Result<Vec<AdaptationProposal>, FrameworkError> {
            let _ = (context, assessment, forecasts, scenarios);
            Ok(Vec::new())
        }

        async fn calibrate(
            &self,
            context: &RoleContext<V>,
            forecasts: &[Forecast],
            actuals: &[EnvironmentalObservation],
        ) -> Result<Vec<ForecastCalibration>, FrameworkError> {
            let _ = (context, forecasts, actuals);
            Ok(Vec::new())
        }
    }
}
