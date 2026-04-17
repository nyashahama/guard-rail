use crate::auth::api_keys::{IssuedApiKey, generate_api_key, hash_api_key, key_prefix};
use crate::tenant::Tenant;
use crate::tenant::cache::{CachedApiKey, TenantAuthSnapshot};
use sqlx::Row;

#[derive(Clone)]
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

    pub async fn bind_route(
        &self,
        route_id: &str,
        tenant_id: uuid::Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            insert into tenant_routes (route_id, tenant_id)
            values ($1, $2)
            on conflict (route_id) do update
            set tenant_id = excluded.tenant_id,
                updated_at = now()
            "#,
        )
        .bind(route_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn revoke_api_key(
        &self,
        key_id: uuid::Uuid,
        reason: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("update api_keys set revoked_at = now(), revoked_reason = $2 where id = $1")
            .bind(key_id)
            .bind(reason)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_tenants(&self) -> Result<Vec<Tenant>, sqlx::Error> {
        let rows = sqlx::query("select id, name, status, created_at, disabled_at from tenants order by created_at desc")
            .fetch_all(&self.pool)
            .await?;

        let tenants = rows
            .into_iter()
            .map(|row| Tenant {
                id: row.get("id"),
                name: row.get("name"),
                status: row.get("status"),
                created_at: row.get("created_at"),
                disabled_at: row.get("disabled_at"),
            })
            .collect();

        Ok(tenants)
    }

    pub async fn list_api_keys(
        &self,
        tenant_id: uuid::Uuid,
    ) -> Result<Vec<super::api::ApiKeyListItem>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            select id, name, key_prefix, created_at, revoked_at
            from api_keys
            where tenant_id = $1
            order by created_at desc
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        let keys = rows
            .into_iter()
            .map(|row| super::api::ApiKeyListItem {
                id: row.get("id"),
                name: row.get("name"),
                key_prefix: row.get("key_prefix"),
                created_at: row.get("created_at"),
                revoked_at: row.get("revoked_at"),
            })
            .collect();

        Ok(keys)
    }

    pub async fn load_auth_snapshot(&self) -> Result<TenantAuthSnapshot, sqlx::Error> {
        let route_rows = sqlx::query("select route_id, tenant_id from tenant_routes")
            .fetch_all(&self.pool)
            .await?;
        let key_rows = sqlx::query(
            r#"
            select
                api_keys.id,
                api_keys.tenant_id,
                api_keys.key_hash,
                api_keys.key_prefix,
                api_keys.name,
                api_keys.revoked_at,
                tenants.status as tenant_status
            from api_keys
            join tenants on tenants.id = api_keys.tenant_id
            where api_keys.revoked_at is null
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let route_bindings = route_rows
            .into_iter()
            .map(|row| {
                let route_id: String = row.get("route_id");
                let tenant_id: uuid::Uuid = row.get("tenant_id");
                (route_id, tenant_id)
            })
            .collect();

        let api_keys = key_rows
            .into_iter()
            .map(|row| {
                let key_hash: String = row.get("key_hash");
                let cached = CachedApiKey {
                    id: row.get("id"),
                    tenant_id: row.get("tenant_id"),
                    name: row.get("name"),
                    key_prefix: row.get("key_prefix"),
                    tenant_status: row.get("tenant_status"),
                };
                (key_hash, cached)
            })
            .collect();

        Ok(TenantAuthSnapshot {
            route_bindings,
            api_keys,
        })
    }
}
