# Guard Rail Runbooks

This document contains operational procedures for common scenarios.

## Deploy

**Symptoms:** New version needs to be rolled out.

**Immediate Checks:**
1. Verify the new container image is available
2. Check current deployment's health: `curl http://localhost:8080/ready`
3. Check metrics: `curl http://localhost:8080/metrics`

**Mitigation:**
1. Update the container image reference in your deployment
2. Rolling update: `kubectl rollout restart deployment/guard-rail` or equivalent
3. Verify new pods become ready

**Recovery Verification:**
1. Check readiness: `curl http://localhost:8080/ready`
2. Check logs for errors
3. Verify request throughput returns to normal

---

## Rollback

**Symptoms:** New version is causing issues (errors, latency, unexpected behavior).

**Immediate Checks:**
1. Check `/ready` endpoint: `curl http://localhost:8080/ready`
2. Check logs for errors: `kubectl logs -l app=guard-rail --tail=100`
3. Check `guardrail_readiness` metric

**Mitigation:**
1. Rollback to previous version: `kubectl rollout undo deployment/guard-rail` or equivalent
2. Verify pods return to ready state

**Recovery Verification:**
1. Confirm `/ready` returns 200
2. Confirm error rate returns to baseline

---

## Readiness Down

**Symptoms:** `GuardRailReadinessDown` fires and `/ready` returns non-200 responses.

**Immediate Checks:**
1. Check `/ready`: `curl -i http://localhost:8080/ready`
2. Check readiness metrics: `guardrail_readiness` and `guardrail_readiness_failures_total`
3. Check recent shutdown and reload events in logs

**Mitigation:**
1. If cause is `database_unavailable`, follow the **Database Outage** runbook
2. If cause is lifecycle draining during deploy, validate rollout health and finish/rollback deploy
3. Restart the instance if it is stuck in a non-ready state unexpectedly

**Recovery Verification:**
1. `/ready` returns 200 consistently
2. `guardrail_readiness` returns to `1`

---

## Rotate Admin Token

**Symptoms:** Admin token needs to be rotated for security.

**Immediate Checks:**
1. Identify current admin token in config

**Mitigation:**
1. Generate new token (use a secure random generator)
2. Update config file or secret
3. For container: update secret and restart

**Recovery Verification:**
1. Test new token: `curl -H "Authorization: Bearer <new-token>" http://localhost:8081/v1/admin/tenants`
2. Verify old token is rejected

---

## Rotate Tenant Keys

**Symptoms:** Tenant API keys need rotation.

**Immediate Checks:**
1. Identify affected tenants via tenant repository

**Mitigation:**
1. Generate new keys via tenant API or database
2. Distribute new keys to tenants securely

**Recovery Verification:**
1. Verify new keys work: request with new key returns 200

---

## Database Outage

**Symptoms:** `/ready` returns failure with `database_unavailable`, `guardrail_readiness_failures_total{cause="database_unavailable"}` increasing.

**Immediate Checks:**
1. Check database connectivity: `pg_isready -h <db-host> -U guardrail`
2. Check database logs
3. Check `guardrail_readiness_failures_total` metric

**Mitigation:**
1. If database is down, wait for recovery
2. If database is up but Guard Rail can't reach it, check network/firewall
3. Restart Guard Rail if needed

**Recovery Verification:**
1. `/ready` returns 200
2. Check `guardrail_readiness_failures_total` stops increasing

---

## Audit Persistence Failures

**Symptoms:** `GuardRailAuditPersistenceFailures` alert is firing and `guardrail_audit_persist_failures_total` is increasing.

**Immediate Checks:**
1. Check runtime logs for `failed to persist execution` and `failed to persist execution bundle`
2. Check database health and connectivity
3. Check whether failures are continuous or bursty

**Mitigation:**
1. If DB connectivity is degraded, follow **Database Outage**
2. If failures are tied to a recent rollout, rollback and re-check
3. If failures are sustained, temporarily reduce traffic while resolving DB/write-path errors

**Recovery Verification:**
1. `guardrail_audit_persist_failures_total` stops increasing
2. New execution rows are present in `execution_audit`

---

## Replay Persistence Failures

**Symptoms:** `GuardRailReplayPersistenceFailures` alert is firing and `guardrail_replay_persist_failures_total` is increasing.

**Immediate Checks:**
1. Check runtime logs for replay bundle persistence errors
2. Check DB health and storage capacity
3. Confirm whether replay capture is enabled in config

**Mitigation:**
1. If DB path is degraded, follow **Database Outage**
2. If failures are new after rollout, rollback and compare config changes
3. Reduce replay capture pressure if needed while restoring DB health

**Recovery Verification:**
1. `guardrail_replay_persist_failures_total` stops increasing
2. New replay artifacts are persisted successfully

---

## Upstream Outage

**Symptoms:** High `guardrail_upstream_failures_total`, requests returning 502/503.

**Immediate Checks:**
1. Check upstream service health
2. Check `guardrail_upstream_failures_total` metric
3. Check logs for upstream errors

**Mitigation:**
1. If upstream is down, wait for upstream recovery
2. If upstream is slow, consider adjusting timeout config

**Recovery Verification:**
1. Request rate returns to normal
2. Upstream failure rate drops to near-zero

---

## Auth Rejection Spike

**Symptoms:** `GuardRailAuthRejectionSpike` alert is firing and `guardrail_auth_rejections_total` grows quickly.

**Immediate Checks:**
1. Break down by reason label: `sum by (reason) (increase(guardrail_auth_rejections_total[5m]))`
2. Confirm whether spike is from a single tenant, route, or client rollout
3. Check recent tenant key rotation or route-binding changes

**Mitigation:**
1. If `missing_api_key` or `invalid_api_key` dominates, coordinate client credential fix
2. If `tenant_route_mismatch` dominates, validate route bindings and tenant config
3. If `rate_limited` dominates, review traffic spike and tenant quotas

**Recovery Verification:**
1. Auth rejection increase returns to baseline
2. Expected request success rates recover

---

## Bad Policy Reload

**Symptoms:** `guardrail_reload_events_total{outcome=~"rejected|failed"}` increasing, new policies not loading.

**Immediate Checks:**
1. Check reload events: `guardrail_reload_events_total`
2. Check logs for reload errors

**Mitigation:**
1. Fix policy files in `/etc/guard-rail-engine/policies/`
2. Trigger manual reload or wait for auto-reload

**Recovery Verification:**
1. Check `guardrail_reload_events_total{outcome="succeeded"}` increments

---

## Related Alerts

- **GuardRailReadinessDown**: `guardrail_readiness == 0`
- **GuardRailAuditPersistenceFailures**: `guardrail_audit_persist_failures_total` increasing
- **GuardRailReplayPersistenceFailures**: `guardrail_replay_persist_failures_total` increasing
- **GuardRailReloadFailures**: `guardrail_reload_events_total{outcome=~"rejected|failed"}` increasing
- **GuardRailAuthRejectionSpike**: `guardrail_auth_rejections_total` elevated
- **GuardRailUpstreamFailures**: `guardrail_upstream_failures_total` elevated
