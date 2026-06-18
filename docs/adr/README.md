# Architecture Decision Records

This directory contains durable architecture decisions for the trait-driven VSM
runtime migration. `IMPLEMENTATION.md` remains the roadmap and acceptance
criteria; ADRs record decisions that should survive individual work sessions.

## Process

1. Create a new ADR from `0000-template.md` before making a material public API,
   compatibility, persistence, dependency, or milestone-scope decision.
2. Use `Proposed` while a decision is being discussed.
3. Move to `Accepted` only after explicit user approval.
4. Link accepted ADRs from `CODEX.md`.
5. Mark replaced decisions as `Superseded` and link to the newer ADR.

Silence is not approval. A recommendation in an ADR does not authorize the next
implementation milestone by itself.

## Index

| ADR | Status | Decision |
|---|---|---|
| [0001](0001-clean-breaking-migration-posture.md) | Accepted | Clean breaking migration posture and Phase 0 boundary |
| [0002](0002-application-type-family-and-role-contracts.md) | Accepted | Minimal application type family and role contract shape |
| [0003](0003-system1-runtime-semantics.md) | Accepted | First System 1 runtime semantics |
| [0004](0004-protocol-boundaries-and-deferred-decisions.md) | Accepted | Protocol boundaries and explicitly deferred choices |
| [0005](0005-system2-coordination-policy.md) | Accepted | Minimal view-centric System 2 coordination policy |
| [0006](0006-system3-role-boundary.md) | Accepted | System 3 control and System 3* audit role boundary |
