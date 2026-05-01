use serde_json::Value;
use std::collections::HashSet;

pub fn redact_headers(headers: &mut Value, fields: &[String], redaction_text: &str) {
    let Some(header_map) = headers.as_object_mut() else {
        return;
    };

    let redact_fields = normalize_fields(fields);
    for (header_name, header_value) in header_map.iter_mut() {
        if redact_fields.contains(&header_name.to_ascii_lowercase()) {
            *header_value = Value::String(redaction_text.to_string());
        }
    }
}

pub fn redact_json_fields(value: &mut Value, fields: &[String], redaction_text: &str) {
    let redact_fields = normalize_fields(fields);
    redact_json_fields_inner(value, &redact_fields, redaction_text);
}

fn redact_json_fields_inner(
    value: &mut Value,
    redact_fields: &HashSet<String>,
    redaction_text: &str,
) {
    match value {
        Value::Object(map) => {
            for (field_name, field_value) in map.iter_mut() {
                if redact_fields.contains(&field_name.to_ascii_lowercase()) {
                    *field_value = Value::String(redaction_text.to_string());
                } else {
                    redact_json_fields_inner(field_value, redact_fields, redaction_text);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_json_fields_inner(item, redact_fields, redaction_text);
            }
        }
        _ => {}
    }
}

fn normalize_fields(fields: &[String]) -> HashSet<String> {
    fields
        .iter()
        .map(|field| field.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn redacts_headers_case_insensitively() {
        let mut headers = json!({
            "Authorization": "Bearer secret",
            "x-request-id": "req-123",
            "X-API-Key": "abc123"
        });

        super::redact_headers(
            &mut headers,
            &["authorization".into(), "x-api-key".into()],
            "[REDACTED]",
        );

        assert_eq!(headers["Authorization"], "[REDACTED]");
        assert_eq!(headers["X-API-Key"], "[REDACTED]");
        assert_eq!(headers["x-request-id"], "req-123");
    }

    #[test]
    fn redacts_nested_json_fields_case_insensitively() {
        let mut payload = json!({
            "email": "ops@example.com",
            "Password": "plaintext",
            "nested": {
                "token": "abc123",
                "profile": {
                    "SSN": "123-45-6789"
                }
            }
        });

        super::redact_json_fields(
            &mut payload,
            &["password".into(), "token".into(), "ssn".into()],
            "[REDACTED]",
        );

        assert_eq!(payload["email"], "ops@example.com");
        assert_eq!(payload["Password"], "[REDACTED]");
        assert_eq!(payload["nested"]["token"], "[REDACTED]");
        assert_eq!(payload["nested"]["profile"]["SSN"], "[REDACTED]");
    }
}
