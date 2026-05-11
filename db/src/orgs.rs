use sqlx::Row;
use uuid::Uuid;

use crate::{
    models::{now_ts, ts_to_dt, ApiKey, Organization},
    Db, DbError,
};

impl Db {
    pub async fn create_org(
        &self,
        name: &str,
        wallet_address: Option<&str>,
    ) -> Result<Organization, DbError> {
        let id = Uuid::new_v4().to_string();
        let now = now_ts();

        sqlx::query(
            "INSERT INTO organizations (id, name, wallet_address, credits_usdc, created_at)
             VALUES (?1, ?2, ?3, 0, ?4)",
        )
        .bind(&id)
        .bind(name)
        .bind(wallet_address)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_org_required(&id).await
    }

    pub async fn get_org(&self, id: Uuid) -> Result<Option<Organization>, DbError> {
        let id_str = id.to_string();
        self.get_org_optional(&id_str).await
    }

    pub async fn get_org_by_api_key_hash(
        &self,
        key_hash: &str,
    ) -> Result<Option<Organization>, DbError> {
        let row = sqlx::query(
            "SELECT o.id, o.name, o.wallet_address, o.credits_usdc, o.created_at
             FROM organizations o
             INNER JOIN api_keys k ON k.org_id = o.id
             WHERE k.key_hash = ?1
               AND k.revoked_at IS NULL",
        )
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| row_to_org(&r)).transpose()
    }

    pub async fn create_api_key(
        &self,
        org_id: Uuid,
        key_hash: &str,
        name: Option<&str>,
    ) -> Result<ApiKey, DbError> {
        let id = Uuid::new_v4().to_string();
        let org_id_str = org_id.to_string();
        let now = now_ts();

        sqlx::query(
            "INSERT INTO api_keys (id, org_id, key_hash, name, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(&id)
        .bind(&org_id_str)
        .bind(key_hash)
        .bind(name)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_api_key_required(&id).await
    }

    pub async fn list_api_keys(&self, org_id: Uuid) -> Result<Vec<ApiKey>, DbError> {
        let org_id_str = org_id.to_string();

        let rows = sqlx::query(
            "SELECT id, org_id, key_hash, name, last_used_at, created_at, revoked_at
             FROM api_keys WHERE org_id = ?1 ORDER BY created_at DESC",
        )
        .bind(&org_id_str)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_api_key).collect()
    }

    pub async fn revoke_api_key(&self, key_id: Uuid) -> Result<(), DbError> {
        let key_id_str = key_id.to_string();
        let now = now_ts();

        sqlx::query("UPDATE api_keys SET revoked_at = ?1 WHERE id = ?2")
            .bind(now)
            .bind(&key_id_str)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn touch_api_key(&self, key_hash: &str) -> Result<(), DbError> {
        let now = now_ts();

        sqlx::query("UPDATE api_keys SET last_used_at = ?1 WHERE key_hash = ?2")
            .bind(now)
            .bind(key_hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    async fn get_org_optional(&self, id_str: &str) -> Result<Option<Organization>, DbError> {
        let row = sqlx::query(
            "SELECT id, name, wallet_address, credits_usdc, created_at
             FROM organizations WHERE id = ?1",
        )
        .bind(id_str)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| row_to_org(&r)).transpose()
    }

    async fn get_org_required(&self, id_str: &str) -> Result<Organization, DbError> {
        self.get_org_optional(id_str)
            .await?
            .ok_or(DbError::NotFound)
    }

    async fn get_api_key_required(&self, id_str: &str) -> Result<ApiKey, DbError> {
        let row = sqlx::query(
            "SELECT id, org_id, key_hash, name, last_used_at, created_at, revoked_at
             FROM api_keys WHERE id = ?1",
        )
        .bind(id_str)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| row_to_api_key(&r))
            .transpose()?
            .ok_or(DbError::NotFound)
    }
}

// ---------------------------------------------------------------------------
// Row conversion helpers
// ---------------------------------------------------------------------------

fn row_to_org(r: &sqlx::sqlite::SqliteRow) -> Result<Organization, DbError> {
    let id_str: String = r.try_get("id")?;
    Ok(Organization {
        id: Uuid::parse_str(&id_str)?,
        name: r.try_get("name")?,
        wallet_address: r.try_get("wallet_address")?,
        credits_usdc: r.try_get("credits_usdc")?,
        created_at: ts_to_dt(r.try_get("created_at")?),
    })
}

fn row_to_api_key(r: &sqlx::sqlite::SqliteRow) -> Result<ApiKey, DbError> {
    let id_str: String = r.try_get("id")?;
    let org_id_str: String = r.try_get("org_id")?;
    Ok(ApiKey {
        id: Uuid::parse_str(&id_str)?,
        org_id: Uuid::parse_str(&org_id_str)?,
        key_hash: r.try_get("key_hash")?,
        name: r.try_get("name")?,
        last_used_at: r.try_get::<Option<i64>, _>("last_used_at")?.map(ts_to_dt),
        created_at: ts_to_dt(r.try_get("created_at")?),
        revoked_at: r.try_get::<Option<i64>, _>("revoked_at")?.map(ts_to_dt),
    })
}
