use serde_json::json;
use tokio::time::{sleep, Duration};
use tracing_subscriber::EnvFilter;

use vsm_rs::channels::algedonic::signals::Severity;
use vsm_rs::system1::{self, Transaction, UnitConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let (root, root_handle) = vsm_rs::app::start_vsm_core().await?;
    sleep(Duration::from_millis(100)).await;

    system1::register_unit(UnitConfig::new("payments", ["payment", "io"])).await?;
    let processed = system1::process_transaction(Transaction::new(
        "payment",
        vec!["payment".into()],
        json!({"amount": 125.0, "currency": "USD"}),
    ))
    .await?;

    vsm_rs::channels::algedonic::send_pain_signal(
        "payments",
        json!({"message":"latency spike", "urgency":0.8}),
        Severity::High,
    )?;

    let intelligence_report = vsm_rs::system4::defaults::scan_environment(
        &[json!({"id":"market", "value":0.72})],
        &json!({}),
    );

    let policy_decision =
        vsm_rs::system5::defaults::make_weighted_decision(&json!({"subject": "demo_policy"}));

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "transaction": processed,
            "status": vsm_rs::vsm_core::status().await?,
            "intelligence_report": intelligence_report,
            "policy_decision": policy_decision,
        }))?
    );

    root.stop(Some("example complete".to_string()));
    let _ = root_handle.await;
    Ok(())
}
