//! Demonstration binary for starting the default VSM actor tree.
//!
//! The binary mirrors the quick-start flow from `docs/USAGE.md`: start the
//! globally named supervision tree, register a demo System 1 unit, process one
//! transaction, print status, and stop the root supervisor. It is not a
//! production entry point and intentionally uses a short startup delay because
//! the library does not yet expose a formal readiness barrier.

use serde_json::json;
use tokio::time::{sleep, Duration};
use tracing_subscriber::EnvFilter;

use vsm_rs::app;
use vsm_rs::system1::{self, Transaction, UnitConfig};
use vsm_rs::vsm_core;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let (root, root_handle) = app::start_vsm_core().await?;
    sleep(Duration::from_millis(100)).await;

    let unit_id = system1::register_unit(UnitConfig::new("payments", ["payment", "card", "settlement"])).await?;
    println!("registered unit: {unit_id}");

    let result = system1::process_transaction(Transaction::new(
        "payment_authorization",
        vec!["payment".to_string(), "card".to_string()],
        json!({"amount":42.50,"currency":"USD","card_token":"tok_demo"}),
    )).await?;
    println!("transaction result: {result:#?}");
    println!("status: {:#?}", vsm_core::status().await?);

    root.stop(Some("demo complete".to_string()));
    let _ = root_handle.await;
    Ok(())
}
