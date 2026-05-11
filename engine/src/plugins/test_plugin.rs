//! `test.echo` plugin — for integration tests only.
//!
//! `echo` is a `ReadOnly` action that returns its input `params` unchanged as
//! the output. Useful for testing the executor pipeline without hitting any
//! real external service.

use async_trait::async_trait;
use serde_json::Value;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

use super::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};

pub struct EchoPlugin;

#[async_trait]
impl SolanaKeeperPlugin for EchoPlugin {
    fn id(&self) -> &'static str {
        "test.echo"
    }

    fn name(&self) -> &'static str {
        "Test Echo Plugin"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![ActionDefinition {
            id: "echo".to_string(),
            name: "Echo".to_string(),
            description: "Returns params verbatim as output. For testing only.".to_string(),
            action_type: ActionType::ReadOnly,
            params_schema: serde_json::json!({ "type": "object" }),
            returns_schema: serde_json::json!({ "type": "object" }),
        }]
    }

    async fn build_transactions(
        &self,
        action: &str,
        _params: &Value,
        _wallet_pubkey: &Pubkey,
        _rpc: &RpcClient,
    ) -> Result<Vec<VersionedTransaction>, PluginError> {
        Err(PluginError::UnknownAction(action.to_string()))
    }

    async fn read(
        &self,
        action: &str,
        params: &Value,
        _rpc: &RpcClient,
    ) -> Result<Value, PluginError> {
        match action {
            "echo" => Ok(params.clone()),
            _ => Err(PluginError::UnknownAction(action.to_string())),
        }
    }
}
