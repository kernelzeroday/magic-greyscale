pub struct DeckEntry {
    pub quantity: u32,
    pub card_name: String,
    pub set_code: Option<String>,
    pub collector_number: Option<String>,
}

pub struct Deck {
    pub mainboard: Vec<DeckEntry>,
    pub sideboard: Vec<DeckEntry>,
}
