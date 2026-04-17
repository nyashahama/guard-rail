use sqlx::{PgPool, Row};

pub use sqlx::postgres::PgPoolOptions;

pub async fn connect_pool(config: &crate::config::DatabaseConfig) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .connect(&config.url)
        .await
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

pub async fn assert_schema_ready(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "select count(*) from information_schema.tables where table_name = 'execution_audit'",
    )
    .fetch_one(pool)
    .await
    .and_then(|count| {
        if count == 1 {
            Ok(())
        } else {
            Err(sqlx::Error::Protocol(
                "execution_audit table missing".into(),
            ))
        }
    })
}

#[derive(Clone)]
pub struct PostgresAuditStore {
    pool: PgPool,
    write_timeout: std::time::Duration,
}

impl PostgresAuditStore {
    pub fn new(pool: PgPool, write_timeout: std::time::Duration) -> Self {
        Self {
            pool,
            write_timeout,
        }
    }

    pub async fn insert_execution(
        &self,
        record: &crate::execution::ExecutionRecord,
    ) -> Result<(), sqlx::Error> {
        let previous_hash: Option<String> =
            sqlx::query_scalar("select record_hash from execution_audit order by id desc limit 1")
                .fetch_optional(&self.pool)
                .await?;

        let record_hash = crate::audit::hash::record_hash(record, previous_hash.as_deref());

        tokio::time::timeout(
            self.write_timeout,
            sqlx::query(
                r#"
                insert into execution_audit (
                    execution_id, execution_started_at, route_id, tenant_id, api_key_id, auth_outcome,
                    upstream_url, method, source_ip, content_type, user_agent,
                    had_authorization_header, request_size_bytes, request_body_sha256,
                    verdict, rejection_reason, matched_policy_name, matched_rule_field,
                    matched_rule_condition, matched_rule_severity, violation_value_hash,
                    violation_value_preview, upstream_status, forward_error,
                    latency_inspect_us, latency_forward_ms, latency_total_ms,
                    route_config_hash, policy_set_hash, previous_hash, record_hash
                ) values (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                    $11, $12, $13, $14, $15, $16, $17, $18,
                    $19, $20, $21, $22, $23, $24,
                    $25, $26, $27, $28, $29, $30, $31
                )
                "#,
            )
            .bind(&record.execution_id)
            .bind(record.execution_started_at)
            .bind(&record.route_id)
            .bind(record.tenant_id)
            .bind(record.api_key_id)
            .bind(&record.auth_outcome)
            .bind(&record.upstream_url)
            .bind(&record.method)
            .bind(&record.source_ip)
            .bind(&record.content_type)
            .bind(&record.user_agent)
            .bind(record.had_authorization_header)
            .bind(record.request_size_bytes as i64)
            .bind(&record.request_body_sha256)
            .bind(match record.verdict {
                crate::execution::ExecutionVerdict::Rejected => "REJECTED",
                crate::execution::ExecutionVerdict::Blocked => "BLOCKED",
                crate::execution::ExecutionVerdict::Allowed => "ALLOWED",
            })
            .bind(&record.rejection_reason)
            .bind(&record.matched_policy_name)
            .bind(&record.matched_rule_field)
            .bind(&record.matched_rule_condition)
            .bind(&record.matched_rule_severity)
            .bind(&record.violation_value_hash)
            .bind(&record.violation_value_preview)
            .bind(record.upstream_status.map(i32::from))
            .bind(&record.forward_error)
            .bind(record.latency_inspect_us as i64)
            .bind(record.latency_forward_ms.map(|v| v as i64))
            .bind(record.latency_total_ms as i64)
            .bind(&record.route_config_hash)
            .bind(&record.policy_set_hash)
            .bind(&previous_hash)
            .bind(record_hash)
            .execute(&self.pool),
        )
        .await
        .map_err(|_| sqlx::Error::Protocol("audit insert timed out".into()))??;

        Ok(())
    }

    pub async fn get_execution_by_id(
        &self,
        execution_id: &str,
    ) -> Result<Option<ExecutionAuditRow>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            select
                execution_id, execution_started_at, route_id, tenant_id, api_key_id, auth_outcome,
                upstream_url, method, source_ip, content_type, user_agent,
                had_authorization_header, request_size_bytes, request_body_sha256,
                verdict, rejection_reason, matched_policy_name, matched_rule_field,
                matched_rule_condition, matched_rule_severity, violation_value_hash,
                violation_value_preview, upstream_status, forward_error,
                latency_inspect_us, latency_forward_ms, latency_total_ms,
                route_config_hash, policy_set_hash, previous_hash, record_hash
            from execution_audit
            where execution_id = $1
            "#,
        )
        .bind(execution_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| ExecutionAuditRow {
            execution_id: r.get("execution_id"),
            execution_started_at: r.get("execution_started_at"),
            route_id: r.get("route_id"),
            tenant_id: r.get("tenant_id"),
            api_key_id: r.get("api_key_id"),
            auth_outcome: r.get("auth_outcome"),
            upstream_url: r.get("upstream_url"),
            method: r.get("method"),
            source_ip: r.get("source_ip"),
            content_type: r.get("content_type"),
            user_agent: r.get("user_agent"),
            had_authorization_header: r.get("had_authorization_header"),
            request_size_bytes: r.get::<i64, _>("request_size_bytes") as usize,
            request_body_sha256: r.get("request_body_sha256"),
            verdict: r.get("verdict"),
            rejection_reason: r.get("rejection_reason"),
            matched_policy_name: r.get("matched_policy_name"),
            matched_rule_field: r.get("matched_rule_field"),
            matched_rule_condition: r.get("matched_rule_condition"),
            matched_rule_severity: r.get("matched_rule_severity"),
            violation_value_hash: r.get("violation_value_hash"),
            violation_value_preview: r.get("violation_value_preview"),
            upstream_status: r.get::<Option<i32>, _>("upstream_status").map(|v| v as u16),
            forward_error: r.get("forward_error"),
            latency_inspect_us: r.get::<i64, _>("latency_inspect_us") as u128,
            latency_forward_ms: r
                .get::<Option<i64>, _>("latency_forward_ms")
                .map(|v| v as u128),
            latency_total_ms: r.get::<i64, _>("latency_total_ms") as u128,
            route_config_hash: r.get("route_config_hash"),
            policy_set_hash: r.get("policy_set_hash"),
            previous_hash: r.get("previous_hash"),
            record_hash: r.get("record_hash"),
        }))
    }

    pub async fn get_execution_detail(
        &self,
        execution_id: &str,
    ) -> Result<Option<crate::audit::api::ExecutionAuditDetail>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            select
                ea.execution_id, ea.execution_started_at, ea.route_id, ea.tenant_id, ea.api_key_id, ea.auth_outcome,
                ea.upstream_url, ea.method, ea.source_ip, ea.content_type, ea.user_agent,
                ea.had_authorization_header, ea.request_size_bytes, ea.request_body_sha256,
                ea.verdict, ea.rejection_reason, ea.matched_policy_name, ea.matched_rule_field,
                ea.matched_rule_condition, ea.matched_rule_severity, ea.violation_value_hash,
                ea.violation_value_preview, ea.upstream_status, ea.forward_error,
                ea.latency_inspect_us, ea.latency_forward_ms, ea.latency_total_ms,
                ea.route_config_hash, ea.policy_set_hash, ea.previous_hash, ea.record_hash,
                art.snapshot_hash,
                exists(select 1 from execution_artifacts where execution_id = ea.execution_id) as replay_available,
                (
                    select rr.id
                    from replay_runs rr
                    where rr.execution_id = ea.execution_id
                    order by rr.created_at desc
                    limit 1
                ) as latest_replay_run_id
            from execution_audit ea
            left join execution_artifacts art on art.execution_id = ea.execution_id
            where ea.execution_id = $1
            "#,
        )
        .bind(execution_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| crate::audit::api::ExecutionAuditDetail {
            row: ExecutionAuditRow {
                execution_id: r.get("execution_id"),
                execution_started_at: r.get("execution_started_at"),
                route_id: r.get("route_id"),
                tenant_id: r.get("tenant_id"),
                api_key_id: r.get("api_key_id"),
                auth_outcome: r.get("auth_outcome"),
                upstream_url: r.get("upstream_url"),
                method: r.get("method"),
                source_ip: r.get("source_ip"),
                content_type: r.get("content_type"),
                user_agent: r.get("user_agent"),
                had_authorization_header: r.get("had_authorization_header"),
                request_size_bytes: r.get::<i64, _>("request_size_bytes") as usize,
                request_body_sha256: r.get("request_body_sha256"),
                verdict: r.get("verdict"),
                rejection_reason: r.get("rejection_reason"),
                matched_policy_name: r.get("matched_policy_name"),
                matched_rule_field: r.get("matched_rule_field"),
                matched_rule_condition: r.get("matched_rule_condition"),
                matched_rule_severity: r.get("matched_rule_severity"),
                violation_value_hash: r.get("violation_value_hash"),
                violation_value_preview: r.get("violation_value_preview"),
                upstream_status: r.get::<Option<i32>, _>("upstream_status").map(|v| v as u16),
                forward_error: r.get("forward_error"),
                latency_inspect_us: r.get::<i64, _>("latency_inspect_us") as u128,
                latency_forward_ms: r.get::<Option<i64>, _>("latency_forward_ms").map(|v| v as u128),
                latency_total_ms: r.get::<i64, _>("latency_total_ms") as u128,
                route_config_hash: r.get("route_config_hash"),
                policy_set_hash: r.get("policy_set_hash"),
                previous_hash: r.get("previous_hash"),
                record_hash: r.get("record_hash"),
            },
            replay_available: r.get("replay_available"),
            snapshot_hash: r.get("snapshot_hash"),
            latest_replay_run_id: r.get("latest_replay_run_id"),
        }))
    }

    pub async fn count_executions(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("select count(*) from execution_audit")
            .fetch_one(&self.pool)
            .await
    }

    pub async fn list_executions(
        &self,
        query: crate::audit::api::AuditListQuery,
        access: crate::auth::context::AuditAccess,
    ) -> Result<crate::audit::api::AuditListResponse, sqlx::Error> {
        match access {
            crate::auth::context::AuditAccess::Admin => {
                self.list_executions_for_tenant(query.tenant_id, query)
                    .await
            }
            crate::auth::context::AuditAccess::Tenant { tenant_id } => {
                self.list_executions_for_tenant(Some(tenant_id), query)
                    .await
            }
        }
    }

    pub async fn list_executions_for_tenant(
        &self,
        tenant_id: Option<uuid::Uuid>,
        query: crate::audit::api::AuditListQuery,
    ) -> Result<crate::audit::api::AuditListResponse, sqlx::Error> {
        let limit = query.limit.unwrap_or(50).min(1000);
        let offset = query.cursor.unwrap_or(0);
        let order = query.order.as_deref().unwrap_or("desc");

        let order_sql = if order == "asc" { "ASC" } else { "DESC" };

        let (base_sql, bind_tenant_id) = if let Some(tid) = tenant_id.or(query.tenant_id) {
            (
                r#"
                select
                    execution_id, execution_started_at, route_id, tenant_id, api_key_id, auth_outcome,
                    upstream_url, method, source_ip, content_type, user_agent,
                    had_authorization_header, request_size_bytes, request_body_sha256,
                    verdict, rejection_reason, matched_policy_name, matched_rule_field,
                    matched_rule_condition, matched_rule_severity, violation_value_hash,
                    violation_value_preview, upstream_status, forward_error,
                    latency_inspect_us, latency_forward_ms, latency_total_ms,
                    route_config_hash, policy_set_hash, previous_hash, record_hash
                from execution_audit
                where tenant_id = $1
                order by execution_started_at "#,
                Some(tid),
            )
        } else {
            (
                r#"
                select
                    execution_id, execution_started_at, route_id, tenant_id, api_key_id, auth_outcome,
                    upstream_url, method, source_ip, content_type, user_agent,
                    had_authorization_header, request_size_bytes, request_body_sha256,
                    verdict, rejection_reason, matched_policy_name, matched_rule_field,
                    matched_rule_condition, matched_rule_severity, violation_value_hash,
                    violation_value_preview, upstream_status, forward_error,
                    latency_inspect_us, latency_forward_ms, latency_total_ms,
                    route_config_hash, policy_set_hash, previous_hash, record_hash
                from execution_audit
                order by execution_started_at "#,
                None,
            )
        };

        let sql = format!(
            "{} {} limit {} offset {}",
            base_sql, order_sql, limit, offset
        );

        let rows = if let Some(tid) = bind_tenant_id {
            sqlx::query(&sql).bind(tid).fetch_all(&self.pool).await?
        } else {
            sqlx::query(&sql).fetch_all(&self.pool).await?
        };

        let items: Vec<ExecutionAuditRow> = rows
            .iter()
            .map(|r| ExecutionAuditRow {
                execution_id: r.get("execution_id"),
                execution_started_at: r.get("execution_started_at"),
                route_id: r.get("route_id"),
                tenant_id: r.get("tenant_id"),
                api_key_id: r.get("api_key_id"),
                auth_outcome: r.get("auth_outcome"),
                upstream_url: r.get("upstream_url"),
                method: r.get("method"),
                source_ip: r.get("source_ip"),
                content_type: r.get("content_type"),
                user_agent: r.get("user_agent"),
                had_authorization_header: r.get("had_authorization_header"),
                request_size_bytes: r.get::<i64, _>("request_size_bytes") as usize,
                request_body_sha256: r.get("request_body_sha256"),
                verdict: r.get("verdict"),
                rejection_reason: r.get("rejection_reason"),
                matched_policy_name: r.get("matched_policy_name"),
                matched_rule_field: r.get("matched_rule_field"),
                matched_rule_condition: r.get("matched_rule_condition"),
                matched_rule_severity: r.get("matched_rule_severity"),
                violation_value_hash: r.get("violation_value_hash"),
                violation_value_preview: r.get("violation_value_preview"),
                upstream_status: r.get::<Option<i32>, _>("upstream_status").map(|v| v as u16),
                forward_error: r.get("forward_error"),
                latency_inspect_us: r.get::<i64, _>("latency_inspect_us") as u128,
                latency_forward_ms: r
                    .get::<Option<i64>, _>("latency_forward_ms")
                    .map(|v| v as u128),
                latency_total_ms: r.get::<i64, _>("latency_total_ms") as u128,
                route_config_hash: r.get("route_config_hash"),
                policy_set_hash: r.get("policy_set_hash"),
                previous_hash: r.get("previous_hash"),
                record_hash: r.get("record_hash"),
            })
            .collect();

        let total = if let Some(_tid) = bind_tenant_id {
            sqlx::query_scalar::<_, i64>(
                "select count(*) from execution_audit where tenant_id = $1",
            )
            .bind(bind_tenant_id.unwrap())
            .fetch_one(&self.pool)
            .await?
        } else {
            self.count_executions().await?
        };

        let next_cursor = if items.len() as i64 == limit {
            Some(offset + limit)
        } else {
            None
        };

        Ok(crate::audit::api::AuditListResponse {
            items,
            total,
            next_cursor,
        })
    }

    pub async fn insert_execution_bundle(
        &self,
        record: &crate::execution::ExecutionRecord,
        artifacts: Option<&crate::proxy::ReplayArtifacts>,
        snapshot: Option<&crate::replay::snapshot::PolicySnapshotRecord>,
    ) -> Result<(), sqlx::Error> {
        if let Some(snap) = snapshot {
            tokio::time::timeout(
                self.write_timeout,
                sqlx::query(
                    r#"
                    insert into policy_snapshots (
                        snapshot_hash, route_id, route_definition, policies_definition,
                        route_config_hash, policy_set_hash
                    ) values ($1, $2, $3, $4, $5, $6)
                    on conflict (snapshot_hash) do nothing
                    "#,
                )
                .bind(&snap.snapshot_hash)
                .bind(&snap.route_id)
                .bind(&snap.route_definition)
                .bind(&snap.policies_definition)
                .bind(&snap.route_config_hash)
                .bind(&snap.policy_set_hash)
                .execute(&self.pool),
            )
            .await
            .map_err(|_| sqlx::Error::Protocol("snapshot insert timed out".into()))??;
        }

        if let Some(art) = artifacts {
            tokio::time::timeout(
                self.write_timeout,
                sqlx::query(
                    r#"
                    insert into execution_artifacts (
                        execution_id, snapshot_hash, request_body_json, request_headers,
                        response_status, response_headers, response_body,
                        response_body_sha256, response_body_truncated
                    ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    on conflict (execution_id) do update set
                        snapshot_hash = excluded.snapshot_hash,
                        request_body_json = excluded.request_body_json,
                        request_headers = excluded.request_headers,
                        response_status = excluded.response_status,
                        response_headers = excluded.response_headers,
                        response_body = excluded.response_body,
                        response_body_sha256 = excluded.response_body_sha256,
                        response_body_truncated = excluded.response_body_truncated
                    "#,
                )
                .bind(&record.execution_id)
                .bind(&art.snapshot_hash)
                .bind(&art.request_body_json)
                .bind(&art.request_headers)
                .bind(art.response_status.map(i32::from))
                .bind(&art.response_headers)
                .bind(&art.response_body)
                .bind(&art.response_body_sha256)
                .bind(art.response_body_truncated)
                .execute(&self.pool),
            )
            .await
            .map_err(|_| sqlx::Error::Protocol("artifact insert timed out".into()))??;
        }

        Ok(())
    }

    pub async fn get_execution_artifacts(
        &self,
        execution_id: &str,
    ) -> Result<Option<ExecutionArtifactRow>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            select
                ea.execution_id, ea.snapshot_hash, ea.request_body_json, ea.request_headers,
                ea.response_status, ea.response_headers, ea.response_body,
                ea.response_body_sha256, ea.response_body_truncated, ea.created_at,
                ps.route_definition, ps.policies_definition
            from execution_artifacts ea
            join policy_snapshots ps on ps.snapshot_hash = ea.snapshot_hash
            where ea.execution_id = $1
            "#,
        )
        .bind(execution_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| ExecutionArtifactRow {
            execution_id: r.get("execution_id"),
            snapshot_hash: r.get("snapshot_hash"),
            request_body_json: r.get("request_body_json"),
            request_headers: r.get("request_headers"),
            response_status: r.get::<Option<i32>, _>("response_status").map(|v| v as u16),
            response_headers: r.get("response_headers"),
            response_body: r.get("response_body"),
            response_body_sha256: r.get("response_body_sha256"),
            response_body_truncated: r.get("response_body_truncated"),
            created_at: r.get("created_at"),
            route_definition: r.get("route_definition"),
            policies_definition: r.get("policies_definition"),
        }))
    }

    pub async fn insert_replay_run(
        &self,
        id: &str,
        execution_id: &str,
        policy_source: &str,
        evaluated_snapshot_hash: &str,
        original_verdict: &str,
        replay_verdict: &str,
        original_policy_name: Option<&str>,
        replay_policy_name: Option<&str>,
        original_rule_field: Option<&str>,
        replay_rule_field: Option<&str>,
        verdict_changed: bool,
    ) -> Result<(), sqlx::Error> {
        tokio::time::timeout(
            self.write_timeout,
            sqlx::query(
                r#"
                insert into replay_runs (
                    id, execution_id, policy_source, evaluated_snapshot_hash,
                    original_verdict, replay_verdict, original_policy_name,
                    replay_policy_name, original_rule_field, replay_rule_field,
                    verdict_changed
                ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                "#,
            )
            .bind(id)
            .bind(execution_id)
            .bind(policy_source)
            .bind(evaluated_snapshot_hash)
            .bind(original_verdict)
            .bind(replay_verdict)
            .bind(original_policy_name)
            .bind(replay_policy_name)
            .bind(original_rule_field)
            .bind(replay_rule_field)
            .bind(verdict_changed)
            .execute(&self.pool),
        )
        .await
        .map_err(|_| sqlx::Error::Protocol("replay run insert timed out".into()))??;

        Ok(())
    }

    pub async fn verify_integrity(
        &self,
        query: crate::audit::api::IntegrityQuery,
    ) -> Result<crate::audit::api::IntegrityResponse, sqlx::Error> {
        let from_id = query.from_execution_id.clone();
        let to_id = query.to_execution_id.clone();

        let from_row = self.get_execution_by_id(&from_id).await?;
        let to_row = self.get_execution_by_id(&to_id).await?;

        let mut chain_valid = true;
        let mut first_invalid_record: Option<String> = None;

        if let (Some(from), Some(_to)) = (from_row, to_row) {
            let mut current = Some(from.record_hash.clone());

            let rows = sqlx::query(
                r#"
                select record_hash, previous_hash from execution_audit
                where execution_id >= $1 and execution_id <= $2
                order by execution_started_at asc
                "#,
            )
            .bind(&from_id)
            .bind(&to_id)
            .fetch_all(&self.pool)
            .await?;

            for row in rows {
                let record_hash: String = row.get("record_hash");
                let previous_hash: Option<String> = row.get("previous_hash");

                if previous_hash.as_ref() != current.as_ref() {
                    chain_valid = false;
                    first_invalid_record = Some(record_hash);
                    break;
                }
                current = Some(record_hash);
            }
        }

        Ok(crate::audit::api::IntegrityResponse {
            chain_valid,
            first_invalid_record,
            checked_from: from_id,
            checked_to: to_id,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutionAuditRow {
    pub execution_id: String,
    pub execution_started_at: chrono::DateTime<chrono::Utc>,
    pub route_id: String,
    pub tenant_id: Option<uuid::Uuid>,
    pub api_key_id: Option<uuid::Uuid>,
    pub auth_outcome: Option<String>,
    pub upstream_url: Option<String>,
    pub method: String,
    pub source_ip: String,
    pub content_type: Option<String>,
    pub user_agent: Option<String>,
    pub had_authorization_header: bool,
    pub request_size_bytes: usize,
    pub request_body_sha256: String,
    pub verdict: String,
    pub rejection_reason: Option<String>,
    pub matched_policy_name: Option<String>,
    pub matched_rule_field: Option<String>,
    pub matched_rule_condition: Option<String>,
    pub matched_rule_severity: Option<String>,
    pub violation_value_hash: Option<String>,
    pub violation_value_preview: Option<String>,
    pub upstream_status: Option<u16>,
    pub forward_error: Option<String>,
    pub latency_inspect_us: u128,
    pub latency_forward_ms: Option<u128>,
    pub latency_total_ms: u128,
    pub route_config_hash: String,
    pub policy_set_hash: String,
    pub previous_hash: Option<String>,
    pub record_hash: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutionArtifactRow {
    pub execution_id: String,
    pub snapshot_hash: String,
    pub request_body_json: serde_json::Value,
    pub request_headers: serde_json::Value,
    pub response_status: Option<u16>,
    pub response_headers: Option<serde_json::Value>,
    pub response_body: Option<String>,
    pub response_body_sha256: Option<String>,
    pub response_body_truncated: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub route_definition: serde_json::Value,
    pub policies_definition: serde_json::Value,
}
