use super::conditions::{ConditionParams, evaluate_condition};
use super::{Policy, PolicySet};
use jsonpath_rust::JsonPath;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum Verdict {
    Allow,
    Block {
        policy_name: String,
        rule_field: String,
        message: String,
        violation_value: Option<String>,
    },
}

pub fn evaluate(
    payload: &serde_json::Value,
    raw_bytes: usize,
    policy_names: &[String],
    policy_set: &PolicySet,
) -> Verdict {
    for name in policy_names {
        let policy = match policy_set.get(name) {
            Some(p) => p,
            None => {
                return Verdict::Block {
                    policy_name: name.clone(),
                    rule_field: String::new(),
                    message: format!("Policy not found: {}", name),
                    violation_value: None,
                };
            }
        };

        if let Some(block) = evaluate_policy(payload, raw_bytes, policy) {
            return block;
        }
    }

    Verdict::Allow
}

fn evaluate_policy(
    payload: &serde_json::Value,
    raw_bytes: usize,
    policy: &Policy,
) -> Option<Verdict> {
    for rule in &policy.rules {
        let extracted = extract_values(payload, &rule.field);

        let params = ConditionParams {
            values: rule.values.clone(),
            value: rule.value.clone(),
            pattern: rule.pattern.clone(),
            max_bytes: rule.max_bytes,
        };

        match evaluate_condition(&rule.condition, &extracted, &params, raw_bytes) {
            Ok(true) => {
                let violation_value = extracted.first().map(|v| {
                    if let Some(s) = v.as_str() {
                        s.to_string()
                    } else {
                        v.to_string()
                    }
                });

                return Some(Verdict::Block {
                    policy_name: policy.name.clone(),
                    rule_field: rule.field.clone(),
                    message: format!(
                        "{} condition triggered on field {}",
                        rule.condition, rule.field
                    ),
                    violation_value,
                });
            }
            Ok(false) => continue,
            Err(e) => {
                return Some(Verdict::Block {
                    policy_name: policy.name.clone(),
                    rule_field: rule.field.clone(),
                    message: format!("Condition evaluation error: {}", e),
                    violation_value: None,
                });
            }
        }
    }

    None
}

fn extract_values(payload: &serde_json::Value, path: &str) -> Vec<serde_json::Value> {
    if path == "$" {
        return vec![payload.clone()];
    }

    let jsonpath = match JsonPath::from_str(path) {
        Ok(jp) => jp,
        Err(_) => return vec![],
    };

    let result = jsonpath.find(payload);
    match result {
        serde_json::Value::Array(arr) => arr,
        serde_json::Value::Null => vec![],
        other => vec![other],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{Policy, PolicySet, Rule};
    use serde_json::json;
    use std::collections::HashMap;

    fn make_policy_set(policies: Vec<Policy>) -> PolicySet {
        let mut by_name = HashMap::new();
        for p in policies {
            by_name.insert(p.name.clone(), p);
        }
        PolicySet { by_name }
    }

    fn simple_domain_policy() -> Policy {
        Policy {
            name: "block-callbacks".to_string(),
            description: String::new(),
            rules: vec![Rule {
                field: "$.callback".to_string(),
                condition: "domain_not_in".to_string(),
                values: vec!["*.safe.com".to_string()],
                value: None,
                pattern: None,
                max_bytes: None,
                action: "block".to_string(),
                severity: "critical".to_string(),
            }],
        }
    }

    fn size_policy() -> Policy {
        Policy {
            name: "size-limit".to_string(),
            description: String::new(),
            rules: vec![Rule {
                field: "$".to_string(),
                condition: "size_exceeds".to_string(),
                values: vec![],
                value: None,
                pattern: None,
                max_bytes: Some(100),
                action: "block".to_string(),
                severity: "warning".to_string(),
            }],
        }
    }

    #[test]
    fn test_allow_when_all_policies_pass() {
        let set = make_policy_set(vec![simple_domain_policy()]);
        let payload = json!({"callback": "https://api.safe.com/hook", "amount": 100});
        let verdict = evaluate(&payload, 50, &["block-callbacks".to_string()], &set);
        assert!(matches!(verdict, Verdict::Allow));
    }

    #[test]
    fn test_block_when_policy_triggers() {
        let set = make_policy_set(vec![simple_domain_policy()]);
        let payload = json!({"callback": "https://evil.sh/exfil", "amount": 100});
        let verdict = evaluate(&payload, 50, &["block-callbacks".to_string()], &set);
        match verdict {
            Verdict::Block {
                policy_name,
                rule_field,
                ..
            } => {
                assert_eq!(policy_name, "block-callbacks");
                assert_eq!(rule_field, "$.callback");
            }
            Verdict::Allow => panic!("Expected block"),
        }
    }

    #[test]
    fn test_short_circuit_on_first_block() {
        let set = make_policy_set(vec![simple_domain_policy(), size_policy()]);
        let payload = json!({"callback": "https://evil.sh/exfil"});
        let verdict = evaluate(
            &payload,
            50,
            &["block-callbacks".to_string(), "size-limit".to_string()],
            &set,
        );
        match verdict {
            Verdict::Block { policy_name, .. } => {
                assert_eq!(policy_name, "block-callbacks");
            }
            Verdict::Allow => panic!("Expected block"),
        }
    }

    #[test]
    fn test_multiple_policies_all_pass() {
        let set = make_policy_set(vec![simple_domain_policy(), size_policy()]);
        let payload = json!({"callback": "https://api.safe.com/hook"});
        let verdict = evaluate(
            &payload,
            50,
            &["block-callbacks".to_string(), "size-limit".to_string()],
            &set,
        );
        assert!(matches!(verdict, Verdict::Allow));
    }

    #[test]
    fn test_size_exceeds_blocks() {
        let set = make_policy_set(vec![size_policy()]);
        let payload = json!({"data": "x"});
        let verdict = evaluate(&payload, 200, &["size-limit".to_string()], &set);
        assert!(matches!(verdict, Verdict::Block { .. }));
    }

    #[test]
    fn test_missing_field_passes_domain_check() {
        let set = make_policy_set(vec![simple_domain_policy()]);
        let payload = json!({"amount": 100});
        let verdict = evaluate(&payload, 50, &["block-callbacks".to_string()], &set);
        assert!(matches!(verdict, Verdict::Allow));
    }

    #[test]
    fn test_empty_policy_list_allows() {
        let set = make_policy_set(vec![]);
        let payload = json!({"anything": true});
        let verdict = evaluate(&payload, 50, &[], &set);
        assert!(matches!(verdict, Verdict::Allow));
    }
}
