use regex::Regex;
use url::Url;

/// Result of evaluating a condition. `true` means the condition triggered (should block).
pub fn evaluate_condition(
    condition: &str,
    extracted_values: &[serde_json::Value],
    params: &ConditionParams,
    raw_payload_bytes: usize,
) -> Result<bool, String> {
    match condition {
        "domain_not_in" => domain_not_in(extracted_values, &params.values),
        "domain_in" => domain_in(extracted_values, &params.values),
        "regex_match" => regex_match(extracted_values, params.pattern.as_deref()),
        "regex_not_match" => regex_not_match(extracted_values, params.pattern.as_deref()),
        "equals" => equals(extracted_values, &params.value),
        "not_equals" => not_equals(extracted_values, &params.value),
        "contains" => contains_check(extracted_values, &params.value),
        "not_contains" => not_contains_check(extracted_values, &params.value),
        "size_exceeds" => size_exceeds(raw_payload_bytes, params.max_bytes),
        "field_exists" => Ok(!extracted_values.is_empty()),
        "field_not_exists" => Ok(extracted_values.is_empty()),
        _ => Err(format!("Unknown condition: {}", condition)),
    }
}

#[derive(Debug, Default)]
pub struct ConditionParams {
    pub values: Vec<String>,
    pub value: Option<serde_json::Value>,
    pub pattern: Option<String>,
    pub max_bytes: Option<usize>,
}

fn domain_matches_glob(domain: &str, pattern: &str) -> bool {
    if pattern.starts_with("*.") {
        let suffix = &pattern[1..]; // ".example.com"
        domain.ends_with(suffix) || domain == &pattern[2..]
    } else {
        domain == pattern
    }
}

fn extract_domain(val: &serde_json::Value) -> Option<String> {
    let s = val.as_str()?;
    let url = Url::parse(s).ok()?;
    url.host_str().map(|h| h.to_lowercase())
}

fn domain_not_in(values: &[serde_json::Value], allowlist: &[String]) -> Result<bool, String> {
    for val in values {
        if let Some(domain) = extract_domain(val) {
            let allowed = allowlist
                .iter()
                .any(|pat| domain_matches_glob(&domain, pat));
            if !allowed {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn domain_in(values: &[serde_json::Value], blocklist: &[String]) -> Result<bool, String> {
    for val in values {
        if let Some(domain) = extract_domain(val) {
            let blocked = blocklist
                .iter()
                .any(|pat| domain_matches_glob(&domain, pat));
            if blocked {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn regex_match(values: &[serde_json::Value], pattern: Option<&str>) -> Result<bool, String> {
    let pat = pattern.ok_or("regex_match requires 'pattern' parameter")?;
    let re = Regex::new(pat).map_err(|e| format!("Invalid regex: {}", e))?;
    for val in values {
        if let Some(s) = val.as_str() {
            if re.is_match(s) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn regex_not_match(values: &[serde_json::Value], pattern: Option<&str>) -> Result<bool, String> {
    let pat = pattern.ok_or("regex_not_match requires 'pattern' parameter")?;
    let re = Regex::new(pat).map_err(|e| format!("Invalid regex: {}", e))?;
    for val in values {
        if let Some(s) = val.as_str() {
            if !re.is_match(s) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn equals(
    values: &[serde_json::Value],
    target: &Option<serde_json::Value>,
) -> Result<bool, String> {
    let target = target.as_ref().ok_or("equals requires 'value' parameter")?;
    for val in values {
        if val == target {
            return Ok(true);
        }
    }
    Ok(false)
}

fn not_equals(
    values: &[serde_json::Value],
    target: &Option<serde_json::Value>,
) -> Result<bool, String> {
    let target = target
        .as_ref()
        .ok_or("not_equals requires 'value' parameter")?;
    for val in values {
        if val != target {
            return Ok(true);
        }
    }
    Ok(false)
}

fn contains_check(
    values: &[serde_json::Value],
    target: &Option<serde_json::Value>,
) -> Result<bool, String> {
    let target_str = target
        .as_ref()
        .and_then(|v| v.as_str())
        .ok_or("contains requires 'value' parameter (string)")?;
    for val in values {
        if let Some(s) = val.as_str() {
            if s.contains(target_str) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn not_contains_check(
    values: &[serde_json::Value],
    target: &Option<serde_json::Value>,
) -> Result<bool, String> {
    let target_str = target
        .as_ref()
        .and_then(|v| v.as_str())
        .ok_or("not_contains requires 'value' parameter (string)")?;
    for val in values {
        if let Some(s) = val.as_str() {
            if !s.contains(target_str) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn size_exceeds(raw_bytes: usize, max: Option<usize>) -> Result<bool, String> {
    let max = max.ok_or("size_exceeds requires 'max_bytes' parameter")?;
    Ok(raw_bytes > max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn params() -> ConditionParams {
        ConditionParams::default()
    }

    #[test]
    fn test_domain_not_in_blocks_unknown_domain() {
        let vals = vec![json!("https://evil.sh/exfil")];
        let mut p = params();
        p.values = vec!["*.safe.com".to_string()];
        assert!(evaluate_condition("domain_not_in", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_domain_not_in_allows_known_domain() {
        let vals = vec![json!("https://api.safe.com/hook")];
        let mut p = params();
        p.values = vec!["*.safe.com".to_string()];
        assert!(!evaluate_condition("domain_not_in", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_domain_not_in_exact_match() {
        let vals = vec![json!("https://safe.com/hook")];
        let mut p = params();
        p.values = vec!["*.safe.com".to_string()];
        assert!(!evaluate_condition("domain_not_in", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_domain_in_blocks_blacklisted() {
        let vals = vec![json!("https://evil.sh/exfil")];
        let mut p = params();
        p.values = vec!["evil.sh".to_string()];
        assert!(evaluate_condition("domain_in", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_domain_in_allows_non_blacklisted() {
        let vals = vec![json!("https://safe.com/hook")];
        let mut p = params();
        p.values = vec!["evil.sh".to_string()];
        assert!(!evaluate_condition("domain_in", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_regex_match_triggers() {
        let vals = vec![json!("8501015009087")];
        let mut p = params();
        p.pattern = Some(r"\d{13}".to_string());
        assert!(evaluate_condition("regex_match", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_regex_match_no_trigger() {
        let vals = vec![json!("hello world")];
        let mut p = params();
        p.pattern = Some(r"\d{13}".to_string());
        assert!(!evaluate_condition("regex_match", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_regex_not_match_triggers() {
        let vals = vec![json!("hello")];
        let mut p = params();
        p.pattern = Some(r"^\d+$".to_string());
        assert!(evaluate_condition("regex_not_match", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_equals_triggers() {
        let vals = vec![json!("blocked_value")];
        let mut p = params();
        p.value = Some(json!("blocked_value"));
        assert!(evaluate_condition("equals", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_equals_no_trigger() {
        let vals = vec![json!("ok_value")];
        let mut p = params();
        p.value = Some(json!("blocked_value"));
        assert!(!evaluate_condition("equals", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_equals_numeric() {
        let vals = vec![json!(42)];
        let mut p = params();
        p.value = Some(json!(42));
        assert!(evaluate_condition("equals", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_not_equals_triggers() {
        let vals = vec![json!("something_else")];
        let mut p = params();
        p.value = Some(json!("expected"));
        assert!(evaluate_condition("not_equals", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_contains_triggers() {
        let vals = vec![json!("this has evil inside")];
        let mut p = params();
        p.value = Some(json!("evil"));
        assert!(evaluate_condition("contains", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_contains_no_trigger() {
        let vals = vec![json!("this is fine")];
        let mut p = params();
        p.value = Some(json!("evil"));
        assert!(!evaluate_condition("contains", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_not_contains_triggers() {
        let vals = vec![json!("missing keyword")];
        let mut p = params();
        p.value = Some(json!("required"));
        assert!(evaluate_condition("not_contains", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_size_exceeds_triggers() {
        let mut p = params();
        p.max_bytes = Some(100);
        assert!(evaluate_condition("size_exceeds", &[], &p, 200).unwrap());
    }

    #[test]
    fn test_size_exceeds_no_trigger() {
        let mut p = params();
        p.max_bytes = Some(100);
        assert!(!evaluate_condition("size_exceeds", &[], &p, 50).unwrap());
    }

    #[test]
    fn test_field_exists_triggers() {
        let vals = vec![json!("anything")];
        assert!(evaluate_condition("field_exists", &vals, &params(), 0).unwrap());
    }

    #[test]
    fn test_field_exists_no_trigger() {
        let vals: Vec<serde_json::Value> = vec![];
        assert!(!evaluate_condition("field_exists", &vals, &params(), 0).unwrap());
    }

    #[test]
    fn test_field_not_exists_triggers() {
        let vals: Vec<serde_json::Value> = vec![];
        assert!(evaluate_condition("field_not_exists", &vals, &params(), 0).unwrap());
    }

    #[test]
    fn test_field_not_exists_no_trigger() {
        let vals = vec![json!("present")];
        assert!(!evaluate_condition("field_not_exists", &vals, &params(), 0).unwrap());
    }

    #[test]
    fn test_unknown_condition_errors() {
        let result = evaluate_condition("made_up", &[], &params(), 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_non_string_value_skipped_by_regex() {
        let vals = vec![json!(12345)];
        let mut p = params();
        p.pattern = Some(r"\d+".to_string());
        assert!(!evaluate_condition("regex_match", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_non_url_value_skipped_by_domain() {
        let vals = vec![json!("not a url")];
        let mut p = params();
        p.values = vec!["*.safe.com".to_string()];
        assert!(!evaluate_condition("domain_not_in", &vals, &p, 0).unwrap());
    }
}
