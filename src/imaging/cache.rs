use std::path::PathBuf;
use anyhow::{Context, Result};

pub struct ImageCache {
    cache_dir: PathBuf,
}

impl ImageCache {
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("magic_greyscale")
            .join("images");
        std::fs::create_dir_all(&cache_dir)
            .with_context(|| format!("failed to create cache dir: {:?}", cache_dir))?;
        Ok(Self { cache_dir })
    }

    pub fn get(&self, card_id: &str, quality: &str) -> Option<PathBuf> {
        let path = self.path_for(card_id, quality);
        if path.exists() { Some(path) } else { None }
    }

    pub fn put(&self, card_id: &str, quality: &str, data: &[u8]) -> Result<PathBuf> {
        let path = self.path_for(card_id, quality);
        std::fs::write(&path, data)
            .with_context(|| format!("failed to write cache file: {:?}", path))?;
        Ok(path)
    }

    fn path_for(&self, card_id: &str, quality: &str) -> PathBuf {
        self.cache_dir.join(format!("{}_{}.png", card_id, quality))
    }
}
