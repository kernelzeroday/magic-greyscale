use anyhow::Result;
use serde_json::json;
use super::client::ScryfallClient;
use super::models::{Card, CollectionResult, SearchResult, ScryfallSet};

const BASE_URL: &str = "https://api.scryfall.com";

impl ScryfallClient {
    pub async fn search(&self, query: &str, max_cards: usize) -> Result<Vec<Card>> {
        let mut cards = Vec::new();
        let encoded = urlencoding::encode(query);
        let mut url = format!("{}/cards/search?q={}", BASE_URL, encoded);

        loop {
            let result: SearchResult = self.get_json(&url).await?;
            cards.extend(result.data);
            if cards.len() >= max_cards || !result.has_more {
                break;
            }
            match result.next_page {
                Some(next) => url = next,
                None => break,
            }
        }

        cards.truncate(max_cards);
        Ok(cards)
    }

    pub async fn card_by_name(&self, name: &str) -> Result<Card> {
        let encoded = urlencoding::encode(name);
        let url = format!("{}/cards/named?fuzzy={}", BASE_URL, encoded);
        self.get_json(&url).await
    }

    pub async fn card_by_set_number(&self, set: &str, number: &str) -> Result<Card> {
        let url = format!("{}/cards/{}/{}", BASE_URL, set.to_lowercase(), number);
        self.get_json(&url).await
    }

    pub async fn cards_by_names(&self, names: &[String]) -> Result<Vec<(String, Option<Card>)>> {
        let mut results: Vec<(String, Option<Card>)> = Vec::new();

        for chunk in names.chunks(75) {
            let identifiers: Vec<serde_json::Value> = chunk.iter()
                .map(|name| json!({"name": name}))
                .collect();
            let body = json!({"identifiers": identifiers});
            let url = format!("{}/cards/collection", BASE_URL);

            let response: CollectionResult = self.post_json(&url, &body).await?;

            let mut found: std::collections::HashMap<String, Card> = response.data.into_iter()
                .map(|c| (c.name.to_lowercase(), c))
                .collect();

            for name in chunk {
                let card = found.remove(&name.to_lowercase());
                results.push((name.clone(), card));
            }

            if !response.not_found.is_empty() {
                eprintln!("  {} cards not found in batch", response.not_found.len());
            }
        }

        Ok(results)
    }

    pub async fn get_set(&self, code: &str) -> Result<ScryfallSet> {
        let url = format!("{}/sets/{}", BASE_URL, code.to_lowercase());
        self.get_json(&url).await
    }

    pub async fn set_cards(&self, set_code: &str, max_cards: usize) -> Result<Vec<Card>> {
        self.search(&format!("set:{}", set_code.to_lowercase()), max_cards).await
    }
}
