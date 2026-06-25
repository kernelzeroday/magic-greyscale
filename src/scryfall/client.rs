use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use anyhow::{Context, Result};
use reqwest::Client;

use super::response_cache::ResponseCache;

pub struct ScryfallClient {
    client: Client,
    last_request: Arc<Mutex<Instant>>,
    response_cache: ResponseCache,
}

const MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(150);
const MAX_RETRIES: u32 = 8;

impl ScryfallClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent("MagicGreyscaleCLI/0.1")
            .build()
            .context("failed to build HTTP client")?;
        let response_cache = ResponseCache::new()?;
        Ok(Self {
            client,
            last_request: Arc::new(Mutex::new(Instant::now() - MIN_REQUEST_INTERVAL)),
            response_cache,
        })
    }

    async fn rate_limit(&self) {
        let mut last = self.last_request.lock().await;
        let elapsed = last.elapsed();
        if elapsed < MIN_REQUEST_INTERVAL {
            tokio::time::sleep(MIN_REQUEST_INTERVAL - elapsed).await;
        }
        *last = Instant::now();
    }

    pub async fn get(&self, url: &str) -> Result<reqwest::Response> {
        for attempt in 0..MAX_RETRIES {
            self.rate_limit().await;
            let resp = self.client.get(url).send().await
                .with_context(|| format!("request to {} failed", url))?;
            let status = resp.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                let delay = Duration::from_secs(1 + attempt as u64);
                eprintln!("  rate limited, waiting {}s...", delay.as_secs());
                tokio::time::sleep(delay).await;
                continue;
            }
            if !status.is_success() {
                anyhow::bail!("HTTP {} from {}", status, url);
            }
            return Ok(resp);
        }
        anyhow::bail!("gave up after {} retries for {}", MAX_RETRIES, url)
    }

    pub async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        if let Some(cached) = self.response_cache.get(url) {
            return serde_json::from_str(&cached)
                .with_context(|| format!("failed to parse cached JSON for {}", url));
        }
        let resp = self.get(url).await?;
        let text = resp.text().await
            .with_context(|| format!("failed to read response from {}", url))?;
        self.response_cache.put(url, &text)?;
        serde_json::from_str(&text)
            .with_context(|| format!("failed to parse JSON from {}", url))
    }

    pub async fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self.get(url).await?;
        let bytes = resp.bytes().await
            .with_context(|| format!("failed to read bytes from {}", url))?;
        Ok(bytes.to_vec())
    }

    pub async fn post_json<T: serde::de::DeserializeOwned>(&self, url: &str, body: &serde_json::Value) -> Result<T> {
        let cache_key = format!("{}|{}", url, body);
        if let Some(cached) = self.response_cache.get(&cache_key) {
            return serde_json::from_str(&cached)
                .with_context(|| format!("failed to parse cached JSON for {}", url));
        }
        for attempt in 0..MAX_RETRIES {
            self.rate_limit().await;
            let resp = self.client.post(url).json(body).send().await
                .with_context(|| format!("POST to {} failed", url))?;
            let status = resp.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                let delay = Duration::from_secs(1 + attempt as u64);
                eprintln!("  rate limited, waiting {}s...", delay.as_secs());
                tokio::time::sleep(delay).await;
                continue;
            }
            if !status.is_success() {
                anyhow::bail!("HTTP {} from POST {}", status, url);
            }
            let text = resp.text().await
                .with_context(|| format!("failed to read POST response from {}", url))?;
            self.response_cache.put(&cache_key, &text)?;
            return serde_json::from_str(&text)
                .with_context(|| format!("failed to parse JSON from POST {}", url));
        }
        anyhow::bail!("gave up after {} retries for POST {}", MAX_RETRIES, url)
    }
}
