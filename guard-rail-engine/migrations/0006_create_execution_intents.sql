create table if not exists execution_intents (
    execution_id text primary key,
    route_id text not null,
    tenant_id uuid references tenants(id),
    api_key_id uuid references api_keys(id),
    method text not null,
    source_ip text not null,
    content_type text,
    user_agent text,
    request_size_bytes bigint not null check (request_size_bytes >= 0),
    request_body_sha256 text not null,
    route_config_hash text not null,
    policy_set_hash text not null,
    status text not null check (status in ('pending', 'finalized', 'finalization_failed')),
    finalization_error text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    finalized_at timestamptz,
    check (
        (status = 'pending' and finalization_error is null and finalized_at is null)
        or (status = 'finalized' and finalization_error is null and finalized_at is not null)
        or (status = 'finalization_failed' and finalization_error is not null and finalized_at is null)
    )
);

create index if not exists idx_execution_intents_status_created_at_desc
    on execution_intents (status, created_at desc);
create index if not exists idx_execution_intents_route_created_at_desc
    on execution_intents (route_id, created_at desc);
