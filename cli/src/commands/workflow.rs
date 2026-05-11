use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::ApiClient;
use crate::config::CliConfig;

#[derive(Args)]
pub struct WorkflowCmd {
    #[command(subcommand)]
    cmd: WorkflowSub,
}

#[derive(Subcommand)]
enum WorkflowSub {
    /// List all workflows
    List,
    /// Create a workflow from a JSON file
    Create {
        /// Path to the workflow JSON file
        #[arg(short, long)]
        file: String,
    },
    /// Create a workflow and register it onchain
    Deploy {
        /// Path to the workflow JSON file
        #[arg(short, long)]
        file: String,
    },
    /// Enable a workflow
    Enable {
        /// Workflow ID
        id: String,
    },
    /// Disable a workflow
    Disable {
        /// Workflow ID
        id: String,
    },
    /// Delete a workflow
    Delete {
        /// Workflow ID
        id: String,
    },
}

#[derive(Debug, Deserialize)]
struct Workflow {
    id: String,
    name: String,
    trigger_type: String,
    is_active: bool,
}

#[derive(Serialize)]
struct PatchActive {
    is_active: bool,
}

pub async fn run(c: WorkflowCmd) -> Result<()> {
    let cfg = CliConfig::load()?;
    let client = ApiClient::new(&cfg);

    match c.cmd {
        WorkflowSub::List => list(&client).await,
        WorkflowSub::Create { file } => create(&client, &file).await,
        WorkflowSub::Deploy { file } => deploy(&client, &file).await,
        WorkflowSub::Enable { id } => set_active(&client, &id, true).await,
        WorkflowSub::Disable { id } => set_active(&client, &id, false).await,
        WorkflowSub::Delete { id } => delete(&client, &id).await,
    }
}

async fn list(client: &ApiClient) -> Result<()> {
    let workflows: Vec<Workflow> = client.get_json("/v1/workflows").await?;
    if workflows.is_empty() {
        println!("No workflows found.");
        return Ok(());
    }
    println!("{:<36}  {:<30}  {:<15}  ACTIVE", "ID", "NAME", "TRIGGER");
    println!("{}", "-".repeat(90));
    for w in &workflows {
        println!(
            "{:<36}  {:<30}  {:<15}  {}",
            w.id, w.name, w.trigger_type, w.is_active
        );
    }
    Ok(())
}

async fn create(client: &ApiClient, file: &str) -> Result<()> {
    let content =
        std::fs::read_to_string(file).with_context(|| format!("reading file: {file}"))?;
    let body: Value = serde_json::from_str(&content)
        .with_context(|| format!("parsing JSON from: {file}"))?;
    let resp: Value = client.post_json("/v1/workflows", &body).await?;
    let id = resp
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("<unknown>");
    println!("Created workflow: {id}");
    Ok(())
}

async fn deploy(client: &ApiClient, file: &str) -> Result<()> {
    let content =
        std::fs::read_to_string(file).with_context(|| format!("reading file: {file}"))?;
    let body: Value = serde_json::from_str(&content)
        .with_context(|| format!("parsing JSON from: {file}"))?;
    let resp: Value = client.post_json("/v1/workflows", &body).await?;
    let id = resp
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("<unknown>");
    println!("Created workflow: {id}");
    println!(
        "Note: onchain registration (POST /v1/workflows/{id}/deploy) is not yet implemented."
    );
    Ok(())
}

async fn set_active(client: &ApiClient, id: &str, active: bool) -> Result<()> {
    let body = PatchActive { is_active: active };
    let _: Value = client
        .patch_json(&format!("/v1/workflows/{id}"), &body)
        .await?;
    let state = if active { "enabled" } else { "disabled" };
    println!("Workflow {id} {state}.");
    Ok(())
}

async fn delete(client: &ApiClient, id: &str) -> Result<()> {
    client.delete(&format!("/v1/workflows/{id}")).await?;
    println!("Workflow {id} deleted.");
    Ok(())
}
