alter table execution_audit add column if not exists tenant_id uuid;
alter table execution_audit add column if not exists api_key_id uuid;
alter table execution_audit add column if not exists auth_outcome text;

create index if not exists idx_execution_audit_tenant_id on execution_audit (tenant_id);
create index if not exists idx_execution_audit_auth_outcome on execution_audit (auth_outcome);