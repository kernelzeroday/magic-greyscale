use std::path::Path;
use anyhow::{Context, Result};
use super::models::{Deck, DeckEntry};

pub fn parse_mtga(input: &str) -> Result<Deck> {
    let mut mainboard = Vec::new();
    let mut sideboard = Vec::new();
    let mut in_sideboard = false;

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.eq_ignore_ascii_case("deck") {
            in_sideboard = false;
            continue;
        }
        if line.eq_ignore_ascii_case("sideboard") {
            in_sideboard = true;
            continue;
        }

        let entry = parse_line(line)
            .with_context(|| format!("failed to parse line: {}", line))?;

        if in_sideboard {
            sideboard.push(entry);
        } else {
            mainboard.push(entry);
        }
    }

    Ok(Deck { mainboard, sideboard })
}

pub fn parse_cockatrice(input: &str) -> Result<Deck> {
    let mut mainboard = Vec::new();
    let mut sideboard = Vec::new();

    let mut in_side = false;
    for line in input.lines() {
        let line = line.trim();
        if line.contains(r#"name="side""#) || line.contains(r#"name="sb""#) {
            in_side = true;
        } else if line.contains(r#"name="main""#) {
            in_side = false;
        }

        if !line.starts_with("<card ") {
            continue;
        }

        let number = extract_attr(line, "number").unwrap_or("1".to_string());
        let name = extract_attr(line, "name")
            .context("card element missing name attribute")?;

        let quantity: u32 = number.parse().unwrap_or(1);
        let entry = DeckEntry {
            quantity,
            card_name: name,
            set_code: None,
            collector_number: None,
        };

        if in_side {
            sideboard.push(entry);
        } else {
            mainboard.push(entry);
        }
    }

    Ok(Deck { mainboard, sideboard })
}

pub fn parse_cockatrice_folder(folder: &Path) -> Result<Deck> {
    let mut all_mainboard = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let mut files: Vec<_> = std::fs::read_dir(folder)
        .with_context(|| format!("failed to read directory: {:?}", folder))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "cod"))
        .collect();
    files.sort_by_key(|e| e.file_name());

    for entry in &files {
        let content = std::fs::read_to_string(entry.path())
            .with_context(|| format!("failed to read: {:?}", entry.path()))?;
        let deck = parse_cockatrice(&content)?;
        for card in deck.mainboard.into_iter().chain(deck.sideboard) {
            if seen.insert(card.card_name.clone()) {
                all_mainboard.push(DeckEntry {
                    quantity: 1,
                    card_name: card.card_name,
                    set_code: None,
                    collector_number: None,
                });
            }
        }
    }

    Ok(Deck {
        mainboard: all_mainboard,
        sideboard: Vec::new(),
    })
}

fn extract_attr(line: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    let start = line.find(&pattern)? + pattern.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    let value = &rest[..end];
    let value = value.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'");
    Some(value)
}

fn parse_line(line: &str) -> Result<DeckEntry> {
    let (qty_str, rest) = line.split_once(' ')
        .context("expected 'N CardName'")?;
    let quantity: u32 = qty_str.parse()
        .context("expected quantity as first token")?;

    let (card_name, set_code, collector_number) = if let Some(paren_start) = rest.rfind('(') {
        let name = rest[..paren_start].trim();
        let after_paren = &rest[paren_start + 1..];
        if let Some(paren_end) = after_paren.find(')') {
            let set = &after_paren[..paren_end];
            let number = after_paren[paren_end + 1..].trim();
            let number = if number.is_empty() { None } else { Some(number.to_string()) };
            (name.to_string(), Some(set.to_string()), number)
        } else {
            (rest.to_string(), None, None)
        }
    } else {
        (rest.trim().to_string(), None, None)
    };

    Ok(DeckEntry { quantity, card_name, set_code, collector_number })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_line() {
        let entry = parse_line("4 Lightning Bolt (LEB) 162").unwrap();
        assert_eq!(entry.quantity, 4);
        assert_eq!(entry.card_name, "Lightning Bolt");
        assert_eq!(entry.set_code.as_deref(), Some("LEB"));
        assert_eq!(entry.collector_number.as_deref(), Some("162"));
    }

    #[test]
    fn test_parse_name_only() {
        let entry = parse_line("1 Counterspell").unwrap();
        assert_eq!(entry.quantity, 1);
        assert_eq!(entry.card_name, "Counterspell");
        assert!(entry.set_code.is_none());
    }

    #[test]
    fn test_parse_cockatrice() {
        let input = r#"<?xml version="1.0" encoding="UTF-8"?>
<cockatrice_deck version="1">
    <zone name="main">
        <card number="4" name="Lightning Bolt"/>
        <card number="3" name="Counterspell"/>
    </zone>
    <zone name="side">
        <card number="2" name="Pyroblast"/>
    </zone>
</cockatrice_deck>"#;
        let deck = parse_cockatrice(input).unwrap();
        assert_eq!(deck.mainboard.len(), 2);
        assert_eq!(deck.mainboard[0].card_name, "Lightning Bolt");
        assert_eq!(deck.mainboard[0].quantity, 4);
        assert_eq!(deck.sideboard.len(), 1);
    }
}
