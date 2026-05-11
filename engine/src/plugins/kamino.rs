use super::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};
use async_trait::async_trait;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

pub struct KaminoPlugin;

impl KaminoPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KaminoPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolanaKeeperPlugin for KaminoPlugin {
    fn id(&self) -> &'static str {
        "kamino"
    }

    fn name(&self) -> &'static str {
        "Kamino"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![
            ActionDefinition {
                id: "deposit".to_string(),
                name: "Deposit".to_string(),
                description: "Deposit assets into a Kamino reserve".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["reserve", "amount"],
                    "properties": {
                        "reserve": {"type": "string", "description": "Reserve account pubkey"},
                        "amount": {"type": "integer", "description": "Amount in base units"}
                    }
                }),
                returns_schema: json!({"signature": "string"}),
            },
            ActionDefinition {
                id: "withdraw".to_string(),
                name: "Withdraw".to_string(),
                description: "Withdraw assets from a Kamino reserve".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["reserve", "amount"],
                    "properties": {
                        "reserve": {"type": "string", "description": "Reserve account pubkey"},
                        "amount": {"type": "integer", "description": "Amount in base units"}
                    }
                }),
                returns_schema: json!({"signature": "string"}),
            },
            ActionDefinition {
                id: "claim_rewards".to_string(),
                name: "Claim Rewards".to_string(),
                description: "Claim accrued Kamino rewards".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["reserve"],
                    "properties": {
                        "reserve": {"type": "string", "description": "Reserve account pubkey"}
                    }
                }),
                returns_schema: json!({"signature": "string", "rewards_claimed": "integer"}),
            },
            ActionDefinition {
                id: "check_ltv".to_string(),
                name: "Check LTV".to_string(),
                description: "Read current loan-to-value ratio for a position".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["obligation"],
                    "properties": {
                        "obligation": {"type": "string", "description": "Obligation account pubkey"}
                    }
                }),
                returns_schema: json!({"ltv": "number", "max_ltv": "number", "liquidation_ltv": "number"}),
            },
            ActionDefinition {
                id: "check_rewards".to_string(),
                name: "Check Rewards".to_string(),
                description: "Read pending rewards for a position".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["reserve"],
                    "properties": {
                        "reserve": {"type": "string", "description": "Reserve account pubkey"}
                    }
                }),
                returns_schema: json!({"pending_rewards": "integer", "reward_mint": "string"}),
            },
        ]
    }

    async fn build_transactions(
        &self,
        _action: &str,
        _params: &Value,
        _wallet_pubkey: &Pubkey,
        _rpc: &RpcClient,
    ) -> Result<Vec<VersionedTransaction>, PluginError> {
        Err(PluginError::NotImplemented)
    }

    async fn read(
        &self,
        _action: &str,
        _params: &Value,
        _rpc: &RpcClient,
    ) -> Result<Value, PluginError> {
        Err(PluginError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_returns_expected_count() {
        let plugin = KaminoPlugin::new();
        let actions = plugin.actions();
        assert_eq!(actions.len(), 5);
        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"deposit"));
        assert!(ids.contains(&"withdraw"));
        assert!(ids.contains(&"claim_rewards"));
        assert!(ids.contains(&"check_ltv"));
        assert!(ids.contains(&"check_rewards"));
    }
}
