use crate::plugins::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};
use async_trait::async_trait;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

pub struct TelegramPlugin {
    http: reqwest::Client,
    base_url: String,
}

impl TelegramPlugin {
    pub fn new() -> Self {
        Self::with_base_url("https://api.telegram.org")
    }

    pub fn with_base_url(url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: url.into(),
        }
    }
}

impl Default for TelegramPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolanaKeeperPlugin for TelegramPlugin {
    fn id(&self) -> &'static str {
        "notify.telegram"
    }

    fn name(&self) -> &'static str {
        "Telegram"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![ActionDefinition {
            id: "send_message".to_string(),
            name: "Send Message".to_string(),
            description: "Send a Telegram message via bot".to_string(),
            action_type: ActionType::Notification,
            params_schema: json!({
                "type": "object",
                "required": ["chat_id", "text"],
                "properties": {
                    "chat_id": {"type": "string", "description": "Telegram chat or channel ID"},
                    "text": {"type": "string", "description": "Message text (Markdown supported)"},
                    "bot_token": {"type": "string", "description": "Bot token override (uses TELEGRAM_BOT_TOKEN env var if not provided)"}
                }
            }),
            returns_schema: json!({
                "type": "object",
                "properties": {
                    "ok": {"type": "boolean"},
                    "message_id": {"type": "integer"}
                }
            }),
        }]
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
        _action: &str,
        _params: &Value,
        _rpc: &RpcClient,
    ) -> Result<Value, PluginError> {
        Err(PluginError::NotSupported)
    }

    async fn notify(&self, action: &str, params: &Value) -> Result<Value, PluginError> {
        match action {
            "send_message" => {
                let chat_id = params["chat_id"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("chat_id".into()))?;
                let text = params["text"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("text".into()))?;
                let bot_token = if let Some(t) = params["bot_token"].as_str() {
                    t.to_string()
                } else {
                    std::env::var("TELEGRAM_BOT_TOKEN").map_err(|_| {
                        PluginError::InvalidParam(
                            "bot_token not in params and TELEGRAM_BOT_TOKEN not set".into(),
                        )
                    })?
                };

                let url = format!("{}/bot{}/sendMessage", self.base_url, bot_token);
                let body = json!({
                    "chat_id": chat_id,
                    "text": text,
                    "parse_mode": "Markdown"
                });

                let resp = self
                    .http
                    .post(&url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;

                if !resp.status().is_success() {
                    return Err(PluginError::Network(format!(
                        "Telegram API returned {}",
                        resp.status()
                    )));
                }

                let result: Value = resp
                    .json()
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;

                Ok(json!({
                    "ok": result["ok"],
                    "message_id": result["result"]["message_id"]
                }))
            }
            _ => Err(PluginError::UnknownAction(action.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[test]
    fn actions_includes_send_message() {
        let plugin = TelegramPlugin::new();
        let actions = plugin.actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].id, "send_message");
        assert_eq!(actions[0].action_type, ActionType::Notification);
    }

    #[tokio::test]
    async fn send_message_posts_to_telegram_api() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/bottest-token-123/sendMessage")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ok":true,"result":{"message_id":42,"chat":{"id":12345},"text":"hello"}}"#)
            .create_async()
            .await;

        let plugin = TelegramPlugin::with_base_url(server.url());
        let params = json!({
            "chat_id": "12345",
            "text": "hello",
            "bot_token": "test-token-123"
        });

        let result = plugin.notify("send_message", &params).await;
        assert!(result.is_ok(), "expected ok, got {:?}", result);
        let val = result.unwrap();
        assert_eq!(val["ok"], true);
        assert_eq!(val["message_id"], 42);

        mock.assert_async().await;
    }
}
