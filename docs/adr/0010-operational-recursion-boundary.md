# ADR-0010: Operational Recursion Boundary

- Status: Accepted
- Date: 2026-06-22
- Deciders: User, Codex

## Context

Milestone 11 turns recursion from a descriptive tree into an operational
runtime capability. The current crate already carries `RuntimeId`,
`RecursionPath`, and `VsmAddress` metadata, and `shared::recursion` can model a
hierarchy in memory. It does not yet start child VSM runtimes, treat child VSMs
as System 1 units, route typed protocols across parent/child boundaries, or
apply explicit translation, authority, and disclosure rules.

The public API decision is how much of recursion becomes a framework-owned
runtime surface now, and whether parent/child translation needs new application
associated types or can be represented by typed framework records plus
application-owned transducer roles.

Milestone 11 exit criteria require:

- a two-level VSM can execute work through a child runtime;
- a child can escalate a resource request and an algedonic alert to its parent;
- parent policy can be transduced into child-level directives;
- two child VSMs do not collide in actor registry names.

## Options

### Option A: Minimal Framework-Owned Recursion Protocols

Define framework-owned recursion records over the existing `ViableSystem`
family. Add public role traits for the application-owned boundary behavior,
centered on a `RecursionTransducer` that translates or filters information
crossing parent/child boundaries.

Add a typed recursion runtime surface that can register child runtime factories
and expose child VSMs as System 1 units through a bridge adapter. Keep generic
runtime mechanics in the framework:

- child runtime registration, listing, startup, shutdown, and health snapshots;
- child-as-unit descriptors, capacity snapshots, and work delegation;
- parent/child envelopes carrying runtime ID, recursion path, source,
  destination, correlation, causation, priority, deadline, and protocol version;
- framework records for delegated work, performance aggregation, resource
  escalation, policy directive delivery, intelligence summaries, and algedonic
  escalation;
- explicit authority and disclosure decisions returned by application roles;
- instance-scoped internal names derived from runtime ID and recursion path.

Do not add new required `ViableSystem` associated types in this milestone.

### Option B: Recursion Extension Type Family

Add a recursion-specific extension trait with application-owned associated
types for parent work, child work, performance summaries, resource escalation
payloads, policy translations, intelligence summaries, algedonic alerts, and
authority decisions.

This gives applications maximum fidelity at recursion boundaries, but it grows
the public type family before persistence, replay, and external adapters have
proven which payloads need to be first-class application types.

### Option C: Child Runtime Manager First

Implement only child runtime startup, shutdown, listing, and instance-scoped
name isolation. Defer child-as-System-1-unit behavior and cross-boundary
protocol transduction to a later milestone.

This lowers immediate API risk, but it would not satisfy the Milestone 11 exit
criteria around delegated work, resource/algedonic escalation, and parent
policy transduction.

## Recommendation

Option A.

This follows the pattern accepted for Systems 2 through 5 and the
variety/algedonic/temporal slice: keep `ViableSystem` minimal, put VSM runtime
mechanics in framework-owned typed records, and put application meaning,
translation, disclosure, and authority in object-safe roles. It also gives the
milestone enough surface to satisfy the two-level runtime and escalation exit
criteria without prematurely committing to durable recursion persistence.

## Decision

Option A is accepted.

Milestone 11 will add framework-owned recursion protocol records and public
transducer roles over the existing `ViableSystem` associated types. It will not
add a recursion-specific application type family in this slice.

## Consequences

If Option A is accepted:

- add typed recursion protocol records without adding new required
  `ViableSystem` associated types;
- add public recursion role traits, including a `RecursionTransducer`;
- add a typed recursion handle/manager owned by `VsmRuntime`;
- support child runtime factories and child VSM bridge units;
- route delegated work, resource escalation, algedonic escalation, policy
  directives, and intelligence summaries through typed boundary records;
- enforce authority and disclosure through application role decisions;
- keep durability, replay, cross-process transport, and persistent child
  runtime recovery deferred to their owning milestones.

## Links

- `IMPLEMENTATION.md`
- `CODEX.md`
