use crate::auth::api_keys::{generate_api_key, hash_api_key, key_prefix, IssuedApiKey};
use crate::tenant::Tenant;

pub struct TenantRepository {
    pool: sqlx::PgPool,
}

impl TenantRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_tenant(&self, name: &str) -> Result<Tenant, sqlx::Error> {
        let id = uuid::Uuid::new_v4();
        let created_at = chrono::Utc::now();
        sqlx::query(
            "insert into tenants (id, name, status, created_at) values ($1, $2, 'active', $3)",
        )
        .bind(id)
        .bind(name)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        Ok(Tenant {
            id,
            name: name.to_string(),
            status: "active".to_string(),
            created_at,
            disabled_at: None,
        })
    }

    pub async fn create_api_key(
        &self,
        tenant_id: uuid::Uuid,
        name: &str,
    ) -> Result<IssuedApiKey, sqlx::Error> {
        let id = uuid::Uuid::new_v4();
        let raw_key = generate_api_key();
        let key_prefix = key_prefix(&raw_key);
        let key_hash = hash_api_key(&raw_key);

        sqlx::query(
            r#"
            insert into api_keys (id, tenant_id, key_prefix, key_hash, name)
            values ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(&key_prefix)
        .bind(&key_hash)
        .bind(name)
        .execute(&self.pool)
        .await?;

        Ok(IssuedApiKey {
            id,
            tenant_id,
            name: name.to_string(),
            key_prefix,
            raw_key,
        })
    }
}