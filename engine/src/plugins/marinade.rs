use super::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};
use async_trait::async_trait;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

pub struct MarinadePlugin;

impl MarinadePlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MarinadePlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolanaKeeperPlugin for MarinadePlugin {
    fn id(&self) -> &'static str {
        "marinade"
    }

    fn name(&self) -> &'static str {
        "Marinade"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![
            ActionDefinition {
                id: "stake".to_string(),
                name: "Stake SOL".to_string(),
                description: "Stake SOL via Marinade native staking".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["amount"],
                    "properties": {
                        "amount": {"type": "integer", "description": "Amount in lamports"}
                    }
                }),
                returns_schema: json!({"signature": "string"}),
            },
            ActionDefinition {
                id: "unstake".to_string(),
                name: "Unstake SOL".to_string(),
                description: "Begin delayed unstake of SOL from Marinade".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["msol_amount"],
                    "properties": {
                        "msol_amount": {"type": "integer", "description": "Amount of mSOL in base units"}
                    }
                }),
                returns_schema: json!({"signature": "string", "ticket": "string"}),
            },
            ActionDefinition {
                id: "liquid_stake".to_string(),
                name: "Liquid Stake SOL".to_string(),
                description: "Stake SOL and receive mSOL immediately".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["amount"],
                    "properties": {
                        "amount": {"type": "integer", "description": "Amount in lamports"}
                    }
                }),
                returns_schema: json!({"signature": "string", "msol_received": "integer"}),
            },
            ActionDefinition {
                id: "check_rewards".to_string(),
                name: "Check Staking Rewards".to_string(),
                description: "Read pending staking rewards for a position".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["stake_account"],
                    "properties": {
                        "stake_account": {"type": "string", "description": "Stake account pubkey"}
                    }
                }),
                returns_schema: json!({"pending_rewards_lamports": "integer", "apy": "number"}),
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
        let plugin = MarinadePlugin::new();
        let actions = plugin.actions();
        assert_eq!(actions.len(), 4);
        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"stake"));
        assert!(ids.contains(&"unstake"));
        assert!(ids.contains(&"liquid_stake"));
        assert!(ids.contains(&"check_rewards"));
    }
}
