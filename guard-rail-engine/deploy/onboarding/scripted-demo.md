# Scripted Demo

This is an end-to-end demo script for a pilot tenant-bound route with replay-backed audit.

Copy this into `~/guard-rail-onboard-demo.sh` and run it from the repo root.

## Demo script

    #!/usr/bin/env bash
    set -euo pipefail

    BASE_DIR="/tmp/guard-rail-onboarding"
    BASE_URL="http://localhost:8080"
    ADMIN_URL="http://localhost:8081"
    ADMIN_TOKEN="pilot-admin-token"

    # --- Start a fresh demo stack ---
    docker network create guard-rail-onboard >/dev/null 2>&1 || true

    docker run -d --name guardrail-postgres --network guard-rail-onboard \
      -e POSTGRES_DB=guardrail \
      -e POSTGRES_USER=guardrail \
      -e POSTGRES_PASSWORD=guardrail \
      postgres:16

    docker run --rm --network guard-rail-onboard \
      -v "$BASE_DIR:/etc/guard-rail-engine:ro" \
      guard-rail-engine:pilot \
      migrate --config /etc/guard-rail-engine/config.yaml

    docker run -d --name guardrail-engine --network guard-rail-onboard \
      -p 8080:8080 -p 8081:8081 \
      -v "$BASE_DIR:/etc/guard-rail-engine:ro" \
      guard-rail-engine:pilot

    until curl -s "$BASE_URL/ready" >/dev/null; do sleep 1; done

    # --- Tenant setup ---
    TENANT_ID=$(curl -s -X POST "$ADMIN_URL/v1/admin/tenants" \
      -H "Authorization: Bearer $ADMIN_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"name":"acme-demo"}' | jq -r '.id')

    API_KEY=$(curl -s -X POST "$ADMIN_URL/v1/admin/tenants/$TENANT_ID/keys" \
      -H "Authorization: Bearer $ADMIN_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"name":"demo-key"}' | jq -r '.raw_key')

    curl -s -X POST "$ADMIN_URL/v1/admin/tenants/$TENANT_ID/routes" \
      -H "Authorization: Bearer $ADMIN_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"route_id":"pilot-webhook"}'

    # --- Execute allowed call ---
    curl -s -X POST "$BASE_URL/v1/execute/pilot-webhook" \
      -H "Authorization: Bearer $API_KEY" \
      -H "Content-Type: application/json" \
      -d '{"callback":"https://api.safe.com/hook","notes":"demo"}'

    # --- Execute blocked call ---
    BLOCK_RESPONSE=$(curl -s -i -X POST "$BASE_URL/v1/execute/pilot-webhook" \
      -H "Authorization: Bearer $API_KEY" \
      -H "Content-Type: application/json" \
      -d '{"callback":"https://attacker.example.com/callback"}')

    echo "$BLOCK_RESPONSE"

    EXEC_ID=$(printf '%s' "$BLOCK_RESPONSE" | grep -oE 'GR-EXE-[0-9a-f-]+' | head -n 1)

    # --- Pull audit trail for this execution ---
    curl -s -H "Authorization: Bearer $API_KEY" \
      "$BASE_URL/v1/audit/executions?limit=5&order=desc" | jq '.items | map(.execution_id)'

    # --- Replay against snapshot policy source ---
    if [ -n "${EXEC_ID}" ]; then
      curl -s -X POST "$BASE_URL/v1/replay/executions/$EXEC_ID" \
        -H "Authorization: Bearer $API_KEY" \
        -H "Content-Type: application/json" \
        -d '{"policy_source":"snapshot"}' | jq
    fi

    # --- Optional cleanup ---
    # docker stop guardrail-engine guardrail-postgres
    # docker rm -f guardrail-engine guardrail-postgres
    # docker network rm guard-rail-onboard

## Expected outputs to check

1. **Setup response**: each create call returns JSON with tenant ID / raw key.
2. **Allowed execution**: `/v1/execute/pilot-webhook` returns `200` with upstream body.
3. **Blocked execution**: returned status `403` and block payload with:

   ```json
   {
     "status": "blocked",
     "execution_id": "GR-EXE-...",
     "policy": "callback-allowlist",
     "rule_field": "$.callback"
   }
   ```

4. **Audit tail**: blocked execution appears with verdict `BLOCKED`.
5. **Replay call**: returns verdict pair and whether the decision changed.

## Why this script is useful

This demo validates:
- route lookup and auth binding
- policy gate behavior under pilot traffic
- audit and replay read path
- containerized command usage for migration/replay and operator sanity checks
