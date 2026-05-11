use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    pub api_url: Option<String>,
    pub api_key: Option<String>,
    pub rpc_url: Option<String>,
}

impl CliConfig {
    pub fn path() -> Result<PathBuf> {
        let dirs = ProjectDirs::from("dev", "solhub", "skh")
            .context("could not determine config directory")?;
        let dir = dirs.config_dir();
        std::fs::create_dir_all(dir)?;
        Ok(dir.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let p = Self::path()?;
        if !p.exists() {
            return Ok(Self::default());
        }
        let s = std::fs::read_to_string(&p)?;
        Ok(toml::from_str(&s)?)
    }

    pub fn save(&self) -> Result<()> {
        let p = Self::path()?;
        std::fs::write(p, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn api_url(&self) -> String {
        self.api_url
            .clone()
            .or_else(|| std::env::var("SOLHUB_API_URL").ok())
            .unwrap_or_else(|| "http://localhost:8080".to_string())
    }

    pub fn api_key(&self) -> Option<String> {
        self.api_key
            .clone()
            .or_else(|| std::env::var("SOLHUB_API_KEY").ok())
    }
}
