use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct SearchResult {
    pub total_cards: Option<u32>,
    pub has_more: bool,
    pub next_page: Option<String>,
    pub data: Vec<Card>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub layout: String,
    pub mana_cost: Option<String>,
    pub type_line: Option<String>,
    pub oracle_text: Option<String>,
    pub rarity: Option<String>,
    pub set: String,
    pub set_name: String,
    pub collector_number: String,
    pub image_uris: Option<ImageUris>,
    pub card_faces: Option<Vec<CardFace>>,
    pub arena_id: Option<i64>,
}

impl Card {
    pub fn image_uri(&self, quality: &str) -> Option<String> {
        if let Some(ref uris) = self.image_uris {
            return uris.get(quality);
        }
        if let Some(ref faces) = self.card_faces {
            if let Some(face) = faces.first() {
                if let Some(ref uris) = face.image_uris {
                    return uris.get(quality);
                }
            }
        }
        None
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct CardFace {
    pub name: String,
    pub mana_cost: Option<String>,
    pub type_line: Option<String>,
    pub oracle_text: Option<String>,
    pub image_uris: Option<ImageUris>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ImageUris {
    pub small: Option<String>,
    pub normal: Option<String>,
    pub large: Option<String>,
    pub png: Option<String>,
    pub art_crop: Option<String>,
    pub border_crop: Option<String>,
}

impl ImageUris {
    pub fn get(&self, quality: &str) -> Option<String> {
        match quality {
            "small" => self.small.clone(),
            "normal" => self.normal.clone(),
            "large" => self.large.clone(),
            "png" => self.png.clone(),
            "art_crop" => self.art_crop.clone(),
            "border_crop" => self.border_crop.clone(),
            _ => self.png.clone(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct CollectionResult {
    pub data: Vec<Card>,
    pub not_found: Vec<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct ScryfallSet {
    pub code: String,
    pub name: String,
    pub set_type: String,
    pub card_count: u32,
    pub released_at: Option<String>,
}
