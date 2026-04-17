use crate::policy::engine::{Verdict, evaluate};
use crate::proxy::AppState;
use crate::replay::snapshot::PolicySnapshotRecord;
use crate::storage::postgres::ExecutionArtifactRow;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReplayRequest {
    #[serde(default)]
    pub policy_source: ReplayPolicySource,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayPolicySource {
    #[default]
    Snapshot,
    Current,
}

pub struct ReplayRunParams {
    pub id: String,
    pub execution_id: String,
    pub policy_source: String,
    pub evaluated_snapshot_hash: String,
    pub original_verdict: String,
    pub replay_verdict: String,
    pub original_policy_name: Option<String>,
    pub replay_policy_name: Option<String>,
    pub original_rule_field: Option<String>,
    pub replay_rule_field: Option<String>,
    pub verdict_changed: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReplayResult {
    pub execution_id: String,
    pub policy_source: ReplayPolicySource,
    pub evaluated_snapshot_hash: String,
    pub original_verdict: String,
    pub replay_verdict: String,
    pub original_policy_name: Option<String>,
    pub replay_policy_name: Option<String>,
    pub original_rule_field: Option<String>,
    pub replay_rule_field: Option<String>,
    pub verdict_changed: bool,
}

#[derive(Debug)]
pub enum ReplayError {
    Unavailable,
    #[allow(dead_code)]
    NotFound,
    ArtifactsMissing,
    ExecutionNotFound,
    #[allow(dead_code)]
    PolicyNotFound(String),
    Storage(#[allow(dead_code)] sqlx::Error),
}

impl From<sqlx::Error> for ReplayError {
    fn from(e: sqlx::Error) -> Self {
        ReplayError::Storage(e)
    }
}

pub async fn replay_execution(
    state: &AppState,
    execution_id: &str,
    policy_source: ReplayPolicySource,
) -> Result<ReplayResult, ReplayError> {
    let store = state.audit_store.as_ref().ok_or(ReplayError::Unavailable)?;

    let original = store
        .get_execution_by_id(execution_id)
        .await?
        .ok_or(ReplayError::ExecutionNotFound)?;

    let artifacts = store
        .get_execution_artifacts(execution_id)
        .await?
        .ok_or(ReplayError::ArtifactsMissing)?;

    let original_verdict_str = original.verdict.clone();
    let original_policy_name = original.matched_policy_name.clone();
    let original_rule_field = original.matched_rule_field.clone();

    let replay_verdict = match policy_source {
        ReplayPolicySource::Snapshot => evaluate_snapshot(&artifacts)?,
        ReplayPolicySource::Current => {
            evaluate_current(state, &original.route_id, &artifacts).await?
        }
    };

    let replay_verdict_str = match &replay_verdict {
        Verdict::Allow => "ALLOWED".to_string(),
        Verdict::Block { .. } => "BLOCKED".to_string(),
    };

    let replay_policy_name = match &replay_verdict {
        Verdict::Block { policy_name, .. } => Some(policy_name.clone()),
        Verdict::Allow => None,
    };

    let replay_rule_field = match &replay_verdict {
        Verdict::Block { rule_field, .. } => Some(rule_field.clone()),
        Verdict::Allow => None,
    };

    let verdict_changed = original_verdict_str != replay_verdict_str;

    let evaluated_snapshot_hash = match policy_source {
        ReplayPolicySource::Snapshot => artifacts.snapshot_hash.clone(),
        ReplayPolicySource::Current => {
            let routes = state.routes.read().await;
            let route = routes.lookup(&original.route_id).cloned().ok_or_else(|| {
                ReplayError::PolicyNotFound(format!("route {} not found", original.route_id))
            })?;
            drop(routes);
            let policies = state.policies.read().await;
            PolicySnapshotRecord::from_route_and_set(&route, &policies)
                .map(|s| s.snapshot_hash)
                .unwrap_or_else(|_| artifacts.snapshot_hash.clone())
        }
    };

    let result = ReplayResult {
        execution_id: execution_id.to_string(),
        policy_source,
        evaluated_snapshot_hash,
        original_verdict: original_verdict_str,
        replay_verdict: replay_verdict_str,
        original_policy_name,
        replay_policy_name,
        original_rule_field,
        replay_rule_field,
        verdict_changed,
    };

    persist_replay_run(store, &result).await?;

    Ok(result)
}

fn evaluate_snapshot(artifacts: &ExecutionArtifactRow) -> Result<Verdict, ReplayError> {
    let policies_value = &artifacts.policies_definition;
    let policies: Vec<crate::policy::Policy> = serde_json::from_value(policies_value.clone())
        .map_err(|e| ReplayError::PolicyNotFound(format!("invalid snapshot policies: {e}")))?;

    let route_value = &artifacts.route_definition;
    let route_policies: Vec<String> = route_value
        .get("policies")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let policy_set = crate::policy::PolicySet::from_policies(policies);
    let policy_names: Vec<String> = route_policies;

    Ok(evaluate(
        &artifacts.request_body_json,
        0,
        &policy_names,
        &policy_set,
    ))
}

async fn evaluate_current(
    state: &AppState,
    route_id: &str,
    artifacts: &ExecutionArtifactRow,
) -> Result<Verdict, ReplayError> {
    let routes = state.routes.read().await;
    let route = routes
        .lookup(route_id)
        .cloned()
        .ok_or_else(|| ReplayError::PolicyNotFound(format!("route {} not found", route_id)))?;
    drop(routes);

    let policies = state.policies.read().await;
    Ok(evaluate(
        &artifacts.request_body_json,
        0,
        &route.policies,
        &policies,
    ))
}

async fn persist_replay_run(
    store: &crate::storage::postgres::PostgresAuditStore,
    result: &ReplayResult,
) -> Result<(), ReplayError> {
    let policy_source_str = match result.policy_source {
        ReplayPolicySource::Snapshot => "snapshot",
        ReplayPolicySource::Current => "current",
    };

    let params = ReplayRunParams {
        id: uuid::Uuid::new_v4().to_string(),
        execution_id: result.execution_id.clone(),
        policy_source: policy_source_str.to_string(),
        evaluated_snapshot_hash: result.evaluated_snapshot_hash.clone(),
        original_verdict: result.original_verdict.clone(),
        replay_verdict: result.replay_verdict.clone(),
        original_policy_name: result.original_policy_name.clone(),
        replay_policy_name: result.replay_policy_name.clone(),
        original_rule_field: result.original_rule_field.clone(),
        replay_rule_field: result.replay_rule_field.clone(),
        verdict_changed: result.verdict_changed,
    };

    store
        .insert_replay_run(&params)
        .await
        .map_err(ReplayError::Storage)
}

pub fn map_replay_error(e: ReplayError) -> axum::http::StatusCode {
    match e {
        ReplayError::Unavailable => axum::http::StatusCode::SERVICE_UNAVAILABLE,
        ReplayError::NotFound | ReplayError::ExecutionNotFound => axum::http::StatusCode::NOT_FOUND,
        ReplayError::ArtifactsMissing => axum::http::StatusCode::NOT_FOUND,
        ReplayError::PolicyNotFound(_) => axum::http::StatusCode::NOT_FOUND,
        ReplayError::Storage(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}
