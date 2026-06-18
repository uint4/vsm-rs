# ADR-0004: Protocol Boundaries and Deferred Decisions

- Status: Accepted
- Date: 2026-06-18
- Deciders: User, Codex

## Context

The current port uses `serde_json::Value`, string operation names, process-global
actor names, and a channel broker that falls back from a missing targeted
subscriber to broadcast. The target architecture needs typed protocols and clear
control versus observation semantics, but not every subsystem decision belongs
in the first slice.

## Decision

Framework metadata may derive serde, but application payloads and snapshots do
not require serde in core. JSON and serde support belongs in explicit adapters
or features.

Targeted delivery must eventually report target correctness directly; a missing
target must not silently become a broadcast in the typed-bus milestone. Until
that milestone, Phase 0 characterization should label the current fallback as a
bug-to-remove.

Typed reports may precede the full typed bus. Cross-system report sinks are
separate from the eventual canonical control protocol.

Runtime defaults are opt-in and live under explicit default namespaces. The
first default strategy set includes lowest-load selection and no-op
performance, variety, and algedonic policies where appropriate.

The following decisions are deferred to their owning milestones:

- detailed Systems 2-5 role catalogs;
- recursion authority and translation rules;
- durable external `StateStore` adapters;
- full event replay and durability;
- automatic work retries;
- richer defaults;
- publication metadata;
- feature matrix;
- formal MSRV;
- removal timing for remaining temporary internals.

## Rationale

System 1 should establish the migration pattern without forcing later subsystem
semantics. Explicit deferral keeps Phase 0 and the first vertical slice from
becoming a hidden all-systems redesign.

## Consequences

- Phase 0 tests may characterize current fallback and broadcast validation gaps
  without preserving them as desired behavior.
- Later milestones must reopen the deferred decisions before implementation.
- `CODEX.md` must continue to record unresolved risks instead of erasing them.
