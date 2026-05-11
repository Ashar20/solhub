use super::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};
use async_trait::async_trait;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

pub struct OrcaPlugin;

impl OrcaPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OrcaPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolanaKeeperPlugin for OrcaPlugin {
    fn id(&self) -> &'static str {
        "orca"
    }

    fn name(&self) -> &'static str {
        "Orca"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![
            ActionDefinition {
                id: "add_liquidity".to_string(),
                name: "Add Liquidity".to_string(),
                description: "Add liquidity to an Orca Whirlpool position".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["whirlpool", "token_a_amount", "token_b_amount"],
                    "properties": {
                        "whirlpool": {"type": "string", "description": "Whirlpool account pubkey"},
                        "token_a_amount": {"type": "integer", "description": "Amount of token A"},
                        "token_b_amount": {"type": "integer", "description": "Amount of token B"},
                        "lower_tick": {"type": "integer", "description": "Lower price tick"},
                        "upper_tick": {"type": "integer", "description": "Upper price tick"}
                    }
                }),
                returns_schema: json!({"signature": "string", "position": "string", "liquidity": "integer"}),
            },
            ActionDefinition {
                id: "remove_liquidity".to_string(),
                name: "Remove Liquidity".to_string(),
                description: "Remove liquidity from an Orca Whirlpool position".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["position", "liquidity_amount"],
                    "properties": {
                        "position": {"type": "string", "description": "Position NFT mint pubkey"},
                        "liquidity_amount": {"type": "integer", "description": "Liquidity units to remove"}
                    }
                }),
                returns_schema: json!({"signature": "string", "token_a_withdrawn": "integer", "token_b_withdrawn": "integer"}),
            },
            ActionDefinition {
                id: "collect_fees".to_string(),
                name: "Collect Fees".to_string(),
                description: "Collect accrued trading fees from a Whirlpool position".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["position"],
                    "properties": {
                        "position": {"type": "string", "description": "Position NFT mint pubkey"}
                    }
                }),
                returns_schema: json!({"signature": "string", "fee_a": "integer", "fee_b": "integer"}),
            },
            ActionDefinition {
                id: "rebalance_range".to_string(),
                name: "Rebalance Range".to_string(),
                description: "Rebalance a Whirlpool position to a new price range".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["position", "new_lower_tick", "new_upper_tick"],
                    "properties": {
                        "position": {"type": "string", "description": "Position NFT mint pubkey"},
                        "new_lower_tick": {"type": "integer", "description": "New lower price tick"},
                        "new_upper_tick": {"type": "integer", "description": "New upper price tick"}
                    }
                }),
                returns_schema: json!({"signature": "string", "new_position": "string"}),
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
        let plugin = OrcaPlugin::new();
        let actions = plugin.actions();
        assert_eq!(actions.len(), 4);
        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"add_liquidity"));
        assert!(ids.contains(&"remove_liquidity"));
        assert!(ids.contains(&"collect_fees"));
        assert!(ids.contains(&"rebalance_range"));
    }
}
