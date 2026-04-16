use guard_rail_engine::execution::{ExecutionRecord, ExecutionVerdict};
use guard_rail_engine::storage::postgres::PostgresAuditStore;

#[tokio::test]
async fn test_insert_execution_and_fetch_it_back() {
    async fn connect_test_pool(database_url: &str) -> sqlx::PgPool {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    async fn reset_execution_audit(pool: &sqlx::PgPool) {
        sqlx::query("truncate table execution_audit restart identity")
            .execute(pool)
            .await
            .unwrap();
    }

    fn sample_blocked_record() -> ExecutionRecord {
        ExecutionRecord {
            execution_id: "GR-EXE-blocked-1".to_string(),
            execution_started_at: chrono::Utc::now(),
            route_id: "test-route".to_string(),
            upstream_url: Some("http://upstream.test/api".to_string()),
            method: "POST".to_string(),
            source_ip: "127.0.0.1".to_string(),
            content_type: Some("application/json".to_string()),
            user_agent: Some("integration-test".to_string()),
            had_authorization_header: false,
            request_size_bytes: 32,
            request_body_sha256: guard_rail_engine::audit::hash::hash_body(br#"{"callback":"https://evil.sh"}"#),
            verdict: ExecutionVerdict::Blocked,
            rejection_reason: None,
            matched_policy_name: Some("block-callbacks".to_string()),
            matched_rule_field: Some("$.callback".to_string()),
            matched_rule_condition: Some("domain_not_in".to_string()),
            matched_rule_severity: Some("critical".to_string()),
            violation_value_hash: Some(guard_rail_engine::audit::hash::hash_string("https://evil.sh")),
            violation_value_preview: Some("https://evil.sh".to_string()),
            upstream_status: None,
            forward_error: None,
            latency_inspect_us: 20,
            latency_forward_ms: None,
            latency_total_ms: 1,
            route_config_hash: "route-hash".to_string(),
            policy_set_hash: "policy-hash".to_string(),
        }
    }

    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = connect_test_pool(&database_url).await;
    reset_execution_audit(&pool).await;

    let store = PostgresAuditStore::new(pool.clone(), std::time::Duration::from_millis(250));
    let record = sample_blocked_record();

    store.insert_execution(&record).await.unwrap();

    let row = store.get_execution_by_id("GR-EXE-blocked-1").await.unwrap().unwrap();
    assert_eq!(row.route_id, "test-route");
    assert_eq!(row.verdict, "BLOCKED");
    assert!(row.previous_hash.is_none());
    assert!(!row.record_hash.is_empty());
}