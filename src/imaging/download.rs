use std::path::PathBuf;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};

use crate::scryfall::client::ScryfallClient;
use crate::scryfall::models::Card;
use super::cache::ImageCache;

pub async fn download_card_images(
    client: &ScryfallClient,
    cards: &[(Card, u32)],
    cache: &ImageCache,
) -> Result<Vec<(PathBuf, u32)>> {
    let mut seen = std::collections::HashMap::new();
    let mut results = Vec::new();

    let pb = ProgressBar::new(cards.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg} [{bar:40}] {pos}/{len}")
        .unwrap()
        .progress_chars("=> "));
    pb.set_message("Downloading cards");

    for (card, qty) in cards {
        if let Some(path) = seen.get(&card.id) {
            results.push((PathBuf::from(path), *qty));
            pb.inc(1);
            continue;
        }

        let path = if let Some(cached) = cache.get(&card.id, "png") {
            cached
        } else {
            let uri = card.image_uri("png")
                .with_context(|| format!("no image URI for card: {}", card.name))?;
            let bytes = client.get_bytes(&uri).await
                .with_context(|| format!("failed to download image for: {}", card.name))?;
            cache.put(&card.id, "png", &bytes)?
        };

        seen.insert(card.id.clone(), path.clone());
        results.push((path, *qty));
        pb.inc(1);
    }

    pb.finish_with_message("Download complete");
    Ok(results)
}
