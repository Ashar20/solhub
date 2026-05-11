use super::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};
use async_trait::async_trait;
use base64::Engine as _;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

pub struct JupiterPlugin {
    http: reqwest::Client,
    base_url: String,
    price_base_url: String,
}

impl JupiterPlugin {
    pub fn new() -> Self {
        Self::with_base_url("https://lite-api.jup.ag/swap/v1")
    }

    pub fn with_base_url(url: impl Into<String>) -> Self {
        let base = url.into();
        Self {
            http: reqwest::Client::new(),
            base_url: base,
            price_base_url: "https://lite-api.jup.ag/price/v3".to_string(),
        }
    }

    pub fn with_price_url(mut self, url: impl Into<String>) -> Self {
        self.price_base_url = url.into();
        self
    }
}

impl Default for JupiterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolanaKeeperPlugin for JupiterPlugin {
    fn id(&self) -> &'static str {
        "jupiter"
    }

    fn name(&self) -> &'static str {
        "Jupiter"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![
            ActionDefinition {
                id: "swap".to_string(),
                name: "Swap Tokens".to_string(),
                description: "Best-route token swap via Jupiter aggregator".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["input_mint", "output_mint", "amount"],
                    "properties": {
                        "input_mint": {
                            "type": "string",
                            "description": "Input token mint address"
                        },
                        "output_mint": {
                            "type": "string",
                            "description": "Output token mint address"
                        },
                        "amount": {
                            "type": "integer",
                            "description": "Amount in input token base units"
                        },
                        "slippage_bps": {
                            "type": "integer",
                            "default": 50,
                            "description": "Slippage in bps (50 = 0.5%)"
                        }
                    }
                }),
                returns_schema: json!({
                    "output_amount": "integer",
                    "price_impact_pct": "number",
                    "route": "array"
                }),
            },
            ActionDefinition {
                id: "quote".to_string(),
                name: "Get Swap Quote".to_string(),
                description: "Get a Jupiter best-route price quote for an input/output token pair (no transaction).".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["input_mint", "output_mint", "amount"],
                    "properties": {
                        "input_mint": {"type": "string"},
                        "output_mint": {"type": "string"},
                        "amount": {"type": "integer", "description": "Amount in input token base units"},
                        "slippage_bps": {"type": "integer", "default": 50}
                    }
                }),
                returns_schema: json!({
                    "input_amount": "string",
                    "output_amount": "string",
                    "price_impact_pct": "string",
                    "route_plan": "array"
                }),
            },
            ActionDefinition {
                id: "price".to_string(),
                name: "Token Prices".to_string(),
                description: "Fetch USD prices for one or more token mints from Jupiter Price API v3.".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["ids"],
                    "properties": {
                        "ids": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Array of token mint addresses to price"
                        }
                    }
                }),
                returns_schema: json!({
                    "prices": "object"
                }),
            },
        ]
    }

    async fn build_transactions(
        &self,
        action: &str,
        params: &Value,
        wallet_pubkey: &Pubkey,
        _rpc: &RpcClient,
    ) -> Result<Vec<VersionedTransaction>, PluginError> {
        match action {
            "swap" => {
                let input_mint = params["input_mint"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("input_mint".into()))?;
                let output_mint = params["output_mint"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("output_mint".into()))?;
                let amount = params["amount"]
                    .as_u64()
                    .ok_or_else(|| PluginError::InvalidParam("amount".into()))?;
                let slippage_bps = params["slippage_bps"].as_u64().unwrap_or(50);

                // 1. Get quote
                let quote_url = format!(
                    "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}",
                    self.base_url, input_mint, output_mint, amount, slippage_bps
                );
                let quote_resp = self
                    .http
                    .get(&quote_url)
                    .send()
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;
                if !quote_resp.status().is_success() {
                    return Err(PluginError::Network(format!(
                        "quote API returned {}",
                        quote_resp.status()
                    )));
                }
                let quote: Value = quote_resp
                    .json()
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;

                // 2. Get swap transaction
                let swap_url = format!("{}/swap", self.base_url);
                let swap_body = json!({
                    "userPublicKey": wallet_pubkey.to_string(),
                    "quoteResponse": quote,
                    "wrapAndUnwrapSol": true
                });
                let swap_resp = self
                    .http
                    .post(&swap_url)
                    .json(&swap_body)
                    .send()
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;
                if !swap_resp.status().is_success() {
                    return Err(PluginError::Network(format!(
                        "swap API returned {}",
                        swap_resp.status()
                    )));
                }
                let swap_json: Value = swap_resp
                    .json()
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;

                let tx_b64 = swap_json["swapTransaction"]
                    .as_str()
                    .ok_or_else(|| PluginError::Network("missing swapTransaction".into()))?;

                let tx_bytes = base64::prelude::BASE64_STANDARD
                    .decode(tx_b64)
                    .map_err(|e| PluginError::Other(format!("base64 decode: {e}")))?;

                let tx = bincode::deserialize::<VersionedTransaction>(&tx_bytes)
                    .map_err(|e| PluginError::Other(format!("deserialize tx: {e}")))?;

                Ok(vec![tx])
            }
            _ => Err(PluginError::UnknownAction(action.to_string())),
        }
    }

    async fn read(
        &self,
        action: &str,
        params: &Value,
        _rpc: &RpcClient,
    ) -> Result<Value, PluginError> {
        match action {
            "quote" => {
                let input_mint = params["input_mint"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("input_mint".into()))?;
                let output_mint = params["output_mint"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("output_mint".into()))?;
                let amount = params["amount"]
                    .as_u64()
                    .ok_or_else(|| PluginError::InvalidParam("amount".into()))?;
                let slippage_bps = params["slippage_bps"].as_u64().unwrap_or(50);

                let url = format!(
                    "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}",
                    self.base_url, input_mint, output_mint, amount, slippage_bps
                );
                let resp = self
                    .http
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;
                if !resp.status().is_success() {
                    return Err(PluginError::Network(format!(
                        "jupiter quote {} status {}",
                        url,
                        resp.status()
                    )));
                }
                let json: Value = resp
                    .json()
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;
                Ok(json)
            }
            "price" => {
                let ids = params["ids"]
                    .as_array()
                    .ok_or_else(|| PluginError::InvalidParam("ids".into()))?;
                if ids.is_empty() {
                    return Err(PluginError::InvalidParam("ids: must not be empty".into()));
                }
                let id_str = ids
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(",");
                let url = format!("{}?ids={}", self.price_base_url, id_str);
                let resp = self
                    .http
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;
                if !resp.status().is_success() {
                    return Err(PluginError::Network(format!(
                        "jupiter price API returned {}",
                        resp.status()
                    )));
                }
                let body: Value = resp
                    .json()
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;
                // Pass through the data field as `prices`
                let prices = body.get("data").cloned().unwrap_or(body);
                Ok(json!({ "prices": prices }))
            }
            _ => Err(PluginError::NotSupported),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use solana_sdk::{
        hash::Hash,
        message::{v0, VersionedMessage},
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
    };

    /// Build a minimal valid VersionedTransaction with no instructions.
    fn make_versioned_tx() -> VersionedTransaction {
        let payer = Keypair::new();
        let msg = v0::Message::try_compile(&payer.pubkey(), &[], &[], Hash::default())
            .expect("compile empty message");
        let versioned_msg = VersionedMessage::V0(msg);
        VersionedTransaction::try_new(versioned_msg, &[&payer])
            .expect("sign transaction")
    }

    #[test]
    fn actions_includes_swap_with_full_schema() {
        let plugin = JupiterPlugin::new();
        let actions = plugin.actions();
        assert_eq!(actions.len(), 3, "expected swap + quote + price");
        let swap = actions.iter().find(|a| a.id == "swap").unwrap();
        assert_eq!(swap.name, "Swap Tokens");
        assert_eq!(swap.action_type, ActionType::Transaction);

        let schema = &swap.params_schema;
        assert_eq!(schema["type"], "object");
        let props = &schema["properties"];
        assert!(props.get("input_mint").is_some());
        assert!(props.get("output_mint").is_some());
        assert!(props.get("amount").is_some());
        assert!(props.get("slippage_bps").is_some());

        let required = schema["required"].as_array().expect("required array");
        assert!(required.iter().any(|v| v == "input_mint"));
        assert!(required.iter().any(|v| v == "output_mint"));
        assert!(required.iter().any(|v| v == "amount"));
    }

    #[tokio::test]
    async fn swap_builds_transaction_from_mocked_response() {
        let mut server = Server::new_async().await;

        let tx = make_versioned_tx();
        let tx_bytes = bincode::serialize(&tx).expect("serialize");
        let tx_b64 = base64::prelude::BASE64_STANDARD.encode(&tx_bytes);

        let quote_mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/quote".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"inputMint":"So11111111111111111111111111111111111111112","outputMint":"EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v","inAmount":"1000000","outAmount":"99000","priceImpactPct":"0.01","routePlan":[]}"#)
            .create_async()
            .await;

        let swap_mock = server
            .mock("POST", "/swap")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(r#"{{"swapTransaction":"{}"}}"#, tx_b64))
            .create_async()
            .await;

        let plugin = JupiterPlugin::with_base_url(server.url());
        let wallet = Pubkey::new_unique();
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());

        let params = serde_json::json!({
            "input_mint": "So11111111111111111111111111111111111111112",
            "output_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "amount": 1_000_000u64
        });

        let txs = plugin
            .build_transactions("swap", &params, &wallet, &rpc)
            .await
            .expect("build_transactions should succeed");

        assert_eq!(txs.len(), 1);

        quote_mock.assert_async().await;
        swap_mock.assert_async().await;
    }

    #[tokio::test]
    async fn build_transactions_rejects_missing_input_mint() {
        let plugin = JupiterPlugin::new();
        let wallet = Pubkey::new_unique();
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = serde_json::json!({
            "output_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "amount": 1_000_000u64
        });

        let result = plugin
            .build_transactions("swap", &params, &wallet, &rpc)
            .await;

        assert!(matches!(result, Err(PluginError::InvalidParam(ref s)) if s == "input_mint"));
    }

    #[test]
    fn quote_action_in_schema() {
        let plugin = JupiterPlugin::new();
        let actions = plugin.actions();
        assert_eq!(actions.len(), 3, "expected swap + quote + price");
        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"swap"));
        assert!(ids.contains(&"quote"));
        let quote = actions.iter().find(|a| a.id == "quote").unwrap();
        assert_eq!(quote.action_type, ActionType::ReadOnly);
        let required = quote.params_schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "input_mint"));
        assert!(required.iter().any(|v| v == "output_mint"));
        assert!(required.iter().any(|v| v == "amount"));
    }

    #[tokio::test]
    async fn quote_calls_correct_endpoint() {
        let mut server = Server::new_async().await;

        let quote_body = r#"{"inputAmount":"1000000","outputAmount":"99000","priceImpactPct":"0.01","routePlan":[]}"#;
        let mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/quote".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(quote_body)
            .create_async()
            .await;

        let plugin = JupiterPlugin::with_base_url(server.url());
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = serde_json::json!({
            "input_mint": "So11111111111111111111111111111111111111112",
            "output_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "amount": 1_000_000u64
        });

        let result = plugin.read("quote", &params, &rpc).await.expect("quote should succeed");
        assert_eq!(result["inputAmount"], "1000000");
        assert_eq!(result["outputAmount"], "99000");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn quote_missing_input_mint_returns_invalid_param() {
        let plugin = JupiterPlugin::new();
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = serde_json::json!({
            "output_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "amount": 1_000_000u64
        });

        let result = plugin.read("quote", &params, &rpc).await;
        assert!(matches!(result, Err(PluginError::InvalidParam(ref s)) if s == "input_mint"));
    }

    #[test]
    fn price_action_in_schema() {
        let plugin = JupiterPlugin::new();
        let actions = plugin.actions();
        let price = actions.iter().find(|a| a.id == "price").unwrap();
        assert_eq!(price.action_type, ActionType::ReadOnly);
        let required = price.params_schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "ids"));
    }

    #[tokio::test]
    async fn price_calls_price_api_and_returns_prices() {
        let mut server = Server::new_async().await;

        let price_body = r#"{
            "data": {
                "So11111111111111111111111111111111111111112": {
                    "id": "So11111111111111111111111111111111111111112",
                    "type": "derivedPrice",
                    "price": "150.25",
                    "last_updated_at": 1700000000
                }
            }
        }"#;

        let mint = "So11111111111111111111111111111111111111112";
        let mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/price\?ids=".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(price_body)
            .create_async()
            .await;

        // Use server.url() + "/price" so the URL becomes /price?ids=... which mockito can match
        let plugin = JupiterPlugin::new().with_price_url(format!("{}/price", server.url()));
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = serde_json::json!({
            "ids": [mint]
        });

        let result = plugin.read("price", &params, &rpc).await.expect("price should succeed");
        let prices = &result["prices"];
        assert!(prices[mint].is_object());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn price_empty_ids_returns_invalid_param() {
        let plugin = JupiterPlugin::new();
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = serde_json::json!({"ids": []});
        let result = plugin.read("price", &params, &rpc).await;
        assert!(matches!(result, Err(PluginError::InvalidParam(_))));
    }
}
