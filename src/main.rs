use serde_json::json;
use tokio::time::{sleep, Duration};
use tracing_subscriber::EnvFilter;

use vsm_ractor_full::app;
use vsm_ractor_full::system1::{self, Transaction, UnitConfig};
use vsm_ractor_full::vsm_core;

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
