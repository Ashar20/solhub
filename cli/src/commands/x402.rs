use anyhow::{anyhow, Context, Result};
use clap::Args;
use serde_json::Value;

use crate::{client::ApiClient, config::CliConfig};

/// `skh x402 pay --workflow <id> [--fee <lamports>] --keypair <path>`
///
/// Full x402 flow:
///   1. GET /v1/hub/:id/payment_info  → fetch fee requirements.
///   2. Transfer SOL from the keypair to the treasury via `solana transfer`.
///   3. POST /v1/hub/:id/call with X-PAYMENT: solana:devnet:tx:<sig>.
///   4. Print the run_id.
#[derive(Args, Debug)]
pub struct X402PayArgs {
    /// Workflow UUID to call.
    #[arg(long)]
    pub workflow: String,

    /// Path to Solana keypair JSON file (defaults to solhub-dev.json in cwd).
    #[arg(long, default_value = "solhub-dev.json")]
    pub keypair: String,

    /// Override fee in lamports (if not provided, the server's requirement is used).
    #[arg(long)]
    pub fee: Option<u64>,

    /// Seconds to wait for the tx to confirm before calling the API (default: 15).
    #[arg(long, default_value_t = 15)]
    pub confirm_wait: u64,
}

pub async fn run(args: X402PayArgs) -> Result<()> {
    let cfg = CliConfig::load()?;
    let client = ApiClient::new(&cfg);

    // 1. Fetch payment requirements.
    let path = format!("/v1/hub/{}/payment_info", args.workflow);
    let reqs: Value = client
        .get_json(&path)
        .await
        .context("Failed to fetch payment_info")?;

    let amount_lamports = args.fee.unwrap_or_else(|| {
        reqs["amount_lamports"]
            .as_u64()
            .unwrap_or(0)
    });
    let recipient = reqs["recipient"]
        .as_str()
        .ok_or_else(|| anyhow!("missing recipient in payment_info"))?
        .to_string();

    println!("Payment required: {} lamports → {}", amount_lamports, recipient);

    // 2. Send SOL via solana CLI.
    let sol_amount = format!("{:.9}", amount_lamports as f64 / 1_000_000_000.0);
    let output = std::process::Command::new("solana")
        .args([
            "transfer",
            "--keypair",
            &args.keypair,
            "--allow-unfunded-recipient",
            &recipient,
            &sol_amount,
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

    println!("TX signature: {}", signature);
    println!(
        "Waiting {}s for confirmation...",
        args.confirm_wait
    );
    tokio::time::sleep(std::time::Duration::from_secs(args.confirm_wait)).await;

    // 3. Call the hub endpoint with X-PAYMENT header.
    let call_path = format!("/v1/hub/{}/call", args.workflow);
    let payment_header = format!("solana:devnet:tx:{}", signature);

    let resp: Value = client
        .post_with_header(&call_path, &serde_json::Value::Null, "x-payment", &payment_header)
        .await
        .context("Hub call failed")?;

    // 4. Print run_id.
    let run_id = resp["run_id"]
        .as_str()
        .ok_or_else(|| anyhow!("missing run_id in response: {:?}", resp))?;

    println!("Run created: {}", run_id);
    println!("Status:      {}", resp["status"].as_str().unwrap_or("unknown"));
    println!("Payment sig: {}", resp["payment_signature"].as_str().unwrap_or(&signature));

    Ok(())
}
