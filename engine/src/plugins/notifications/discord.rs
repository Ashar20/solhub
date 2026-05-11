use crate::plugins::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};
use async_trait::async_trait;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

pub struct DiscordPlugin {
    http: reqwest::Client,
}

impl DiscordPlugin {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }
}

impl Default for DiscordPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolanaKeeperPlugin for DiscordPlugin {
    fn id(&self) -> &'static str {
        "notify.discord"
    }

    fn name(&self) -> &'static str {
        "Discord"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![
            ActionDefinition {
                id: "send_message".to_string(),
                name: "Send Message".to_string(),
                description: "Send a plain text message to a Discord webhook".to_string(),
                action_type: ActionType::Notification,
                params_schema: json!({
                    "type": "object",
                    "required": ["webhook_url", "content"],
                    "properties": {
                        "webhook_url": {"type": "string", "description": "Discord webhook URL"},
                        "content": {"type": "string", "description": "Message content (max 2000 chars)"}
                    }
                }),
                returns_schema: json!({
                    "type": "object",
                    "properties": {
                        "ok": {"type": "boolean"}
                    }
                }),
            },
            ActionDefinition {
                id: "send_embed".to_string(),
                name: "Send Embed".to_string(),
                description: "Send a rich embed message to a Discord webhook".to_string(),
                action_type: ActionType::Notification,
                params_schema: json!({
                    "type": "object",
                    "required": ["webhook_url", "title", "description"],
                    "properties": {
                        "webhook_url": {"type": "string", "description": "Discord webhook URL"},
                        "title": {"type": "string", "description": "Embed title"},
                        "description": {"type": "string", "description": "Embed description"},
                        "color": {"type": "integer", "description": "Embed sidebar color as decimal integer"}
                    }
                }),
                returns_schema: json!({
                    "type": "object",
                    "properties": {
                        "ok": {"type": "boolean"}
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
        _action: &str,
        _params: &Value,
        _rpc: &RpcClient,
    ) -> Result<Value, PluginError> {
        Err(PluginError::NotSupported)
    }

    async fn notify(&self, action: &str, params: &Value) -> Result<Value, PluginError> {
        match action {
            "send_message" => {
                let webhook_url = params["webhook_url"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("webhook_url".into()))?;
                let content = params["content"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("content".into()))?;

                let body = json!({ "content": content });
                let resp = self
                    .http
                    .post(webhook_url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;

                if !resp.status().is_success() {
                    return Err(PluginError::Network(format!(
                        "Discord webhook returned {}",
                        resp.status()
                    )));
                }
                Ok(json!({ "ok": true }))
            }
            "send_embed" => {
                let webhook_url = params["webhook_url"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("webhook_url".into()))?;
                let title = params["title"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("title".into()))?;
                let description = params["description"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("description".into()))?;

                let mut embed = json!({
                    "title": title,
                    "description": description
                });
                if let Some(color) = params["color"].as_i64() {
                    embed["color"] = json!(color);
                }

                let body = json!({ "embeds": [embed] });
                let resp = self
                    .http
                    .post(webhook_url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;

                if !resp.status().is_success() {
                    return Err(PluginError::Network(format!(
                        "Discord webhook returned {}",
                        resp.status()
                    )));
                }
                Ok(json!({ "ok": true }))
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
    fn actions_returns_send_message_and_send_embed() {
        let plugin = DiscordPlugin::new();
        let actions = plugin.actions();
        assert_eq!(actions.len(), 2);
        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"send_message"));
        assert!(ids.contains(&"send_embed"));
    }

    #[tokio::test]
    async fn send_message_posts_to_webhook() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/webhook/test123")
            .with_status(204)
            .create_async()
            .await;

        let plugin = DiscordPlugin::new();
        let webhook_url = format!("{}/webhook/test123", server.url());
        let params = json!({
            "webhook_url": webhook_url,
            "content": "hello from solhub"
        });

        let result = plugin.notify("send_message", &params).await;
        assert!(result.is_ok(), "expected ok, got {:?}", result);
        assert_eq!(result.unwrap()["ok"], true);

        mock.assert_async().await;
    }
}
