# ADR-0008: System 5 Policy Boundary

- Status: Accepted
- Date: 2026-06-19
- Deciders: User, Codex

## Context

Milestone 9 converts System 5 from the remaining legacy JSON
`ServiceActor` services into typed runtime roles and protocol records. System 5
owns policy, identity, values, strategic decisions, crisis response, and typed
directives toward the rest of the viable system.

The current implementation exposes separate JSON services for policy, identity,
values, and decisions. Policy can act as an aggregate, but standalone actors
keep independent state. Current behavior also embeds application meaning in the
crate through default mission text, keyword-based alignment checks, fixed
weighted scoring, and generic crisis directives.

The public API decision is whether System 5 should follow the System 2 through
System 4 pattern of framework-owned records over the existing `ViableSystem`
type family, or whether it should expand the application type family with
domain-owned governance payload types.

## Options

### Option A: Minimal Framework-Owned Governance Records

Define framework-owned System 5 protocol records and public roles over the
existing `ViableSystem` associated types.

Use provider roles for identity and values data, behavior roles for alignment,
decision making, and crisis response, and framework-owned records for identity
versions, value sets, policy versions, decision requests, alternatives,
evidence, rationale, authority, review dates, directives, acknowledgements,
crisis signals, crisis responses, escalation records, and snapshots.

Do not add new required `ViableSystem` associated types in this milestone.
Move current mission text, keyword matching, fixed weighted scoring, and generic
crisis response behavior under `system5::defaults` or examples as opt-in
helpers.

Represent recursion escalation and algedonic input with typed records now, but
defer detailed recursion authority, translation rules, and durable event replay
to their owning milestones.

### Option B: System 5 Extension Type Family

Define a `System5Types` or `System5Roles` extension with application-owned
associated types for identity documents, value sets, policy payloads, decision
subjects, alternatives, evidence, crisis inputs, directives, and escalation
records.

This gives applications maximum domain fidelity immediately, but expands the
public type family before recursion, persistence, event replay, and
application-specific governance adapters have proven the necessary boundaries.

### Option C: Decision Lifecycle First

Convert the typed policy-decision lifecycle now, including evidence,
alternatives, rationale, authority, review date, and directive
acknowledgements. Leave identity, values, and crisis response on the legacy JSON
boundary or defer them to a later review gate.

This reduces the first System 5 slice, but it leaves the crate with mixed
policy ownership and does not fully satisfy the milestone goal of removing the
remaining System 5 JSON service boundary.

## Recommendation

Option A.

This matches the accepted migration posture from Systems 2, 3, and 4: keep
`ViableSystem` minimal, put application meaning in object-safe role traits, and
make runtime concerns such as identity versions, policy versions, audit trails,
directive acknowledgement, crisis routing, and escalation metadata
framework-owned. It also completes the System 5 boundary in one slice without
requiring the crate to define an application's mission, values, or decision
ethics.

## Decision

Use Option A: minimal framework-owned System 5 governance records and public
provider/evaluator/decision/crisis roles over the existing `ViableSystem` type
family.

Do not add new required associated types to `ViableSystem` for this milestone.
Identity, values, policy versions, decision audit trails, directives,
acknowledgements, crisis responses, and escalation records are represented as
framework-owned typed records. Applications provide organizational meaning
through role implementations.

## Consequences

If Option A is accepted:

- add public System 5 role traits without adding new required `ViableSystem`
  associated types;
- make identity and values supplied data through explicit provider roles or
  builder configuration, not crate-owned defaults;
- make alignment evaluation, decision procedure, and crisis response
  application-owned behavior roles;
- record typed decision evidence, alternatives, rationale, authority, review
  dates, and policy/identity versions;
- distribute typed directives and track acknowledgements;
- route or record System 3 operational concerns and System 4 future concerns as
  inputs to System 5 decision policy;
- represent algedonic crisis triggers and parent-recursion escalations as typed
  records while deferring detailed recursion semantics;
- move existing JSON heuristics to opt-in defaults/examples;
- remove the legacy System 5 JSON core path in this milestone.

## Links

- `IMPLEMENTATION.md`
- `CODEX.md`
