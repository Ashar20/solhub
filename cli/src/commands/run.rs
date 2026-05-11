use anyhow::Result;
use clap::{Args, Subcommand};
use serde::Deserialize;
use serde_json::Value;

use crate::client::ApiClient;
use crate::config::CliConfig;

#[derive(Args)]
pub struct RunCmd {
    #[command(subcommand)]
    cmd: RunSub,
}

#[derive(Subcommand)]
enum RunSub {
    /// List recent runs for a workflow
    List {
        /// Workflow ID to filter by
        #[arg(long)]
        workflow: String,
    },
    /// Get the status of a run
    Status {
        /// Run ID
        run_id: String,
    },
    /// Stream logs for a run
    Logs {
        /// Run ID
        run_id: String,
        /// Follow live logs (tail -f style)
        #[arg(long)]
        follow: bool,
    },
    /// Cancel a run
    Cancel {
        /// Run ID
        run_id: String,
    },
}

#[derive(Debug, Deserialize)]
struct Run {
    id: String,
    workflow_id: String,
    status: String,
}

pub async fn run(c: RunCmd) -> Result<()> {
    let cfg = CliConfig::load()?;
    let client = ApiClient::new(&cfg);

    match c.cmd {
        RunSub::List { workflow } => list(&client, &workflow).await,
        RunSub::Status { run_id } => status(&client, &run_id).await,
        RunSub::Logs { run_id, follow } => logs(&client, &run_id, follow).await,
        RunSub::Cancel { run_id } => cancel(&client, &run_id).await,
    }
}

async fn list(client: &ApiClient, workflow_id: &str) -> Result<()> {
    let runs: Vec<Run> = client
        .get_json(&format!("/v1/runs?workflow_id={workflow_id}"))
        .await?;
    if runs.is_empty() {
        println!("No runs found.");
        return Ok(());
    }
    println!("{:<36}  {:<36}  STATUS", "RUN ID", "WORKFLOW ID");
    println!("{}", "-".repeat(85));
    for r in &runs {
        println!("{:<36}  {:<36}  {}", r.id, r.workflow_id, r.status);
    }
    Ok(())
}

async fn status(client: &ApiClient, run_id: &str) -> Result<()> {
    let run: Value = client.get_json(&format!("/v1/runs/{run_id}")).await?;
    println!("{}", serde_json::to_string_pretty(&run)?);
    Ok(())
}

async fn logs(client: &ApiClient, run_id: &str, follow: bool) -> Result<()> {
    client
        .stream_sse(&format!("/v1/runs/{run_id}/logs"), follow)
        .await
}

async fn cancel(client: &ApiClient, run_id: &str) -> Result<()> {
    let _: Value = client
        .post_json(&format!("/v1/runs/{run_id}/cancel"), &serde_json::json!({}))
        .await?;
    println!("Run {run_id} cancelled.");
    Ok(())
}
