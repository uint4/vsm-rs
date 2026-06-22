use std::fmt::{Display, Formatter};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use serde_json::json;
use vsm_rs::async_trait;
use vsm_rs::channels::algedonic::signals::{
    create_signal, Severity as LegacySeverity, SignalKind as LegacySignalKind,
};
use vsm_rs::error::FrameworkError;
use vsm_rs::protocol::algedonic::{
    AlgedonicLifecycleStatus, AlgedonicSeverity, AlgedonicSignalKind, AlgedonicSignalRecord,
};
use vsm_rs::protocol::system1::{CapacitySnapshot, UnitDescriptor};
use vsm_rs::protocol::temporal::{
    CausalHypothesis, TemporalAnalysis, TemporalForecast, TemporalForecastPoint, TemporalPattern,
    TemporalPatternKind, TemporalSample,
};
use vsm_rs::protocol::variety::{
    VarietyEstimate, VarietyIntervention, VarietyInterventionKind, VarietyInterventionOutcome,
    VarietyObservation,
};
use vsm_rs::protocol::{RuntimeEvent, RuntimeId};
use vsm_rs::roles::system1::testing::{AcceptAllWorkModel, StaticOperationalUnitFactory};
use vsm_rs::roles::{
    AlertRecord, AlertSink, AlgedonicLifecyclePolicy, RoleContext, TemporalAnalysisPolicy,
    VarietyEngineeringPolicy, ViableSystem,
};
use vsm_rs::{ChannelKind, MessageKind, SystemId, VsmBuilder, VsmMessage};

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
struct Capability(&'static str);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct UnitId(&'static str);

struct DomainSnapshot;

struct DomainSystem;

impl ViableSystem for DomainSystem {
    type Work = DomainWork;
    type Outcome = DomainOutcome;
    type AppError = DomainError;
    type Capability = Capability;
    type UnitId = UnitId;
    type UnitSnapshot = DomainSnapshot;
}

#[tokio::test]
async fn variety_policy_records_interventions_and_outcomes() {
    let runtime = runtime_builder("variety-cycle")
        .variety_engineering_policy(PlanningVarietyPolicy)
        .start()
        .await
        .expect("runtime should start");

    let cycle = runtime
        .variety()
        .record_variety(VarietyObservation::new(VarietyEstimate::new(10.0, 6.0)))
        .await
        .expect("variety observation should run");

    assert_eq!(cycle.interventions.len(), 1);
    assert_eq!(cycle.interventions[0].target_units, vec![UnitId("unit-a")]);
    assert!(matches!(
        cycle.interventions[0].kind,
        VarietyInterventionKind::Amplification
    ));

    let snapshot = runtime
        .variety()
        .record_variety_outcomes(vec![VarietyInterventionOutcome::accepted(
            &cycle.interventions[0],
        )])
        .await
        .expect("outcome should record");

    assert_eq!(snapshot.variety.observations.len(), 1);
    assert_eq!(snapshot.variety.interventions.len(), 1);
    assert_eq!(snapshot.variety.outcomes.len(), 1);

    runtime.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
async fn high_priority_algedonic_signal_dispatches_to_system5_and_alert_sink() {
    let alerts = Arc::new(Mutex::new(Vec::new()));
    let runtime = runtime_builder("algedonic-dispatch")
        .algedonic_lifecycle_policy(ClassifyingAlgedonicPolicy)
        .alert_sink(CapturingAlertSink {
            alerts: Arc::clone(&alerts),
        })
        .start()
        .await
        .expect("runtime should start");

    let signal = AlgedonicSignalRecord::<DomainSystem>::new(
        AlgedonicSignalKind::Pain,
        AlgedonicSeverity::Critical,
        "primary unit saturated",
    )
    .with_priority(0.98)
    .from_source("test-sensor");

    let cycle = runtime
        .variety()
        .handle_algedonic_signal(signal)
        .await
        .expect("algedonic signal should run");

    assert_eq!(cycle.signal.status, AlgedonicLifecycleStatus::Dispatched);
    assert!(cycle.crisis_response.is_some());
    assert_eq!(alerts.lock().expect("alerts mutex").len(), 1);
    assert!(runtime
        .observer_event_history()
        .expect("event history should return")
        .iter()
        .any(|event| matches!(
            event,
            RuntimeEvent::Algedonic(algedonic)
                if matches!(
                    &**algedonic,
                    vsm_rs::protocol::AlgedonicEvent::SignalDispatched { .. }
                )
        )));

    runtime.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
async fn legacy_and_advanced_algedonic_inputs_bridge_to_typed_lifecycle() {
    let runtime = runtime_builder("algedonic-bridge")
        .algedonic_lifecycle_policy(ClassifyingAlgedonicPolicy)
        .start()
        .await
        .expect("runtime should start");

    let legacy = VsmMessage::new(
        SystemId::System1,
        SystemId::System5,
        ChannelKind::Algedonic,
        MessageKind::PainSignal,
        json!({
            "severity": "high",
            "priority": 0.8,
            "reason": "legacy pressure spike"
        }),
    );
    let legacy_cycle = runtime
        .variety()
        .handle_legacy_algedonic_message(legacy)
        .await
        .expect("legacy signal should bridge");

    assert_eq!(legacy_cycle.signal.kind, AlgedonicSignalKind::Pain);
    assert_eq!(legacy_cycle.signal.severity, AlgedonicSeverity::High);
    assert_eq!(legacy_cycle.signal.source_label.as_deref(), Some("system1"));
    assert!(legacy_cycle.crisis_response.is_some());

    let advanced = create_signal(
        LegacySignalKind::Pleasure,
        "advanced-source",
        json!({ "message": "capacity recovered" }),
        LegacySeverity::Medium,
    );
    let advanced_cycle = runtime
        .variety()
        .handle_advanced_algedonic_signal(advanced)
        .await
        .expect("advanced signal should bridge");

    assert_eq!(advanced_cycle.signal.kind, AlgedonicSignalKind::Pleasure);
    assert_eq!(
        advanced_cycle.signal.source_label.as_deref(),
        Some("advanced-source")
    );
    assert!(advanced_cycle.crisis_response.is_none());

    runtime.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
async fn temporal_policy_receives_aggregates_and_records_analysis() {
    let runtime = runtime_builder("temporal-analysis")
        .temporal_analysis_policy(TrendTemporalPolicy)
        .start()
        .await
        .expect("runtime should start");

    runtime
        .variety()
        .record_temporal_sample(TemporalSample::new("minute", 2.0))
        .await
        .expect("first sample should record");
    runtime
        .variety()
        .record_temporal_sample(TemporalSample::new("minute", 4.0))
        .await
        .expect("second sample should record");

    let analysis = runtime
        .variety()
        .analyze_temporal()
        .await
        .expect("temporal analysis should run");

    assert_eq!(analysis.aggregates.len(), 1);
    assert_eq!(analysis.aggregates[0].mean, 3.0);
    assert_eq!(analysis.patterns.len(), 1);
    assert_eq!(analysis.forecasts.len(), 1);
    assert_eq!(analysis.causal_hypotheses.len(), 1);

    let snapshot = runtime.variety().snapshot().await.expect("snapshot");
    assert_eq!(snapshot.temporal.samples.len(), 2);
    assert_eq!(snapshot.temporal.analyses.len(), 1);

    runtime.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
async fn expired_algedonic_signal_records_escalation() {
    let runtime = runtime_builder("algedonic-expiry")
        .algedonic_lifecycle_policy(ClassifyingAlgedonicPolicy)
        .start()
        .await
        .expect("runtime should start");

    let mut signal = AlgedonicSignalRecord::<DomainSystem>::new(
        AlgedonicSignalKind::Anomaly,
        AlgedonicSeverity::Medium,
        "ack overdue",
    )
    .with_priority(0.5);
    signal.acknowledgement_deadline = Some(Utc::now() - ChronoDuration::seconds(5));

    let cycle = runtime
        .variety()
        .handle_algedonic_signal(signal)
        .await
        .expect("algedonic signal should record");
    assert_eq!(cycle.signal.status, AlgedonicLifecycleStatus::Classified);

    let escalations = runtime
        .variety()
        .expire_algedonic(Utc::now())
        .await
        .expect("expiry should escalate");

    assert_eq!(escalations.len(), 1);
    let snapshot = runtime
        .variety()
        .algedonic_snapshot()
        .await
        .expect("snapshot");
    assert_eq!(snapshot.escalations.len(), 1);
    assert_eq!(
        snapshot.signals[0].status,
        AlgedonicLifecycleStatus::Expired
    );

    runtime.shutdown().await.expect("shutdown should succeed");
}

fn runtime_builder(runtime_id: &'static str) -> VsmBuilder<DomainSystem> {
    let descriptor = UnitDescriptor::<DomainSystem>::new(UnitId("unit-a"), [Capability("work")]);
    let capacity = CapacitySnapshot::new(0, Some(4), 0.0);

    VsmBuilder::new()
        .runtime_id(RuntimeId::from_string(runtime_id))
        .work_model(AcceptAllWorkModel::new([Capability("work")]))
        .operational_unit_factory(StaticOperationalUnitFactory::new(
            descriptor,
            capacity,
            DomainOutcome,
        ))
}

struct PlanningVarietyPolicy;

#[async_trait]
impl VarietyEngineeringPolicy<DomainSystem> for PlanningVarietyPolicy {
    async fn plan_interventions(
        &self,
        _context: &RoleContext<DomainSystem>,
        observation: &VarietyObservation<DomainSystem>,
    ) -> Result<Vec<VarietyIntervention<DomainSystem>>, FrameworkError> {
        if observation.estimate.ratio < 1.0 {
            Ok(vec![VarietyIntervention::new(
                VarietyInterventionKind::Amplification,
                "output variety trails input variety",
            )
            .target_unit(UnitId("unit-a"))])
        } else {
            Ok(Vec::new())
        }
    }
}

struct ClassifyingAlgedonicPolicy;

#[async_trait]
impl AlgedonicLifecyclePolicy<DomainSystem> for ClassifyingAlgedonicPolicy {
    async fn classify_signal(
        &self,
        _context: &RoleContext<DomainSystem>,
        mut signal: AlgedonicSignalRecord<DomainSystem>,
    ) -> Result<AlgedonicSignalRecord<DomainSystem>, FrameworkError> {
        if signal.status == AlgedonicLifecycleStatus::Proposed {
            signal.status = AlgedonicLifecycleStatus::Classified;
        }
        Ok(signal)
    }
}

struct TrendTemporalPolicy;

#[async_trait]
impl TemporalAnalysisPolicy<DomainSystem> for TrendTemporalPolicy {
    async fn analyze_temporal(
        &self,
        _context: &RoleContext<DomainSystem>,
        _samples: &[TemporalSample],
        aggregates: &[vsm_rs::protocol::temporal::TemporalAggregate],
    ) -> Result<TemporalAnalysis, FrameworkError> {
        let mut analysis = TemporalAnalysis::empty(aggregates.to_vec());
        analysis.patterns.push(TemporalPattern::new(
            TemporalPatternKind::Trend,
            "minute",
            "fixture trend",
        ));
        let mut forecast = TemporalForecast::new("minute");
        forecast.points.push(TemporalForecastPoint {
            offset: Duration::from_secs(60),
            value: 5.0,
            confidence: 0.8,
        });
        analysis.forecasts.push(forecast);
        analysis
            .causal_hypotheses
            .push(CausalHypothesis::new("input", "output", 0.7));
        Ok(analysis)
    }
}

struct CapturingAlertSink {
    alerts: Arc<Mutex<Vec<AlertRecord>>>,
}

#[async_trait]
impl AlertSink for CapturingAlertSink {
    async fn publish_alert(&self, alert: AlertRecord) -> Result<(), FrameworkError> {
        self.alerts
            .lock()
            .map_err(|_| FrameworkError::Runtime {
                reason: "alert capture mutex poisoned".to_string(),
            })?
            .push(alert);
        Ok(())
    }
}
