use std::path::PathBuf;
use anyhow::{Context, Result};

pub struct ResponseCache {
    cache_dir: PathBuf,
}

impl ResponseCache {
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("magic_greyscale")
            .join("api");
        std::fs::create_dir_all(&cache_dir)
            .with_context(|| format!("failed to create api cache dir: {:?}", cache_dir))?;
        Ok(Self { cache_dir })
    }

    fn key_for(url: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    pub fn get(&self, url: &str) -> Option<String> {
        let path = self.cache_dir.join(Self::key_for(url));
        std::fs::read_to_string(path).ok()
    }

    pub fn put(&self, url: &str, body: &str) -> Result<()> {
        let path = self.cache_dir.join(Self::key_for(url));
        std::fs::write(&path, body)
            .with_context(|| format!("failed to write api cache: {:?}", path))?;
        Ok(())
    }
}
