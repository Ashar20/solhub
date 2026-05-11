use db::{Db, NewRun};
use tokio_cron_scheduler::{Job, JobScheduler};

/// Reads all active cron-triggered workflows from the DB and registers
/// a `tokio-cron-scheduler` job for each one. On each tick it creates
/// a `Pending` run which the `Scheduler` will pick up.
pub struct CronTriggers {
    pub db: Db,
    pub sched: JobScheduler,
}

impl CronTriggers {
    pub async fn new(db: Db) -> anyhow::Result<Self> {
        let sched = JobScheduler::new().await?;
        Ok(Self { db, sched })
    }

    /// Load active cron workflows from the DB and start the scheduler.
    pub async fn load_and_start(&self) -> anyhow::Result<()> {
        let workflows = self.db.list_active_cron_workflows().await?;
        for wf in workflows {
            let schedule = wf
                .trigger_config
                .get("schedule")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    anyhow::anyhow!("missing `schedule` in trigger_config for workflow {}", wf.id)
                })?
                .to_string();

            let db = self.db.clone();
            let wf_id = wf.id;
            let org_id = wf.org_id;

            let job = Job::new_async(schedule.as_str(), move |_uuid, _l| {
                let db = db.clone();
                Box::pin(async move {
                    if let Err(e) = db
                        .create_run(NewRun {
                            workflow_id: wf_id,
                            org_id,
                            triggered_by: "cron".to_string(),
                        })
                        .await
                    {
                        tracing::error!(
                            %wf_id,
                            error = %e,
                            "cron: failed to create run"
                        );
                    }
                })
            })?;

            self.sched.add(job).await?;
        }

        self.sched.start().await?;
        Ok(())
    }
}
