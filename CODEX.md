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

- Approved milestone: Milestone 2 — role contracts and role contexts
- Approved scope: First-wave role contracts, role contexts, supporting
  infrastructure ports, no-op/test implementations, documentation, and
  downstream-style tests only. Do not rewrite actors, introduce the
  builder/runtime handle, or begin System 1 adapter migration.
- Approved architectural decisions: Recorded in ADR-0001 through ADR-0004
- Pending decisions: None for the approved Milestone 2 scope
- Permission to begin next milestone: No

## Pending user decisions

| ID | Decision | Options | Recommendation | Blocking milestone | Status |
|---|---|---|---|---|---|
| — | — | — | — | — | — |

## Current status

- Overall state: Milestone 2 complete; review gate active
- Current phase: Milestone 2 — role contracts and role contexts
- Current milestone: First-wave role contracts
- Last updated: 2026-06-18
- Last updated by: Codex
- Baseline commit: `4c2fa54`
- Working branch: `master`
- Repository clean at start: Yes
- Repository status now: Milestone 2 changes are present in the working tree
  and are not yet committed.

## Current objective

Milestone 2 role contracts and contexts are implemented: first-wave System 1
role traits, role object aliases, role contexts that expose runtime identity,
correlation, deadline, cancellation, clock, narrow event/report ports, and
explicitly allowed stores/adapters; supporting telemetry/alert/clock/id ports;
and no-op/test implementations. Actors were not rewritten and no
builder/runtime handle was introduced.

## Next action

Wait for explicit user approval to begin Milestone 3. Proposed next milestone:
add the typed builder, runtime handle, readiness/lifecycle surface, and
instance scope while keeping old actors underneath.

---

## Milestone status

| Phase | Milestone | Status | Evidence |
|---|---|---:|---|
| 0 | Repository baseline | Complete | Formatting, check, tests, Clippy, docs, doctests, and example validation pass. |
| 0 | Characterization tests | Complete | `tests/phase0_characterization.rs` covers startup/health, System 1 no-unit resource request, targeted fallback bug-to-remove, broadcast validation gap, and Systems 2-5 JSON service calls. Existing System 1 and full-system tests still pass. |
| 0 | ADR setup | Complete | `docs/adr/README.md`, template, and ADR-0001 through ADR-0004 added. |
| 1 | Application type family | Complete | `src/roles/types.rs` defines `ViableSystem`; `tests/foundational_types.rs` proves non-serde application work, outcome, and snapshot payloads compile. |
| 1 | Typed core envelopes | Complete | `src/protocol/*`, `src/error.rs`, `src/cancellation.rs`, `src/roles/ports.rs`, and `src/legacy/*` added with tests, docs, and full validation passing. |
| 2 | Role contracts and factories | Complete | `src/roles/context.rs`, `src/roles/system1.rs`, expanded `src/roles/ports.rs`, and `tests/role_contracts.rs` added; full validation passes. |
| 2 | Runtime builder and handles | Not started | Awaiting user approval. |
| 3 | System 1 vertical slice | Not started | Awaiting user approval. |
| 4 | Typed protocol bus | Not started | Awaiting user approval. |
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
| `cargo test --all-targets --all-features --locked` | Passed | 2026-06-18 | 28 integration tests across foundational, role-contract, Phase 0, full-system, and System 1 suites; example test target has 0 tests. |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed | 2026-06-18 | No warnings. |
| `cargo doc --all-features --no-deps --locked` | Passed | 2026-06-18 | Generated `target/doc/vsm_rs/index.html`. |
| `cargo test --doc --all-features --locked` | Passed | 2026-06-18 | 0 doctests. |
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

---

## Work in progress

None. Milestone 2 is at the review gate. Do not begin Milestone 3 without
explicit user approval.

---

## Decisions made

The user approved the Phase 0-only scope, approved Milestone 1 after the Phase
0 review gate, and approved Milestone 2 after the Milestone 1 review gate.
Accepted migration decisions are recorded as ADRs.

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

---

## Compatibility changes

Milestones 1 and 2 add public foundational APIs and do not remove, rename, or
semantically redesign existing public APIs.

New public modules and re-exports:

- `vsm_rs::cancellation`
- `vsm_rs::protocol`
- `vsm_rs::roles`
- `vsm_rs::legacy`
- `vsm_rs::{ApplicationFailure, FrameworkError, ViableSystem, WorkError}`
- `vsm_rs::{OperationalUnit, OperationalUnitFactory, WorkModel}`
- `vsm_rs::{UnitSelectionPolicy, PerformanceModel, VarietyModel}`
- `vsm_rs::{AlgedonicPolicy, System1Roles, RoleContext, UnitRoleContext}`
- `vsm_rs::async_trait`

Observed current behaviors are now characterized, including behaviors intended
for later removal:

- missing targeted channel subscriber falls back to broadcast;
- explicit channel broadcast bypasses targeted-message validation.

---

## Known issues and risks

- `PORTING_MAP.md` is still absent; docs now state this fact.
- The crate is `publish = false` and lacks final publication metadata and a
  `rust-version`; publication hardening is deferred.
- Application readiness still relies on sleeps; Phase 0 characterizes startup
  but does not add a readiness API.
- Actor names remain process-global; only one default VSM runtime can safely run
  per process.
- State, metrics, channel history, and most service data remain in memory and
  restart-volatile.
- Systems 2-5 still use string operation names and `serde_json::Value`.
- Typed foundations are not wired into the actor runtime yet; existing examples
  still run through the legacy actor/JSON facade.
- Temporary `legacy` adapters intentionally bridge current JSON forms for
  round-trip tests only; they are not the target public application surface.
- First-wave role contracts and contexts are defined but not yet wired into the
  running actor runtime.
- Channel targeted-delivery miss falls back to broadcast; characterized as a
  current bug-to-remove in a later typed-bus milestone.
- Explicit channel broadcast bypasses targeted-message validation; characterized
  as a current validation gap.
- Broker restart still loses subscriptions and retained history.
- System 1 Operations restart still loses its unit directory.
- System 1 unit supervisor restart can leave Operations with a stale supervisor
  reference.

Do not remove an issue merely because it is inconvenient. Remove it only when
resolved, and record the resolution in the development history.

---

## Deferred work

| Deferred item | Reason | Impact | Prerequisite | Intended milestone |
|---|---|---|---|---|
| Builder/runtime handles and readiness API | Would change public lifecycle architecture. | Tests still use startup sleeps around the current global runtime. | Typed foundations and user approval for builder work. | Builder/runtime handles |
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
