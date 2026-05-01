# Guard Rail Future Hybrid Platform System Design

**Date:** 2026-04-28

**Status:** Approved design

## Goal

Define the future full-platform architecture for Guard Rail as a hybrid enterprise SaaS: a hosted multi-tenant control plane paired with customer-deployed enforcement gateways.

The design expands beyond the current pilot runtime while preserving its most important boundary: Guard Rail gateways enforce traffic locally, and the hosted platform does not sit in the customer's hot request path.

## Recommended Approach

Guard Rail should use a hybrid model:

- hosted control plane for organizations, users, policy authoring, route management, gateway fleet management, config rollout, metadata audit search, billing, and operational visibility
- customer-deployed data plane for low-latency policy enforcement, upstream forwarding, local audit persistence, and local replay artifacts
- metadata-only hosted audit by default, with full request and response payloads staying in the customer environment unless explicitly exported

This is the most credible architecture for a security/runtime product because customers can keep internal API traffic and sensitive payloads inside their own network boundary while still receiving a managed SaaS workflow for configuration, visibility, and operations.

## Scope

The baseline target is early enterprise SaaS with a documented path to larger scale.

Initial scale assumptions:

- 10-100 customer organizations
- 100-1,000 deployed gateways
- 1,000 active users
- 10,000 configured routes
- 100,000 policy versions
- 1k-10k aggregate gateway requests per second, mostly handled outside the hosted control plane
- 50-200 gateway heartbeats per second at peak
- 500-2,000 audit metadata events per second at peak
- 1-5 TB per year of hosted metadata and operational records

The first version is a single-primary-region SaaS control plane. It should not require global active-active operation, sharded core data, or centralized payload retention from day one.

## Functional Requirements

The platform must support:

- organization and workspace management
- user authentication, SSO, RBAC, and service accounts
- tenant and customer isolation
- route and upstream API configuration
- policy authoring, validation, testing, versioning, and approval
- signed config snapshot generation
- secure gateway registration, credential rotation, and revocation
- gateway config sync and heartbeat reporting
- local request enforcement at gateways
- local audit and replay storage at gateways
- hosted audit metadata ingest and search
- alerts for blocked traffic, gateway health, config drift, and policy rollout failures
- billing and usage metering based on organizations, gateways, routes, and metadata volume
- admin APIs and UI APIs for platform workflows

## Non-Functional Requirements

Availability:

- hosted control plane target: 99.9%
- gateway request enforcement continues during short control-plane outages
- gateway deployments should support multiple replicas behind a customer-owned internal load balancer

Latency:

- gateway policy evaluation should add low single-digit millisecond p95 overhead under normal load, excluding upstream latency
- hosted control-plane latency is not part of customer request-path latency

Consistency:

- strong consistency for control-plane route, policy, identity, and config snapshot writes
- eventual consistency for gateway rollout and hosted audit metadata ingest
- immutable policy versions, route versions, and config snapshots

Security:

- zero-trust gateway registration
- mTLS or signed request authentication for gateway APIs
- signed config snapshots
- encrypted data in transit and at rest
- envelope encryption for customer-sensitive configuration
- least-privilege access and tenant isolation

Data boundary:

- hosted control plane stores metadata by default
- full request and response bodies remain customer-local by default
- optional central exports require explicit customer configuration and retention policy

Operability:

- structured logs, metrics, traces, alerts, runbooks, migration controls, and disaster recovery procedures
- explicit retention and cleanup for hosted and gateway-local data

## Out Of Scope For The First Platform Version

- active-active global control plane
- vendor-hosted hot-path gateway as the default model
- customer self-hosting the full SaaS control plane
- full payload centralization by default
- real-time global analytics over all customer payloads
- custom per-customer database clusters from day one
- Kubernetes-only deployment requirement for gateways

## Data Model And Storage

### Storage Choices

Use Postgres as the hosted control-plane system of record. The domain is relational: organizations, users, roles, gateways, routes, policies, versions, approvals, rollout state, and billing accounts need transactions and referential integrity.

Use Redis for short-lived cache, distributed locks, idempotency keys, rate limits, and ephemeral sync state.

Use a durable queue or event bus for asynchronous ingest and background work. Suitable first choices include SQS/SNS, Pub/Sub, Kafka, or Redpanda depending on the hosting environment.

Use object storage for exported artifacts, generated reports, config snapshot archives, and optional customer-enabled replay exports. Do not store full payload artifacts centrally by default.

At the gateway, keep local Postgres or customer-approved durable storage for execution audit, replay artifacts, and tamper-evident ledger data.

### Core Entity Relationships

```text
organizations
  1 -> many workspaces
  1 -> many users through organization_memberships
  1 -> many gateways
  1 -> many billing_accounts

users
  1 -> many organization_memberships
  1 -> many audit_actor_events

roles
  1 -> many role_permissions
  many -> many users through organization_memberships

workspaces
  many -> 1 organizations
  1 -> many environments
  1 -> many route_definitions
  1 -> many policy_sets

environments
  many -> 1 workspaces
  values: dev, staging, production, custom

gateway_clusters
  many -> 1 organizations
  1 -> many gateways
  1 -> many environment_bindings

gateways
  many -> 1 gateway_clusters
  1 -> many gateway_heartbeats
  1 -> many config_deployments
  1 -> many audit_metadata_events

route_definitions
  many -> 1 workspaces
  1 -> many route_versions
  many -> many policy_sets through route_policy_bindings

policy_sets
  many -> 1 workspaces
  1 -> many policy_versions

config_snapshots
  many -> 1 environments
  1 -> many config_snapshot_items
  1 -> many config_deployments

config_deployments
  many -> 1 config_snapshots
  many -> 1 gateway_clusters or gateways

audit_metadata_events
  many -> 1 organizations
  many -> 1 workspaces
  optional many -> 1 gateways
  optional route_id, policy_version_id, config_snapshot_id

replay_exports
  many -> 1 organizations
  many -> 1 audit_metadata_events
  optional object_storage reference

usage_events
  many -> 1 organizations

billing_accounts
  many -> 1 organizations
  1 -> many subscriptions
  1 -> many invoices
```

### Hosted Control-Plane Tables

Representative schema:

```sql
organizations (
  id uuid primary key,
  slug text unique not null,
  name text not null,
  status text not null,
  created_at timestamptz not null,
  updated_at timestamptz not null
);

users (
  id uuid primary key,
  email citext unique not null,
  display_name text,
  status text not null,
  created_at timestamptz not null
);

organization_memberships (
  organization_id uuid references organizations(id),
  user_id uuid references users(id),
  role_id uuid references roles(id),
  status text not null,
  created_at timestamptz not null,
  primary key (organization_id, user_id)
);

workspaces (
  id uuid primary key,
  organization_id uuid references organizations(id),
  name text not null,
  slug text not null,
  created_at timestamptz not null,
  unique (organization_id, slug)
);

environments (
  id uuid primary key,
  workspace_id uuid references workspaces(id),
  name text not null,
  type text not null,
  created_at timestamptz not null,
  unique (workspace_id, name)
);

gateway_clusters (
  id uuid primary key,
  organization_id uuid references organizations(id),
  name text not null,
  deployment_mode text not null,
  created_at timestamptz not null
);

gateways (
  id uuid primary key,
  organization_id uuid references organizations(id),
  cluster_id uuid references gateway_clusters(id),
  name text not null,
  status text not null,
  public_key text not null,
  version text,
  last_seen_at timestamptz,
  created_at timestamptz not null
);

route_definitions (
  id uuid primary key,
  workspace_id uuid references workspaces(id),
  route_key text not null,
  status text not null,
  created_at timestamptz not null,
  unique (workspace_id, route_key)
);

route_versions (
  id uuid primary key,
  route_id uuid references route_definitions(id),
  version_number integer not null,
  upstream_url_ciphertext bytea not null,
  method_allowlist text[] not null,
  config_json jsonb not null,
  created_by uuid references users(id),
  created_at timestamptz not null,
  unique (route_id, version_number)
);

policy_sets (
  id uuid primary key,
  workspace_id uuid references workspaces(id),
  policy_key text not null,
  status text not null,
  created_at timestamptz not null,
  unique (workspace_id, policy_key)
);

policy_versions (
  id uuid primary key,
  policy_set_id uuid references policy_sets(id),
  version_number integer not null,
  policy_json jsonb not null,
  policy_hash text not null,
  created_by uuid references users(id),
  created_at timestamptz not null,
  unique (policy_set_id, version_number)
);

config_snapshots (
  id uuid primary key,
  environment_id uuid references environments(id),
  snapshot_version bigint not null,
  snapshot_hash text not null,
  signature text not null,
  status text not null,
  created_by uuid references users(id),
  created_at timestamptz not null,
  unique (environment_id, snapshot_version)
);

config_deployments (
  id uuid primary key,
  snapshot_id uuid references config_snapshots(id),
  gateway_id uuid references gateways(id),
  cluster_id uuid references gateway_clusters(id),
  status text not null,
  deployed_at timestamptz,
  acknowledged_at timestamptz,
  error_message text,
  created_at timestamptz not null
);

audit_metadata_events (
  id uuid primary key,
  organization_id uuid references organizations(id),
  workspace_id uuid references workspaces(id),
  gateway_id uuid references gateways(id),
  execution_id text not null,
  execution_started_at timestamptz not null,
  route_key text,
  verdict text not null,
  reason_code text,
  policy_hash text,
  config_snapshot_hash text,
  request_body_sha256 text,
  response_status integer,
  latency_total_ms integer,
  metadata_json jsonb not null,
  received_at timestamptz not null,
  unique (organization_id, gateway_id, execution_id)
);
```

### Normalization

Use third normal form for core identity, authorization, route, policy, gateway, and billing records:

- organizations, users, roles, gateways, policies, and route versions are separate tables
- versioned configuration is immutable
- many-to-many relationships use join tables
- operational event tables are append-oriented and indexed around access patterns

Use `jsonb` only where it is appropriate:

- policy expressions
- route-specific match options
- event metadata
- gateway capability details

Do not use `jsonb` for core relational fields such as `organization_id`, `workspace_id`, `gateway_id`, `status`, `created_at`, or version numbers.

### Indexes

Core indexes:

```sql
create index on organization_memberships (user_id, organization_id);
create index on workspaces (organization_id);
create index on gateways (organization_id, status, last_seen_at desc);
create index on gateway_heartbeats (gateway_id, observed_at desc);
create index on route_definitions (workspace_id, route_key);
create index on route_versions (route_id, version_number desc);
create index on policy_versions (policy_set_id, version_number desc);
create index on config_snapshots (environment_id, snapshot_version desc);
create index on config_deployments (gateway_id, status, created_at desc);
create index on audit_metadata_events (organization_id, execution_started_at desc);
create index on audit_metadata_events (organization_id, route_key, execution_started_at desc);
create index on audit_metadata_events (organization_id, verdict, execution_started_at desc);
```

For early scale, keep audit metadata in Postgres. Add monthly partitioning once event volume justifies it. Move high-volume analytics to a columnar store later rather than making it a day-one dependency.

### Gateway Local Data Model

Customer-side gateways keep high-sensitivity records:

- `execution_audit`
- `execution_artifacts`
- `replay_runs`
- `policy_snapshots`
- `audit_retention_checkpoints`
- local config snapshot cache
- gateway registration and key material references

Hosted metadata references gateway-local records by `execution_id`, `gateway_id`, hashes, and timestamps. It does not store raw payloads by default.

## API Design

Use REST plus asynchronous event ingest for the first platform version.

- REST is the default for UI, admin, gateway registration, config fetch, audit search, billing, and operational workflows.
- Gateway event ingest can start as REST batch endpoints and later move to Kafka-compatible private ingest, gRPC streaming, or regional collectors.
- GraphQL is optional for the UI later.
- gRPC is useful for gateway-control-plane streaming later, but REST long-poll plus signed snapshots is simpler and more debuggable early.

### API Groups

```text
/v1/auth/*
/v1/orgs/*
/v1/workspaces/*
/v1/environments/*
/v1/routes/*
/v1/policies/*
/v1/config-snapshots/*
/v1/gateway-registration/*
/v1/gateways/*
/v1/gateway-sync/*
/v1/audit-metadata/*
/v1/replay-exports/*
/v1/usage/*
/v1/billing/*
/v1/admin/*
```

### Authentication

Human users:

- OIDC/SAML SSO for enterprise organizations
- local login only if needed for early non-enterprise accounts
- short-lived access token plus refresh token
- organization and workspace scoped RBAC
- optional SCIM provisioning later

Machine clients:

- gateways register with a one-time enrollment token
- gateway receives a unique identity, key pair, and certificate or signed credential
- all gateway APIs require mTLS or signed request authentication
- credentials are rotatable and revocable
- every gateway request includes `gateway_id`, `organization_id`, timestamp, nonce or idempotency key, and signature

### Versioning And Pagination

Prefix all public APIs with `/v1`.

Gateway APIs include:

- `gateway_version`
- `capabilities`
- `min_supported_snapshot_version`
- `protocol_version`

Use cursor pagination for list APIs:

```http
GET /v1/orgs/{org_id}/audit-metadata?limit=100&cursor=...
```

Response:

```json
{
  "items": [],
  "next_cursor": "opaque_cursor",
  "has_more": true
}
```

### Route And Policy APIs

Representative endpoints:

```http
POST /v1/orgs/{org_id}/workspaces
GET /v1/orgs/{org_id}/workspaces

POST /v1/workspaces/{workspace_id}/routes
GET /v1/workspaces/{workspace_id}/routes
GET /v1/routes/{route_id}
POST /v1/routes/{route_id}/versions
POST /v1/route-versions/{version_id}/validate

POST /v1/workspaces/{workspace_id}/policies
GET /v1/workspaces/{workspace_id}/policies
POST /v1/policies/{policy_set_id}/versions
POST /v1/policy-versions/{version_id}/validate
POST /v1/policy-versions/{version_id}/test

POST /v1/environments/{environment_id}/config-snapshots
GET /v1/environments/{environment_id}/config-snapshots
POST /v1/config-snapshots/{snapshot_id}/promote
POST /v1/config-snapshots/{snapshot_id}/rollback
```

Policy test request:

```json
{
  "route_version_id": "uuid",
  "policy_version_ids": ["uuid"],
  "sample_request": {
    "method": "POST",
    "headers": {},
    "body_json": {}
  }
}
```

Policy test response:

```json
{
  "verdict": "blocked",
  "matched_policy": "pii-blocking",
  "matched_rule": "deny-ssn",
  "reason_code": "policy_rule_matched",
  "evaluation_ms": 2
}
```

### Config Snapshot APIs

A config snapshot is immutable and signed.

```http
POST /v1/environments/{environment_id}/config-snapshots
```

Request:

```json
{
  "route_version_ids": ["uuid"],
  "policy_version_ids": ["uuid"],
  "description": "production rollout 2026-04-28"
}
```

Response:

```json
{
  "snapshot_id": "uuid",
  "snapshot_version": 42,
  "snapshot_hash": "sha256:...",
  "signature": "base64...",
  "status": "created"
}
```

Deploy to cluster:

```http
POST /v1/config-snapshots/{snapshot_id}/deployments
```

```json
{
  "target_type": "gateway_cluster",
  "target_id": "uuid",
  "strategy": "rolling",
  "max_unavailable": 1
}
```

### Gateway Registration APIs

Enrollment starts from the control plane and finishes from the gateway.

```http
POST /v1/orgs/{org_id}/gateway-enrollment-tokens
```

```json
{
  "cluster_id": "uuid",
  "expires_in_minutes": 30,
  "allowed_gateway_version": ">=1.0.0"
}
```

Gateway registration:

```http
POST /v1/gateway-registration/register
```

```json
{
  "enrollment_token": "opaque",
  "gateway_name": "prod-vpc-a-gw-1",
  "public_key": "base64...",
  "gateway_version": "1.0.3",
  "capabilities": ["local_replay", "signed_snapshots", "metadata_ingest"]
}
```

Response:

```json
{
  "gateway_id": "uuid",
  "organization_id": "uuid",
  "cluster_id": "uuid",
  "credential_bundle": "encrypted-or-mtls-material",
  "control_plane_base_url": "https://api.guardrail.example"
}
```

### Gateway Sync APIs

Gateway heartbeat:

```http
POST /v1/gateway-sync/heartbeat
```

```json
{
  "gateway_id": "uuid",
  "gateway_version": "1.0.3",
  "current_snapshot_hash": "sha256:...",
  "current_snapshot_version": 41,
  "status": "healthy",
  "metrics_summary": {
    "requests_1m": 1200,
    "blocked_1m": 18,
    "p95_added_latency_ms": 3
  }
}
```

Config check:

```http
GET /v1/gateway-sync/config?gateway_id=uuid&current_snapshot_version=41
```

Response when an update exists:

```json
{
  "update_available": true,
  "snapshot_version": 42,
  "snapshot_hash": "sha256:...",
  "download_url": "https://api.guardrail.example/v1/gateway-sync/config-snapshots/uuid",
  "signature": "base64..."
}
```

Gateway acknowledgement:

```http
POST /v1/gateway-sync/config-ack
```

```json
{
  "gateway_id": "uuid",
  "snapshot_id": "uuid",
  "snapshot_version": 42,
  "status": "applied",
  "applied_at": "2026-04-28T10:00:00Z"
}
```

### Audit Metadata APIs

Gateway batch ingest:

```http
POST /v1/audit-metadata/batches
```

```json
{
  "gateway_id": "uuid",
  "batch_id": "uuid",
  "events": [
    {
      "execution_id": "GR-EXE-...",
      "execution_started_at": "2026-04-28T10:01:00Z",
      "route_key": "partner-webhook",
      "verdict": "blocked",
      "reason_code": "policy_rule_matched",
      "policy_hash": "sha256:...",
      "config_snapshot_hash": "sha256:...",
      "request_body_sha256": "sha256:...",
      "response_status": 403,
      "latency_total_ms": 4,
      "metadata": {}
    }
  ]
}
```

Response:

```json
{
  "batch_id": "uuid",
  "accepted": 100,
  "duplicates": 0,
  "rejected": []
}
```

Hosted audit search:

```http
GET /v1/orgs/{org_id}/audit-metadata?workspace_id=...&route_key=...&verdict=blocked&from=...&to=...
```

Audit metadata detail:

```http
GET /v1/audit-metadata/{event_id}
```

This returns metadata and local replay pointers, not raw payloads.

### Error Contract

All APIs return a consistent error shape:

```json
{
  "error": {
    "code": "config_snapshot_invalid",
    "message": "The config snapshot contains a policy version from another workspace.",
    "request_id": "req_...",
    "details": {}
  }
}
```

Apply rate limits by user ID, organization ID, gateway ID, endpoint class, write-heavy admin APIs, and ingest APIs.

## High-Level Architecture

Use a modular monolith control plane first, with clear internal domain boundaries and an async worker layer. Split into separate services only when scale, deployment cadence, or ownership requires it.

Recommended first deployable units:

```text
control-plane-api
control-plane-worker
gateway-ingest-worker or ingest-api
web-app
postgres
redis
event-bus / queue
object-storage
observability stack
customer-deployed gateways
```

### Architecture Diagram

```text
Browser / CLI / Terraform Provider
        |
        v
CDN + WAF
        |
        v
API Load Balancer
        |
        v
control-plane-api
  - auth and RBAC
  - org/workspace management
  - route and policy authoring
  - config snapshot creation
  - gateway registration
  - audit metadata query
  - billing and usage APIs
        |
        +--> Postgres
        +--> Redis
        +--> Object Storage
        +--> Event Bus / Queue
                 |
                 v
          control-plane-worker
            - config compilation
            - snapshot signing
            - rollout orchestration
            - ingest processing
            - alert evaluation
            - usage aggregation
            - billing sync
            - notifications

Customer VPC
  |
  v
Guard Rail Gateway
  - local policy enforcement
  - local config verification
  - upstream forwarding
  - local audit ledger
  - local replay artifacts
  - heartbeat and metadata sync
```

### Control Plane Responsibilities

The hosted control plane owns:

- organizations, users, roles, and permissions
- route and policy source of truth
- immutable route and policy versions
- signed config snapshot compilation
- deployment intent and rollout state
- gateway identity and lifecycle
- gateway health and config drift detection
- audit metadata ingest and query
- usage metering and billing
- alerting and notification workflows

The hosted control plane does not sit in the customer request path.

### Gateway Responsibilities

The customer-deployed gateway owns:

- enforcing route and policy decisions locally
- validating signed config snapshots before use
- continuing to enforce the last valid snapshot during control-plane outages
- forwarding allowed requests to customer upstreams
- persisting local audit and replay records
- emitting metadata batches and health summaries
- supporting local retention and data operations

Gateway failure behavior:

- if no valid config has ever been loaded, gateway fails closed
- if a previously valid config exists and the control plane is unreachable, gateway continues with the last valid snapshot
- if a newer snapshot fails signature or schema validation, gateway rejects that snapshot and keeps the current one
- if metadata ingest fails, gateway buffers within configured limits and retries without blocking request enforcement

### Caching Layers

Use caching selectively:

- CDN caches static web app assets and public documentation.
- Redis caches user session metadata, organization permission summaries, config compilation locks, idempotency keys, and short-lived gateway sync state.
- Gateways keep local in-memory config and policy evaluation caches for hot-path performance.
- Postgres remains the source of truth for control-plane state.

Do not cache authorization decisions beyond short TTLs unless the cache is invalidated on membership and role changes.

### Event Bus And Queues

Use asynchronous processing for:

- gateway heartbeat processing
- audit metadata batch ingestion
- usage aggregation
- config snapshot compilation
- rollout state transitions
- alert evaluation
- notification delivery
- billing provider synchronization

Topics or queues:

```text
gateway.heartbeat.received
gateway.audit_metadata.received
config_snapshot.requested
config_snapshot.compiled
config_deployment.requested
config_deployment.acknowledged
usage.event.recorded
alert.condition.detected
billing.sync.requested
```

Early implementation can use a managed queue with durable workers. Kafka or Redpanda becomes useful when ingest volume and replayable event streams justify it.

### Config Rollout Flow

```text
User creates policy/route version
  -> user validates policy
  -> user creates config snapshot for environment
  -> API writes snapshot request
  -> worker compiles canonical config
  -> worker signs snapshot
  -> user deploys snapshot to gateway cluster
  -> gateways poll or long-poll for updates
  -> gateway downloads snapshot
  -> gateway verifies signature and schema
  -> gateway stages config
  -> gateway atomically switches active snapshot
  -> gateway sends ack
  -> control plane marks rollout progress
```

Rollout state is visible per cluster and gateway:

```text
pending -> available -> downloaded -> applied
pending -> available -> failed_validation
pending -> available -> failed_apply
```

### Request Enforcement Flow

```text
Client inside customer environment
  -> Guard Rail Gateway
  -> route lookup
  -> method/auth checks
  -> policy evaluation
  -> block/reject or forward upstream
  -> local audit write
  -> local replay artifact write if configured
  -> response to client
  -> async metadata batch to hosted control plane
```

### Audit Metadata Flow

```text
Gateway local execution record
  -> local tamper-evident audit ledger
  -> metadata projection
  -> batch queue on gateway
  -> hosted audit metadata ingest endpoint
  -> ingest validation and dedupe
  -> append audit_metadata_events
  -> usage event emitted
  -> alert rules evaluated async
```

Central audit search is a metadata index. Deep replay requires customer-local access or explicit export.

### Monolith vs Microservices

Start with:

- one backend API codebase
- one worker codebase
- shared domain modules
- separate queues and worker jobs by domain
- strong module boundaries around identity, policy, config, gateways, audit metadata, billing, and notifications

Extract later when there is real pressure:

- `gateway-ingest` if audit metadata volume needs independent scaling
- `config-compiler` if snapshot compilation becomes CPU-heavy or security-sensitive
- `billing` if financial workflows require stricter isolation
- `notifications` if delivery volume grows
- `analytics` if reporting moves to a warehouse or columnar store

## Low-Level Design

### Control Plane Internal Modules

Recommended backend modules:

```text
identity
  - users
  - org memberships
  - SSO connections
  - service accounts
  - RBAC checks

workspace
  - workspaces
  - environments
  - environment bindings

policy
  - policy sets
  - policy versions
  - validation
  - test evaluation

routes
  - route definitions
  - route versions
  - upstream config encryption
  - route-policy bindings

config
  - snapshot compiler
  - canonical serialization
  - signing
  - rollout orchestration
  - deployment state

gateway
  - enrollment tokens
  - gateway identity
  - key/cert rotation
  - heartbeats
  - drift detection

audit_metadata
  - ingest validation
  - dedupe
  - search
  - alert projection

usage
  - usage events
  - aggregation
  - plan limits

billing
  - subscriptions
  - invoice sync
  - entitlement checks

notifications
  - email/slack/webhook delivery
  - alert routing
```

Each module owns its tables through repository interfaces. Cross-module operations happen through application services and domain events, not direct table mutation from unrelated modules.

### Core Domain Objects

```text
Organization
Workspace
Environment
User
Role
Permission
GatewayCluster
Gateway
EnrollmentToken
RouteDefinition
RouteVersion
PolicySet
PolicyVersion
ConfigSnapshot
ConfigDeployment
AuditMetadataEvent
ReplayExport
UsageEvent
BillingAccount
Subscription
AlertRule
NotificationTarget
```

### State Machines

Gateway lifecycle:

```text
created -> enrollment_pending -> active -> degraded -> revoked
active -> rotating_credentials -> active
active -> disabled
degraded -> active
degraded -> revoked
```

Config snapshot lifecycle:

```text
draft_requested -> compiling -> compiled -> signed -> deployable
compiling -> failed
signed -> archived
```

Deployment lifecycle:

```text
created -> pending_gateway_pickup -> downloaded -> applied
pending_gateway_pickup -> timed_out
downloaded -> failed_validation
downloaded -> failed_apply
applied -> superseded
```

Policy version lifecycle:

```text
draft -> validated -> approved -> included_in_snapshot
draft -> rejected
approved -> deprecated
```

### Config Compiler

The config compiler produces deterministic output.

Inputs:

- environment
- route versions
- policy versions
- environment-specific bindings
- gateway capability constraints

Steps:

1. Load selected immutable route and policy versions in one transaction.
2. Verify every version belongs to the target workspace and environment.
3. Validate route-policy references.
4. Resolve environment-specific upstreams and secret references.
5. Produce canonical JSON or protobuf.
6. Compute snapshot hash over canonical bytes.
7. Sign snapshot hash and metadata with the control-plane signing key.
8. Store the immutable snapshot record and archive canonical bytes.
9. Emit `config_snapshot.compiled`.

Use a strategy pattern for target formats if gateway config formats evolve:

```text
ConfigCompiler
  -> CanonicalJsonCompiler
  -> FutureProtobufCompiler
```

### Gateway Sync

Gateway sync is pull-based first.

Reasons:

- works through customer firewalls
- avoids inbound access into customer networks
- simplifies operations
- supports offline and degraded states naturally

Gateway sync loop:

```text
every N seconds with jitter:
  send heartbeat
  ask for latest deployment for gateway or cluster
  if newer snapshot exists:
    download snapshot
    verify signature
    validate schema and capability compatibility
    stage snapshot on disk
    run local consistency checks
    atomically swap active config pointer
    acknowledge success or failure
```

Use exponential backoff on failed control-plane calls. Keep request enforcement separate from sync loop failure.

### Gateway Request Internals

Gateway hot path:

```text
HttpListener
  -> RequestContextBuilder
  -> ActiveConfigReader
  -> RouteMatcher
  -> AuthEvaluator
  -> PolicyEvaluator
  -> UpstreamForwarder
  -> ExecutionRecorder
  -> MetadataProjector
```

Design rules:

- active config is immutable once loaded
- config swap is atomic
- policy evaluation does not call the hosted control plane
- audit writes are bounded and observable
- metadata upload is async and retryable
- upstream forwarding timeout is configurable per route

### Key Sequences

Policy rollout:

```text
User -> API: create policy version
API -> PolicyValidator: validate
API -> Postgres: save immutable version
User -> API: create config snapshot
API -> Queue: config_snapshot.requested
Worker -> ConfigCompiler: compile
Worker -> KMS: sign
Worker -> Postgres: save signed snapshot
User -> API: deploy snapshot to cluster
Gateway -> API: heartbeat/config check
API -> Gateway: snapshot available
Gateway -> API/Object Storage: download
Gateway -> Gateway: verify/stage/apply
Gateway -> API: ack applied
API -> Postgres: update deployment state
```

Audit metadata ingest:

```text
Gateway -> Local DB: write execution audit
Gateway -> Local DB/Object Store: write replay artifact if enabled
Gateway -> MetadataQueue: enqueue metadata projection
Gateway -> API: send batch with batch_id
API -> Auth: verify gateway credential
API -> IngestService: validate schema and org/gateway ownership
API -> Postgres: insert deduped events
API -> Queue: usage.event.recorded
API -> Gateway: accepted/duplicate/rejected counts
Worker -> AlertEvaluator: evaluate async
```

Control-plane outage:

```text
Gateway sync loop -> Control Plane: request fails
Gateway -> Logs/Metrics: record sync failure
Gateway -> Local State: keep last valid snapshot
Gateway -> Request Path: continue enforcing
Gateway -> Local Buffer: retain metadata until buffer limit
Gateway -> Request Path: continue unless local storage is exhausted and policy requires fail-closed
```

### Design Patterns

Use these patterns where they solve concrete problems:

- repository pattern for database access
- unit of work for multi-table transactional writes like snapshot creation
- strategy pattern for config compiler formats and notification delivery providers
- outbox pattern for reliable event emission from Postgres-backed writes
- idempotency keys for gateway batch ingest and payment or billing callbacks
- circuit breaker for billing provider, notification provider, and optional external integrations
- immutable value objects for route versions, policy versions, and config snapshots

Avoid singleton-based domain services. Use dependency injection through application wiring.

### Error Handling

Control plane:

- validation errors return `400`
- auth failures return `401`
- permission failures return `403`
- hidden cross-tenant resources return `404`
- conflicts return `409`
- rate limits return `429`
- internal errors return `500` with request ID

Gateway:

- invalid config snapshot: reject snapshot, keep current config
- missing route: `404`
- invalid auth: `401` or `403` depending on route mode
- policy block: `403`
- rate limit: `429`
- upstream timeout or error: configured `502` or `504`
- local audit write failure: configurable, with fail-closed support for security-sensitive deployments

### Testing Strategy

- Unit tests for policy validation, config compiler, RBAC, and gateway route matching.
- Integration tests for Postgres repositories and migrations.
- Contract tests for gateway sync and metadata ingest APIs.
- End-to-end tests for policy rollout and gateway apply acknowledgement.
- Failure tests for control-plane outage, bad signatures, duplicate metadata batches, and stale snapshots.
- Load tests for gateway hot path, heartbeat ingest, and audit metadata batch ingest.

## Scalability And Reliability

The key scaling decision is that gateway request traffic does not traverse the hosted control plane.

Scale independently:

```text
Gateway hot path:
  scales inside customer environments

Hosted API:
  scales by stateless horizontal replicas

Workers:
  scale by queue depth and job type

Postgres:
  scales vertically first, then read replicas, partitioning, and selected service extraction

Redis:
  scales for cache/rate-limit/session workloads

Event bus:
  scales ingest and async processing independently from API request latency
```

### Horizontal And Vertical Scaling

Control plane API:

- stateless containers behind load balancer
- horizontal autoscaling by CPU, memory, request rate, and latency
- no local durable state

Workers:

- horizontal autoscaling by queue depth and oldest message age
- separate worker pools for config compilation, ingest, billing, notifications, and alerting when needed

Postgres:

- start with managed primary plus read replica
- vertical scale first for early enterprise stage
- add partitioning for audit metadata and usage events
- add read replicas for audit search and reporting
- later split high-write event tables or move analytics to columnar storage

Gateways:

- horizontally deployed by customers per region or VPC
- active-active gateway replicas behind customer internal load balancer
- each gateway keeps local config and local audit storage
- cluster-level rollout state tracks which replicas have applied snapshots

### Database Partitioning And Sharding

Do not shard the core control-plane database on day one.

Use:

- tenant-aware indexes on `organization_id`
- monthly partitioning for `audit_metadata_events`
- monthly partitioning for `usage_events`
- archival and lifecycle policies for old event partitions
- read replicas for reporting and search load

Future sharding path:

- shard by `organization_id` for very large customers
- move audit metadata to a dedicated event database or columnar store
- keep identity, billing, and config source of truth in a strongly consistent relational store
- introduce regional control-plane cells for data residency

### Replication

Hosted control plane:

- managed Postgres primary in one region
- synchronous or semi-synchronous storage replication depending on cloud provider
- at least one read replica
- PITR enabled
- regular logical backups tested through restore drills
- Redis configured for managed HA
- queue/event bus durable replication enabled

Gateway local:

- customer decides local database HA posture
- recommended production gateway cluster uses external Postgres or managed equivalent
- local audit durability should match customer compliance needs
- metadata buffering should survive gateway restarts

### Failover And Disaster Recovery

Control plane RPO/RTO baseline:

- RPO: 5-15 minutes for hosted control-plane data
- RTO: 1-4 hours for full regional restore in early enterprise stage
- stricter targets can be added for higher tiers later

Failure modes:

- API replica failure: load balancer routes around it
- worker failure: message visibility timeout and retry
- Redis failure: degraded caching and rate limits; no permanent data loss
- queue failure: backpressure non-critical async paths
- Postgres primary failure: managed failover
- full control-plane outage: gateways continue with last valid config
- object storage outage: config snapshot downloads and exports degrade; active gateways continue

### Gateway Offline Behavior

Rules:

- enforce last valid signed config
- reject unsigned or expired snapshots
- continue local audit writes
- buffer metadata upload up to configured storage limit
- expose local health indicating sync degradation
- never allow traffic because the control plane is unreachable
- fail closed if no valid config exists

Snapshot expiration is configurable. Highly regulated customers may require gateways to fail closed after a maximum offline period.

### CAP Model

Gateway enforcement:

- choose availability and partition tolerance for the request path using the last valid signed config
- accept temporary staleness during a control-plane partition
- preserve safety through signatures, config versioning, and expiration policy

Control-plane writes:

- choose consistency and partition tolerance
- route, policy, and config writes require the primary database
- avoid accepting conflicting policy writes in multiple regions in the first version

Audit metadata:

- choose availability and eventual consistency
- gateway local audit ledger is authoritative for full execution history
- hosted metadata can arrive late and is deduped by `(organization_id, gateway_id, execution_id)`

### Reliability Controls

Use:

- idempotency keys for retried writes
- outbox pattern for reliable domain event publication
- dead-letter queues for poison messages
- retry with exponential backoff and jitter
- circuit breakers for external providers
- health checks for API, workers, database, Redis, and queues
- readiness checks that fail when required dependencies are unavailable
- graceful shutdown for API and workers
- migration gates before deployment
- canary or rolling deployments for API and workers
- staged config rollout for gateways

### Backpressure

Control plane should shed load predictably:

- rate limit noisy users, organizations, and gateways
- accept smaller metadata batches when large batches overload ingest
- return `429` with `Retry-After`
- pause non-critical workers before critical config and auth flows
- prioritize gateway config sync over heavy audit search during incidents

Gateway should also apply backpressure:

- bounded metadata buffer
- bounded local replay artifact storage
- retention cleanup
- configurable behavior when local audit storage is unavailable
- request body size limits
- upstream timeout limits

### Consistency Rules

- Route and policy versions are immutable.
- Config snapshots are immutable.
- Deployment state is mutable and operational.
- Gateway active config changes only through atomic snapshot swap.
- Audit metadata ingest is append-only and deduped.
- Billing usage aggregation can be eventually consistent.
- RBAC changes should take effect quickly, with short cache TTL and invalidation.

### Upgrade Path To Larger Scale

When the platform grows beyond early enterprise:

1. Extract gateway ingest into a separate service.
2. Partition audit metadata and usage event storage aggressively.
3. Add regional ingest endpoints near customers.
4. Move analytics to ClickHouse, BigQuery, Snowflake, or similar.
5. Add regional control-plane cells for data residency.
6. Split high-scale customers onto dedicated database shards.
7. Introduce gRPC streaming for gateway fleets that need lower-latency rollout.
8. Add multi-region read replicas and eventually active-active only for carefully scoped read/query surfaces.

## Observability

Control plane telemetry:

- structured JSON logs with `request_id`, `organization_id`, `user_id`, `gateway_id`, and `trace_id`
- Prometheus/OpenTelemetry metrics
- distributed traces across API, workers, queue jobs, database calls, and external providers
- audit actor log for user and admin actions
- immutable security event log for sensitive actions

Core metrics:

```text
api_request_count
api_request_latency_ms
api_error_count
auth_failure_count
rbac_denial_count
gateway_heartbeat_count
gateway_last_seen_age_seconds
gateway_config_drift_count
config_snapshot_compile_duration_ms
config_deployment_apply_latency_seconds
audit_metadata_batch_count
audit_metadata_ingest_lag_seconds
audit_metadata_rejected_count
queue_depth
queue_oldest_message_age_seconds
postgres_connection_pool_usage
worker_job_failures
billing_sync_failures
notification_failures
```

Gateway telemetry:

```text
gateway_request_count
gateway_policy_verdict_count
gateway_added_latency_ms
gateway_upstream_latency_ms
gateway_audit_write_failures
gateway_metadata_buffer_depth
gateway_metadata_flush_failures
gateway_active_snapshot_version
gateway_config_apply_failures
gateway_control_plane_sync_failures
```

Alerts:

- gateway offline or stale heartbeat
- config drift after rollout deadline
- snapshot compile failure
- metadata ingest lag above threshold
- elevated policy block rate
- audit write failures on gateway
- queue backlog
- database saturation
- error budget burn
- billing sync failures
- suspicious auth failures or enrollment attempts

## Security

### Identity And Access

- OIDC/SAML SSO for enterprise organizations.
- MFA for local accounts and privileged users.
- SCIM later for enterprise provisioning.
- Organization, workspace, and environment RBAC.
- Separate roles for owner, admin, policy editor, deploy approver, auditor, billing admin, and read-only viewer.
- Service accounts with scoped tokens.
- Break-glass admin access with strict logging and time bounds.

### Gateway Security

- One-time enrollment token with short TTL.
- Gateway-generated key pair.
- mTLS or signed requests for gateway APIs.
- Credential rotation.
- Gateway revocation.
- Signed config snapshots.
- Gateway verifies snapshot signature and hash before applying.
- No hosted control-plane dependency during request enforcement.
- Least-privilege egress from gateway to control-plane endpoints.

### Data Protection

- TLS everywhere.
- Encryption at rest for Postgres, Redis, queue storage, and object storage.
- KMS-managed keys.
- Envelope encryption for upstream URLs, secrets, and customer-sensitive config.
- Raw request and response payloads stay customer-local by default.
- Optional exports require explicit customer configuration and retention policy.
- Secrets never appear in logs, metadata events, or config snapshots in plaintext.

### Tenant Isolation

- Every hosted table includes tenant ownership through `organization_id` or a parent relation.
- API authorization checks verify organization and workspace membership.
- Cross-tenant resource access returns `404` where resource existence should be hidden.
- Background jobs carry tenant context.
- Object storage paths include organization-scoped prefixes and IAM restrictions.
- Audit metadata dedupes within organization and gateway scope.

### Policy Supply-Chain Integrity

- Immutable policy versions.
- Immutable route versions.
- Signed config snapshots.
- Snapshot hash included in gateway audit metadata.
- Optional two-person approval for production deployments.
- Deployment history preserved.
- Rollback creates a new deployment event pointing to a previous immutable snapshot.

### Rate Limiting And Abuse Controls

- Per-user and per-organization admin API limits.
- Per-gateway heartbeat and ingest limits.
- WAF for public API.
- Bot and brute-force protections on auth endpoints.
- Payload size limits.
- Metadata batch size limits.
- Idempotency to prevent retry amplification.

## Infrastructure

Baseline cloud architecture:

```text
DNS
  -> CDN/WAF
  -> API Load Balancer
  -> control-plane-api containers
  -> Postgres / Redis / Queue / Object Storage

Worker Autoscaling Group
  -> queue consumers
  -> Postgres / Redis / Object Storage / external providers

Web App
  -> CDN-hosted static assets
  -> API

Observability
  -> logs
  -> metrics
  -> traces
  -> alert manager
```

Recommended managed dependencies:

- managed Postgres with PITR and backups
- managed Redis
- managed queue or event bus
- object storage with lifecycle policies
- KMS and secrets manager
- container registry
- managed log, metric, and trace backend
- external billing provider
- transactional email provider
- optional Slack and webhook notification integrations

Environments:

```text
dev
staging
production
```

Deployment rules:

- migrations run as explicit jobs before API and worker rollout
- app startup does not auto-run destructive migrations
- blue/green or rolling deploy for API
- workers drain current jobs before shutdown
- config compiler and signing code changes receive extra review
- infrastructure is managed with Terraform or OpenTofu
- secrets are injected through a secrets manager, not committed env files
- CI runs unit, integration, contract, migration, and smoke tests

### Infrastructure Security Controls

- Private networking for databases and queues.
- No public database access.
- Least-privilege IAM per service.
- Separate KMS keys for signing, database encryption, and object exports.
- Access logs for admin and production infrastructure actions.
- Vulnerability scanning for containers.
- Dependency scanning.
- SAST and secret scanning.
- Regular restore drills.
- Incident runbooks.

## Data Retention

Hosted defaults:

- audit metadata: 180-365 days by plan
- gateway health summaries: 90 days detailed, longer aggregated
- usage events: retained according to billing and legal requirements
- security events: 1-7 years depending on compliance tier
- object exports: customer-configurable lifecycle
- deleted organizations: soft-delete first, hard-delete after retention window unless legal hold exists

Gateway defaults:

- local audit retention configurable by customer
- local replay artifact retention shorter than audit metadata
- tamper-evident checkpoints preserve audit-chain verification across cleanup
- cleanup is explicit and observable

## Acceptance Criteria

The architecture is successful when:

- customer request traffic can be enforced without traversing the hosted control plane
- gateways continue with the last valid config during control-plane outages
- hosted control plane can manage organizations, users, routes, policies, snapshots, gateways, metadata, and billing
- full payloads remain customer-local by default
- policy and config rollout is signed, auditable, and reversible
- audit metadata is searchable centrally without centralizing sensitive bodies
- the system can scale through early enterprise usage without sharding core data
- there is a credible path to regional ingest, sharded metadata, and multi-region control plane later

## Implementation Notes

This document is a platform system design, not an implementation plan. The current repository should continue to describe the existing runtime as a verified pilot deployment until platform implementation work is planned and built.

The first implementation plan should decompose this architecture into independently deliverable programs:

1. hosted identity, organization, workspace, and RBAC foundation
2. route and policy versioning APIs
3. signed config snapshot compiler
4. gateway enrollment and sync protocol
5. hosted audit metadata ingest and search
6. usage, billing, and alerting
7. infrastructure, observability, and security hardening
