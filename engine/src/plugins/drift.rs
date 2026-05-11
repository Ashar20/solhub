use super::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};
use async_trait::async_trait;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

pub struct DriftPlugin;

impl DriftPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DriftPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolanaKeeperPlugin for DriftPlugin {
    fn id(&self) -> &'static str {
        "drift"
    }

    fn name(&self) -> &'static str {
        "Drift"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![
            ActionDefinition {
                id: "open_position".to_string(),
                name: "Open Position".to_string(),
                description: "Open a perpetual position on Drift Protocol".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["market_index", "direction", "base_asset_amount"],
                    "properties": {
                        "market_index": {"type": "integer", "description": "Drift market index"},
                        "direction": {"type": "string", "enum": ["long", "short"]},
                        "base_asset_amount": {"type": "integer", "description": "Base asset amount"},
                        "price": {"type": "integer", "description": "Limit price (0 for market order)"}
                    }
                }),
                returns_schema: json!({"signature": "string", "position_id": "string"}),
            },
            ActionDefinition {
                id: "close_position".to_string(),
                name: "Close Position".to_string(),
                description: "Close an open perpetual position on Drift".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["market_index"],
                    "properties": {
                        "market_index": {"type": "integer", "description": "Drift market index"},
                        "price": {"type": "integer", "description": "Limit price (0 for market order)"}
                    }
                }),
                returns_schema: json!({"signature": "string", "pnl": "integer"}),
            },
            ActionDefinition {
                id: "check_margin".to_string(),
                name: "Check Margin".to_string(),
                description: "Read current margin ratio for a Drift sub-account".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["sub_account_id"],
                    "properties": {
                        "sub_account_id": {"type": "integer", "description": "Drift sub-account id"}
                    }
                }),
                returns_schema: json!({"margin_ratio": "number", "free_collateral": "integer", "total_collateral": "integer"}),
            },
            ActionDefinition {
                id: "liquidation_guard".to_string(),
                name: "Liquidation Guard".to_string(),
                description: "Check liquidation risk and return warning if near threshold".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["sub_account_id"],
                    "properties": {
                        "sub_account_id": {"type": "integer", "description": "Drift sub-account id"},
                        "warning_threshold": {"type": "number", "description": "Margin ratio below which to warn", "default": 0.1}
                    }
                }),
                returns_schema: json!({"at_risk": "boolean", "margin_ratio": "number", "distance_to_liquidation": "number"}),
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
        let plugin = DriftPlugin::new();
        let actions = plugin.actions();
        assert_eq!(actions.len(), 4);
        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"open_position"));
        assert!(ids.contains(&"close_position"));
        assert!(ids.contains(&"check_margin"));
        assert!(ids.contains(&"liquidation_guard"));
    }
}
