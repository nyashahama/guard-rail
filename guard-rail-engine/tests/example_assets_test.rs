use std::path::PathBuf;

use guard_rail_engine::policy::PolicySet;

fn manifest_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn sample_policy_templates_load() {
    let policies = PolicySet::load_dir(&manifest_path("examples/policies"))
        .expect("sample policy templates should load");

    for name in [
        "callback-allowlist",
        "sa-id-pii-block",
        "payload-size-limit",
    ] {
        assert!(policies.get(name).is_some(), "missing sample policy {name}");
    }
}
