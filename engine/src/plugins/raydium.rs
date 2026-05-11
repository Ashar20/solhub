use super::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};
use async_trait::async_trait;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

pub struct RaydiumPlugin;

impl RaydiumPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RaydiumPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolanaKeeperPlugin for RaydiumPlugin {
    fn id(&self) -> &'static str {
        "raydium"
    }

    fn name(&self) -> &'static str {
        "Raydium"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![
            ActionDefinition {
                id: "swap".to_string(),
                name: "Swap".to_string(),
                description: "Token swap via Raydium AMM".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["pool", "input_mint", "output_mint", "amount"],
                    "properties": {
                        "pool": {"type": "string", "description": "Raydium AMM pool account pubkey"},
                        "input_mint": {"type": "string", "description": "Input token mint address"},
                        "output_mint": {"type": "string", "description": "Output token mint address"},
                        "amount": {"type": "integer", "description": "Amount in input token base units"},
                        "min_out": {"type": "integer", "description": "Minimum output amount (slippage guard)"}
                    }
                }),
                returns_schema: json!({"signature": "string", "output_amount": "integer"}),
            },
            ActionDefinition {
                id: "add_liquidity".to_string(),
                name: "Add Liquidity".to_string(),
                description: "Add liquidity to a Raydium AMM pool".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["pool", "base_amount", "quote_amount"],
                    "properties": {
                        "pool": {"type": "string", "description": "Raydium AMM pool account pubkey"},
                        "base_amount": {"type": "integer", "description": "Base token amount"},
                        "quote_amount": {"type": "integer", "description": "Quote token amount"},
                        "fix_side": {"type": "string", "enum": ["base", "quote"], "description": "Which side to fix"}
                    }
                }),
                returns_schema: json!({"signature": "string", "lp_tokens": "integer"}),
            },
            ActionDefinition {
                id: "harvest_yield".to_string(),
                name: "Harvest Yield".to_string(),
                description: "Harvest yield farming rewards from Raydium farm".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["farm"],
                    "properties": {
                        "farm": {"type": "string", "description": "Raydium farm account pubkey"}
                    }
                }),
                returns_schema: json!({"signature": "string", "reward_amount": "integer", "reward_mint": "string"}),
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
        let plugin = RaydiumPlugin::new();
        let actions = plugin.actions();
        assert_eq!(actions.len(), 3);
        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"swap"));
        assert!(ids.contains(&"add_liquidity"));
        assert!(ids.contains(&"harvest_yield"));
    }
}
