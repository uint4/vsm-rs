use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use vsm_rs::error::FrameworkError;
use vsm_rs::protocol::system1::{CapacitySnapshot, UnitDescriptor};
use vsm_rs::protocol::system4::{
    AdaptationProposal, EnvironmentSourceDescriptor, EnvironmentalMeasurement,
    EnvironmentalObservation, Forecast, ForecastCalibration, ForecastPoint, FreshnessStatus,
    IntelligenceAssessment, InterpretedSignal, Scenario, SignalKind,
};
use vsm_rs::protocol::{RuntimeId, SubsystemRole};
use vsm_rs::roles::system1::testing::{AcceptAllWorkModel, StaticOperationalUnitFactory};
use vsm_rs::roles::{RoleContext, ViableSystem};
use vsm_rs::{
    async_trait, EnvironmentalSource, EnvironmentalSourceFactory, Forecaster, IntelligenceModel,
    SignalInterpreter, VsmBuilder,
};

#[derive(Clone, Debug)]
struct DomainWork;

#[derive(Clone, Debug)]
struct DomainOutcome;

#[derive(Debug)]
struct DomainError;

impl Display for DomainError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("domain error")
    }
}

impl std::error::Error for DomainError {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct DomainCapability(&'static str);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct DomainUnitId(&'static str);

struct DomainSnapshot;

struct DomainSystem;

impl ViableSystem for DomainSystem {
    type Work = DomainWork;
    type Outcome = DomainOutcome;
    type AppError = DomainError;
    type Capability = DomainCapability;
    type UnitId = DomainUnitId;
    type UnitSnapshot = DomainSnapshot;
}

#[tokio::test]
async fn system4_cycle_routes_proposals_with_system3_feasibility() {
    let factory = TestSourceFactory::new();
    let runtime = builder_for("system4-cycle", factory)
        .start()
        .await
        .expect("runtime should start");
    let system4 = runtime.system4();

    let status = system4
        .register_source(EnvironmentSourceDescriptor::new("market").with_provenance(["fixture"]))
        .await
        .expect("source should register");
    assert_eq!(status.observation_count, 0);

    let cycle = system4
        .run_intelligence_cycle()
        .await
        .expect("cycle should run");

    assert_eq!(cycle.observations.len(), 1);
    assert_eq!(cycle.signals.len(), 1);
    assert_eq!(cycle.forecasts.len(), 1);
    assert_eq!(cycle.scenarios.len(), 1);
    assert_eq!(cycle.proposals.len(), 1);
    assert!(!cycle.scenarios[0].provenance.is_empty());
    assert!(!cycle.proposals[0].provenance.is_empty());
    assert_eq!(
        cycle.proposals[0]
            .destination
            .as_ref()
            .expect("proposal should be routed")
            .role,
        SubsystemRole::System5
    );
    assert!(cycle.proposals[0]
        .feasibility
        .as_ref()
        .expect("proposal should include feasibility")
        .summary
        .contains("System 3 snapshot observed"));

    let snapshot = system4.snapshot().await.expect("snapshot should return");
    assert_eq!(snapshot.sources[0].observation_count, 1);
    assert!(snapshot.proposals[0].feasibility.is_some());

    runtime.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
async fn system4_detects_stale_observations_and_calibrates_forecasts() {
    let factory = TestSourceFactory::new().stale();
    let runtime = builder_for("system4-stale", factory)
        .start()
        .await
        .expect("runtime should start");
    let system4 = runtime.system4();

    system4
        .register_source(
            EnvironmentSourceDescriptor::new("stale-market")
                .with_stale_after(Duration::from_millis(1)),
        )
        .await
        .expect("source should register");
    let cycle = system4
        .run_intelligence_cycle()
        .await
        .expect("cycle should run");

    assert_eq!(cycle.observations[0].freshness, FreshnessStatus::Expired);
    assert_eq!(cycle.stale_sources.len(), 1);

    let calibrations = system4
        .calibrate_forecasts(cycle.observations.clone())
        .await
        .expect("calibration should run");
    assert_eq!(calibrations.len(), 1);
    assert_eq!(calibrations[0].sample_size, 1);

    runtime.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
async fn failing_source_restarts_without_stopping_system4() {
    let created_sources = Arc::new(AtomicUsize::new(0));
    let factory = TestSourceFactory::new()
        .fail_first_observation()
        .with_create_count(Arc::clone(&created_sources));
    let runtime = builder_for("system4-source-restart", factory)
        .start()
        .await
        .expect("runtime should start");
    let system4 = runtime.system4();

    system4
        .register_source(EnvironmentSourceDescriptor::new("flaky-source"))
        .await
        .expect("source should register");

    let first = system4
        .collect_observations()
        .await
        .expect("failed source should be contained");
    assert!(first.is_empty());
    let failed_status = system4
        .list_sources()
        .await
        .expect("sources should list")
        .remove(0);
    assert_eq!(failed_status.restart_count, 1);
    assert!(failed_status.last_error.is_some());

    let second = system4
        .collect_observations()
        .await
        .expect("restarted source should collect");
    assert_eq!(second.len(), 1);
    assert!(created_sources.load(Ordering::SeqCst) >= 2);

    let snapshot = system4.snapshot().await.expect("snapshot should return");
    assert_eq!(snapshot.sources[0].observation_count, 1);

    runtime.shutdown().await.expect("shutdown should succeed");
}

#[derive(Clone)]
struct TestSourceFactory {
    fail_first: Arc<AtomicBool>,
    stale: bool,
    create_count: Arc<AtomicUsize>,
}

impl TestSourceFactory {
    fn new() -> Self {
        Self {
            fail_first: Arc::new(AtomicBool::new(false)),
            stale: false,
            create_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn fail_first_observation(self) -> Self {
        self.fail_first.store(true, Ordering::SeqCst);
        self
    }

    fn stale(mut self) -> Self {
        self.stale = true;
        self
    }

    fn with_create_count(mut self, create_count: Arc<AtomicUsize>) -> Self {
        self.create_count = create_count;
        self
    }
}

#[async_trait]
impl EnvironmentalSourceFactory<DomainSystem> for TestSourceFactory {
    async fn create_source(
        &self,
        _context: &RoleContext<DomainSystem>,
        descriptor: &EnvironmentSourceDescriptor,
    ) -> Result<Box<dyn EnvironmentalSource<DomainSystem>>, FrameworkError> {
        self.create_count.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(TestSource {
            source_id: descriptor.source_id.clone(),
            fail_first: Arc::clone(&self.fail_first),
            stale: self.stale,
        }))
    }
}

struct TestSource {
    source_id: String,
    fail_first: Arc<AtomicBool>,
    stale: bool,
}

#[async_trait]
impl EnvironmentalSource<DomainSystem> for TestSource {
    async fn observe(
        &mut self,
        _context: &RoleContext<DomainSystem>,
        _descriptor: &EnvironmentSourceDescriptor,
    ) -> Result<Vec<EnvironmentalObservation>, FrameworkError> {
        if self.fail_first.swap(false, Ordering::SeqCst) {
            return Err(FrameworkError::Runtime {
                reason: "source unavailable once".to_string(),
            });
        }

        let mut observation = EnvironmentalObservation::new(self.source_id.clone())
            .with_measurement(EnvironmentalMeasurement::new("demand", 0.72))
            .with_confidence(0.8)
            .with_summary("market demand");
        if self.stale {
            observation.observed_at = Utc::now() - chrono::Duration::seconds(60);
        }

        Ok(vec![observation])
    }
}

struct TestInterpreter;

#[async_trait]
impl SignalInterpreter<DomainSystem> for TestInterpreter {
    async fn interpret(
        &self,
        _context: &RoleContext<DomainSystem>,
        observations: &[EnvironmentalObservation],
    ) -> Result<Vec<InterpretedSignal>, FrameworkError> {
        Ok(observations
            .iter()
            .map(|observation| {
                InterpretedSignal::new(SignalKind::Opportunity)
                    .from_observation(observation)
                    .with_strength(0.7)
                    .with_uncertainty(0.2)
                    .with_rationale("positive demand signal")
            })
            .collect())
    }
}

struct TestIntelligenceModel;

#[async_trait]
impl IntelligenceModel<DomainSystem> for TestIntelligenceModel {
    async fn assess(
        &self,
        _context: &RoleContext<DomainSystem>,
        signals: &[InterpretedSignal],
    ) -> Result<IntelligenceAssessment, FrameworkError> {
        let mut assessment = IntelligenceAssessment::new(signals.to_vec());
        assessment.summary = Some("one external opportunity".to_string());
        Ok(assessment)
    }
}

struct TestForecaster;

#[async_trait]
impl Forecaster<DomainSystem> for TestForecaster {
    async fn forecast(
        &self,
        _context: &RoleContext<DomainSystem>,
        assessment: &IntelligenceAssessment,
        _signals: &[InterpretedSignal],
    ) -> Result<Vec<Forecast>, FrameworkError> {
        let mut forecast = Forecast::new(assessment, Duration::from_secs(60));
        forecast.points.push(ForecastPoint::new(
            Duration::from_secs(60),
            assessment.opportunity_score,
            0.8,
        ));
        Ok(vec![forecast])
    }

    async fn plan_scenarios(
        &self,
        _context: &RoleContext<DomainSystem>,
        _assessment: &IntelligenceAssessment,
        forecasts: &[Forecast],
    ) -> Result<Vec<Scenario>, FrameworkError> {
        let mut scenario = Scenario::new("demand growth");
        scenario.forecast_id = forecasts
            .first()
            .map(|forecast| forecast.forecast_id.clone());
        scenario.probability = 0.6;
        scenario.impact = 0.7;
        scenario.uncertainty = 0.25;
        Ok(vec![scenario])
    }

    async fn propose_adaptations(
        &self,
        _context: &RoleContext<DomainSystem>,
        _assessment: &IntelligenceAssessment,
        _forecasts: &[Forecast],
        scenarios: &[Scenario],
    ) -> Result<Vec<AdaptationProposal>, FrameworkError> {
        let mut proposal =
            AdaptationProposal::new("increase sensing cadence", "demand growth scenario");
        proposal.scenario_id = scenarios
            .first()
            .map(|scenario| scenario.scenario_id.clone());
        proposal.expected_benefit = 0.55;
        proposal.uncertainty = 0.25;
        Ok(vec![proposal])
    }

    async fn calibrate(
        &self,
        _context: &RoleContext<DomainSystem>,
        forecasts: &[Forecast],
        actuals: &[EnvironmentalObservation],
    ) -> Result<Vec<ForecastCalibration>, FrameworkError> {
        Ok(forecasts
            .iter()
            .map(|forecast| {
                let mut calibration =
                    ForecastCalibration::new(forecast.forecast_id.clone(), actuals.len());
                calibration.mean_absolute_error = 0.1;
                calibration.uncertainty_after = 0.2;
                calibration
            })
            .collect())
    }
}

fn builder_for(runtime_id: &'static str, factory: TestSourceFactory) -> VsmBuilder<DomainSystem> {
    let descriptor =
        UnitDescriptor::<DomainSystem>::new(DomainUnitId("unit-a"), [DomainCapability("work")]);
    let capacity = CapacitySnapshot::new(0, Some(4), 0.0);

    VsmBuilder::new()
        .runtime_id(RuntimeId::from_string(runtime_id))
        .work_model(AcceptAllWorkModel::new([DomainCapability("work")]))
        .operational_unit_factory(StaticOperationalUnitFactory::new(
            descriptor,
            capacity,
            DomainOutcome,
        ))
        .environmental_source_factory(factory)
        .signal_interpreter(TestInterpreter)
        .intelligence_model(TestIntelligenceModel)
        .forecaster(TestForecaster)
}
