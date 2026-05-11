use super::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};
use async_trait::async_trait;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

pub struct NewsPlugin {
    pub http: reqwest::Client,
    pub base_url: String,
    pub cryptopanic_base_url: String,
}

impl NewsPlugin {
    pub fn new() -> Self {
        Self::with_base_url("https://www.coindesk.com/arc/outboundfeeds/rss/")
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
            cryptopanic_base_url: "https://cryptopanic.com/api/v1".to_string(),
        }
    }

    pub fn with_cryptopanic_url(mut self, url: impl Into<String>) -> Self {
        self.cryptopanic_base_url = url.into();
        self
    }
}

impl Default for NewsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolanaKeeperPlugin for NewsPlugin {
    fn id(&self) -> &'static str {
        "news"
    }

    fn name(&self) -> &'static str {
        "News"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![
            ActionDefinition {
                id: "fetch_headlines".to_string(),
                name: "Fetch Crypto Headlines".to_string(),
                description: "Fetch the latest crypto news headlines from the configured RSS source.".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "properties": {
                        "limit": {"type": "integer", "default": 5},
                        "feed_url": {"type": "string", "description": "Override the RSS URL"}
                    }
                }),
                returns_schema: json!({"items": "array", "count": "integer", "source": "string"}),
            },
            ActionDefinition {
                id: "fetch_url".to_string(),
                name: "Fetch URL Body".to_string(),
                description: "Fetch the body of an arbitrary URL (text).".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["url"],
                    "properties": {
                        "url": {"type": "string"},
                        "max_bytes": {"type": "integer", "default": 65536}
                    }
                }),
                returns_schema: json!({"status": "integer", "body": "string", "url": "string"}),
            },
            ActionDefinition {
                id: "crypto_panic".to_string(),
                name: "CryptoPanic News".to_string(),
                description: "Fetch crypto news posts from CryptoPanic. Uses CRYPTOPANIC_TOKEN env var when available, otherwise uses the public endpoint.".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "properties": {
                        "filter": {
                            "type": "string",
                            "enum": ["rising", "hot", "bullish", "bearish", "important"],
                            "description": "Optional filter for post type"
                        },
                        "currencies": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Optional list of currency symbols, e.g. [\"SOL\", \"BTC\"]"
                        },
                        "limit": {
                            "type": "integer",
                            "default": 10,
                            "description": "Max number of posts to return"
                        }
                    }
                }),
                returns_schema: json!({"posts": "array"}),
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
            "fetch_headlines" => self.fetch_headlines(params).await,
            "fetch_url" => self.fetch_url(params).await,
            "crypto_panic" => self.crypto_panic(params).await,
            other => Err(PluginError::UnknownAction(other.to_string())),
        }
    }
}

impl NewsPlugin {
    async fn fetch_headlines(&self, params: &Value) -> Result<Value, PluginError> {
        let limit = params["limit"].as_u64().unwrap_or(5) as usize;
        let url = params["feed_url"]
            .as_str()
            .unwrap_or(&self.base_url)
            .to_string();
        let xml = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?
            .text()
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?;

        let items =
            parse_rss_items(&xml, limit).map_err(|e| PluginError::Other(format!("rss parse: {}", e)))?;
        let count = items.len();
        Ok(json!({"items": items, "count": count, "source": url}))
    }

    async fn fetch_url(&self, params: &Value) -> Result<Value, PluginError> {
        let url = params["url"]
            .as_str()
            .ok_or_else(|| PluginError::InvalidParam("url".into()))?;
        let max_bytes = params["max_bytes"].as_u64().unwrap_or(65536) as usize;
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?;
        let truncated = if bytes.len() > max_bytes {
            &bytes[..max_bytes]
        } else {
            &bytes[..]
        };
        let body = String::from_utf8_lossy(truncated).to_string();
        Ok(json!({"status": status, "body": body, "url": url}))
    }

    async fn crypto_panic(&self, params: &Value) -> Result<Value, PluginError> {
        let limit = params["limit"].as_u64().unwrap_or(10);
        let filter = params["filter"].as_str();
        let currencies = params["currencies"].as_array();

        // Build query string
        let mut query_parts: Vec<String> = vec!["kind=news".to_string()];

        // Auth: prefer env token, fall back to public
        let auth_token = std::env::var("CRYPTOPANIC_TOKEN").ok();
        if let Some(ref token) = auth_token {
            query_parts.push(format!("auth_token={}", token));
        } else {
            query_parts.push("public=true".to_string());
        }

        if let Some(f) = filter {
            query_parts.push(format!("filter={}", f));
        }
        if let Some(cur_arr) = currencies {
            let cur_str = cur_arr
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(",");
            if !cur_str.is_empty() {
                query_parts.push(format!("currencies={}", cur_str));
            }
        }

        let url = format!(
            "{}/posts/?{}",
            self.cryptopanic_base_url,
            query_parts.join("&")
        );

        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(PluginError::Network(format!(
                "cryptopanic API returned {}",
                resp.status()
            )));
        }
        let body: Value = resp
            .json()
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?;

        let results = body
            .get("results")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();

        let posts: Vec<Value> = results
            .iter()
            .take(limit as usize)
            .map(|post| {
                json!({
                    "title": post["title"],
                    "url": post["url"],
                    "published_at": post["published_at"],
                    "source": post.get("source").and_then(|s| s.get("title")).cloned().unwrap_or(Value::Null)
                })
            })
            .collect();

        Ok(json!({ "posts": posts }))
    }
}

fn parse_rss_items(xml: &str, limit: usize) -> Result<Vec<Value>, String> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut items = Vec::new();
    let mut in_item = false;
    let mut current_tag: Option<String> = None;
    let mut title = String::new();
    let mut link = String::new();
    let mut pub_date = String::new();
    let mut description = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "item" {
                    in_item = true;
                    title.clear();
                    link.clear();
                    pub_date.clear();
                    description.clear();
                } else if in_item {
                    current_tag = Some(name);
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "item" {
                    items.push(json!({
                        "title": title.clone(),
                        "link": link.clone(),
                        "pub_date": pub_date.clone(),
                        "description": description.clone()
                    }));
                    in_item = false;
                    if items.len() >= limit {
                        break;
                    }
                }
                current_tag = None;
            }
            Ok(Event::Text(t)) => {
                if !in_item {
                    continue;
                }
                let text = t.unescape().map(|s| s.into_owned()).unwrap_or_default();
                match current_tag.as_deref() {
                    Some("title") => title = text,
                    Some("link") => link = text,
                    Some("pubDate") => pub_date = text,
                    Some("description") => description = text,
                    _ => {}
                }
            }
            Ok(Event::CData(t)) => {
                if !in_item {
                    continue;
                }
                let text = t.escape().map(|e| e.unescape().map(|s| s.into_owned()).unwrap_or_default()).unwrap_or_default();
                match current_tag.as_deref() {
                    Some("title") => title = text,
                    Some("link") => link = text,
                    Some("pubDate") => pub_date = text,
                    Some("description") => description = text,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.to_string()),
            _ => {}
        }
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    const SAMPLE_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>CoinDesk</title>
    <item>
      <title>Bitcoin Hits New High</title>
      <link>https://coindesk.com/bitcoin-high</link>
      <pubDate>Mon, 11 May 2026 10:00:00 +0000</pubDate>
      <description>Bitcoin surged past previous records today.</description>
    </item>
    <item>
      <title>Ethereum ETF Approved</title>
      <link>https://coindesk.com/eth-etf</link>
      <pubDate>Mon, 11 May 2026 09:00:00 +0000</pubDate>
      <description>The SEC approved the first Ethereum ETF.</description>
    </item>
    <item>
      <title>Solana DEX Volume Soars</title>
      <link>https://coindesk.com/sol-dex</link>
      <pubDate>Mon, 11 May 2026 08:00:00 +0000</pubDate>
      <description>Solana-based DEX volume hit all-time high.</description>
    </item>
  </channel>
</rss>"#;

    #[test]
    fn actions_includes_headlines_and_url() {
        let plugin = NewsPlugin::new();
        let actions = plugin.actions();
        assert_eq!(actions.len(), 3);
        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"fetch_headlines"));
        assert!(ids.contains(&"fetch_url"));
        assert!(ids.contains(&"crypto_panic"));
        let headlines = actions.iter().find(|a| a.id == "fetch_headlines").unwrap();
        assert_eq!(headlines.action_type, ActionType::ReadOnly);
        let fetch = actions.iter().find(|a| a.id == "fetch_url").unwrap();
        assert_eq!(fetch.action_type, ActionType::ReadOnly);
        let cp = actions.iter().find(|a| a.id == "crypto_panic").unwrap();
        assert_eq!(cp.action_type, ActionType::ReadOnly);
    }

    #[tokio::test]
    async fn fetch_headlines_parses_rss() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("GET", "/rss")
            .with_status(200)
            .with_header("content-type", "application/rss+xml")
            .with_body(SAMPLE_RSS)
            .create_async()
            .await;

        let plugin = NewsPlugin::with_base_url(format!("{}/rss", server.url()));
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = json!({});

        let result = plugin
            .read("fetch_headlines", &params, &rpc)
            .await
            .expect("fetch_headlines should succeed");

        let items = result["items"].as_array().unwrap();
        assert!(!items.is_empty());
        assert_eq!(items[0]["title"], "Bitcoin Hits New High");
        assert_eq!(items[0]["link"], "https://coindesk.com/bitcoin-high");
        assert_eq!(result["count"], items.len());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn fetch_headlines_respects_limit() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("GET", "/rss")
            .with_status(200)
            .with_header("content-type", "application/rss+xml")
            .with_body(SAMPLE_RSS)
            .create_async()
            .await;

        let plugin = NewsPlugin::with_base_url(format!("{}/rss", server.url()));
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = json!({"limit": 2});

        let result = plugin
            .read("fetch_headlines", &params, &rpc)
            .await
            .expect("fetch_headlines with limit should succeed");

        let items = result["items"].as_array().unwrap();
        assert_eq!(items.len(), 2, "limit=2 should return exactly 2 items");
        assert_eq!(result["count"], 2);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn fetch_url_returns_body_and_status() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("GET", "/some-page")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body("<html>hello</html>")
            .create_async()
            .await;

        let plugin = NewsPlugin::new();
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = json!({"url": format!("{}/some-page", server.url())});

        let result = plugin
            .read("fetch_url", &params, &rpc)
            .await
            .expect("fetch_url should succeed");

        assert_eq!(result["status"], 200);
        assert!(result["body"].as_str().unwrap().contains("hello"));
        assert!(result["url"].as_str().unwrap().contains("/some-page"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn crypto_panic_returns_posts() {
        let mut server = Server::new_async().await;

        let body = r#"{
            "count": 2,
            "results": [
                {
                    "title": "Solana hits ATH",
                    "url": "https://example.com/sol-ath",
                    "published_at": "2026-05-12T10:00:00Z",
                    "source": {"title": "CoinTelegraph"}
                },
                {
                    "title": "Bitcoin ETF approved",
                    "url": "https://example.com/btc-etf",
                    "published_at": "2026-05-12T09:00:00Z",
                    "source": {"title": "Bloomberg"}
                }
            ]
        }"#;

        let mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/posts/\?".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create_async()
            .await;

        let plugin = NewsPlugin::new().with_cryptopanic_url(server.url());
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        // Remove env token so we go through the public path
        std::env::remove_var("CRYPTOPANIC_TOKEN");
        let params = json!({"limit": 10});

        let result = plugin
            .read("crypto_panic", &params, &rpc)
            .await
            .expect("crypto_panic should succeed");

        let posts = result["posts"].as_array().expect("posts should be array");
        assert_eq!(posts.len(), 2);
        assert_eq!(posts[0]["title"], "Solana hits ATH");
        assert_eq!(posts[0]["source"], "CoinTelegraph");
        assert_eq!(posts[1]["title"], "Bitcoin ETF approved");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn crypto_panic_with_filter_and_currencies() {
        let mut server = Server::new_async().await;

        let body = r#"{"count": 1, "results": [{"title": "SOL bullish", "url": "https://example.com/sol", "published_at": "2026-05-12T08:00:00Z", "source": {"title": "CryptoNews"}}]}"#;

        let mock = server
            .mock(
                "GET",
                mockito::Matcher::AllOf(vec![
                    mockito::Matcher::UrlEncoded("filter".into(), "bullish".into()),
                    mockito::Matcher::UrlEncoded("currencies".into(), "SOL".into()),
                ]),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create_async()
            .await;

        let plugin = NewsPlugin::new().with_cryptopanic_url(server.url());
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        std::env::remove_var("CRYPTOPANIC_TOKEN");
        let params = json!({"filter": "bullish", "currencies": ["SOL"], "limit": 5});

        let result = plugin
            .read("crypto_panic", &params, &rpc)
            .await
            .expect("crypto_panic with filter should succeed");

        let posts = result["posts"].as_array().expect("posts should be array");
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0]["title"], "SOL bullish");

        mock.assert_async().await;
    }
}
