use super::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};
use async_trait::async_trait;
use pyth_sdk_solana::state::SolanaPriceAccount;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use std::str::FromStr;

pub struct PythPlugin;

impl PythPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PythPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolanaKeeperPlugin for PythPlugin {
    fn id(&self) -> &'static str {
        "pyth"
    }

    fn name(&self) -> &'static str {
        "Pyth"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![
            ActionDefinition {
                id: "read_price".to_string(),
                name: "Read Price".to_string(),
                description: "Read current price from a Pyth price feed account".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["feed"],
                    "properties": {
                        "feed": {
                            "type": "string",
                            "description": "Pyth price account pubkey (base58)"
                        }
                    }
                }),
                returns_schema: json!({
                    "type": "object",
                    "properties": {
                        "price": {"type": "number"},
                        "conf": {"type": "number"},
                        "expo": {"type": "integer"},
                        "publish_time": {"type": "integer"}
                    }
                }),
            },
            ActionDefinition {
                id: "staleness_check".to_string(),
                name: "Staleness Check".to_string(),
                description: "Check if a Pyth price feed is stale".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["feed", "max_age_seconds"],
                    "properties": {
                        "feed": {
                            "type": "string",
                            "description": "Pyth price account pubkey (base58)"
                        },
                        "max_age_seconds": {
                            "type": "integer",
                            "description": "Maximum acceptable age of price in seconds"
                        }
                    }
                }),
                returns_schema: json!({
                    "type": "object",
                    "properties": {
                        "stale": {"type": "boolean"},
                        "age_seconds": {"type": "integer"}
                    }
                }),
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
        Err(PluginError::NotSupported)
    }

    async fn read(
        &self,
        action: &str,
        params: &Value,
        rpc: &RpcClient,
    ) -> Result<Value, PluginError> {
        match action {
            "read_price" => {
                let feed_str = params["feed"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("feed".into()))?;
                let feed_pubkey = Pubkey::from_str(feed_str)
                    .map_err(|e| PluginError::InvalidParam(format!("feed: {e}")))?;

                let mut account = rpc
                    .get_account(&feed_pubkey)
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;

                let price_feed =
                    SolanaPriceAccount::account_to_feed(&feed_pubkey, &mut account)
                        .map_err(|e| PluginError::Other(format!("pyth parse: {e}")))?;

                let price = price_feed.get_price_unchecked();

                Ok(json!({
                    "price": price.price,
                    "conf": price.conf,
                    "expo": price.expo,
                    "publish_time": price.publish_time
                }))
            }
            "staleness_check" => {
                let feed_str = params["feed"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("feed".into()))?;
                let max_age = params["max_age_seconds"]
                    .as_u64()
                    .ok_or_else(|| PluginError::InvalidParam("max_age_seconds".into()))?;
                let feed_pubkey = Pubkey::from_str(feed_str)
                    .map_err(|e| PluginError::InvalidParam(format!("feed: {e}")))?;

                let mut account = rpc
                    .get_account(&feed_pubkey)
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;

                let price_feed =
                    SolanaPriceAccount::account_to_feed(&feed_pubkey, &mut account)
                        .map_err(|e| PluginError::Other(format!("pyth parse: {e}")))?;

                let price = price_feed.get_price_unchecked();
                let now = chrono::Utc::now().timestamp();
                let age_seconds = (now - price.publish_time).max(0) as u64;
                let stale = age_seconds > max_age;

                Ok(json!({
                    "stale": stale,
                    "age_seconds": age_seconds
                }))
            }
            _ => Err(PluginError::UnknownAction(action.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_returns_read_price_and_staleness_check_schemas() {
        let plugin = PythPlugin::new();
        let actions = plugin.actions();
        assert_eq!(actions.len(), 2);

        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"read_price"));
        assert!(ids.contains(&"staleness_check"));

        let read_price = actions.iter().find(|a| a.id == "read_price").unwrap();
        assert_eq!(read_price.action_type, ActionType::ReadOnly);
        assert!(read_price.params_schema["properties"].get("feed").is_some());

        let staleness = actions.iter().find(|a| a.id == "staleness_check").unwrap();
        assert_eq!(staleness.action_type, ActionType::ReadOnly);
        assert!(staleness.params_schema["properties"]
            .get("max_age_seconds")
            .is_some());
    }

    #[tokio::test]
    #[ignore = "requires live Solana RPC"]
    async fn read_price_live() {
        let plugin = PythPlugin::new();
        let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        // SOL/USD feed
        let params = serde_json::json!({
            "feed": "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG"
        });
        let result = plugin.read("read_price", &params, &rpc).await;
        assert!(result.is_ok(), "read_price failed: {:?}", result);
    }
}
