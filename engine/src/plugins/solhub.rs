use super::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};
use async_trait::async_trait;
use db::{Db, NewRun};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use std::str::FromStr;
use uuid::Uuid;

pub struct SolhubPlugin {
    pub db: Db,
    pub max_depth: usize,
    pub default_timeout_secs: u64,
}

impl SolhubPlugin {
    pub fn new(db: Db) -> Self {
        Self {
            db,
            max_depth: 3,
            default_timeout_secs: 60,
        }
    }
}

#[async_trait]
impl SolanaKeeperPlugin for SolhubPlugin {
    fn id(&self) -> &'static str {
        "solhub"
    }

    fn name(&self) -> &'static str {
        "SolHub"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![
            ActionDefinition {
                id: "run_workflow".to_string(),
                name: "Run Sub-Workflow".to_string(),
                description: "Trigger another workflow and wait for its terminal state. Cycle-detected; max depth 3.".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["workflow_id"],
                    "properties": {
                        "workflow_id": {"type": "string", "description": "UUID of the workflow to trigger"},
                        "timeout_secs": {"type": "integer", "default": 60, "description": "Max wait for sub-run to complete"},
                        "parent_run_id": {"type": "string", "description": "Set by executor; clients should not pass this"},
                        "depth": {"type": "integer", "default": 0, "description": "Set by executor"}
                    }
                }),
                returns_schema: json!({
                    "child_run_id": "string",
                    "status": "string",
                    "steps_log": "array",
                    "signature": "string"
                }),
            },
            ActionDefinition {
                id: "delta_calc".to_string(),
                name: "Portfolio Delta Calculator".to_string(),
                description: "Compute rebalancing swaps from current to target weights.".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["current", "target", "total_value_usd"],
                    "properties": {
                        "current": {
                            "type": "object",
                            "description": "Current weights as {mint: pct} summing to 100"
                        },
                        "target": {
                            "type": "object",
                            "description": "Target weights as {mint: pct} summing to 100"
                        },
                        "total_value_usd": {
                            "type": "number",
                            "description": "Total portfolio value in USD"
                        }
                    }
                }),
                returns_schema: json!({
                    "swaps": "array",
                    "total_swap_value_usd": "number"
                }),
            },
            ActionDefinition {
                id: "guard_rails".to_string(),
                name: "Guard Rails".to_string(),
                description: "Validate proposed swaps against safety rules before execution.".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["swaps", "total_value_usd", "confidence_score"],
                    "properties": {
                        "swaps": {"type": "array"},
                        "total_value_usd": {"type": "number"},
                        "confidence_score": {"type": "number"},
                        "quotes": {"type": "array"},
                        "rules": {
                            "type": "object",
                            "properties": {
                                "max_single_swap_pct": {"type": "number", "default": 15.0},
                                "max_slippage_pct": {"type": "number", "default": 1.0},
                                "min_confidence": {"type": "number", "default": 0.6}
                            }
                        }
                    }
                }),
                returns_schema: json!({
                    "passed": "bool",
                    "blocked_reasons": "array",
                    "checks": "array"
                }),
            },
            ActionDefinition {
                id: "emit_webhook".to_string(),
                name: "Emit Webhook".to_string(),
                description: "POST a payload to another workflow's webhook endpoint.".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["target_workflow_id", "payload"],
                    "properties": {
                        "target_workflow_id": {"type": "string", "description": "UUID of the target workflow"},
                        "payload": {"type": "object"},
                        "secret": {"type": "string", "description": "If set, HMAC-SHA256 sign the body"},
                        "base_url": {"type": "string", "description": "Override API base URL"}
                    }
                }),
                returns_schema: json!({
                    "status": "number",
                    "target_workflow_id": "string",
                    "target_run_id": "string"
                }),
            },
            ActionDefinition {
                id: "require_approval".to_string(),
                name: "Human-In-The-Loop Approval".to_string(),
                description: "Pauses the workflow run, awaits approval via POST /v1/runs/:id/approve.".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "properties": {
                        "message": {"type": "string", "description": "Prompt shown to the approver"},
                        "timeout_secs": {"type": "integer", "description": "Auto-reject after this many seconds; 0 = no timeout"}
                    }
                }),
                returns_schema: json!({"approval_required": "bool", "message": "string"}),
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
            "run_workflow" => self.run_workflow(params).await,
            "delta_calc" => self.delta_calc(params),
            "guard_rails" => self.guard_rails(params),
            "emit_webhook" => self.emit_webhook(params).await,
            "require_approval" => {
                let message = params["message"]
                    .as_str()
                    .unwrap_or("approval required")
                    .to_string();
                Ok(json!({
                    "__pause__": true,
                    "approval_required": true,
                    "message": message
                }))
            }
            other => Err(PluginError::UnknownAction(other.to_string())),
        }
    }
}

impl SolhubPlugin {
    // -----------------------------------------------------------------------
    // delta_calc
    // -----------------------------------------------------------------------

    fn delta_calc(&self, params: &Value) -> Result<Value, PluginError> {
        let current = params["current"]
            .as_object()
            .ok_or_else(|| PluginError::InvalidParam("current must be an object".into()))?;
        let target = params["target"]
            .as_object()
            .ok_or_else(|| PluginError::InvalidParam("target must be an object".into()))?;
        let total_value_usd = params["total_value_usd"]
            .as_f64()
            .ok_or_else(|| PluginError::InvalidParam("total_value_usd must be a number".into()))?;

        // Collect all mints mentioned in either map.
        let mut all_mints: std::collections::HashSet<String> = std::collections::HashSet::new();
        all_mints.extend(current.keys().cloned());
        all_mints.extend(target.keys().cloned());

        // Compute per-mint delta (target - current).
        let mut deltas: Vec<(String, f64)> = all_mints
            .into_iter()
            .map(|mint| {
                let cur = current
                    .get(&mint)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let tgt = target.get(&mint).and_then(|v| v.as_f64()).unwrap_or(0.0);
                (mint, tgt - cur)
            })
            .collect();

        // Split into sells (delta < 0) and buys (delta > 0).
        let mut sellers: Vec<(String, f64)> = deltas
            .iter()
            .filter(|(_, d)| *d < -0.0001)
            .map(|(m, d)| (m.clone(), d.abs()))
            .collect();
        let mut buyers: Vec<(String, f64)> = deltas
            .iter_mut()
            .filter(|(_, d)| *d > 0.0001)
            .map(|(m, d)| (m.clone(), *d))
            .collect();

        // Sort: largest first so we can pop from the back cheaply.
        sellers.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        buyers.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let mut swaps: Vec<Value> = Vec::new();
        let tolerance = 0.0001f64;

        while let (Some(sell), Some(buy)) = (sellers.last_mut(), buyers.last_mut()) {
            let matched_pct = sell.1.min(buy.1);
            if matched_pct < tolerance {
                break;
            }
            let amount_usd = matched_pct * total_value_usd / 100.0;
            swaps.push(json!({
                "from": sell.0,
                "to":   buy.0,
                "amount_usd":  (amount_usd * 1e6).round() / 1e6,
                "delta_pct":   (matched_pct * 1e6).round() / 1e6,
            }));
            sell.1 -= matched_pct;
            buy.1 -= matched_pct;
            if sell.1 < tolerance {
                sellers.pop();
            }
            if buy.1 < tolerance {
                buyers.pop();
            }
        }

        let total_swap_value_usd: f64 = swaps
            .iter()
            .filter_map(|s| s["amount_usd"].as_f64())
            .sum();

        Ok(json!({
            "swaps": swaps,
            "total_swap_value_usd": (total_swap_value_usd * 1e6).round() / 1e6,
        }))
    }

    // -----------------------------------------------------------------------
    // guard_rails
    // -----------------------------------------------------------------------

    fn guard_rails(&self, params: &Value) -> Result<Value, PluginError> {
        let swaps = params["swaps"]
            .as_array()
            .ok_or_else(|| PluginError::InvalidParam("swaps must be an array".into()))?;
        let total_value_usd = params["total_value_usd"]
            .as_f64()
            .ok_or_else(|| PluginError::InvalidParam("total_value_usd must be a number".into()))?;
        let confidence_score = params["confidence_score"]
            .as_f64()
            .ok_or_else(|| PluginError::InvalidParam("confidence_score must be a number".into()))?;

        let rules = &params["rules"];
        let max_single_swap_pct = rules["max_single_swap_pct"].as_f64().unwrap_or(15.0);
        let max_slippage_pct = rules["max_slippage_pct"].as_f64().unwrap_or(1.0);
        let min_confidence = rules["min_confidence"].as_f64().unwrap_or(0.6);

        let mut checks: Vec<Value> = Vec::new();
        let mut blocked_reasons: Vec<String> = Vec::new();

        // Check 1: single swap size.
        let mut size_passed = true;
        let mut size_detail = String::from("all swap sizes within limit");
        for swap in swaps {
            if let Some(amount) = swap["amount_usd"].as_f64() {
                let pct = if total_value_usd > 0.0 {
                    amount / total_value_usd * 100.0
                } else {
                    0.0
                };
                if pct > max_single_swap_pct {
                    size_passed = false;
                    size_detail = format!(
                        "swap from {} to {} is {:.2}% of portfolio (limit {:.2}%)",
                        swap["from"].as_str().unwrap_or("?"),
                        swap["to"].as_str().unwrap_or("?"),
                        pct,
                        max_single_swap_pct
                    );
                    blocked_reasons.push(size_detail.clone());
                    break;
                }
            }
        }
        checks.push(json!({
            "name": "single_swap_size",
            "passed": size_passed,
            "detail": size_detail,
        }));

        // Check 2: slippage from jupiter quotes (optional).
        let mut slippage_passed = true;
        let mut slippage_detail = String::from("no quotes provided or all within limit");
        if let Some(quotes) = params["quotes"].as_array() {
            for q in quotes {
                if let Some(impact_str) = q["priceImpactPct"].as_str() {
                    let impact: f64 = impact_str.parse().unwrap_or(0.0);
                    if impact > max_slippage_pct {
                        slippage_passed = false;
                        slippage_detail = format!(
                            "price impact {:.4}% exceeds limit {:.2}%",
                            impact, max_slippage_pct
                        );
                        blocked_reasons.push(slippage_detail.clone());
                        break;
                    }
                } else if let Some(impact) = q["priceImpactPct"].as_f64() {
                    if impact > max_slippage_pct {
                        slippage_passed = false;
                        slippage_detail = format!(
                            "price impact {:.4}% exceeds limit {:.2}%",
                            impact, max_slippage_pct
                        );
                        blocked_reasons.push(slippage_detail.clone());
                        break;
                    }
                }
            }
        }
        checks.push(json!({
            "name": "slippage",
            "passed": slippage_passed,
            "detail": slippage_detail,
        }));

        // Check 3: confidence score.
        let confidence_passed = confidence_score >= min_confidence;
        let confidence_detail = if confidence_passed {
            format!("confidence {:.3} >= minimum {:.3}", confidence_score, min_confidence)
        } else {
            let reason = format!(
                "confidence {:.3} below minimum {:.3}",
                confidence_score, min_confidence
            );
            blocked_reasons.push(reason.clone());
            reason
        };
        checks.push(json!({
            "name": "confidence",
            "passed": confidence_passed,
            "detail": confidence_detail,
        }));

        Ok(json!({
            "passed": blocked_reasons.is_empty(),
            "blocked_reasons": blocked_reasons,
            "checks": checks,
        }))
    }

    // -----------------------------------------------------------------------
    // emit_webhook
    // -----------------------------------------------------------------------

    async fn emit_webhook(&self, params: &Value) -> Result<Value, PluginError> {
        let target_workflow_id = params["target_workflow_id"]
            .as_str()
            .ok_or_else(|| PluginError::InvalidParam("target_workflow_id".into()))?;
        // Validate it is a UUID.
        Uuid::from_str(target_workflow_id)
            .map_err(|_| PluginError::InvalidParam("target_workflow_id: not a uuid".into()))?;

        let payload = &params["payload"];
        if payload.is_null() {
            return Err(PluginError::InvalidParam("payload is required".into()));
        }

        let base_url = params["base_url"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                std::env::var("SOLHUB_API_BASE")
                    .unwrap_or_else(|_| "http://localhost:8080".to_string())
            });

        let url = format!("{}/v1/webhooks/{}", base_url.trim_end_matches('/'), target_workflow_id);
        let body = serde_json::to_string(&json!({"trigger_data": payload}))
            .map_err(|e| PluginError::Other(e.to_string()))?;

        let mut req = reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(secret) = params["secret"].as_str() {
            type HmacSha256 = Hmac<Sha256>;
            let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                .map_err(|e| PluginError::Other(e.to_string()))?;
            mac.update(body.as_bytes());
            let sig = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
            req = req.header("X-SK-Signature", sig);
        }

        let resp = req
            .body(body)
            .send()
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let resp_body: Value = resp
            .json()
            .await
            .unwrap_or_else(|_| json!({"error": "non-json response"}));

        Ok(json!({
            "status": status,
            "target_workflow_id": target_workflow_id,
            "target_run_id": resp_body.get("run_id").cloned().unwrap_or(Value::Null),
        }))
    }

    // -----------------------------------------------------------------------
    // run_workflow (existing)
    // -----------------------------------------------------------------------

    async fn run_workflow(&self, params: &Value) -> Result<Value, PluginError> {
        let wf_id_str = params["workflow_id"]
            .as_str()
            .ok_or_else(|| PluginError::InvalidParam("workflow_id".into()))?;
        let wf_id = Uuid::from_str(wf_id_str)
            .map_err(|_| PluginError::InvalidParam("workflow_id: not a uuid".into()))?;

        let depth = params["depth"].as_u64().unwrap_or(0) as usize;
        if depth >= self.max_depth {
            return Err(PluginError::Other(format!(
                "sub-workflow depth limit {} reached",
                self.max_depth
            )));
        }

        let wf = self
            .db
            .get_workflow(wf_id)
            .await
            .map_err(|e| PluginError::Other(e.to_string()))?
            .ok_or_else(|| PluginError::Other(format!("workflow {} not found", wf_id)))?;

        let parent_run_id = params["parent_run_id"].as_str().unwrap_or("");

        let child_run = self
            .db
            .create_run(NewRun {
                workflow_id: wf_id,
                org_id: wf.org_id,
                triggered_by: format!("parent:{}", parent_run_id),
            })
            .await
            .map_err(|e| PluginError::Other(e.to_string()))?;

        let timeout = std::time::Duration::from_secs(
            params["timeout_secs"]
                .as_u64()
                .unwrap_or(self.default_timeout_secs),
        );
        let started = std::time::Instant::now();
        loop {
            if started.elapsed() > timeout {
                return Err(PluginError::Other(format!(
                    "sub-workflow timed out after {:?}",
                    timeout
                )));
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let r = self
                .db
                .get_run(child_run.run_id)
                .await
                .map_err(|e| PluginError::Other(e.to_string()))?
                .ok_or_else(|| PluginError::Other("child run vanished".into()))?;
            if matches!(r.status.as_str(), "Confirmed" | "Failed" | "Skipped") {
                return Ok(json!({
                    "child_run_id": r.run_id.to_string(),
                    "status": r.status,
                    "steps_log": r.steps_log,
                    "signature": r.signature,
                    "error": r.error_message,
                }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn make_plugin(timeout_secs: u64) -> SolhubPlugin {
        let db = Db::connect_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        SolhubPlugin {
            db,
            max_depth: 3,
            default_timeout_secs: timeout_secs,
        }
    }

    // -----------------------------------------------------------------------
    // delta_calc tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn delta_calc_zero_when_balanced() {
        let plugin = make_plugin(1).await;
        let rpc = RpcClient::new("http://localhost:8899".to_string());
        let mint_a = "So11111111111111111111111111111111111111112";
        let mint_b = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let params = json!({
            "current": { mint_a: 50.0, mint_b: 50.0 },
            "target":  { mint_a: 50.0, mint_b: 50.0 },
            "total_value_usd": 10000.0
        });
        let result = plugin.read("delta_calc", &params, &rpc).await.unwrap();
        let swaps = result["swaps"].as_array().unwrap();
        assert!(swaps.is_empty(), "no swaps needed when already balanced");
        assert_eq!(result["total_swap_value_usd"].as_f64().unwrap(), 0.0);
    }

    #[tokio::test]
    async fn delta_calc_single_swap() {
        let plugin = make_plugin(1).await;
        let rpc = RpcClient::new("http://localhost:8899".to_string());
        let sol = "So11111111111111111111111111111111111111112";
        let usdc = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        // Move 20% from SOL to USDC
        let params = json!({
            "current": { sol: 70.0, usdc: 30.0 },
            "target":  { sol: 50.0, usdc: 50.0 },
            "total_value_usd": 1000.0
        });
        let result = plugin.read("delta_calc", &params, &rpc).await.unwrap();
        let swaps = result["swaps"].as_array().unwrap();
        assert_eq!(swaps.len(), 1);
        assert_eq!(swaps[0]["from"].as_str().unwrap(), sol);
        assert_eq!(swaps[0]["to"].as_str().unwrap(), usdc);
        let amount = swaps[0]["amount_usd"].as_f64().unwrap();
        assert!((amount - 200.0).abs() < 0.01, "amount should be ~200 USD, got {amount}");
    }

    #[tokio::test]
    async fn delta_calc_pairs_largest_with_largest() {
        let plugin = make_plugin(1).await;
        let rpc = RpcClient::new("http://localhost:8899".to_string());
        // 3 mints: SOL 60→30 (-30%), USDC 10→40 (+30%), BTC 30→30 (0%)
        let sol = "So11111111111111111111111111111111111111112";
        let usdc = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let btc = "9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E";
        let params = json!({
            "current": { sol: 60.0, usdc: 10.0, btc: 30.0 },
            "target":  { sol: 30.0, usdc: 40.0, btc: 30.0 },
            "total_value_usd": 1000.0
        });
        let result = plugin.read("delta_calc", &params, &rpc).await.unwrap();
        let swaps = result["swaps"].as_array().unwrap();
        // Net: SOL -30 pct → USDC +30 pct.  Should be exactly 1 swap.
        assert_eq!(swaps.len(), 1);
        assert_eq!(swaps[0]["from"].as_str().unwrap(), sol);
        assert_eq!(swaps[0]["to"].as_str().unwrap(), usdc);
        let amount = swaps[0]["amount_usd"].as_f64().unwrap();
        assert!((amount - 300.0).abs() < 0.01, "amount should be ~300 USD, got {amount}");
    }

    // -----------------------------------------------------------------------
    // guard_rails tests
    // -----------------------------------------------------------------------

    fn make_swap(from: &str, to: &str, amount_usd: f64) -> Value {
        json!({ "from": from, "to": to, "amount_usd": amount_usd, "delta_pct": 10.0 })
    }

    #[tokio::test]
    async fn guard_rails_passes_on_clean_input() {
        let plugin = make_plugin(1).await;
        let rpc = RpcClient::new("http://localhost:8899".to_string());
        let params = json!({
            "swaps": [make_swap("SOL", "USDC", 100.0)],
            "total_value_usd": 1000.0,
            "confidence_score": 0.9,
            "rules": { "max_single_swap_pct": 20.0, "max_slippage_pct": 1.0, "min_confidence": 0.6 }
        });
        let result = plugin.read("guard_rails", &params, &rpc).await.unwrap();
        assert_eq!(result["passed"], true);
        let reasons = result["blocked_reasons"].as_array().unwrap();
        assert!(reasons.is_empty());
    }

    #[tokio::test]
    async fn guard_rails_blocks_oversized_swap() {
        let plugin = make_plugin(1).await;
        let rpc = RpcClient::new("http://localhost:8899".to_string());
        // Swap is 20% of portfolio, limit is 15%.
        let params = json!({
            "swaps": [make_swap("SOL", "USDC", 200.0)],
            "total_value_usd": 1000.0,
            "confidence_score": 0.9,
            "rules": { "max_single_swap_pct": 15.0, "max_slippage_pct": 1.0, "min_confidence": 0.6 }
        });
        let result = plugin.read("guard_rails", &params, &rpc).await.unwrap();
        assert_eq!(result["passed"], false);
        let reasons = result["blocked_reasons"].as_array().unwrap();
        assert!(!reasons.is_empty());
        let checks = result["checks"].as_array().unwrap();
        let size_check = checks.iter().find(|c| c["name"] == "single_swap_size").unwrap();
        assert_eq!(size_check["passed"], false);
    }

    #[tokio::test]
    async fn guard_rails_blocks_high_slippage() {
        let plugin = make_plugin(1).await;
        let rpc = RpcClient::new("http://localhost:8899".to_string());
        let params = json!({
            "swaps": [make_swap("SOL", "USDC", 50.0)],
            "total_value_usd": 1000.0,
            "confidence_score": 0.9,
            "quotes": [{ "priceImpactPct": "2.5" }],
            "rules": { "max_single_swap_pct": 15.0, "max_slippage_pct": 1.0, "min_confidence": 0.6 }
        });
        let result = plugin.read("guard_rails", &params, &rpc).await.unwrap();
        assert_eq!(result["passed"], false);
        let checks = result["checks"].as_array().unwrap();
        let slip_check = checks.iter().find(|c| c["name"] == "slippage").unwrap();
        assert_eq!(slip_check["passed"], false);
    }

    #[tokio::test]
    async fn guard_rails_blocks_low_confidence() {
        let plugin = make_plugin(1).await;
        let rpc = RpcClient::new("http://localhost:8899".to_string());
        let params = json!({
            "swaps": [make_swap("SOL", "USDC", 50.0)],
            "total_value_usd": 1000.0,
            "confidence_score": 0.4,
            "rules": { "max_single_swap_pct": 15.0, "max_slippage_pct": 1.0, "min_confidence": 0.6 }
        });
        let result = plugin.read("guard_rails", &params, &rpc).await.unwrap();
        assert_eq!(result["passed"], false);
        let checks = result["checks"].as_array().unwrap();
        let conf_check = checks.iter().find(|c| c["name"] == "confidence").unwrap();
        assert_eq!(conf_check["passed"], false);
    }

    // -----------------------------------------------------------------------
    // emit_webhook tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn emit_webhook_posts_to_correct_url() {
        let plugin = make_plugin(1).await;
        let rpc = RpcClient::new("http://localhost:8899".to_string());

        let mut server = mockito::Server::new_async().await;
        let wf_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();

        let _mock = server
            .mock("POST", format!("/v1/webhooks/{}", wf_id).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(r#"{{"run_id":"{}","status":"Pending"}}"#, run_id))
            .create_async()
            .await;

        let params = json!({
            "target_workflow_id": wf_id.to_string(),
            "payload": { "price": 42.0 },
            "base_url": server.url(),
        });
        let result = plugin.read("emit_webhook", &params, &rpc).await.unwrap();
        assert_eq!(result["status"].as_u64().unwrap(), 200);
        assert_eq!(result["target_workflow_id"].as_str().unwrap(), wf_id.to_string());
        assert_eq!(result["target_run_id"].as_str().unwrap(), run_id.to_string());
    }

    #[tokio::test]
    async fn emit_webhook_signs_with_hmac_when_secret_provided() {
        let plugin = make_plugin(1).await;
        let rpc = RpcClient::new("http://localhost:8899".to_string());

        let mut server = mockito::Server::new_async().await;
        let wf_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();

        let _mock = server
            .mock("POST", format!("/v1/webhooks/{}", wf_id).as_str())
            .match_header("X-SK-Signature", mockito::Matcher::Regex("^sha256=".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(r#"{{"run_id":"{}","status":"Pending"}}"#, run_id))
            .create_async()
            .await;

        let params = json!({
            "target_workflow_id": wf_id.to_string(),
            "payload": { "price": 42.0 },
            "secret": "mysecret",
            "base_url": server.url(),
        });
        let result = plugin.read("emit_webhook", &params, &rpc).await.unwrap();
        assert_eq!(result["status"].as_u64().unwrap(), 200);
    }

    // -----------------------------------------------------------------------
    // require_approval test
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn require_approval_returns_pause_sentinel() {
        let plugin = make_plugin(1).await;
        let rpc = RpcClient::new("http://localhost:8899".to_string());
        let params = json!({ "message": "please review before trading" });
        let result = plugin.read("require_approval", &params, &rpc).await.unwrap();
        assert_eq!(result["__pause__"], true);
        assert_eq!(result["approval_required"], true);
        assert_eq!(result["message"].as_str().unwrap(), "please review before trading");
    }

    #[tokio::test]
    async fn actions_includes_run_workflow() {
        let plugin = make_plugin(60).await;
        let actions = plugin.actions();
        assert_eq!(actions.len(), 5, "plugin should expose 5 actions");

        let run_wf = actions.iter().find(|a| a.id == "run_workflow").unwrap();
        assert_eq!(run_wf.action_type, ActionType::ReadOnly);
        let required = run_wf.params_schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "workflow_id"));

        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"delta_calc"));
        assert!(ids.contains(&"guard_rails"));
        assert!(ids.contains(&"emit_webhook"));
        assert!(ids.contains(&"require_approval"));
    }

    #[tokio::test]
    async fn run_workflow_rejects_non_uuid_workflow_id() {
        let plugin = make_plugin(1).await;
        let rpc = RpcClient::new("http://localhost:8899".to_string());
        let params = json!({"workflow_id": "not-a-uuid"});

        let err = plugin.read("run_workflow", &params, &rpc).await.unwrap_err();
        assert!(
            matches!(err, PluginError::InvalidParam(ref s) if s.contains("workflow_id")),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn run_workflow_rejects_when_depth_exceeds_limit() {
        let plugin = make_plugin(1).await;
        let rpc = RpcClient::new("http://localhost:8899".to_string());
        let params = json!({
            "workflow_id": Uuid::new_v4().to_string(),
            "depth": 3u64,
        });

        let err = plugin.read("run_workflow", &params, &rpc).await.unwrap_err();
        assert!(
            matches!(err, PluginError::Other(ref s) if s.contains("depth limit")),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn run_workflow_returns_timeout_when_child_doesnt_complete() {
        // Create an in-memory DB, migrate, insert a workflow so the DB
        // lookup succeeds, then let the polling loop time out after 1 second.
        let db = Db::connect_in_memory().await.unwrap();
        db.migrate().await.unwrap();

        let org = db.create_org("test-org", None).await.unwrap();
        let wf = db
            .create_workflow(db::NewWorkflow {
                org_id: org.id,
                name: "sub-wf".into(),
                trigger_type: "manual".into(),
                trigger_config: serde_json::json!({}),
                steps: serde_json::json!([]),
                is_public: false,
                fee_per_exec_usdc: None,
            })
            .await
            .unwrap();

        let plugin = SolhubPlugin {
            db,
            max_depth: 3,
            default_timeout_secs: 1,
        };
        let rpc = RpcClient::new("http://localhost:8899".to_string());
        let params = json!({
            "workflow_id": wf.id.to_string(),
            "timeout_secs": 1u64,
        });

        let err = plugin
            .read("run_workflow", &params, &rpc)
            .await
            .unwrap_err();
        assert!(
            matches!(err, PluginError::Other(ref s) if s.contains("timed out")),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn run_workflow_unknown_action_returns_error() {
        let plugin = make_plugin(1).await;
        let rpc = RpcClient::new("http://localhost:8899".to_string());

        let err = plugin
            .read("nonexistent_action", &json!({}), &rpc)
            .await
            .unwrap_err();
        assert!(matches!(err, PluginError::UnknownAction(_)));
    }
}
