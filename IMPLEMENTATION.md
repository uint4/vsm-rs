# Implementation Plan: Trait-Driven VSM Runtime

## Purpose

This plan takes the crate from its current state—a direct actor-oriented port with demo behavior and JSON service dispatch—to a reusable VSM runtime in which:

- the crate owns VSM structure, supervision, protocols, lifecycle, routing, escalation, recursion, and observability;
- applications implement domain behavior through role traits;
- application authors do not need to implement `ractor::Actor`, manipulate `ActorRef`, use global actor names, or exchange untyped JSON internally;
- JSON remains available only as an optional integration adapter;
- default algorithms are clearly labeled as defaults or examples rather than VSM invariants.

The migration should be incremental. System 1 becomes the first complete vertical slice, then the same pattern is extended to Systems 2–5, algedonic signaling, variety engineering, and recursion.

---

## 1. Current-state assessment

### 1.1 Runtime and supervision

The crate currently has a root `ractor-supervisor` tree with static supervisors for channels and Systems 1–5, plus dynamic supervision for System 1 units. This is a useful runtime foundation and should be retained.

Current constraints:

- actor names are process-global;
- the public API frequently discovers actors through those names;
- only one default VSM runtime can safely exist in a process;
- startup returns before there is an explicit, application-facing readiness contract;
- restart recovery is incomplete for subscriptions, System 1's unit directory, and service state.

### 1.2 Public API

The crate exposes actor-oriented convenience functions and several runtime internals. The public surface currently includes concepts that should eventually become internal:

- global names;
- `ActorRef` lookup;
- generic service calls by operation string;
- `serde_json::Value` for core requests and results;
- actor message and actor implementation types.

The desired public surface is a typed builder, a runtime handle, subsystem handles, role contracts, protocol types, configuration, and adapter traits.

### 1.3 System 1

System 1 is the most mature part of the port. It already has dedicated typed actors, typed actor messages, dynamic unit supervision, unit registration, work selection, basic metrics, variety history, resource-request signaling, command handling, coordination handling, and audit responses.

It nevertheless mixes framework and application policy:

- `Unit` manufactures a canned JSON `"processed"` response;
- `Transaction` defines work with strings and JSON;
- capability matching is hard-coded string containment;
- unit selection is hard-coded lowest load;
- variety is inferred from JSON shape;
- raw JSON state is gathered, merged, and redistributed;
- load migration is represented by mutating a number;
- audit evidence is manufactured by the runtime;
- resource requests use JSON payloads.

### 1.4 Systems 2–5

Systems 2–5 primarily use `actor_support::ServiceActor`:

- operations are strings;
- request and reply payloads are `serde_json::Value`;
- unknown operations return JSON rather than typed errors;
- service state is one mutable JSON document plus JSON history;
- many VSM-semantic decisions are implemented directly in the crate.

Examples of embedded policy include scheduling assumptions, fixed balancing thresholds, simple resource allocation, audit scoring, trend analysis, keyword-based identity/value alignment, fixed crisis options, and generic policy directives.

### 1.5 Channels

The channel broker provides a useful actor-owned subscription registry and bounded history. Its current behavior still needs redesign:

- the message envelope always contains JSON;
- subscriber identity is a free-form string;
- a failed targeted delivery falls back to broadcast;
- subscriptions and history disappear when the broker restarts;
- channel subscriptions are not automatically reconciled;
- inter-system protocols and observer/event distribution are conflated.

### 1.6 Recursion, persistence, and adapters

`shared::recursion` currently models hierarchy as an in-memory data structure, but it does not instantiate nested VSM runtimes or route across recursion boundaries.

Persistence is not abstracted. State is mostly restart-volatile. Alert history also uses process-global mutable storage, which conflicts with the intended actor-owned or adapter-owned state model.

---

## 2. Target architecture

The target architecture has five layers.

### 2.1 Public role contracts

The crate defines contracts such as:

- operational work execution;
- work interpretation;
- unit selection;
- coordination;
- performance interpretation;
- resource governance;
- audit;
- environmental observation;
- intelligence and forecasting;
- identity, values, policy, and decisions;
- variety measurement and intervention;
- algedonic classification and response;
- recursion-boundary transduction;
- persistence, telemetry, and external alerting.

Applications implement these contracts without importing `ractor` types.

### 2.2 Typed VSM protocols

The crate owns typed protocol families for:

- work submission and outcome;
- unit capability and capacity;
- coordination views and interventions;
- performance reports;
- resource bargains and allocations;
- commands and acknowledgements;
- audit requests, evidence, findings, and remediation;
- environmental observations, intelligence, scenarios, and proposals;
- policy decisions, versions, directives, and acknowledgements;
- variety observations and interventions;
- algedonic alerts and escalation;
- recursion addressing and parent/child translation.

These protocols carry framework metadata such as correlation, deadline, priority, recursion path, source, destination, and protocol version.

### 2.3 Internal actor adapters

The crate wraps role implementations in internal actors. Actors own:

- serialization of access to mutable role state;
- supervision and restart behavior;
- mailbox processing;
- runtime metrics;
- timeout and cancellation handling;
- protocol routing;
- snapshots and recovery;
- failure classification and escalation.

Applications normally never implement `Actor`.

### 2.4 Runtime facade

A typed builder assembles a runtime from role implementations and configuration. A runtime handle exposes safe subsystem operations. Actor references and names remain private.

The builder validates that required roles and adapters are present before startup.

### 2.5 Optional adapters and defaults

The crate may ship:

- in-memory stores;
- tracing/metrics adapters;
- JSON/Serde interoperability;
- simple unit selectors;
- simple coordination and balancing policies;
- simple statistical forecasting;
- simple variety calculations;
- test fakes.

These must live under explicit `defaults`, `adapters`, `testing`, or `examples` namespaces. They must not masquerade as mandatory VSM semantics.

---

## 3. Design decisions to settle before refactoring

Record these as Architecture Decision Records before changing public APIs.

### ADR-001: Domain type family

Adopt one application type family for all typed protocols. Conceptually, it defines the application's work, outcome, error, capability, command, resource, coordination view, audit evidence, environmental observation, policy, and related types.

Why this is needed:

- actor messages require concrete compile-time types;
- all actors in one runtime need to agree on the same protocol family;
- a single type family keeps subsystem contracts interoperable;
- it permits a `VsmRuntime<ApplicationTypes>` and typed handles without exposing actors.

Guardrail: keep the required associated types minimal. Add optional protocol extensions through separate traits instead of growing one unmanageable type trait.

### ADR-002: Dynamic role dispatch

Use object-safe role traits held by internal actors so implementations can be selected and registered at runtime.

Public async role methods must be deliberately object-safe. Native `async fn` in a public trait is not dyn-compatible; select one of these approaches and use it consistently:

1. an `async-trait` macro with `Send` futures; or
2. explicit boxed `Send` futures in trait signatures.

Recommended direction: use an `async-trait`-style public contract initially for ergonomics, while documenting the allocation/dynamic-dispatch trade-off. The runtime is already actor- and I/O-oriented, so clarity is more valuable than avoiding one boxed future per role call.

### ADR-003: Mutable role state

A unit actor should own one boxed role implementation and invoke it mutably. This preserves actor serialization and avoids requiring application implementations to add their own mutexes.

For shared stateless policies, use immutable `Arc`-backed trait objects.

### ADR-004: Restartable role factories

Dynamic supervisors must be able to create a fresh role implementation after failure. Unit registration therefore cannot store only a one-shot boxed implementation. It must store a restartable factory plus descriptor and recovery information.

The factory creates a new implementation; the state store restores its durable snapshot when configured.

### ADR-005: Instance-scoped addressing

Replace global constant names as the primary addressing mechanism with an instance-scoped address containing:

- runtime instance ID;
- recursion path;
- subsystem role;
- optional unit ID.

The runtime handle should retain typed actor references privately. Registry names should be internal diagnostics and restart-discovery aids, not the public API.

### ADR-006: Protocol bus versus observer bus

Use typed, direct subsystem protocols for control flow. Keep pub/sub for observation and extension points.

Consequences:

- a command to one target never silently becomes a broadcast;
- request/reply and acknowledgements have explicit failure semantics;
- observers can subscribe to a typed `VsmEvent` stream;
- inter-system correctness does not depend on string subscriber IDs.

### ADR-007: JSON boundary

Remove `serde_json::Value` from core role contracts and canonical protocols. Provide an optional `json` or `serde` adapter feature for external APIs, legacy compatibility, and dynamic integrations.

### ADR-008: Persistence model

Begin with snapshot-oriented persistence behind a `StateStore` port. Do not require full event sourcing in the first trait-driven release.

The store should support:

- role snapshot load/save;
- runtime metadata;
- policy and identity versions;
- channel or event cursor state where needed;
- unit registration descriptors;
- recovery transactions or version checks.

Event journaling can be added later without changing role contracts if lifecycle events are already explicit.

### ADR-009: Error model

Separate:

- framework failures: unavailable actor, timeout, invalid protocol, rejected admission, persistence failure, shutdown;
- application failures: domain rejection, operational failure, policy failure;
- transient versus permanent classification;
- panic/crash versus returned error.

The public API should preserve application error information without forcing every role to use `VsmError`.

### ADR-010: Compatibility policy

Because the current manifest is still `0.1.0` and `publish = false`, prefer a clean breaking redesign before the first registry release.

Keep compatibility only as a temporary feature-gated `legacy-json` facade if it materially helps validate behavior or migrate examples. Do not allow legacy compatibility to dictate the final public API.

---

## 4. Target crate layout

The final names may change, but the ownership boundaries should resemble this layout:

```text
src/
├── lib.rs
├── builder.rs                 Public VSM builder
├── runtime.rs                 Public runtime and lifecycle handles
├── config.rs                  Public configuration values
├── error.rs                   Framework/public error model
├── protocol/
│   ├── mod.rs
│   ├── address.rs             Instance and recursion addressing
│   ├── envelope.rs            Correlation, deadline, priority, version
│   ├── system1.rs
│   ├── system2.rs
│   ├── system3.rs
│   ├── system4.rs
│   ├── system5.rs
│   ├── algedonic.rs
│   ├── variety.rs
│   └── events.rs
├── roles/
│   ├── mod.rs
│   ├── types.rs               Application type family
│   ├── system1.rs
│   ├── system2.rs
│   ├── system3.rs
│   ├── system4.rs
│   ├── system5.rs
│   ├── algedonic.rs
│   ├── variety.rs
│   ├── recursion.rs
│   └── ports.rs               Store, telemetry, clock, alert sinks
├── kernel/                    Private actor/runtime implementation
│   ├── mod.rs
│   ├── supervision.rs
│   ├── registry.rs
│   ├── lifecycle.rs
│   ├── event_bus.rs
│   ├── system1/
│   ├── system2/
│   ├── system3/
│   ├── system4/
│   ├── system5/
│   ├── algedonic/
│   └── recursion/
├── defaults/                  Optional, explicitly non-normative policies
├── adapters/
│   ├── memory.rs
│   ├── serde_json.rs
│   ├── tracing.rs
│   └── ...
├── testing/                   Fakes, harnesses, deterministic clock
└── legacy/                    Temporary feature-gated compatibility layer
```

The existing `system1` through `system5` modules can be migrated gradually. Do not perform the entire layout move before the behavior is covered by tests.

---

## 5. Milestone sequence

## Milestone 0: Establish a trustworthy baseline

### Objectives

Make the current crate reproducible and characterize its existing behavior before changing architecture.

### Work

1. Add CI jobs for:
   - formatting;
   - compilation on the minimum supported Rust version and stable Rust;
   - Clippy with warnings treated as failures for library code;
   - unit and integration tests;
   - doctests;
   - `cargo publish --dry-run` once publication metadata is enabled.
2. Resolve all current compile failures and version mismatches.
3. Add a dependency lock policy appropriate for a library and CI.
4. Decide the supported `ractor` and `ractor-supervisor` versions together; do not let Cargo resolve incompatible major/minor lines.
5. Replace startup sleeps in tests with a readiness mechanism.
6. Add deterministic shutdown assertions so tests do not leak globally named actors.
7. Add characterization tests for:
   - root startup and shutdown;
   - System 1 unit registration;
   - successful and unsuccessful work selection;
   - System 1 metrics and variety history;
   - targeted channel routing;
   - broadcast behavior;
   - channel validation;
   - Systems 2–5 service operations;
   - actor restart behavior;
   - current known recovery gaps.
8. Tag or preserve this baseline so later migrations can compare behavior.

### Exit criteria

- clean CI on all supported toolchains;
- no test relies on arbitrary sleeps;
- all current public operations have at least characterization coverage;
- current restart and message-delivery semantics are documented, including undesirable behavior.

---

## Milestone 1: Introduce typed protocol foundations

### Objectives

Create the new public types alongside the existing runtime without yet rewriting actors.

### Work

1. Add instance-scoped address types.
2. Add recursion path and subsystem role identifiers.
3. Add framework metadata:
   - correlation ID;
   - causation ID;
   - deadline;
   - priority;
   - protocol version;
   - trace context;
   - source and destination.
4. Define the minimal application type family.
5. Define typed protocol records for System 1 first:
   - work request;
   - work result;
   - capability description;
   - capacity/load snapshot;
   - unit descriptor;
   - command and acknowledgement;
   - performance observation;
   - resource-shortage request;
   - audit request and evidence;
   - coordination view.
6. Define framework error and application failure wrappers.
7. Add conversion adapters between the existing `Transaction`/`VsmMessage` JSON forms and the new typed System 1 protocols under a temporary legacy module.
8. Add serialization only behind an optional feature where possible.

### Exit criteria

- the new protocol modules contain no `ActorRef`, actor names, or `ractor` message types;
- core protocol payloads are not `serde_json::Value`;
- the legacy types can round-trip through adapters for the existing examples;
- public type documentation explains which fields are framework-owned and application-owned.

---

## Milestone 2: Define role contracts and role contexts

### Objectives

Establish the application-facing behavior boundary before changing actor implementations.

### Required first-wave roles

1. `OperationalUnit`
   - perform work;
   - report capability and capacity;
   - accept commands;
   - expose explicit coordination and audit views;
   - support snapshot/recovery when configured.
2. `OperationalUnitFactory`
   - create a fresh unit implementation for initial start and restart.
3. `WorkModel`
   - validate work;
   - determine required capabilities;
   - classify outcomes and errors;
   - derive domain measurements.
4. `UnitSelectionPolicy`
   - choose among eligible units.
5. `PerformanceModel`
   - convert operational measurements into actuality, capability, potentiality, quality, and risk views.
6. `VarietyModel`
   - measure application-relevant input/output variety.
7. `AlgedonicPolicy`
   - determine when a performance observation should become a pain or pleasure signal.

### Second-wave roles

- `CoordinationPolicy`;
- `StateReconciliationPolicy`;
- `WorkMigrationPolicy`;
- `ResourceGovernance`;
- `OperationalControlPolicy`;
- `Auditor`;
- `EnvironmentSource`;
- `SignalInterpreter`;
- `IntelligenceModel`;
- `Forecaster`;
- `IdentityModel` or identity provider;
- `ValuesEvaluator`;
- `DecisionPolicy`;
- `CrisisPolicy`;
- `VarietyEngineeringPolicy`;
- `RecursionTransducer`.

### Infrastructure ports

- `StateStore`;
- `TelemetrySink`;
- `AlertSink`;
- `Clock`;
- `IdGenerator`;
- optional retry/admission policy ports.

### Role context rules

A role context may expose:

- runtime and recursion identity;
- correlation and causation;
- deadline and cancellation;
- clock;
- event emission through a narrow domain/VSM event port;
- access to explicitly allowed stores or adapters.

It must not expose:

- actor references;
- global actor names;
- arbitrary channel publishing;
- mutable state belonging to another role;
- raw supervisor operations.

### Exit criteria

- a downstream test crate can implement every required first-wave role without importing `ractor` or `serde_json`;
- all role traits are dyn-compatible under the selected async strategy;
- trait bounds clearly require `Send`/`Sync` only where necessary;
- the crate includes mock and no-op implementations for tests;
- behavior versus configuration is documented so static values do not become unnecessary traits.

---

## Milestone 3: Add the builder, runtime handle, and instance scope

### Objectives

Create the target application-facing lifecycle while the old actors still exist underneath.

### Work

1. Introduce a typed builder that accepts:
   - application type family;
   - required role implementations/factories;
   - optional policies and defaults;
   - persistence and telemetry ports;
   - restart, timeout, retention, and capacity configuration;
   - runtime instance ID and recursion configuration.
2. Validate required roles before actor startup.
3. Return a `VsmRuntime` handle that owns lifecycle and typed subsystem handles.
4. Store actor references privately in the runtime handle or an internal runtime directory.
5. Generate internal names from instance ID and recursion path.
6. Add explicit readiness:
   - infrastructure ready;
   - subsystem actors ready;
   - role implementations initialized;
   - subscriptions reconciled;
   - persisted state restored.
7. Add graceful shutdown with a completion result.
8. Prove two runtime instances can coexist in one process.
9. Keep the old global `start()` facade only in `legacy-json` mode.

### Exit criteria

- application examples start the runtime through the builder;
- no application code looks up actors by global name;
- two independent VSM runtimes pass integration tests concurrently;
- readiness and shutdown use acknowledgements rather than sleeps.

---

## Milestone 4: Convert System 1 as the first complete vertical slice

### Objectives

Replace demo operational behavior with application-provided roles while retaining supervision and routing.

### Work: unit actor

1. Make the unit actor generic over the application type family.
2. Have it own one `OperationalUnit` implementation.
3. Move canned processing out of `unit.rs`.
4. Keep only actor/runtime state in the actor shell:
   - lifecycle state;
   - in-flight count;
   - timing and timeout data;
   - restart metadata;
   - generic health;
   - snapshot version;
   - framework telemetry.
5. Invoke application work and return the application's typed result.
6. Map panic, timeout, cancellation, and application error separately.
7. Run long or blocking application work outside the actor's message loop under a deliberate execution policy, while preserving per-unit concurrency limits.

### Work: unit registration and restart

1. Registration takes a unit descriptor plus restartable factory.
2. Persist the descriptor where configured.
3. Spawn through the dynamic supervisor.
4. On restart, create a new implementation from the factory and restore its snapshot.
5. Reconcile unit registrations when Operations or the unit supervisor restarts.
6. Stop relying on a stale dynamic-supervisor reference held indefinitely.

### Work: System 1 orchestration

1. Delegate validation and capability requirements to `WorkModel`.
2. Collect typed capability/capacity snapshots.
3. Delegate selection to `UnitSelectionPolicy`.
4. Enforce deadline and admission rules.
5. Dispatch typed work.
6. Record generic runtime metrics.
7. Delegate domain performance and variety interpretation.
8. Generate typed reports for Systems 2 and 3.
9. Generate a typed resource-shortage request when no unit is eligible.
10. Invoke `AlgedonicPolicy` on material deviations.

### Work: remove unsafe abstractions

1. Remove raw `GetState`/`UpdateState` from the core unit protocol.
2. Replace state synchronization with explicit coordination views and a reconciliation policy.
3. Replace numeric `MigrateWork(In/Out, amount)` with a typed migration plan and explicit prepare/commit/abort stages where migration is supported.
4. Replace runtime-generated audit JSON with application-provided audit evidence.
5. Keep generic metrics distinct from domain performance measures.

### Tests

- custom unit returns a typed domain result;
- validation comes from the work model;
- custom selector changes routing;
- no suitable unit emits a typed resource request;
- timeout and cancellation are distinct;
- unit crash restarts through its factory;
- snapshot restores after restart;
- Operations restart reconstructs its directory;
- unit supervisor restart is reconciled;
- application does not import actor APIs.

### Exit criteria

- no canned business result remains in `system1/unit.rs`;
- no JSON is required for System 1's core path;
- System 1 can run with at least two different example domains unchanged at the runtime layer;
- restart and reconciliation tests pass.

---

## Milestone 5: Replace the generic channel contract with typed protocols

### Objectives

Make internal VSM communication type-safe and give delivery failures explicit meaning.

### Work

1. Introduce a generic typed internal protocol enum or dedicated typed messages per subsystem actor.
2. Use direct typed actor references for control-path messages between Systems 1–5.
3. Introduce a typed `VsmEvent` stream for observers, telemetry, and extension integrations.
4. Remove automatic targeted-delivery fallback to broadcast.
5. Define explicit outcomes:
   - delivered;
   - target unavailable;
   - rejected by protocol validation;
   - deadline expired;
   - backpressured;
   - runtime shutting down.
6. Add dead-letter or undeliverable event reporting.
7. Make subscription ownership restart-safe:
   - subscriber actors re-register on broker generation change; or
   - a runtime directory rebuilds subscriptions centrally.
8. Move bounded history to a store or clearly optional in-memory event-log adapter.
9. Version JSON serialization separately from Rust's internal protocol version.
10. Keep extension channels possible without weakening canonical protocols.

### Exit criteria

- canonical inter-system messages are not JSON;
- a missing target never receives a hidden broadcast;
- broker restart does not permanently lose standard subscribers;
- observers can subscribe without participating in the control path;
- delivery and rejection metrics are available.

---

## Milestone 6: Convert System 2

### Objectives

Retain System 2's coordination loop while moving domain conflict and scheduling meaning into application policy.

### Work

1. Replace `ServiceActor` with a dedicated typed coordination actor.
2. Receive typed coordination views from System 1 units.
3. Maintain freshness and version information for each view.
4. Invoke `CoordinationPolicy` to identify conflicts, oscillations, dependencies, and proposed interventions.
5. Deliver typed recommendations or constraints to affected units.
6. Track acknowledgements and outcomes.
7. Escalate unresolved coordination conflicts to System 3.
8. Move current scheduler and balancer algorithms under `defaults` or `examples`.
9. Keep authoritative resource allocation out of System 2; it belongs to System 3.

### Exit criteria

- System 2 contains no string operation dispatch;
- the core does not assume time intervals, a `resource` JSON field, or fixed 20% thresholds;
- applications can replace coordination policy independently;
- conflict-to-intervention-to-acknowledgement is tested end to end.

---

## Milestone 7: Convert System 3 and System 3*

### Objectives

Separate internal control, resource governance, and independent audit into typed roles and actors.

### Work: System 3

1. Replace `ServiceActor` with a dedicated control actor.
2. Consume typed performance reports and resource bargains.
3. Invoke `ResourceGovernance` for grants, denials, or counteroffers.
4. Invoke `OperationalControlPolicy` for directives.
5. Track directive authority, version, acknowledgement, expiry, and outcome.
6. Publish operational summaries to Systems 4 and 5.

### Work: System 3*

1. Create a distinct audit actor under the System 3 supervisor.
2. Define typed audit request, scope, evidence, finding, severity, response, and remediation protocols.
3. Invoke the application's `Auditor` implementation.
4. Keep audit access separate from normal System 1 reporting.
5. Add audit authorization and sensitive-data boundaries.
6. Track remediation and verification.

### Migration

Move `system3/resources.rs` and `system3/audit.rs` algorithms into defaults/examples or delete them if they have no defensible generic meaning.

### Exit criteria

- no JSON/string dispatch remains on System 3's core path;
- resource shortage from System 1 can produce a typed allocation and acknowledgement;
- an independent audit can request application evidence and produce findings;
- authority and failed acknowledgement paths are tested.

---

## Milestone 8: Convert System 4

### Objectives

Build a typed, supervised environmental-intelligence pipeline.

### Work

1. Replace System 4 `ServiceActor` services with dedicated typed actors or a coordinated pipeline.
2. Register environment sources dynamically.
3. Supervise source polling/streaming separately from analysis.
4. Normalize observations with timestamp, provenance, confidence, and freshness.
5. Invoke `SignalInterpreter` and `IntelligenceModel`.
6. Invoke `Forecaster` or scenario planning roles.
7. compare forecasts with actual outcomes for calibration.
8. Construct typed adaptation proposals for System 5.
9. Request operational feasibility information from System 3.
10. Move current scanner, analytics, and forecasting functions to default/example implementations.

### Exit criteria

- core System 4 has no keyword extraction, fixed z-score policy, or fixed forecast multipliers;
- a failing source restarts without losing the entire intelligence subsystem;
- stale observations are detectable;
- a typed scenario reaches System 5 with provenance and uncertainty.

---

## Milestone 9: Convert System 5

### Objectives

Separate decision lifecycle from application-specific identity, values, and governance semantics.

### Work

1. Replace the four generic System 5 services with typed role adapters and a clear ownership model.
2. Decide which data is configuration and which behavior is a role:
   - identity document: data/provider;
   - values: data/provider;
   - alignment evaluation: behavior;
   - decision procedure: behavior;
   - crisis response: behavior.
3. Invoke `ValuesEvaluator`, `DecisionPolicy`, and `CrisisPolicy`.
4. Record decision evidence, alternatives, rationale, authority, and review date.
5. Version policies and identity changes.
6. Distribute typed directives and track acknowledgements.
7. Balance typed operational concerns from System 3 with future concerns from System 4.
8. Escalate decisions outside local authority to the parent recursion.
9. Move default mission text, keyword matching, fixed weighted scoring, and generic crisis directives out of core.

### Exit criteria

- the library imposes no default organizational mission, ethics, or decision values;
- policy decisions retain a complete typed audit trail;
- System 3/System 4 balancing is exercised end to end;
- crisis decisions are triggered through the algedonic path and can escalate recursively.

---

## Milestone 10: Rebuild variety, algedonic, and temporal capabilities

Status: Complete for ADR-0009 Option A as of 2026-06-19. Durable replay,
automatic broker-to-runtime subscription, richer defaults, and recursion
authority remain deferred to their owning milestones.

### Objectives

Keep generic mechanisms in core while moving domain interpretation to roles.

### Variety

1. Define typed variety estimates and uncertainty.
2. Retain generic windowing, time series, ratios, and trend mechanics.
3. Invoke `VarietyModel` for domain-relevant measurement.
4. Invoke `VarietyEngineeringPolicy` for attenuation/amplification choices.
5. Track intervention outcome so policies can be evaluated.
6. Move JSON key-counting and current attenuation/amplification heuristics to defaults/examples.

### Algedonic signaling

1. Define a typed signal lifecycle:
   - proposed;
   - classified;
   - dispatched;
   - acknowledged;
   - acted upon;
   - resolved;
   - escalated;
   - expired.
2. Invoke `AlgedonicPolicy` for classification and severity.
3. Guarantee a priority path to System 5.
4. Add acknowledgement timers and recursive escalation.
5. Add deduplication and correlation.
6. Route external notifications through `AlertSink`.
7. Remove process-global alert history.
8. Keep filters and thresholds as configuration or defaults.

### Temporal analysis

1. Keep generic timescale/window/aggregation infrastructure.
2. Make pattern, causality, and forecasting algorithms replaceable.
3. Version data schemas at adapter boundaries.

### Exit criteria

- the core does not infer variety from JSON structure;
- high-priority algedonic signals cannot be silently treated as ordinary broadcasts;
- acknowledgement timeout causes tested escalation;
- alert history is actor/store owned;
- temporal algorithms are clearly optional strategies.

---

## Milestone 11: Make recursion operational

### Objectives

Turn recursion from a data structure into a runtime capability.

### Work

1. Introduce instance and recursion-path addressing everywhere.
2. Allow a System 1 unit to be either:
   - a leaf `OperationalUnit`; or
   - a child VSM bridge backed by a nested runtime.
3. Start and supervise child runtimes through an explicit recursion manager.
4. Define parent/child protocols for:
   - delegated work;
   - performance aggregation;
   - resource requests;
   - policy directives;
   - intelligence summaries;
   - algedonic escalation.
5. Invoke `RecursionTransducer` when information crosses a recursion boundary.
6. Enforce authority and information-disclosure boundaries.
7. Prevent registry-name collisions through instance-scoped addressing.
8. Track correlation across parent and child requests.

### Exit criteria

- a two-level VSM can execute work through a child runtime;
- a child can escalate a resource request and an algedonic alert to its parent;
- parent policy can be transduced into child-level directives;
- two child VSMs do not collide in actor registry names.

---

## Milestone 12: Persistence, recovery, and reconciliation

### Objectives

Make supervision restore a coherent system rather than merely respawn empty actors.

### Work

1. Implement an in-memory `StateStore` adapter for tests.
2. Define snapshot keys using instance, recursion path, role, and entity ID.
3. Add snapshot schema versions and migrations.
4. Persist:
   - registered unit descriptors/factory keys;
   - role snapshots where supported;
   - policy and identity versions;
   - pending decisions/directives;
   - pending audit remediation;
   - unresolved algedonic signals;
   - optional event cursors/history.
5. Add actor startup reconciliation:
   - restore state;
   - discover surviving children;
   - recreate missing children;
   - rebuild subscriptions;
   - verify protocol versions;
   - signal readiness only after reconciliation.
6. Add a durable factory registry strategy for units that must be recreated after process restart.
7. Define behavior when a role snapshot cannot be restored.
8. Add optional external store adapters later without coupling core to a database.

### Exit criteria

- restarting each major actor preserves documented invariants;
- process restart can reconstruct configured units when using a persistent adapter and factory registry;
- incompatible snapshots fail clearly rather than being silently ignored;
- readiness waits for reconciliation.

---

## Milestone 13: Backpressure, execution, and observability hardening

### Objectives

Prepare the runtime for real workloads and diagnosable failures.

### Work

1. Add explicit per-subsystem and per-unit admission limits.
2. Add in-flight limits and queue policies.
3. Make deadlines and cancellation flow through every role context.
4. Move blocking work to configured task execution or dedicated worker actors.
5. Add retry policies based on failure classification, not blanket retries.
6. Add circuit-breaker or degraded-mode extension points for external sources/adapters.
7. Emit structured telemetry for:
   - actor starts/stops/restarts;
   - mailbox or admission pressure;
   - call latency;
   - work outcomes;
   - selection failures;
   - rejected protocols;
   - broker delivery failures;
   - resource bargain duration;
   - audit duration/findings;
   - forecast freshness;
   - policy decision latency;
   - algedonic acknowledgement/escalation.
8. Add deterministic test clock and ID generator.
9. Add stress and fault-injection tests.

### Exit criteria

- overload behavior is configured and tested;
- no core path can wait forever without an explicit policy;
- operational metrics explain why work was delayed, rejected, failed, or escalated;
- fault-injection tests cover actor crash, store failure, unavailable target, source failure, and slow role implementation.

---

## Milestone 14: Remove scaffolding and prepare the public crate

### Objectives

Finalize the public architecture and remove porting artifacts.

### Work

1. Delete or feature-gate `actor_support::ServiceActor`.
2. Remove public string-operation APIs.
3. Remove application-specific `SystemId` variants such as starter/test ecosystem identifiers.
4. Make actor modules private or doc-hidden.
5. Remove global `names` from the public API.
6. Move all demo policies to examples/defaults.
7. Decide whether `legacy-json` ships for one transitional minor release or is removed before first publication.
8. Rewrite examples around domain role implementations and the builder.
9. Update:
   - `README.md`;
   - `ARCHITECTURE.md`;
   - `USAGE.md`;
   - `DEVELOPERS.md`;
   - crate-level Rustdoc;
   - migration guide;
   - feature documentation.
10. Add complete crates.io metadata and remove `publish = false`.
11. Run:
   - format;
   - check on MSRV and stable;
   - Clippy;
   - tests and doctests;
   - documentation build with all feature combinations;
   - dependency/security audit;
   - minimal-version or semver checks as appropriate;
   - `cargo publish --dry-run`.

### Exit criteria

- application-facing examples contain no `ActorRef`, global actor names, or string service operations;
- core workflows do not require JSON;
- every default algorithm is labeled non-normative;
- public APIs have rustdoc examples and stability notes;
- publication dry run succeeds.

---

## 6. Recommended release train

The exact version numbers can change, but each release should have a coherent user story.

### 0.2: Contracts and System 1

- baseline stabilization;
- typed protocol foundations;
- role contracts;
- builder/runtime handle;
- trait-backed System 1;
- instance-scoped runtime;
- legacy JSON adapter where needed.

This is the first release that demonstrates the intended public architecture.

### 0.3: Typed control loop

- typed internal protocols;
- System 2;
- System 3 and 3*;
- delivery acknowledgements;
- initial persistence/reconciliation.

### 0.4: Intelligence and governance

- System 4;
- System 5;
- policy lifecycle;
- typed algedonic integration.

### 0.5: Variety, temporal behavior, and recursion

- trait-driven variety;
- hardened algedonic lifecycle;
- temporal strategy interfaces;
- nested VSM runtime support.

### 0.6–0.9: Hardening and API convergence

- persistence adapters;
- backpressure;
- fault injection;
- observability;
- performance work;
- deprecation/removal of legacy API;
- downstream adopter feedback;
- API freeze candidate.

### 1.0: Stable role and protocol surface

Release only after:

- at least two substantially different example domains use the same runtime contracts;
- recursive composition is demonstrated;
- restart and recovery invariants are tested;
- no core role depends on JSON;
- the application API hides actor implementation details;
- protocol and persistence versioning policies are documented.

---

## 7. Current-file migration map

| Current file or area | Planned treatment |
|---|---|
| `app.rs` | Replace public startup functions with builder/runtime startup; retain internal supervisor construction. |
| `vsm_core.rs` | Replace global facade with typed runtime and subsystem handles. |
| `names.rs` | Move internal; generate names from runtime instance and recursion path. |
| `domain.rs` | Split into typed protocols, addressing, metadata, and optional JSON adapters. |
| `actor_support.rs` | Freeze, feature-gate as legacy, then remove. |
| `channels/broker.rs` | Split typed control protocols from observer event bus; add explicit delivery outcomes and restart reconciliation. |
| `channels/mod.rs` | Stop exposing broker lookup; expose subscriptions/observation through runtime handles. |
| `system1/unit.rs` | Keep private actor shell; invoke `OperationalUnit`; remove demo result and raw state APIs. |
| `system1/operations.rs` | Keep orchestration; delegate work interpretation, selection, performance, variety, migration, and audit semantics. |
| `system1/transaction.rs` | Replace with typed work envelope and legacy adapter. |
| `system1/metrics.rs` | Retain runtime counters; separate application performance model. |
| `system2/coordination.rs` | Replace generic service dispatch with typed coordination actor. |
| `system2/scheduler.rs` | Move to optional default/example policy. |
| `system2/balancer.rs` | Move to default/example; move authoritative resource allocation to System 3. |
| `system3/control.rs` | Replace with typed control actor. |
| `system3/resources.rs` | Convert to `ResourceGovernance` default/example implementation. |
| `system3/audit.rs` | Convert to `Auditor` default/example; add dedicated System 3* actor. |
| `system4/intelligence.rs` | Replace with typed intelligence pipeline coordinator. |
| `system4/scanner.rs` | Convert to source/interpreter defaults or examples. |
| `system4/analytics.rs` | Convert to intelligence-model default/example. |
| `system4/forecasting.rs` | Convert to forecaster default/example. |
| `system5/policy.rs` | Keep policy lifecycle in runtime; delegate semantic decisions. |
| `system5/identity.rs` | Split identity data/provider from alignment behavior. |
| `system5/values.rs` | Split values data/provider from evaluator behavior. |
| `system5/decisions.rs` | Convert algorithm into application `DecisionPolicy` or optional default. |
| `channels/algedonic/*` | Keep priority/escalation mechanism; extract classification, routing policy, and external alert sink. |
| `channels/temporal/*` | Keep generic windows/aggregation; make analysis algorithms strategies. |
| `shared/variety/*` | Keep typed measurements/time-series mechanics; move formulas/interventions to roles/defaults. |
| `shared/recursion.rs` | Replace descriptive tree with runtime address/manager plus application transducers. |
| `telemetry_reporter.rs` | Convert to telemetry port and internal runtime instrumentation. |
| `prelude.rs`, `util.rs` | Remove JSON-centric helpers from core; keep only internal generic utilities. |

---

## 8. Test strategy for the migration

### 8.1 Contract tests

Every role should have a reusable contract test suite verifying:

- lifecycle behavior;
- cancellation/deadline behavior;
- error classification;
- snapshot compatibility where supported;
- no actor/runtime dependency leaks into the role.

### 8.2 Actor-adapter tests

For each internal actor adapter:

- role invocation occurs exactly once unless retry policy says otherwise;
- actor state remains coherent after returned errors;
- panic/crash triggers intended supervisor behavior;
- timeout does not corrupt subsequent requests;
- restart creates a fresh implementation through its factory;
- snapshot and registration reconciliation are correct.

### 8.3 Protocol tests

- legal routes compile and deliver;
- invalid routes are rejected;
- correlation and causation propagate;
- targeted failures do not broadcast;
- deadlines expire predictably;
- version mismatch is explicit;
- observer events do not control system behavior.

### 8.4 VSM flow tests

Maintain one end-to-end scenario for each major loop:

1. work accepted and completed;
2. no capability → System 3 resource bargain;
3. coordination conflict → System 2 intervention;
4. unresolved conflict → System 3 escalation;
5. System 3* audit → evidence → finding → remediation;
6. environmental observation → System 4 scenario → System 5 decision;
7. performance deviation → algedonic signal → System 5 response;
8. child VSM alert → parent escalation;
9. policy directive → System 1 acknowledgement;
10. restart/recovery during each flow.

### 8.5 Example-domain requirement

Use at least two contrasting domains to prevent accidental overfitting:

- a request-processing/service domain;
- a physical or resource-constrained operations domain.

The same role traits and runtime should support both without core changes.

---

## 9. Pull-request decomposition

Avoid one large rewrite. Recommended PR boundaries:

1. CI and readiness baseline.
2. Characterization tests.
3. Protocol metadata and addresses.
4. Application type family.
5. First-wave role traits and test fakes.
6. Builder/runtime handle with legacy backend.
7. Instance-scoped naming and multi-runtime test.
8. System 1 unit factory and role adapter.
9. System 1 typed work pipeline.
10. System 1 performance/variety/algedonic extraction.
11. Typed control protocols and removal of broadcast fallback.
12. Broker/subscription reconciliation.
13. System 2 conversion.
14. System 3 conversion.
15. System 3* conversion.
16. System 4 source pipeline.
17. System 4 analysis/forecast roles.
18. System 5 identity/values/decision roles.
19. Algedonic lifecycle and alert sink.
20. Variety and temporal strategy conversion.
21. Runtime recursion.
22. Persistence/recovery.
23. Backpressure and telemetry hardening.
24. Legacy removal and publication cleanup.

Every PR should include tests and documentation for the boundary it introduces.

---

## 10. Guardrails

Do not:

- ask application authors to implement `ractor::Actor`;
- expose `ActorRef` as the normal public API;
- make core role methods accept or return `serde_json::Value`;
- turn every numeric threshold into a trait;
- preserve targeted-to-broadcast fallback;
- treat raw state replication as coordination;
- make System 2 the authoritative resource allocator;
- place application identity or ethics defaults in core;
- classify JSON key counting as a universal variety model;
- add persistence by coupling the runtime to one database;
- depend on process-global mutable state;
- attempt recursion only as a metadata tree;
- change all five systems in one unreviewable patch.

Prefer:

- typed protocols;
- explicit role ownership;
- configuration for static choices;
- traits for algorithms, external integrations, and domain interpretation;
- actor-owned runtime state;
- adapter-owned durable state;
- explicit delivery, acknowledgement, timeout, and escalation semantics;
- small vertical slices with end-to-end tests.

---

## 11. Definition of the desired state

The migration is complete when an application can:

1. define its domain types;
2. implement operational and governance roles as ordinary Rust traits;
3. assemble those roles with a builder;
4. start multiple independent VSM instances;
5. submit typed work and receive typed outcomes;
6. observe typed VSM events;
7. replace any policy without modifying actor code;
8. recover coherent state after supervised restarts;
9. nest a child VSM as a System 1 unit;
10. use JSON only at chosen external boundaries;
11. avoid direct knowledge of actors, registries, supervisors, and channel internals.

At that point the crate is no longer merely a Rust translation of the Elixir implementation. It is a reusable VSM runtime with a stable separation between cybernetic mechanism and application meaning.
