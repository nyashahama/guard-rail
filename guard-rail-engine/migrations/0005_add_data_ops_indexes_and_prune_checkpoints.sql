create table if not exists audit_retention_checkpoints (
    boundary_execution_id text primary key references execution_audit(execution_id) on delete cascade,
    deleted_through_execution_id text not null,
    deleted_through_record_hash text not null,
    deleted_row_count bigint not null,
    created_at timestamptz not null default now()
);

create index if not exists idx_execution_audit_tenant_route_started_at
    on execution_audit (tenant_id, route_id, execution_started_at desc, id desc);
create index if not exists idx_execution_audit_tenant_verdict_started_at
    on execution_audit (tenant_id, verdict, execution_started_at desc, id desc);
create index if not exists idx_execution_artifacts_created_at
    on execution_artifacts (created_at);
create index if not exists idx_replay_runs_created_at
    on replay_runs (created_at);
create index if not exists idx_policy_snapshots_created_at
    on policy_snapshots (created_at);
