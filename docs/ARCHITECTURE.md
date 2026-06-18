# Architecture

This document describes the architecture of the current `vsm-rs` crate: how it starts, where state lives, how actors communicate, how the five VSM systems are represented, and where the current implementation intentionally differs from Elixir/OTP.

The crate is a conceptual port of `vsm-core`, not a byte-for-byte translation. Elixir process registries, GenServers, supervisors, and dynamic supervisors are represented with `ractor`, `ractor-supervisor`, typed Rust messages, and JSON integration boundaries.

## 1. Architectural model

The system has four layers:

1. **Application and supervision** — starts and restarts the root services.
2. **Actor services** — own mutable runtime state and serialize access through mailboxes.
3. **Channels** — route `VsmMessage` values between actors and retain in-memory history.
4. **Pure domain modules** — scheduling, allocation, analysis, identity, variety engineering, recursion, and related calculations.

The five VSM systems map to the code as follows:

| VSM subsystem | Responsibility in this crate | Primary code |
|---|---|---|
| System 1 | Operational units, transaction routing, operational metrics, operational variety | `src/system1/` |
| System 2 | Coordination, schedule conflict handling, load/resource balancing | `src/system2/` |
| System 3 | Resource allocation, control state, audit calculations | `src/system3/` |
| System 4 | Environmental scanning, analytics, intelligence, forecasting | `src/system4/` |
| System 5 | Policy, identity, values, decisions, crisis response | `src/system5/` |

Cross-cutting services live under `src/channels/` and `src/shared/`.

## 2. Crate layout

```text
src/
├── lib.rs                    Public module exports
├── main.rs                   Runnable demonstration binary
├── app.rs                    Root supervision tree and startup
├── vsm_core.rs               High-level facade: start, stop, health, status
├── actor_support.rs          Shared JSON service actor implementation
├── domain.rs                 SystemId, ChannelKind, MessageKind, VsmMessage
├── error.rs                  VsmError and VsmResult
├── cancellation.rs           Crate-owned cooperative cancellation token
├── protocol/                 Typed migration foundations: addresses, metadata, snapshots, events, System 1 records
├── roles/                    ViableSystem type family and runtime ports
├── legacy/                   Temporary adapters from current JSON/System 1 API to typed foundations
├── names.rs                  Stable global actor names
├── prelude.rs                Small JSON/time helpers
├── util.rs                   Numeric and JSON merge helpers
├── telemetry_reporter.rs     Supervised telemetry service facade
├── channels/
│   ├── broker.rs             Pub/sub and targeted message broker
│   ├── supervisor.rs         Broker, algedonic, temporal-variety children
│   ├── *_channel.rs          Channel-specific convenience APIs
│   ├── algedonic/            Typed algedonic signal processor
│   ├── temporal_variety.rs   Typed temporal-variety actor
│   └── temporal/             Pure temporal analysis helpers
├── shared/
│   ├── message.rs            Message construction/serialization facade
│   ├── channel.rs            Generic channel facade
│   ├── recursion.rs          Recursive-system hierarchy helpers
│   ├── variety_engineering.rs
│   └── variety/              Calculator, attenuation, amplification
├── system1/                  Typed Operations and Unit actors
├── system2/                  Coordination service and pure algorithms
├── system3/                  Control service and pure algorithms
├── system4/                  Intelligence service family
└── system5/                  Policy service family
```

A porting map for the original Elixir files is not currently present in this
repository. Recreate it before relying on file-by-file correspondence during a
future migration slice.

### 2.1 Trait-driven foundation modules

The crate now includes public migration foundations under `protocol`, `roles`,
`cancellation`, and `legacy`.

These modules are intentionally alongside the current runtime:

- `roles::ViableSystem` defines the minimal application type family.
- `protocol::address`, `protocol::envelope`, and `protocol::snapshot` define
  instance-scoped runtime metadata.
- `protocol::system1` defines typed System 1 records for work, unit descriptors,
  capacity/load, command acknowledgements, performance observations, resource
  shortages, audit evidence, and coordination views.
- `roles::ports` defines `StateStore`, `EventSink`, and `ReportSink`, plus
  no-op implementations. `NoopStateStore` is not persistent.
- `cancellation::CancellationToken` is the crate-owned cooperative cancellation
  primitive for future role contexts.
- `legacy::system1` contains temporary adapters for current
  `Transaction`/`TransactionResult`/`UnitConfig`/`VsmMessage` shapes.

The foundation modules do not expose `ActorRef`, actor names, or `ractor`
message types. Core typed protocol records do not require application work,
outcome, error, capability, unit ID, or snapshot payloads to implement serde.
The existing actor runtime still uses the legacy facade until later approved
milestones wire these types into role adapters and runtime handles.

## 3. Supervision tree

`app::start_vsm_core()` starts one globally named root supervisor. Every static supervisor uses `SupervisorStrategy::OneForOne`.

```text
vsm.root_supervisor
├── vsm.dynamic_supervisor
│   └── currently unused general-purpose dynamic child area
├── vsm.channels.supervisor
│   ├── vsm.channels.broker
│   ├── vsm.channels.algedonic
│   └── vsm.channels.temporal_variety
├── vsm.system1.supervisor
│   ├── vsm.system1.unit_supervisor
│   │   ├── vsm.system1.unit.<unit-id>
│   │   └── ...runtime units...
│   └── vsm.system1.operations
├── vsm.system2.supervisor
│   └── vsm.system2.coordination
├── vsm.system3.supervisor
│   └── vsm.system3.control
├── vsm.system4.supervisor
│   ├── vsm.system4.intelligence
│   ├── vsm.system4.scanner
│   ├── vsm.system4.analytics
│   └── vsm.system4.forecasting
├── vsm.system5.supervisor
│   ├── vsm.system5.policy
│   ├── vsm.system5.identity
│   ├── vsm.system5.values
│   └── vsm.system5.decisions
└── vsm.telemetry_reporter
```

### Restart configuration

The static supervisors and their permanent children use the following common policy:

- Strategy: `OneForOne`
- Maximum restarts: 5
- Restart window: 10 seconds
- Counter reset: 30 seconds
- Child reset interval: generally 60 seconds

The two dynamic supervisors allow up to 10 restarts in a 10-second window. System 1 units use:

- `Restart::Permanent` when `UnitConfig.auto_restart == true`
- `Restart::Temporary` when `auto_restart == false`

### Startup order

The root child specifications place channel infrastructure before Systems 1–5. That matters because actors subscribe to the broker during `pre_start`. System 1 also starts its unit dynamic supervisor before its Operations actor, allowing Operations to resolve it by name during startup.

The strategy remains `OneForOne`: ordering helps initial startup but does not create `RestForOne` failure coupling.

## 4. Global actor registry

The crate uses ractor's global named actor registry instead of a separate unique Elixir `Registry` process. Stable names are defined in `src/names.rs`.

Examples:

```text
vsm.root_supervisor
vsm.channels.broker
vsm.system1.operations
vsm.system3.control
vsm.system5.policy
vsm.system1.unit.payments
```

Code retrieves an actor with a typed lookup:

```rust
ActorRef::<OperationsMsg>::where_is(names::SYSTEM1_OPERATIONS.to_string())
```

This creates two important constraints:

1. A process can run only one application instance using these names.
2. Tests that start the application must run serially or use a separate process.

The test suite uses `serial_test` for this reason.

## 5. Actor implementation styles

The crate currently uses two actor styles.

### 5.1 Dedicated typed actors

These actors have explicit message enums and purpose-built state:

| Actor | Message type | State |
|---|---|---|
| Channel broker | `ChannelBrokerMsg` | Subscribers and per-channel history |
| System 1 Operations | `OperationsMsg` | Unit directory, metrics, variety log |
| System 1 Unit | `UnitMsg` | Unit config, status, load, local state |
| Algedonic processor | `AlgedonicMsg` | Signals, filters, routes, counters |
| Temporal variety | `TemporalVarietyMsg` | Timescale buffers and analyses |

This style gives compile-time message checking and is the preferred model for behavior with a stable domain protocol.

### 5.2 Shared JSON `ServiceActor`

Systems 2–5 and telemetry use `actor_support::ServiceActor`.

```rust
pub enum ServiceMsg {
    Call(String, Value, RpcReplyPort<VsmResult<Value>>),
    Cast(String, Value),
    Channel(VsmActorMsg),
    Tick(String),
}
```

Each instance has a `ServiceKind`, and calls are delegated to a module-specific `actor_call` function:

```text
ServiceKind::System2Coordination  -> system2::coordination::actor_call
ServiceKind::System3Control       -> system3::control::actor_call
ServiceKind::System4Analytics     -> system4::analytics::actor_call
ServiceKind::System5Policy        -> system5::policy::actor_call
...
```

`ServiceState` contains:

```text
kind      Service identity
 data      Mutable JSON object
 history   Newest-first event history, capped at 1,000 entries
```

The initial `data` value has this shape:

```json
{
  "id": "system4",
  "config": {
    "subsystem": "system4",
    "role": "analytics"
  },
  "started_at": "...",
  "status": "running"
}
```

This service shell is flexible and expedites the port, but operation names and payload schemas are checked at runtime rather than by Rust's type system. Unknown operations return a JSON `unknown_operation` response rather than a `VsmError`.

## 6. Message model

Inter-subsystem messages use `domain::VsmMessage`:

```rust
pub struct VsmMessage {
    pub id: String,
    pub from: SystemId,
    pub to: SystemId,
    pub channel: ChannelKind,
    pub kind: MessageKind,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<Value>,
    pub correlation_id: Option<String>,
    pub reply_to: Option<String>,
}
```

The serialized `kind` field is named `type` for compatibility with the Elixir message shape.

### Channel kinds

```text
Command
Coordination
Audit
Algedonic
ResourceBargain
TemporalVariety
```

### Flow validation

`VsmMessage::validate_basic_flow()` permits these internal routes:

| Channel | Permitted internal routes |
|---|---|
| Command | S5→S4, S5→S3, S4→S3, S3→S1, S5→S1, S4→S1 |
| Coordination | S1↔S2 |
| Audit | S3*↔S1 and S3↔S1 |
| Algedonic | Any source to S5 or the algedonic endpoint |
| Resource bargain | S1↔S3 |
| Temporal variety | Any route |

Messages from or to `SystemId::External`, and messages to `SystemId::All`, bypass the internal flow matrix.

A reply generated with `VsmMessage::reply()` reverses the endpoints, preserves the channel, sets `reply_to`, and propagates or creates a correlation ID.

## 7. Channel broker

`channels::broker::ChannelBroker` replaces the original duplicate channel registries. It owns:

```text
HashMap<ChannelKind, HashMap<subscriber_id, subscriber_ref>>
HashMap<ChannelKind, Vec<VsmMessage>>
```

Each channel retains up to 10,000 messages, newest first.

### 7.1 Subscription identity

A subscription is keyed by `(ChannelKind, subscriber_id)`. Registering the same ID again replaces the previous subscriber. This is not identical to an Elixir duplicate registry, which can retain multiple processes under the same key.

Use unique IDs when multiple listeners need the same channel. For targeted subsystem routing, use the IDs returned by `SystemId::subscriber_id()`, such as `system1`, `system2`, or `system5`.

### 7.2 Publish routing

For `ChannelBrokerMsg::Publish`:

1. The broker validates the `VsmMessage`.
2. High-priority message kinds are logged.
3. Algedonic messages are always delivered to subscriber ID `system5`.
4. Other messages target `message.to.subscriber_id()`.
5. If the target does not exist, the broker broadcasts the message to all subscribers on that channel.
6. The message is retained in channel history.

The fallback broadcast preserves the permissive behavior of the original registry dispatch, but it also means a misspelled or absent target can result in wider delivery than intended.

### 7.3 Explicit broadcast

`channels::broadcast()` sends to every current subscriber on a channel and retains the message. The broker's explicit broadcast branch currently does not run the same validation step used by `Publish`.

### 7.4 Delivery semantics

Channel delivery is:

- in-process
- asynchronous
- best effort
- not durable
- not acknowledged by recipients

`channels::publish()` reports whether the message reached the broker mailbox, not whether a recipient processed it. Because validation happens asynchronously inside the broker, an invalid message may be accepted by `publish()` and then logged and dropped. Call `message.validate()` before publishing when synchronous validation matters.

The broker removes a subscriber only when a delivery attempt reports a closed actor reference. It does not currently monitor every subscriber proactively.

## 8. Derived actor references

Channel subscribers do not need to expose their entire actor protocol. Actors implement conversions between their own message enum and `VsmActorMsg`, then create a restricted reference:

```rust
let channel_ref = myself.get_derived::<VsmActorMsg>();
```

For example, System 1 uses:

```rust
OperationsMsg::Channel(VsmActorMsg)
```

This lets the broker store a homogeneous `DerivedActorRef<VsmActorMsg>` while the receiving actor retains a richer private protocol.

## 9. Default channel subscriptions

The following subscriptions are created during actor startup:

| Actor | Subscriber ID | Channels |
|---|---|---|
| System 1 Operations | `system1` | Command, Coordination, Audit |
| System 2 Coordination | `system2` | Coordination |
| System 3 Control | `system3` | ResourceBargain, Command, Audit |
| System 4 Intelligence | `system4` | Command |
| System 5 Policy | `system5` | Algedonic |

The dedicated algedonic processor and temporal-variety actor are accessed through their typed APIs; they do not subscribe to the broker in the current supervision tree.

A major current distinction is that `ServiceActor` channel handling records the received message in `history`, but does not dispatch it into the module's domain operations. System 1 has explicit channel behavior; Systems 2–5 currently treat channel messages as observable events unless application code makes a separate service call.

## 10. System 1: operational execution

System 1 is the most complete actor subsystem in the port.

### 10.1 State ownership

`OperationsState` owns:

```text
units            Unit ID -> stable actor name, config, start time
unit_supervisor  ActorRef to vsm.system1.unit_supervisor
metrics          In-memory MetricsStore
variety_log      Newest-first measurements, capped at 1,000
config           Startup JSON configuration
```

The directory stores stable actor names rather than long-lived unit references. Each operation resolves the current `ActorRef<UnitMsg>` from the registry, so a unit restarted under the same name can be found again.

### 10.2 Unit registration

```text
caller
  -> OperationsMsg::RegisterUnit
      -> DynamicSupervisor::spawn_child
          -> vsm.system1.unit.<id>
      -> store UnitInfo
      -> publish Coordination/UnitRegistered to System 2
      -> reply with unit ID
```

A duplicate ID returns `VsmError::UnitAlreadyRegistered`.

### 10.3 Transaction routing

```text
caller
  -> OperationsMsg::ProcessTransaction
      -> validate transaction kind
      -> query every registered unit: CanHandle
      -> query matching units: GetLoad
      -> choose the lowest-load unit
      -> UnitMsg::Process
      -> record metrics
      -> calculate and retain operational variety
      -> reply TransactionResult
```

A unit can handle a transaction only when it contains **all** required capabilities.

When no suitable unit exists, Operations publishes a `ResourceBargain/UnitRequest` message to System 3 and returns `TransactionResult::NoSuitableUnit`. System 3 currently records that channel event; it does not automatically allocate a unit or reply.

### 10.4 Current unit implementation

`system1::unit::Unit` is a generic demonstration actor. Processing returns JSON metadata describing the transaction and unit. Its load is incremented before processing and decremented immediately afterward. Real applications are expected to replace or extend this behavior with domain work, external I/O, queues, and meaningful load measurement.

### 10.5 Metrics and variety

System 1 metrics track:

- total transactions
- successes
- failures
- invalid transactions
- no-suitable-unit outcomes

Operational variety is derived from transaction capability/payload cardinality and result cardinality. The current snapshot averages the most recent 100 measurements and computes a trend from two windows of up to 10 ratios.

### 10.6 System 1 channel behavior

System 1 actively handles:

- `Command + Execute`: forwards the command to every registered unit.
- `Coordination + Coordinate`: deserializes `SyncState` or `LoadBalance` and performs it.
- `Audit + AuditRequest`: publishes an audit response with units, metrics, variety, timestamp, and config.

Other messages are ignored after a debug log.

## 11. System 2: coordination

System 2 has one `ServiceActor`, `vsm.system2.coordination`, plus pure modules:

- `scheduler`: combines schedules, detects temporal/resource/dependency conflicts, reorders entries, validates schedules, and calculates metrics.
- `balancer`: allocates resources by priority, calculates efficiency, detects imbalance, and suggests transfers.

The service accepts runtime operations such as `coordinate`, `balance`, `detect_conflicts`, and `get_state`.

Its coordination-channel subscription currently adds incoming messages to service history; it does not automatically turn `UnitRegistered` into a schedule or balancing action.

## 12. System 3: control

System 3 has one `ServiceActor`, `vsm.system3.control`, plus:

- `resources`: request prioritization, resource allocation, availability, optimization, prediction, and validation.
- `audit`: unit audits, audit scheduling, pattern analysis, and report generation.

The service accepts `allocate_resources`, `audit`, and `get_state` operations.

It subscribes to ResourceBargain, Command, and Audit channels. Those messages are recorded in its history but are not automatically translated into allocation, control, or audit calls.

## 13. System 4: intelligence

System 4 contains four separately supervised `ServiceActor` instances:

- `vsm.system4.intelligence`
- `vsm.system4.scanner`
- `vsm.system4.analytics`
- `vsm.system4.forecasting`

The modules provide:

- environmental source scanning and signal classification
- change and trend detection
- summary, trend, correlation, anomaly, and insight analysis
- linear/mean/naive forecasting, scenarios, and validation

The Intelligence service can orchestrate the pure scanner, analytics, and forecasting functions. It does not send RPCs to the other three service actors when doing so; it calls their module functions directly. The independently supervised scanner, analytics, and forecasting actors remain useful as separate API/state boundaries.

Only the Intelligence actor subscribes to Command messages, and current `ServiceActor` behavior records those messages without invoking an intelligence operation.

## 14. System 5: policy and identity

System 5 contains four separately supervised service actors:

- Policy
- Identity
- Values
- Decisions

The Policy service can set policy, set identity, define values, make decisions, evaluate alignment, handle crises, and return its organizational state.

### State isolation inside System 5

Each actor owns an independent `ServiceState`. Calling the standalone Identity actor does not mutate the Policy actor's state. Likewise, a decision stored by the standalone Decisions actor is not automatically visible in Policy state.

The Policy module calls the identity, values, and decisions **functions using Policy's own state**. For a coherent single organizational state, use the Policy actor as the aggregate boundary. Use the standalone actors only when deliberately maintaining independent state or when building a higher-level synchronization layer.

System 5 Policy subscribes to the algedonic channel. Received messages are currently recorded in its history; they do not automatically invoke `handle_crisis`.

## 15. Algedonic architecture

There are two related but distinct paths.

### 15.1 VSM algedonic channel

`system1::send_algedonic_signal()` and `channels::algedonic_channel` create `VsmMessage` values and route them through the broker to subscriber ID `system5`. This path reaches the Policy service's channel history.

### 15.2 Advanced algedonic processor

`channels::algedonic::Algedonic` is a separate typed actor. Its API creates `AlgedonicSignal` values with severity, urgency, priority, kind, source, and context. The actor:

1. applies filters
2. chooses a route description
3. records an alert
4. stores the active signal and route
5. updates accepted/rejected counters

The calculated route is descriptive in the current implementation. It does not publish a `VsmMessage` to the channel broker or invoke the destination actor.

Alert history is held in a process-global `Mutex<Vec<AlertRecord>>`; active signals and metrics live in the actor state.

## 16. Temporal variety architecture

`TemporalVariety` is a dedicated typed actor with:

- a raw buffer capped at 10,000 values
- per-timescale buffers capped by `max_points`, default 1,000
- pattern, forecast, and causality fields

Default timescale names are:

```text
instant, minute, hour, day
```

Callers explicitly record measurements and query variety, patterns, forecasts, causality, summaries, or visualization data. The actor defines internal maintenance messages such as `AnalyzePatterns` and `UpdateForecasts`, but no timer currently schedules them. Query methods calculate results on demand.

The broker's `TemporalVariety` channel is separate from this typed actor and has no default subscriber.

## 17. Shared pure modules

The modules below do not require application startup unless they call an actor facade:

### Variety engineering

- `shared::variety::calculator`: variety, entropy, comparison, summaries
- `shared::variety::attenuator`: filter, aggregate, summarize
- `shared::variety::amplifier`: delegate, empower, multiply, distribute, parallelize
- `shared::variety_engineering`: select attenuation/amplification recommendations

### Recursive-system structure

`shared::recursion` models nested viable systems as a tree of `RecursionLevel` values. It supports creation, navigation, context updates, tree rendering, metrics, validation, pruning, and merging.

### Temporal analysis

`channels::temporal` provides pure timescale, pattern, forecast, causality, aggregation, and visualization helpers used by the TemporalVariety actor.

## 18. State, persistence, and recovery

All state is currently process-local and in memory. There is no database, event log, snapshot store, or distributed registry.

| State | Owner | Retention | Lost on actor restart? |
|---|---|---:|---|
| Channel subscriptions | Broker | Until unsubscribe/send failure | Yes |
| Channel history | Broker | 10,000/channel | Yes |
| System 1 unit directory | Operations | Unbounded IDs | Yes |
| System 1 metrics | Operations | Lifetime totals | Yes |
| System 1 variety | Operations | 1,000 | Yes |
| Unit state | Unit actor | Current value | Yes |
| Generic service history | ServiceActor | 1,000 | Yes |
| Algedonic active signals | Algedonic actor | 1,000 entries in current implementation | Yes |
| Alert history | Process-global static | Approximately 10,000 | No actor restart; yes process exit |
| Temporal raw buffer | TemporalVariety actor | 10,000 | Yes |

### Recovery gaps to understand

The supervision structure restarts actors, but some relationships are not yet rebuilt automatically:

- If the channel broker restarts alone, live subscribers do not automatically re-register.
- If System 1 Operations restarts alone, it loses its unit directory even if unit actors remain alive.
- If the System 1 unit supervisor restarts alone, Operations retains its old supervisor reference.
- Generic service state resets to startup configuration after restart.

For production use, add durable configuration/state, actor discovery/reconciliation, and subscription re-registration.

## 19. Concurrency and request semantics

Each actor processes its mailbox serially. Mutable state does not need external locking when it is actor-owned.

The public API uses two communication modes:

- **RPC calls** using `call_t!`: wait for a reply and apply a timeout.
- **casts/messages** using `send_message`: enqueue and return without domain acknowledgment.

Current timeout values include:

| API | Timeout |
|---|---:|
| Generic `call_service` | 5 seconds |
| System 1 register unit | 5 seconds |
| System 1 process transaction | 10 seconds |
| System 1 metrics/variety/list | 2 seconds |
| Unit capability/load/status queries | 1 second |
| Algedonic queries | 1 second |
| Temporal variety queries | 1 second |
| Broker subscribe/stats/history | 2 seconds |

Long-running work should not execute directly inside an actor handler without careful design, because it blocks subsequent messages to that actor. Delegate slow work to task actors, worker actors, or external async services and report results back.

## 20. Observability

The crate uses `tracing` for startup, warnings, high-priority messages, registration, and selected operations.

Runtime inspection APIs include:

- `vsm_core::health()`
- `vsm_core::status()`
- `vsm_core::subsystem_state()`
- `channels::stats()`
- `channels::history()`
- `system1::get_metrics()`
- `system1::get_variety()`
- `channels::algedonic::get_metrics()`
- `channels::temporal_variety::*`

The telemetry reporter is currently a generic service actor returning its data and event-history length. It does not yet emit external metrics or schedule periodic reporting.

## 21. Current cross-system flows

### Unit registration

```text
System 1 registers unit
  -> Coordination/UnitRegistered message
  -> System 2 receives and records channel event
```

No automatic System 2 scheduling action follows yet.

### Successful operational transaction

```text
Caller -> System 1 Operations -> selected Unit -> result
       -> metrics + variety updated
```

### Missing capability

```text
Caller -> System 1 Operations
       -> no matching unit
       -> ResourceBargain/UnitRequest to System 3
       -> System 3 records channel event
       -> caller receives NoSuitableUnit
```

No automatic resource allocation or unit creation follows yet.

### Command

```text
S3/S4/S5 -> Command/Execute -> System 1 Operations -> every registered Unit
```

The demo Unit recognizes a `status` field and updates its status.

### Audit

```text
S3 or S3* -> Audit/AuditRequest -> System 1 Operations
          -> Audit/AuditResponse -> target/fallback channel delivery
```

### Algedonic escalation

```text
System 1 -> broker Algedonic message -> System 5 Policy history
```

This does not automatically call `handle_crisis`.

## 22. Extension strategy

### Prefer typed actors for stable protocols

When a service has a known protocol or meaningful state transitions, define:

1. a message enum
2. a state struct
3. an actor implementation
4. typed public facade functions
5. a `ChildSpec` under the appropriate supervisor

System 1 is the reference pattern.

### Use `ServiceActor` for exploratory or integration-facing behavior

The shared service shell is useful when payloads are intentionally dynamic or the operation set is still evolving. Wrap string operations in typed Rust functions as the interface stabilizes.

### Adding a channel subscriber

A subscriber must:

1. include a `VsmActorMsg` variant in its own message enum
2. implement `From<VsmActorMsg>` and `TryFrom<OwnMsg> for VsmActorMsg`
3. subscribe with `myself.get_derived::<VsmActorMsg>()`
4. unsubscribe during `post_stop`

### Adding a channel

A new channel requires coordinated changes to:

- `ChannelKind`
- `ChannelKind::ALL`
- string conversion if needed
- validation rules
- broker initialization
- convenience facade modules
- subscribers and tests

### Replacing the demo Unit

For real operations, introduce a domain-specific unit actor or a unit behavior abstraction. Keep `Operations` responsible for registration, routing, and governance, and keep domain execution inside unit actors.

## 23. Architectural limitations

The most important current limitations are:

- The crate is still in baseline hardening; see `CODEX.md` for the latest
  validation evidence.
- Most Systems 2–5 APIs use string operation names and `serde_json::Value`.
- Channel events for Systems 2–5 are recorded, not converted into domain actions.
- Broker restart loses registrations and history.
- Application readiness has no explicit barrier.
- State is in-memory and restart-volatile.
- System 5's four actors have independent state stores.
- The root dynamic supervisor is present for parity but is not exposed by a public child-management API.
- There is no System 1 unit unregister/update API.
- Some algorithms are intentionally lightweight starter implementations rather than production statistical or optimization models.
- Algedonic route calculation does not perform actor delivery.
- Temporal maintenance messages are not scheduled automatically.

These are useful boundaries for the next architectural iteration: typed protocols, durable state, subscription reconciliation, event-driven subsystem handlers, and explicit readiness/lifecycle management.
