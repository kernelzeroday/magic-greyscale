use std::collections::HashMap;
use std::path::Path;
use anyhow::{Context, Result};
use serde::Deserialize;

use crate::scryfall::client::ScryfallClient;
use crate::scryfall::models::Card;

const MTGA_LOG_PATH: &str = "~/Library/Logs/Wizards Of The Coast/MTGA/Player.log";
const MTGA_PREV_LOG_PATH: &str = "~/Library/Logs/Wizards Of The Coast/MTGA/Player-prev.log";

#[derive(Deserialize, Debug, Clone)]
struct DeckCard {
    #[serde(alias = "cardId")]
    card_id: u64,
    quantity: u32,
}

#[derive(Deserialize, Debug)]
struct CourseDeck {
    #[serde(alias = "MainDeck")]
    main_deck: Vec<DeckCard>,
    #[serde(alias = "Sideboard", default)]
    sideboard: Vec<DeckCard>,
}

pub fn expand_log_path(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]).to_string_lossy().to_string();
        }
    }
    path.to_string()
}

pub fn find_log_file() -> Result<String> {
    let current = expand_log_path(MTGA_LOG_PATH);
    let prev = expand_log_path(MTGA_PREV_LOG_PATH);

    let current_size = std::fs::metadata(&current).map(|m| m.len()).unwrap_or(0);
    let prev_size = std::fs::metadata(&prev).map(|m| m.len()).unwrap_or(0);

    if current_size > 100_000 {
        Ok(current)
    } else if prev_size > 0 {
        eprintln!("  current log is small ({}B), using Player-prev.log ({}B)",
            current_size, prev_size);
        Ok(prev)
    } else if current_size > 0 {
        Ok(current)
    } else {
        anyhow::bail!("no MTGA log found at {} or {}", current, prev)
    }
}

pub fn extract_decks_from_log(log_path: &str) -> Result<Vec<Vec<(u64, u32)>>> {
    let content = std::fs::read_to_string(log_path)
        .with_context(|| format!("failed to read MTGA log: {}", log_path))?;

    let mut decks = Vec::new();

    for line in content.lines() {
        if !line.contains("CourseDeck") && !line.contains("MainDeck") {
            continue;
        }

        for start in find_all_occurrences(line, "\"MainDeck\":[") {
            if let Some(deck) = extract_deck_array(line, start) {
                if !deck.is_empty() {
                    decks.push(deck);
                }
            }
        }
    }

    decks.sort_by_key(|d| std::cmp::Reverse(d.len()));
    decks.dedup();
    Ok(decks)
}

pub fn extract_all_card_ids(log_path: &str) -> Result<HashMap<u64, u32>> {
    let decks = extract_decks_from_log(log_path)?;
    let mut all_cards: HashMap<u64, u32> = HashMap::new();
    for deck in &decks {
        for (card_id, qty) in deck {
            let entry = all_cards.entry(*card_id).or_insert(0);
            *entry = (*entry).max(*qty);
        }
    }
    Ok(all_cards)
}

pub async fn resolve_arena_ids(
    client: &ScryfallClient,
    arena_ids: &HashMap<u64, u32>,
) -> Result<Vec<(Card, u32)>> {
    let mut results = Vec::new();
    let ids: Vec<u64> = arena_ids.keys().cloned().collect();
    let total = ids.len();
    let mut skipped = 0usize;

    for (i, id) in ids.iter().enumerate() {
        if (i + 1) % 100 == 0 {
            eprintln!("  resolving arena ID {}/{}...", i + 1, total);
        }
        let url = format!("https://api.scryfall.com/cards/arena/{}", id);
        match client.get_json::<Card>(&url).await {
            Ok(card) => {
                let qty = arena_ids.get(id).copied().unwrap_or(1);
                results.push((card, qty));
            }
            Err(_) => {
                skipped += 1;
            }
        }
    }

    if skipped > 0 {
        eprintln!("  {} arena IDs could not be resolved", skipped);
    }

    Ok(results)
}

fn find_all_occurrences(haystack: &str, needle: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        positions.push(start + pos);
        start += pos + needle.len();
    }
    positions
}

fn extract_deck_array(line: &str, main_deck_start: usize) -> Option<Vec<(u64, u32)>> {
    let bracket_start = line[main_deck_start..].find('[')? + main_deck_start;
    let mut depth = 0;
    let mut bracket_end = None;
    for (i, ch) in line[bracket_start..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    bracket_end = Some(bracket_start + i + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let bracket_end = bracket_end?;
    let array_str = &line[bracket_start..bracket_end];

    let cards: Vec<DeckCard> = serde_json::from_str(array_str).ok()?;
    Some(cards.into_iter().map(|c| (c.card_id, c.quantity)).collect())
}
