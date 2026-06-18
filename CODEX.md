# Codex Development Log

This file is the live execution journal for the migration described in
[`IMPLEMENTATION.md`](IMPLEMENTATION.md).

It records the current repository state, completed work, validation evidence,
decisions, risks, and the exact next task.

Do not use this file as the architectural source of truth. Architectural goals
and acceptance criteria live in `IMPLEMENTATION.md`. Durable decisions live in
`docs/adr/`.

---

## Approval state

- Approved milestone: Milestone 7 — System 3 and System 3* migration
- Approved scope: Replace System 3's JSON `ServiceActor` core path with typed
  control/resource governance and a distinct System 3* audit actor, consume
  typed performance/resource inputs, invoke application-owned governance,
  control, and audit roles, track authority/version/acknowledgement/expiry,
  keep audit access separate from normal System 1 reporting, move current
  resource/audit algorithms under defaults/examples or remove them, and update
  tests/documentation.
- Approved architectural decisions: Recorded in ADR-0001 through ADR-0006
- Pending decisions: None for the approved Milestone 7 scope
- Permission to begin next milestone: No

## Pending user decisions

| ID | Decision | Options | Recommendation | Blocking milestone | Status |
|---|---|---|---|---|---|
| S2-001 | Public `CoordinationPolicy` role shape | A. Minimal view-centric policy over typed System 1 coordination views, generic conflict/intervention/ack records, no new `ViableSystem` associated types; B. System 2 extension type family with app-owned conflict/intervention payload types; C. Keep System 2 policy private for this slice and defer public replacement | A | Milestone 6 | Approved 2026-06-18; recorded in ADR-0005 |
| S3-001 | Public System 3/System 3* role boundary | A. Minimal framework-owned records with `ResourceGovernance`, `OperationalControlPolicy`, and `Auditor` roles over existing `ViableSystem` types; B. Add a System 3 extension type family for app-owned resource/directive/audit payloads; C. Convert control/resource governance now and defer System 3* audit | A | Milestone 7 | Approved 2026-06-18; recorded in ADR-0006 |

## Current status

- Overall state: Milestone 7 complete; review gate reached
- Current phase: Review gate after Milestone 7 — System 3 and System 3*
  migration
- Current milestone: Typed System 3 control/resource governance and System 3*
  audit
- Last updated: 2026-06-18
- Last updated by: Codex
- Baseline commit: `1e1aed5`
- Working branch: `master`
- Repository clean at start: No; earlier migration changes remained
  uncommitted in the working tree.
- Repository status now: Milestone 7 implementation complete with uncommitted
  changes; validation passed and review gate is reached.

## Current objective

Milestone 7 has been implemented with the approved Option A role boundary:
typed System 3 resource governance/control and distinct System 3* audit roles
over framework-owned records without adding new `ViableSystem` associated
types.

## Next action

Stop at the Milestone 7 review gate. Present validation evidence and wait for
explicit user approval before beginning Milestone 8: System 4 migration.

---

## Milestone status

| Phase | Milestone | Status | Evidence |
|---|---|---:|---|
| 0 | Repository baseline | Complete | Formatting, check, tests, Clippy, docs, doctests, and example validation pass. |
| 0 | Characterization tests | Complete | `tests/phase0_characterization.rs` covers startup/health, System 1 no-unit resource request, explicit delivery outcomes, removed targeted fallback, broadcast validation, removed System 2/System 3 JSON dispatch, and remaining Systems 4-5 JSON service calls. Existing System 1 and full-system tests still pass. |
| 0 | ADR setup | Complete | `docs/adr/README.md`, template, and ADR-0001 through ADR-0004 added. |
| 1 | Application type family | Complete | `src/roles/types.rs` defines `ViableSystem`; `tests/foundational_types.rs` proves non-serde application work, outcome, and snapshot payloads compile. |
| 1 | Typed core envelopes | Complete | `src/protocol/*`, `src/error.rs`, `src/cancellation.rs`, `src/roles/ports.rs`, and `src/legacy/*` added with tests, docs, and full validation passing. |
| 2 | Role contracts and factories | Complete | `src/roles/context.rs`, `src/roles/system1.rs`, expanded `src/roles/ports.rs`, and `tests/role_contracts.rs` added; full validation passes. |
| 2 | Runtime builder and handles | Complete | `src/builder.rs`, `src/config.rs`, `src/runtime.rs`, private `src/kernel/registry.rs`, `tests/runtime_builder.rs`, and `examples/typed_runtime_builder.rs`; full validation passes. |
| 3 | System 1 vertical slice | Complete | `src/kernel/system1.rs`, expanded `src/runtime.rs`, `tests/system1_typed_runtime.rs`, and `examples/typed_runtime_builder.rs`; full validation passes. |
| 4 | Typed protocol bus | Complete | `src/protocol/bus.rs`, `src/kernel/event_bus.rs`, expanded `src/channels/broker.rs`, runtime observer APIs, tests, docs, and full validation pass. |
| 5 | System 2 migration | Complete | `src/protocol/system2.rs`, `src/roles/system2.rs`, `src/kernel/system2.rs`, expanded runtime handles, updated System 1 coordination hooks, defaults relocation, docs, and `tests/system2_typed_runtime.rs`; full validation passes. |
| 6 | System 3 and System 3* migration | Complete | ADR-0006 accepted as Option A; `src/protocol/system3.rs`, `src/roles/system3.rs`, `src/kernel/system3.rs`, `System3Handle`, builder hooks, docs, and `tests/system3_typed_runtime.rs` added; full validation passes. |
| 7 | System 4 migration | Not started | Awaiting user approval. |
| 8 | System 5 migration | Not started | Awaiting user approval. |
| 9 | Variety and algedonic migration | Not started | Awaiting user approval. |
| 10 | Temporal processing | Not started | Awaiting user approval. |
| 11 | Recursive runtimes | Not started | Awaiting user approval. |
| 12 | Persistence and recovery | Not started | Awaiting user approval. |
| 13 | Publication hardening | Not started | Awaiting user approval. |

Allowed status values:

- `Not started`
- `In progress`
- `Blocked`
- `Complete`
- `Deferred`

A milestone may be marked `Complete` only when its code, tests, validation, and
documentation are complete.

---

## Validation status

| Command | Result | Last run | Notes |
|---|---:|---|---|
| `cargo fmt --all -- --check` | Passed | 2026-06-18 | No formatting drift. |
| `CARGO_INCREMENTAL=0 cargo check --all-targets --all-features --locked` | Passed | 2026-06-18 | No warnings. |
| `CARGO_INCREMENTAL=0 cargo test --all-targets --all-features --locked` | Passed | 2026-06-18 | 54 integration tests across foundational, full-system, Phase 0, role-contract, runtime-builder, typed-System-1, typed-System-2, and typed-System-3 suites; example test targets have 0 tests. |
| `CARGO_INCREMENTAL=0 cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed | 2026-06-18 | No warnings after factoring the private System 3 audit snapshot reply alias. |
| `CARGO_INCREMENTAL=0 cargo doc --all-features --no-deps --locked` | Passed | 2026-06-18 | Generated `target/doc/vsm_rs/index.html`. |
| `CARGO_INCREMENTAL=0 cargo test --doc --all-features --locked` | Passed | 2026-06-18 | 0 doctests. |
| `CARGO_INCREMENTAL=0 cargo run --example typed_runtime_builder --locked` | Passed | 2026-06-18 | Example starts typed runtime handle through `VsmBuilder`, registers a typed unit, processes typed work, and shuts down. |
| `CARGO_INCREMENTAL=0 cargo run --example basic_usage --locked` | Passed | 2026-06-18 | Example starts runtime, registers `payments`, processes a transaction, prints status, records System 2 target unavailability on the legacy coordination channel, and exits. |
| `git diff --check` | Passed | 2026-06-18 | No whitespace errors. |

Do not replace failing results with “not run.” Preserve the most recent failure
until a subsequent run succeeds.

---

## Completed work

- Recorded Phase 0 start state and final evidence in this journal.
- Fixed original formatting baseline with `cargo fmt --all`.
- Fixed original Clippy failures without changing architecture:
  - removed unused broker import and ignored unused validation error binding;
  - replaced redundant spawn closures with function items;
  - used assign-op for scheduler cursor advancement;
  - filtered before cloning analytics anomalies;
  - replaced manual numeric clamp with `f64::clamp`.
- Added ADR process and accepted-decision records under `docs/adr/`.
- Corrected factual documentation drift in README and docs:
  - moved root documentation links to `docs/`;
  - documented that `PORTING_MAP.md` is not currently present;
  - replaced stale generated-environment build note;
  - corrected local dependency path;
  - documented current `publish = false` status.
- Added focused characterization tests for current behavior.
- Tightened `tests/full_system_flow.rs` shutdown to await the returned join
  handle instead of using the non-waiting `stop()` facade.
- Added typed protocol foundations:
  - `src/protocol/*` for runtime addresses, metadata, snapshots, System 1
    protocol records, events, and reports;
  - `src/roles/*` for `ViableSystem`, `StateStore`, event/report sinks, and
    no-op ports;
  - `src/cancellation.rs` for crate-owned cooperative cancellation;
  - `src/error.rs` framework/application/work error wrappers;
  - `src/legacy/*` temporary adapters for current JSON transaction/message
    shapes.
- Re-exported `ViableSystem`, `FrameworkError`, `ApplicationFailure`,
  `WorkError`, and `async_trait` from the crate root.
- Added `tests/foundational_types.rs` to prove no-serde payload bounds, no-op
  port behavior, cancellation mapping, legacy round-trips, and error
  separation.
- Updated README, architecture, and usage docs to describe the foundation
  modules and their current non-wired status.
- Added first-wave System 1 role contracts:
  - `OperationalUnit`, `OperationalUnitFactory`, `WorkModel`,
    `UnitSelectionPolicy`, `PerformanceModel`, `VarietyModel`,
    `AlgedonicPolicy`, and `System1Roles`;
  - object aliases for boxed/shared dynamic role dispatch;
  - opt-in default lowest-load selection and no-op performance, variety, and
    algedonic policies;
  - downstream test helpers for static units and accept-all work models.
- Added `RoleContext` and `UnitRoleContext` with runtime identity, recursion
  path, framework metadata, deadline, cancellation, clock, event/report sinks,
  and explicitly allowed state store access.
- Added `TelemetrySink`, `AlertSink`, `Clock`, `IdGenerator`, and no-op/system
  implementations to `roles::ports`.
- Added `tests/role_contracts.rs` to prove downstream-style role
  implementations, dyn compatibility, no direct `ractor`/`serde_json` imports,
  defaults/no-ops, test fakes, and context boundaries.
- Updated README, architecture, usage, and developer docs for role contracts and
  contexts.
- Added the typed runtime lifecycle surface:
  - `RuntimeConfig` for instance ID, recursion path, timeouts, unit-capacity
    admission configuration, and event-buffer capacity;
  - `VsmBuilder` for required System 1 role validation, optional/default
    policy injection, runtime ports, and async `start()`;
  - `VsmRuntime`, `System1Handle`, readiness checks, lifecycle state, shutdown
    acknowledgement, runtime ports, and System 1 role bundle accessors;
  - private `kernel::registry` scaffold for instance-derived internal
    component names and directory snapshots.
- Added `tests/runtime_builder.rs` to prove missing-role validation, default
  policy behavior, deterministic readiness, instance-scoped directory names,
  two coexisting runtime handles, role-context identity, and idempotent
  shutdown.
- Added `examples/typed_runtime_builder.rs` as a runnable typed builder
  lifecycle example.
- Updated README, architecture, usage, and developer docs for the builder,
  runtime handle, readiness, shutdown, and current non-actor-backed boundary.
- Added the actor-backed typed System 1 runtime path:
  - private `kernel::system1` unit actor adapters that own
    `OperationalUnit` implementations;
  - typed unit registration through `UnitRegistration`;
  - `System1Handle` APIs for register, list, process, response wrapping,
    drain, and unregister;
  - role-driven work validation, capability derivation, and unit selection;
  - basic admission/backpressure and deadline enforcement;
  - typed resource-shortage events and performance reports through configured
    sinks;
  - typed snapshot restore on registration and save on unregister through
    `StateStore`.
- Added public typed System 1 runtime support types:
  - `UnitAdmissionLimits`;
  - `UnitSnapshotConfig`;
  - `UnitRegistration`;
  - `RegisteredUnit`.
- Updated `examples/typed_runtime_builder.rs` to register a typed unit and
  process typed work.
- Added `tests/system1_typed_runtime.rs` for typed work processing, work-model
  validation, custom selection, resource-shortage events, admission
  backpressure, deadline timeout, drain/unregister lifecycle, and snapshot
  restore/save behavior.
- Updated README, architecture, usage, and developer docs for actor-backed typed
  System 1 behavior.
- Added typed bus delivery foundations:
  - `src/protocol/bus.rs` for `DeliveryStatus`, `DeliveryMetrics`,
    `RuntimeControlMessage`, and `System1ControlMessage`;
  - crate-root re-exports for typed delivery/control message records.
- Added the typed runtime observer event bus:
  - private `kernel::event_bus` implementing the `EventSink` port;
  - `VsmRuntime::subscribe_events`, `observer_event_history`, and
    `observer_bus_snapshot`;
  - bounded newest-first runtime event history and non-blocking fan-out;
  - downstream event sink failure counting without failing the control path.
- Reworked the legacy channel broker delivery boundary:
  - `DeliveryOutcome` and `UndeliverableMessage`;
  - `publish_with_outcome` and `broadcast_with_outcome`;
  - channel `dead_letters`;
  - delivery metrics in `ChannelStats`;
  - missing targeted subscribers now produce `TargetUnavailable` and dead
    letters instead of falling back to broadcast;
  - explicit broadcast now requires `SystemId::All` and records rejected
    targeted messages as `RejectedByProtocol`.
- Updated channel-specific broadcast helpers to construct explicit
  `SystemId::All` broadcasts.
- Added and updated tests for typed control records, broker delivery outcomes,
  dead letters, removed fallback delivery, validated broadcast, observer
  subscriptions, and non-blocking sink failure behavior.
- Updated README, architecture, usage, and developer docs for explicit broker
  delivery semantics, dead letters, delivery metrics, and typed observer
  subscriptions.
- Added typed System 2 coordination foundations:
  - `src/protocol/system2.rs` for coordination view records, conflicts,
    interventions, acknowledgements, escalations, cycles, and snapshots;
  - System 2 event/report/control message variants in `src/protocol/events.rs`
    and `src/protocol/bus.rs`;
  - `src/roles/system2.rs` for the public `CoordinationPolicy` role,
    `System2Roles`, shared dynamic policy dispatch, and no-op default policy.
- Added the private typed System 2 runtime adapter:
  - `src/kernel/system2.rs` owns view freshness/version tracking, policy
    invocation, intervention planning, acknowledgement tracking, escalation
    records, and event/report emission;
  - `VsmRuntime` now starts and shuts down a System 2 runtime alongside System
    1 and exposes `System2Handle`;
  - `System2Handle` can coordinate supplied views, query System 1 views,
    acknowledge interventions, and produce a typed snapshot.
- Extended the typed System 1 path so unit actors can expose coordination views
  and receive typed coordination interventions through
  `OperationalUnit::handle_coordination_intervention`.
- Removed the legacy System 2 JSON `ServiceActor` core path:
  - deleted `src/system2/coordination.rs`;
  - moved scheduler and balancer examples under `src/system2/defaults/`;
  - removed `ServiceKind::System2Coordination` dispatch;
  - changed the legacy System 2 supervisor to start no JSON coordination child.
- Added `tests/system2_typed_runtime.rs` for downstream-style policy
  replacement, conflict detection, intervention delivery, acknowledgement
  recording, rejection escalation, view-version advancement, and no-op default
  behavior.
- Updated README, architecture, usage, and developer docs for typed System 2,
  the then-remaining later-subsystem JSON boundaries, and the moved defaults.
- Accepted ADR-0006 for the minimal System 3/System 3* Option A role boundary.
- Added typed System 3 protocol foundations:
  - `src/protocol/system3.rs` for resource requests, resource decisions and
    allocations, allocation acknowledgements, control authorities,
    operational directives, directive acknowledgements, operational summaries,
    System 3* audit requests, evidence boundaries, findings, remediations,
    audit responses, and snapshots;
  - System 3 control-message, event, and report variants in
    `src/protocol/bus.rs` and `src/protocol/events.rs`.
- Added public System 3 role contracts:
  - `ResourceGovernance`;
  - `OperationalControlPolicy`;
  - `Auditor`;
  - `System3Roles`;
  - shared dynamic role aliases and opt-in defaults for deny-all governance,
    no-op control, and no-op audit.
- Added the private typed System 3 runtime adapter:
  - `src/kernel/system3.rs` starts separate control and System 3* audit
    actors;
  - control invokes application governance/control roles, tracks
    authority/version/expiry/acknowledgement records, emits events, and records
    reports;
  - audit invokes application auditors with evidence collected through a
    separate System 1 audit path and applies audit boundaries before role
    dispatch.
- Extended the typed runtime surface:
  - `VsmBuilder` now accepts optional System 3 governance/control/auditor
    roles;
  - `VsmRuntime` starts and shuts down typed System 3 alongside System 1 and
    System 2;
  - `System3Handle` exposes resource governance, resource-shortage handling,
    directive acknowledgement, System 3* audit, supplied-evidence audit, role
    contexts, and snapshots.
- Extended the typed System 1 adapter with operational-directive delivery and
  audit-evidence collection used by System 3/System 3*.
- Removed the legacy System 3 JSON `ServiceActor` core path:
  - deleted `src/system3/control.rs`, `src/system3/resources.rs`, and
    `src/system3/audit.rs`;
  - moved old JSON resource and audit helper algorithms under
    `src/system3/defaults/`;
  - removed `ServiceKind::System3Control` dispatch;
  - changed the legacy System 3 supervisor to start no JSON control child.
- Added `tests/system3_typed_runtime.rs` for downstream-style governance,
  directive delivery/acknowledgement, System 3* audit authorization and
  evidence collection, and default/no-op behavior.
- Updated Phase 0 and full-flow tests for removed System 3 JSON dispatch and
  retained Systems 4-5 JSON services.
- Updated README, architecture, usage, and developer docs for typed System 3,
  remaining Systems 4-5 JSON boundaries, and the moved defaults.

---

## Work in progress

No implementation is currently in progress. Milestone 7 is complete under
ADR-0006 Option A and is paused at the review gate.

---

## Decisions made

The user approved the Phase 0-only scope, approved Milestone 1 after the Phase
0 review gate, approved Milestone 2 after the Milestone 1 review gate, and
approved Milestone 3 after the Milestone 2 review gate, approved Milestone 4
after the Milestone 3 review gate, approved Milestone 5 after the Milestone 4
review gate, approved Milestone 6 after the Milestone 5 review gate, and
approved Milestone 7 after the Milestone 6 review gate. Accepted migration
decisions are recorded as ADRs.

| ADR | Decision | Status |
|---|---|---|
| [ADR-0001](docs/adr/0001-clean-breaking-migration-posture.md) | Clean breaking migration posture and Phase 0 boundary | Accepted |
| [ADR-0002](docs/adr/0002-application-type-family-and-role-contracts.md) | Minimal application type family and role contract shape | Accepted |
| [ADR-0003](docs/adr/0003-system1-runtime-semantics.md) | First System 1 runtime semantics | Accepted |
| [ADR-0004](docs/adr/0004-protocol-boundaries-and-deferred-decisions.md) | Protocol boundaries and explicitly deferred choices | Accepted |
| [ADR-0005](docs/adr/0005-system2-coordination-policy.md) | Minimal view-centric System 2 coordination policy | Accepted |
| [ADR-0006](docs/adr/0006-system3-role-boundary.md) | Minimal System 3/System 3* role boundary | Accepted |

Milestone 1 introduced no new ADR-level architectural decisions. Implementation
notes:

- `UnitSnapshot` is `Send + 'static` because the always-present async
  `StateStore` boundary must be safe across runtime tasks. It is not required
  to implement `Serialize`, `Deserialize`, `Clone`, or `Debug`.
- Framework-owned metadata derives serde where useful; application work,
  outcome, and snapshot payloads do not require serde.
- Milestone 2 introduced no new ADR-level decisions. Implementation notes:
  - `OperationalUnit` methods use `&mut self` so application unit state only
    needs to satisfy `Send`, not `Sync`; policy/model/factory roles use `&self`
    and require `Send + Sync` because they are shared.
  - Work model and variety methods move `WorkRequest`/`WorkResponse` values
    rather than borrowing app payloads across async futures, preserving the
    accepted `Work`/`Outcome: Clone + Send + 'static` bounds.
  - `WorkRequest` and `UnitDescriptor` now have manual `Clone`
    implementations so the application type family itself is not required to
    implement `Clone`.
- Milestone 3 introduced no new ADR-level decisions. Implementation notes:
  - `VsmBuilder::start()` is async even though the first lifecycle shell does
    not await actor startup yet, preserving room for actor-backed startup
    without another immediate public lifecycle break.
  - Required role validation is runtime validation that returns
    `FrameworkError::InvalidProtocol`; no typestate builder was added in this
    slice.
  - Readiness gates use `Ready`, `NotApplicable`, `Pending`, and `Failed`.
    `NotApplicable` gates are treated as satisfied so the non-actor-backed
    lifecycle shell can report deterministic readiness without pretending that
    actor adapters or typed observer subscriptions exist yet.
  - Private runtime component names are generated from `RuntimeId`,
    `RecursionPath`, subsystem role, and entity label; no global actor lookup
    or `ActorRef` is exposed through the typed runtime handle.
- Milestone 4 introduced no new ADR-level decisions. Implementation notes:
  - The typed runtime path uses private unit actor adapters under
    `kernel::system1`; public handles expose no `ActorRef`, actor names, or
    JSON payloads.
  - The first actor-backed slice covers register, list, process, drain, and
    unregister. Automatic restart/reconciliation remains deferred.
  - `UnitRegistration` may provide a per-unit factory and snapshot/admission
    configuration; `System1Handle::register_descriptor` uses the runtime's
    default factory role for the common case.
  - Observer event/report sink failures are not allowed to fail the work
    control path in this slice.
- Milestone 5 introduced no new ADR-level decisions. Implementation notes:
  - The legacy broker now reports target correctness directly: a missing target
    records a `TargetUnavailable` outcome and dead letter instead of falling
    back to broadcast.
  - Explicit broadcast is valid only for messages addressed to `SystemId::All`;
    targeted messages sent through the broadcast path are recorded as
    `RejectedByProtocol`.
  - The typed runtime observer bus is private runtime machinery that implements
    `EventSink`; public subscribers receive `RuntimeEvent<V>` values through
    `VsmRuntime::subscribe_events`.
  - Observer fan-out and downstream event sink failures are non-blocking for
    the control path. Failures are counted in `ObserverBusSnapshot`.
  - Later subsystem typed semantics remained deferred; Milestone 5 adds bus
    mechanics and status records, not subsystem role catalogs.
- Milestone 6 introduced ADR-0005. Implementation notes:
  - `CoordinationPolicy` is public, view-centric, object-safe, and replaceable
    without adding new required associated types to `ViableSystem`.
  - System 2 records are framework-owned and generic over existing unit
    identity/capability types; scheduling/resource meaning remains policy,
    defaults, adapter, or later-extension responsibility.
  - The typed System 2 runtime runs inside `VsmRuntime`; the legacy global
    facade no longer starts a System 2 JSON coordination service child.
  - Rejected intervention acknowledgements produce System 2 escalation records
    addressed toward System 3. At the Milestone 6 gate, typed System 3 handling
    was deferred to the next milestone.
- Milestone 7 introduced ADR-0006. Implementation notes:
  - `ResourceGovernance`, `OperationalControlPolicy`, and `Auditor` are
    public, object-safe, runtime-selectable roles over framework-owned records
    and the existing `ViableSystem` associated types.
  - System 3 control and System 3* audit run as separate private actors inside
    `VsmRuntime`; the public handle exposes no actor references, global names,
    or JSON payloads.
  - System 3* audit evidence is collected through a distinct System 1 audit
    path. Audit boundaries can remove snapshots and cap evidence count before
    the application auditor is invoked.
  - Former JSON resource and audit algorithms are retained only as
    `system3::defaults` helpers; the legacy global System 3 supervisor starts
    no JSON control service child.
  - Automatic routing from System 2 escalation records into System 3 governance
    remains deferred.

---

## Compatibility changes

Milestones 1 through 7 add public foundational APIs. Milestone 5 intentionally
changed legacy broker behavior by removing targeted-to-broadcast fallback and
validating explicit broadcast targets. Milestone 6 intentionally removes the
legacy System 2 JSON coordination service from the core path and replaces it
with typed runtime coordination. Milestone 7 intentionally removes the legacy
System 3 JSON control service from the core path and replaces it with typed
runtime governance/control and System 3* audit.

New public modules and re-exports:

- `vsm_rs::cancellation`
- `vsm_rs::protocol`
- `vsm_rs::roles`
- `vsm_rs::legacy`
- `vsm_rs::builder`
- `vsm_rs::config`
- `vsm_rs::runtime`
- `vsm_rs::{ApplicationFailure, FrameworkError, ViableSystem, WorkError}`
- `vsm_rs::{OperationalUnit, OperationalUnitFactory, WorkModel}`
- `vsm_rs::{UnitSelectionPolicy, PerformanceModel, VarietyModel}`
- `vsm_rs::{AlgedonicPolicy, System1Roles, RoleContext, UnitRoleContext}`
- `vsm_rs::{VsmBuilder, RuntimeConfig, VsmRuntime, System1Handle}`
- `vsm_rs::{RuntimeState, RuntimeReadiness, ReadinessCheck}`
- `vsm_rs::{ReadinessGate, ReadinessStatus, ShutdownReport}`
- `vsm_rs::{RuntimeDirectorySnapshot, RuntimeComponentSnapshot}`
- `vsm_rs::{RuntimeComponentStatus, RuntimePorts, System1RuntimeRoles}`
- `vsm_rs::System2RuntimeRoles`
- `vsm_rs::{UnitAdmissionLimits, UnitSnapshotConfig, UnitRegistration}`
- `vsm_rs::RegisteredUnit`
- `vsm_rs::{DeliveryMetrics, DeliveryStatus}`
- `vsm_rs::{RuntimeControlMessage, System1ControlMessage}`
- `vsm_rs::System2ControlMessage`
- `vsm_rs::{DeliveryOutcome, UndeliverableMessage}`
- `vsm_rs::{ObserverBusSnapshot, ObserverId, ObserverSubscription}`
- `vsm_rs::{CoordinationPolicy, System2Roles}`
- `vsm_rs::System2Handle`
- `vsm_rs::System3ControlMessage`
- `vsm_rs::{ResourceGovernance, OperationalControlPolicy, Auditor}`
- `vsm_rs::{System3Roles, System3RuntimeRoles, System3Handle}`
- `vsm_rs::async_trait`

New public channel/runtime APIs:

- `channels::publish_with_outcome`
- `channels::broadcast_with_outcome`
- `channels::dead_letters`
- `VsmRuntime::subscribe_events`
- `VsmRuntime::observer_event_history`
- `VsmRuntime::observer_bus_snapshot`
- `VsmBuilder::coordination_policy`
- `VsmBuilder::coordination_policy_arc`
- `VsmRuntime::system2`
- `System2Handle::coordinate_views`
- `System2Handle::coordinate_system1`
- `System2Handle::acknowledge_interventions`
- `System2Handle::snapshot`
- `OperationalUnit::handle_coordination_intervention`
- `VsmBuilder::resource_governance`
- `VsmBuilder::resource_governance_arc`
- `VsmBuilder::operational_control_policy`
- `VsmBuilder::operational_control_policy_arc`
- `VsmBuilder::auditor`
- `VsmBuilder::auditor_arc`
- `VsmRuntime::system3`
- `System3Handle::govern_resources`
- `System3Handle::handle_resource_shortage`
- `System3Handle::acknowledge_directives`
- `System3Handle::audit_system1`
- `System3Handle::audit_with_evidence`
- `System3Handle::snapshot`

Public behavior changed:

- legacy System 2 JSON service calls no longer dispatch to
  `system2::coordination`;
- the legacy global System 2 supervisor remains present but starts no JSON
  coordination child;
- `basic_usage` still runs, but the legacy coordination channel records
  `TargetUnavailable` for the removed System 2 target.
- legacy System 3 JSON service calls no longer dispatch to `system3::control`;
- the legacy global System 3 supervisor remains present but starts no JSON
  control child;
- legacy System 1 no-suitable-unit resource-bargain messages addressed to
  System 3 now record `TargetUnavailable`; typed shortage handling is available
  through `VsmRuntime::system3()`;
- former System 3 resource and audit helpers live under `system3::defaults` as
  opt-in examples.

Removed characterized bug behavior:

- missing targeted channel subscriber no longer falls back to broadcast;
- explicit channel broadcast no longer bypasses targeted-message validation.

---

## Known issues and risks

- `PORTING_MAP.md` is still absent; docs now state this fact.
- The crate is `publish = false` and lacks final publication metadata and a
  `rust-version`; publication hardening is deferred.
- Legacy actor-facade readiness still relies on sleeps; the typed `VsmRuntime`
  handle has deterministic readiness for the typed runtime path.
- Legacy actor names remain process-global; only one default actor-backed VSM
  runtime can safely run per process. Typed runtime handles are instance-scoped,
  and the typed System 1 path uses private actor adapters.
- State, metrics, channel history, dead-letter history, observer event history,
  and most service data remain in memory and restart-volatile.
- Systems 4-5 still use string operation names and `serde_json::Value`.
- The typed runtime path now processes System 1 work, System 2 coordination,
  and System 3 governance/audit through private actor adapters. Systems 4-5
  and the legacy `start()` facade still use the current actor/JSON runtime.
- The legacy global System 2 and System 3 supervisors no longer start JSON
  service children; callers using the old targeted coordination or
  resource-bargain channels receive `TargetUnavailable`.
- Automatic routing of System 2 escalation records into System 3 governance is
  still deferred; callers can invoke System 3 through the typed handle.
- Temporary `legacy` adapters intentionally bridge current JSON forms for
  round-trip tests only; they are not the target public application surface.
- First-wave role contracts, contexts, and runtime handles are wired into the
  typed System 1 path, but automatic unit restart/reconciliation is still
  deferred.
- Broker delivery outcomes report actor-mailbox delivery, not recipient domain
  processing acknowledgement.
- Broker restart still loses subscriptions, retained history, dead-letter
  history, and delivery metrics.
- System 1 Operations restart still loses its unit directory.
- System 1 unit supervisor restart can leave Operations with a stale supervisor
  reference.

Do not remove an issue merely because it is inconvenient. Remove it only when
resolved, and record the resolution in the development history.

---

## Deferred work

| Deferred item | Reason | Impact | Prerequisite | Intended milestone |
|---|---|---|---|---|
| System 1 restart/reconciliation | Automatic unit restart, Operations restart directory reconstruction, and unit-supervisor reconciliation are outside the first actor-backed typed slice. | Typed unit actors stop cleanly on unregister/shutdown, but crash recovery is not complete. | Typed System 1 registration/work path. | System 1 hardening |
| Durable `StateStore` implementations | Persistence contract is accepted, but durable adapters are outside Phase 0. | Current stores are in-memory or no-op only. | StateStore core contract and persistence milestone approval. | Persistence and recovery |
| Systems 4-5 typed role catalogs and migrations | Later subsystem semantics require separate review gates. | Systems 4-5 continue to use string/JSON service calls. | System 1/System 2/System 3 patterns and owning milestone approval. | Systems 4-5 migrations |
| Full event replay and durability | Requires event model and store semantics not approved for Phase 0. | Events and channel history remain non-durable. | Typed bus/event bus and persistence decisions. | Persistence and recovery |
| Automatic work retries | User chose no automatic work retries in first System 1 slice. | Work retry behavior remains caller/application responsibility. | Failure classification and retry policy review. | Backpressure/execution hardening |
| Richer defaults | Defaults must be opt-in and non-normative. | Initial defaults remain minimal. | Role contracts and default namespaces. | System 1 and later default milestones |
| Feature matrix | Too early before typed/public API shape is implemented. | Cargo currently has no feature matrix. | Adapter/default/publication decisions. | Publication hardening |
| Formal MSRV | User deferred until publication hardening. | Current validation uses the installed toolchain, not an MSRV matrix. | Public API stabilization and release preparation. | Publication hardening |
| Publication metadata | Crate remains unpublished by design. | `cargo publish --dry-run` is not part of Phase 0 completion. | Final crate metadata and release gate. | Publication hardening |

---

## Development history

#### 2026-06-18 — Phase 0 Baseline Stabilization

**Objective**

Execute Phase 0 only for the trait-driven VSM runtime migration: maintain the
execution journal, record accepted decisions in ADRs, fix formatting and Clippy
baseline failures without behavior redesign, correct factual docs, add focused
characterization tests, run validation, and stop for user review.

**Changes**

- Files changed:
  - `CODEX.md`
  - `README.md`
  - `docs/ARCHITECTURE.md`
  - `docs/DEVELOPERS.md`
  - `docs/USAGE.md`
  - `docs/adr/*`
  - `tests/phase0_characterization.rs`
  - `tests/full_system_flow.rs`
  - Rust source and example files reformatted by `cargo fmt --all`
- APIs added, removed, or modified: none.
- Tests added:
  - startup health before shutdown;
  - System 1 no-suitable-unit resource request;
  - missing targeted subscriber fallback to broadcast, labeled bug-to-remove;
  - explicit broadcast validation gap;
  - Systems 2-5 JSON service call responses.
- Documentation updated:
  - ADR process and accepted ADRs;
  - current publication/path/status notes;
  - moved docs links;
  - missing `PORTING_MAP.md` status.

**Decisions**

- Phase 0 scope and clean breaking posture recorded in
  [ADR-0001](docs/adr/0001-clean-breaking-migration-posture.md).
- Application type family and role contract posture recorded in
  [ADR-0002](docs/adr/0002-application-type-family-and-role-contracts.md).
- System 1 runtime semantics recorded in
  [ADR-0003](docs/adr/0003-system1-runtime-semantics.md).
- Protocol boundaries and deferred decisions recorded in
  [ADR-0004](docs/adr/0004-protocol-boundaries-and-deferred-decisions.md).

**Validation**

```text
cargo fmt --all -- --check
passed

cargo check --all-targets --all-features --locked
passed

cargo test --all-targets --all-features --locked
passed

cargo clippy --all-targets --all-features --locked -- -D warnings
passed

cargo doc --all-features --no-deps --locked
passed

cargo test --doc --all-features --locked
passed

cargo run --example basic_usage --locked
passed

git diff --check
passed
```

**Failures and warnings**

- Initial baseline before edits had failing formatting and Clippy, plus broker
  warnings during check/test/example. These are resolved.
- A first run of the new characterization test binary exposed assertion and
  cleanup issues in the test code. The tests were corrected and the final full
  validation suite passed.

**Next task**

Wait for explicit user approval to begin Milestone 1 foundational typed runtime
work. Do not begin it automatically.

#### 2026-06-18 — Milestone 1 Start

**Objective**

Begin the approved typed protocol foundations milestone after the user completed
the Phase 0 review gate.

**Changes**

- Updated this journal to record Milestone 1 approval and scope.
- No Rust source changes yet.

**Decisions**

- User explicitly approved proceeding after Phase 0 review.
- Existing ADR-0001 through ADR-0004 remain the active decision record.

**Validation**

Most recent validation remains the Phase 0 gate suite, all passing on
2026-06-18. Validation will be rerun after implementation.

**Next task**

Add foundational public modules and tests, then stop at the Milestone 1 review
gate.

#### 2026-06-18 — Milestone 1 Typed Protocol Foundations

**Objective**

Implement typed protocol foundations alongside the existing actor runtime,
without rewriting actors, adding the builder/runtime handle, or beginning System
1 adapter migration.

**Changes**

- Files changed:
  - `CODEX.md`
  - `README.md`
  - `docs/ARCHITECTURE.md`
  - `docs/USAGE.md`
  - `src/error.rs`
  - `src/lib.rs`
  - `src/cancellation.rs`
  - `src/legacy/*`
  - `src/protocol/*`
  - `src/roles/*`
  - `tests/foundational_types.rs`
- Public APIs added:
  - `ViableSystem`;
  - `FrameworkError`, `ApplicationFailure`, and `WorkError`;
  - cooperative `CancellationToken`;
  - protocol address, metadata, snapshot, event, report, and System 1 record
    types;
  - `StateStore`, `NoopStateStore`, `EventSink`, `NoopEventSink`,
    `ReportSink`, and `NoopReportSink`;
  - temporary `legacy` adapters for current `Transaction`, `TransactionResult`,
    `UnitConfig`, and `VsmMessage` shapes;
  - crate-root re-export of `async_trait`.
- Public APIs removed or renamed: none.
- Tests added:
  - downstream-style `ViableSystem` implementation with non-serde work,
    outcome, and snapshot payloads;
  - capacity snapshot admission state;
  - instance-scoped metadata and causation;
  - cancellation-to-work-error mapping;
  - no-op state/event/report ports;
  - legacy transaction, transaction-result, unit-config, and resource-shortage
    round-trips;
  - application/framework work error separation.
- Documentation updated:
  - README feature summary for typed foundations;
  - architecture module map and foundation boundary;
  - usage notes explaining that foundations are not yet wired into the actor
    facade.

**Decisions**

- No new ADR-level decisions were made.
- `UnitSnapshot` is `Send + 'static` at the core type-family boundary because
  snapshot storage is an async runtime port. No serde, clone, or debug bound was
  added to application snapshots.
- Event/report enum variants that can contain application payloads are boxed to
  keep public enum sizes reasonable and satisfy Clippy.

**Validation**

```text
cargo fmt --all -- --check
passed

cargo check --all-targets --all-features --locked
passed

cargo test --test foundational_types --all-features --locked
passed

cargo test --all-targets --all-features --locked
passed

cargo clippy --all-targets --all-features --locked -- -D warnings
passed

cargo doc --all-features --no-deps --locked
passed

cargo test --doc --all-features --locked
passed

cargo run --example basic_usage --locked
passed

git diff --check
passed
```

**Failures and warnings**

- One `cargo check --all-targets --all-features --locked` run hit a nightly
  incremental compiler ICE in rustc 1.98.0-nightly. `cargo clean -p vsm-rs`
  cleared the stale incremental state, and the same command passed afterward.
- Initial Milestone 1 compile iterations exposed expected type-bound and Clippy
  issues while the new public generics settled. The final validation suite
  passes.
- The new typed foundations are intentionally not wired into the running actor
  facade yet.

**Next task**

Wait for explicit user approval to begin Milestone 2: first-wave role contracts
and role contexts. Do not begin it automatically.

#### 2026-06-18 — Milestone 2 Start

**Objective**

Begin the approved role contracts and role contexts milestone after the user
completed the Milestone 1 review gate.

**Changes**

- Updated this journal to record Milestone 2 approval and scope.
- No Milestone 2 Rust source changes yet.

**Decisions**

- User explicitly approved proceeding after the Milestone 1 review gate.
- Existing ADR-0001 through ADR-0004 remain the active decision record.
- No new dependency, compatibility, persistence, or restart guarantee decisions
  have been made.

**Validation**

Most recent validation remains the Milestone 1 gate suite, all passing on
2026-06-18. The repository was clean at baseline commit `4c2fa54` before
Milestone 2 edits. Validation will be rerun after implementation.

**Next task**

Add role contracts, role contexts, supporting ports, no-op/test
implementations, downstream-style tests, and docs, then stop at the Milestone 2
review gate.

#### 2026-06-18 — Milestone 2 Role Contracts and Contexts

**Objective**

Implement the first-wave role contracts and role contexts before changing actor
implementations.

**Changes**

- Files changed:
  - `CODEX.md`
  - `README.md`
  - `docs/ARCHITECTURE.md`
  - `docs/DEVELOPERS.md`
  - `docs/USAGE.md`
  - `src/lib.rs`
  - `src/protocol/system1.rs`
  - `src/roles/mod.rs`
  - `src/roles/ports.rs`
  - `src/roles/context.rs`
  - `src/roles/system1.rs`
  - `tests/role_contracts.rs`
- Public APIs added:
  - `RoleContext` and `UnitRoleContext`;
  - `OperationalUnit`, `OperationalUnitFactory`, `WorkModel`,
    `UnitSelectionPolicy`, `PerformanceModel`, `VarietyModel`,
    `AlgedonicPolicy`, and `System1Roles`;
  - role object aliases for boxed/shared dynamic dispatch;
  - `WorkMeasurement`, `UnitCandidate`, `PerformanceAssessment`,
    `VarietyAssessment`, and algedonic signal support types;
  - opt-in `roles::system1::defaults` policies;
  - `roles::system1::testing` fakes;
  - `TelemetrySink`, `AlertSink`, `Clock`, `IdGenerator`, and no-op/system/UUID
    implementations.
- Public APIs removed or renamed: none.
- Tests added:
  - downstream-style implementation of every first-wave role;
  - dyn-compatible role object construction;
  - opt-in default/no-op policy behavior;
  - static unit factory and accept-all work model fakes;
  - role context identity, cancellation, and sink access;
  - explicit unsupported snapshot failure;
  - unit command, coordination view, audit evidence, and static capability
    checks.
- Documentation updated:
  - README feature summary;
  - architecture foundation module boundary;
  - usage guide role-contract section;
  - developer guide layout.

**Decisions**

- No new ADR-level decisions were made.
- Unit role methods use `&mut self` consistently. This preserves the accepted
  mutable-unit posture and avoids requiring application unit state to implement
  `Sync` only because of async trait futures.
- Policy/model/factory roles use `&self` and are `Send + Sync` because runtime
  adapters are expected to share them.
- Methods that inspect application work/outcome move typed protocol values
  rather than borrowing app payloads across async futures. This preserves the
  accepted `Work` and `Outcome` bounds without adding `Sync`.

**Validation**

```text
cargo fmt --all -- --check
passed

cargo check --all-targets --all-features --locked
passed

cargo test --test role_contracts --all-features --locked
passed

cargo test --all-targets --all-features --locked
passed

cargo clippy --all-targets --all-features --locked -- -D warnings
passed

cargo doc --all-features --no-deps --locked
passed

cargo test --doc --all-features --locked
passed

cargo run --example basic_usage --locked
passed

git diff --check
passed
```

**Failures and warnings**

- Initial compile iterations exposed over-broad derive bounds on generic
  protocol/context types and accidental `Sync` pressure from async methods that
  borrowed application payloads. The final implementation preserves the
  accepted bounds and validation passes.
- The new role contracts are intentionally not wired into the actor runtime yet.

**Next task**

Wait for explicit user approval to begin Milestone 3: builder, runtime handle,
readiness/lifecycle surface, and instance scope. Do not begin it automatically.

#### 2026-06-18 — Milestone 3 Start

**Objective**

Begin the approved builder, runtime handle, readiness/lifecycle, and instance
scope milestone after the user completed the Milestone 2 review gate.

**Changes**

- Updated this journal to record Milestone 3 approval and scope.
- No Milestone 3 Rust source changes yet.

**Decisions**

- User explicitly approved proceeding after the Milestone 2 review gate.
- Existing ADR-0001 through ADR-0004 remain the active decision record.
- No new dependency, compatibility, persistence, or restart guarantee decisions
  have been made.

**Validation**

Most recent validation remains the Milestone 2 gate suite, all passing on
2026-06-18. The repository was clean at baseline commit `dcfc9f9` before
Milestone 3 edits. Validation will be rerun after implementation.

**Next task**

Add the typed builder, runtime configuration, runtime handle, readiness records,
private runtime directory scaffold, lifecycle tests, and docs, then stop at the
Milestone 3 review gate.

#### 2026-06-18 — Milestone 3 Runtime Builder and Handles

**Objective**

Implement the typed runtime lifecycle surface before wiring role contracts into
actor adapters.

**Changes**

- Files changed:
  - `CODEX.md`
  - `README.md`
  - `docs/ARCHITECTURE.md`
  - `docs/DEVELOPERS.md`
  - `docs/USAGE.md`
  - `src/lib.rs`
  - `src/builder.rs`
  - `src/config.rs`
  - `src/runtime.rs`
  - `src/kernel/mod.rs`
  - `src/kernel/registry.rs`
  - `tests/runtime_builder.rs`
  - `examples/typed_runtime_builder.rs`
- Public APIs added:
  - `VsmBuilder`;
  - `RuntimeConfig`;
  - `VsmRuntime` and `System1Handle`;
  - `RuntimePorts` and `System1RuntimeRoles`;
  - `RuntimeState`, `RuntimeReadiness`, `ReadinessCheck`,
    `ReadinessGate`, and `ReadinessStatus`;
  - `ShutdownReport`;
  - `RuntimeDirectorySnapshot`, `RuntimeComponentSnapshot`, and
    `RuntimeComponentStatus`.
- Public APIs removed or renamed: none.
- Tests added:
  - builder rejects missing `WorkModel`;
  - builder rejects missing `OperationalUnitFactory`;
  - builder starts with default optional policies;
  - default lowest-load selector works through the runtime handle;
  - runtime handles are instance-scoped and can coexist;
  - role contexts use runtime identity and recursion path;
  - shutdown acknowledgement is idempotent and updates the directory snapshot.
- Documentation updated:
  - README feature summary and typed-builder example command;
  - architecture crate layout and typed lifecycle-shell boundary;
  - usage guide typed builder section;
  - developer guide layout and runtime-module boundary.

**Decisions**

- No new ADR-level decisions were made.
- The builder validates required roles at runtime and returns
  `FrameworkError::InvalidProtocol` for missing required roles.
- `VsmBuilder::start()` is async now so actor-backed startup can be added in
  the next slice without changing the lifecycle entry point again.
- Readiness includes `NotApplicable` so this lifecycle shell can report
  deterministic readiness while actor adapters and typed observer bus work are
  still explicitly outside scope.

**Validation**

```text
cargo fmt --all -- --check
passed

cargo check --all-targets --all-features --locked
passed

cargo test --test runtime_builder --all-features --locked
passed

cargo test --all-targets --all-features --locked
passed

cargo clippy --all-targets --all-features --locked -- -D warnings
passed

cargo doc --all-features --no-deps --locked
passed

cargo test --doc --all-features --locked
passed

cargo run --example typed_runtime_builder --locked
passed

cargo run --example basic_usage --locked
passed

git diff --check
passed
```

**Failures and warnings**

- The first `cargo clippy` run found one redundant borrow in an internal
  `format!` argument. It was fixed, and the rerun passed with `-D warnings`.
- The typed runtime handle is intentionally a lifecycle shell in this milestone:
  it validates roles, exposes readiness and scoped contexts, supports multiple
  runtime handles, and acknowledges shutdown. It does not process work through
  actor adapters yet.

**Next task**

Wait for explicit user approval to begin Milestone 4: convert System 1 as the
first complete vertical slice by wiring role contracts into supervised actor
adapters. Do not begin it automatically.

#### 2026-06-18 — Milestone 4 Start

**Objective**

Begin the approved System 1 vertical slice after the user completed the
Milestone 3 review gate.

**Changes**

- Updated this journal to record Milestone 4 approval, scope, baseline, and
  next task.
- No Milestone 4 Rust source changes yet.

**Decisions**

- User explicitly approved proceeding after the Milestone 3 review gate.
- Existing ADR-0001 through ADR-0004 remain the active decision record.
- No new dependency, compatibility, persistence, typed-bus, or Systems 2-5
  migration decisions have been made.

**Validation**

Most recent validation remains the Milestone 3 gate suite, all passing on
2026-06-18. The repository was clean at baseline commit `3024b69` before
Milestone 4 edits. Validation will be rerun after implementation.

**Next task**

Map existing System 1 actors and typed role contracts, implement the smallest
complete actor-backed typed System 1 runtime path, add tests and docs, then
stop at the Milestone 4 review gate.

#### 2026-06-18 — Milestone 4 Typed System 1 Vertical Slice

**Objective**

Connect the first-wave System 1 role contracts to an actor-backed typed runtime
path without beginning the typed bus or Systems 2-5 migrations.

**Changes**

- Files changed:
  - `CODEX.md`
  - `README.md`
  - `docs/ARCHITECTURE.md`
  - `docs/DEVELOPERS.md`
  - `docs/USAGE.md`
  - `examples/typed_runtime_builder.rs`
  - `src/builder.rs`
  - `src/kernel/mod.rs`
  - `src/kernel/system1.rs`
  - `src/lib.rs`
  - `src/runtime.rs`
  - `tests/runtime_builder.rs`
  - `tests/system1_typed_runtime.rs`
- Public APIs added:
  - `UnitAdmissionLimits`;
  - `UnitSnapshotConfig`;
  - `UnitRegistration`;
  - `RegisteredUnit`;
  - `System1Handle::register_unit`;
  - `System1Handle::register_descriptor`;
  - `System1Handle::list_units`;
  - `System1Handle::process_work`;
  - `System1Handle::process`;
  - `System1Handle::process_response`;
  - `System1Handle::drain_unit`;
  - `System1Handle::unregister_unit`.
- Public APIs removed or renamed: none.
- Tests added:
  - typed System 1 unit returns a domain outcome;
  - work-model validation rejects before unit dispatch;
  - custom selector changes routing;
  - no suitable unit emits a typed resource-shortage event;
  - admission limit returns `Backpressured`;
  - expired deadline returns framework `Timeout`;
  - drain and unregister update lifecycle state;
  - snapshot restore and save use `StateStore`.
- Documentation updated:
  - README feature summary and typed builder behavior;
  - architecture typed foundation and `kernel::system1` boundary;
  - usage guide builder example with registration and typed processing;
  - developer guide private adapter boundary.

**Decisions**

- No new ADR-level decisions were made.
- The typed System 1 path is private-actor-backed but not yet a full supervised
  restart/reconciliation implementation. Automatic restarts remain deferred to
  System 1 hardening.
- Legacy `system1::*` transaction APIs and the global actor facade remain
  unchanged and characterized.
- Event/report sink failures are ignored for the work control path in this
  slice, matching the non-blocking observer posture.

**Validation**

```text
cargo fmt --all -- --check
passed

cargo check --all-targets --all-features --locked
passed

cargo test --test system1_typed_runtime --all-features --locked
passed

cargo test --all-targets --all-features --locked
passed

cargo clippy --all-targets --all-features --locked -- -D warnings
passed

cargo doc --all-features --no-deps --locked
passed

cargo test --doc --all-features --locked
passed

cargo run --example typed_runtime_builder --locked
passed

cargo run --example basic_usage --locked
passed

git diff --check
passed
```

**Failures and warnings**

- An exploratory `cargo metadata --format-version 1 --locked` attempted to
  download uncached registry artifacts and failed because network access is not
  available. It was not part of gate validation.
- The first compile after adding the adapter hit the known nightly incremental
  compiler ICE. `cargo clean -p vsm-rs` cleared local incremental state and the
  subsequent compile passed.
- The first Clippy run flagged a large private actor-message variant. Boxing the
  typed work request fixed it; the rerun passed with `-D warnings`.
- Automatic unit crash restart, Operations restart directory reconstruction,
  and unit-supervisor reconciliation remain unresolved.

**Next task**

Wait for explicit user approval to begin Milestone 5: typed protocol bus and
observer event bus. Do not begin it automatically.

#### 2026-06-18 — Milestone 5 Start

**Objective**

Begin the approved typed protocol bus and observer event bus milestone after the
user completed the Milestone 4 review gate.

**Changes**

- Updated this journal to record Milestone 5 approval, scope, baseline, and
  next task.
- No Milestone 5 Rust source changes yet.

**Decisions**

- User explicitly approved proceeding after the Milestone 4 review gate.
- Existing ADR-0001 through ADR-0004 remain the active decision record.
- No new dependency, persistence, restart guarantee, or Systems 2-5 semantic
  migration decisions have been made.

**Validation**

Most recent validation remains the Milestone 4 gate suite, all passing on
2026-06-18. The repository was clean at baseline commit `a5f3663` before
Milestone 5 edits. Validation will be rerun after implementation.

**Next task**

Map the existing broker and runtime event ports, implement explicit delivery
outcomes and typed observer subscriptions, add tests and docs, then stop at the
Milestone 5 review gate.

#### 2026-06-18 — Milestone 5 Typed Protocol Bus and Observer Event Bus

**Objective**

Replace hidden broadcast fallback with explicit delivery outcomes, add typed
observer-event subscriptions for runtime handles, and keep Systems 2-5 semantic
migrations deferred.

**Changes**

- Files changed:
  - `CODEX.md`
  - `README.md`
  - `docs/ARCHITECTURE.md`
  - `docs/DEVELOPERS.md`
  - `docs/USAGE.md`
  - `src/channels/*_channel.rs`
  - `src/channels/broker.rs`
  - `src/channels/mod.rs`
  - `src/domain.rs`
  - `src/kernel/event_bus.rs`
  - `src/kernel/mod.rs`
  - `src/lib.rs`
  - `src/protocol/bus.rs`
  - `src/protocol/events.rs`
  - `src/protocol/mod.rs`
  - `src/runtime.rs`
  - `src/shared/message.rs`
  - `tests/foundational_types.rs`
  - `tests/phase0_characterization.rs`
  - `tests/runtime_builder.rs`
  - `tests/system1_typed_runtime.rs`
- Public APIs added:
  - `DeliveryStatus`;
  - `DeliveryMetrics`;
  - `RuntimeControlMessage`;
  - `System1ControlMessage`;
  - `DeliveryOutcome`;
  - `UndeliverableMessage`;
  - `ObserverId`;
  - `ObserverSubscription`;
  - `ObserverBusSnapshot`;
  - `channels::publish_with_outcome`;
  - `channels::broadcast_with_outcome`;
  - `channels::dead_letters`;
  - `VsmRuntime::subscribe_events`;
  - `VsmRuntime::observer_event_history`;
  - `VsmRuntime::observer_bus_snapshot`.
- Public behavior changed:
  - targeted broker delivery no longer falls back to broadcast when the target
    subscriber is missing;
  - explicit broadcast validates that the message is addressed to
    `SystemId::All`;
  - `ChannelStats` now includes delivery metrics and dead-letter counts.
- Tests added or updated:
  - typed control bus records work with non-serde application payloads;
  - targeted publish reports a delivered outcome;
  - missing targeted subscriber returns `TargetUnavailable`, records a dead
    letter, and does not broadcast to observers;
  - explicit broadcast rejects targeted messages and records a dead letter;
  - typed runtime observer subscriptions receive `RuntimeEvent` values;
  - downstream event sink failures are counted without blocking observer
    delivery.
- Documentation updated:
  - README feature/channel summary;
  - architecture channel broker and typed runtime module boundaries;
  - usage guide outcome/dead-letter and observer examples;
  - developer guide channel-extension rules.

**Decisions**

- No new ADR-level decisions were made.
- Existing ADR-0004 required removing targeted-to-broadcast fallback in this
  milestone; the implementation resolves that characterized bug.
- Broker outcomes acknowledge actor mailbox delivery only. Recipient processing
  acknowledgements, retry, durable replay, and typed Systems 2-5 semantics
  remain deferred.

**Validation**

```text
cargo fmt --all -- --check
passed

cargo check --all-targets --all-features --locked
passed

cargo test --test foundational_types --all-features --locked
passed

cargo test --test phase0_characterization --all-features --locked
passed

cargo test --test runtime_builder --all-features --locked
passed

cargo test --test system1_typed_runtime --all-features --locked
passed

cargo test --all-targets --all-features --locked
passed

cargo clippy --all-targets --all-features --locked -- -D warnings
passed

cargo doc --all-features --no-deps --locked
passed

cargo test --doc --all-features --locked
passed

cargo run --example typed_runtime_builder --locked
passed

cargo run --example basic_usage --locked
passed

git diff --check
passed
```

**Failures and warnings**

- Initial compile after adding the observer event bus exposed an overly broad
  derived `Clone` bound on `RuntimeEvent<V>`. Manual clone implementations now
  preserve the `ViableSystem` bounds.
- Initial typed observer test matching moved a boxed event out of a pattern
  guard. The assertion now borrows the event.
- Full event durability, broker restart subscription recovery, recipient
  processing acknowledgements, and Systems 2-5 typed migrations remain
  unresolved.

**Next task**

Wait for explicit user approval to begin Milestone 6: System 2 migration. Do
not begin it automatically.

#### 2026-06-18 — Milestone 6 System 2 Typed Coordination

**Objective**

Convert System 2 from a JSON `ServiceActor` core path into typed coordination
runtime machinery, using the approved minimal view-centric
`CoordinationPolicy` shape from ADR-0005.

**Changes**

- Files changed:
  - `CODEX.md`
  - `README.md`
  - `docs/ARCHITECTURE.md`
  - `docs/DEVELOPERS.md`
  - `docs/USAGE.md`
  - `docs/adr/README.md`
  - `docs/adr/0005-system2-coordination-policy.md`
  - `src/actor_support.rs`
  - `src/builder.rs`
  - `src/channels/mod.rs`
  - `src/kernel/mod.rs`
  - `src/kernel/system1.rs`
  - `src/kernel/system2.rs`
  - `src/lib.rs`
  - `src/protocol/bus.rs`
  - `src/protocol/events.rs`
  - `src/protocol/mod.rs`
  - `src/protocol/system1.rs`
  - `src/protocol/system2.rs`
  - `src/roles/mod.rs`
  - `src/roles/system1.rs`
  - `src/roles/system2.rs`
  - `src/runtime.rs`
  - `src/system2/defaults/*`
  - `src/system2/mod.rs`
  - `src/system2/supervisor.rs`
  - `src/vsm_core.rs`
  - `tests/foundational_types.rs`
  - `tests/full_system_flow.rs`
  - `tests/phase0_characterization.rs`
  - `tests/system2_typed_runtime.rs`
- Files removed or moved:
  - `src/system2/coordination.rs` removed;
  - `src/system2/scheduler.rs` moved to `src/system2/defaults/scheduler.rs`;
  - `src/system2/balancer.rs` moved to `src/system2/defaults/balancer.rs`.
- Public APIs added:
  - `CoordinationPolicy`;
  - `System2Roles`;
  - `System2RuntimeRoles`;
  - `System2Handle`;
  - `System2ControlMessage`;
  - typed System 2 protocol records for coordination views, conflicts,
    interventions, acknowledgements, escalations, cycles, and snapshots;
  - `VsmBuilder::coordination_policy`;
  - `VsmBuilder::coordination_policy_arc`;
  - `VsmRuntime::system2`;
  - `System2Handle::coordinate_views`;
  - `System2Handle::coordinate_system1`;
  - `System2Handle::acknowledge_interventions`;
  - `System2Handle::snapshot`;
  - `OperationalUnit::handle_coordination_intervention`.
- Public behavior changed:
  - System 2 no longer has JSON string-operation dispatch in the core path;
  - the legacy System 2 supervisor starts no JSON coordination child;
  - legacy targeted coordination-channel calls to System 2 now report
    `TargetUnavailable`;
  - scheduler and balancer behavior are labeled as defaults rather than core
    System 2 semantics.
- Tests added or updated:
  - downstream-style typed System 2 policy and unit implementations compile
    without actor or JSON APIs;
  - System 2 detects a conflict, delivers typed interventions to System 1
    units, and records acknowledgements;
  - rejected intervention acknowledgements are escalated toward System 3;
  - coordination view versions advance on repeated observation;
  - the default System 2 policy is no-op and replaceable;
  - At the Milestone 6 gate, Phase 0 characterization recorded that System 2
    JSON dispatch had been removed while the later-subsystem JSON service
    calls remained characterized.
- Documentation updated:
  - README public API and migration status;
  - architecture module boundaries and runtime tree;
  - usage examples for typed System 2 coordination;
  - developer guide extension rules;
  - ADR index.

**Decisions**

- ADR-0005 records the accepted Option A decision: public, minimal,
  view-centric `CoordinationPolicy` over typed System 1 coordination views,
  without new required `ViableSystem` associated types.
- Authoritative resource allocation remains outside System 2 and deferred to
  System 3.
- Typed System 3 handling of escalation records was deferred to Milestone 7 at
  the Milestone 6 gate.

**Validation**

```text
cargo fmt --all -- --check
passed

CARGO_INCREMENTAL=0 cargo check --all-targets --all-features --locked
passed

CARGO_INCREMENTAL=0 cargo test --test system2_typed_runtime --all-features --locked
passed

CARGO_INCREMENTAL=0 cargo test --test phase0_characterization --test full_system_flow --all-features --locked
passed

CARGO_INCREMENTAL=0 cargo clippy --all-targets --all-features --locked -- -D warnings
passed

CARGO_INCREMENTAL=0 cargo test --all-targets --all-features --locked
passed

CARGO_INCREMENTAL=0 cargo doc --all-features --no-deps --locked
passed

CARGO_INCREMENTAL=0 cargo test --doc --all-features --locked
passed

CARGO_INCREMENTAL=0 cargo run --example typed_runtime_builder --locked
passed

CARGO_INCREMENTAL=0 cargo run --example basic_usage --locked
passed

git diff --check
passed
```

**Failures and warnings**

- The first full `cargo check --all-targets --all-features --locked` run hit an
  installed nightly `rustc` incremental compilation ICE. The generated ICE
  artifacts were removed, and the full validation suite passed with
  `CARGO_INCREMENTAL=0`.
- Clippy flagged large enum variants after adding System 2 protocol payloads.
  Runtime control, event, and report enum variants now box large System 1 and
  System 2 payloads.
- Automatic consumption of System 2 escalation records by System 3, durable
  coordination history, automatic coordination retries, and richer System 2
  defaults remained unresolved at the Milestone 6 gate.

**Next task**

Wait for explicit user approval to begin Milestone 7: System 3 and System 3*
migration. Do not begin it automatically.

#### 2026-06-18 — Milestone 7 Start

**Objective**

Begin the System 3 and System 3* migration after the user completed the
Milestone 6 review gate.

**Changes**

- Updated this journal to record Milestone 7 approval and scope.
- Added proposed ADR-0006 for the public System 3/System 3* role boundary.
- No Milestone 7 Rust public API or runtime changes have begun.

**Decisions**

- User explicitly approved proceeding after the Milestone 6 review gate.
- S3-001 was pending because the System 3 role boundary is a material public API
  decision.

**Validation**

Most recent validation remains the Milestone 6 gate suite, all passing on
2026-06-18. Validation will be rerun after implementation.

**Next task**

Wait for the user to choose S3-001. If Option A is approved, accept ADR-0006,
implement the typed System 3/System 3* slice, and stop at the Milestone 7
review gate.

#### 2026-06-18 — Milestone 7 Typed System 3 Control and Audit

**Objective**

Convert System 3 from a JSON `ServiceActor` core path into typed runtime
governance/control machinery, and split System 3* audit into a distinct typed
actor path, using the approved minimal role boundary from ADR-0006.

**Changes**

- Files changed:
  - `CODEX.md`
  - `README.md`
  - `docs/ARCHITECTURE.md`
  - `docs/DEVELOPERS.md`
  - `docs/USAGE.md`
  - `docs/adr/README.md`
  - `docs/adr/0006-system3-role-boundary.md`
  - `src/actor_support.rs`
  - `src/builder.rs`
  - `src/channels/mod.rs`
  - `src/kernel/mod.rs`
  - `src/kernel/system1.rs`
  - `src/kernel/system3.rs`
  - `src/lib.rs`
  - `src/protocol/bus.rs`
  - `src/protocol/events.rs`
  - `src/protocol/mod.rs`
  - `src/protocol/system1.rs`
  - `src/protocol/system3.rs`
  - `src/roles/mod.rs`
  - `src/roles/system3.rs`
  - `src/runtime.rs`
  - `src/system3/defaults/*`
  - `src/system3/mod.rs`
  - `src/system3/supervisor.rs`
  - `src/vsm_core.rs`
  - `tests/full_system_flow.rs`
  - `tests/phase0_characterization.rs`
  - `tests/system3_typed_runtime.rs`
- Files removed or moved:
  - `src/system3/control.rs` removed;
  - `src/system3/resources.rs` moved to
    `src/system3/defaults/resources.rs`;
  - `src/system3/audit.rs` moved to `src/system3/defaults/audit.rs`.
- Public APIs added:
  - `ResourceGovernance`;
  - `OperationalControlPolicy`;
  - `Auditor`;
  - `System3Roles`;
  - `System3RuntimeRoles`;
  - `System3Handle`;
  - `System3ControlMessage`;
  - typed System 3 protocol records for resource requests, allocation
    decisions, allocations, allocation acknowledgements, operational
    directives, directive acknowledgements, operational summaries, audit
    requests, audit boundaries, findings, remediations, audit responses, and
    snapshots;
  - `VsmBuilder::resource_governance`;
  - `VsmBuilder::resource_governance_arc`;
  - `VsmBuilder::operational_control_policy`;
  - `VsmBuilder::operational_control_policy_arc`;
  - `VsmBuilder::auditor`;
  - `VsmBuilder::auditor_arc`;
  - `VsmRuntime::system3`;
  - `System3Handle::govern_resources`;
  - `System3Handle::handle_resource_shortage`;
  - `System3Handle::acknowledge_directives`;
  - `System3Handle::audit_system1`;
  - `System3Handle::audit_with_evidence`;
  - `System3Handle::snapshot`.
- Public behavior changed:
  - System 3 no longer has JSON string-operation dispatch in the core path;
  - the legacy System 3 supervisor starts no JSON control child;
  - legacy targeted resource-bargain messages to System 3 now report
    `TargetUnavailable`;
  - resource and audit helper algorithms are labeled as defaults rather than
    core System 3 semantics.
- Tests added or updated:
  - downstream-style System 3 governance/control and System 3* audit role
    implementations compile without actor or JSON APIs;
  - resource shortage handling produces typed allocations and allocation
    acknowledgements;
  - operational directives are delivered to System 1 units and failed
    acknowledgements are recorded;
  - System 3* audit uses an authorized audit request and independent System 1
    evidence collection;
  - unauthorized audit requests fail before auditor invocation;
  - default System 3 roles deny resource requests explicitly and no-op audit;
  - Phase 0 characterization now records that System 2 and System 3 JSON
    dispatch have been removed while Systems 4-5 JSON service calls remain.
- Documentation updated:
  - README feature summary and System 3 usage notes;
  - architecture module boundaries, supervision tree, System 3 section, and
    current limitations;
  - usage examples for typed System 3 governance and System 3* audit;
  - developer guide layout and typed-runtime boundary;
  - ADR index.

**Decisions**

- ADR-0006 records the accepted Option A decision: minimal framework-owned
  System 3 records with `ResourceGovernance`, `OperationalControlPolicy`, and
  `Auditor` roles over the existing `ViableSystem` associated types.
- System 3* audit is a separate private actor path from System 3 control.
- Former resource/audit algorithms are opt-in defaults/examples, not core
  semantics.
- Automatic routing from System 2 escalation records into System 3 governance
  remains deferred.

**Validation**

```text
cargo fmt --all -- --check
passed

CARGO_INCREMENTAL=0 cargo check --all-targets --all-features --locked
passed

CARGO_INCREMENTAL=0 cargo test --all-targets --all-features --locked
passed

CARGO_INCREMENTAL=0 cargo clippy --all-targets --all-features --locked -- -D warnings
passed

CARGO_INCREMENTAL=0 cargo doc --all-features --no-deps --locked
passed

CARGO_INCREMENTAL=0 cargo test --doc --all-features --locked
passed

CARGO_INCREMENTAL=0 cargo run --example typed_runtime_builder --locked
passed

CARGO_INCREMENTAL=0 cargo run --example basic_usage --locked
passed

git diff --check
passed
```

**Failures and warnings**

- Clippy flagged a complex private System 3* audit snapshot reply type. The
  reply result is now factored through a private type alias in
  `kernel::system3`.
- The legacy no-suitable-unit path still emits a resource-bargain channel
  message to System 3, but the legacy System 3 subscriber is gone. The broker
  records this as `TargetUnavailable`; typed shortage handling is available
  through `VsmRuntime::system3()`.
- System 2 escalation records are typed and addressed toward System 3, but
  automatic escalation consumption remains deferred.

**Next task**

Wait for explicit user approval to begin Milestone 8: System 4 migration. Do
not begin it automatically.
