# vsm-ractor-full

A Rust/ractor port of the uploaded `viable-systems/vsm-core` Elixir codebase.

This crate mirrors the Elixir `lib/vsm_core/**/*.ex` tree and keeps the same conceptual boundaries:

- `src/app.rs` ports `VSMCore.Application` and the top-level supervision tree.
- `src/domain.rs` and `src/shared/*` port shared message, channel, recursion, and variety-engineering modules.
- `src/channels/*` ports command, coordination, audit, algedonic, temporal-variety, and advanced algedonic/temporal helpers.
- `src/system1..system5/*` ports all five VSM subsystems and their supervisors.
- `src/telemetry_reporter.rs` ports the telemetry reporter.
- `src/vsm_core.rs` ports the public `VSMCore` facade.

## Actor design

The original GenServers are represented as `ractor` actors. Static infrastructure is run under `ractor-supervisor::Supervisor`; runtime System 1 units are run under `ractor-supervisor::DynamicSupervisor`. Elixir duplicate registries are represented by `channels::broker::ChannelBroker`, which provides subscription, routing, broadcast, stats, and message history.

System 1 uses a dedicated typed actor because it owns dynamic operational units. Systems 2-5 and telemetry use a shared `actor_support::ServiceActor` shell that keeps JSON service state and delegates operations to the corresponding module functions.

## Build locally

```bash
cargo fmt
cargo check
cargo test
cargo run
cargo run --example basic_usage
```

The generation environment did not have `cargo`/`rustc` installed, so this archive was not compiled in the sandbox. The crate has a complete source tree and no dangling module placeholders, but you should still run `cargo check` locally and adjust any API drift if your chosen `ractor` / `ractor-supervisor` versions differ.

The dependency versions are pinned to keep `ractor` aligned with `ractor-supervisor`:

```toml
ractor = { version = "0.14.3", features = ["async-trait"] }
ractor-supervisor = "0.1.9"
```

## Module mapping

See `PORTING_MAP.md` for the Elixir-to-Rust file mapping.
