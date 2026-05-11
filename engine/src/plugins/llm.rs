use super::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};
use async_trait::async_trait;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

/// Which LLM provider to use.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LlmProvider {
    #[default]
    OpenAi,
    Anthropic,
}

impl LlmProvider {
    fn from_str(s: &str) -> Self {
        match s {
            "anthropic" => Self::Anthropic,
            _ => Self::OpenAi,
        }
    }
}

pub struct LlmPlugin {
    pub http: reqwest::Client,
    pub base_url: String,
    pub api_key_env: String,
    pub default_model: String,
    pub default_provider: LlmProvider,
    /// Separate base URL for the Anthropic API (for testability).
    pub anthropic_base_url: String,
}

impl LlmPlugin {
    pub fn new() -> Self {
        Self::with_base_url("https://api.openai.com/v1")
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
            api_key_env: "OPENAI_API_KEY".to_string(),
            default_model: "gpt-4o-mini".to_string(),
            default_provider: LlmProvider::OpenAi,
            anthropic_base_url: "https://api.anthropic.com/v1".to_string(),
        }
    }

    pub fn with_anthropic_base_url(mut self, url: impl Into<String>) -> Self {
        self.anthropic_base_url = url.into();
        self
    }

    fn api_key(&self) -> Result<String, PluginError> {
        std::env::var(&self.api_key_env)
            .map_err(|_| PluginError::Other(format!("missing env var {}", self.api_key_env)))
    }

    fn anthropic_api_key(&self) -> Result<String, PluginError> {
        std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| PluginError::Other("missing env var ANTHROPIC_API_KEY".to_string()))
    }
}

impl Default for LlmPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolanaKeeperPlugin for LlmPlugin {
    fn id(&self) -> &'static str {
        "llm"
    }

    fn name(&self) -> &'static str {
        "LLM"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![
            ActionDefinition {
                id: "complete".to_string(),
                name: "Chat Completion".to_string(),
                description: "Send a prompt to the LLM and return the response text. Supports openai and anthropic providers.".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["prompt"],
                    "properties": {
                        "prompt": {"type": "string"},
                        "system": {"type": "string", "description": "Optional system prompt"},
                        "model": {"type": "string", "description": "Defaults to provider default model"},
                        "max_tokens": {"type": "integer", "default": 512},
                        "temperature": {"type": "number", "default": 0.2},
                        "provider": {
                            "type": "string",
                            "enum": ["openai", "anthropic"],
                            "description": "LLM provider to use (defaults to openai)"
                        }
                    }
                }),
                returns_schema: json!({"text": "string", "model": "string", "usage": "object"}),
            },
            ActionDefinition {
                id: "analyze_sentiment".to_string(),
                name: "Sentiment Analysis".to_string(),
                description: "Analyze sentiment of a text input (returns 'positive', 'neutral', 'negative' + score).".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["text"],
                    "properties": {
                        "text": {"type": "string"},
                        "context": {"type": "string", "description": "Optional context (e.g. 'crypto market news')"}
                    }
                }),
                returns_schema: json!({"sentiment": "string", "score": "number", "explanation": "string"}),
            },
            ActionDefinition {
                id: "recommend_rebalance".to_string(),
                name: "Recommend Portfolio Rebalance".to_string(),
                description: "Ask an LLM to analyze a portfolio snapshot + market signals and return a JSON rebalance recommendation.".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["portfolio"],
                    "properties": {
                        "portfolio": {
                            "type": "object",
                            "description": "Portfolio snapshot JSON (from portfolio.snapshot)"
                        },
                        "signals": {
                            "type": "object",
                            "description": "Optional market signals: {news?, fear_greed?, fng?}"
                        },
                        "risk_profile": {
                            "type": "string",
                            "enum": ["conservative", "balanced", "aggressive"],
                            "default": "balanced"
                        },
                        "model": {"type": "string"},
                        "provider": {
                            "type": "string",
                            "enum": ["openai", "anthropic"],
                            "default": "openai"
                        }
                    }
                }),
                returns_schema: json!({
                    "confidence": "integer",
                    "target_weights": "object",
                    "reasoning": "string",
                    "triggered_by": "string",
                    "timestamp": "string"
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
        _rpc: &RpcClient,
    ) -> Result<Value, PluginError> {
        match action {
            "complete" => self.complete(params).await,
            "analyze_sentiment" => self.analyze_sentiment(params).await,
            "recommend_rebalance" => self.recommend_rebalance(params).await,
            other => Err(PluginError::UnknownAction(other.to_string())),
        }
    }
}

impl LlmPlugin {
    async fn complete(&self, params: &Value) -> Result<Value, PluginError> {
        // Determine provider: param > default_provider
        let provider = params["provider"]
            .as_str()
            .map(LlmProvider::from_str)
            .unwrap_or_else(|| match &self.default_provider {
                LlmProvider::Anthropic => LlmProvider::Anthropic,
                LlmProvider::OpenAi => LlmProvider::OpenAi,
            });

        match provider {
            LlmProvider::Anthropic => self.complete_anthropic(params).await,
            LlmProvider::OpenAi => self.complete_openai(params).await,
        }
    }

    async fn complete_openai(&self, params: &Value) -> Result<Value, PluginError> {
        let prompt = params["prompt"]
            .as_str()
            .ok_or_else(|| PluginError::InvalidParam("prompt".into()))?;
        let system = params["system"].as_str();
        let model = params["model"].as_str().unwrap_or(&self.default_model);
        let max_tokens = params["max_tokens"].as_u64().unwrap_or(512);
        let temperature = params["temperature"].as_f64().unwrap_or(0.2);
        let api_key = self.api_key()?;

        let mut messages = Vec::new();
        if let Some(s) = system {
            messages.push(json!({"role": "system", "content": s}));
        }
        messages.push(json!({"role": "user", "content": prompt}));

        let body = json!({
            "model": model,
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": temperature,
        });

        let url = format!("{}/chat/completions", self.base_url);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            return Err(PluginError::Network(format!("openai {}: {}", status, txt)));
        }
        let json: Value = resp
            .json()
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?;
        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        Ok(json!({
            "text": text,
            "model": json["model"],
            "usage": json["usage"],
        }))
    }

    async fn complete_anthropic(&self, params: &Value) -> Result<Value, PluginError> {
        let prompt = params["prompt"]
            .as_str()
            .ok_or_else(|| PluginError::InvalidParam("prompt".into()))?;
        let system = params["system"].as_str();
        let default_model = "claude-sonnet-4-5";
        let model = params["model"].as_str().unwrap_or(default_model);
        let max_tokens = params["max_tokens"].as_u64().unwrap_or(512);
        let api_key = self.anthropic_api_key()?;

        let mut body = json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": max_tokens,
        });
        if let Some(s) = system {
            body.as_object_mut()
                .unwrap()
                .insert("system".to_string(), json!(s));
        }

        let url = format!("{}/messages", self.anthropic_base_url);
        let resp = self
            .http
            .post(&url)
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            return Err(PluginError::Network(format!("anthropic {}: {}", status, txt)));
        }
        let json: Value = resp
            .json()
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?;
        // Anthropic response: {"content": [{"type":"text","text":"..."}], "model":"...", "usage":{...}}
        let text = json["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["text"].as_str())
            .unwrap_or("")
            .to_string();
        Ok(json!({
            "text": text,
            "model": json["model"],
            "usage": json["usage"],
        }))
    }

    async fn analyze_sentiment(&self, params: &Value) -> Result<Value, PluginError> {
        let text = params["text"]
            .as_str()
            .ok_or_else(|| PluginError::InvalidParam("text".into()))?;
        let context = params["context"].as_str().unwrap_or("");
        let prompt = format!(
            "Analyze the sentiment of the following text{}. Respond with ONLY a JSON object: {{\"sentiment\": \"positive\"|\"neutral\"|\"negative\", \"score\": <number from -1 to 1>, \"explanation\": \"<1 sentence>\"}}\n\nText:\n{}",
            if context.is_empty() {
                String::new()
            } else {
                format!(" (context: {})", context)
            },
            text
        );
        let resp = self
            .complete(&json!({
                "prompt": prompt,
                "system": "You are a sentiment analyzer. Return ONLY valid JSON, no markdown fences, no prose.",
                "max_tokens": 200,
                "temperature": 0.0,
            }))
            .await?;
        let raw = resp["text"].as_str().unwrap_or("{}");
        let cleaned = raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        let parsed: Value = serde_json::from_str(cleaned).map_err(|e| {
            PluginError::Other(format!(
                "failed to parse sentiment JSON: {}; raw: {}",
                e, raw
            ))
        })?;
        Ok(parsed)
    }

    async fn recommend_rebalance(&self, params: &Value) -> Result<Value, PluginError> {
        let portfolio = params
            .get("portfolio")
            .ok_or_else(|| PluginError::InvalidParam("portfolio".into()))?;
        let signals = params.get("signals").cloned().unwrap_or(json!({}));
        let risk_profile = params["risk_profile"].as_str().unwrap_or("balanced");
        let provider = params["provider"].as_str().unwrap_or("openai");

        let prompt = format!(
            "You are analyzing a crypto portfolio to recommend a rebalance.\n\
            Risk profile: {risk_profile}\n\
            Portfolio snapshot: {portfolio}\n\
            Market signals: {signals}\n\n\
            Based on the above, output ONLY a JSON object matching exactly this schema:\n\
            {{\"confidence\": <0-100 int>, \"target_weights\": {{\"SYMBOL\": <fraction 0.0-1.0>, ...}}, \"reasoning\": \"<2-3 sentences>\"}}\n\
            target_weights values must sum to 1.0. No markdown fences, no prose.",
            risk_profile = risk_profile,
            portfolio = serde_json::to_string(portfolio).unwrap_or_default(),
            signals = serde_json::to_string(&signals).unwrap_or_default(),
        );

        let completion_params = json!({
            "prompt": prompt,
            "system": "You are a portfolio analyst. Return ONLY valid JSON matching the exact schema. No markdown fences, no prose.",
            "max_tokens": 1024,
            "temperature": 0.2,
            "provider": provider,
        });
        let resp = self.complete(&completion_params).await?;
        let raw = resp["text"].as_str().unwrap_or("{}");

        // Strip markdown fences if present
        let cleaned = raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let parsed: Value = serde_json::from_str(cleaned).map_err(|_| {
            PluginError::Other(format!("failed to parse rebalance JSON from LLM: {}", raw))
        })?;

        // Validate target_weights sum to 1.0 ± 0.01
        if let Some(weights) = parsed["target_weights"].as_object() {
            let sum: f64 = weights.values().filter_map(|v| v.as_f64()).sum();
            if (sum - 1.0).abs() > 0.01 {
                return Err(PluginError::Other(format!(
                    "weights don't sum to 1.0 (got {:.4})",
                    sum
                )));
            }
        }

        // Append metadata
        let mut result = parsed;
        if let Some(obj) = result.as_object_mut() {
            obj.entry("triggered_by").or_insert(json!("cron"));
            obj.entry("timestamp")
                .or_insert(json!(chrono::Utc::now().to_rfc3339()));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[test]
    fn actions_includes_complete_and_analyze() {
        let plugin = LlmPlugin::new();
        let actions = plugin.actions();
        assert_eq!(actions.len(), 3);
        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"complete"));
        assert!(ids.contains(&"analyze_sentiment"));
        assert!(ids.contains(&"recommend_rebalance"));
        let complete = actions.iter().find(|a| a.id == "complete").unwrap();
        assert_eq!(complete.action_type, ActionType::ReadOnly);
        let sentiment = actions.iter().find(|a| a.id == "analyze_sentiment").unwrap();
        assert_eq!(sentiment.action_type, ActionType::ReadOnly);
        let rebalance = actions.iter().find(|a| a.id == "recommend_rebalance").unwrap();
        assert_eq!(rebalance.action_type, ActionType::ReadOnly);
    }

    #[tokio::test]
    async fn complete_calls_openai_with_correct_headers() {
        let mut server = Server::new_async().await;

        let response_body = r#"{
            "id": "chatcmpl-test",
            "choices": [{"message": {"role": "assistant", "content": "Hello world"}, "finish_reason": "stop"}],
            "model": "gpt-4o-mini",
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        }"#;

        let mock = server
            .mock("POST", "/chat/completions")
            .match_header("authorization", mockito::Matcher::Regex(r"^Bearer test-key".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response_body)
            .create_async()
            .await;

        let mut plugin = LlmPlugin::with_base_url(server.url());
        plugin.api_key_env = "TEST_OPENAI_KEY".to_string();
        std::env::set_var("TEST_OPENAI_KEY", "test-key");

        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = json!({"prompt": "Say hello"});

        let result = plugin.read("complete", &params, &rpc).await.expect("complete should succeed");
        assert_eq!(result["text"], "Hello world");
        assert_eq!(result["model"], "gpt-4o-mini");

        mock.assert_async().await;
        std::env::remove_var("TEST_OPENAI_KEY");
    }

    #[tokio::test]
    async fn complete_returns_error_when_api_key_missing() {
        let plugin = LlmPlugin::with_base_url("https://api.openai.com/v1");
        // Use a unique env var name that is definitely not set
        let mut plugin = plugin;
        plugin.api_key_env = "DEFINITELY_NOT_SET_OPENAI_KEY_XYZ123".to_string();
        std::env::remove_var("DEFINITELY_NOT_SET_OPENAI_KEY_XYZ123");

        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = json!({"prompt": "test"});

        let result = plugin.read("complete", &params, &rpc).await;
        assert!(
            matches!(result, Err(PluginError::Other(ref s)) if s.contains("missing env var")),
            "expected Other error about missing env var, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn analyze_sentiment_parses_json_response() {
        let mut server = Server::new_async().await;

        let sentiment_json = r#"{"sentiment":"positive","score":0.8,"explanation":"good"}"#;
        let response_body = format!(
            r#"{{"id":"chatcmpl-test","choices":[{{"message":{{"role":"assistant","content":"{}"}},"finish_reason":"stop"}}],"model":"gpt-4o-mini","usage":{{"prompt_tokens":20,"completion_tokens":15,"total_tokens":35}}}}"#,
            sentiment_json.replace('"', "\\\"")
        );

        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(&response_body)
            .create_async()
            .await;

        let mut plugin = LlmPlugin::with_base_url(server.url());
        plugin.api_key_env = "TEST_OPENAI_SENTIMENT_KEY".to_string();
        std::env::set_var("TEST_OPENAI_SENTIMENT_KEY", "test-key");

        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = json!({"text": "Bitcoin is going to the moon!"});

        let result = plugin
            .read("analyze_sentiment", &params, &rpc)
            .await
            .expect("analyze_sentiment should succeed");

        assert_eq!(result["sentiment"], "positive");
        assert_eq!(result["score"], 0.8);
        assert_eq!(result["explanation"], "good");

        mock.assert_async().await;
        std::env::remove_var("TEST_OPENAI_SENTIMENT_KEY");
    }

    #[test]
    fn complete_action_schema_includes_provider_param() {
        let plugin = LlmPlugin::new();
        let actions = plugin.actions();
        let complete = actions.iter().find(|a| a.id == "complete").unwrap();
        let props = &complete.params_schema["properties"];
        assert!(props.get("provider").is_some(), "provider param must be in schema");
        let provider_enum = &props["provider"]["enum"];
        let values: Vec<&str> = provider_enum
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert!(values.contains(&"openai"));
        assert!(values.contains(&"anthropic"));
    }

    #[tokio::test]
    async fn complete_anthropic_calls_correct_endpoint_with_headers() {
        let mut server = Server::new_async().await;

        let response_body = r#"{
            "id": "msg_test",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Hello from Claude"}],
            "model": "claude-sonnet-4-5",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        }"#;

        let mock = server
            .mock("POST", "/messages")
            .match_header("x-api-key", mockito::Matcher::Regex(r"^anthropic-test-key".to_string()))
            .match_header("anthropic-version", "2023-06-01")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response_body)
            .create_async()
            .await;

        let plugin = LlmPlugin::with_base_url("https://api.openai.com/v1")
            .with_anthropic_base_url(server.url());
        std::env::set_var("ANTHROPIC_API_KEY", "anthropic-test-key");

        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = json!({
            "prompt": "Say hello",
            "provider": "anthropic"
        });

        let result = plugin
            .read("complete", &params, &rpc)
            .await
            .expect("anthropic complete should succeed");

        assert_eq!(result["text"], "Hello from Claude");
        assert_eq!(result["model"], "claude-sonnet-4-5");

        mock.assert_async().await;
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[tokio::test]
    async fn complete_anthropic_missing_api_key_returns_error() {
        let plugin = LlmPlugin::with_base_url("https://api.openai.com/v1");
        std::env::remove_var("ANTHROPIC_API_KEY");

        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = json!({"prompt": "test", "provider": "anthropic"});

        let result = plugin.read("complete", &params, &rpc).await;
        assert!(
            matches!(result, Err(PluginError::Other(ref s)) if s.contains("ANTHROPIC_API_KEY")),
            "expected Other error about ANTHROPIC_API_KEY, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn complete_with_system_prompt_anthropic() {
        let mut server = Server::new_async().await;

        let response_body = r#"{
            "id": "msg_test2",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "I am a helpful assistant"}],
            "model": "claude-sonnet-4-5",
            "usage": {"input_tokens": 15, "output_tokens": 6}
        }"#;

        let mock = server
            .mock("POST", "/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response_body)
            .create_async()
            .await;

        let plugin = LlmPlugin::with_base_url("https://api.openai.com/v1")
            .with_anthropic_base_url(server.url());
        std::env::set_var("ANTHROPIC_API_KEY", "anthropic-test-key");

        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = json!({
            "prompt": "Who are you?",
            "system": "You are a helpful assistant.",
            "provider": "anthropic"
        });

        let result = plugin
            .read("complete", &params, &rpc)
            .await
            .expect("anthropic complete with system should succeed");

        assert_eq!(result["text"], "I am a helpful assistant");
        mock.assert_async().await;
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[tokio::test]
    async fn recommend_rebalance_openai_parses_response() {
        let mut server = Server::new_async().await;

        let rebalance_json = r#"{"confidence":75,"target_weights":{"SOL":0.45,"USDC":0.35,"JUP":0.20},"reasoning":"Market signals are mixed."}"#;
        let response_body = format!(
            r#"{{"id":"chatcmpl-rb","choices":[{{"message":{{"role":"assistant","content":"{}"}},"finish_reason":"stop"}}],"model":"gpt-4o-mini","usage":{{"prompt_tokens":50,"completion_tokens":30,"total_tokens":80}}}}"#,
            rebalance_json.replace('"', "\\\"")
        );

        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(&response_body)
            .create_async()
            .await;

        let mut plugin = LlmPlugin::with_base_url(server.url());
        plugin.api_key_env = "TEST_OPENAI_RB_KEY".to_string();
        std::env::set_var("TEST_OPENAI_RB_KEY", "test-key");

        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = json!({
            "portfolio": {
                "wallet": "So11111111111111111111111111111111111111112",
                "total_usd": 1000.0,
                "holdings": [{"symbol":"SOL","value_usd":500.0}],
                "current_weights": {"SOL": 1.0}
            },
            "provider": "openai"
        });

        let result = plugin
            .read("recommend_rebalance", &params, &rpc)
            .await
            .expect("recommend_rebalance should succeed");

        assert_eq!(result["confidence"], 75);
        let weights = &result["target_weights"];
        assert!((weights["SOL"].as_f64().unwrap() - 0.45).abs() < 1e-9);
        assert!((weights["USDC"].as_f64().unwrap() - 0.35).abs() < 1e-9);
        assert!((weights["JUP"].as_f64().unwrap() - 0.20).abs() < 1e-9);
        assert!(result["reasoning"].as_str().is_some());
        assert!(result["timestamp"].as_str().is_some());

        mock.assert_async().await;
        std::env::remove_var("TEST_OPENAI_RB_KEY");
    }

    #[tokio::test]
    async fn recommend_rebalance_anthropic_parses_response() {
        let mut server = Server::new_async().await;

        let rebalance_json = r#"{"confidence":82,"target_weights":{"SOL":0.50,"USDC":0.30,"JUP":0.20},"reasoning":"Bullish signals support higher SOL allocation."}"#;
        let response_body = format!(
            r#"{{"id":"msg_rb","type":"message","role":"assistant","content":[{{"type":"text","text":"{}"}}],"model":"claude-sonnet-4-5","usage":{{"input_tokens":60,"output_tokens":40}}}}"#,
            rebalance_json.replace('"', "\\\"")
        );

        let mock = server
            .mock("POST", "/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(&response_body)
            .create_async()
            .await;

        let plugin = LlmPlugin::with_base_url("https://api.openai.com/v1")
            .with_anthropic_base_url(server.url());
        std::env::set_var("ANTHROPIC_API_KEY", "anthropic-test-key");

        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = json!({
            "portfolio": {"wallet": "test", "total_usd": 500.0, "holdings": [], "current_weights": {}},
            "signals": {"fear_greed": {"value": 72, "classification": "Greed"}},
            "provider": "anthropic",
            "risk_profile": "aggressive"
        });

        let result = plugin
            .read("recommend_rebalance", &params, &rpc)
            .await
            .expect("recommend_rebalance anthropic should succeed");

        assert_eq!(result["confidence"], 82);
        let weights = &result["target_weights"];
        assert!((weights["SOL"].as_f64().unwrap() - 0.50).abs() < 1e-9);
        assert!(result["reasoning"].as_str().is_some());

        mock.assert_async().await;
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[tokio::test]
    async fn recommend_rebalance_rejects_weights_not_summing_to_one() {
        let mut server = Server::new_async().await;

        // Return weights that don't sum to 1
        let bad_json = r#"{"confidence":50,"target_weights":{"SOL":0.40,"USDC":0.20},"reasoning":"bad weights"}"#;
        let response_body = format!(
            r#"{{"id":"chatcmpl-bad","choices":[{{"message":{{"role":"assistant","content":"{}"}},"finish_reason":"stop"}}],"model":"gpt-4o-mini","usage":{{"prompt_tokens":20,"completion_tokens":10,"total_tokens":30}}}}"#,
            bad_json.replace('"', "\\\"")
        );

        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(&response_body)
            .create_async()
            .await;

        let mut plugin = LlmPlugin::with_base_url(server.url());
        plugin.api_key_env = "TEST_OPENAI_BAD_KEY".to_string();
        std::env::set_var("TEST_OPENAI_BAD_KEY", "test-key");

        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = json!({
            "portfolio": {"wallet": "test", "total_usd": 100.0, "holdings": [], "current_weights": {}},
            "provider": "openai"
        });

        let result = plugin.read("recommend_rebalance", &params, &rpc).await;
        assert!(
            matches!(result, Err(PluginError::Other(ref s)) if s.contains("weights don't sum")),
            "expected weights validation error, got: {:?}",
            result
        );
        std::env::remove_var("TEST_OPENAI_BAD_KEY");
    }
}
