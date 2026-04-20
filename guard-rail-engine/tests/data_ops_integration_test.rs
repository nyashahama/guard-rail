use std::sync::{Arc, OnceLock};

async fn reset_test_database(pool: &sqlx::PgPool) {
    sqlx::query(
        r#"
        truncate table
            audit_retention_checkpoints,
            replay_runs,
            execution_artifacts,
            policy_snapshots,
            execution_audit,
            tenant_routes,
            api_keys,
            tenants
        restart identity cascade
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
}

static TEST_DB_LOCK: OnceLock<Arc<tokio::sync::Mutex<()>>> = OnceLock::new();

struct TestDatabaseGuard {
    _guard: tokio::sync::OwnedMutexGuard<()>,
}

impl TestDatabaseGuard {
    async fn acquire() -> Self {
        let lock = TEST_DB_LOCK
            .get_or_init(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone();
        Self {
            _guard: lock.lock_owned().await,
        }
    }
}

struct DataOpsHarness {
    pool: sqlx::PgPool,
    _db_guard: TestDatabaseGuard,
}

async fn seed_phase3_data_ops_fixture() -> DataOpsHarness {
    let db_guard = TestDatabaseGuard::acquire().await;
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    reset_test_database(&pool).await;

    let store = guard_rail_engine::storage::postgres::PostgresAuditStore::new(
        pool.clone(),
        std::time::Duration::from_millis(250),
    );
    let now = chrono::Utc::now();

    for (execution_id, started_at) in [
        ("GR-OLD-1", now - chrono::Duration::days(90)),
        ("GR-OLD-2", now - chrono::Duration::days(60)),
        ("GR-KEEP-1", now - chrono::Duration::days(1)),
    ] {
        let record = guard_rail_engine::execution::ExecutionRecord {
            execution_id: execution_id.to_string(),
            execution_started_at: started_at,
            route_id: "test-route".to_string(),
            tenant_id: None,
            api_key_id: None,
            auth_outcome: Some("public".to_string()),
            upstream_url: Some("http://upstream.test/api".to_string()),
            method: "POST".to_string(),
            source_ip: "127.0.0.1".to_string(),
            content_type: Some("application/json".to_string()),
            user_agent: Some("phase3-data-ops-test".to_string()),
            had_authorization_header: false,
            request_size_bytes: 8,
            request_body_sha256: guard_rail_engine::audit::hash::hash_body(br#"{"ok":1}"#),
            verdict: guard_rail_engine::execution::ExecutionVerdict::Allowed,
            rejection_reason: None,
            matched_policy_name: None,
            matched_rule_field: None,
            matched_rule_condition: None,
            matched_rule_severity: None,
            violation_value_hash: None,
            violation_value_preview: None,
            upstream_status: Some(200),
            forward_error: None,
            latency_inspect_us: 10,
            latency_forward_ms: Some(3),
            latency_total_ms: 4,
            route_config_hash: "route-hash".to_string(),
            policy_set_hash: "policy-hash".to_string(),
        };
        store.insert_execution(&record).await.unwrap();
    }

    sqlx::query(
        r#"
        insert into policy_snapshots (
            snapshot_hash, route_id, route_definition, policies_definition, route_config_hash, policy_set_hash, created_at
        ) values (
            'SNAP-OLD', 'test-route', '{}'::jsonb, '[]'::jsonb, 'route-hash', 'policy-hash', $1
        )
        "#,
    )
    .bind(now - chrono::Duration::days(90))
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        insert into execution_artifacts (
            execution_id, snapshot_hash, request_body_json, request_headers, response_status,
            response_headers, response_body, response_body_sha256, response_body_truncated, created_at
        ) values (
            'GR-OLD-2', 'SNAP-OLD', '{}'::jsonb, '{}'::jsonb, 200,
            '{}'::jsonb, 'ok', 'resp-hash', false, $1
        )
        "#,
    )
    .bind(now - chrono::Duration::days(90))
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        insert into replay_runs (
            id, execution_id, policy_source, evaluated_snapshot_hash, original_verdict, replay_verdict,
            original_policy_name, replay_policy_name, original_rule_field, replay_rule_field, verdict_changed, created_at
        ) values (
            $1, 'GR-OLD-2', 'snapshot', 'SNAP-OLD', 'ALLOWED', 'ALLOWED',
            null, null, null, null, false, $2
        )
        "#,
    )
    .bind(uuid::Uuid::new_v4())
    .bind(now - chrono::Duration::days(90))
    .execute(&pool)
    .await
    .unwrap();

    DataOpsHarness {
        pool,
        _db_guard: db_guard,
    }
}

#[tokio::test]
async fn test_cleanup_preview_reports_candidates_without_deleting() {
    let harness = seed_phase3_data_ops_fixture().await;

    let manager = guard_rail_engine::storage::retention::RetentionManager::new(
        harness.pool.clone(),
        guard_rail_engine::config::DataOpsConfig {
            audit_retention_days: 30,
            artifact_retention_days: 7,
            replay_run_retention_days: 7,
            orphan_snapshot_retention_days: 7,
            cleanup_batch_size: 1000,
        },
    );

    let preview = manager.preview().await.unwrap();

    assert_eq!(preview.audit_rows, 2);
    assert_eq!(preview.execution_artifacts, 1);
    assert_eq!(preview.replay_runs, 1);
    assert_eq!(preview.orphan_policy_snapshots, 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("select count(*) from execution_audit")
            .fetch_one(&harness.pool)
            .await
            .unwrap(),
        3
    );
}

#[tokio::test]
async fn test_cleanup_apply_prunes_old_rows_and_preserves_integrity_boundary() {
    let harness = seed_phase3_data_ops_fixture().await;

    let manager = guard_rail_engine::storage::retention::RetentionManager::new(
        harness.pool.clone(),
        guard_rail_engine::config::DataOpsConfig {
            audit_retention_days: 30,
            artifact_retention_days: 7,
            replay_run_retention_days: 7,
            orphan_snapshot_retention_days: 7,
            cleanup_batch_size: 1000,
        },
    );

    let result = manager.apply().await.unwrap();
    assert_eq!(result.deleted_audit_rows, 2);
    assert_eq!(result.deleted_execution_artifacts, 1);
    assert_eq!(result.deleted_replay_runs, 1);
    assert_eq!(result.deleted_policy_snapshots, 1);
    assert_eq!(result.boundary_execution_id.as_deref(), Some("GR-KEEP-1"));

    let store = guard_rail_engine::storage::postgres::PostgresAuditStore::new(
        harness.pool.clone(),
        std::time::Duration::from_millis(250),
    );

    let integrity = store
        .verify_integrity(guard_rail_engine::audit::api::IntegrityQuery {
            from_execution_id: "GR-KEEP-1".to_string(),
            to_execution_id: "GR-KEEP-1".to_string(),
        })
        .await
        .unwrap();

    assert!(integrity.chain_valid);
}
