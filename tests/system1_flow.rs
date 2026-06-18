use serde_json::json;
use serial_test::serial;
use tokio::time::{sleep, Duration};

use vsm_ractor_full::app;
use vsm_ractor_full::system1::{self, Transaction, TransactionResult, UnitConfig};

#[tokio::test]
#[serial]
async fn register_and_process_transaction() {
    let (root, root_handle) = app::start_vsm_core().await.expect("app should start");
    sleep(Duration::from_millis(100)).await;

    let unit_id = system1::register_unit(UnitConfig::new("unit-a", ["alpha", "beta"]))
        .await
        .expect("unit should register");
    assert_eq!(unit_id, "unit-a");

    let result = system1::process_transaction(Transaction::new(
        "alpha_work",
        vec!["alpha".to_string()],
        json!({ "x": 1, "y": 2 }),
    ))
    .await
    .expect("transaction call should succeed");

    assert!(matches!(result, TransactionResult::Ok(_)));

    let metrics = system1::get_metrics().await.expect("metrics should return");
    assert_eq!(metrics.transaction_count, 1);
    assert_eq!(metrics.success_count, 1);

    root.stop(Some("test complete".to_string()));
    let _ = root_handle.await;
}
