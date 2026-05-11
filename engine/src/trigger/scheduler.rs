use crate::executor::ExecutorWorker;
use db::Db;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use uuid::Uuid;

/// Polls the DB for `Pending` runs and dispatches them to `ExecutorWorker`.
/// Also listens on `manual` channel for runs submitted by the API layer.
pub struct Scheduler {
    pub db: Db,
    pub executor: Arc<ExecutorWorker>,
    /// Manual trigger channel — the API/MCP layer publishes run IDs here.
    pub manual: mpsc::Receiver<Uuid>,
}

impl Scheduler {
    /// Run forever. Polls `Pending` runs every 500 ms and drains the manual channel.
    pub async fn run(mut self) -> anyhow::Result<()> {
        let mut tick =
            tokio::time::interval(std::time::Duration::from_millis(500));
        let mut tasks: JoinSet<()> = JoinSet::new();

        loop {
            tokio::select! {
                _ = tick.tick() => {
                    match self.db.list_runs_to_execute(50).await {
                        Ok(pending) => {
                            for run_id in pending {
                                // Mark as Triggered so the next poll doesn't
                                // re-pick the same run.
                                let _ = self.db.update_run_status(run_id, "Triggered", None).await;
                                let ex = self.executor.clone();
                                tasks.spawn(async move {
                                    if let Err(e) = ex.execute_run(run_id).await {
                                        tracing::error!(%run_id, error = %e, "executor failed");
                                    }
                                });
                            }
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "failed to list pending runs");
                        }
                    }
                }
                Some(run_id) = self.manual.recv() => {
                    let ex = self.executor.clone();
                    tasks.spawn(async move {
                        if let Err(e) = ex.execute_run(run_id).await {
                            tracing::error!(%run_id, error = %e, "manual run failed");
                        }
                    });
                }
                Some(res) = tasks.join_next() => {
                    if let Err(e) = res {
                        tracing::warn!(error = ?e, "task join error");
                    }
                }
            }
        }
    }
}
