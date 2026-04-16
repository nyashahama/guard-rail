create table if not exists execution_audit (
    id bigserial primary key,
    execution_id text not null unique,
    execution_started_at timestamptz not null,
    audit_persisted_at timestamptz not null default now(),
    route_id text not null,
    upstream_url text,
    method text not null,
    source_ip text not null,
    content_type text,
    user_agent text,
    had_authorization_header boolean not null,
    request_size_bytes bigint not null,
    request_body_sha256 text not null,
    verdict text not null,
    rejection_reason text,
    matched_policy_name text,
    matched_rule_field text,
    matched_rule_condition text,
    matched_rule_severity text,
    violation_value_hash text,
    violation_value_preview text,
    upstream_status integer,
    forward_error text,
    latency_inspect_us bigint not null,
    latency_forward_ms bigint,
    latency_total_ms bigint not null,
    route_config_hash text not null,
    policy_set_hash text not null,
    previous_hash text,
    record_hash text not null
);

create index if not exists idx_execution_audit_route_id_desc on execution_audit (route_id, id desc);
create index if not exists idx_execution_audit_verdict_desc on execution_audit (verdict, id desc);
create index if not exists idx_execution_audit_execution_started_desc on execution_audit (execution_started_at desc, id desc);