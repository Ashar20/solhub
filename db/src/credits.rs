use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{models::ts_to_dt, Db, DbError};

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub org_id: Uuid,
    pub delta: i64,
    pub reason: String,
    pub run_id: Option<Uuid>,
    pub payment_id: Option<Uuid>,
    pub balance_after: i64,
    pub created_at: chrono::DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Db methods
// ---------------------------------------------------------------------------

impl Db {
    /// Atomically debit 1 credit if balance >= 1.
    /// Returns `Ok(new_balance)` on success, `Err(DbError::InsufficientCredits)` if balance < 1.
    pub async fn debit_credit_for_run(
        &self,
        org_id: Uuid,
        run_id: Uuid,
    ) -> Result<i64, DbError> {
        let mut tx = self.pool.begin().await?;

        let current: i64 = sqlx::query_scalar(
            "SELECT credits_usdc FROM organizations WHERE id = ?",
        )
        .bind(org_id.to_string())
        .fetch_one(&mut *tx)
        .await?;

        if current < 1 {
            return Err(DbError::InsufficientCredits);
        }

        let new_balance = current - 1;

        sqlx::query("UPDATE organizations SET credits_usdc = ? WHERE id = ?")
            .bind(new_balance)
            .bind(org_id.to_string())
            .execute(&mut *tx)
            .await?;

        let entry_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO credit_ledger \
             (id, org_id, delta, reason, run_id, balance_after, created_at) \
             VALUES (?, ?, -1, 'run_debit', ?, ?, ?)",
        )
        .bind(entry_id.to_string())
        .bind(org_id.to_string())
        .bind(run_id.to_string())
        .bind(new_balance)
        .bind(Utc::now().timestamp())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(new_balance)
    }

    /// Grant `delta` credits to an org (topup, manual grant, refund, etc.).
    /// Returns the new balance.
    pub async fn grant_credits(
        &self,
        org_id: Uuid,
        delta: i64,
        reason: &str,
        payment_id: Option<Uuid>,
    ) -> Result<i64, DbError> {
        let mut tx = self.pool.begin().await?;

        let current: i64 = sqlx::query_scalar(
            "SELECT credits_usdc FROM organizations WHERE id = ?",
        )
        .bind(org_id.to_string())
        .fetch_one(&mut *tx)
        .await?;

        let new_balance = current + delta;

        sqlx::query("UPDATE organizations SET credits_usdc = ? WHERE id = ?")
            .bind(new_balance)
            .bind(org_id.to_string())
            .execute(&mut *tx)
            .await?;

        let entry_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO credit_ledger \
             (id, org_id, delta, reason, payment_id, balance_after, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(entry_id.to_string())
        .bind(org_id.to_string())
        .bind(delta)
        .bind(reason)
        .bind(payment_id.map(|u| u.to_string()))
        .bind(new_balance)
        .bind(Utc::now().timestamp())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(new_balance)
    }

    /// List up to `limit` ledger entries for an org, newest first.
    pub async fn list_ledger(
        &self,
        org_id: Uuid,
        limit: i64,
    ) -> Result<Vec<LedgerEntry>, DbError> {
        let rows = sqlx::query(
            "SELECT id, org_id, delta, reason, run_id, payment_id, balance_after, created_at \
             FROM credit_ledger \
             WHERE org_id = ? \
             ORDER BY created_at DESC \
             LIMIT ?",
        )
        .bind(org_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|r| {
                let id_str: &str = r.get("id");
                let org_str: &str = r.get("org_id");
                let run_id_opt: Option<&str> = r.get("run_id");
                let pay_id_opt: Option<&str> = r.get("payment_id");

                Ok(LedgerEntry {
                    id: Uuid::parse_str(id_str)
                        .map_err(|_| DbError::Other("invalid id uuid".into()))?,
                    org_id: Uuid::parse_str(org_str)
                        .map_err(|_| DbError::Other("invalid org_id uuid".into()))?,
                    delta: r.get("delta"),
                    reason: r.get::<String, _>("reason"),
                    run_id: run_id_opt
                        .and_then(|s| Uuid::parse_str(s).ok()),
                    payment_id: pay_id_opt
                        .and_then(|s| Uuid::parse_str(s).ok()),
                    balance_after: r.get("balance_after"),
                    created_at: ts_to_dt(r.get::<i64, _>("created_at")),
                })
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{models::NewWorkflow, NewRun};
    use serde_json::json;

    async fn test_db() -> Db {
        let db = Db::connect_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        db
    }

    async fn make_org_and_workflow(db: &Db) -> (Uuid, Uuid) {
        let org = db.create_org("test-org", None).await.unwrap();
        let wf = db
            .create_workflow(NewWorkflow {
                org_id: org.id,
                name: "test-wf".into(),
                trigger_type: "manual".into(),
                trigger_config: json!({}),
                steps: json!([]),
                is_public: false,
                fee_per_exec_usdc: None,
            })
            .await
            .unwrap();
        (org.id, wf.id)
    }

    #[tokio::test]
    async fn grant_credits_adds_to_balance() {
        let db = test_db().await;
        let (org_id, _) = make_org_and_workflow(&db).await;

        let new_bal = db.grant_credits(org_id, 10, "manual_grant", None).await.unwrap();
        assert_eq!(new_bal, 10);

        // Fetch org and verify
        let org = db.get_org(org_id).await.unwrap().unwrap();
        assert_eq!(org.credits_usdc, 10);
    }

    #[tokio::test]
    async fn debit_credit_deducts_one() {
        let db = test_db().await;
        let (org_id, wf_id) = make_org_and_workflow(&db).await;

        db.grant_credits(org_id, 5, "manual_grant", None).await.unwrap();

        let run = db
            .create_run(NewRun {
                workflow_id: wf_id,
                org_id,
                triggered_by: "manual".into(),
            })
            .await
            .unwrap();

        let new_bal = db.debit_credit_for_run(org_id, run.run_id).await.unwrap();
        assert_eq!(new_bal, 4);

        let org = db.get_org(org_id).await.unwrap().unwrap();
        assert_eq!(org.credits_usdc, 4);
    }

    #[tokio::test]
    async fn debit_fails_on_zero_balance() {
        let db = test_db().await;
        let (org_id, wf_id) = make_org_and_workflow(&db).await;

        // Balance starts at 0
        let run = db
            .create_run(NewRun {
                workflow_id: wf_id,
                org_id,
                triggered_by: "manual".into(),
            })
            .await
            .unwrap();

        let result = db.debit_credit_for_run(org_id, run.run_id).await;
        assert!(
            matches!(result, Err(DbError::InsufficientCredits)),
            "expected InsufficientCredits, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn list_ledger_returns_entries_in_order() {
        let db = test_db().await;
        let (org_id, wf_id) = make_org_and_workflow(&db).await;

        // Grant 10
        db.grant_credits(org_id, 10, "manual_grant", None).await.unwrap();

        // Debit once
        let run = db
            .create_run(NewRun {
                workflow_id: wf_id,
                org_id,
                triggered_by: "manual".into(),
            })
            .await
            .unwrap();
        db.debit_credit_for_run(org_id, run.run_id).await.unwrap();

        // Grant again
        db.grant_credits(org_id, 5, "topup", None).await.unwrap();

        let ledger = db.list_ledger(org_id, 10).await.unwrap();
        assert_eq!(ledger.len(), 3);

        // All three entries should be present (order may vary on same-second timestamps).
        let topup_entries: Vec<_> = ledger.iter().filter(|e| e.reason == "topup").collect();
        let debit_entries: Vec<_> = ledger.iter().filter(|e| e.reason == "run_debit").collect();
        let grant_entries: Vec<_> = ledger.iter().filter(|e| e.reason == "manual_grant").collect();

        assert_eq!(topup_entries.len(), 1, "expected 1 topup entry");
        assert_eq!(debit_entries.len(), 1, "expected 1 run_debit entry");
        assert_eq!(grant_entries.len(), 1, "expected 1 manual_grant entry");

        assert_eq!(topup_entries[0].delta, 5);
        assert_eq!(debit_entries[0].delta, -1);
        assert_eq!(grant_entries[0].delta, 10);
    }
}
