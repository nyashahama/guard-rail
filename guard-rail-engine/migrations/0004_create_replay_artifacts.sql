-- Replay storage schema
-- Stage 4: Add tables for policy snapshots, execution artifacts, and replay runs

create table if not exists policy_snapshots (
    snapshot_hash text primary key,
    route_id text not null,
    route_definition jsonb not null,
    policies_definition jsonb not null,
    route_config_hash text not null,
    policy_set_hash text not null,
    created_at timestamptz not null default now()
);

create table if not exists execution_artifacts (
    execution_id text primary key references execution_audit(execution_id) on delete cascade,
    snapshot_hash text not null references policy_snapshots(snapshot_hash),
    request_body_json jsonb not null,
    request_headers jsonb not null,
    response_status integer,
    response_headers jsonb,
    response_body text,
    response_body_sha256 text,
    response_body_truncated boolean not null default false,
    created_at timestamptz not null default now()
);

create table if not exists replay_runs (
    id uuid primary key,
    execution_id text not null references execution_audit(execution_id) on delete cascade,
    policy_source text not null check (policy_source in ('snapshot', 'current')),
    evaluated_snapshot_hash text not null,
    original_verdict text not null,
    replay_verdict text not null,
    original_policy_name text,
    replay_policy_name text,
    original_rule_field text,
    replay_rule_field text,
    verdict_changed boolean not null,
    created_at timestamptz not null default now()
);

create index if not exists idx_execution_artifacts_snapshot_hash
    on execution_artifacts (snapshot_hash);
create index if not exists idx_replay_runs_execution_created_at
    on replay_runs (execution_id, created_at desc);