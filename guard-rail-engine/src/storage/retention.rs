use sqlx::Row;

const AUDIT_CHAIN_LOCK_KEY: i64 = 0x4752_4149;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CleanupPreview {
    pub audit_rows: i64,
    pub execution_artifacts: i64,
    pub replay_runs: i64,
    pub orphan_policy_snapshots: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CleanupResult {
    pub deleted_audit_rows: i64,
    pub deleted_execution_artifacts: i64,
    pub deleted_replay_runs: i64,
    pub deleted_policy_snapshots: i64,
    pub boundary_execution_id: Option<String>,
}

#[derive(Clone)]
pub struct RetentionManager {
    pool: sqlx::PgPool,
    config: crate::config::DataOpsConfig,
}

impl RetentionManager {
    pub fn new(pool: sqlx::PgPool, config: crate::config::DataOpsConfig) -> Self {
        Self { pool, config }
    }

    pub async fn preview(&self) -> Result<CleanupPreview, sqlx::Error> {
        let now = chrono::Utc::now();
        let replay_cutoff = now - chrono::Duration::days(self.config.replay_run_retention_days as i64);
        let artifact_cutoff = now - chrono::Duration::days(self.config.artifact_retention_days as i64);
        let audit_cutoff = now - chrono::Duration::days(self.config.audit_retention_days as i64);
        let snapshot_cutoff = now - chrono::Duration::days(self.config.orphan_snapshot_retention_days as i64);

        Ok(CleanupPreview {
            replay_runs: sqlx::query_scalar("select count(*) from replay_runs where created_at < $1")
                .bind(replay_cutoff)
                .fetch_one(&self.pool)
                .await?,
            execution_artifacts: sqlx::query_scalar("select count(*) from execution_artifacts where created_at < $1")
                .bind(artifact_cutoff)
                .fetch_one(&self.pool)
                .await?,
            audit_rows: sqlx::query_scalar("select count(*) from execution_audit where execution_started_at < $1")
                .bind(audit_cutoff)
                .fetch_one(&self.pool)
                .await?,
            orphan_policy_snapshots: sqlx::query_scalar(
                "select count(*) from policy_snapshots ps where ps.created_at < $1 and not exists (select 1 from execution_artifacts ea where ea.snapshot_hash = ps.snapshot_hash and ea.created_at >= $2)"
            )
            .bind(snapshot_cutoff)
            .bind(artifact_cutoff)
            .fetch_one(&self.pool)
            .await?,
        })
    }

    pub async fn apply(&self) -> Result<CleanupResult, sqlx::Error> {
        let now = chrono::Utc::now();
        let replay_cutoff = now - chrono::Duration::days(self.config.replay_run_retention_days as i64);
        let artifact_cutoff = now - chrono::Duration::days(self.config.artifact_retention_days as i64);
        let audit_cutoff = now - chrono::Duration::days(self.config.audit_retention_days as i64);
        let snapshot_cutoff = now - chrono::Duration::days(self.config.orphan_snapshot_retention_days as i64);

        let mut tx = self.pool.begin().await?;

        let deleted_replay_runs = sqlx::query("delete from replay_runs where created_at < $1")
            .bind(replay_cutoff)
            .execute(&mut *tx)
            .await?
            .rows_affected() as i64;

        let deleted_execution_artifacts =
            sqlx::query("delete from execution_artifacts where created_at < $1")
                .bind(artifact_cutoff)
                .execute(&mut *tx)
                .await?
                .rows_affected() as i64;

        let deleted_policy_snapshots = sqlx::query(
            r#"
            delete from policy_snapshots ps
            where ps.created_at < $1
              and not exists (
                  select 1 from execution_artifacts ea
                  where ea.snapshot_hash = ps.snapshot_hash
              )
            "#,
        )
        .bind(snapshot_cutoff)
        .execute(&mut *tx)
        .await?
        .rows_affected() as i64;

        sqlx::query("select pg_advisory_xact_lock($1)")
            .bind(AUDIT_CHAIN_LOCK_KEY)
            .execute(&mut *tx)
            .await?;

        let prune_rows = sqlx::query(
            r#"
            select id, execution_id, record_hash
            from execution_audit
            where execution_started_at < $1
            order by id asc
            limit $2
            "#,
        )
        .bind(audit_cutoff)
        .bind(self.config.cleanup_batch_size as i64)
        .fetch_all(&mut *tx)
        .await?;

        let mut boundary_execution_id = None;
        let mut deleted_audit_rows = 0i64;

        if let Some(last_deleted) = prune_rows.last() {
            let last_deleted_id: i64 = last_deleted.get("id");
            let last_deleted_execution_id: String = last_deleted.get("execution_id");
            let last_deleted_record_hash: String = last_deleted.get("record_hash");

            let next_retained = sqlx::query(
                "select execution_id from execution_audit where id > $1 order by id asc limit 1",
            )
            .bind(last_deleted_id)
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(next_retained) = next_retained {
                let next_execution_id: String = next_retained.get("execution_id");
                sqlx::query(
                    r#"
                    insert into audit_retention_checkpoints (
                        boundary_execution_id, deleted_through_execution_id, deleted_through_record_hash, deleted_row_count
                    ) values ($1, $2, $3, $4)
                    on conflict (boundary_execution_id) do update set
                        deleted_through_execution_id = excluded.deleted_through_execution_id,
                        deleted_through_record_hash = excluded.deleted_through_record_hash,
                        deleted_row_count = excluded.deleted_row_count,
                        created_at = now()
                    "#,
                )
                .bind(&next_execution_id)
                .bind(&last_deleted_execution_id)
                .bind(&last_deleted_record_hash)
                .bind(prune_rows.len() as i64)
                .execute(&mut *tx)
                .await?;

                boundary_execution_id = Some(next_execution_id);
            }

            deleted_audit_rows = sqlx::query("delete from execution_audit where id <= $1 and execution_started_at < $2")
                .bind(last_deleted_id)
                .bind(audit_cutoff)
                .execute(&mut *tx)
                .await?
                .rows_affected() as i64;
        }

        tx.commit().await?;

        Ok(CleanupResult {
            deleted_audit_rows,
            deleted_execution_artifacts,
            deleted_replay_runs,
            deleted_policy_snapshots,
            boundary_execution_id,
        })
    }
}
