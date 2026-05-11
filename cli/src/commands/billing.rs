use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::Value;

use crate::client::ApiClient;
use crate::config::CliConfig;

#[derive(Args)]
pub struct BillingCmd {
    #[command(subcommand)]
    cmd: BillingSub,
}

#[derive(Subcommand)]
enum BillingSub {
    /// Show current credit balance
    Status,
    /// Deposit USDC credits (prints onchain vault address)
    Deposit {
        /// Amount of USDC to deposit
        #[arg(long)]
        amount: f64,
    },
}

pub async fn run(c: BillingCmd) -> Result<()> {
    let cfg = CliConfig::load()?;
    let client = ApiClient::new(&cfg);

    match c.cmd {
        BillingSub::Status => status(&client).await,
        BillingSub::Deposit { amount } => deposit(amount).await,
    }
}

async fn status(client: &ApiClient) -> Result<()> {
    let org: Value = client.get_json("/v1/orgs/me").await?;
    let credits = org
        .get("credits")
        .cloned()
        .unwrap_or(Value::Null);
    println!("Credits: {credits}");
    Ok(())
}

async fn deposit(amount: f64) -> Result<()> {
    // The actual onchain deposit is an Anchor program call and is out of scope for the CLI.
    // The vault PDA is derived from the organization's public key via the execution-vault program.
    println!(
        "To deposit {amount} USDC, send USDC to the onchain execution vault PDA for your org."
    );
    println!("The vault address is derived by the execution-vault Anchor program from your org ID.");
    println!("Onchain deposit via CLI is not yet implemented — use the web dashboard instead.");
    Ok(())
}
