# Usage

This guide explains how to embed, start, call, observe, extend, and shut down the current `vsm-rs` library.

The package name is `vsm-rs`; Rust imports use the crate name `vsm_rs`.

> **Build status:** see [`../CODEX.md`](../CODEX.md) for the latest validation evidence. Run the full validation suite locally before treating this port as production-ready.

## 1. Add the crate to a project

For a local path dependency:

```toml
[dependencies]
vsm-rs = { path = "../vsm-rs" }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }
```

The port currently pins compatible actor versions internally:

```toml
ractor = { version = "0.14.3", features = ["async-trait"] }
ractor-supervisor = "0.1.9"
```

Build and test:

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features --locked
cargo test --all-targets --all-features --locked
cargo run --example basic_usage --locked
```

## 2. Start and stop the application

The application is a singleton within one process because all actors use global names.

```rust
use tracing_subscriber::EnvFilter;
use vsm_rs::{start, VsmApplication};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("vsm_rs=info".parse()?),
        )
        .init();

    let VsmApplication {
        supervisor,
        join_handle,
    } = start().await?;

    // Use the VSM here.

    supervisor.stop(Some("application shutdown".to_string()));
    let _ = join_handle.await;
    Ok(())
}
```

`vsm_core::stop()` is also available:

```rust
vsm_rs::stop().await?;
```

It sends a stop request to the root supervisor but does not wait for shutdown. Retain the `join_handle` returned at startup when graceful completion matters.

### Do not start it twice

A second startup in the same process will collide with names such as `vsm.root_supervisor` and `vsm.channels.broker`. Start once, share the APIs, and stop once.

Tests that start the application should be serialized. The included tests use `serial_test`.

## 3. Wait for child readiness

`start()` returns after spawning the root supervisor. The included examples use a small sleep because the crate does not yet expose a formal readiness barrier.

A polling helper is safer:

```rust
use std::time::Instant;
use tokio::time::{sleep, Duration};
use vsm_rs::{VsmError, VsmResult};

async fn wait_until_ready() -> VsmResult<()> {
    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        let broker_ready = vsm_rs::channels::broker_ref().is_ok();
        let operations_ready = vsm_rs::system1::operations_ref().is_ok();

        if broker_ready && operations_ready {
            return Ok(());
        }

        if Instant::now() >= deadline {
            return Err(VsmError::Runtime(
                "VSM children did not become ready in time".to_string(),
            ));
        }

        sleep(Duration::from_millis(20)).await;
    }
}
```

`require_running()` checks only that the root supervisor is registered. It is not a complete child-readiness test.

## 4. Which API should you use?

Use interfaces in this order:

1. **Typed subsystem facade**, such as `system1::process_transaction`.
2. **Typed dedicated service API**, such as `channels::temporal_variety::get_patterns`.
3. **Channel facade**, when the interaction is an asynchronous VSM event.
4. **`actor_support::call_service`**, when using a Systems 4–5 JSON service operation that has no typed wrapper.
5. **Direct pure function**, when no actor state or supervision is needed.

The typed facade gives the strongest compile-time guarantees. Generic service calls are intentionally flexible but operation and payload mistakes become runtime behavior.

### 4.1 Typed migration foundations

The crate also exposes early trait-driven foundations for the approved migration:

- `ViableSystem` for the minimal application type family.
- `protocol` records for instance-scoped metadata, snapshots, events, reports,
  typed bus delivery statuses, and typed System 1 messages.
- `roles::RoleContext` and `roles::UnitRoleContext` for framework identity,
  correlation/deadline metadata, cancellation, clock access, event/report
  emission, and explicitly allowed state storage.
- `roles::system1` contracts for `OperationalUnit`, `OperationalUnitFactory`,
  `WorkModel`, `UnitSelectionPolicy`, `PerformanceModel`, `VarietyModel`,
  `AlgedonicPolicy`, and `System1Roles`.
- `roles::system2` contracts for `CoordinationPolicy` and `System2Roles`.
- `roles::system3` contracts for `ResourceGovernance`,
  `OperationalControlPolicy`, `Auditor`, and `System3Roles`.
- `roles::system1::defaults` for opt-in lowest-load selection and no-op
  performance, variety, and algedonic policies.
- `roles::system2::defaults` for the no-op coordination policy.
- `roles::system3::defaults` for deny-all resource governance, no-op
  operational control, and no-op audit.
- `roles::system1::testing` for downstream-style test fakes.
- `roles::ports` for `StateStore`, `EventSink`, `ReportSink`, `TelemetrySink`,
  `AlertSink`, `Clock`, and `IdGenerator`.
- `cancellation::CancellationToken` for cooperative role cancellation.
- `legacy::system1` adapters for current `Transaction`, `TransactionResult`,
  `UnitConfig`, and `VsmMessage` shapes.
- `VsmBuilder`, `RuntimeConfig`, and `VsmRuntime` for instance-scoped typed
  runtime handles, readiness checks, System 1 role access, typed observer-event
  subscriptions, and shutdown acknowledgement.

These types are public so downstream-style code can compile against the future
boundary. `VsmBuilder` now starts actor-backed typed System 1, System 2, and
System 3 paths for unit registration, work processing, coordination,
governance/audit, and observer-event delivery; the current global
actor/JSON-backed runtime continues to serve the legacy transaction facade and
Systems 4–5 service APIs.

Application role implementations should import `vsm_rs::async_trait` when
implementing async role methods. They should not need to import `ractor`,
`ActorRef`, global actor names, broker message types, or `serde_json` for the
core role contracts.

### 4.2 Start a typed runtime handle

Use `VsmBuilder` to run application role implementations against the
trait-driven runtime boundary. The builder requires a `WorkModel` and an
`OperationalUnitFactory`; optional System 1 policies default to lowest-load
selection and no-op performance, variety, and algedonic policies.

```rust
use vsm_rs::protocol::system1::{CapacitySnapshot, UnitDescriptor};
use vsm_rs::protocol::RuntimeId;
use vsm_rs::roles::system1::testing::{
    AcceptAllWorkModel, StaticOperationalUnitFactory,
};
use vsm_rs::VsmBuilder;

# #[derive(Clone, Debug)]
# struct ExampleWork;
# #[derive(Clone, Debug)]
# struct ExampleOutcome;
# #[derive(Debug)]
# struct ExampleError;
# impl std::fmt::Display for ExampleError {
#     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
#         f.write_str("example error")
#     }
# }
# impl std::error::Error for ExampleError {}
# #[derive(Clone, Debug, PartialEq, Eq, Hash)]
# struct ExampleCapability(&'static str);
# #[derive(Clone, Debug, PartialEq, Eq, Hash)]
# struct ExampleUnitId(&'static str);
# struct ExampleSnapshot;
# struct ExampleSystem;
# impl vsm_rs::ViableSystem for ExampleSystem {
#     type Work = ExampleWork;
#     type Outcome = ExampleOutcome;
#     type AppError = ExampleError;
#     type Capability = ExampleCapability;
#     type UnitId = ExampleUnitId;
#     type UnitSnapshot = ExampleSnapshot;
# }
# async fn build() -> Result<(), vsm_rs::FrameworkError> {
let descriptor =
    UnitDescriptor::<ExampleSystem>::new(ExampleUnitId("unit-a"), [ExampleCapability("work")]);

let runtime = VsmBuilder::new()
    .runtime_id(RuntimeId::from_string("example-runtime"))
    .work_model(AcceptAllWorkModel::new([ExampleCapability("work")]))
    .operational_unit_factory(StaticOperationalUnitFactory::new(
        descriptor.clone(),
        CapacitySnapshot::new(0, Some(4), 0.0),
        ExampleOutcome,
    ))
    .start()
    .await?;

assert!(runtime.is_ready());
runtime.system1().register_descriptor(descriptor).await?;
let _outcome = runtime.system1().process_work(ExampleWork).await?;
runtime.shutdown().await?;
# Ok(())
# }
```

Subscribe observers through the runtime handle when application code needs the
typed event stream without participating in control flow:

```rust
# use vsm_rs::protocol::system1::{CapacitySnapshot, UnitDescriptor};
# use vsm_rs::protocol::{RuntimeEvent, RuntimeId};
# use vsm_rs::roles::system1::testing::{
#     AcceptAllWorkModel, StaticOperationalUnitFactory,
# };
# use vsm_rs::VsmBuilder;
# #[derive(Clone, Debug)]
# struct ExampleWork;
# #[derive(Clone, Debug)]
# struct ExampleOutcome;
# #[derive(Debug)]
# struct ExampleError;
# impl std::fmt::Display for ExampleError {
#     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
#         f.write_str("example error")
#     }
# }
# impl std::error::Error for ExampleError {}
# #[derive(Clone, Debug, PartialEq, Eq, Hash)]
# struct ExampleCapability(&'static str);
# #[derive(Clone, Debug, PartialEq, Eq, Hash)]
# struct ExampleUnitId(&'static str);
# struct ExampleSnapshot;
# struct ExampleSystem;
# impl vsm_rs::ViableSystem for ExampleSystem {
#     type Work = ExampleWork;
#     type Outcome = ExampleOutcome;
#     type AppError = ExampleError;
#     type Capability = ExampleCapability;
#     type UnitId = ExampleUnitId;
#     type UnitSnapshot = ExampleSnapshot;
# }
# async fn observe() -> Result<(), vsm_rs::FrameworkError> {
# let descriptor =
#     UnitDescriptor::<ExampleSystem>::new(ExampleUnitId("unit-a"), [ExampleCapability("work")]);
# let runtime = VsmBuilder::new()
#     .runtime_id(RuntimeId::from_string("observer-runtime"))
#     .work_model(AcceptAllWorkModel::new([ExampleCapability("work")]))
#     .operational_unit_factory(StaticOperationalUnitFactory::new(
#         descriptor.clone(),
#         CapacitySnapshot::new(0, Some(4), 0.0),
#         ExampleOutcome,
#     ))
#     .start()
#     .await?;
let mut events = runtime.subscribe_events("audit-ui")?;
runtime.system1().register_descriptor(descriptor).await?;

if let Some(RuntimeEvent::System1(event)) = events.recv().await {
    let _ = event;
}
# runtime.shutdown().await?;
# Ok(())
# }
```

A runnable version is available in
[`examples/typed_runtime_builder.rs`](../examples/typed_runtime_builder.rs).

## 5. Complete minimal example

```rust
use serde_json::json;
use tokio::time::{sleep, Duration};
use vsm_rs::system1::{self, Transaction, TransactionResult, UnitConfig};
use vsm_rs::VsmApplication;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let VsmApplication {
        supervisor,
        join_handle,
    } = vsm_rs::start().await?;

    // Replace with the readiness helper in long-running applications.
    sleep(Duration::from_millis(100)).await;

    system1::register_unit(UnitConfig::new(
        "payments",
        ["payment", "card", "settlement"],
    ))
    .await?;

    let result = system1::process_transaction(Transaction::new(
        "payment_authorization",
        vec!["payment".into(), "card".into()],
        json!({
            "amount": 42.50,
            "currency": "USD",
            "card_token": "tok_demo"
        }),
    ))
    .await?;

    match result {
        TransactionResult::Ok(output) => println!("processed: {output:#}"),
        other => println!("not processed: {other:#?}"),
    }

    println!("metrics: {:#?}", system1::get_metrics().await?);
    println!("variety: {:#?}", system1::get_variety().await?);
    println!("status: {:#}", vsm_rs::status().await?);

    supervisor.stop(Some("done".to_string()));
    let _ = join_handle.await;
    Ok(())
}
```

## 6. System 1 operations

System 1 is the primary typed operational API.

### 6.1 Register an operational unit

```rust
use serde_json::json;
use vsm_rs::system1::{self, UnitConfig};

let mut config = UnitConfig::new(
    "payments-eu",
    ["payment", "card", "settlement"],
);
config.auto_restart = true;
config.metadata = json!({
    "region": "eu-west",
    "owner": "payments-team"
});

let unit_id = system1::register_unit(config).await?;
assert_eq!(unit_id, "payments-eu");
```

Unit IDs must be unique. The actor name is derived as:

```text
vsm.system1.unit.<unit-id>
```

`UnitConfig::new()` defaults `auto_restart` to `true` and `metadata` to `null`.

There is currently no public unregister or update-unit API. Restart the application or extend `OperationsMsg` when lifecycle removal is required.

### 6.2 Capability matching

A unit is eligible only when it contains **every** capability listed in `Transaction.required_capabilities`.

```text
Unit capabilities:        [payment, card, settlement]
Required capabilities:    [payment, card]
Eligible:                  yes

Unit capabilities:        [payment]
Required capabilities:    [payment, card]
Eligible:                  no
```

Among eligible units, Operations asks each unit for its load and selects the lowest-load actor.

### 6.3 Process a transaction

```rust
use serde_json::json;
use vsm_rs::system1::{self, Transaction, TransactionResult};

let tx = Transaction::new(
    "capture_payment",
    vec!["payment".into(), "settlement".into()],
    json!({
        "payment_id": "pay_123",
        "amount": 80.00
    }),
);

match system1::process_transaction(tx).await? {
    TransactionResult::Ok(value) => {
        println!("unit response: {value:#}");
    }
    TransactionResult::InvalidTransaction(reason) => {
        eprintln!("bad transaction: {reason}");
    }
    TransactionResult::NoSuitableUnit => {
        eprintln!("System 1 requested resources from System 3");
    }
    TransactionResult::UnitUnavailable(unit) => {
        eprintln!("unit unavailable: {unit}");
    }
    TransactionResult::UnitError(error) => {
        eprintln!("unit failed: {error}");
    }
}
```

A transaction is currently invalid only when its `kind`/serialized `type` is empty.

The built-in Unit actor is a demonstration implementation. It returns a JSON success envelope and does not perform application-specific work. Replace or extend `system1/unit.rs` before using it as a real operational worker.

### 6.4 Inspect units

```rust
for unit in system1::list_units().await? {
    println!(
        "{}: status={}, capabilities={:?}",
        unit.id,
        unit.status,
        unit.config.capabilities
    );
}
```

Possible reported status values include:

- `running`
- a status assigned by a command
- `unknown` after an RPC failure
- `down` when the actor name is not registered

### 6.5 Metrics

```rust
let metrics = system1::get_metrics().await?;
println!("total: {}", metrics.transaction_count);
println!("successes: {}", metrics.success_count);
println!("failures: {}", metrics.failure_count);
println!("invalid: {}", metrics.invalid_transaction_count);
println!("no suitable unit: {}", metrics.no_suitable_unit_count);
```

Metrics are held in the Operations actor and reset when that actor restarts.

### 6.6 Operational variety

```rust
let variety = system1::get_variety().await?;
println!("input variety: {}", variety.input);
println!("output variety: {}", variety.output);
println!("ratio: {}", variety.ratio);
println!("trend: {:?}", variety.trend);
```

The snapshot uses at most the latest 100 transactions. Trend compares recent and older ratio windows with a 10% threshold.

### 6.7 Send an algedonic signal from System 1

```rust
use serde_json::json;

system1::send_algedonic_signal(json!({
    "severity": "high",
    "message": "payment failure rate exceeded threshold",
    "failure_rate": 0.18
}))?;
```

This is a non-blocking send to Operations. Operations publishes an algedonic `VsmMessage` to System 5. The current Policy actor records the event in history but does not automatically invoke crisis handling.

## 7. Drive System 1 through channels

### 7.1 Execute a command on all units

The demo Unit recognizes a `status` field:

```rust
use serde_json::json;
use vsm_rs::channels::command_channel;
use vsm_rs::{MessageKind, SystemId};

command_channel::send_message(
    SystemId::System3,
    SystemId::System1,
    MessageKind::Execute,
    json!({ "status": "paused" }),
)?;
```

System 1 forwards the command to every registered unit.

### 7.2 Request state synchronization

`CoordinationRequest` is a tagged enum. The payload must contain a snake-case `type` field:

```rust
use serde_json::json;
use vsm_rs::channels::coordination_channel;
use vsm_rs::{MessageKind, SystemId};

coordination_channel::send_message(
    SystemId::System2,
    SystemId::System1,
    MessageKind::Coordinate,
    json!({
        "type": "sync_state",
        "unit_ids": ["payments-eu", "payments-us"]
    }),
)?;
```

System 1 asks the selected units for their JSON state, merges object fields by RFC 3339 `timestamp` when available, and sends the merged state to all registered units.

### 7.3 Request load balancing

```rust
coordination_channel::send_message(
    SystemId::System2,
    SystemId::System1,
    MessageKind::Coordinate,
    json!({
        "type": "load_balance",
        "unit_ids": ["payments-eu", "payments-us"]
    }),
)?;
```

Loads more than 20% above average receive `MigrateWork::Out`; loads more than 20% below average receive `MigrateWork::In`.

### 7.4 Request a System 1 audit

```rust
use vsm_rs::channels::audit_channel;

audit_channel::send_message(
    SystemId::System3Star,
    SystemId::System1,
    MessageKind::AuditRequest,
    json!({ "scope": "all_units" }),
)?;
```

System 1 publishes an `AuditResponse` containing unit IDs, metrics, variety, timestamp, and config.

## 8. Generic message and channel APIs

### 8.1 Construct and publish a typed VSM message

```rust
use serde_json::json;
use vsm_rs::channels;
use vsm_rs::{ChannelKind, MessageKind, SystemId, VsmMessage};

let message = VsmMessage::new(
    SystemId::System4,
    SystemId::System3,
    ChannelKind::Command,
    MessageKind::StrategicChange,
    json!({ "directive": "increase resilience reserve" }),
);

// Validate synchronously when rejection must be visible to the caller.
message
    .validate()
    .map_err(vsm_rs::VsmError::Validation)?;

let outcome = channels::publish_with_outcome(message).await?;
assert!(outcome.is_delivered());
```

`publish_with_outcome()` returns the broker delivery result. `Delivered` means
the recipient actor mailbox accepted the message; it does not prove that the
recipient completed domain processing. `publish()` remains available when the
caller only needs to enqueue a message to the broker.

### 8.2 Message convenience facade

```rust
use vsm_rs::shared::message;

let sent = message::send(
    SystemId::System1,
    SystemId::System2,
    ChannelKind::Coordination,
    MessageKind::UnitRegistered,
    json!({ "unit_id": "payments" }),
)?;

println!("message id: {}", sent.id);
```

### 8.3 Correlated replies

```rust
let reply = original.reply(
    MessageKind::DecisionResponse,
    json!({ "approved": true }),
);

assert_eq!(reply.reply_to.as_deref(), Some(original.id.as_str()));
```

### 8.4 Broadcast

```rust
channels::broadcast(
    ChannelKind::Command,
    VsmMessage::new(
        SystemId::External,
        SystemId::All,
        ChannelKind::Command,
        MessageKind::Other("refresh".into()),
        json!({}),
    ),
)?;
```

Explicit broadcast sends to every subscriber on the channel only when the
message target is `SystemId::All`. Use `broadcast_with_outcome()` when the
caller needs to know whether the broker delivered, rejected, or dead-lettered
the broadcast.

### 8.5 Channel inspection

```rust
for channel in channels::list_channels().await? {
    let stats = channels::stats(channel).await?;
    println!(
        "{:?}: {} subscribers, {} retained messages, {} dead letters",
        channel,
        stats.subscriber_count,
        stats.retained_message_count,
        stats.undeliverable_count
    );
}

let command_history = channels::history(ChannelKind::Command).await?;
for message in command_history.iter().take(10) {
    println!("{} {:?}", message.id, message.kind);
}

let dead_letters = channels::dead_letters(ChannelKind::Command).await?;
for entry in dead_letters.iter().take(10) {
    println!("{} {:?}", entry.outcome.message_id, entry.outcome.status);
}
```

History is newest first and capped at 10,000 messages per channel.

### 8.6 Subscriber IDs

Targeted routing uses `message.to.subscriber_id()`:

```text
System1      -> system1
System2      -> system2
System3      -> system3
System3Star  -> system3_star
System4      -> system4
System5      -> system5
```

Use these IDs for subsystem listeners. Use unique IDs such as `audit-recorder-1` for additional observers. A second subscription with the same channel and ID replaces the first.

## 9. Implement a custom channel subscriber

A custom actor can expose only the channel portion of its protocol through a `DerivedActorRef`.

```rust
use ractor::{Actor, ActorProcessingErr, ActorRef};
use vsm_rs::channels;
use vsm_rs::channels::broker::VsmActorMsg;
use vsm_rs::ChannelKind;

pub enum ListenerMsg {
    Channel(VsmActorMsg),
    Flush,
}

impl From<VsmActorMsg> for ListenerMsg {
    fn from(message: VsmActorMsg) -> Self {
        Self::Channel(message)
    }
}

impl TryFrom<ListenerMsg> for VsmActorMsg {
    type Error = ListenerMsg;

    fn try_from(message: ListenerMsg) -> Result<Self, Self::Error> {
        match message {
            ListenerMsg::Channel(inner) => Ok(inner),
            other => Err(other),
        }
    }
}

pub struct Listener;

#[ractor::async_trait]
impl Actor for Listener {
    type Msg = ListenerMsg;
    type State = Vec<String>;
    type Arguments = ();

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _args: (),
    ) -> Result<Self::State, ActorProcessingErr> {
        channels::subscribe(
            ChannelKind::Audit,
            "audit-recorder-1",
            myself.get_derived::<VsmActorMsg>(),
        )
        .await
        .map_err(|error| -> ActorProcessingErr { Box::new(error) })?;

        Ok(Vec::new())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            ListenerMsg::Channel(VsmActorMsg::ChannelMessage(message)) => {
                state.push(message.id);
            }
            ListenerMsg::Channel(VsmActorMsg::AlgedonicSignal(message)) => {
                state.push(message.id);
            }
            ListenerMsg::Flush => state.clear(),
        }
        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        let _ = channels::unsubscribe(ChannelKind::Audit, "audit-recorder-1").await;
        Ok(())
    }
}
```

Supervise long-lived subscribers rather than spawning detached actors. Re-subscription after a broker restart is not automatic in the current implementation, so production subscribers should also include reconciliation logic.

## 10. JSON service calls

Systems 4–5 use the shared service actor facade:

```rust
use serde_json::json;
use vsm_rs::actor_support::{call_service, cast_service};
use vsm_rs::names;

let value = call_service(
    names::SYSTEM4_INTELLIGENCE,
    "intelligence_report",
    json!({ /* payload */ }),
)
.await?;

cast_service(
    names::SYSTEM4_FORECASTING,
    "update_models",
    json!({ "reason": "new data" }),
)?;
```

`call_service` uses a fixed five-second timeout. `cast_service` is non-blocking.

Unknown operations return:

```json
{
  "status": "unknown_operation",
  "op": "the_name_used"
}
```

They are not returned as an error, so inspect the response when invoking dynamic operation names.

## 11. System 2 typed coordination

System 2 is part of the typed runtime handle. It collects typed System 1
coordination views, invokes the configured `CoordinationPolicy`, delivers typed
interventions to affected units, records acknowledgements, and reports rejected
or failed interventions as typed escalation records for System 3.

```rust
# async fn run<V: vsm_rs::ViableSystem>(
#     runtime: vsm_rs::VsmRuntime<V>,
# ) -> Result<(), vsm_rs::FrameworkError> {
let cycle = runtime.system2().coordinate_system1().await?;
let snapshot = runtime.system2().snapshot().await?;

println!("conflicts: {}", cycle.conflicts.len());
println!("stored views: {}", snapshot.views.len());
# Ok(())
# }
```

Provide a custom policy with `VsmBuilder::coordination_policy`. If omitted, the
typed runtime uses the no-op policy, which records views but produces no
conflicts or interventions.

The old schedule and balancing helpers are retained under
`system2::defaults::{scheduler, balancer}` as explicit example algorithms. They
are not part of the typed System 2 core path and do not perform authoritative
resource allocation.

## 12. System 3 usage

System 3 is part of the typed runtime handle. It invokes application
`ResourceGovernance`, `OperationalControlPolicy`, and `Auditor` roles over
framework-owned typed records. The global JSON `system3::control` service is no
longer started.

```rust
# async fn run<V: vsm_rs::ViableSystem>(
#     runtime: vsm_rs::VsmRuntime<V>,
#     shortage: vsm_rs::protocol::system1::ResourceShortageRequest<V>,
# ) -> Result<(), vsm_rs::FrameworkError> {
let cycle = runtime.system3().handle_resource_shortage(shortage).await?;
let snapshot = runtime.system3().snapshot().await?;

println!("allocations: {}", cycle.allocations.len());
println!("directives tracked: {}", snapshot.directives.len());
# Ok(())
# }
```

System 3* audit collects evidence through a separate System 1 audit path rather
than by reading ordinary report history:

```rust
# async fn run<V: vsm_rs::ViableSystem>(
#     runtime: vsm_rs::VsmRuntime<V>,
#     request: vsm_rs::protocol::system3::System3AuditRequest<V>,
# ) -> Result<(), vsm_rs::FrameworkError> {
let response = runtime.system3().audit_system1(request).await?;
println!("findings: {}", response.findings.len());
# Ok(())
# }
```

The old JSON resource and audit helpers are retained under
`system3::defaults::{resources, audit}` as explicit example algorithms. They
are not part of the typed System 3 core path.

## 13. System 4 usage

System 4 offers an aggregate Intelligence actor and separate Scanner, Analytics, and Forecasting actors.

### 13.1 Environmental scan

```rust
use serde_json::json;
use vsm_rs::system4::intelligence;

let scan = intelligence::environmental_scan(
    vec![
        json!({ "id": "market", "value": 0.72 }),
        json!({ "id": "regulation", "value": -0.50 }),
    ],
    json!({ "region": "global" }),
)
.await?;
```

Signals above `0.65` are classified as opportunities, below `-0.35` as threats, and the remainder as weak signals.

### 13.2 Intelligence report with custom sources

`get_intelligence_report()` uses an empty source list. For real input, call the service directly:

```rust
use vsm_rs::actor_support::call_service;
use vsm_rs::names;

let report = call_service(
    names::SYSTEM4_INTELLIGENCE,
    "intelligence_report",
    json!({
        "sources": [
            { "id": "market", "value": 0.72 },
            { "id": "supply", "value": -0.40 }
        ],
        "z_threshold": 2.0
    }),
)
.await?;
```

### 13.3 Analyze data

```rust
use vsm_rs::system4::analytics;

let analysis = analytics::analyze_data(
    json!([1.0, 1.2, 1.3, 4.8]),
    "anomaly",
)
.await?;

let trend = analytics::analyze_trends(
    json!([1, 2, 3, 4]),
    "hour",
)
.await?;
```

`analyze_data` calls the Analytics actor. `analyze_trends` is a direct helper and does not use actor state.

### 13.4 Forecast

```rust
let forecast = call_service(
    names::SYSTEM4_FORECASTING,
    "forecast",
    json!({
        "history": [10.0, 12.0, 13.0, 15.0],
        "horizon": 5,
        "model": "linear"
    }),
)
.await?;
```

Models recognized by the current function are:

- `linear` or any unrecognized value: trend projection
- `mean`: historical mean
- `naive`: last observed value

### 13.5 System 4 operation reference

#### Intelligence actor

| Operation | Payload |
|---|---|
| `environmental_scan`, `scan` | `{sources: [...], ...options}` |
| `analyze` | Analytics payload |
| `forecast` | Forecasting payload |
| `intelligence_report` | `{sources: [...], ...options}` |

#### Scanner actor

| Operation | Payload |
|---|---|
| `scan`, `scan_environment` | `{sources: [...]}` |
| `detect_changes` | `{current: {...}, previous: {...}}` |
| `classify` | JSON array of signals |
| `trends` | JSON array of values/signals |

#### Analytics actor

| Operation | Payload |
|---|---|
| `analyze` | `{data: [...], analysis_type: "summary|trend|correlation|anomaly|insight", ...options}` |
| `correlate` | `{data: [{x, y}, ...]}` |
| `detect_anomalies` | `{data: [...], z_threshold: 2.0}` |
| `generate_insights` | `{data: [...], ...options}` |

#### Forecasting actor

| Operation | Payload |
|---|---|
| `forecast` | `{history: [...], horizon: 10, model: "linear"}` |
| `scenarios` | `{base_forecast: {...}, scenario_delta: 0.15}` |
| `validate` | `{forecast: {...}, actuals: [...]}` |
| `models` | `{}` |

## 14. System 5 usage

For a single coherent organizational state, use the Policy actor as the aggregate boundary.

### 14.1 Set identity through Policy

```rust
let identity = call_service(
    names::SYSTEM5_POLICY,
    "set_identity",
    json!({
        "purpose": "deliver resilient payment infrastructure",
        "mission": "operate safely across regions",
        "core_values": ["resilience", "fairness", "autonomy"]
    }),
)
.await?;
```

### 14.2 Define values through Policy

```rust
let values = call_service(
    names::SYSTEM5_POLICY,
    "define_values",
    json!([
        {
            "name": "resilience",
            "priority": 1.0,
            "indicators": ["resilient", "redundant", "recoverable"]
        },
        {
            "name": "fairness",
            "priority": 0.9,
            "indicators": ["fair", "transparent", "inclusive"]
        }
    ]),
)
.await?;
```

### 14.3 Set policy

```rust
use vsm_rs::system5::policy;

let policy_result = policy::set_policy_area(
    "risk",
    json!({
        "max_failure_rate": 0.02,
        "escalation_threshold": "high"
    }),
)
.await?;
```

### 14.4 Make a decision

```rust
let decision = call_service(
    names::SYSTEM5_POLICY,
    "make_decision",
    json!({
        "subject": "capacity_expansion",
        "options": [
            { "name": "expand_now", "viability": 0.9, "cost": 0.4 },
            { "name": "defer", "viability": 0.5, "cost": 0.9 }
        ],
        "criteria": [
            { "name": "viability", "weight": 0.8 },
            { "name": "cost", "weight": 0.2 }
        ]
    }),
)
.await?;
```

The scorer multiplies each named option field by the criterion weight and selects the maximum total.

`system5::policy::make_decision()` is a convenience wrapper, but the current wrapper adds a nested copy under `decision`. Use `call_service` directly when a raw, non-duplicated decision object is preferred.

### 14.5 Evaluate alignment

```rust
let alignment = call_service(
    names::SYSTEM5_POLICY,
    "evaluate_alignment",
    json!({
        "proposal": "build a resilient, transparent regional service"
    }),
)
.await?;
```

The current identity and values alignment methods are keyword-based heuristics, not semantic models.

### 14.6 Handle a crisis explicitly

```rust
let response = call_service(
    names::SYSTEM5_POLICY,
    "handle_crisis",
    json!({
        "severity": "critical",
        "message": "regional settlement outage"
    }),
)
.await?;
```

An algedonic channel message does not invoke this operation automatically in the current port.

### 14.7 Get organizational state

```rust
let state = policy::get_organizational_state().await?;
println!("{state:#}");
```

### 14.8 State isolation warning

These actors have separate state:

```text
vsm.system5.policy
vsm.system5.identity
vsm.system5.values
vsm.system5.decisions
```

For example:

```rust
call_service(names::SYSTEM5_IDENTITY, "set_identity", identity).await?;
```

updates the standalone Identity actor, not the Policy actor. A later Policy `evaluate_alignment` call will use Policy's own identity/default identity. Choose one aggregate boundary and use it consistently.

### 14.9 System 5 operation reference

#### Policy actor

| Operation | Payload |
|---|---|
| `set_identity` | Identity object/patch |
| `define_values` | Array of value definitions |
| `make_decision` | `{subject, options, criteria}` |
| `set_policy` | `{policy_area, policy_details}` |
| `evaluate_alignment` | Proposal/subject JSON |
| `handle_crisis` | Crisis JSON with optional `severity` |
| `get_organizational_state`, `state` | `{}` |

#### Identity actor

| Operation | Payload |
|---|---|
| `set_identity` | Identity object/patch |
| `get_current_identity`, `identity` | `{}` |
| `check_alignment` | Proposal JSON |
| `get_relevant_aspects` | Context JSON |
| `update_aspect` | `{aspect, value}` |
| `evolve_identity` | Deep-merge patch |

#### Values actor

| Operation | Payload |
|---|---|
| `define_values` | Array of values |
| `get_current_values`, `values` | `{}` |
| `evaluate_against_values`, `check_alignment` | Subject JSON |
| `validate_policy` | `{policy_area, policy_details}` |
| `add_value` | One value object |
| `update_value_priority` | `{name, priority}` |

#### Decisions actor

| Operation | Payload |
|---|---|
| `make_decision`, `decide` | `{subject, options, criteria}` |
| `history`, `decision_history` | Optional `{subject}` filter |
| `review`, `review_decision` | `{decision_id, outcome_data}` |
| `patterns` | `{}` |

## 15. Advanced algedonic processor

The advanced algedonic actor is separate from the broker's algedonic channel.

### 15.1 Send pain and pleasure signals

```rust
use serde_json::json;
use vsm_rs::channels::algedonic;
use vsm_rs::channels::algedonic::signals::Severity;

algedonic::send_pain_signal(
    "payments",
    json!({
        "message": "latency spike",
        "urgency": 0.8,
        "latency_ms": 1_500
    }),
    Severity::High,
)?;

algedonic::send_pleasure_signal(
    "payments",
    json!({
        "message": "throughput target exceeded",
        "urgency": 0.3
    }),
    Severity::Medium,
)?;
```

### 15.2 Inspect signals and metrics

```rust
let active = algedonic::get_active_signals().await?;
let metrics = algedonic::get_metrics().await?;
println!("active: {}", active.len());
println!("metrics: {metrics:#}");
```

### 15.3 Configure filters

```rust
use vsm_rs::channels::algedonic::filtering::{
    create_filter, FilterKind,
};

algedonic::configure_filters(vec![
    create_filter(
        FilterKind::Priority,
        "production_priority_floor",
        json!({ "min_priority": 0.55 }),
        true,
    ),
    create_filter(
        FilterKind::Source,
        "approved_sources",
        json!({ "allow": ["payments", "settlement"] }),
        true,
    ),
])
.await?;
```

### 15.4 Alert history

```rust
use vsm_rs::channels::algedonic::alerting;

let history = alerting::get_alert_history(&json!({ "limit": 25 }));
```

The actor computes descriptive routes and alert records. It does not deliver those routes to System 3 or System 5. Publish a `VsmMessage` separately when actor delivery is required.

## 16. Temporal variety

### 16.1 Record measurements

```rust
use serde_json::json;
use vsm_rs::channels::temporal_variety;

temporal_variety::record_variety(json!({
    "input": 12.0,
    "output": 9.0,
    "ratio": 0.75
}))?;
```

Recording is asynchronous.

### 16.2 Query a timescale

Default scale names are `instant`, `minute`, `hour`, and `day`.

```rust
let instant = temporal_variety::get_variety("instant").await?;
let hourly = temporal_variety::get_variety("hour").await?;
```

Using another name returns an empty calculation unless you started the actor with custom scales. The public application currently starts it with default configuration.

### 16.3 Patterns, forecasts, causality, and summaries

```rust
let patterns = temporal_variety::get_patterns().await?;
let forecasts = temporal_variety::get_forecasts(vec![1, 5, 10]).await?;
let causality = temporal_variety::get_causality().await?;
let summary = temporal_variety::get_summary().await?;
let visualization = temporal_variety::get_visualization_data(json!({
    "format": "json"
})).await?;
```

These are lightweight calculations over in-memory buffers. Internal maintenance messages exist but are not timer-driven; queries calculate current values on demand.

## 17. Shared variety engineering

Pure helper modules can be used without starting the actor application.

```rust
use serde_json::json;
use vsm_rs::shared::variety::{amplifier, attenuator, calculator};
use vsm_rs::shared::variety_engineering;

let input = json!([1.0, 2.0, 3.0, 9.0]);
let output = json!([1.0, 2.0]);

let analysis = calculator::analyze_variety(&input, &output);
let recommendation = variety_engineering::analyze(&input, &output);

let filtered = attenuator::filter(
    &[
        json!({"value": 0.1}),
        json!({"value": 0.9}),
    ],
    "threshold",
    &json!({"min": 0.5}),
);

let amplified = amplifier::multiply(
    &json!({"workers": 4}),
    2.0,
    &json!({"reason": "variety deficit"}),
);
```

These functions are heuristic starter implementations. Validate their formulas for your domain before using them for operational control.

## 18. Recursive viable-system structures

```rust
use serde_json::json;
use vsm_rs::shared::recursion;

let structure = recursion::initialize_structure(json!({
    "id": "enterprise",
    "name": "Enterprise VSM"
}));

let structure = recursion::create_level(
    structure,
    "enterprise",
    json!({ "id": "division-a", "name": "Division A" }),
);

let structure = recursion::create_level(
    structure,
    "division-a",
    json!({ "id": "team-1", "name": "Team 1" }),
);

let structure = recursion::update_context(
    structure,
    "division-a",
    json!({ "market": "EU", "risk": "medium" }),
);

println!(
    "tree: {:#}",
    recursion::get_hierarchy_tree(&structure, None)
);
println!("metrics: {:#}", recursion::calculate_metrics(&structure));
assert!(recursion::validate_structure(&structure).is_ok());
```

The recursion module is a pure value API: functions generally consume and return `RecursionStructure`.

## 19. Health, status, and observability

### 19.1 Health

```rust
let health = vsm_rs::health().await?;
println!("{health:#}");
```

The result includes:

- a literal top-level status
- whether the root supervisor is registered
- stats for channels that answered
- telemetry service health or an unavailable object

Inspect `root_supervisor` rather than relying only on the top-level `status` string.

### 19.2 Full status

```rust
let status = vsm_rs::status().await?;
```

`status()` combines health with best-effort state from Systems 4–5. Failed subsystem calls are omitted from the subsystem object rather than failing the whole operation.

### 19.3 Tracing

Enable logging with `RUST_LOG`:

```bash
RUST_LOG=vsm_rs=debug,ractor=info cargo run
```

Or configure a default directive in code:

```rust
tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("info".parse()?),
    )
    .init();
```

## 20. Error handling

Public typed APIs generally return `VsmResult<T>` or `Result<T, VsmError>`.

Useful variants include:

```text
ActorNotFound / ActorUnavailable   Service was not started or has stopped
UnitAlreadyRegistered             Duplicate System 1 unit ID
InvalidPayload                    JSON could not be decoded into a typed request
Validation                        Message/filter validation failed
Supervisor                        Dynamic child operation failed
Ractor / Runtime                  Actor send, call, or timeout failure
Serialization                     serde_json conversion failed
```

A standard application pattern is:

```rust
match system1::process_transaction(tx).await {
    Ok(result) => handle_result(result),
    Err(vsm_rs::VsmError::ActorNotFound(name)) => {
        eprintln!("VSM service is not ready: {name}");
    }
    Err(error) => return Err(error.into()),
}
```

Remember that some domain failures are values, not `VsmError`s. For example, `NoSuitableUnit` is a `TransactionResult`.

## 21. Timeouts and backpressure

Current RPC timeouts are fixed in the facades. Important values are:

```text
System 1 transaction             10 seconds
System 1 registration             5 seconds
Generic JSON service call         5 seconds
System 1 status/metrics/variety   2 seconds
Channel queries/subscription      2 seconds
Unit and advanced service RPCs    1 second
```

An actor handles one message at a time. Avoid blocking handlers with long database calls, CPU-heavy analysis, or network retries. A common extension pattern is:

1. accept the actor message
2. spawn or delegate slow work
3. keep a request/correlation ID in actor state
4. send completion back to the actor
5. reply or publish a result

The broker does not currently implement queue limits, recipient processing
acknowledgements, retries, or consumer demand. Outcome-returning APIs report
broker delivery, rejection, and dead-lettering; add durable replay and recipient
acknowledgement before treating channels as a durable event bus.

## 22. Testing

The included tests demonstrate startup, unit registration, transaction processing, typed System 2 coordination, typed System 3 governance/audit, System 4 analysis, System 5 decisions, and health.

Run:

```bash
cargo test --all-targets --all-features --locked -- --nocapture
```

When writing more integration tests:

```rust
use serial_test::serial;

#[tokio::test]
#[serial]
async fn my_vsm_test() {
    let app = vsm_rs::start().await.unwrap();
    // Exercise APIs.
    app.supervisor.stop(Some("test complete".into()));
    let _ = app.join_handle.await;
}
```

Always stop and await the application so global names are released before the next test.

## 23. Recommended production integration pattern

A practical application boundary is:

```text
Your HTTP/CLI/event adapters
        |
        v
Typed application service layer
        |
        +-- System 1 typed facade
        +-- typed System 2 coordination
        +-- typed System 3 governance and audit
        +-- typed wrappers around Systems 4–5 operations
        +-- validated VsmMessage publishing
        +-- domain-specific channel subscribers
        |
        v
VSM actor tree
```

Recommended next steps before production use:

1. Add a readiness actor or barrier.
2. Wrap every string-based service operation in typed request/response structs.
3. Convert channel events into domain actions for Systems 4–5.
4. Add durable state or event sourcing for policies, decisions, units, and metrics.
5. Reconcile subscriptions after broker restart.
6. Reconcile System 1 unit state after Operations or unit-supervisor restart.
7. Replace the demo Unit implementation with domain workers.
8. Add explicit unit unregister, drain, and update operations.
9. Add recipient processing acknowledgements, retry, and durable replay where required.
10. Add real telemetry export and periodic reporting.

## 24. Important current behavior at a glance

| Behavior | Current implementation |
|---|---|
| Application instances | One per process due to global names |
| State persistence | In-memory only |
| Channel delivery | Broker delivery outcomes available; recipient processing is not acknowledged |
| Invalid targeted publish | `RejectedByProtocol` outcome and dead-letter history |
| Missing targeted subscriber | `TargetUnavailable` outcome and dead-letter history |
| Duplicate subscriber ID | Replaces previous subscription |
| System 2 coordination | Typed runtime policy over System 1 coordination views |
| Systems 4–5 channel reactions | Record history only |
| System 1 channel reactions | Execute command, coordinate, audit |
| Advanced algedonic routing | Records route; does not deliver it |
| Temporal scheduled analysis | Not scheduled; queries calculate on demand |
| Unknown generic operation | JSON `unknown_operation`, not an error |
| System 5 state | Separate per actor unless Policy is used as aggregate |
| Restart recovery | Actor restart exists; relationship/state reconciliation is incomplete |

Read `ARCHITECTURE.md` before changing supervision, channel routing, or state ownership.
