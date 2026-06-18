# ADR-0003: System 1 Runtime Semantics

- Status: Accepted
- Date: 2026-06-18
- Deciders: User, Codex

## Context

System 1 is the first vertical slice of the trait-driven runtime. The current
implementation routes JSON transactions to demo unit actors. The new slice must
define runtime responsibilities without taking over application behavior.

## Decision

The first System 1 slice supports register, list, process, drain, and unregister.

Work execution returns:

```rust
Result<Outcome, WorkError<AppError>>
```

Admission includes basic backpressure in the first System 1 slice. Overload
returns a typed `Backpressured` result immediately.

`WorkOptions` may set a deadline; otherwise a runtime default applies. Role
contexts expose a crate-owned cooperative cancellation abstraction rather than a
public dependency on `tokio-util`.

The runtime uses actor task isolation by default and may add opt-in offload
policy later. There are no automatic work retries in the first slice.

`StateStore` is always present. The default is `NoopStateStore`, which is not
persistent. Persistent `StateStore` implementations are deferred to the
persistence milestone. Core snapshot metadata includes typed snapshot keys and
versions. Missing snapshots start fresh; incompatible or rejected snapshots fail
unit restart.

The runtime has separate event and report sinks. Observer acknowledgements are
non-blocking for the control path, so monitoring can detect timeout or failure
without delaying work execution.

Capabilities come from static unit descriptors. Capacity and load come from
dynamic role data.

## Rationale

These rules establish correctness-sensitive runtime mechanics early while
avoiding premature durable persistence, retry, and execution-policy complexity.

## Consequences

- System 1 adapter tests must cover backpressure, timeout/cancel, restore
  success/failure, drain/unregister, default selector, and no-op policies.
- In-memory and no-op stores must not be described as persistent.
- Retry policy remains an explicit later decision.
