create table if not exists tenants (
    id uuid primary key,
    name text not null unique,
    status text not null check (status in ('active', 'disabled')),
    created_at timestamptz not null default now(),
    disabled_at timestamptz
);

create table if not exists api_keys (
    id uuid primary key,
    tenant_id uuid not null references tenants(id),
    key_prefix text not null,
    key_hash text not null unique,
    name text not null,
    created_at timestamptz not null default now(),
    last_used_at timestamptz,
    revoked_at timestamptz,
    revoked_reason text
);

create table if not exists tenant_routes (
    route_id text primary key,
    tenant_id uuid not null references tenants(id),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

alter table execution_audit
    add column if not exists tenant_id uuid references tenants(id),
    add column if not exists api_key_id uuid references api_keys(id),
    add column if not exists auth_outcome text;

create index if not exists idx_execution_audit_tenant_started_at
    on execution_audit (tenant_id, execution_started_at desc);
create index if not exists idx_api_keys_tenant_active
    on api_keys (tenant_id, revoked_at);