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
| System 4 | Typed environmental sources, intelligence, forecasting, scenarios, proposals | `src/system4/`, `src/protocol/system4.rs`, `src/roles/system4.rs`, `src/kernel/system4.rs` |
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
├── config.rs                 Typed runtime configuration
├── builder.rs                Typed runtime builder
├── runtime.rs                Typed runtime handles, readiness, shutdown, component snapshots
├── kernel/                   Private runtime registry, observer bus, and typed System 1-5 actor adapters
├── protocol/                 Typed foundations: addresses, metadata, snapshots, bus outcomes, events, System 1-5 records
├── roles/                    ViableSystem type family, role contexts, System 1-5 contracts, runtime ports
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
├── system2/                  Typed coordination defaults and legacy supervisor placeholder
├── system3/                  Typed defaults and legacy supervisor placeholder
├── system4/                  System 4 prototype defaults and legacy supervisor placeholder
└── system5/                  System 5 prototype defaults and legacy supervisor placeholder
```

A porting map for the original Elixir files is not currently present in this
repository. Recreate it before relying on file-by-file correspondence during a
future migration slice.

### 2.1 Trait-driven foundation modules

The crate now includes public migration foundations under `protocol`, `roles`,
`cancellation`, `config`, `builder`, `runtime`, and `legacy`.

These modules are intentionally alongside the current runtime:

- `roles::ViableSystem` defines the minimal application type family.
- `roles::RoleContext` and `roles::UnitRoleContext` expose runtime identity,
  recursion path, correlation/deadline metadata, cooperative cancellation,
  clock, no-op/default event and report sinks, and explicitly allowed state
  storage to application roles.
- `roles::system1` defines first-wave role contracts for `OperationalUnit`,
  `OperationalUnitFactory`, `WorkModel`, `UnitSelectionPolicy`,
  `PerformanceModel`, `VarietyModel`, `AlgedonicPolicy`, and the
  `System1Roles` role catalog.
- `protocol::address`, `protocol::envelope`, `protocol::snapshot`, and
  `protocol::bus` define instance-scoped runtime metadata, typed control-message
  families, and delivery-status records.
- `protocol::system1` defines typed System 1 records for work, unit descriptors,
  capacity/load, command acknowledgements, performance observations, resource
  shortages, audit evidence, and coordination views.
- `protocol::system2` defines typed System 2 coordination view records,
  conflicts, interventions, acknowledgements, escalation records, cycles, and
  runtime snapshots.
- `protocol::system3` defines typed System 3 resource requests, allocation
  decisions, operational directives, acknowledgements, operational summaries,
  System 3* audit requests, findings, remediations, responses, and snapshots.
- `protocol::system4` defines typed System 4 source descriptors, observations,
  interpreted signals, intelligence assessments, forecasts, scenarios,
  calibration records, adaptation proposals, and runtime snapshots.
- `protocol::system5` defines typed System 5 identity, values, decision,
  evidence, directive, acknowledgement, crisis, escalation, and snapshot
  records.
- `roles::system2` defines the view-centric `CoordinationPolicy` role plus the
  no-op default policy.
- `roles::system3` defines `ResourceGovernance`, `OperationalControlPolicy`,
  and `Auditor` roles plus explicit defaults.
- `roles::system4` defines environmental source factory/source, signal
  interpretation, intelligence model, and forecasting/scenario/proposal roles
  plus no-op defaults.
- `roles::system5` defines identity provider, values provider, values
  evaluator, decision policy, and crisis policy roles plus no-op defaults.
- `roles::ports` defines `StateStore`, `EventSink`, and `ReportSink`, plus
  no-op implementations. It also defines early `TelemetrySink`, `AlertSink`,
  `Clock`, and `IdGenerator` ports for role contexts and future adapters.
  `NoopStateStore` is not persistent.
- `cancellation::CancellationToken` is the crate-owned cooperative cancellation
  primitive for future role contexts.
- `VsmBuilder` builds an instance-scoped `VsmRuntime` handle from required
  `WorkModel` and `OperationalUnitFactory` role objects plus opt-in default
  policies and no-op ports.
- `runtime` defines readiness checks, shutdown acknowledgements, a System 1
  handle, typed unit registration/work APIs, and private component-directory
  snapshots. The directory generates internal component names from `RuntimeId`
  and `RecursionPath` rather than the current process-global actor names.
- `kernel::system1` contains private actor adapters for the typed System 1
  runtime. Registered units own application `OperationalUnit` implementations
  behind private actors; the public handle owns orchestration and never exposes
  actor references.
- `kernel::system2` contains the private typed coordination actor. It stores
  System 1 coordination views with freshness/version metadata, invokes
  `CoordinationPolicy`, tracks interventions and acknowledgements, and records
  unresolved-conflict escalations for System 3.
- `kernel::system3` contains private typed System 3 control and System 3* audit
  actors. Control invokes resource governance and operational-control roles,
  tracks authority/version/expiry/acknowledgement records, and delivers
  directives through private System 1 unit adapters. Audit invokes the
  application `Auditor` with evidence collected through a separate System 1
  audit path.
- `kernel::system4` contains the private typed System 4 intelligence actor and
  source actors. It dynamically registers environmental sources, normalizes
  observations with provenance/confidence/freshness metadata, restarts failing
  source roles, invokes intelligence/forecasting roles, records calibration
  results, and annotates adaptation proposals with System 3 feasibility context.
- `kernel::system5` contains the private typed System 5 policy actor. It
  invokes identity/value providers, values evaluation, decision policy, and
  crisis policy roles, records decision audit trails, emits directives and
  acknowledgements, and retains typed crisis/escalation records.
- `kernel::event_bus` contains the private observer event bus used by typed
  runtime handles. It implements the `EventSink` port, fans out runtime events
  to subscribers without blocking the control path, retains a bounded
  newest-first in-memory event history, and forwards events to the configured
  downstream sink.
- `legacy::system1` contains temporary adapters for current
  `Transaction`/`TransactionResult`/`UnitConfig`/`VsmMessage` shapes.

The foundation modules do not expose `ActorRef`, actor names, or `ractor`
message types. Downstream role implementations use the crate's re-exported
`async_trait` macro rather than importing `ractor`. Core typed protocol records
and role contracts do not require application work, outcome, error, capability,
unit ID, or snapshot payloads to implement serde.

`VsmBuilder` starts a typed runtime path: it validates required System 1 role
objects, reports readiness deterministically, exposes scoped role contexts,
permits multiple runtime handles in one process, registers typed operational
units, dispatches typed work through private unit actors, supports typed System
2 coordination, supports typed System 3 governance/audit, supports typed System
4 environmental intelligence, supports typed System 5 decisions/crises,
supports typed observer-event subscriptions, and acknowledges shutdown. The
existing global actor runtime still serves the legacy `Transaction`/JSON
facade.

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
│   └── no legacy JSON children; typed System 2 runs under VsmRuntime
├── vsm.system3.supervisor
│   └── no legacy JSON children; typed System 3 runs under VsmRuntime
├── vsm.system4.supervisor
│   └── no legacy JSON children; typed System 4 runs under VsmRuntime
├── vsm.system5.supervisor
│   └── no legacy JSON children; typed System 5 runs under VsmRuntime
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

Telemetry and auxiliary services use `actor_support::ServiceActor`. Systems 2,
3, 4, and 5 have moved to typed runtime actors and no longer use this JSON
service shell.

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
ServiceKind::TemporalVariety      -> channels::temporal_variety::actor_call
ServiceKind::TelemetryReporter    -> telemetry reporter status shell
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
  "id": "telemetry",
  "config": {
    "subsystem": "telemetry"
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
HashMap<ChannelKind, Vec<UndeliverableMessage>>
```

Each channel retains up to 10,000 messages, newest first.

### 7.1 Subscription identity

A subscription is keyed by `(ChannelKind, subscriber_id)`. Registering the same ID again replaces the previous subscriber. This is not identical to an Elixir duplicate registry, which can retain multiple processes under the same key.

Use unique IDs when multiple listeners need the same channel. For targeted subsystem routing, use the IDs returned by `SystemId::subscriber_id()`, such as `system1`, `system2`, or `system5`.

### 7.2 Publish routing

For `ChannelBrokerMsg::Publish`:

1. The broker validates the `VsmMessage`.
2. High-priority message kinds are logged.
3. Legacy algedonic messages target subscriber ID `system5`; no built-in typed
   System 5 actor subscribes there in this milestone.
4. Other messages target `message.to.subscriber_id()`.
5. If the target does not exist or the subscriber reference is unavailable, the
   broker records a `TargetUnavailable` outcome and stores the message in
   dead-letter history.
6. Delivered messages are retained in channel history.

`channels::publish_with_outcome()` performs the same path and returns the
broker delivery outcome. `channels::publish()` remains a legacy enqueue helper:
it reports only whether the broker mailbox accepted the message.

### 7.3 Explicit broadcast

`channels::broadcast()` sends to every current subscriber on a channel only when
the message target is `SystemId::All`. The broadcast path uses the same broker
validation boundary as targeted publish. Rejected broadcasts are recorded in
dead-letter history rather than retained as delivered messages.

### 7.4 Delivery semantics

Channel delivery is:

- in-process
- asynchronous
- explicit at the broker boundary when using outcome-returning APIs
- not durable
- not acknowledged by recipients after actor mailbox delivery

`channels::publish_with_outcome()` and `channels::broadcast_with_outcome()`
return `Delivered`, `TargetUnavailable`, or `RejectedByProtocol` for the current
broker implementation. Delivery means the recipient actor mailbox accepted the
message, not that the recipient processed it. `ChannelStats` includes delivery
metrics and dead-letter counts; `channels::dead_letters()` returns the retained
undeliverable records.

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

The dedicated algedonic processor and temporal-variety actor are accessed through their typed APIs; they do not subscribe to the broker in the current supervision tree.

System 1 has explicit legacy channel behavior. Typed algedonic lifecycle
handling is available through `VsmRuntime::variety()`, including bridge helpers
for legacy broker `VsmMessage` values and advanced algedonic actor signals.
High-priority typed algedonic records are dispatched into the typed System 5
crisis path.

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

When no suitable unit exists, legacy Operations publishes a
`ResourceBargain/UnitRequest` message addressed to System 3 and returns
`TransactionResult::NoSuitableUnit`. The legacy System 3 JSON subscriber has
been removed, so that message is recorded as target-unavailable by the broker.
Typed resource-shortage handling is available through `VsmRuntime::system3()`.

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

System 2 is now owned by the typed runtime path rather than the global JSON
service shell. `VsmRuntime::system2()` returns a handle that:

- collects typed System 1 `CoordinationView` records;
- stores freshness and monotonic view versions per unit;
- invokes the configured `CoordinationPolicy`;
- delivers typed `CoordinationIntervention` values to affected units;
- records unit acknowledgements and rejected/failed intervention escalations
  for System 3.

The previous JSON `vsm.system2.coordination` service actor is not started by the
legacy global supervisor. The old scheduler and balancer helpers live under
`system2::defaults` as explicit example algorithms and are not core System 2
semantics.

## 12. System 3: control and audit

Typed System 3 runs under `VsmRuntime`, not the legacy global service tree. It
has two private actor adapters:

- a System 3 control actor for resource governance, operational directives,
  directive acknowledgements, authority/version metadata, and operational
  summaries;
- a System 3* audit actor for authorized audit requests, evidence boundaries,
  findings, remediation proposals, and audit responses.

Applications provide `ResourceGovernance`, `OperationalControlPolicy`, and
`Auditor` role implementations. The default resource governance role denies
requests explicitly, the default control policy emits no directives, and the
default auditor returns an empty response.

The old `vsm.system3.control` JSON service actor is not started by the legacy
global supervisor. Former JSON resource and audit helper algorithms live under
`system3::defaults` as opt-in examples and are not core System 3 semantics.

## 13. System 4: intelligence

System 4 is available through `VsmRuntime::system4()`. It is a typed
environmental-intelligence pipeline rather than a JSON service family.

The public boundary includes:

- `protocol::system4` framework-owned source, observation, signal, assessment,
  forecast, scenario, calibration, proposal, and snapshot records.
- `roles::system4` environmental source factory/source, signal interpreter,
  intelligence model, and forecaster contracts.
- `runtime::System4Handle` methods for source registration, source listing,
  observation collection, intelligence cycles, forecast calibration, and
  snapshots.

The private `kernel::system4` adapter owns one intelligence actor plus source
actors created from the configured source factory. A source observation failure
recreates that source role instance and records the error without stopping the
whole System 4 runtime. Observations carry provenance, confidence, timestamps,
and freshness status. Intelligence cycles emit typed events/reports and route
adaptation proposals toward System 5 with System 3 feasibility context.

The old scanner, analytics, and forecasting heuristics are retained only as
opt-in prototype helpers under `system4::defaults`.

## 14. System 5: policy and identity

System 5 is part of the typed runtime handle. It provides:

- `protocol::system5` framework-owned identity, values, policy version,
  evidence, decision, directive, acknowledgement, crisis, escalation, and
  snapshot records.
- `roles::system5` identity provider, values provider, values evaluator,
  decision policy, and crisis policy contracts.
- `runtime::System5Handle` methods for identity, values, decision cycles,
  directive acknowledgement, crisis handling, algedonic crisis handling, and
  snapshots.

The private `kernel::system5` adapter owns one policy actor. It invokes
application-owned provider and policy roles, attaches System 3 operational
summaries and System 4 adaptation proposals to decision requests, records typed
decision audit trails, emits typed directives/reports/events, tracks
acknowledgements, and stores crisis/escalation records in memory.

The crate no longer imposes default mission text, decision values, weighted
scoring, keyword alignment, or generic crisis directives in the core runtime.
Prototype JSON helpers are retained only as opt-in examples under
`system5::defaults`.

## 15. Algedonic architecture

There are three related but distinct paths.

### 15.1 Typed variety, algedonic, and temporal lifecycle

`VsmRuntime::variety()` exposes the typed lifecycle adapter for:

- variety observations, estimates, interventions, and outcomes;
- algedonic signal classification, acknowledgement, expiry, escalation, alert
  delivery, and System 5 crisis dispatch;
- temporal samples, generic aggregates, and replaceable temporal analyses.

The public role contracts are `VarietyEngineeringPolicy`,
`AlgedonicLifecyclePolicy`, and `TemporalAnalysisPolicy`. Defaults are opt-in
and deliberately minimal: no-op variety engineering, basic algedonic
classification, and no-op temporal analysis.

### 15.2 VSM algedonic channel

`system1::send_algedonic_signal()` and `channels::algedonic_channel` create
`VsmMessage` values and route them through the legacy broker. The typed
`VarietyHandle::handle_legacy_algedonic_message` bridge converts those messages
into typed algedonic lifecycle records when callers opt into the trait-driven
runtime path.

### 15.3 Advanced algedonic processor

`channels::algedonic::Algedonic` is a separate typed actor. Its API creates `AlgedonicSignal` values with severity, urgency, priority, kind, source, and context. The actor:

1. applies filters
2. chooses a route description
3. records an alert
4. stores the active signal and route
5. updates accepted/rejected counters

The calculated route is descriptive in the current implementation. It does not publish a `VsmMessage` to the channel broker or invoke the destination actor.

Alert history, active signals, routes, and metrics live in the actor state.

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
Typed runtime registers unit
  -> System 2 collects typed CoordinationView records from System 1
  -> CoordinationPolicy detects conflicts and proposes typed interventions
  -> System 1 units acknowledge interventions
```

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
       -> broker records target unavailable on the legacy channel
       -> caller receives NoSuitableUnit
```

Typed resource-shortage handling is available through the typed System 3
runtime handle; the legacy channel path does not automatically allocate or
create units.

### Command

```text
S3/S4/S5 -> Command/Execute -> System 1 Operations -> every registered Unit
```

The demo Unit recognizes a `status` field and updates its status.

### Audit

```text
S3 or S3* -> Audit/AuditRequest -> System 1 Operations
          -> Audit/AuditResponse -> targeted delivery or dead letter
```

### Algedonic escalation

```text
Typed caller -> VarietyHandle::handle_algedonic_signal -> System5 crisis path -> CrisisPolicy
Legacy broker message -> VarietyHandle::handle_legacy_algedonic_message -> typed algedonic lifecycle
Advanced algedonic signal -> VarietyHandle::handle_advanced_algedonic_signal -> typed algedonic lifecycle
```

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
- Legacy broker algedonic messages are converted into the typed lifecycle only
  when callers pass them through `VarietyHandle::handle_legacy_algedonic_message`;
  the legacy broker itself remains non-durable.
- Broker restart loses registrations and history for the legacy global actor
  facade. The typed runtime handle's observer subscriptions are owned by the
  handle, not the legacy broker.
- The legacy global actor facade has no explicit readiness barrier. The typed
  runtime handle reports readiness gates.
- State is in-memory and restart-volatile.
- The root dynamic supervisor is present for parity but is not exposed by a public child-management API.
- There is no System 1 unit unregister/update API.
- Some algorithms are intentionally lightweight starter implementations rather than production statistical or optimization models.
- Algedonic route calculation does not perform actor delivery.
- Temporal maintenance messages are not scheduled automatically.

These are useful boundaries for the next architectural iteration: typed protocols, durable state, subscription reconciliation, event-driven subsystem handlers, and explicit readiness/lifecycle management.
