# ADR-0009: Variety, Algedonic, and Temporal Boundary

- Status: Accepted
- Date: 2026-06-19
- Deciders: User, Codex

## Context

Milestone 10 rebuilds variety, algedonic, and temporal capabilities after the
typed System 1 through System 5 runtime migrations. The current crate still has
three partly separate surfaces:

- System 1 has `VarietyModel` and `AlgedonicPolicy` hooks, but generic variety
  logic still exists in legacy JSON-oriented paths and pure helper modules.
- The legacy broker can carry algedonic `VsmMessage` values, but those messages
  are not automatically converted into typed System 5 crisis records.
- `channels::algedonic` has a separate typed actor that classifies signals,
  computes descriptive routes, stores active signals, and writes alert history
  through a process-global static.
- `channels::temporal_variety` and `channels::temporal` keep useful generic
  windowing, aggregation, pattern, forecast, and causality mechanics, but the
  interpretation algorithms are currently starter implementations rather than
  application-owned strategy roles.

The public API decision is how much of this milestone should become a typed
runtime subsystem boundary now, and whether variety, algedonic, and temporal
domains need new application associated types or can continue the System 2
through System 5 pattern of framework-owned records over the existing
`ViableSystem` family.

## Options

### Option A: Minimal Framework-Owned Lifecycle Records

Define framework-owned variety, algedonic, and temporal protocol records over
the existing `ViableSystem` associated types. Add public role traits for the
remaining application-owned behavior, such as `VarietyEngineeringPolicy`,
`AlgedonicClassifier` or an expanded algedonic policy boundary, and temporal
analysis/strategy roles.

Keep the runtime-owned parts generic:

- variety windows, estimates, ratios, uncertainty, interventions, and outcome
  tracking;
- algedonic signal lifecycle, priority delivery, acknowledgement timers,
  deduplication, correlation, escalation, and alert-sink dispatch;
- temporal windows, aggregation, freshness, schema/version metadata, and
  strategy outputs.

Bridge legacy broker algedonic messages and the advanced algedonic actor into
typed System 5 crisis handling. Move JSON key-counting, thresholds, filters,
route descriptions, attenuation/amplification heuristics, and temporal pattern
algorithms under defaults or examples.

Do not add new required `ViableSystem` associated types in this milestone.

### Option B: Extension Type Family

Add a `VarietyAlgedonicTemporalTypes` or separate extension traits with
application-owned associated types for variety measures, interventions,
algedonic signals, alerts, temporal observations, detected patterns,
forecasts, and causal hypotheses.

This gives applications maximum domain fidelity immediately, but expands the
public type family before recursion, persistence, durable event replay, and
external adapter boundaries have proven which payloads need to be first-class
application types.

### Option C: Algedonic Bridge First

Implement the typed algedonic lifecycle and bridge to System 5 now. Move
variety engineering and temporal analysis algorithms to defaults/examples but
defer public replacement roles and runtime integration to later milestones.

This resolves the highest-priority delivery correctness gap first, but leaves
the crate with incomplete typed variety and temporal boundaries after the
milestone.

## Recommendation

Option A.

This follows the established migration pattern from Systems 2, 3, 4, and 5:
keep `ViableSystem` minimal, represent VSM mechanics as framework-owned typed
records, and place domain interpretation in object-safe roles. It also lets the
milestone close the known algedonic bridge and process-global alert-history
gaps without forcing durable persistence or recursion authority semantics ahead
of their owning milestones.

## Decision

Use Option A: minimal framework-owned variety, algedonic, and temporal
lifecycle records with public role traits over the existing `ViableSystem`
family.

Do not add new required associated types to `ViableSystem` for this milestone.
Variety estimates, interventions, algedonic signal lifecycle, acknowledgements,
escalations, alert records, temporal windows, aggregates, patterns, forecasts,
and causal hypotheses are represented as framework-owned typed records.
Applications provide interpretation and strategy through role implementations.

## Consequences

If Option A is accepted:

- add typed variety, algedonic, and temporal protocol records without adding
  new required `ViableSystem` associated types;
- add public role traits for variety engineering, algedonic classification or
  lifecycle policy, and temporal analysis strategies;
- keep generic windows, ratios, uncertainty, correlation, deduplication,
  acknowledgement timers, escalation records, and alert-sink integration in the
  runtime;
- bridge legacy broker algedonic input into the typed System 5 crisis path;
- make alert history actor- or store-owned instead of process-global;
- move current JSON, threshold, route-description, attenuation/amplification,
  pattern, forecast, and causality heuristics to defaults/examples;
- defer durable replay, external store adapters, and detailed recursion
  authority/translation semantics to their owning milestones.

## Links

- `IMPLEMENTATION.md`
- `CODEX.md`
