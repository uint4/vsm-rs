# ADR-0006: System 3 Role Boundary

- Status: Accepted
- Date: 2026-06-18
- Deciders: User, Codex

## Context

Milestone 7 converts System 3 and System 3* from the legacy JSON
`ServiceActor` path into typed runtime actors. System 3 owns internal control
and resource governance. System 3* is a distinct audit function that must keep
audit access separate from normal System 1 reporting.

The roadmap requires typed resource allocations, acknowledgements, authority,
directive versions, expiry, operational summaries, audit requests, evidence,
findings, remediation, and failed acknowledgement paths. The open public API
question is whether System 3 needs application-owned protocol payload types or
whether the first slice should use framework-owned generic records over the
existing `ViableSystem` type family.

## Decision

Use minimal framework-owned System 3 and System 3* protocol records with public
`ResourceGovernance`, `OperationalControlPolicy`, and `Auditor` roles over the
existing `ViableSystem` type family.

Do not add new required associated types to `ViableSystem` for this milestone.
Resource requests, allocation decisions, operational directives, audit
findings, and remediations are represented as framework-owned typed records
with IDs, metadata, authority, version, expiry, acknowledgement, and status
fields.

## Options

### Option A: Minimal Framework-Owned Records

Define public `ResourceGovernance`, `OperationalControlPolicy`, and `Auditor`
roles over framework-owned typed records. Use existing `ViableSystem` associated
types only, primarily `UnitId`, `Capability`, `AppError`, and `UnitSnapshot`.
Represent resource requests, allocations, directives, audit findings, and
remediations as generic framework records with IDs, metadata, authority,
version, expiry, acknowledgement, and status fields.

### Option B: System 3 Extension Type Family

Define a `System3Types` or `System3Roles` extension that introduces
application-owned associated types for resources, allocation decisions,
directives, audit evidence, findings, and remediation payloads.

### Option C: Control First, Audit Deferred

Convert System 3 resource governance/control to typed roles and actors now, but
leave System 3* audit as legacy JSON until a separate audit-specific milestone.

## Recommendation

Option A.

This preserves the accepted migration posture: keep the core application type
family minimal, use subsystem role traits for behavior, and let later
extensions introduce application-owned payload families only when real domains
need them. It also satisfies the Milestone 7 exit criteria in one slice without
forcing domain resource semantics into the crate.

## Consequences

Option A:

- add public System 3 role traits without adding new required `ViableSystem`
  associated types;
- keep resource allocation, operational directives, audit findings, and
  remediation records typed but framework-owned;
- let applications replace governance, control, and audit policy independently;
- allow richer domain-specific System 3 extensions later;
- remove the legacy System 3 JSON core path in this milestone.

## Links

- `IMPLEMENTATION.md`
- `CODEX.md`
