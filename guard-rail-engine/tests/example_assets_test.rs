use std::path::PathBuf;

use guard_rail_engine::policy::{
    PolicySet,
    engine::{Verdict, evaluate},
};
use serde_json::json;

fn manifest_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn sample_policies() -> PolicySet {
    PolicySet::load_dir(&manifest_path("examples/policies"))
        .expect("sample policy templates should load")
}

fn evaluate_sample_policy(
    policies: &PolicySet,
    policy_name: &str,
    payload: &serde_json::Value,
    raw_bytes: usize,
) -> Verdict {
    evaluate(payload, raw_bytes, &[policy_name.to_string()], policies)
}

fn assert_allows(verdict: Verdict) {
    assert!(
        matches!(verdict, Verdict::Allow),
        "expected allow, got {verdict:?}"
    );
}

fn assert_blocks(verdict: Verdict, expected_policy_name: &str) {
    match verdict {
        Verdict::Block { policy_name, .. } => {
            assert_eq!(policy_name, expected_policy_name);
        }
        Verdict::Allow => panic!("expected block from {expected_policy_name}, got allow"),
    }
}

#[test]
fn sample_policy_templates_load() {
    let policies = sample_policies();

    for name in [
        "callback-allowlist",
        "sa-id-pii-block",
        "payload-size-limit",
    ] {
        assert!(policies.get(name).is_some(), "missing sample policy {name}");
    }
}

#[test]
fn callback_allowlist_allows_safe_domain_and_blocks_unknown_domain() {
    let policies = sample_policies();

    let allowed_payload = json!({
        "callback": "https://api.safe.example/hook",
        "value": "ship"
    });
    assert_allows(evaluate_sample_policy(
        &policies,
        "callback-allowlist",
        &allowed_payload,
        allowed_payload.to_string().len(),
    ));

    let blocked_payload = json!({
        "callback": "https://evil.example/exfiltrate",
        "value": "ship"
    });
    assert_blocks(
        evaluate_sample_policy(
            &policies,
            "callback-allowlist",
            &blocked_payload,
            blocked_payload.to_string().len(),
        ),
        "callback-allowlist",
    );
}

#[test]
fn sa_id_pii_block_allows_reference_and_blocks_id_number() {
    let policies = sample_policies();

    let allowed_payload = json!({
        "fields": [
            { "name": "reference", "value": "INV-2048" }
        ]
    });
    assert_allows(evaluate_sample_policy(
        &policies,
        "sa-id-pii-block",
        &allowed_payload,
        allowed_payload.to_string().len(),
    ));

    let blocked_payload = json!({
        "fields": [
            { "name": "customer_id", "value": "8501015009087" }
        ]
    });
    assert_blocks(
        evaluate_sample_policy(
            &policies,
            "sa-id-pii-block",
            &blocked_payload,
            blocked_payload.to_string().len(),
        ),
        "sa-id-pii-block",
    );
}

#[test]
fn payload_size_limit_allows_small_payload_and_blocks_oversized_payload() {
    let policies = sample_policies();
    let payload = json!({ "value": "ship" });

    assert_allows(evaluate_sample_policy(
        &policies,
        "payload-size-limit",
        &payload,
        payload.to_string().len(),
    ));

    assert_blocks(
        evaluate_sample_policy(&policies, "payload-size-limit", &payload, 102_401),
        "payload-size-limit",
    );
}
