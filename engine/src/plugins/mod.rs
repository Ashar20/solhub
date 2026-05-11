use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

pub mod system;
pub mod jupiter;
pub mod pyth;
pub mod kamino;
pub mod marinade;
pub mod drift;
pub mod orca;
pub mod raydium;
pub mod notifications;
pub mod test_plugin;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    ReadOnly,
    Transaction,
    Notification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub action_type: ActionType,
    pub params_schema: Value,
    pub returns_schema: Value,
}

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("invalid param: {0}")]
    InvalidParam(String),
    #[error("unknown action: {0}")]
    UnknownAction(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("not implemented")]
    NotImplemented,
    #[error("not supported")]
    NotSupported,
    #[error("other: {0}")]
    Other(String),
}

#[async_trait]
pub trait SolanaKeeperPlugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn actions(&self) -> Vec<ActionDefinition>;

    async fn build_transactions(
        &self,
        action: &str,
        params: &Value,
        wallet_pubkey: &Pubkey,
        rpc: &RpcClient,
    ) -> Result<Vec<VersionedTransaction>, PluginError>;

    async fn read(
        &self,
        action: &str,
        params: &Value,
        rpc: &RpcClient,
    ) -> Result<Value, PluginError>;

    async fn notify(
        &self,
        action: &str,
        params: &Value,
    ) -> Result<Value, PluginError> {
        let _ = (action, params);
        Err(PluginError::NotSupported)
    }
}

pub struct PluginRegistry {
    plugins: HashMap<String, Arc<dyn SolanaKeeperPlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    pub fn register(&mut self, plugin: Arc<dyn SolanaKeeperPlugin>) {
        self.plugins.insert(plugin.id().to_string(), plugin);
    }

    pub fn get(&self, id: &str) -> Option<&Arc<dyn SolanaKeeperPlugin>> {
        self.plugins.get(id)
    }

    pub fn all_actions(&self) -> Vec<(String, Vec<ActionDefinition>)> {
        self.plugins
            .iter()
            .map(|(id, p)| (id.clone(), p.actions()))
            .collect()
    }

    pub fn with_default_plugins() -> Self {
        let mut reg = Self::new();
        reg.register(Arc::new(system::SystemPlugin::new()));
        reg.register(Arc::new(jupiter::JupiterPlugin::new()));
        reg.register(Arc::new(pyth::PythPlugin::new()));
        reg.register(Arc::new(kamino::KaminoPlugin::new()));
        reg.register(Arc::new(marinade::MarinadePlugin::new()));
        reg.register(Arc::new(drift::DriftPlugin::new()));
        reg.register(Arc::new(orca::OrcaPlugin::new()));
        reg.register(Arc::new(raydium::RaydiumPlugin::new()));
        reg.register(Arc::new(
            notifications::telegram::TelegramPlugin::new(),
        ));
        reg.register(Arc::new(notifications::discord::DiscordPlugin::new()));
        reg
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::with_default_plugins()
    }
}
