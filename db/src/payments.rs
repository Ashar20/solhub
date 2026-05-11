use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    models::{now_ts, ts_to_dt},
    Db, DbError,
};

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub payer_pubkey: String,
    pub recipient: String,
    pub network: String,
    pub amount_lamports: i64,
    pub signature: String,
    /// 'pending' | 'verified' | 'rejected'
    pub status: String,
    pub run_id: Option<Uuid>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// Db methods
// ---------------------------------------------------------------------------

impl Db {
    /// Insert a new payment row with status='pending'.
    pub async fn create_payment(
        &self,
        workflow_id: Uuid,
        payer_pubkey: &str,
        recipient: &str,
        network: &str,
        amount_lamports: i64,
        signature: &str,
    ) -> Result<Payment, DbError> {
        let id = Uuid::new_v4().to_string();
        let wf_str = workflow_id.to_string();
        let now = now_ts();

        sqlx::query(
            "INSERT INTO payments
                (id, workflow_id, payer_pubkey, recipient, network, amount_lamports,
                 signature, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8)",
        )
        .bind(&id)
        .bind(&wf_str)
        .bind(payer_pubkey)
        .bind(recipient)
        .bind(network)
        .bind(amount_lamports)
        .bind(signature)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_payment_required(&id).await
    }

    /// Look up a payment by its on-chain Solana transaction signature.
    pub async fn get_payment_by_signature(
        &self,
        signature: &str,
    ) -> Result<Option<Payment>, DbError> {
        let row = sqlx::query(
            "SELECT id, workflow_id, payer_pubkey, recipient, network, amount_lamports,
                    signature, status, run_id, error, created_at, verified_at
             FROM payments WHERE signature = ?1",
        )
        .bind(signature)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| row_to_payment(&r)).transpose()
    }

    /// Mark a payment as verified and link it to a workflow run.
    pub async fn mark_payment_verified(
        &self,
        payment_id: Uuid,
        run_id: Uuid,
    ) -> Result<(), DbError> {
        let id_str = payment_id.to_string();
        let run_str = run_id.to_string();
        let now = now_ts();

        sqlx::query(
            "UPDATE payments
             SET status = 'verified', run_id = ?1, verified_at = ?2
             WHERE id = ?3",
        )
        .bind(&run_str)
        .bind(now)
        .bind(&id_str)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark a payment as rejected with an error message.
    pub async fn mark_payment_rejected(
        &self,
        payment_id: Uuid,
        error: &str,
    ) -> Result<(), DbError> {
        let id_str = payment_id.to_string();

        sqlx::query(
            "UPDATE payments
             SET status = 'rejected', error = ?1
             WHERE id = ?2",
        )
        .bind(error)
        .bind(&id_str)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    async fn get_payment_required(&self, id: &str) -> Result<Payment, DbError> {
        let row = sqlx::query(
            "SELECT id, workflow_id, payer_pubkey, recipient, network, amount_lamports,
                    signature, status, run_id, error, created_at, verified_at
             FROM payments WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| row_to_payment(&r))
            .transpose()?
            .ok_or(DbError::NotFound)
    }
}

// ---------------------------------------------------------------------------
// Row conversion
// ---------------------------------------------------------------------------

fn row_to_payment(r: &sqlx::sqlite::SqliteRow) -> Result<Payment, DbError> {
    let id_str: String = r.try_get("id")?;
    let wf_str: String = r.try_get("workflow_id")?;
    let run_id_str: Option<String> = r.try_get("run_id")?;

    Ok(Payment {
        id: Uuid::parse_str(&id_str)?,
        workflow_id: Uuid::parse_str(&wf_str)?,
        payer_pubkey: r.try_get("payer_pubkey")?,
        recipient: r.try_get("recipient")?,
        network: r.try_get("network")?,
        amount_lamports: r.try_get("amount_lamports")?,
        signature: r.try_get("signature")?,
        status: r.try_get("status")?,
        run_id: run_id_str
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()?,
        error: r.try_get("error")?,
        created_at: ts_to_dt(r.try_get("created_at")?),
        verified_at: r
            .try_get::<Option<i64>, _>("verified_at")?
            .map(ts_to_dt),
    })
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
                name: "paid-workflow".into(),
                trigger_type: "manual".into(),
                trigger_config: json!({}),
                steps: json!([]),
                is_public: true,
                fee_per_exec_usdc: Some(5_000_000),
            })
            .await
            .unwrap();
        (org.id, wf.id)
    }

    #[tokio::test]
    async fn create_payment_then_lookup_by_signature() {
        let db = test_db().await;
        let (_org_id, wf_id) = make_org_and_workflow(&db).await;

        let sig = "5xSig1TestSignatureABCDEF";
        let payment = db
            .create_payment(
                wf_id,
                "PayerPubkeyABCDEF1234",
                "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb",
                "solana-devnet",
                5_000_000,
                sig,
            )
            .await
            .unwrap();

        assert_eq!(payment.workflow_id, wf_id);
        assert_eq!(payment.signature, sig);
        assert_eq!(payment.status, "pending");
        assert!(payment.run_id.is_none());
        assert!(payment.verified_at.is_none());

        // Lookup by signature
        let found = db.get_payment_by_signature(sig).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, payment.id);
        assert_eq!(found.payer_pubkey, "PayerPubkeyABCDEF1234");
    }

    #[tokio::test]
    async fn mark_payment_verified_updates_status_and_run_id() {
        let db = test_db().await;
        let (org_id, wf_id) = make_org_and_workflow(&db).await;

        let sig = "5xSig2TestSignatureXYZZZ";
        let payment = db
            .create_payment(
                wf_id,
                "PayerPubkeyXYZ",
                "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb",
                "solana-devnet",
                1_000_000,
                sig,
            )
            .await
            .unwrap();

        assert_eq!(payment.status, "pending");

        // Create a run to link to
        let run = db
            .create_run(NewRun {
                workflow_id: wf_id,
                org_id,
                triggered_by: "x402".into(),
            })
            .await
            .unwrap();

        db.mark_payment_verified(payment.id, run.run_id)
            .await
            .unwrap();

        // Re-fetch via signature to confirm update
        let updated = db.get_payment_by_signature(sig).await.unwrap().unwrap();
        assert_eq!(updated.status, "verified");
        assert_eq!(updated.run_id, Some(run.run_id));
        assert!(updated.verified_at.is_some());
    }

    #[tokio::test]
    async fn mark_payment_rejected_sets_error() {
        let db = test_db().await;
        let (_org_id, wf_id) = make_org_and_workflow(&db).await;

        let sig = "5xSig3TestSignatureReject";
        let payment = db
            .create_payment(
                wf_id,
                "PayerPubkeyReject",
                "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb",
                "solana-devnet",
                500_000,
                sig,
            )
            .await
            .unwrap();

        db.mark_payment_rejected(payment.id, "payment too old: 700s > 600s")
            .await
            .unwrap();

        let updated = db.get_payment_by_signature(sig).await.unwrap().unwrap();
        assert_eq!(updated.status, "rejected");
        assert_eq!(
            updated.error.as_deref(),
            Some("payment too old: 700s > 600s")
        );
    }

    #[tokio::test]
    async fn get_payment_by_signature_returns_none_for_unknown() {
        let db = test_db().await;
        let result = db
            .get_payment_by_signature("nonexistent-sig")
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
