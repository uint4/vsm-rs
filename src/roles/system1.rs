//! First-wave System 1 role contracts.

use std::collections::BTreeMap;
use std::marker::PhantomData;

use ractor::async_trait;

use crate::error::{FrameworkError, WorkError};
use crate::protocol::system1::{
    Acknowledgement, AuditEvidence, AuditRequest, CapacitySnapshot, CoordinationView,
    PerformanceObservation, UnitCommand, UnitDescriptor, WorkDisposition, WorkRequest,
    WorkResponse, WorkResult,
};
use crate::protocol::system2::{CoordinationAcknowledgement, CoordinationIntervention};
use crate::protocol::SnapshotRecord;

use super::{RoleContext, UnitRoleContext, ViableSystem};

/// Boxed operational unit object owned by a future unit actor adapter.
pub type BoxOperationalUnit<V> = Box<dyn OperationalUnit<V>>;

/// Shared work model object.
pub type SharedWorkModel<V> = std::sync::Arc<dyn WorkModel<V>>;

/// Shared operational unit factory object.
pub type SharedOperationalUnitFactory<V> = std::sync::Arc<dyn OperationalUnitFactory<V>>;

/// Shared unit selection policy object.
pub type SharedUnitSelectionPolicy<V> = std::sync::Arc<dyn UnitSelectionPolicy<V>>;

/// Shared performance model object.
pub type SharedPerformanceModel<V> = std::sync::Arc<dyn PerformanceModel<V>>;

/// Shared variety model object.
pub type SharedVarietyModel<V> = std::sync::Arc<dyn VarietyModel<V>>;

/// Shared algedonic policy object.
pub type SharedAlgedonicPolicy<V> = std::sync::Arc<dyn AlgedonicPolicy<V>>;

/// Domain measurement derived by an application work model.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkMeasurement {
    pub name: String,
    pub value: f64,
    pub unit: Option<String>,
    pub dimensions: BTreeMap<String, String>,
}

impl WorkMeasurement {
    /// Creates a scalar work measurement.
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
            unit: None,
            dimensions: BTreeMap::new(),
        }
    }

    /// Adds a human-readable unit label.
    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }
}

/// Unit candidate considered by a selection policy.
#[derive(Debug, Clone)]
pub struct UnitCandidate<V>
where
    V: ViableSystem,
{
    pub descriptor: UnitDescriptor<V>,
    pub capacity: CapacitySnapshot,
}

impl<V> UnitCandidate<V>
where
    V: ViableSystem,
{
    /// Creates a candidate from a descriptor and capacity snapshot.
    pub fn new(descriptor: UnitDescriptor<V>, capacity: CapacitySnapshot) -> Self {
        Self {
            descriptor,
            capacity,
        }
    }

    /// Returns true when the descriptor advertises every required capability.
    pub fn advertises_all(&self, required: &[V::Capability]) -> bool {
        required.iter().all(|required_capability| {
            self.descriptor
                .capabilities
                .iter()
                .any(|candidate| &candidate.capability == required_capability)
        })
    }
}

/// Generic performance assessment produced from operational observations.
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceAssessment {
    pub actuality: f64,
    pub capability: f64,
    pub potentiality: f64,
    pub quality: f64,
    pub risk: f64,
    pub notes: BTreeMap<String, String>,
}

impl PerformanceAssessment {
    /// Creates an assessment with values clamped to the inclusive 0-1 range.
    pub fn new(
        actuality: f64,
        capability: f64,
        potentiality: f64,
        quality: f64,
        risk: f64,
    ) -> Self {
        Self {
            actuality: actuality.clamp(0.0, 1.0),
            capability: capability.clamp(0.0, 1.0),
            potentiality: potentiality.clamp(0.0, 1.0),
            quality: quality.clamp(0.0, 1.0),
            risk: risk.clamp(0.0, 1.0),
            notes: BTreeMap::new(),
        }
    }

    /// Returns a no-signal assessment used by no-op defaults.
    pub fn no_signal() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0, 0.0)
    }
}

/// Application-relevant variety assessment.
#[derive(Debug, Clone, PartialEq)]
pub struct VarietyAssessment {
    pub input: f64,
    pub output: f64,
    pub ratio: f64,
    pub notes: BTreeMap<String, String>,
}

impl VarietyAssessment {
    /// Creates a variety assessment with a derived output/input ratio.
    pub fn new(input: f64, output: f64) -> Self {
        let ratio = if input > 0.0 { output / input } else { 0.0 };
        Self {
            input: input.max(0.0),
            output: output.max(0.0),
            ratio,
            notes: BTreeMap::new(),
        }
    }

    /// Returns an assessment with no semantic signal.
    pub fn no_signal() -> Self {
        Self::new(0.0, 0.0)
    }
}

/// Algedonic signal category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgedonicKind {
    Pain,
    Pleasure,
}

/// Algedonic signal severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgedonicSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Candidate algedonic signal produced by an application policy.
#[derive(Debug, Clone)]
pub struct AlgedonicSignal<V>
where
    V: ViableSystem,
{
    pub unit_id: V::UnitId,
    pub kind: AlgedonicKind,
    pub severity: AlgedonicSeverity,
    pub reason: String,
    pub measurements: Vec<WorkMeasurement>,
}

/// Application-owned operational unit behavior.
///
/// A future actor adapter will own one implementation and invoke mutable methods
/// serially. The trait itself does not expose actor messages, actor references,
/// global names, or supervisor operations.
#[async_trait]
pub trait OperationalUnit<V>: Send
where
    V: ViableSystem,
{
    async fn descriptor(
        &mut self,
        context: &UnitRoleContext<V>,
    ) -> Result<UnitDescriptor<V>, FrameworkError>;

    async fn capacity(
        &mut self,
        context: &UnitRoleContext<V>,
    ) -> Result<CapacitySnapshot, FrameworkError>;

    async fn handle_work(
        &mut self,
        context: &UnitRoleContext<V>,
        request: WorkRequest<V>,
    ) -> WorkResult<V>;

    async fn handle_command(
        &mut self,
        context: &UnitRoleContext<V>,
        command: UnitCommand<V>,
    ) -> Result<Acknowledgement, FrameworkError>;

    async fn coordination_view(
        &mut self,
        context: &UnitRoleContext<V>,
    ) -> Result<CoordinationView<V>, FrameworkError>;

    async fn handle_coordination_intervention(
        &mut self,
        context: &UnitRoleContext<V>,
        intervention: CoordinationIntervention<V>,
    ) -> Result<CoordinationAcknowledgement<V>, FrameworkError> {
        Ok(CoordinationAcknowledgement::accepted(
            &intervention,
            context.unit_id().clone(),
        ))
    }

    async fn audit_evidence(
        &mut self,
        context: &UnitRoleContext<V>,
        request: AuditRequest<V>,
    ) -> Result<AuditEvidence<V>, FrameworkError>;

    async fn snapshot(
        &mut self,
        context: &UnitRoleContext<V>,
    ) -> Result<V::UnitSnapshot, FrameworkError> {
        let _ = context;
        Err(FrameworkError::InvalidProtocol {
            reason: "unit snapshots are not supported by this role".to_string(),
        })
    }

    async fn restore(
        &mut self,
        context: &UnitRoleContext<V>,
        snapshot: SnapshotRecord<V::UnitSnapshot>,
    ) -> Result<(), FrameworkError> {
        let _ = (context, snapshot);
        Err(FrameworkError::InvalidProtocol {
            reason: "unit snapshot restore is not supported by this role".to_string(),
        })
    }
}

/// Restartable factory for operational unit role instances.
#[async_trait]
pub trait OperationalUnitFactory<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn create_unit(
        &self,
        context: &RoleContext<V>,
        descriptor: &UnitDescriptor<V>,
    ) -> Result<BoxOperationalUnit<V>, FrameworkError>;
}

/// Application work interpretation model.
#[async_trait]
pub trait WorkModel<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn validate_work(
        &self,
        context: &RoleContext<V>,
        request: WorkRequest<V>,
    ) -> Result<(), WorkError<V::AppError>>;

    async fn required_capabilities(
        &self,
        context: &RoleContext<V>,
        request: WorkRequest<V>,
    ) -> Result<Vec<V::Capability>, WorkError<V::AppError>>;

    async fn classify_outcome(
        &self,
        context: &RoleContext<V>,
        request: WorkRequest<V>,
        outcome: V::Outcome,
    ) -> Result<WorkDisposition, WorkError<V::AppError>> {
        let _ = (context, request, outcome);
        Ok(WorkDisposition::Completed)
    }

    async fn classify_error(
        &self,
        context: &RoleContext<V>,
        request: WorkRequest<V>,
        error: &WorkError<V::AppError>,
    ) -> Result<WorkDisposition, FrameworkError> {
        let _ = (context, request);
        Ok(WorkDisposition::from(error))
    }

    async fn measurements(
        &self,
        context: &RoleContext<V>,
        request: WorkRequest<V>,
        response: WorkResponse<V>,
    ) -> Result<Vec<WorkMeasurement>, WorkError<V::AppError>>;
}

/// Policy that selects one eligible unit for work.
#[async_trait]
pub trait UnitSelectionPolicy<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn select_unit(
        &self,
        context: &RoleContext<V>,
        request: WorkRequest<V>,
        candidates: &[UnitCandidate<V>],
    ) -> Result<Option<V::UnitId>, FrameworkError>;
}

/// Model that converts operational observations into performance views.
#[async_trait]
pub trait PerformanceModel<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn assess_performance(
        &self,
        context: &RoleContext<V>,
        observation: &PerformanceObservation<V>,
        measurements: &[WorkMeasurement],
    ) -> Result<PerformanceAssessment, FrameworkError>;
}

/// Model that measures application-relevant operational variety.
#[async_trait]
pub trait VarietyModel<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn assess_variety(
        &self,
        context: &RoleContext<V>,
        request: WorkRequest<V>,
        response: Option<WorkResponse<V>>,
    ) -> Result<VarietyAssessment, FrameworkError>;
}

/// Policy that decides whether performance should emit an algedonic signal.
#[async_trait]
pub trait AlgedonicPolicy<V>: Send + Sync
where
    V: ViableSystem,
{
    async fn classify_algedonic(
        &self,
        context: &RoleContext<V>,
        observation: &PerformanceObservation<V>,
        assessment: &PerformanceAssessment,
    ) -> Result<Option<AlgedonicSignal<V>>, FrameworkError>;
}

/// Static catalog of first-wave System 1 roles for one application type family.
pub trait System1Roles<V>: Send + Sync + 'static
where
    V: ViableSystem,
{
    type OperationalUnit: OperationalUnit<V>;
    type OperationalUnitFactory: OperationalUnitFactory<V>;
    type WorkModel: WorkModel<V>;
    type UnitSelectionPolicy: UnitSelectionPolicy<V>;
    type PerformanceModel: PerformanceModel<V>;
    type VarietyModel: VarietyModel<V>;
    type AlgedonicPolicy: AlgedonicPolicy<V>;
}

/// Opt-in default and no-op System 1 policies.
pub mod defaults {
    use super::*;

    /// Selects the accepting candidate with the lowest reported load.
    #[derive(Debug, Default)]
    pub struct LowestLoadSelectionPolicy;

    #[async_trait]
    impl<V> UnitSelectionPolicy<V> for LowestLoadSelectionPolicy
    where
        V: ViableSystem,
    {
        async fn select_unit(
            &self,
            context: &RoleContext<V>,
            request: WorkRequest<V>,
            candidates: &[UnitCandidate<V>],
        ) -> Result<Option<V::UnitId>, FrameworkError> {
            let _ = (context, request);
            Ok(candidates
                .iter()
                .filter(|candidate| candidate.capacity.accepting_work)
                .min_by(|left, right| left.capacity.load.total_cmp(&right.capacity.load))
                .map(|candidate| candidate.descriptor.unit_id.clone()))
        }
    }

    /// Performance model that emits no semantic performance signal.
    #[derive(Debug, Default)]
    pub struct NoopPerformanceModel;

    #[async_trait]
    impl<V> PerformanceModel<V> for NoopPerformanceModel
    where
        V: ViableSystem,
    {
        async fn assess_performance(
            &self,
            context: &RoleContext<V>,
            observation: &PerformanceObservation<V>,
            measurements: &[WorkMeasurement],
        ) -> Result<PerformanceAssessment, FrameworkError> {
            let _ = (context, observation, measurements);
            Ok(PerformanceAssessment::no_signal())
        }
    }

    /// Variety model that emits no semantic variety signal.
    #[derive(Debug, Default)]
    pub struct NoopVarietyModel;

    #[async_trait]
    impl<V> VarietyModel<V> for NoopVarietyModel
    where
        V: ViableSystem,
    {
        async fn assess_variety(
            &self,
            context: &RoleContext<V>,
            request: WorkRequest<V>,
            response: Option<WorkResponse<V>>,
        ) -> Result<VarietyAssessment, FrameworkError> {
            let _ = (context, request, response);
            Ok(VarietyAssessment::no_signal())
        }
    }

    /// Algedonic policy that never emits a pain or pleasure signal.
    #[derive(Debug, Default)]
    pub struct NoopAlgedonicPolicy;

    #[async_trait]
    impl<V> AlgedonicPolicy<V> for NoopAlgedonicPolicy
    where
        V: ViableSystem,
    {
        async fn classify_algedonic(
            &self,
            context: &RoleContext<V>,
            observation: &PerformanceObservation<V>,
            assessment: &PerformanceAssessment,
        ) -> Result<Option<AlgedonicSignal<V>>, FrameworkError> {
            let _ = (context, observation, assessment);
            Ok(None)
        }
    }
}

/// Test helpers for downstream-style role contract tests.
pub mod testing {
    use super::*;

    /// Work model that accepts every request and returns fixed capabilities.
    pub struct AcceptAllWorkModel<V>
    where
        V: ViableSystem,
    {
        required_capabilities: Vec<V::Capability>,
        measurements: Vec<WorkMeasurement>,
        _system: PhantomData<V>,
    }

    impl<V> AcceptAllWorkModel<V>
    where
        V: ViableSystem,
    {
        /// Creates a work model with fixed required capabilities.
        pub fn new(required_capabilities: impl IntoIterator<Item = V::Capability>) -> Self {
            Self {
                required_capabilities: required_capabilities.into_iter().collect(),
                measurements: Vec::new(),
                _system: PhantomData,
            }
        }

        /// Adds fixed measurements returned after work execution.
        pub fn with_measurements(mut self, measurements: Vec<WorkMeasurement>) -> Self {
            self.measurements = measurements;
            self
        }
    }

    #[async_trait]
    impl<V> WorkModel<V> for AcceptAllWorkModel<V>
    where
        V: ViableSystem,
    {
        async fn validate_work(
            &self,
            context: &RoleContext<V>,
            request: WorkRequest<V>,
        ) -> Result<(), WorkError<V::AppError>> {
            let _ = (context, request);
            Ok(())
        }

        async fn required_capabilities(
            &self,
            context: &RoleContext<V>,
            request: WorkRequest<V>,
        ) -> Result<Vec<V::Capability>, WorkError<V::AppError>> {
            let _ = (context, request);
            Ok(self.required_capabilities.clone())
        }

        async fn measurements(
            &self,
            context: &RoleContext<V>,
            request: WorkRequest<V>,
            response: WorkResponse<V>,
        ) -> Result<Vec<WorkMeasurement>, WorkError<V::AppError>> {
            let _ = (context, request, response);
            Ok(self.measurements.clone())
        }
    }

    /// Operational unit that returns a fixed outcome and fixed descriptor.
    pub struct StaticOperationalUnit<V>
    where
        V: ViableSystem,
    {
        descriptor: UnitDescriptor<V>,
        capacity: CapacitySnapshot,
        outcome: V::Outcome,
    }

    impl<V> StaticOperationalUnit<V>
    where
        V: ViableSystem,
    {
        /// Creates a static operational unit.
        pub fn new(
            descriptor: UnitDescriptor<V>,
            capacity: CapacitySnapshot,
            outcome: V::Outcome,
        ) -> Self {
            Self {
                descriptor,
                capacity,
                outcome,
            }
        }
    }

    #[async_trait]
    impl<V> OperationalUnit<V> for StaticOperationalUnit<V>
    where
        V: ViableSystem,
    {
        async fn descriptor(
            &mut self,
            context: &UnitRoleContext<V>,
        ) -> Result<UnitDescriptor<V>, FrameworkError> {
            let _ = context;
            Ok(self.descriptor.clone())
        }

        async fn capacity(
            &mut self,
            context: &UnitRoleContext<V>,
        ) -> Result<CapacitySnapshot, FrameworkError> {
            let _ = context;
            Ok(self.capacity.clone())
        }

        async fn handle_work(
            &mut self,
            context: &UnitRoleContext<V>,
            request: WorkRequest<V>,
        ) -> WorkResult<V> {
            let _ = (context, request);
            Ok(self.outcome.clone())
        }

        async fn handle_command(
            &mut self,
            context: &UnitRoleContext<V>,
            command: UnitCommand<V>,
        ) -> Result<Acknowledgement, FrameworkError> {
            let _ = context;
            Ok(Acknowledgement::accepted(command.metadata))
        }

        async fn coordination_view(
            &mut self,
            context: &UnitRoleContext<V>,
        ) -> Result<CoordinationView<V>, FrameworkError> {
            Ok(CoordinationView {
                metadata: context.metadata().clone(),
                unit_id: self.descriptor.unit_id.clone(),
                capabilities: self.descriptor.capabilities.clone(),
                capacity: self.capacity.clone(),
                snapshot_version: None,
            })
        }

        async fn audit_evidence(
            &mut self,
            context: &UnitRoleContext<V>,
            request: AuditRequest<V>,
        ) -> Result<AuditEvidence<V>, FrameworkError> {
            let _ = request;
            Ok(AuditEvidence {
                metadata: context.metadata().clone(),
                unit_id: self.descriptor.unit_id.clone(),
                capabilities: self.descriptor.capabilities.clone(),
                capacity: self.capacity.clone(),
                snapshot_version: None,
                snapshot: None,
            })
        }
    }

    /// Factory that creates fresh [`StaticOperationalUnit`] instances.
    pub struct StaticOperationalUnitFactory<V>
    where
        V: ViableSystem,
    {
        descriptor: UnitDescriptor<V>,
        capacity: CapacitySnapshot,
        outcome: std::sync::Mutex<V::Outcome>,
    }

    impl<V> StaticOperationalUnitFactory<V>
    where
        V: ViableSystem,
    {
        /// Creates a factory for static operational units.
        pub fn new(
            descriptor: UnitDescriptor<V>,
            capacity: CapacitySnapshot,
            outcome: V::Outcome,
        ) -> Self {
            Self {
                descriptor,
                capacity,
                outcome: std::sync::Mutex::new(outcome),
            }
        }
    }

    #[async_trait]
    impl<V> OperationalUnitFactory<V> for StaticOperationalUnitFactory<V>
    where
        V: ViableSystem,
    {
        async fn create_unit(
            &self,
            context: &RoleContext<V>,
            descriptor: &UnitDescriptor<V>,
        ) -> Result<BoxOperationalUnit<V>, FrameworkError> {
            let _ = (context, descriptor);
            let outcome = self
                .outcome
                .lock()
                .map_err(|_| FrameworkError::Runtime {
                    reason: "static unit factory outcome mutex poisoned".to_string(),
                })?
                .clone();
            Ok(Box::new(StaticOperationalUnit::new(
                self.descriptor.clone(),
                self.capacity.clone(),
                outcome,
            )))
        }
    }
}
