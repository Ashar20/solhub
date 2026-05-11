use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use reqwest::{Client, Method, RequestBuilder};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::config::CliConfig;

pub struct ApiClient {
    http: Client,
    base: String,
    api_key: Option<String>,
}

impl ApiClient {
    pub fn new(cfg: &CliConfig) -> Self {
        Self {
            http: Client::new(),
            base: cfg.api_url(),
            api_key: cfg.api_key(),
        }
    }

    fn req(&self, method: Method, path: &str) -> RequestBuilder {
        let mut b = self
            .http
            .request(method, format!("{}{}", self.base, path));
        if let Some(key) = &self.api_key {
            b = b.bearer_auth(key);
        }
        b
    }

    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let r = self.req(Method::GET, path).send().await?;
        if !r.status().is_success() {
            return Err(anyhow!(
                "GET {} -> {}: {}",
                path,
                r.status(),
                r.text().await.unwrap_or_default()
            ));
        }
        Ok(r.json().await?)
    }

    pub async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let r = self.req(Method::POST, path).json(body).send().await?;
        if !r.status().is_success() {
            return Err(anyhow!(
                "POST {} -> {}: {}",
                path,
                r.status(),
                r.text().await.unwrap_or_default()
            ));
        }
        Ok(r.json().await?)
    }

    pub async fn patch_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let r = self
            .req(Method::PATCH, path)
            .json(body)
            .send()
            .await?;
        if !r.status().is_success() {
            return Err(anyhow!(
                "PATCH {} -> {}: {}",
                path,
                r.status(),
                r.text().await.unwrap_or_default()
            ));
        }
        Ok(r.json().await?)
    }

    pub async fn delete(&self, path: &str) -> Result<()> {
        let r = self.req(Method::DELETE, path).send().await?;
        if !r.status().is_success() {
            return Err(anyhow!("DELETE {} -> {}", path, r.status()));
        }
        Ok(())
    }

    pub async fn stream_sse(&self, path: &str, follow: bool) -> Result<()> {
        let r = self
            .req(Method::GET, path)
            .header("Accept", "text/event-stream")
            .send()
            .await?;
        let mut stream = r.bytes_stream();
        while let Some(item) = stream.next().await {
            let bytes = item?;
            let text = String::from_utf8_lossy(&bytes);
            print!("{}", text);
            if !follow && text.contains("event: run_complete") {
                break;
            }
        }
        Ok(())
    }
}
