use sqlx::{PgPool, QueryBuilder, Row};

pub use sqlx::postgres::PgPoolOptions;

#[derive(Debug, Clone)]
pub struct ExecutionIntentRecord {
    pub execution_id: String,
    pub route_id: String,
    pub tenant_id: Option<uuid::Uuid>,
    pub api_key_id: Option<uuid::Uuid>,
    pub method: String,
    pub source_ip: String,
    pub content_type: Option<String>,
    pub user_agent: Option<String>,
    pub request_size_bytes: usize,
    pub request_body_sha256: String,
    pub route_config_hash: String,
    pub policy_set_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionIntentStatus {
    Finalized,
    FinalizationFailed,
}

impl ExecutionIntentStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Finalized => "finalized",
            Self::FinalizationFailed => "finalization_failed",
        }
    }
}

#[derive(Debug)]
pub enum IntegrityCheckError {
    #[allow(dead_code)]
    MissingExecution(String),
    #[allow(dead_code)]
    ReversedRange {
        from_execution_id: String,
        to_execution_id: String,
    },
    #[allow(dead_code)]
    Storage(sqlx::Error),
}

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
    let audit_table_count = sqlx::query_scalar::<_, i64>(
        "select count(*) from information_schema.tables where table_schema = 'public' and table_name = 'execution_audit'",
    )
    .fetch_one(pool)
    .await?;

    if audit_table_count != 1 {
        return Err(sqlx::Error::Protocol(
            "required audit tables missing".into(),
        ));
    }

    let intent_column_signature_count = sqlx::query_scalar::<_, i64>(
        r#"
        with expected(column_name, data_type, is_nullable) as (
            values
                ('execution_id', 'text', 'NO'),
                ('route_id', 'text', 'NO'),
                ('tenant_id', 'uuid', 'YES'),
                ('api_key_id', 'uuid', 'YES'),
                ('method', 'text', 'NO'),
                ('source_ip', 'text', 'NO'),
                ('content_type', 'text', 'YES'),
                ('user_agent', 'text', 'YES'),
                ('request_size_bytes', 'bigint', 'NO'),
                ('request_body_sha256', 'text', 'NO'),
                ('route_config_hash', 'text', 'NO'),
                ('policy_set_hash', 'text', 'NO'),
                ('status', 'text', 'NO'),
                ('finalization_error', 'text', 'YES'),
                ('created_at', 'timestamp with time zone', 'NO'),
                ('updated_at', 'timestamp with time zone', 'NO'),
                ('finalized_at', 'timestamp with time zone', 'YES')
        )
        select count(*)
        from expected
        join information_schema.columns columns
          on columns.table_schema = 'public'
         and columns.table_name = 'execution_intents'
         and columns.column_name = expected.column_name
         and columns.data_type = expected.data_type
         and columns.is_nullable = expected.is_nullable
        "#,
    )
    .fetch_one(pool)
    .await?;

    if intent_column_signature_count != 17 {
        return Err(sqlx::Error::Protocol(
            "execution_intents schema does not match required column definitions".into(),
        ));
    }

    let intent_foreign_key_count = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from pg_constraint constraints
        join pg_class table_ref on table_ref.oid = constraints.conrelid
        join pg_namespace table_ns on table_ns.oid = table_ref.relnamespace
        join pg_class target_ref on target_ref.oid = constraints.confrelid
        join pg_namespace target_ns on target_ns.oid = target_ref.relnamespace
        where constraints.contype = 'f'
          and table_ns.nspname = 'public'
          and table_ref.relname = 'execution_intents'
          and target_ns.nspname = 'public'
          and target_ref.relname in ('tenants', 'api_keys')
        "#,
    )
    .fetch_one(pool)
    .await?;

    if intent_foreign_key_count != 2 {
        return Err(sqlx::Error::Protocol(
            "execution_intents schema missing required foreign keys".into(),
        ));
    }

    let request_size_check_count = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from pg_constraint constraints
        join pg_class table_ref on table_ref.oid = constraints.conrelid
        join pg_namespace table_ns on table_ns.oid = table_ref.relnamespace
        where constraints.contype = 'c'
          and table_ns.nspname = 'public'
          and table_ref.relname = 'execution_intents'
          and pg_get_constraintdef(constraints.oid) like '%request_size_bytes >= 0%'
        "#,
    )
    .fetch_one(pool)
    .await?;

    if request_size_check_count != 1 {
        return Err(sqlx::Error::Protocol(
            "execution_intents schema missing request size invariant".into(),
        ));
    }

    let terminal_state_check_count = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from pg_constraint constraints
        join pg_class table_ref on table_ref.oid = constraints.conrelid
        join pg_namespace table_ns on table_ns.oid = table_ref.relnamespace
        where constraints.contype = 'c'
          and table_ns.nspname = 'public'
          and table_ref.relname = 'execution_intents'
          and pg_get_constraintdef(constraints.oid) like '%finalization_error is null%'
          and pg_get_constraintdef(constraints.oid) like '%finalized_at is not null%'
          and pg_get_constraintdef(constraints.oid) like '%finalization_failed%'
        "#,
    )
    .fetch_one(pool)
    .await?;

    if terminal_state_check_count != 1 {
        return Err(sqlx::Error::Protocol(
            "execution_intents schema missing terminal state invariant".into(),
        ));
    }

    Ok(())
}

#[derive(Clone)]
pub struct PostgresAuditStore {
    pool: PgPool,
    write_timeout: std::time::Duration,
}

const AUDIT_CHAIN_LOCK_KEY: i64 = 0x4752_4149;

impl PostgresAuditStore {
    pub fn new(pool: PgPool, write_timeout: std::time::Duration) -> Self {
        Self {
            pool,
            write_timeout,
        }
    }

    pub async fn readiness_check(&self) -> Result<(), sqlx::Error> {
        sqlx::query_scalar::<_, i32>("select 1")
            .fetch_one(&self.pool)
            .await
            .map(|_| ())
    }

    pub async fn insert_execution(
        &self,
        record: &crate::execution::ExecutionRecord,
    ) -> Result<(), sqlx::Error> {
        let result =
            tokio::time::timeout(self.write_timeout, self.append_execution_row(record)).await;
        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(sqlx::Error::Protocol("audit insert timed out".into())),
        }
    }

    pub async fn insert_execution_intent(
        &self,
        intent: &ExecutionIntentRecord,
    ) -> Result<(), sqlx::Error> {
        let request_size_bytes = i64::try_from(intent.request_size_bytes).map_err(|_| {
            sqlx::Error::Protocol("execution intent request size exceeds bigint".into())
        })?;
        let result = tokio::time::timeout(
            self.write_timeout,
            sqlx::query(
                r#"
                insert into execution_intents (
                    execution_id, route_id, tenant_id, api_key_id, method, source_ip,
                    content_type, user_agent, request_size_bytes, request_body_sha256,
                    route_config_hash, policy_set_hash, status
                ) values (
                    $1, $2, $3, $4, $5, $6,
                    $7, $8, $9, $10,
                    $11, $12, 'pending'
                )
                "#,
            )
            .bind(&intent.execution_id)
            .bind(&intent.route_id)
            .bind(&intent.tenant_id)
            .bind(&intent.api_key_id)
            .bind(&intent.method)
            .bind(&intent.source_ip)
            .bind(&intent.content_type)
            .bind(&intent.user_agent)
            .bind(request_size_bytes)
            .bind(&intent.request_body_sha256)
            .bind(&intent.route_config_hash)
            .bind(&intent.policy_set_hash)
            .execute(&self.pool),
        )
        .await;

        match result {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(sqlx::Error::Protocol(
                "execution intent insert timed out".into(),
            )),
        }
    }

    pub async fn update_execution_intent_status(
        &self,
        execution_id: &str,
        status: ExecutionIntentStatus,
        finalization_error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let status_value = status.as_str();
        let (finalization_error, finalized_at) = match status {
            ExecutionIntentStatus::Finalized => {
                if finalization_error.is_some() {
                    return Err(sqlx::Error::Protocol(
                        "finalized execution intents cannot include a finalization error".into(),
                    ));
                }
                (None, Some(chrono::Utc::now()))
            }
            ExecutionIntentStatus::FinalizationFailed => {
                let Some(finalization_error) = finalization_error else {
                    return Err(sqlx::Error::Protocol(
                        "finalization_failed execution intents require a finalization error"
                            .into(),
                    ));
                };
                (Some(finalization_error), None)
            }
        };

        let result = tokio::time::timeout(
            self.write_timeout,
            sqlx::query(
                r#"
                update execution_intents
                set status = $2,
                    finalization_error = $3,
                    finalized_at = $4,
                    updated_at = now()
                where execution_id = $1
                  and status = 'pending'
                "#,
            )
            .bind(execution_id)
            .bind(status_value)
            .bind(finalization_error)
            .bind(finalized_at)
            .execute(&self.pool),
        )
        .await;

        match result {
            Ok(Ok(query_result)) => {
                if query_result.rows_affected() == 1 {
                    Ok(())
                } else {
                    Err(sqlx::Error::Protocol(
                        "execution intent status update affected no pending rows".into(),
                    ))
                }
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(sqlx::Error::Protocol(
                "execution intent status update timed out".into(),
            )),
        }
    }

    async fn append_execution_row(
        &self,
        record: &crate::execution::ExecutionRecord,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("select pg_advisory_xact_lock($1)")
            .bind(AUDIT_CHAIN_LOCK_KEY)
            .execute(&mut *tx)
            .await?;

        let previous_hash: Option<String> =
            sqlx::query_scalar("select record_hash from execution_audit order by id desc limit 1")
                .fetch_optional(&mut *tx)
                .await?;

        let record_hash = crate::audit::hash::record_hash(record, previous_hash.as_deref());

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
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
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
                latency_forward_ms: r
                    .get::<Option<i64>, _>("latency_forward_ms")
                    .map(|v| v as u128),
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

    #[allow(dead_code)]
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

    fn push_audit_filters<'a>(
        query: &'a crate::audit::api::AuditListQuery,
        effective_tenant_id: Option<uuid::Uuid>,
        builder: &mut QueryBuilder<'a, sqlx::Postgres>,
    ) {
        builder.push(" where 1=1");

        if let Some(tid) = effective_tenant_id {
            builder.push(" and tenant_id = ").push_bind(tid);
        }
        if let Some(route_id) = &query.route_id {
            builder.push(" and route_id = ").push_bind(route_id);
        }
        if let Some(verdict) = &query.verdict {
            builder.push(" and verdict = ").push_bind(verdict);
        }
        if let Some(from) = query.from {
            builder
                .push(" and execution_started_at >= ")
                .push_bind(from);
        }
        if let Some(to) = query.to {
            builder.push(" and execution_started_at <= ").push_bind(to);
        }
    }

    pub async fn list_executions_for_tenant(
        &self,
        tenant_id: Option<uuid::Uuid>,
        query: crate::audit::api::AuditListQuery,
    ) -> Result<crate::audit::api::AuditListResponse, sqlx::Error> {
        let limit = query.limit.unwrap_or(50).min(1000);
        let offset = query.cursor.unwrap_or(0);
        let order_sql = if query.order.as_deref() == Some("asc") {
            "ASC"
        } else {
            "DESC"
        };
        let effective_tenant_id = tenant_id.or(query.tenant_id);

        let column_list = "execution_id, execution_started_at, route_id, tenant_id, api_key_id, auth_outcome, \
             upstream_url, method, source_ip, content_type, user_agent, \
             had_authorization_header, request_size_bytes, request_body_sha256, verdict, rejection_reason, matched_policy_name, \
             matched_rule_field, matched_rule_condition, matched_rule_severity, violation_value_hash, \
             violation_value_preview, upstream_status, forward_error, latency_inspect_us, latency_forward_ms, \
             latency_total_ms, route_config_hash, policy_set_hash, previous_hash, record_hash";

        let mut select = QueryBuilder::<sqlx::Postgres>::new(format!(
            "select {column_list} from execution_audit"
        ));
        Self::push_audit_filters(&query, effective_tenant_id, &mut select);
        select
            .push(" order by execution_started_at ")
            .push(order_sql)
            .push(", id ")
            .push(order_sql)
            .push(" limit ")
            .push_bind(limit)
            .push(" offset ")
            .push_bind(offset);

        let rows = select.build().fetch_all(&self.pool).await?;

        let mut count = QueryBuilder::<sqlx::Postgres>::new("select count(*) from execution_audit");
        Self::push_audit_filters(&query, effective_tenant_id, &mut count);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;

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
        self.insert_execution(record).await?;

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
        params: &crate::replay::engine::ReplayRunParams,
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
            .bind(&params.id)
            .bind(&params.execution_id)
            .bind(&params.policy_source)
            .bind(&params.evaluated_snapshot_hash)
            .bind(&params.original_verdict)
            .bind(&params.replay_verdict)
            .bind(&params.original_policy_name)
            .bind(&params.replay_policy_name)
            .bind(&params.original_rule_field)
            .bind(&params.replay_rule_field)
            .bind(params.verdict_changed)
            .execute(&self.pool),
        )
        .await
        .map_err(|_| sqlx::Error::Protocol("replay run insert timed out".into()))??;

        Ok(())
    }

    pub async fn verify_integrity(
        &self,
        query: crate::audit::api::IntegrityQuery,
    ) -> Result<crate::audit::api::IntegrityResponse, IntegrityCheckError> {
        let from_row =
            sqlx::query("select id, execution_id from execution_audit where execution_id = $1")
                .bind(&query.from_execution_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(IntegrityCheckError::Storage)?
                .ok_or_else(|| {
                    IntegrityCheckError::MissingExecution(query.from_execution_id.clone())
                })?;
        let to_row =
            sqlx::query("select id, execution_id from execution_audit where execution_id = $1")
                .bind(&query.to_execution_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(IntegrityCheckError::Storage)?
                .ok_or_else(|| {
                    IntegrityCheckError::MissingExecution(query.to_execution_id.clone())
                })?;

        let from_id: i64 = from_row.get("id");
        let to_id: i64 = to_row.get("id");
        if from_id > to_id {
            return Err(IntegrityCheckError::ReversedRange {
                from_execution_id: query.from_execution_id,
                to_execution_id: query.to_execution_id,
            });
        }

        let predecessor_hash: Option<String> =
            match sqlx::query_scalar("select record_hash from execution_audit where id < $1 order by id desc limit 1")
                .bind(from_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(IntegrityCheckError::Storage)?
            {
                Some(hash) => Some(hash),
                None => sqlx::query_scalar(
                    "select deleted_through_record_hash from audit_retention_checkpoints where boundary_execution_id = $1",
                )
                .bind(&query.from_execution_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(IntegrityCheckError::Storage)?,
            };

        let rows = sqlx::query(
            r#"
            select id, execution_id, previous_hash, record_hash
            from execution_audit
            where id between $1 and $2
            order by id asc
            "#,
        )
        .bind(from_id)
        .bind(to_id)
        .fetch_all(&self.pool)
        .await
        .map_err(IntegrityCheckError::Storage)?;

        let mut expected_previous = predecessor_hash;
        for row in rows {
            let execution_id: String = row.get("execution_id");
            let previous_hash: Option<String> = row.get("previous_hash");
            let record_hash: String = row.get("record_hash");

            if previous_hash != expected_previous {
                return Ok(crate::audit::api::IntegrityResponse {
                    chain_valid: false,
                    first_invalid_record: Some(execution_id),
                    checked_from: query.from_execution_id.clone(),
                    checked_to: query.to_execution_id.clone(),
                });
            }

            expected_previous = Some(record_hash);
        }

        Ok(crate::audit::api::IntegrityResponse {
            chain_valid: true,
            first_invalid_record: None,
            checked_from: query.from_execution_id,
            checked_to: query.to_execution_id,
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
