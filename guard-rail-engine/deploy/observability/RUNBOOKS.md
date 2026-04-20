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

## Rotate Admin Token

**Symptoms:** Admin token needs to be rotated for security.

**Immediate Checks:**
1. Identify current admin token in config

**Mitigation:**
1. Generate new token (use a secure random generator)
2. Update config file or secret
3. For container: update secret and restart

**Recovery Verification:**
1. Test new token: `curl -H "Authorization: Bearer <new-token>" http://localhost:8081/metrics`
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
