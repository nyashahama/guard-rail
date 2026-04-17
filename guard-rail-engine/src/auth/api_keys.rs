#[derive(Debug, Clone)]
pub struct IssuedApiKey {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub name: String,
    pub key_prefix: String,
    pub raw_key: String,
}

pub fn generate_api_key() -> String {
    format!(
        "grk_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

pub fn key_prefix(raw_key: &str) -> String {
    raw_key.chars().take(12).collect()
}

pub fn hash_api_key(raw_key: &str) -> String {
    crate::audit::hash::hash_string(raw_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_api_key_is_deterministic() {
        let hash_a = hash_api_key("grk_test");
        let hash_b = hash_api_key("grk_test");
        assert_eq!(hash_a, hash_b);
    }
}
