# Guard Rail Data Operations

## Cleanup

Dry-run the retention pass:

```bash
docker run --rm \
  -v "$(pwd)/deploy/container:/etc/guard-rail-engine:ro" \
  --env-file ./deploy/container/guard-rail-engine.env.example \
  guard-rail-engine \
  cleanup --config /etc/guard-rail-engine/config.yaml
```

Apply the retention pass:

```bash
docker run --rm \
  -v "$(pwd)/deploy/container:/etc/guard-rail-engine:ro" \
  --env-file ./deploy/container/guard-rail-engine.env.example \
  guard-rail-engine \
  cleanup --config /etc/guard-rail-engine/config.yaml --apply
```

Retention defaults:
- `audit_retention_days`: 180
- `artifact_retention_days`: 30
- `replay_run_retention_days`: 30
- `orphan_snapshot_retention_days`: 30
- `cleanup_batch_size`: 1000

Override any setting via environment variables (`GUARDRAIL_DATA_OPS__*`).

Cleanup processes data in this order: replay runs, execution artifacts, orphaned policy snapshots, then audit rows with checkpoint preservation.

## Backup

```bash
pg_dump --format=custom --no-owner --no-privileges \
  --dbname="$GUARDRAIL_DATABASE__URL" \
  --file=guardrail-$(date +%F).dump
```

## Restore

```bash
dropdb --if-exists guardrail_restore
createdb guardrail_restore
pg_restore --clean --if-exists --no-owner --dbname=guardrail_restore guardrail-2026-04-20.dump
```

Run migrations separately after restore if the target runtime expects a newer schema version than the dump contains.

## Rollback

If cleanup code is reverted, data already deleted is not restored by application rollback. Restoration requires database restore procedures.
