use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "magic_greyscale", about = "MTG proxy sheet generator with greyscale conversion")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Search for cards by name or Scryfall query
    Search {
        /// Scryfall search query (e.g. "lightning bolt", "set:sta", "t:instant c:u")
        query: Vec<String>,
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Browse all cards in a specific set
    Set {
        /// Set code (e.g. sta, wot, spg)
        code: String,
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    /// Generate a print-ready PDF sheet from multiple sources
    Print {
        /// Sources: deck files (.txt) or Scryfall queries. Can specify multiple.
        #[arg(required = true)]
        sources: Vec<String>,
        #[arg(short, long, default_value = "output.pdf")]
        output: String,
        #[arg(short, long, default_value = "a4")]
        paper: PaperSize,
        #[arg(long, default_value = "true")]
        cut_lines: bool,
        #[arg(long, default_value = "true")]
        greyscale: bool,
        #[arg(long, default_value = "10")]
        contrast: f32,
        #[arg(long, default_value = "0")]
        brightness: i32,
        /// Also export a Cockatrice .cod deck file
        #[arg(long)]
        cockatrice: Option<String>,
        /// Toner save: lighten dark areas to reduce toner usage (0-100, 0=off)
        #[arg(long, default_value = "0")]
        toner_save: u32,
    },
    /// Download card images without generating PDF
    Fetch {
        /// Scryfall search query
        query: Vec<String>,
        #[arg(short, long, default_value = ".")]
        output_dir: String,
        #[arg(long)]
        greyscale: bool,
        #[arg(long, default_value = "10")]
        contrast: f32,
        #[arg(long, default_value = "0")]
        brightness: i32,
    },
    /// Generate a print-ready PDF from Pokemon TCG cards
    Pokemon {
        /// Pokemon set IDs (prefixed with "set:") or card name search queries. Can specify multiple.
        #[arg(required = true)]
        sources: Vec<String>,
        #[arg(short, long, default_value = "pokemon_output.pdf")]
        output: String,
        #[arg(short, long, default_value = "a4")]
        paper: PaperSize,
        #[arg(long, default_value = "true")]
        greyscale: bool,
        #[arg(long, default_value = "10")]
        contrast: f32,
        #[arg(long, default_value = "0")]
        brightness: i32,
        /// Toner save: lighten dark areas to reduce toner usage (0-100, 0=off)
        #[arg(long, default_value = "0")]
        toner_save: u32,
    },
}

#[derive(Clone, ValueEnum)]
pub enum PaperSize {
    A4,
    Letter,
}
