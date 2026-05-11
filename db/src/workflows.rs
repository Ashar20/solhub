use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    models::{now_ts, ts_to_dt, Workflow},
    Db, DbError, NewWorkflow,
};

impl Db {
    pub async fn create_workflow(&self, w: NewWorkflow) -> Result<Workflow, DbError> {
        let id = Uuid::new_v4().to_string();
        let org_id_str = w.org_id.to_string();
        let trigger_config = serde_json::to_string(&w.trigger_config)?;
        let steps = serde_json::to_string(&w.steps)?;
        let is_public: i64 = if w.is_public { 1 } else { 0 };
        let now = now_ts();

        sqlx::query(
            "INSERT INTO workflows
                (id, org_id, name, trigger_type, trigger_config, steps,
                 is_active, is_public, fee_per_exec_usdc, execution_count,
                 created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, 0, ?9, ?9)",
        )
        .bind(&id)
        .bind(&org_id_str)
        .bind(&w.name)
        .bind(&w.trigger_type)
        .bind(&trigger_config)
        .bind(&steps)
        .bind(is_public)
        .bind(w.fee_per_exec_usdc)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_workflow_required(&id).await
    }

    pub async fn get_workflow(&self, id: Uuid) -> Result<Option<Workflow>, DbError> {
        let id_str = id.to_string();
        self.get_workflow_optional(&id_str).await
    }

    pub async fn list_workflows(
        &self,
        org_id: Uuid,
        active_only: bool,
    ) -> Result<Vec<Workflow>, DbError> {
        let org_id_str = org_id.to_string();

        let rows = if active_only {
            sqlx::query(
                "SELECT id, org_id, name, trigger_type, trigger_config, steps,
                        is_active, is_public, onchain_pda, fee_per_exec_usdc,
                        execution_count, created_at, updated_at
                 FROM workflows
                 WHERE org_id = ?1 AND is_active = 1
                 ORDER BY created_at DESC",
            )
            .bind(&org_id_str)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id, org_id, name, trigger_type, trigger_config, steps,
                        is_active, is_public, onchain_pda, fee_per_exec_usdc,
                        execution_count, created_at, updated_at
                 FROM workflows
                 WHERE org_id = ?1
                 ORDER BY created_at DESC",
            )
            .bind(&org_id_str)
            .fetch_all(&self.pool)
            .await?
        };

        rows.iter().map(row_to_workflow).collect()
    }

    pub async fn update_workflow(
        &self,
        id: Uuid,
        trigger_config: Option<Value>,
        steps: Option<Value>,
        is_active: Option<bool>,
    ) -> Result<Workflow, DbError> {
        let id_str = id.to_string();
        let now = now_ts();

        if let Some(tc) = trigger_config {
            let tc_str = serde_json::to_string(&tc)?;
            sqlx::query(
                "UPDATE workflows SET trigger_config = ?1, updated_at = ?2 WHERE id = ?3",
            )
            .bind(&tc_str)
            .bind(now)
            .bind(&id_str)
            .execute(&self.pool)
            .await?;
        }

        if let Some(s) = steps {
            let s_str = serde_json::to_string(&s)?;
            sqlx::query("UPDATE workflows SET steps = ?1, updated_at = ?2 WHERE id = ?3")
                .bind(&s_str)
                .bind(now)
                .bind(&id_str)
                .execute(&self.pool)
                .await?;
        }

        if let Some(active) = is_active {
            let active_int: i64 = if active { 1 } else { 0 };
            sqlx::query(
                "UPDATE workflows SET is_active = ?1, updated_at = ?2 WHERE id = ?3",
            )
            .bind(active_int)
            .bind(now)
            .bind(&id_str)
            .execute(&self.pool)
            .await?;
        }

        // Always update updated_at
        sqlx::query("UPDATE workflows SET updated_at = ?1 WHERE id = ?2")
            .bind(now)
            .bind(&id_str)
            .execute(&self.pool)
            .await?;

        self.get_workflow_required(&id_str).await
    }

    /// Soft delete: set is_active = false.
    pub async fn delete_workflow(&self, id: Uuid) -> Result<(), DbError> {
        let id_str = id.to_string();
        let now = now_ts();

        sqlx::query("UPDATE workflows SET is_active = 0, updated_at = ?1 WHERE id = ?2")
            .bind(now)
            .bind(&id_str)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn set_workflow_pda(&self, id: Uuid, pda: &str) -> Result<(), DbError> {
        let id_str = id.to_string();
        let now = now_ts();

        sqlx::query(
            "UPDATE workflows SET onchain_pda = ?1, updated_at = ?2 WHERE id = ?3",
        )
        .bind(pda)
        .bind(now)
        .bind(&id_str)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Returns all active workflows whose trigger_type is "cron".
    pub async fn list_active_cron_workflows(&self) -> Result<Vec<Workflow>, DbError> {
        let rows = sqlx::query(
            "SELECT id, org_id, name, trigger_type, trigger_config, steps,
                    is_active, is_public, onchain_pda, fee_per_exec_usdc,
                    execution_count, created_at, updated_at
             FROM workflows
             WHERE is_active = 1 AND trigger_type = 'cron'
             ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_workflow).collect()
    }

    pub async fn increment_execution_count(&self, id: Uuid) -> Result<(), DbError> {
        let id_str = id.to_string();

        sqlx::query(
            "UPDATE workflows SET execution_count = execution_count + 1 WHERE id = ?1",
        )
        .bind(&id_str)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    pub(crate) async fn get_workflow_optional(
        &self,
        id_str: &str,
    ) -> Result<Option<Workflow>, DbError> {
        let row = sqlx::query(
            "SELECT id, org_id, name, trigger_type, trigger_config, steps,
                    is_active, is_public, onchain_pda, fee_per_exec_usdc,
                    execution_count, created_at, updated_at
             FROM workflows WHERE id = ?1",
        )
        .bind(id_str)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| row_to_workflow(&r)).transpose()
    }

    pub(crate) async fn get_workflow_required(&self, id_str: &str) -> Result<Workflow, DbError> {
        self.get_workflow_optional(id_str)
            .await?
            .ok_or(DbError::NotFound)
    }
}

// ---------------------------------------------------------------------------
// Row conversion helper
// ---------------------------------------------------------------------------

fn row_to_workflow(r: &sqlx::sqlite::SqliteRow) -> Result<Workflow, DbError> {
    let id_str: String = r.try_get("id")?;
    let org_id_str: String = r.try_get("org_id")?;
    let trigger_config_str: String = r.try_get("trigger_config")?;
    let steps_str: String = r.try_get("steps")?;
    let is_active: i64 = r.try_get("is_active")?;
    let is_public: i64 = r.try_get("is_public")?;

    Ok(Workflow {
        id: Uuid::parse_str(&id_str)?,
        org_id: Uuid::parse_str(&org_id_str)?,
        name: r.try_get("name")?,
        trigger_type: r.try_get("trigger_type")?,
        trigger_config: serde_json::from_str(&trigger_config_str)?,
        steps: serde_json::from_str(&steps_str)?,
        is_active: is_active != 0,
        is_public: is_public != 0,
        onchain_pda: r.try_get("onchain_pda")?,
        fee_per_exec_usdc: r.try_get("fee_per_exec_usdc")?,
        execution_count: r.try_get("execution_count")?,
        created_at: ts_to_dt(r.try_get("created_at")?),
        updated_at: ts_to_dt(r.try_get("updated_at")?),
    })
}
