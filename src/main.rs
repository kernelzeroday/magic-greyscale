mod cli;
mod scryfall;
mod deck;
mod imaging;
mod pdf;
mod pokemon;
mod arena;

use std::collections::{HashMap, HashSet};
use std::path::Path;
use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use image::DynamicImage;

use cli::{Cli, Commands};
use scryfall::client::ScryfallClient;
use scryfall::models::Card;
use imaging::cache::ImageCache;
use imaging::download::download_card_images;
use imaging::greyscale::{convert_to_greyscale, fill_rounded_corners};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = ScryfallClient::new()?;

    match cli.command {
        Commands::Search { query, limit } => {
            cmd_search(&client, &query.join(" "), limit).await
        }
        Commands::Set { code, limit } => {
            cmd_set(&client, &code, limit).await
        }
        Commands::Print { sources, output, paper, cut_lines, greyscale, contrast, brightness, cockatrice, toner_save } => {
            let cache = ImageCache::new()?;
            cmd_print(&client, &cache, &sources, &output, &paper, cut_lines, greyscale, contrast, brightness, cockatrice.as_deref(), toner_save).await
        }
        Commands::Fetch { query, output_dir, greyscale, contrast, brightness } => {
            let cache = ImageCache::new()?;
            cmd_fetch(&client, &cache, &query.join(" "), &output_dir, greyscale, contrast, brightness).await
        }
        Commands::Pokemon { sources, output, paper, greyscale, contrast, brightness, toner_save } => {
            let cache = ImageCache::new()?;
            cmd_pokemon(&cache, &sources, &output, &paper, greyscale, contrast, brightness, toner_save).await
        }
        Commands::Arena { log, output, paper, greyscale, contrast, brightness, toner_save, cockatrice } => {
            let cache = ImageCache::new()?;
            cmd_arena(&client, &cache, log.as_deref(), &output, &paper, greyscale, contrast, brightness, toner_save, cockatrice.as_deref()).await
        }
    }
}

fn display_card(card: &Card) {
    let name = match card.rarity.as_deref() {
        Some("uncommon") => card.name.cyan().to_string(),
        Some("rare") => card.name.yellow().to_string(),
        Some("mythic") => card.name.red().to_string(),
        _ => card.name.white().to_string(),
    };

    let mana = card.mana_cost.as_deref().unwrap_or("");
    let type_line = card.type_line.as_deref().unwrap_or("");
    let set = &card.set.to_uppercase();

    println!("  {} {} {} [{}#{}]",
        name,
        mana.dimmed(),
        type_line.dimmed(),
        set,
        card.collector_number,
    );
}

async fn cmd_search(client: &ScryfallClient, query: &str, limit: usize) -> Result<()> {
    println!("Searching for: {}", query.bold());
    let cards = client.search(query, limit).await?;
    println!("Found {} cards:\n", cards.len());
    for card in &cards {
        display_card(card);
    }
    Ok(())
}

async fn cmd_set(client: &ScryfallClient, code: &str, limit: usize) -> Result<()> {
    let set_info = client.get_set(code).await?;
    println!("{} ({}) — {} cards, released {}",
        set_info.name.bold(),
        set_info.code.to_uppercase(),
        set_info.card_count,
        set_info.released_at.as_deref().unwrap_or("unknown"),
    );
    println!();

    let cards = client.set_cards(code, limit).await?;
    for card in &cards {
        display_card(card);
    }
    Ok(())
}

async fn cmd_print(
    client: &ScryfallClient,
    cache: &ImageCache,
    sources: &[String],
    output: &str,
    paper: &cli::PaperSize,
    cut_lines: bool,
    greyscale: bool,
    contrast: f32,
    brightness: i32,
    cockatrice: Option<&str>,
    toner_save: u32,
) -> Result<()> {
    let mut all_cards: Vec<(Card, u32)> = Vec::new();
    let mut seen_ids: HashMap<String, usize> = HashMap::new();

    for source in sources {
        println!("Resolving: {}", source.dimmed());
        let cards = resolve_source(client, source).await?;
        let before = all_cards.len();
        let mut merged = 0usize;
        for (card, qty) in cards {
            if let Some(&idx) = seen_ids.get(&card.id) {
                all_cards[idx].1 += qty;
                merged += 1;
            } else {
                seen_ids.insert(card.id.clone(), all_cards.len());
                all_cards.push((card, qty));
            }
        }
        println!("  +{} cards ({} merged)",
            all_cards.len() - before, merged);
    }

    let total_cards: u32 = all_cards.iter().map(|(_, q)| q).sum();
    println!("\n{} unique cards, {} total with quantities",
        all_cards.len(), total_cards);

    if let Some(cod_path) = cockatrice {
        write_cockatrice(&all_cards, Path::new(cod_path))?;
        println!("{} {}", "Cockatrice:".green().bold(), cod_path);
    }

    let image_paths = download_card_images(client, &all_cards, cache).await?;

    println!("Processing images...");
    let mut processed: Vec<(DynamicImage, u32)> = Vec::new();
    for (path, qty) in &image_paths {
        let img = if greyscale {
            convert_to_greyscale(path, contrast, brightness, toner_save)?
        } else {
            image::open(path).context("failed to open image")?
        };
        let mut img = img;
        fill_rounded_corners(&mut img);
        processed.push((img, *qty));
    }

    let layout = pdf::layout::LayoutConfig::new(paper);
    let remainder = total_cards as usize % layout.cards_per_page();
    if remainder > 0 {
        let filler_needed = layout.cards_per_page() - remainder;
        println!("Filling {} empty slots with JP full-art lands...", filler_needed);
        let filler_cards = client.search(
            "set:neo frame:fullart t:basic",
            filler_needed,
        ).await?;
        let filler_with_qty: Vec<(Card, u32)> = filler_cards.into_iter().map(|c| (c, 1)).collect();
        let filler_paths = download_card_images(client, &filler_with_qty, cache).await?;
        for (path, qty) in &filler_paths {
            let img = if greyscale {
                convert_to_greyscale(path, contrast, brightness, toner_save)?
            } else {
                image::open(path).context("failed to open image")?
            };
            let mut img = img;
        fill_rounded_corners(&mut img);
        processed.push((img, *qty));
        }
    }

    let total_with_filler: u32 = processed.iter().map(|(_, q)| *q).sum();
    let pages = layout.pages_needed(total_with_filler as usize);
    println!("Generating PDF: {} pages, {} paper", pages, match paper {
        cli::PaperSize::A4 => "A4",
        cli::PaperSize::Letter => "Letter",
    });

    pdf::generator::generate_pdf(&processed, &layout, cut_lines, Path::new(output))?;
    println!("{} {}", "Saved:".green().bold(), output);
    Ok(())
}

fn write_cockatrice(cards: &[(Card, u32)], path: &Path) -> Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)
        .with_context(|| format!("failed to create cockatrice file: {:?}", path))?;
    writeln!(f, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    writeln!(f, r#"<cockatrice_deck version="1">"#)?;
    writeln!(f, r#"    <deckname>Magic Greyscale Master</deckname>"#)?;
    writeln!(f, r#"    <comments>Generated by magic_greyscale</comments>"#)?;
    writeln!(f, r#"    <zone name="main">"#)?;
    for (card, qty) in cards {
        let name = card.name.replace('&', "&amp;").replace('"', "&quot;")
            .replace('<', "&lt;").replace('>', "&gt;");
        writeln!(f, r#"        <card number="{}" name="{}"/>"#, qty, name)?;
    }
    writeln!(f, r#"    </zone>"#)?;
    writeln!(f, r#"</cockatrice_deck>"#)?;
    Ok(())
}

async fn cmd_fetch(
    client: &ScryfallClient,
    cache: &ImageCache,
    query: &str,
    output_dir: &str,
    greyscale: bool,
    contrast: f32,
    brightness: i32,
) -> Result<()> {
    let cards = client.search(query, 500).await?;
    println!("Found {} cards, downloading...", cards.len());

    let cards_with_qty: Vec<(Card, u32)> = cards.into_iter().map(|c| (c, 1)).collect();
    let image_paths = download_card_images(client, &cards_with_qty, cache).await?;

    let out = Path::new(output_dir);
    std::fs::create_dir_all(out)?;

    for (i, (path, _)) in image_paths.iter().enumerate() {
        let card = &cards_with_qty[i].0;
        let filename = format!("{}_{}.png",
            card.name.replace(['/', '\\', ':', '"', '?', '*', '<', '>', '|'], "_"),
            card.set,
        );
        let dest = out.join(&filename);

        if greyscale {
            let img = convert_to_greyscale(path, contrast, brightness, 0)?;
            img.save(&dest).with_context(|| format!("failed to save: {:?}", dest))?;
        } else {
            std::fs::copy(path, &dest)?;
        }
        println!("  {} {}", "->".green(), filename);
    }

    println!("{} {} images to {}", "Saved".green().bold(), image_paths.len(), output_dir);
    Ok(())
}

async fn cmd_pokemon(
    cache: &ImageCache,
    sources: &[String],
    output: &str,
    paper: &cli::PaperSize,
    greyscale: bool,
    contrast: f32,
    brightness: i32,
    toner_save: u32,
) -> Result<()> {
    let client = pokemon::PokemonClient::new()?;
    let mut all_cards: Vec<pokemon::PokemonCard> = Vec::new();
    let mut seen_ids = HashSet::new();

    for source in sources {
        println!("Resolving: {}", source.dimmed());
        let cards = if let Some(set_id) = source.strip_prefix("set:") {
            client.cards_by_set(set_id).await
                .with_context(|| format!("failed to fetch Pokemon set: {}", set_id))?
        } else {
            client.search_by_name(source).await
                .with_context(|| format!("failed to search Pokemon cards: {}", source))?
        };
        let before = all_cards.len();
        let mut dupes = 0usize;
        for card in cards {
            if seen_ids.insert(card.id.clone()) {
                all_cards.push(card);
            } else {
                dupes += 1;
            }
        }
        println!("  +{} cards ({} dupes skipped)",
            all_cards.len() - before, dupes);
    }

    println!("\n{} total Pokemon cards", all_cards.len());

    let image_paths = pokemon::download_pokemon_images(&client, &all_cards, cache).await?;

    println!("Processing images...");
    let mut processed: Vec<(DynamicImage, u32)> = Vec::new();
    for (path, qty) in &image_paths {
        let img = if greyscale {
            convert_to_greyscale(path, contrast, brightness, toner_save)?
        } else {
            image::open(path).context("failed to open image")?
        };
        let mut img = img;
        fill_rounded_corners(&mut img);
        processed.push((img, *qty));
    }

    let layout = pdf::layout::LayoutConfig::new(paper);
    let total_cards: u32 = processed.iter().map(|(_, q)| *q).sum();
    let pages = layout.pages_needed(total_cards as usize);
    println!("Generating PDF: {} pages, {} paper", pages, match paper {
        cli::PaperSize::A4 => "A4",
        cli::PaperSize::Letter => "Letter",
    });

    if let Some(parent) = Path::new(output).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create output directory: {:?}", parent))?;
        }
    }

    pdf::generator::generate_pdf(&processed, &layout, true, Path::new(output))?;
    println!("{} {}", "Saved:".green().bold(), output);
    Ok(())
}

async fn cmd_arena(
    client: &ScryfallClient,
    cache: &ImageCache,
    log_path: Option<&str>,
    output: &str,
    paper: &cli::PaperSize,
    greyscale: bool,
    contrast: f32,
    brightness: i32,
    toner_save: u32,
    cockatrice: Option<&str>,
) -> Result<()> {
    let log_file = match log_path {
        Some(p) => p.to_string(),
        None => arena::find_log_file()?,
    };
    println!("Reading MTGA log: {}", log_file);

    let decks = arena::extract_decks_from_log(&log_file)?;
    println!("Found {} decks in log", decks.len());

    let all_ids = arena::extract_all_card_ids(&log_file)?;
    println!("{} unique arena card IDs across all decks", all_ids.len());

    println!("Resolving arena IDs via Scryfall...");
    let cards_with_qty = arena::resolve_arena_ids(client, &all_ids).await?;
    println!("Resolved {} cards", cards_with_qty.len());

    if cards_with_qty.is_empty() {
        anyhow::bail!("no cards found — try playing a game in Arena first so decks appear in the log");
    }

    if let Some(cod_path) = cockatrice {
        write_cockatrice(&cards_with_qty, Path::new(cod_path))?;
        println!("{} {}", "Cockatrice:".green().bold(), cod_path);
    }

    let image_paths = download_card_images(client, &cards_with_qty, cache).await?;

    println!("Processing images...");
    let mut processed: Vec<(DynamicImage, u32)> = Vec::new();
    for (path, qty) in &image_paths {
        let img = if greyscale {
            convert_to_greyscale(path, contrast, brightness, toner_save)?
        } else {
            image::open(path).context("failed to open image")?
        };
        let mut img = img;
        fill_rounded_corners(&mut img);
        processed.push((img, *qty));
    }

    let layout = pdf::layout::LayoutConfig::new(paper);
    let total_cards: u32 = processed.iter().map(|(_, q)| *q).sum();

    let remainder = total_cards as usize % layout.cards_per_page();
    if remainder > 0 {
        let filler_needed = layout.cards_per_page() - remainder;
        println!("Filling {} empty slots with full-art lands...", filler_needed);
        let filler_cards = client.search("set:neo frame:fullart t:basic", filler_needed).await?;
        let filler_with_qty: Vec<(Card, u32)> = filler_cards.into_iter().map(|c| (c, 1)).collect();
        let filler_paths = download_card_images(client, &filler_with_qty, cache).await?;
        for (path, qty) in &filler_paths {
            let img = if greyscale {
                convert_to_greyscale(path, contrast, brightness, toner_save)?
            } else {
                image::open(path).context("failed to open image")?
            };
            let mut img = img;
        fill_rounded_corners(&mut img);
        processed.push((img, *qty));
        }
    }

    let total_with_filler: u32 = processed.iter().map(|(_, q)| *q).sum();
    let pages = layout.pages_needed(total_with_filler as usize);
    println!("Generating PDF: {} pages, {} paper", pages, match paper {
        cli::PaperSize::A4 => "A4",
        cli::PaperSize::Letter => "Letter",
    });

    pdf::generator::generate_pdf(&processed, &layout, true, Path::new(output))?;
    println!("{} {}", "Saved:".green().bold(), output);
    Ok(())
}

async fn resolve_source(client: &ScryfallClient, source: &str) -> Result<Vec<(Card, u32)>> {
    let path = Path::new(source);
    if path.is_dir() {
        let deck_list = deck::parser::parse_cockatrice_folder(path)
            .with_context(|| format!("failed to parse cockatrice folder: {}", source))?;
        println!("  {} unique cards from {} deck folder",
            deck_list.mainboard.len(), source);
        resolve_deck_entries(client, &deck_list).await
    } else if path.exists() {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read deck file: {}", source))?;
        let deck_list = if source.ends_with(".cod") || content.trim_start().starts_with("<?xml") || content.contains("<cockatrice_deck") {
            deck::parser::parse_cockatrice(&content)?
        } else {
            deck::parser::parse_mtga(&content)?
        };
        resolve_deck_entries(client, &deck_list).await
    } else {
        let query = source.strip_prefix("q:").unwrap_or(source);
        let found = client.search(query, 500).await?;
        Ok(found.into_iter().map(|c| (c, 1)).collect())
    }
}

async fn resolve_deck_entries(client: &ScryfallClient, deck_list: &deck::models::Deck) -> Result<Vec<(Card, u32)>> {
    let mut cards = Vec::new();
    let entries: Vec<_> = deck_list.mainboard.iter().chain(deck_list.sideboard.iter()).collect();

    let mut set_number_entries = Vec::new();
    let mut name_only_entries = Vec::new();
    for entry in &entries {
        if entry.set_code.is_some() && entry.collector_number.is_some() {
            set_number_entries.push(*entry);
        } else {
            name_only_entries.push(*entry);
        }
    }

    for entry in &set_number_entries {
        let set = entry.set_code.as_ref().unwrap();
        let num = entry.collector_number.as_ref().unwrap();
        match client.card_by_set_number(set, num).await {
            Ok(card) => cards.push((card, entry.quantity)),
            Err(_) => {
                eprintln!("  {} ({} {}) not found by set/number, trying fuzzy name...",
                    entry.card_name, set, num);
                name_only_entries.push(*entry);
            }
        }
    }

    if !name_only_entries.is_empty() {
        let names: Vec<String> = name_only_entries.iter()
            .map(|e| e.card_name.clone())
            .collect();
        eprintln!("  batch looking up {} cards...", names.len());
        let results = client.cards_by_names(&names).await?;
        let mut fallback_names = Vec::new();
        let mut fallback_indices = Vec::new();
        for (i, (_name, card_opt)) in results.into_iter().enumerate() {
            match card_opt {
                Some(card) => cards.push((card, name_only_entries[i].quantity)),
                None => {
                    fallback_names.push(name_only_entries[i].card_name.clone());
                    fallback_indices.push(i);
                }
            }
        }
        if !fallback_names.is_empty() {
            eprintln!("  fuzzy fallback for {} cards...", fallback_names.len());
            for (fi, idx) in fallback_indices.iter().enumerate() {
                match client.card_by_name(&fallback_names[fi]).await {
                    Ok(card) => cards.push((card, name_only_entries[*idx].quantity)),
                    Err(e) => eprintln!("  warning: skipping '{}': {}", fallback_names[fi], e),
                }
            }
        }
    }

    Ok(cards)
}
