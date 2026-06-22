# Developer Guide

This guide is for maintainers and contributors working on `vsm-rs`. It explains how the crate is organized, which architectural constraints are intentional, how to extend actors and channels safely, and how to prepare a release.

For the conceptual model and runtime topology, start with [`ARCHITECTURE.md`](ARCHITECTURE.md). For consumer-facing examples, see [`USAGE.md`](USAGE.md).

## 1. Development principles

The crate is a port of an Elixir/OTP VSM implementation, but idiomatic Rust is the goal. Preserve the cybernetic behavior and supervision boundaries without mechanically reproducing every Elixir abstraction.

The core rules are:

1. **Actors own long-lived mutable state.** Do not introduce global mutable state to bypass actor messages.
2. **Supervise every long-lived actor.** A service that should survive failures belongs in a supervisor child specification.
3. **Prefer typed protocols.** Use explicit message enums and typed state when a domain protocol is stable.
4. **Keep JSON at integration boundaries.** `serde_json::Value` is useful for extensibility, but it should not replace domain types where invariants matter.
5. **Keep handlers responsive.** Actor handlers must not perform long blocking operations.
6. **Make failure behavior explicit.** Choose restart policies, RPC timeouts, validation, and error mapping deliberately.
7. **Preserve VSM flow constraints.** New routes must fit the command, coordination, audit, resource-bargain, algedonic, or temporal model—or document why the model is being extended.
8. **Document behavior, not aspirations.** Public docs must distinguish implemented behavior from planned work.

## 2. Prerequisites

Use a current stable Rust toolchain and Cargo. The crate currently targets the Rust 2021 edition.

```bash
rustup toolchain install stable
rustup default stable
```

Clone the repository and run the baseline checks:

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo doc --all-features --no-deps --locked
```

The runtime is Tokio-based. Integration tests use `serial_test` because actor names are process-global.

## 3. Repository layout

```text
src/
├── lib.rs                    Public exports
├── main.rs                   Demonstration binary
├── app.rs                    Root supervisor and application startup
├── vsm_core.rs               High-level lifecycle and status facade
├── actor_support.rs          Shared JSON ServiceActor implementation
├── domain.rs                 Core message and channel domain types
├── error.rs                  VsmError and VsmResult
├── cancellation.rs           Cooperative cancellation primitive for role contexts
├── config.rs                 Typed runtime configuration
├── builder.rs                Typed runtime builder
├── runtime.rs                Typed runtime handles, readiness, observer subscriptions, shutdown, component snapshots
├── kernel/                   Private runtime registry, observer bus, and typed System 1-5 actor adapters
├── protocol/                 Typed migration protocols, delivery outcomes, and framework metadata
├── roles/                    ViableSystem, role contexts, System 1-5 contracts, ports
├── legacy/                   Temporary adapters from current JSON API to typed foundations
├── names.rs                  Stable global actor names
├── channels/                 Broker, channels, algedonic, temporal services
├── shared/                   Message, recursion, and variety utilities
├── system1/                  Typed Operations and Unit actors
├── system2/                  Typed coordination defaults and legacy supervisor placeholder
├── system3/                  Typed defaults and legacy supervisor placeholder
├── system4/                  System 4 prototype defaults and supervisor placeholder
├── system5/                  System 5 prototype defaults and supervisor placeholder
└── telemetry_reporter.rs     Supervised telemetry service

tests/                        End-to-end actor tests
examples/                     Consumer-facing examples
docs/ARCHITECTURE.md           Runtime and design documentation
docs/USAGE.md                  Detailed user guide
docs/DEVELOPERS.md             Contributor guide
docs/adr/                      Architecture decision records
PORTING_MAP.md                 Not currently present
```

The typed builder/runtime modules are the public lifecycle surface for the
migration path. They should remain independent of `ActorRef`, global actor
names, and JSON application payloads. The typed System 1 path uses private
actor adapters under `kernel::system1`; typed System 2 coordination uses
`kernel::system2` and public `CoordinationPolicy` implementations; typed System
3 control and System 3* audit use `kernel::system3` and public
`ResourceGovernance`, `OperationalControlPolicy`, and `Auditor`
implementations. Typed System 4 uses `kernel::system4` and public
environmental source, signal interpreter, intelligence model, and forecaster
roles. Later subsystem adapters should follow that boundary and keep actor
references out of public handles. Observer subscriptions are exposed through
`VsmRuntime`, while fan-out and bounded event retention remain private to
`kernel::event_bus`.

## 4. Supervision and actor names

The root topology is built in `app.rs`. Child order matters during initial startup because channel infrastructure must exist before actors subscribe to it, and the System 1 dynamic unit supervisor must exist before `Operations` resolves it.

All persistent services use stable names from `names.rs`:

```rust
pub const SYSTEM1_OPERATIONS: &str = "vsm.system1.operations";
```

Do not scatter literal actor names through the codebase. Add a constant or name-builder function in `names.rs` and use it everywhere.

### Naming rules

- Use the `vsm.` prefix.
- Reflect the supervision hierarchy: `vsm.<subsystem>.<service>`.
- Dynamic System 1 units use `vsm.system1.unit.<unit-id>`.
- A name must identify one actor protocol. Do not reuse a name for actors with different `Msg` types.
- Treat names as part of the runtime compatibility surface; renaming them can break integrations and tests.

### Restart policies

Use `Restart::Permanent` for infrastructure that should always be restored after failure. Use `Restart::Temporary` for intentionally short-lived children or units that must not restart after normal termination.

Every child specification should make these choices visible:

```rust
ChildSpec {
    id: names::SYSTEM1_OPERATIONS.to_owned(),
    restart: Restart::Permanent,
    spawn_fn: /* ... */,
    backoff_fn: None,
    reset_after: Some(Duration::from_secs(60)),
}
```

When changing a supervisor, test both normal shutdown and abnormal child failure. A `OneForOne` tree should not restart unrelated siblings unless the strategy is intentionally changed.

## 5. Actor implementation patterns

The crate currently has two actor styles.

### 5.1 Typed actors

Typed actors are the preferred design for stable behavior. Existing examples include the channel broker, System 1 Operations, System 1 Unit, the algedonic processor, and temporal variety.

A typed actor normally defines:

```rust
pub enum ExampleMsg {
    GetState(RpcReplyPort<ExampleSnapshot>),
    Apply(ExampleCommand, RpcReplyPort<VsmResult<ExampleResult>>),
    Notify(ExampleEvent),
}

pub struct ExampleArgs {
    pub config: ExampleConfig,
}

pub struct ExampleState {
    config: ExampleConfig,
    // Actor-owned mutable state.
}

pub struct ExampleActor;
```

The actor implementation should keep lifecycle and domain handling separate where practical:

```rust
#[ractor::async_trait]
impl Actor for ExampleActor {
    type Msg = ExampleMsg;
    type State = ExampleState;
    type Arguments = ExampleArgs;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        // Resolve required actors, subscribe to channels, initialize state.
        Ok(ExampleState { config: args.config })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        // Keep message matching small; delegate domain work to helpers.
        Ok(())
    }

    async fn post_stop(
        &self,
        myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        // Remove subscriptions and release resources.
        Ok(())
    }
}
```

Add a public facade so consumers do not need to construct internal actor messages:

```rust
pub async fn get_state() -> VsmResult<ExampleSnapshot> {
    let actor = actor_ref()?;
    call_t!(actor, ExampleMsg::GetState, 2_000)
        .map_err(|err| VsmError::Ractor(err.to_string()))
}
```

#### Typed actor checklist

- Define domain input, output, and snapshot types.
- Define the actor message enum.
- Define startup arguments and actor-owned state.
- Add the actor's stable name.
- Add a `ChildSpec` to the correct supervisor.
- Use an explicit restart policy and timeout values.
- Expose a small public facade.
- Subscribe in `pre_start` and unsubscribe in `post_stop`.
- Add unit and integration tests.
- Update rustdoc, `ARCHITECTURE.md`, and `USAGE.md` as needed.

### 5.2 JSON ServiceActor

Telemetry and auxiliary services use `actor_support::ServiceActor`. It provides a common protocol:

```rust
pub enum ServiceMsg {
    Call(String, Value, RpcReplyPort<VsmResult<Value>>),
    Cast(String, Value),
    Channel(VsmActorMsg),
    Tick(String),
}
```

Each service delegates operations to an `actor_call` function:

```rust
pub async fn actor_call(
    op: &str,
    payload: Value,
    state: &mut ServiceState,
) -> VsmResult<Value> {
    match op {
        "get_state" => Ok(state.data.clone()),
        "apply_policy" => apply_policy(payload, state),
        _ => Ok(json!({"status": "unknown_operation", "op": op})),
    }
}
```

When adding a service operation:

1. Choose a stable operation name.
2. Validate required fields before mutating state.
3. Delegate pure calculations to a separate function.
4. Add a typed convenience wrapper even when the underlying payload is JSON.
5. Record meaningful state/history changes.
6. Test malformed payloads and unknown operations.
7. Document the request and response schema in `USAGE.md`.

Do not expand the generic service interface indefinitely. Promote an operation to a typed actor protocol when it gains meaningful invariants, multiple error states, complex concurrency, or broad public use.

## 6. Channels and messages

`channels::broker::ChannelBroker` owns subscriptions, message history,
dead-letter history, and delivery metrics. Inter-system messages use
`VsmMessage` from `domain.rs`.

### Adding a message kind

1. Add the variant to `MessageKind`.
2. Decide whether it is high priority in `MessageKind::is_high_priority`.
3. Add handling in the intended receiver.
4. Add serialization tests.
5. Document the payload schema.

### Adding or changing a route

Update `VsmMessage::validate_basic_flow` and add tests for both permitted and rejected source/destination pairs. Do not bypass flow validation merely to make a test pass.
Missing targeted subscribers must return or record `TargetUnavailable`; do not
restore targeted-to-broadcast fallback. Explicit broadcast must use
`SystemId::All` and should be exercised through the outcome-returning broker
APIs when correctness matters.

The current route families are:

- Command: policy/intelligence/control toward operations.
- Coordination: System 1 and System 2.
- Audit: System 1 with System 3 or System 3*.
- Resource bargain: System 1 and System 3.
- Algedonic: urgent signals toward System 5 or the algedonic processor.
- Temporal variety: cross-timescale observations and analyses.

### Subscriptions

Subscriptions are keyed by `(ChannelKind, subscriber_id)`. Reusing an ID replaces the previous listener for that channel. Choose IDs deliberately and use `SystemId::subscriber_id()` for standard subsystem targets.

Actors with a larger internal protocol can expose a channel-only view through `get_derived::<VsmActorMsg>()`.

Subscribe during startup:

```rust
let channel_ref = myself.get_derived::<VsmActorMsg>();
channels::subscribe(ChannelKind::Command, "example", channel_ref).await?;
```

Unsubscribe during shutdown:

```rust
let _ = channels::unsubscribe(ChannelKind::Command, "example").await;
```

### Channel-history rules

History and dead-letter history are in memory and bounded by the broker
implementation. Changes to retention count, ordering, delivery metrics, or
message redaction are externally observable and should be documented. Never put
secrets into channel payloads or tracing fields unless the embedding
application explicitly accepts that risk.

## 7. State, concurrency, and blocking work

Actor state is serialized by each mailbox, but the entire runtime remains concurrent. Code in one actor handler can delay every later message to that actor.

Inside `handle` and `actor_call`:

- Do not call blocking filesystem, database, or network APIs directly.
- Use asynchronous clients for I/O.
- Move CPU-heavy work to `tokio::task::spawn_blocking` or a dedicated worker actor.
- Avoid holding locks across `.await`.
- Keep RPC timeouts finite.
- Avoid recursive actor calls that can form request cycles.

Pure calculations should remain outside actors whenever possible. This makes scheduling, allocation, forecasting, variety calculations, and policy evaluation easy to unit test.

## 8. Error handling

Public fallible APIs should return `VsmResult<T>` and map internal failures to `VsmError`.

Use the narrowest useful variant:

- `InvalidInput` or `InvalidPayload` for caller data.
- `Validation` for domain-rule failures.
- `ActorNotFound` or `ActorUnavailable` for registry/runtime failures.
- `Supervisor` for child-management failures.
- `Channel` for broker and routing failures.
- `Serialization` for JSON conversion.
- `Ractor` only when no more specific mapping exists.

Avoid `unwrap`, `expect`, and silent error suppression in library code. Logging an error is not a substitute for returning it when the caller can act on the failure.

Actor RPC helpers should always use explicit timeouts:

```rust
call_t!(actor, ExampleMsg::GetState, 2_000)
```

Timeout values are part of runtime behavior. Changes should be justified and tested under load.

## 9. Testing strategy

### Pure modules

Use ordinary unit tests for deterministic calculations:

- Variety calculation, attenuation, and amplification.
- Scheduling and conflict detection.
- Resource allocation and audit analysis.
- System 4 role implementations and prototype forecasting/analytics helpers.
- Identity, value, policy, and decision evaluation.
- Message-flow validation and serialization.

Prefer small fixtures and assert domain meaning, not only JSON shape.

### Actor integration tests

The actor registry is process-global, so tests that start the default application must use `#[serial]`:

```rust
use serial_test::serial;

#[tokio::test]
#[serial]
async fn actor_flow() {
    let (root, join_handle) = vsm_rs::start_vsm_core()
        .await
        .expect("runtime should start");

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Exercise the flow.

    root.stop(Some("test complete".to_owned()));
    let _ = join_handle.await;
}
```

Every integration test must stop the root supervisor and await its join handle. A leaked actor name can make later tests fail non-deterministically.

Cover at least:

- Successful startup and shutdown.
- Child restart behavior.
- Unit registration and duplicate registration.
- Capability-based transaction routing.
- No-suitable-unit behavior and resource request emission.
- Valid and invalid channel routes.
- Subscription replacement and cleanup.
- Algedonic bypass behavior.
- Metrics, variety, and history bounds.
- Operational recursion with unique runtime IDs and recursion paths for every
  child fixture.
- Malformed JSON service calls.

### Documentation examples

Keep README and rustdoc examples compilable. When practical, move substantial examples into `examples/` and reference them from Markdown so CI exercises the same code users read.

## 10. Logging and observability

The crate uses `tracing`. Library code should emit structured fields rather than formatting large strings:

```rust
tracing::info!(unit_id = %unit_id, "registered System 1 unit");
```

Guidelines:

- `trace`: very high-volume internal detail.
- `debug`: routing decisions and nonessential state transitions.
- `info`: lifecycle events and meaningful successful operations.
- `warn`: degraded behavior, rejected messages, and recoverable failures.
- `error`: failed startup, lost required infrastructure, and unrecoverable actor behavior.

Do not initialize a global tracing subscriber inside library code. Applications and examples own subscriber configuration.

## 11. Documentation standards

Public modules, types, and functions should have rustdoc explaining:

- What the item represents.
- Preconditions and invariants.
- Error behavior.
- Actor or channel side effects.
- Whether state is in memory.
- A minimal example for important entry points.

Keep the guides aligned:

- `README.md`: crate purpose, installation, quick start, major capabilities, constraints.
- `USAGE.md`: comprehensive consumer workflows and payload schemas.
- `ARCHITECTURE.md`: runtime topology, ownership, routing, supervision, recovery.
- `DEVELOPERS.md`: contribution rules, extension patterns, testing, releases.
- `PORTING_MAP.md`: correspondence with the original Elixir project when this
  file is recreated.

A behavior change is incomplete until its public documentation is updated.

## 12. Dependency policy

Before adding a dependency:

- Confirm the standard library or an existing dependency cannot solve the problem.
- Evaluate maintenance, license, security history, and transitive cost.
- Avoid pulling runtime frameworks into pure domain modules.
- Keep `ractor` compatible with the version required by `ractor-supervisor`.
- Add optional functionality behind Cargo features when it materially increases compile time or dependency weight.

Run a dependency audit in release CI when tooling is available, for example with `cargo audit` and `cargo deny`.

## 13. Release process

Crate releases are permanent artifacts. Use a repeatable release checklist.

### Manifest audit

Before the first crates.io release, verify that `Cargo.toml` contains the final values for:

- `name`
- `version`
- `description`
- `license` or `license-file`
- `readme`
- `repository`
- `documentation`
- `keywords`
- `categories`
- `rust-version`
- `publish`

The repository currently uses `publish = false` as a safety guard. Remove it or replace it with the intended registry configuration only when the crate name and metadata are final.

### Release checks

```bash
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --no-deps
cargo package --list
cargo publish --dry-run
```

Inspect the package contents and ensure the archive includes:

- Rust sources required to build.
- `README.md`.
- `LICENSE`.
- Relevant examples.
- Any documentation linked from the README that should ship with the crate.

It must not include build output, local credentials, editor state, generated archives, large test artifacts, or private design notes.

### Versioning and notes

- Update the changelog or release notes.
- Bump the version according to semantic versioning.
- Call out changes to actor names, message enums, JSON schemas, channel routing, restart policies, and persistence behavior.
- Create a signed or annotated git tag for the released commit.
- Publish only from a clean, reviewed commit.
- Verify the crates.io page and docs.rs build after publication.

## 14. Pull request checklist

Before requesting review:

- [ ] The change has a focused purpose and no unrelated generated edits.
- [ ] Actor state remains actor-owned.
- [ ] New long-lived services are supervised.
- [ ] New actor names are centralized in `names.rs`.
- [ ] RPC calls have explicit timeouts.
- [ ] Inputs and message routes are validated.
- [ ] Errors use appropriate `VsmError` variants.
- [ ] Unit and/or actor integration tests cover success and failure paths.
- [ ] `cargo fmt`, `cargo clippy`, and `cargo test` pass.
- [ ] Public rustdoc and Markdown guides are updated.
- [ ] No secrets, local paths, or large generated files are included.

## 15. Known architectural work

The following areas are important candidates for future hardening:

1. **Readiness:** replace startup sleeps with an explicit application-ready signal.
2. **Subscription recovery:** re-register subscribers automatically when the broker restarts.
3. **Unit reconciliation:** ensure System 1 Operations reconstructs its directory after unit or supervisor restarts.
4. **Algedonic durability:** persist typed algedonic lifecycle records and
   reconcile unresolved signals after restart.
5. **Persistence:** define opt-in snapshots or event persistence without coupling the core to one database.
6. **Backpressure:** add explicit mailbox and ingress controls for high-volume channel traffic.
7. **Namespacing:** support more than one VSM runtime in a process through configurable actor-name prefixes.
8. **Graceful shutdown:** expose a high-level shutdown method that signals and awaits the full tree.
9. **Schema versioning:** version public message and JSON payload contracts.
10. **Operational telemetry:** expose restart counts, queue pressure, call latency, and rejected-message metrics.

Changes in these areas should begin with an architecture proposal because they affect multiple subsystems and public behavior.

## 16. Attribution

The project is derived from the MIT-licensed Elixir/OTP implementation at `viable-systems/vsm-core`. Preserve the original license notice. If `PORTING_MAP.md` is recreated, keep it accurate when behavior is moved, replaced, or redesigned.
