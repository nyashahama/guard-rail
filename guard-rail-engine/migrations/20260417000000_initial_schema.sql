-- Create execution_audit table for zero-trust audit logging
CREATE TABLE IF NOT EXISTS execution_audit (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    execution_id VARCHAR(255) NOT NULL,
    route_id VARCHAR(255) NOT NULL,
    method VARCHAR(16) NOT NULL,
    source_ip INET,
    verdict VARCHAR(32) NOT NULL,
    policy VARCHAR(255),
    rule_field VARCHAR(255),
    violation_value TEXT,
    latency_inspect_us INTEGER,
    latency_forward_ms INTEGER,
    latency_total_ms INTEGER,
    upstream VARCHAR(512),
    upstream_status INTEGER,
    forward_error TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_execution_audit_execution_id ON execution_audit(execution_id);
CREATE INDEX IF NOT EXISTS idx_execution_audit_route_id ON execution_audit(route_id);
CREATE INDEX IF NOT EXISTS idx_execution_audit_created_at ON execution_audit(created_at);