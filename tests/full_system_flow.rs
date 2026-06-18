use serde_json::json;
use serial_test::serial;
use tokio::time::{sleep, Duration};

use vsm_rs::system1::{Transaction, TransactionResult, UnitConfig};

#[tokio::test]
#[serial]
async fn full_vsm_flow_exercises_all_systems() {
    let app = vsm_rs::start().await.expect("app should start");
    sleep(Duration::from_millis(100)).await;

    vsm_rs::system1::register_unit(UnitConfig::new("unit-a", ["alpha", "beta"]))
        .await
        .expect("unit should register");

    let result = vsm_rs::system1::process_transaction(Transaction::new(
        "alpha_work",
        vec!["alpha".to_string()],
        json!({"x": 1, "y": 2}),
    ))
    .await
    .expect("transaction call should succeed");
    assert!(matches!(result, TransactionResult::Ok(_)));

    let trend = vsm_rs::system4::defaults::analyze_trends(json!([1, 2, 3, 4]), "hour")
        .await
        .unwrap();
    assert_eq!(trend["direction"], "increasing");

    let decision = vsm_rs::system5::policy::make_decision(json!({"proposal":"maintain viability"}))
        .await
        .unwrap();
    assert!(decision.get("id").is_some());

    let health = vsm_rs::health().await.expect("health should return");
    assert!(health.get("status").is_some());

    app.supervisor.stop(Some("test complete".to_string()));
    let _ = app.join_handle.await;
}
