# ADR-0001: Clean Breaking Migration Posture

- Status: Accepted
- Date: 2026-06-18
- Deciders: User, Codex

## Context

The current crate is `0.1.0`, has `publish = false`, and is not currently in use
elsewhere. The existing public API exposes actor internals, global names,
string/JSON dispatch, and demo behavior that would constrain the desired
trait-driven runtime if preserved as a compatibility requirement.

## Decision

Use a clean breaking redesign for the migration. Do not preserve a legacy public
facade as a design constraint. Remove or privatize legacy modules only when the
current milestone replaces their behavior.

Phase 0 remains a stabilization and characterization milestone only. It may fix
formatting, Clippy, factual documentation drift, ADR process records, and tests
that characterize current behavior. It must not introduce the foundational type
family, public builder, new runtime handles, or typed System 1 redesign.

## Rationale

No downstream compatibility obligation currently outweighs the need for a clear
public architecture. A clean break avoids letting the JSON/global actor facade
shape long-term APIs.

## Consequences

- Phase 0 records current behavior rather than preserving it indefinitely.
- Public compatibility changes are allowed in later approved milestones.
- Documentation must distinguish current behavior from planned architecture.
- Codex must stop after Phase 0 and wait for approval before foundational type
  work begins.
