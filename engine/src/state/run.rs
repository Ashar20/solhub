use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSource {
    Cron,
    AccountWatch,
    Webhook,
    Manual,
    Mcp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RunStatus {
    Pending,
    Triggered,
    Simulating,
    Bundling,
    Submitted,
    WaitingApproval,
    Confirmed,
    Retrying,
    Failed,
    Skipped,
    /// Transient state set by the approve endpoint; treated like Pending by the executor.
    Resumed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub run_id: Uuid,
    pub workflow_id: Uuid,
    pub triggered_by: TriggerSource,
    pub status: RunStatus,
    pub steps: Vec<StepLog>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub slot: Option<u64>,
    pub signature: Option<String>,
    pub fee_lamports: Option<u64>,
    pub jito_tip_lamports: Option<u64>,
    pub error: Option<String>,
}

impl WorkflowRun {
    pub fn new(workflow_id: Uuid, triggered_by: TriggerSource) -> Self {
        Self {
            run_id: Uuid::new_v4(),
            workflow_id,
            triggered_by,
            status: RunStatus::Pending,
            steps: Vec::new(),
            started_at: chrono::Utc::now(),
            completed_at: None,
            slot: None,
            signature: None,
            fee_lamports: None,
            jito_tip_lamports: None,
            error: None,
        }
    }

    pub fn transition_to(&mut self, new: RunStatus) -> Result<(), TransitionError> {
        use RunStatus::*;
        let allowed: &[RunStatus] = match self.status {
            Pending => &[Triggered, WaitingApproval, Failed],
            Triggered => &[Simulating, WaitingApproval, Failed, Skipped],
            Simulating => &[Bundling, WaitingApproval, Failed],
            Bundling => &[Submitted, WaitingApproval, Failed],
            Submitted => &[Confirmed, WaitingApproval, Retrying, Failed],
            WaitingApproval => &[Resumed, Failed, Skipped],
            Retrying => &[Submitted, Failed],
            Resumed => &[Triggered, Failed],
            Confirmed => &[],
            Failed => &[],
            Skipped => &[],
        };
        if !allowed.contains(&new) {
            return Err(TransitionError::Illegal {
                from: self.status.clone(),
                to: new,
            });
        }
        self.status = new.clone();
        if matches!(new, Confirmed | Failed | Skipped) {
            self.completed_at = Some(chrono::Utc::now());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepLog {
    pub step_id: String,
    pub status: StepStatus,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum TransitionError {
    #[error("illegal transition from {from:?} to {to:?}")]
    Illegal { from: RunStatus, to: RunStatus },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_run() -> WorkflowRun {
        WorkflowRun::new(Uuid::new_v4(), TriggerSource::Manual)
    }

    #[test]
    fn pending_to_triggered_ok() {
        let mut run = make_run();
        assert_eq!(run.status, RunStatus::Pending);
        run.transition_to(RunStatus::Triggered).expect("should succeed");
        assert_eq!(run.status, RunStatus::Triggered);
    }

    #[test]
    fn pending_to_confirmed_illegal() {
        let mut run = make_run();
        let result = run.transition_to(RunStatus::Confirmed);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Pending"));
        assert!(err.to_string().contains("Confirmed"));
    }

    #[test]
    fn confirmed_is_terminal() {
        let mut run = make_run();
        // Navigate to Confirmed
        run.transition_to(RunStatus::Triggered).unwrap();
        run.transition_to(RunStatus::Simulating).unwrap();
        run.transition_to(RunStatus::Bundling).unwrap();
        run.transition_to(RunStatus::Submitted).unwrap();
        run.transition_to(RunStatus::Confirmed).unwrap();
        assert_eq!(run.status, RunStatus::Confirmed);

        // Any further transition should fail
        let result = run.transition_to(RunStatus::Failed);
        assert!(result.is_err());
        let result2 = run.transition_to(RunStatus::Pending);
        assert!(result2.is_err());
    }

    #[test]
    fn submitted_to_retrying_to_submitted_ok() {
        let mut run = make_run();
        run.transition_to(RunStatus::Triggered).unwrap();
        run.transition_to(RunStatus::Simulating).unwrap();
        run.transition_to(RunStatus::Bundling).unwrap();
        run.transition_to(RunStatus::Submitted).unwrap();
        run.transition_to(RunStatus::Retrying).unwrap();
        run.transition_to(RunStatus::Submitted).unwrap();
        assert_eq!(run.status, RunStatus::Submitted);
    }

    #[test]
    fn transition_to_confirmed_sets_completed_at() {
        let mut run = make_run();
        assert!(run.completed_at.is_none());
        run.transition_to(RunStatus::Triggered).unwrap();
        run.transition_to(RunStatus::Simulating).unwrap();
        run.transition_to(RunStatus::Bundling).unwrap();
        run.transition_to(RunStatus::Submitted).unwrap();
        run.transition_to(RunStatus::Confirmed).unwrap();
        assert!(run.completed_at.is_some());
    }

    #[test]
    fn pending_to_waiting_approval_ok() {
        let mut run = make_run();
        run.transition_to(RunStatus::WaitingApproval).unwrap();
        assert_eq!(run.status, RunStatus::WaitingApproval);
        // WaitingApproval is not terminal — completed_at must remain None.
        assert!(run.completed_at.is_none());
    }

    #[test]
    fn waiting_approval_to_resumed_ok() {
        let mut run = make_run();
        run.transition_to(RunStatus::WaitingApproval).unwrap();
        run.transition_to(RunStatus::Resumed).unwrap();
        assert_eq!(run.status, RunStatus::Resumed);
    }

    #[test]
    fn waiting_approval_to_failed_ok() {
        let mut run = make_run();
        run.transition_to(RunStatus::WaitingApproval).unwrap();
        run.transition_to(RunStatus::Failed).unwrap();
        assert_eq!(run.status, RunStatus::Failed);
        assert!(run.completed_at.is_some());
    }

    #[test]
    fn resumed_to_triggered_ok() {
        let mut run = make_run();
        run.transition_to(RunStatus::WaitingApproval).unwrap();
        run.transition_to(RunStatus::Resumed).unwrap();
        run.transition_to(RunStatus::Triggered).unwrap();
        assert_eq!(run.status, RunStatus::Triggered);
    }
}
