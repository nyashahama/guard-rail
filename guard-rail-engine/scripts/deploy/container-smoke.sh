#!/usr/bin/env bash
set -euo pipefail

: "${TEST_DATABASE_URL:?TEST_DATABASE_URL must be set}"

IMAGE_TAG="${IMAGE_TAG:-guard-rail-engine:phase5-smoke}"
CONFIG_DIR="${CONFIG_DIR:-$(pwd)/deploy/container}"
HOST_PORT="${HOST_PORT:-18080}"
CONTAINER_NAME="guardrail-phase5-serve"

cleanup() {
  docker rm -f "${CONTAINER_NAME}" >/dev/null 2>&1 || true
}

trap cleanup EXIT

docker build -t "${IMAGE_TAG}" .

test "$(docker image inspect "${IMAGE_TAG}" --format '{{.Config.User}}')" = "guardrail:guardrail"
test "$(docker image inspect "${IMAGE_TAG}" --format '{{json .Config.Healthcheck}}')" != "null"

docker run --rm "${IMAGE_TAG}" sh -lc 'test ! -e /srv/guard-rail-engine/config/config.yaml'

docker run --rm \
  -v "${CONFIG_DIR}:/etc/guard-rail-engine:ro" \
  -e GUARDRAIL_DATABASE__URL="${TEST_DATABASE_URL}" \
  -e GUARDRAIL_ADMIN__TOKEN="phase5-admin-token" \
  -e GUARDRAIL_ENVIRONMENT=production \
  "${IMAGE_TAG}" \
  migrate --config /etc/guard-rail-engine/config.yaml

docker run -d --name "${CONTAINER_NAME}" \
  -p "${HOST_PORT}:8080" \
  -v "${CONFIG_DIR}:/etc/guard-rail-engine:ro" \
  -e GUARDRAIL_DATABASE__URL="${TEST_DATABASE_URL}" \
  -e GUARDRAIL_ADMIN__TOKEN="phase5-admin-token" \
  -e GUARDRAIL_ENVIRONMENT=production \
  "${IMAGE_TAG}"

for _ in $(seq 1 30); do
  if curl --fail --silent --show-error "http://127.0.0.1:${HOST_PORT}/ready" >/dev/null; then
    break
  fi

  if ! docker ps --format '{{.Names}}' | grep -qx "${CONTAINER_NAME}"; then
    docker logs "${CONTAINER_NAME}" || true
    exit 1
  fi

  sleep 1
done

curl --fail --silent --show-error "http://127.0.0.1:${HOST_PORT}/health" >/dev/null
curl --fail --silent --show-error "http://127.0.0.1:${HOST_PORT}/ready" >/dev/null
