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

- Approved milestone: Milestone 5 — typed protocol bus and observer event bus
- Approved scope: Replace the current hidden targeted-to-broadcast delivery
  behavior with explicit typed delivery outcomes, add an observer event stream
  for typed runtime handles, record undeliverable/dead-letter events, update
  documentation and tests, and keep Systems 2-5 semantic migrations deferred.
- Approved architectural decisions: Recorded in ADR-0001 through ADR-0004
- Pending decisions: None for the approved Milestone 5 scope
- Permission to begin next milestone: No

## Pending user decisions

| ID | Decision | Options | Recommendation | Blocking milestone | Status |
|---|---|---|---|---|---|
| — | — | — | — | — | — |

## Current status

- Overall state: Milestone 5 complete; stopped at review gate
- Current phase: Milestone 5 — typed protocol bus and observer event bus
- Current milestone: Explicit delivery outcomes and typed observer stream
- Last updated: 2026-06-18
- Last updated by: Codex
- Baseline commit: `a5f3663`
- Working branch: `master`
- Repository clean at start: Yes
- Repository status now: Contains uncommitted Milestone 5 changes for review.

## Current objective

Milestone 5 implementation is complete for review: the broker now records
explicit delivery outcomes and dead letters instead of falling back from a
missing target to broadcast, explicit broadcast is validated as `SystemId::All`,
the typed runtime handle owns an observer event bus, and typed bus delivery
status/control-message records are public without requiring JSON app payloads.

## Next action

Wait for explicit user review/approval before beginning Milestone 6: System 2
migration. Do not begin it automatically.

---

## Milestone status

| Phase | Milestone | Status | Evidence |
|---|---|---:|---|
| 0 | Repository baseline | Complete | Formatting, check, tests, Clippy, docs, doctests, and example validation pass. |
| 0 | Characterization tests | Complete | `tests/phase0_characterization.rs` covers startup/health, System 1 no-unit resource request, explicit delivery outcomes, removed targeted fallback, broadcast validation, and Systems 2-5 JSON service calls. Existing System 1 and full-system tests still pass. |
| 0 | ADR setup | Complete | `docs/adr/README.md`, template, and ADR-0001 through ADR-0004 added. |
| 1 | Application type family | Complete | `src/roles/types.rs` defines `ViableSystem`; `tests/foundational_types.rs` proves non-serde application work, outcome, and snapshot payloads compile. |
| 1 | Typed core envelopes | Complete | `src/protocol/*`, `src/error.rs`, `src/cancellation.rs`, `src/roles/ports.rs`, and `src/legacy/*` added with tests, docs, and full validation passing. |
| 2 | Role contracts and factories | Complete | `src/roles/context.rs`, `src/roles/system1.rs`, expanded `src/roles/ports.rs`, and `tests/role_contracts.rs` added; full validation passes. |
| 2 | Runtime builder and handles | Complete | `src/builder.rs`, `src/config.rs`, `src/runtime.rs`, private `src/kernel/registry.rs`, `tests/runtime_builder.rs`, and `examples/typed_runtime_builder.rs`; full validation passes. |
| 3 | System 1 vertical slice | Complete | `src/kernel/system1.rs`, expanded `src/runtime.rs`, `tests/system1_typed_runtime.rs`, and `examples/typed_runtime_builder.rs`; full validation passes. |
| 4 | Typed protocol bus | Complete | `src/protocol/bus.rs`, `src/kernel/event_bus.rs`, expanded `src/channels/broker.rs`, runtime observer APIs, tests, docs, and full validation pass. |
| 5 | System 2 migration | Not started | Awaiting user approval. |
| 6 | System 3 and System 3* migration | Not started | Awaiting user approval. |
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
| `cargo fmt --all -- --check` | Passed | 2026-06-18 | Formatting drift resolved by `cargo fmt --all`. |
| `cargo check --all-targets --all-features --locked` | Passed | 2026-06-18 | No warnings. |
| `cargo test --all-targets --all-features --locked` | Passed | 2026-06-18 | 45 integration tests across foundational, role-contract, runtime-builder, typed-System-1, Phase 0, full-system, and legacy System 1 suites; example test targets have 0 tests. |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed | 2026-06-18 | No warnings. |
| `cargo doc --all-features --no-deps --locked` | Passed | 2026-06-18 | Generated `target/doc/vsm_rs/index.html`. |
| `cargo test --doc --all-features --locked` | Passed | 2026-06-18 | 0 doctests. |
| `cargo run --example typed_runtime_builder --locked` | Passed | 2026-06-18 | Example starts typed runtime handle through `VsmBuilder`, registers a typed unit, processes typed work, and shuts down. |
| `cargo run --example basic_usage --locked` | Passed | 2026-06-18 | Example starts runtime, registers `payments`, processes a transaction, prints status, and exits. |
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

---

## Work in progress

No implementation work is currently in progress. Milestone 5 is complete and
paused at the review gate.

---

## Decisions made

The user approved the Phase 0-only scope, approved Milestone 1 after the Phase
0 review gate, approved Milestone 2 after the Milestone 1 review gate, and
approved Milestone 3 after the Milestone 2 review gate, and approved Milestone
4 after the Milestone 3 review gate, and approved Milestone 5 after the
Milestone 4 review gate. Accepted migration decisions are recorded as ADRs.

| ADR | Decision | Status |
|---|---|---|
| [ADR-0001](docs/adr/0001-clean-breaking-migration-posture.md) | Clean breaking migration posture and Phase 0 boundary | Accepted |
| [ADR-0002](docs/adr/0002-application-type-family-and-role-contracts.md) | Minimal application type family and role contract shape | Accepted |
| [ADR-0003](docs/adr/0003-system1-runtime-semantics.md) | First System 1 runtime semantics | Accepted |
| [ADR-0004](docs/adr/0004-protocol-boundaries-and-deferred-decisions.md) | Protocol boundaries and explicitly deferred choices | Accepted |

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
  - Systems 2-5 typed semantics remain deferred; Milestone 5 adds bus mechanics
    and status records, not subsystem role catalogs.

---

## Compatibility changes

Milestones 1 through 5 add public foundational APIs. Milestone 5 intentionally
changes legacy broker behavior by removing targeted-to-broadcast fallback and
validating explicit broadcast targets.

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
- `vsm_rs::{UnitAdmissionLimits, UnitSnapshotConfig, UnitRegistration}`
- `vsm_rs::RegisteredUnit`
- `vsm_rs::{DeliveryMetrics, DeliveryStatus}`
- `vsm_rs::{RuntimeControlMessage, System1ControlMessage}`
- `vsm_rs::{DeliveryOutcome, UndeliverableMessage}`
- `vsm_rs::{ObserverBusSnapshot, ObserverId, ObserverSubscription}`
- `vsm_rs::async_trait`

New public channel/runtime APIs:

- `channels::publish_with_outcome`
- `channels::broadcast_with_outcome`
- `channels::dead_letters`
- `VsmRuntime::subscribe_events`
- `VsmRuntime::observer_event_history`
- `VsmRuntime::observer_bus_snapshot`

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
- Systems 2-5 still use string operation names and `serde_json::Value`.
- The typed runtime path now processes System 1 work through private unit actor
  adapters. Systems 2-5 and the legacy `start()` facade still use the current
  actor/JSON runtime.
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
| Systems 2-5 typed role catalogs and migrations | Later subsystem semantics require separate review gates. | Systems 2-5 continue to use string/JSON service calls. | System 1 pattern and owning milestone approval. | Systems 2-5 migrations |
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
