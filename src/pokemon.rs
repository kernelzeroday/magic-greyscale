use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::imaging::cache::ImageCache;

const BASE_URL: &str = "https://api.pokemontcg.io/v2";
const MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(200);
const MAX_RETRIES: u32 = 8;

// --- Models ---

#[derive(Deserialize, Debug, Clone)]
pub struct PokemonCard {
    pub id: String,
    pub name: String,
    pub set: PokemonSet,
    pub images: PokemonImages,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PokemonSet {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PokemonImages {
    pub small: Option<String>,
    pub large: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct PokemonApiResponse {
    pub data: Vec<PokemonCard>,
    #[serde(rename = "totalCount")]
    pub total_count: Option<u32>,
    pub page: Option<u32>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<u32>,
    pub count: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct PokemonSingleResponse {
    pub data: PokemonCard,
}

// --- Client ---

pub struct PokemonClient {
    client: Client,
    last_request: Arc<Mutex<Instant>>,
}

impl PokemonClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent("MagicGreyscaleCLI/0.1")
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self {
            client,
            last_request: Arc::new(Mutex::new(Instant::now() - MIN_REQUEST_INTERVAL)),
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

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        for attempt in 0..MAX_RETRIES {
            self.rate_limit().await;
            let resp = self.client.get(url).send().await
                .with_context(|| format!("request to {} failed", url))?;
            let status = resp.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                let delay = Duration::from_secs(2 + attempt as u64 * 2);
                eprintln!("  rate limited, waiting {}s...", delay.as_secs());
                tokio::time::sleep(delay).await;
                continue;
            }
            if !status.is_success() {
                anyhow::bail!("HTTP {} from {}", status, url);
            }
            let text = resp.text().await
                .with_context(|| format!("failed to read response from {}", url))?;
            return serde_json::from_str(&text)
                .with_context(|| format!("failed to parse JSON from {}", url));
        }
        anyhow::bail!("gave up after {} retries for {}", MAX_RETRIES, url)
    }

    async fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        for attempt in 0..MAX_RETRIES {
            self.rate_limit().await;
            let resp = self.client.get(url).send().await
                .with_context(|| format!("request to {} failed", url))?;
            let status = resp.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                let delay = Duration::from_secs(2 + attempt as u64 * 2);
                eprintln!("  rate limited, waiting {}s...", delay.as_secs());
                tokio::time::sleep(delay).await;
                continue;
            }
            if !status.is_success() {
                anyhow::bail!("HTTP {} from {}", status, url);
            }
            let bytes = resp.bytes().await
                .with_context(|| format!("failed to read bytes from {}", url))?;
            return Ok(bytes.to_vec());
        }
        anyhow::bail!("gave up after {} retries for {}", MAX_RETRIES, url)
    }

    /// Fetch all cards in a Pokemon TCG set by set ID (e.g. "base1", "neo1").
    pub async fn cards_by_set(&self, set_id: &str) -> Result<Vec<PokemonCard>> {
        let mut all_cards = Vec::new();
        let mut page = 1u32;
        let page_size = 250;

        loop {
            let url = format!(
                "{}/cards?q=set.id:{}&pageSize={}&page={}",
                BASE_URL, urlencoding::encode(set_id), page_size, page
            );
            let result: PokemonApiResponse = self.get_json(&url).await?;
            let count = result.data.len();
            all_cards.extend(result.data);

            if count < page_size as usize {
                break;
            }
            page += 1;
        }

        Ok(all_cards)
    }

    /// Search for cards by name.
    pub async fn search_by_name(&self, name: &str) -> Result<Vec<PokemonCard>> {
        let mut all_cards = Vec::new();
        let mut page = 1u32;
        let page_size = 250;

        loop {
            let url = format!(
                "{}/cards?q=name:\"{}\"&pageSize={}&page={}",
                BASE_URL, urlencoding::encode(name), page_size, page
            );
            let result: PokemonApiResponse = self.get_json(&url).await?;
            let count = result.data.len();
            all_cards.extend(result.data);

            if count < page_size as usize {
                break;
            }
            page += 1;
        }

        Ok(all_cards)
    }

    /// Get a single card by its ID (e.g. "base1-4").
    pub async fn card_by_id(&self, id: &str) -> Result<PokemonCard> {
        let url = format!("{}/cards/{}", BASE_URL, urlencoding::encode(id));
        let result: PokemonSingleResponse = self.get_json(&url).await?;
        Ok(result.data)
    }
}

// --- Image downloading ---

pub async fn download_pokemon_images(
    client: &PokemonClient,
    cards: &[PokemonCard],
    cache: &ImageCache,
) -> Result<Vec<(PathBuf, u32)>> {
    let mut seen = std::collections::HashMap::new();
    let mut results = Vec::new();

    let pb = ProgressBar::new(cards.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg} [{bar:40}] {pos}/{len}")
        .unwrap()
        .progress_chars("=> "));
    pb.set_message("Downloading Pokemon cards");

    for card in cards {
        if let Some(path) = seen.get(&card.id) {
            results.push((PathBuf::from(path), 1u32));
            pb.inc(1);
            continue;
        }

        // Use card ID as cache key with "pokemon" quality prefix to avoid collisions with MTG cache
        let cache_key = format!("pokemon_{}", card.id);
        let path = if let Some(cached) = cache.get(&cache_key, "png") {
            cached
        } else {
            let image_url = card.images.large.as_ref()
                .or(card.images.small.as_ref())
                .with_context(|| format!("no image URL for Pokemon card: {} ({})", card.name, card.id))?;
            let bytes = client.get_bytes(image_url).await
                .with_context(|| format!("failed to download image for: {} ({})", card.name, card.id))?;
            cache.put(&cache_key, "png", &bytes)?
        };

        seen.insert(card.id.clone(), path.clone());
        results.push((path, 1u32));
        pb.inc(1);
    }

    pb.finish_with_message("Download complete");
    Ok(results)
}
