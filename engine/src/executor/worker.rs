use crate::plugins::{ActionType, PluginRegistry};
use crate::wallet::Signer;
use db::Db;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

pub struct ExecutorWorker {
    pub db: Db,
    pub plugins: Arc<PluginRegistry>,
    pub signer: Arc<dyn Signer>,
    pub bundle_builder: Arc<dyn super::BundleBuilder>,
    pub simulator: Arc<dyn super::Simulator>,
}

impl ExecutorWorker {
    /// Drive a single run from `Pending` (or `Resumed`) through to `Confirmed` (or `Failed`).
    pub async fn execute_run(&self, run_id: Uuid) -> anyhow::Result<()> {
        let run = self
            .db
            .get_run(run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("run not found: {}", run_id))?;

        let wf = self
            .db
            .get_workflow(run.workflow_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("workflow not found: {}", run.workflow_id))?;

        // Determine resume offset. If a previous run paused at an approval gate,
        // `resume_from_step_index` is set; pick up there. (The scheduler may have
        // flipped status from Resumed → Triggered already, so don't rely on
        // status alone.)
        let start_index = run.resume_from_step_index.unwrap_or(0) as usize;

        self.db
            .update_run_status(run_id, "Triggered", None)
            .await?;

        // Parse steps from the workflow JSON array.
        let steps: Vec<Value> = serde_json::from_value(wf.steps.clone())?;
        let rpc = solana_client::nonblocking::rpc_client::RpcClient::new(
            std::env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string()),
        );

        let mut step_outputs: Vec<Value> = Vec::new();
        let mut all_txs = Vec::new();

        self.db
            .update_run_status(run_id, "Simulating", None)
            .await?;

        for (i, step) in steps.iter().enumerate().skip(start_index) {
            let plugin_id = step["plugin"].as_str().unwrap_or("");
            let action = step["action"].as_str().unwrap_or("");
            let params = &step["params"];
            let started = std::time::Instant::now();

            let plugin = match self.plugins.get(plugin_id) {
                Some(p) => p,
                None => {
                    let msg = format!("unknown plugin: {}", plugin_id);
                    self.append_step_failure(
                        run_id,
                        plugin_id,
                        action,
                        &msg,
                        started.elapsed().as_millis() as u64,
                        i,
                    )
                    .await?;
                    self.db
                        .update_run_status(run_id, "Failed", Some(&msg))
                        .await?;
                    return Ok(());
                }
            };

            let action_def = plugin.actions().into_iter().find(|a| a.id == action);
            let action_type = action_def.map(|a| a.action_type);

            let out: Value = match action_type {
                Some(ActionType::ReadOnly) => {
                    match plugin.read(action, params, &rpc).await {
                        Ok(v) => {
                            // Check for the approval-gate pause sentinel.
                            if v.get("__pause__").and_then(|p| p.as_bool()).unwrap_or(false) {
                                // Record the current step in the log before pausing.
                                self.db
                                    .append_step_log(
                                        run_id,
                                        json!({
                                            "step_id": step.get("id").cloned()
                                                .unwrap_or_else(|| json!(format!("step_{}", i))),
                                            "status": "WaitingApproval",
                                            "input":  params.clone(),
                                            "output": v.clone(),
                                            "duration_ms": started.elapsed().as_millis() as u64,
                                        }),
                                    )
                                    .await?;
                                // Store the next-step index so the executor can resume there.
                                self.db.set_resume_index(run_id, (i + 1) as i64).await?;
                                self.db
                                    .update_run_status(run_id, "WaitingApproval", None)
                                    .await?;
                                return Ok(());
                            }
                            v
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            self.append_step_failure(
                                run_id,
                                plugin_id,
                                action,
                                &msg,
                                started.elapsed().as_millis() as u64,
                                i,
                            )
                            .await?;
                            self.db
                                .update_run_status(run_id, "Failed", Some(&msg))
                                .await?;
                            return Ok(());
                        }
                    }
                }
                Some(ActionType::Notification) => {
                    match plugin.notify(action, params).await {
                        Ok(v) => v,
                        Err(e) => {
                            let msg = e.to_string();
                            self.append_step_failure(
                                run_id,
                                plugin_id,
                                action,
                                &msg,
                                started.elapsed().as_millis() as u64,
                                i,
                            )
                            .await?;
                            self.db
                                .update_run_status(run_id, "Failed", Some(&msg))
                                .await?;
                            return Ok(());
                        }
                    }
                }
                Some(ActionType::Transaction) | None => {
                    match plugin
                        .build_transactions(action, params, &self.signer.pubkey(), &rpc)
                        .await
                    {
                        Ok(txs) => {
                            for t in &txs {
                                let sim = self.simulator.simulate(t).await?;
                                let _ = sim;
                            }
                            all_txs.extend(txs);
                            json!({ "queued_transactions": all_txs.len() })
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            self.append_step_failure(
                                run_id,
                                plugin_id,
                                action,
                                &msg,
                                started.elapsed().as_millis() as u64,
                                i,
                            )
                            .await?;
                            self.db
                                .update_run_status(run_id, "Failed", Some(&msg))
                                .await?;
                            return Ok(());
                        }
                    }
                }
            };

            self.db
                .append_step_log(
                    run_id,
                    json!({
                        "step_id": step.get("id").cloned()
                            .unwrap_or_else(|| json!(format!("step_{}", i))),
                        "status": "Completed",
                        "input":  params.clone(),
                        "output": out.clone(),
                        "duration_ms": started.elapsed().as_millis() as u64,
                    }),
                )
                .await?;
            step_outputs.push(out);
        }

        if !all_txs.is_empty() {
            self.db
                .update_run_status(run_id, "Bundling", None)
                .await?;

            let mut signed_txs = Vec::with_capacity(all_txs.len());
            for t in all_txs {
                signed_txs.push(self.signer.sign_transaction(t).await?);
            }

            self.db
                .update_run_status(run_id, "Submitted", None)
                .await?;

            let result = self
                .bundle_builder
                .build_and_submit(signed_txs, self.signer.clone())
                .await?;
            let sig = result
                .signatures
                .first()
                .map(|s| s.to_string())
                .unwrap_or_default();
            self.db
                .record_run_outcome(run_id, 0, &sig, 0, result.tip_lamports)
                .await?;
        }

        self.db
            .update_run_status(run_id, "Confirmed", None)
            .await?;
        self.db.increment_execution_count(wf.id).await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    async fn append_step_failure(
        &self,
        run_id: Uuid,
        plugin: &str,
        action: &str,
        err: &str,
        ms: u64,
        idx: usize,
    ) -> anyhow::Result<()> {
        self.db
            .append_step_log(
                run_id,
                json!({
                    "step_id": format!("step_{}", idx),
                    "status": "Failed",
                    "input": json!({"plugin": plugin, "action": action}),
                    "output": null,
                    "duration_ms": ms,
                    "error": err,
                }),
            )
            .await?;
        Ok(())
    }
}
