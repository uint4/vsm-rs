# ADR-0007: System 4 Intelligence Boundary

- Status: Accepted
- Date: 2026-06-18
- Deciders: User, Codex

## Context

Milestone 8 converts System 4 from four legacy JSON `ServiceActor` services
into a typed, supervised environmental-intelligence pipeline. The roadmap calls
for dynamic environment source registration, supervised polling or streaming,
normalized observations with timestamp/provenance/confidence/freshness,
application-provided interpretation and intelligence modeling, forecasting or
scenario planning, calibration against outcomes, typed adaptation proposals for
System 5, and feasibility requests toward System 3.

The public API decision is whether System 4 should follow the System 2/System 3
pattern of framework-owned records over the existing `ViableSystem` type family,
or whether this milestone should expand the application type family with
domain-owned environmental and scenario payload types.

## Options

### Option A: Minimal Framework-Owned Intelligence Pipeline

Define framework-owned System 4 protocol records and public roles such as
`EnvironmentalSource`, `EnvironmentalSourceFactory`, `SignalInterpreter`,
`IntelligenceModel`, and `Forecaster` or `ScenarioPlanner` over the existing
`ViableSystem` associated types.

Use framework-owned identifiers, observations, interpreted signals,
intelligence assessments, forecasts, scenario records, calibration records, and
adaptation proposals. These records carry metadata, provenance, confidence,
freshness, uncertainty, source identity, and links to System 3 feasibility and
System 5 proposal routing.

Do not add new required `ViableSystem` associated types in this milestone.
Move current scanner, analytics, and forecasting algorithms under
`system4::defaults` as opt-in examples.

### Option B: System 4 Extension Type Family

Define a `System4Types` or `System4Roles` extension with application-owned
associated types for raw observations, environmental signals, intelligence
assessments, forecasts, scenarios, calibration outcomes, and adaptation
proposals.

This gives applications richer domain payloads immediately, but adds more type
surface before Systems 5, recursion, persistence, and adapter boundaries are
settled.

### Option C: Observation and Interpretation First

Convert source registration, observation collection, freshness tracking, and
signal interpretation now. Defer forecasting, scenario planning, calibration,
System 3 feasibility requests, and typed System 5 proposals to later review
gates.

This reduces first-slice risk but does not satisfy the roadmap exit criterion
that a typed scenario reaches System 5 with provenance and uncertainty.

## Decision

Use Option A: minimal framework-owned System 4 protocol records and public
roles over the existing `ViableSystem` type family.

Do not add new required associated types to `ViableSystem` for this milestone.
Environmental source identity, observations, signals, intelligence
assessments, forecasts, scenario records, calibration records, and adaptation
proposals are represented as framework-owned typed records with metadata,
provenance, confidence, freshness, uncertainty, source identity, and routing
fields.

## Recommendation

Option A.

This matches the established System 2 and System 3 migration pattern: keep the
required application type family small, put domain behavior in object-safe role
traits, and let later subsystem extensions introduce application-owned payloads
only when concrete domains prove the need. It also keeps Milestone 8 broad
enough to exercise the full System 4 pipeline without forcing app-specific
environment semantics into the crate.

## Consequences

If Option A is accepted:

- add typed System 4 protocol records without adding required `ViableSystem`
  associated types;
- add public System 4 role traits for sources, interpretation, intelligence
  modeling, and forecasting or scenario planning;
- keep source instances restartable through factories;
- make source freshness, provenance, confidence, uncertainty, calibration, and
  proposal routing framework-owned runtime concerns;
- move current scanner, analytics, and forecasting behavior to explicit
  defaults/examples;
- defer domain-specific environmental payload families to later extension
  traits or adapters.

## Links

- `IMPLEMENTATION.md`
- `CODEX.md`
