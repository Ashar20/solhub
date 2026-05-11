use anyhow::Result;
use clap::{Args, Subcommand};

use crate::config::CliConfig;

#[derive(Args)]
pub struct AuthCmd {
    #[command(subcommand)]
    cmd: AuthSub,
}

#[derive(Subcommand)]
enum AuthSub {
    /// Prompt for API key and save to config
    Login,
    /// Show current auth info
    Status,
}

pub async fn run(c: AuthCmd) -> Result<()> {
    match c.cmd {
        AuthSub::Login => login().await,
        AuthSub::Status => status().await,
    }
}

async fn login() -> Result<()> {
    use std::io::{self, BufRead, Write};
    print!("API URL [http://localhost:8080]: ");
    io::stdout().flush()?;
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    let api_url = line.trim().to_string();
    line.clear();

    print!("API Key: ");
    io::stdout().flush()?;
    stdin.lock().read_line(&mut line)?;
    let api_key = line.trim().to_string();

    let mut cfg = CliConfig::load()?;
    if !api_url.is_empty() {
        cfg.api_url = Some(api_url);
    }
    if !api_key.is_empty() {
        cfg.api_key = Some(api_key);
    }
    cfg.save()?;
    println!("Saved.");
    Ok(())
}

async fn status() -> Result<()> {
    let cfg = CliConfig::load()?;
    println!("API URL: {}", cfg.api_url());
    let key_display = cfg
        .api_key()
        .map(|k| {
            let end = k.len().min(8);
            format!("{}…", &k[..end])
        })
        .unwrap_or_else(|| "<not set>".to_string());
    println!("API Key: {}", key_display);
    Ok(())
}
