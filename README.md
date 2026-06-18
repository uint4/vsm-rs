# vsm-rs

[![Crates.io](https://img.shields.io/crates/v/vsm-rs.svg)](https://crates.io/crates/vsm-rs)
[![Documentation](https://docs.rs/vsm-rs/badge.svg)](https://docs.rs/vsm-rs)
[![License](https://img.shields.io/crates/l/vsm-rs.svg)](LICENSE)

An actor-based implementation of Stafford Beer's **Viable System Model (VSM)** for Rust, built with [`ractor`](https://crates.io/crates/ractor) and OTP-style supervision from [`ractor-supervisor`](https://crates.io/crates/ractor-supervisor).

`vsm-rs` provides a supervised runtime for the five VSM subsystems, typed inter-system messages, operational units, pub/sub channels, algedonic escalation, temporal-variety analysis, and reusable variety-engineering utilities.

> **Project status:** the crate is currently in the `0.x` series. It is suitable for experimentation, simulation, research, and application integration, but its public API may evolve before `1.0`. Runtime state and channel history are currently in memory.

## What the crate models

The Viable System Model describes the functions a system needs in order to remain viable in a changing environment:

| System | Role | Implementation |
|---|---|---|
| **System 1 — Operations** | Performs the primary work | Dynamic operational units, transaction routing, metrics, and operational-variety tracking |
| **System 2 — Coordination** | Dampens oscillation between operational units | Schedule coordination, conflict detection, balancing, and anti-oscillation services |
| **System 3 — Control** | Manages internal resources and operational control | Resource allocation, control state, and System 3* audit functions |
| **System 4 — Intelligence** | Observes the environment and looks forward | Environmental scanning, analytics, forecasting, and intelligence reports |
| **System 5 — Policy** | Maintains identity, values, and direction | Identity, values, policy, strategic decisions, and crisis response |

The systems communicate through command, coordination, audit, resource-bargain, algedonic, and temporal-variety channels. Algedonic signals provide an emergency path to System 5 that bypasses the normal hierarchy.

## Features

- A root supervision tree covering Systems 1–5, channel infrastructure, and telemetry.
- Dynamic supervision of System 1 operational units.
- Typed `VsmMessage`, `SystemId`, `ChannelKind`, and `MessageKind` domain types.
- Targeted routing, broadcast, subscriptions, channel statistics, and bounded in-memory history.
- Operational transaction routing by capability and current load.
- Variety measurement and trend tracking based on input and output variety.
- Algedonic pain, pleasure, anomaly, opportunity, and emergency signals.
- Temporal aggregation, trend, cycle, seasonality, anomaly, forecasting, and causality helpers.
- Pure functions for scheduling, resource allocation, auditing, forecasting, recursion, and variety engineering.
- JSON-oriented service boundaries for Systems 2–5, making integration straightforward while the typed API continues to mature.

## Installation

Add the crate and the runtime dependencies used by your application:

```bash
cargo add vsm-rs
cargo add tokio --features macros,rt-multi-thread,time
cargo add serde_json
```

Or add them manually:

```toml
[dependencies]
vsm-rs = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
serde_json = "1"
```

The package name contains hyphens; Rust code imports it as `vsm_rs`.

## Quick start

The following example starts the complete VSM runtime, registers a System 1 unit, processes a transaction, reads system status, and shuts the supervision tree down cleanly.

```rust
use serde_json::json;
use tokio::time::{sleep, Duration};
use vsm_rs::{start, VsmApplication};
use vsm_rs::system1::{
    self, Transaction, TransactionResult, UnitConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let VsmApplication {
        supervisor,
        join_handle,
    } = start().await?;

    // Child supervisors are started asynchronously by the root supervisor.
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

    println!("status: {:#}", vsm_rs::status().await?);

    supervisor.stop(Some("application shutdown".to_owned()));
    let _ = join_handle.await;
    Ok(())
}
```

A runnable version is available in [`examples/basic_usage.rs`](examples/basic_usage.rs):

```bash
RUST_LOG=info cargo run --example basic_usage
```

## Core API

### Application lifecycle

```rust
let app = vsm_rs::start().await?;
let health = vsm_rs::health().await?;
let status = vsm_rs::status().await?;
vsm_rs::stop().await?;
```

Use `app::start_vsm_core()` or the returned `VsmApplication` when the caller needs the root `ActorRef` and join handle for deterministic shutdown.

### System 1 operations

```rust
use serde_json::json;
use vsm_rs::system1::{self, Transaction, UnitConfig};

system1::register_unit(UnitConfig::new("billing", ["invoice", "payment"])).await?;

let result = system1::process_transaction(Transaction::new(
    "create_invoice",
    vec!["invoice".into()],
    json!({"customer_id": "customer-42"}),
))
.await?;

let units = system1::list_units().await?;
let metrics = system1::get_metrics().await?;
let variety = system1::get_variety().await?;
```

A unit is selected when it advertises every capability required by the transaction. When multiple units match, System 1 chooses the unit reporting the lowest load.

### Inter-system channels

```rust
use serde_json::json;
use vsm_rs::{
    channels,
    ChannelKind,
    MessageKind,
    SystemId,
    VsmMessage,
};

channels::publish(VsmMessage::new(
    SystemId::System3,
    SystemId::System1,
    ChannelKind::Command,
    MessageKind::Execute,
    json!({"status": "maintenance"}),
))?;

let command_stats = channels::stats(ChannelKind::Command).await?;
let command_history = channels::history(ChannelKind::Command).await?;
```

The broker validates the basic VSM flow matrix before routing internal messages. External endpoints and explicit broadcasts can be used as integration boundaries.

### Systems 2–5

Systems 2–5 expose convenience functions for common operations and a generic JSON service interface for extensibility:

```rust
use serde_json::json;
use vsm_rs::{actor_support::call_service, names};

let report = call_service(
    names::SYSTEM4_INTELLIGENCE,
    "intelligence_report",
    json!({
        "sources": [
            {"id": "market", "value": 0.72},
            {"id": "operations", "value": 0.61}
        ]
    }),
)
.await?;
```

Prefer the subsystem convenience functions where one exists, such as:

- `system2::coordination::coordinate_schedules`
- `system2::coordination::balance_requests`
- `system3::control::allocate_resources`
- `system3::control::perform_audit`
- `system4::intelligence::environmental_scan`
- `system4::intelligence::get_intelligence_report`
- `system5::policy::make_decision`
- `system5::policy::set_policy_area`

### Algedonic signals

```rust
use serde_json::json;
use vsm_rs::channels::algedonic::{self, signals::Severity};

algedonic::send_pain_signal(
    "payments",
    json!({"message": "latency threshold exceeded"}),
    Severity::High,
)?;

let active = algedonic::get_active_signals().await?;
let metrics = algedonic::get_metrics().await?;
```

## Runtime model and current constraints

The crate deliberately follows actor ownership and supervision rather than shared mutable state:

- Long-lived mutable state belongs to actors and is changed through mailbox messages.
- Every long-lived actor has a stable global name defined in `names.rs`.
- Static actors run under `ractor_supervisor::Supervisor`; runtime System 1 units run under `DynamicSupervisor`.
- The channel broker owns subscriptions and message history.
- System 1 uses a typed actor protocol. Systems 2–5 currently use a shared JSON service actor.

Important operational constraints in the current release:

- Actor names are process-global, so one default VSM runtime can run in a process at a time.
- State, metrics, and channel history are not durably persisted.
- The crate does not provide a network transport or distributed actor cluster.
- Startup completion does not yet expose a dedicated readiness barrier; examples wait briefly before their first actor call.
- Applications should validate and bound untrusted JSON payloads before introducing them into the actor system.

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for the supervision tree, message routing, state ownership, and recovery model.

## Documentation

- [API documentation](https://docs.rs/vsm-rs)
- [Usage guide](USAGE.md)
- [Architecture guide](ARCHITECTURE.md)
- [Developer guide](DEVELOPERS.md)
- [Elixir-to-Rust porting map](PORTING_MAP.md)

## Versioning

The crate follows semantic versioning. While the version is below `1.0`, public APIs and JSON schemas may change between minor releases. Release notes should call out actor protocol, message schema, persistence, and supervision changes explicitly.

## Origin

This crate is a Rust actor-model port of the Elixir/OTP project [`viable-systems/vsm-core`](https://github.com/viable-systems/vsm-core). The Rust implementation preserves the original VSM boundaries while adapting process registries, GenServers, and OTP supervision to `ractor`, typed Rust messages, and `ractor-supervisor`.

## Contributing

Read [`DEVELOPERS.md`](DEVELOPERS.md) before making architectural changes. It describes the repository layout, actor conventions, supervision rules, testing strategy, documentation expectations, and release checklist.

## License

MIT. See [`LICENSE`](LICENSE).
