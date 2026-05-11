use anyhow::{anyhow, Context, Result};
use clap::{Args, Subcommand};
use serde_json::Value;

use crate::{client::ApiClient, config::CliConfig};

#[derive(Args)]
pub struct BillingCmd {
    #[command(subcommand)]
    cmd: BillingSub,
}

#[derive(Subcommand)]
enum BillingSub {
    /// Show current credit balance and recent ledger
    Balance,
    /// Top up credits by sending SOL on devnet via x402
    Topup {
        /// Amount of SOL to send (e.g. 0.001)
        #[arg(long)]
        amount: f64,

        /// Path to Solana keypair JSON file (defaults to solhub-dev.json in cwd)
        #[arg(long, default_value = "solhub-dev.json")]
        keypair: String,

        /// Seconds to wait for the tx to confirm before calling the API (default: 15)
        #[arg(long, default_value_t = 15)]
        confirm_wait: u64,
    },
}

pub async fn run(c: BillingCmd) -> Result<()> {
    let cfg = CliConfig::load()?;
    let client = ApiClient::new(&cfg);

    match c.cmd {
        BillingSub::Balance => balance(&client).await,
        BillingSub::Topup {
            amount,
            keypair,
            confirm_wait,
        } => topup(&client, amount, &keypair, confirm_wait).await,
    }
}

async fn balance(client: &ApiClient) -> Result<()> {
    let resp: Value = client
        .get_json("/v1/orgs/me/credits")
        .await
        .context("Failed to fetch credit balance")?;

    let bal = resp["balance"].as_i64().unwrap_or(0);
    println!("Credit balance: {bal}");

    if let Some(ledger) = resp["recent_ledger"].as_array() {
        if ledger.is_empty() {
            println!("No recent ledger entries.");
        } else {
            println!("\nRecent ledger (newest first):");
            for entry in ledger {
                let delta = entry["delta"].as_i64().unwrap_or(0);
                let reason = entry["reason"].as_str().unwrap_or("?");
                let bal_after = entry["balance_after"].as_i64().unwrap_or(0);
                let sign = if delta >= 0 { "+" } else { "" };
                println!("  {sign}{delta:>5}  reason={reason:<20} balance_after={bal_after}");
            }
        }
    }

    Ok(())
}

async fn topup(client: &ApiClient, amount: f64, keypair: &str, confirm_wait: u64) -> Result<()> {
    // 1. Fetch topup info to get recipient and lamports_per_credit.
    let info: Value = client
        .get_json("/v1/orgs/me/credits/topup_info")
        .await
        .context("Failed to fetch topup_info")?;

    let recipient = info["recipient"]
        .as_str()
        .ok_or_else(|| anyhow!("missing recipient in topup_info"))?
        .to_string();

    let lpc = info["lamports_per_credit"].as_u64().unwrap_or(10_000);
    let lamports = (amount * 1_000_000_000.0) as u64;
    let credits_expected = lamports / lpc;

    println!("Sending {amount} SOL ({lamports} lamports) → {recipient}");
    println!("Expected credits: ~{credits_expected}");

    // 2. Send SOL via solana CLI.
    let sol_str = format!("{:.9}", amount);
    let output = std::process::Command::new("solana")
        .args([
            "transfer",
            "--keypair",
            keypair,
            "--allow-unfunded-recipient",
            &recipient,
            &sol_str,
        ])
        .output()
        .context("Failed to run `solana transfer` — is the solana CLI installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("solana transfer failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let signature = stdout
        .lines()
        .find(|l| l.contains("Signature:"))
        .and_then(|l| l.split_whitespace().last())
        .ok_or_else(|| anyhow!("Could not parse signature from solana output:\n{}", stdout))?
        .to_string();

    println!("TX signature: {signature}");
    println!("Waiting {confirm_wait}s for confirmation...");
    tokio::time::sleep(std::time::Duration::from_secs(confirm_wait)).await;

    // 3. POST topup with X-PAYMENT header.
    let payment_header = format!("solana:devnet:tx:{signature}");
    let resp: Value = client
        .post_with_header(
            "/v1/orgs/me/credits/topup",
            &serde_json::Value::Null,
            "x-payment",
            &payment_header,
        )
        .await
        .context("Topup API call failed")?;

    let granted = resp["credits_granted"].as_i64().unwrap_or(0);
    let new_bal = resp["new_balance"].as_i64().unwrap_or(0);

    println!("Credits granted: {granted}");
    println!("New balance:     {new_bal}");
    println!("Payment ID:      {}", resp["payment_id"].as_str().unwrap_or("?"));

    Ok(())
}
