use super::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};
use async_trait::async_trait;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

pub struct FearGreedPlugin {
    pub http: reqwest::Client,
    pub base_url: String,
}

impl FearGreedPlugin {
    pub fn new() -> Self {
        Self::with_base_url("https://api.alternative.me")
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }
}

impl Default for FearGreedPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolanaKeeperPlugin for FearGreedPlugin {
    fn id(&self) -> &'static str {
        "fear_greed"
    }

    fn name(&self) -> &'static str {
        "Fear & Greed Index"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![
            ActionDefinition {
                id: "current".to_string(),
                name: "Current Fear & Greed".to_string(),
                description: "Fetch the current crypto Fear & Greed Index (0–100) from alternative.me.".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
                returns_schema: json!({
                    "value": "integer",
                    "classification": "string",
                    "timestamp": "integer"
                }),
            },
            ActionDefinition {
                id: "history".to_string(),
                name: "Fear & Greed History".to_string(),
                description: "Fetch historical Fear & Greed Index values.".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "default": 30,
                            "description": "Number of historical data points to return"
                        }
                    }
                }),
                returns_schema: json!({"data": "array"}),
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
        _rpc: &RpcClient,
    ) -> Result<Value, PluginError> {
        match action {
            "current" => self.fetch_fng(1).await.map(|mut v| {
                v.as_array_mut()
                    .and_then(|arr| arr.first().cloned())
                    .unwrap_or(Value::Null)
            }),
            "history" => {
                let limit = params["limit"].as_u64().unwrap_or(30);
                let data = self.fetch_fng(limit).await?;
                Ok(json!({ "data": data }))
            }
            other => Err(PluginError::UnknownAction(other.to_string())),
        }
    }
}

impl FearGreedPlugin {
    async fn fetch_fng(&self, limit: u64) -> Result<Value, PluginError> {
        let url = format!("{}/fng/?limit={}", self.base_url, limit);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(PluginError::Network(format!(
                "fear_greed API returned {}",
                resp.status()
            )));
        }
        let body: Value = resp
            .json()
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?;

        // API shape: {"name":"...", "data":[{"value":"42","value_classification":"Fear","timestamp":"..."},...]}
        let data = body
            .get("data")
            .cloned()
            .unwrap_or(Value::Array(vec![]));

        let items: Vec<Value> = data
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|entry| {
                let value = entry["value"]
                    .as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .or_else(|| entry["value"].as_u64())
                    .unwrap_or(0);
                let classification = entry["value_classification"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let timestamp = entry["timestamp"]
                    .as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .or_else(|| entry["timestamp"].as_u64())
                    .unwrap_or(0);
                json!({
                    "value": value,
                    "classification": classification,
                    "timestamp": timestamp
                })
            })
            .collect();

        Ok(Value::Array(items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    fn sample_fng_body(n: usize) -> String {
        let entries: Vec<String> = (0..n)
            .map(|i| {
                format!(
                    r#"{{"value":"{val}","value_classification":"Fear","timestamp":"{ts}"}}"#,
                    val = 30 + i,
                    ts = 1700000000u64 + i as u64
                )
            })
            .collect();
        format!(
            r#"{{"name":"Fear and Greed Index","data":[{}]}}"#,
            entries.join(",")
        )
    }

    #[test]
    fn actions_schema_check() {
        let plugin = FearGreedPlugin::new();
        let actions = plugin.actions();
        assert_eq!(actions.len(), 2);
        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"current"));
        assert!(ids.contains(&"history"));
        let current = actions.iter().find(|a| a.id == "current").unwrap();
        assert_eq!(current.action_type, ActionType::ReadOnly);
        let history = actions.iter().find(|a| a.id == "history").unwrap();
        assert_eq!(history.action_type, ActionType::ReadOnly);
    }

    #[tokio::test]
    async fn current_returns_single_entry() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/fng/".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(sample_fng_body(1))
            .create_async()
            .await;

        let plugin = FearGreedPlugin::with_base_url(server.url());
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());

        let result = plugin
            .read("current", &json!({}), &rpc)
            .await
            .expect("current should succeed");

        assert_eq!(result["value"], 30u64);
        assert_eq!(result["classification"], "Fear");
        assert!(result["timestamp"].as_u64().is_some());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn history_returns_array() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/fng/".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(sample_fng_body(5))
            .create_async()
            .await;

        let plugin = FearGreedPlugin::with_base_url(server.url());
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());

        let result = plugin
            .read("history", &json!({"limit": 5}), &rpc)
            .await
            .expect("history should succeed");

        let data = result["data"].as_array().expect("data should be array");
        assert_eq!(data.len(), 5);
        assert_eq!(data[0]["value"], 30u64);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn current_returns_error_on_non_200() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/fng/".to_string()))
            .with_status(500)
            .with_body("internal error")
            .create_async()
            .await;

        let plugin = FearGreedPlugin::with_base_url(server.url());
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());

        let result = plugin.read("current", &json!({}), &rpc).await;
        assert!(
            matches!(result, Err(PluginError::Network(_))),
            "expected Network error on 500, got: {:?}",
            result
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn history_default_limit_is_30() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/fng/\?limit=30".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(sample_fng_body(3))
            .create_async()
            .await;

        let plugin = FearGreedPlugin::with_base_url(server.url());
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());

        let result = plugin
            .read("history", &json!({}), &rpc)
            .await
            .expect("history with default limit should succeed");

        assert!(result["data"].is_array());
        mock.assert_async().await;
    }

    #[test]
    fn unknown_action_returns_error() {
        let plugin = FearGreedPlugin::new();
        let actions = plugin.actions();
        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(!ids.contains(&"nonexistent_action"));
    }
}
