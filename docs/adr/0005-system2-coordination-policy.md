# ADR-0005: System 2 Coordination Policy Shape

- Status: Accepted
- Date: 2026-06-18
- Deciders: User, Codex

## Context

Milestone 6 converts System 2 from a JSON `ServiceActor` into typed
coordination. The roadmap requires System 2 to receive typed System 1
coordination views, invoke replaceable application policy, produce typed
interventions and acknowledgements, and keep authoritative resource allocation
out of System 2.

The key public API question is whether System 2 needs additional associated
types on the core application type family.

## Decision

Use a minimal view-centric `CoordinationPolicy` role for System 2.

The policy operates on typed System 1 `CoordinationView` records and returns
framework-owned generic conflict, intervention, acknowledgement, and escalation
records. It does not add new required associated types to `ViableSystem`.

## Rationale

This follows the accepted type-family posture: keep `ViableSystem` minimal and
add subsystem behavior through extension traits and protocol records.

Compared with a System 2 extension type family, generic records are enough for
the first coordination runtime slice and avoid prematurely encoding scheduling,
resource, or dependency semantics into core. Compared with keeping policy
private, a public `CoordinationPolicy` satisfies the milestone requirement that
applications can replace coordination policy independently.

## Consequences

- Applications can replace System 2 conflict and intervention policy without
  importing actor APIs or JSON.
- System 2 core records stay framework-owned and typed over the existing unit
  identity and capability family.
- Domain-specific scheduling/resource meanings remain outside core and can be
  expressed by policy implementations, defaults, adapters, or later subsystem
  extensions.
- Future milestones may add optional System 2 extension traits if real domains
  need application-owned payloads.

## Links

- `IMPLEMENTATION.md`
- `CODEX.md`
