# ADR-0002: Application Type Family and Role Contracts

- Status: Accepted
- Date: 2026-06-18
- Deciders: User, Codex

## Context

The target runtime should own supervision, lifecycle, routing, telemetry, events,
snapshots, and VSM protocol mechanics while applications own domain work,
interpretation, policies, decisions, and persistence integrations. The type
family must be small enough to remain usable while still letting typed protocols
agree on core application concepts.

## Decision

Define a minimal `ViableSystem` core type family with associated types:

- `Work`
- `Outcome`
- `AppError`
- `Capability`
- `UnitId`
- `UnitSnapshot`

Use these core bounds:

- `Work: Clone + Send + 'static`
- `Outcome: Clone + Send + 'static`
- `AppError: std::error::Error + Send + Sync + 'static`
- `UnitId: Clone + Eq + std::hash::Hash + Send + Sync + 'static + std::fmt::Debug`
- `Capability: Clone + Eq + std::hash::Hash + Send + Sync + 'static + std::fmt::Debug`

Do not require serde bounds on application payloads or snapshots. Add optional
protocol extensions through separate traits instead of growing one giant trait.

Public async role contracts will use `async-trait`, re-exported by `vsm_rs`.
Runtime-selectable role policies will use dynamic dispatch. `OperationalUnit`
uses `&mut self`; policy roles use `&self`.

The first System 1 role family contains:

- `WorkModel`
- `OperationalUnitFactory`
- `UnitSelectionPolicy`
- `PerformanceModel`
- `VarietyModel`
- `AlgedonicPolicy`

Defaults or no-op implementations may exist for all except `WorkModel` and
`OperationalUnitFactory`. Factories are shared async factories that create fresh
unit role instances.

## Rationale

A minimal core type family keeps independent subsystem migrations possible. The
role list gives System 1 enough extension points to remove application meaning
from the runtime without forcing Systems 2-5 to inherit premature semantics.
`async-trait` is the most developer-friendly dyn-compatible async trait strategy
for this migration.

## Consequences

- Foundational types must prove that app payloads do not need serde.
- System 1 contract tests must show downstream-style role implementations that
  do not import actor APIs or JSON.
- Larger role catalogs for Systems 2-5 remain deferred to their owning
  milestones.
