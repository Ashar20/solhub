use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};

use crate::config::CliConfig;

#[derive(Args)]
pub struct ConfigCmd {
    #[command(subcommand)]
    cmd: ConfigSub,
}

#[derive(Subcommand)]
enum ConfigSub {
    /// Set a config value (keys: api_url, api_key, rpc_url)
    Set {
        /// Config key
        key: String,
        /// Config value
        value: String,
    },
    /// List current config
    List,
}

pub async fn run(c: ConfigCmd) -> Result<()> {
    match c.cmd {
        ConfigSub::Set { key, value } => set(&key, &value).await,
        ConfigSub::List => list().await,
    }
}

async fn set(key: &str, value: &str) -> Result<()> {
    let mut cfg = CliConfig::load()?;
    match key {
        "api_url" => cfg.api_url = Some(value.to_string()),
        "api_key" => cfg.api_key = Some(value.to_string()),
        "rpc_url" => cfg.rpc_url = Some(value.to_string()),
        other => return Err(anyhow!("unknown config key: {other}. Valid keys: api_url, api_key, rpc_url")),
    }
    cfg.save()?;
    println!("Set {key} = {value}");
    Ok(())
}

async fn list() -> Result<()> {
    let cfg = CliConfig::load()?;
    let toml = toml::to_string_pretty(&cfg)?;
    if toml.trim().is_empty() {
        println!("# No config set.");
    } else {
        print!("{toml}");
    }
    Ok(())
}
