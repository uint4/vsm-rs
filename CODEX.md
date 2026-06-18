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

- Approved milestone: Phase 0
- Approved scope: Baseline, characterization tests, ADR setup, stabilization
- Approved architectural decisions: Recorded in ADR-0001 through ADR-0004
- Pending decisions: None for Phase 0
- Permission to begin next milestone: No

## Pending user decisions

| ID | Decision | Options | Recommendation | Blocking milestone | Status |
|---|---|---|---|---|---|
| M1-approval | Begin foundational typed runtime work | Approve / revise / defer | Approve after reviewing Phase 0 evidence | Milestone 1 | Waiting for user |

## Current status

- Overall state: Phase 0 complete; waiting for user review
- Current phase: Phase 0 — baseline and characterization
- Current milestone: Phase 0 review gate
- Last updated: 2026-06-18
- Last updated by: Codex
- Baseline commit: `dea5b3e`
- Working branch: `master`
- Repository clean at start: Yes
- Repository status now: Intended Phase 0 changes are present in the working
  tree and have not been committed.

## Current objective

Phase 0 is implemented: formatting and Clippy baseline are fixed, ADR records
exist for approved decisions, factual documentation drift is corrected,
characterization tests cover current behavior, and the required validation suite
passes.

## Next action

Stop for user review. If approved, the next implementation milestone is
foundational typed runtime work: `ViableSystem`, framework metadata, errors,
cancellation, snapshot records, `StateStore`, event/report sink traits, and
System 1 protocol records. Do not begin this until the user explicitly approves.

---

## Milestone status

| Phase | Milestone | Status | Evidence |
|---|---|---:|---|
| 0 | Repository baseline | Complete | Formatting, check, tests, Clippy, docs, doctests, and example validation pass. |
| 0 | Characterization tests | Complete | `tests/phase0_characterization.rs` covers startup/health, System 1 no-unit resource request, targeted fallback bug-to-remove, broadcast validation gap, and Systems 2-5 JSON service calls. Existing System 1 and full-system tests still pass. |
| 0 | ADR setup | Complete | `docs/adr/README.md`, template, and ADR-0001 through ADR-0004 added. |
| 1 | Application type family | Not started | Awaiting user approval. |
| 1 | Typed core envelopes | Not started | Awaiting user approval. |
| 2 | Role contracts and factories | Not started | Awaiting user approval. |
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
| `cargo test --all-targets --all-features --locked` | Passed | 2026-06-18 | 7 integration tests plus example test target; 0 unit tests. |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed | 2026-06-18 | Original 9 warnings-as-errors resolved. |
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

---

## Work in progress

None. Phase 0 is at the review gate.

---

## Decisions made

The user approved the Phase 0-only scope and deferred later milestone decisions.
Accepted migration decisions are now recorded as ADRs.

| ADR | Decision | Status |
|---|---|---|
| [ADR-0001](docs/adr/0001-clean-breaking-migration-posture.md) | Clean breaking migration posture and Phase 0 boundary | Accepted |
| [ADR-0002](docs/adr/0002-application-type-family-and-role-contracts.md) | Minimal application type family and role contract shape | Accepted |
| [ADR-0003](docs/adr/0003-system1-runtime-semantics.md) | First System 1 runtime semantics | Accepted |
| [ADR-0004](docs/adr/0004-protocol-boundaries-and-deferred-decisions.md) | Protocol boundaries and explicitly deferred choices | Accepted |

---

## Compatibility changes

None. Phase 0 did not remove, rename, or semantically redesign public APIs.

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
| Foundational typed runtime traits and protocols | Phase 0 is stabilization and characterization only. | Public API remains actor/JSON-oriented until approved work begins. | User approval at Phase 0 gate. | Milestone 1 |
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
