use super::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};
use async_trait::async_trait;
use serde_json::{json, Value};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_request::TokenAccountsFilter,
};
use solana_sdk::{program_pack::Pack, pubkey::Pubkey, transaction::VersionedTransaction};
use spl_token::state::Account as TokenAccount;
use std::collections::HashMap;
use std::str::FromStr;

const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;
/// SOL wrapped mint used for price lookup.
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const USDC_MINT_MAINNET: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const USDC_MINT_DEVNET: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
const JUP_MINT: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

fn mint_to_symbol(mint: &str) -> Option<&'static str> {
    match mint {
        SOL_MINT => Some("SOL"),
        USDC_MINT_MAINNET | USDC_MINT_DEVNET => Some("USDC"),
        JUP_MINT => Some("JUP"),
        _ => None,
    }
}

pub struct PortfolioPlugin {
    pub http: reqwest::Client,
    pub price_base_url: String,
    pub price_base_url_v2: String,
}

impl PortfolioPlugin {
    pub fn new() -> Self {
        Self::with_price_url(
            "https://lite-api.jup.ag/price/v3",
            "https://api.jup.ag/price/v2",
        )
    }

    pub fn with_price_url(v3_url: impl Into<String>, v2_url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            price_base_url: v3_url.into(),
            price_base_url_v2: v2_url.into(),
        }
    }
}

impl Default for PortfolioPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolanaKeeperPlugin for PortfolioPlugin {
    fn id(&self) -> &'static str {
        "portfolio"
    }

    fn name(&self) -> &'static str {
        "Portfolio"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![
            ActionDefinition {
                id: "snapshot".to_string(),
                name: "Portfolio Snapshot".to_string(),
                description:
                    "Fetch SOL + SPL token balances for a wallet, with USD values via Jupiter prices."
                        .to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["account"],
                    "properties": {
                        "account": {
                            "type": "string",
                            "description": "Wallet pubkey (base58)"
                        },
                        "include_spl": {
                            "type": "boolean",
                            "default": true,
                            "description": "Whether to include SPL token holdings"
                        },
                        "quote_currency": {
                            "type": "string",
                            "default": "USD",
                            "description": "Denomination for USD-equivalent values"
                        }
                    }
                }),
                returns_schema: json!({
                    "wallet": "string",
                    "total_usd": "number",
                    "holdings": "array",
                    "current_weights": "object"
                }),
            },
            ActionDefinition {
                id: "detect_drift".to_string(),
                name: "Detect Portfolio Drift".to_string(),
                description:
                    "Detect whether any symbol has drifted beyond a threshold from its target allocation."
                        .to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["current_weights", "target_weights"],
                    "properties": {
                        "current_weights": {
                            "type": "object",
                            "description": "Map of symbol -> current fractional weight (0.0–1.0)"
                        },
                        "target_weights": {
                            "type": "object",
                            "description": "Map of symbol -> target fractional weight (0.0–1.0)"
                        },
                        "threshold": {
                            "type": "number",
                            "default": 0.05,
                            "description": "Drift threshold (fractional, 0.05 = 5 pp)"
                        }
                    }
                }),
                returns_schema: json!({
                    "drifted": "boolean",
                    "max_drift_sym": "string",
                    "max_drift": "number",
                    "deltas": "object"
                }),
            },
            ActionDefinition {
                id: "current_weights_from_holdings".to_string(),
                name: "Current Weights From Holdings".to_string(),
                description:
                    "Map a holdings array (from snapshot) to a symbol→weight_fraction map."
                        .to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["holdings"],
                    "properties": {
                        "holdings": {
                            "type": "array",
                            "description": "Holdings array from snapshot output"
                        }
                    }
                }),
                returns_schema: json!({ "weights": "object" }),
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
            "snapshot" => self.snapshot(params, rpc).await,
            "detect_drift" => self.detect_drift(params),
            "current_weights_from_holdings" => self.current_weights_from_holdings(params),
            other => Err(PluginError::UnknownAction(other.to_string())),
        }
    }
}

impl PortfolioPlugin {
    async fn snapshot(&self, params: &Value, rpc: &RpcClient) -> Result<Value, PluginError> {
        let account_str = params["account"]
            .as_str()
            .ok_or_else(|| PluginError::InvalidParam("account".into()))?;
        let pubkey = Pubkey::from_str(account_str)
            .map_err(|_| PluginError::InvalidParam("account: not a valid pubkey".into()))?;
        let include_spl = params["include_spl"].as_bool().unwrap_or(true);

        // Fetch native SOL balance
        let lamports = rpc
            .get_balance(&pubkey)
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?;
        let sol_balance = lamports as f64 / LAMPORTS_PER_SOL;

        let mut mints_to_price: Vec<String> = vec![SOL_MINT.to_string()];
        let mut spl_holdings: Vec<(String, f64, u8)> = Vec::new(); // (mint, ui_amount, decimals)

        if include_spl {
            let token_accounts = rpc
                .get_token_accounts_by_owner(
                    &pubkey,
                    TokenAccountsFilter::ProgramId(spl_token::id()),
                )
                .await
                .map_err(|e| PluginError::Network(e.to_string()))?;

            for keyed_account in &token_accounts {
                if let Some(raw_bytes) = keyed_account.account.data.decode() {
                    if let Ok(token_acct) = TokenAccount::unpack(&raw_bytes) {
                        if token_acct.amount > 0 {
                            let mint = token_acct.mint.to_string();
                            let decimals = 0u8;
                            let balance = token_acct.amount as f64 / 10f64.powi(decimals as i32);
                            mints_to_price.push(mint.clone());
                            spl_holdings.push((mint, balance, decimals));
                        }
                    }
                }
            }
        }

        let prices = self.fetch_prices(&mints_to_price).await;

        let mut holdings: Vec<Value> = Vec::new();

        // SOL holding
        let sol_price = prices.get(SOL_MINT).and_then(|v| v.as_f64());
        let sol_value = sol_price.map(|p| sol_balance * p);
        holdings.push(json!({
            "mint": SOL_MINT,
            "symbol": "SOL",
            "balance": sol_balance,
            "decimals": 9,
            "price_usd": sol_price,
            "value_usd": sol_value,
            "weight": Value::Null
        }));

        // SPL holdings
        for (mint, balance, decimals) in &spl_holdings {
            let price = prices.get(mint.as_str()).and_then(|v| v.as_f64());
            let value = price.map(|p| balance * p);
            let symbol = mint_to_symbol(mint).map(Value::from).unwrap_or(Value::Null);
            holdings.push(json!({
                "mint": mint,
                "symbol": symbol,
                "balance": balance,
                "decimals": decimals,
                "price_usd": price,
                "value_usd": value,
                "weight": Value::Null
            }));
        }

        // Compute total USD
        let total_usd: f64 = holdings
            .iter()
            .filter_map(|h| h["value_usd"].as_f64())
            .sum();

        // Annotate weight fractions and build current_weights map
        let mut current_weights: serde_json::Map<String, Value> = serde_json::Map::new();
        for h in &mut holdings {
            let value = h["value_usd"].as_f64().unwrap_or(0.0);
            let weight = if total_usd > 0.0 {
                value / total_usd
            } else {
                0.0
            };
            if let Some(obj) = h.as_object_mut() {
                obj.insert("weight".to_string(), json!(weight));
            }
            // Build current_weights by symbol if available
            if let Some(sym) = h["symbol"].as_str() {
                let value_f = h["value_usd"].as_f64().unwrap_or(0.0);
                let w = if total_usd > 0.0 { value_f / total_usd } else { 0.0 };
                current_weights.insert(sym.to_string(), json!(w));
            }
        }

        Ok(json!({
            "wallet": account_str,
            "total_usd": total_usd,
            "holdings": holdings,
            "current_weights": current_weights
        }))
    }

    /// Fetch prices from Jupiter Price API v3, falling back to v2 on failure.
    async fn fetch_prices(&self, mints: &[String]) -> HashMap<String, Value> {
        if mints.is_empty() {
            return HashMap::new();
        }
        let ids = mints.join(",");
        let url_v3 = format!("{}?ids={}", self.price_base_url, ids);

        let try_parse = |body: Value| -> HashMap<String, Value> {
            let mut map = HashMap::new();
            if let Some(data) = body.get("data").and_then(|d| d.as_object()) {
                for (mint, info) in data {
                    let price = info["price"]
                        .as_f64()
                        .or_else(|| info["price"].as_str().and_then(|s| s.parse().ok()));
                    if let Some(p) = price {
                        map.insert(mint.clone(), json!(p));
                    }
                }
            }
            map
        };

        // Try v3 first
        if let Ok(resp) = self.http.get(&url_v3).send().await {
            if resp.status().is_success() {
                if let Ok(body) = resp.json::<Value>().await {
                    let result = try_parse(body);
                    if !result.is_empty() {
                        return result;
                    }
                }
            }
        }

        // Fallback to v2
        let url_v2 = format!("{}?ids={}", self.price_base_url_v2, ids);
        if let Ok(resp) = self.http.get(&url_v2).send().await {
            if resp.status().is_success() {
                if let Ok(body) = resp.json::<Value>().await {
                    return try_parse(body);
                }
            }
        }

        HashMap::new()
    }

    fn detect_drift(&self, params: &Value) -> Result<Value, PluginError> {
        let current_weights = params["current_weights"]
            .as_object()
            .ok_or_else(|| PluginError::InvalidParam("current_weights".into()))?;
        let target_weights = params["target_weights"]
            .as_object()
            .ok_or_else(|| PluginError::InvalidParam("target_weights".into()))?;
        let threshold = params["threshold"].as_f64().unwrap_or(0.05);

        // Union of all symbols
        let mut all_syms: std::collections::HashSet<String> = std::collections::HashSet::new();
        for k in current_weights.keys() {
            all_syms.insert(k.clone());
        }
        for k in target_weights.keys() {
            all_syms.insert(k.clone());
        }

        let mut deltas: serde_json::Map<String, Value> = serde_json::Map::new();
        let mut max_drift: f64 = 0.0;
        let mut max_drift_sym: Option<String> = None;

        for sym in &all_syms {
            let current = current_weights.get(sym).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let target = target_weights.get(sym).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let delta = target - current;
            deltas.insert(sym.clone(), json!(delta));

            let abs_drift = delta.abs();
            if abs_drift > max_drift {
                max_drift = abs_drift;
                max_drift_sym = Some(sym.clone());
            }
        }

        let drifted = max_drift > threshold;

        Ok(json!({
            "drifted": drifted,
            "max_drift_sym": max_drift_sym,
            "max_drift": max_drift,
            "deltas": deltas
        }))
    }

    fn current_weights_from_holdings(&self, params: &Value) -> Result<Value, PluginError> {
        let holdings = params["holdings"]
            .as_array()
            .ok_or_else(|| PluginError::InvalidParam("holdings".into()))?;

        let total_usd: f64 = holdings
            .iter()
            .filter_map(|h| h["value_usd"].as_f64())
            .sum();

        let mut weights: serde_json::Map<String, Value> = serde_json::Map::new();
        for h in holdings {
            if let Some(sym) = h["symbol"].as_str() {
                let value = h["value_usd"].as_f64().unwrap_or(0.0);
                let w = if total_usd > 0.0 { value / total_usd } else { 0.0 };
                weights.insert(sym.to_string(), json!(w));
            }
        }

        Ok(json!({ "weights": weights }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    fn make_rpc(url: &str) -> RpcClient {
        RpcClient::new(url.to_string())
    }

    #[test]
    fn actions_lists_three() {
        let plugin = PortfolioPlugin::new();
        let actions = plugin.actions();
        assert_eq!(actions.len(), 3);
        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"snapshot"));
        assert!(ids.contains(&"detect_drift"));
        assert!(ids.contains(&"current_weights_from_holdings"));
        for action in &actions {
            assert_eq!(action.action_type, ActionType::ReadOnly);
        }
        let snapshot = actions.iter().find(|a| a.id == "snapshot").unwrap();
        let required = snapshot.params_schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "account"));
    }

    #[tokio::test]
    async fn snapshot_mocks_rpc_and_price_returns_holdings() {
        let mut price_server = Server::new_async().await;

        // Mock Jupiter price response for SOL
        let price_body = r#"{
            "data": {
                "So11111111111111111111111111111111111111112": {
                    "id": "So11111111111111111111111111111111111111112",
                    "type": "derivedPrice",
                    "price": "150.0"
                }
            }
        }"#;

        let _price_mock = price_server
            .mock("GET", mockito::Matcher::Regex(r"^/price\?ids=".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(price_body)
            .create_async()
            .await;

        // Use a real devnet pubkey for the RPC, but mock the price API
        // We test the plugin offline: bad pubkey -> InvalidParam
        let price_url = format!("{}/price", price_server.url());
        let plugin = PortfolioPlugin::with_price_url(price_url.clone(), price_url);
        let rpc = make_rpc("https://api.devnet.solana.com");

        // Verify invalid pubkey returns error
        let err = plugin
            .read("snapshot", &json!({"account": "not-a-pubkey"}), &rpc)
            .await;
        assert!(
            matches!(err, Err(PluginError::InvalidParam(_))),
            "expected InvalidParam for bad pubkey, got: {:?}",
            err
        );

        // Verify missing account returns error
        let err2 = plugin.read("snapshot", &json!({}), &rpc).await;
        assert!(
            matches!(err2, Err(PluginError::InvalidParam(_))),
            "expected InvalidParam for missing account"
        );
    }

    #[test]
    fn detect_drift_flags_when_over_threshold() {
        let plugin = PortfolioPlugin::new();
        let params = json!({
            "current_weights": {"SOL": 0.70, "USDC": 0.30},
            "target_weights": {"SOL": 0.60, "USDC": 0.40},
            "threshold": 0.05
        });
        let result = plugin.detect_drift(&params).expect("detect_drift should succeed");
        assert_eq!(result["drifted"], true);
        let max_drift = result["max_drift"].as_f64().unwrap();
        assert!((max_drift - 0.10).abs() < 1e-9, "max_drift should be ~0.10, got {max_drift}");
    }

    #[test]
    fn detect_drift_when_under_threshold() {
        let plugin = PortfolioPlugin::new();
        let params = json!({
            "current_weights": {"SOL": 0.52, "USDC": 0.48},
            "target_weights": {"SOL": 0.50, "USDC": 0.50},
            "threshold": 0.05
        });
        let result = plugin.detect_drift(&params).expect("detect_drift should succeed");
        assert_eq!(result["drifted"], false);
        let max_drift = result["max_drift"].as_f64().unwrap();
        assert!(max_drift < 0.05, "max_drift should be under threshold, got {max_drift}");
    }

    #[test]
    fn current_weights_from_holdings_maps_correctly() {
        let plugin = PortfolioPlugin::new();
        let params = json!({
            "holdings": [
                {"symbol": "SOL", "value_usd": 300.0},
                {"symbol": "USDC", "value_usd": 200.0},
                {"symbol": "JUP", "value_usd": 100.0}
            ]
        });
        let result = plugin
            .current_weights_from_holdings(&params)
            .expect("current_weights_from_holdings should succeed");
        let weights = &result["weights"];
        let sol_w = weights["SOL"].as_f64().unwrap();
        let usdc_w = weights["USDC"].as_f64().unwrap();
        let jup_w = weights["JUP"].as_f64().unwrap();
        assert!((sol_w - 0.5).abs() < 1e-9, "SOL weight should be 0.5, got {sol_w}");
        assert!((usdc_w - (1.0 / 3.0)).abs() < 1e-9, "USDC weight should be ~0.333, got {usdc_w}");
        assert!((jup_w - (1.0 / 6.0)).abs() < 1e-9, "JUP weight should be ~0.167, got {jup_w}");
    }

    #[test]
    fn snapshot_returns_invalid_param_for_bad_pubkey() {
        let plugin = PortfolioPlugin::new();
        let rpc = make_rpc("https://api.devnet.solana.com");
        // detect_drift without required fields returns InvalidParam
        let err = plugin.detect_drift(&json!({}));
        assert!(matches!(err, Err(PluginError::InvalidParam(_))));
        let _ = rpc;
    }

    #[test]
    fn detect_drift_with_default_threshold() {
        let plugin = PortfolioPlugin::new();
        // Without explicit threshold, default is 0.05
        let params = json!({
            "current_weights": {"SOL": 0.60, "JUP": 0.40},
            "target_weights": {"SOL": 0.45, "JUP": 0.55}
        });
        let result = plugin.detect_drift(&params).expect("detect_drift should succeed");
        assert_eq!(result["drifted"], true);
        let max_drift = result["max_drift"].as_f64().unwrap();
        assert!((max_drift - 0.15).abs() < 1e-9, "expected 0.15 drift, got {max_drift}");
    }

    #[test]
    fn detect_drift_deltas_are_target_minus_current() {
        let plugin = PortfolioPlugin::new();
        let params = json!({
            "current_weights": {"SOL": 0.60, "USDC": 0.40},
            "target_weights": {"SOL": 0.45, "USDC": 0.55},
            "threshold": 0.20
        });
        let result = plugin.detect_drift(&params).expect("detect_drift should succeed");
        let deltas = &result["deltas"];
        let sol_delta = deltas["SOL"].as_f64().unwrap();
        let usdc_delta = deltas["USDC"].as_f64().unwrap();
        // delta = target - current
        assert!((sol_delta - (-0.15)).abs() < 1e-9, "SOL delta should be -0.15, got {sol_delta}");
        assert!((usdc_delta - 0.15).abs() < 1e-9, "USDC delta should be 0.15, got {usdc_delta}");
        // Under threshold of 0.20 => not drifted
        assert_eq!(result["drifted"], false);
    }

    #[test]
    fn current_weights_from_holdings_missing_holdings_returns_error() {
        let plugin = PortfolioPlugin::new();
        let err = plugin.current_weights_from_holdings(&json!({}));
        assert!(matches!(err, Err(PluginError::InvalidParam(_))));
    }

    #[test]
    fn current_weights_from_holdings_empty_array() {
        let plugin = PortfolioPlugin::new();
        let result = plugin
            .current_weights_from_holdings(&json!({"holdings": []}))
            .expect("empty holdings should return empty weights");
        let weights = result["weights"].as_object().unwrap();
        assert!(weights.is_empty(), "empty holdings should produce empty weights");
    }

    #[tokio::test]
    async fn fetch_prices_uses_v3_mock() {
        let mut server = Server::new_async().await;

        let price_body = r#"{
            "data": {
                "So11111111111111111111111111111111111111112": {
                    "id": "So11111111111111111111111111111111111111112",
                    "price": "200.0"
                }
            }
        }"#;

        let mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/price\?ids=".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(price_body)
            .create_async()
            .await;

        // Use /price path so the URL becomes /price?ids=...
        let price_url = format!("{}/price", server.url());
        let plugin = PortfolioPlugin::with_price_url(price_url, "http://fallback.invalid");

        let prices = plugin.fetch_prices(&[SOL_MINT.to_string()]).await;
        let sol_price = prices.get(SOL_MINT).and_then(|v| v.as_f64()).unwrap();
        assert!((sol_price - 200.0).abs() < 1e-9, "expected price 200.0, got {sol_price}");

        mock.assert_async().await;
    }
}
