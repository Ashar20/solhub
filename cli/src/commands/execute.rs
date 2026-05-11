use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::Value;

use crate::client::ApiClient;
use crate::config::CliConfig;

#[derive(Args)]
pub struct ExecuteCmd {
    #[command(subcommand)]
    cmd: ExecuteSub,
}

#[derive(Subcommand)]
enum ExecuteSub {
    /// Send a token transfer (SOL or USDC)
    Transfer {
        /// Destination public key
        #[arg(long)]
        to: String,
        /// Amount to send
        #[arg(long)]
        amount: f64,
        /// Token to use (SOL or USDC)
        #[arg(long)]
        token: String,
    },
    /// Call an onchain program with raw instruction data
    ContractCall {
        /// Program public key
        #[arg(long)]
        program: String,
        /// Base64-encoded instruction data
        #[arg(long)]
        instruction: String,
    },
}

pub async fn run(c: ExecuteCmd) -> Result<()> {
    let cfg = CliConfig::load()?;
    let client = ApiClient::new(&cfg);

    match c.cmd {
        ExecuteSub::Transfer { to, amount, token } => {
            transfer(&client, &to, amount, &token).await
        }
        ExecuteSub::ContractCall {
            program,
            instruction,
        } => contract_call(&client, &program, &instruction).await,
    }
}

async fn transfer(client: &ApiClient, to: &str, amount: f64, token: &str) -> Result<()> {
    let body = serde_json::json!({
        "to": to,
        "amount": amount,
        "token": token,
    });
    let resp: Value = client.post_json("/v1/execute/transfer", &body).await?;
    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}

async fn contract_call(client: &ApiClient, program: &str, instruction: &str) -> Result<()> {
    let body = serde_json::json!({
        "program_id": program,
        "instruction_data": instruction,
    });
    let resp: Value = client.post_json("/v1/execute/program", &body).await?;
    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}
