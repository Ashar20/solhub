use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    models::{now_ts, ts_to_dt, WorkflowRun},
    Db, DbError, NewRun,
};

const TERMINAL_STATUSES: &[&str] = &["Confirmed", "Failed", "Skipped"];

impl Db {
    pub async fn create_run(&self, r: NewRun) -> Result<WorkflowRun, DbError> {
        let run_id = Uuid::new_v4().to_string();
        let workflow_id_str = r.workflow_id.to_string();
        let org_id_str = r.org_id.to_string();
        let now = now_ts();

        sqlx::query(
            "INSERT INTO workflow_runs
                (run_id, workflow_id, org_id, status, triggered_by, steps_log, started_at)
             VALUES (?1, ?2, ?3, 'Pending', ?4, '[]', ?5)",
        )
        .bind(&run_id)
        .bind(&workflow_id_str)
        .bind(&org_id_str)
        .bind(&r.triggered_by)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_run_required(&run_id).await
    }

    pub async fn get_run(&self, run_id: Uuid) -> Result<Option<WorkflowRun>, DbError> {
        let run_id_str = run_id.to_string();
        self.get_run_optional(&run_id_str).await
    }

    pub async fn list_runs(
        &self,
        org_id: Uuid,
        workflow_id: Option<Uuid>,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<WorkflowRun>, DbError> {
        let org_id_str = org_id.to_string();
        let wf_str = workflow_id.map(|u| u.to_string());

        let rows = sqlx::query(
            "SELECT run_id, workflow_id, org_id, status, triggered_by,
                    steps_log, slot, signature, fee_lamports, jito_tip_lamports,
                    error_message, started_at, completed_at
             FROM workflow_runs
             WHERE org_id = ?1
               AND (?2 IS NULL OR workflow_id = ?2)
               AND (?3 IS NULL OR status = ?3)
             ORDER BY started_at DESC
             LIMIT ?4",
        )
        .bind(&org_id_str)
        .bind(&wf_str)
        .bind(status)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_run).collect()
    }

    pub async fn update_run_status(
        &self,
        run_id: Uuid,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), DbError> {
        let run_id_str = run_id.to_string();
        let is_terminal = TERMINAL_STATUSES.contains(&status);
        let completed_at: Option<i64> = if is_terminal { Some(now_ts()) } else { None };

        sqlx::query(
            "UPDATE workflow_runs
             SET status = ?1, error_message = ?2, completed_at = ?3
             WHERE run_id = ?4",
        )
        .bind(status)
        .bind(error)
        .bind(completed_at)
        .bind(&run_id_str)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn record_run_outcome(
        &self,
        run_id: Uuid,
        slot: u64,
        signature: &str,
        fee: u64,
        tip: u64,
    ) -> Result<(), DbError> {
        let run_id_str = run_id.to_string();
        let slot_i: i64 = slot as i64;
        let fee_i: i64 = fee as i64;
        let tip_i: i64 = tip as i64;

        sqlx::query(
            "UPDATE workflow_runs
             SET slot = ?1, signature = ?2, fee_lamports = ?3, jito_tip_lamports = ?4
             WHERE run_id = ?5",
        )
        .bind(slot_i)
        .bind(signature)
        .bind(fee_i)
        .bind(tip_i)
        .bind(&run_id_str)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Atomically appends `step_log` to the JSON array in `steps_log`.
    pub async fn append_step_log(
        &self,
        run_id: Uuid,
        step_log: Value,
    ) -> Result<(), DbError> {
        let run_id_str = run_id.to_string();

        let mut tx = self.pool.begin().await?;

        let current: String = sqlx::query_scalar(
            "SELECT steps_log FROM workflow_runs WHERE run_id = ?1",
        )
        .bind(&run_id_str)
        .fetch_one(&mut *tx)
        .await?;

        let mut arr: Vec<Value> = serde_json::from_str(&current)?;
        arr.push(step_log);
        let updated = serde_json::to_string(&arr)?;

        sqlx::query("UPDATE workflow_runs SET steps_log = ?1 WHERE run_id = ?2")
            .bind(&updated)
            .bind(&run_id_str)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Returns run_ids of all runs in `Pending` status, up to `limit`.
    pub async fn list_pending_run_ids(&self, limit: i64) -> Result<Vec<uuid::Uuid>, DbError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT run_id FROM workflow_runs WHERE status = 'Pending' ORDER BY started_at ASC LIMIT ?1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|(s,)| uuid::Uuid::parse_str(&s).map_err(DbError::Uuid))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    async fn get_run_optional(&self, run_id_str: &str) -> Result<Option<WorkflowRun>, DbError> {
        let row = sqlx::query(
            "SELECT run_id, workflow_id, org_id, status, triggered_by,
                    steps_log, slot, signature, fee_lamports, jito_tip_lamports,
                    error_message, started_at, completed_at
             FROM workflow_runs WHERE run_id = ?1",
        )
        .bind(run_id_str)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| row_to_run(&r)).transpose()
    }

    async fn get_run_required(&self, run_id_str: &str) -> Result<WorkflowRun, DbError> {
        self.get_run_optional(run_id_str)
            .await?
            .ok_or(DbError::NotFound)
    }
}

// ---------------------------------------------------------------------------
// Row conversion helper
// ---------------------------------------------------------------------------

fn row_to_run(r: &sqlx::sqlite::SqliteRow) -> Result<WorkflowRun, DbError> {
    let run_id_str: String = r.try_get("run_id")?;
    let workflow_id_str: String = r.try_get("workflow_id")?;
    let org_id_str: String = r.try_get("org_id")?;
    let steps_log_str: String = r.try_get("steps_log")?;

    Ok(WorkflowRun {
        run_id: Uuid::parse_str(&run_id_str)?,
        workflow_id: Uuid::parse_str(&workflow_id_str)?,
        org_id: Uuid::parse_str(&org_id_str)?,
        status: r.try_get("status")?,
        triggered_by: r.try_get("triggered_by")?,
        steps_log: serde_json::from_str(&steps_log_str)?,
        slot: r.try_get("slot")?,
        signature: r.try_get("signature")?,
        fee_lamports: r.try_get("fee_lamports")?,
        jito_tip_lamports: r.try_get("jito_tip_lamports")?,
        error_message: r.try_get("error_message")?,
        started_at: ts_to_dt(r.try_get("started_at")?),
        completed_at: r.try_get::<Option<i64>, _>("completed_at")?.map(ts_to_dt),
    })
}
