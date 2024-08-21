use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};
use tokio::{
    fs::{self, metadata},
    io,
    sync::RwLock,
};

#[derive(Clone, Debug)]
struct CacheEntry {
    data: Arc<[u8]>,
    last_accessed: SystemTime,
}

pub struct FileCache(RwLock<HashMap<PathBuf, CacheEntry>>);

impl FileCache {
    pub fn new() -> Self {
        Self(RwLock::new(HashMap::new()))
    }

    pub async fn open(&self, path: &Path) -> io::Result<Arc<[u8]>> {
        let mut cached = self.0.read().await.get(path).cloned();
        let path_str = path.to_string_lossy(); // for logging

        // Validate the cache entry
        if let Some(e) = &cached {
            println!("Cache hit for {}.", &path_str);

            // Get file metadata
            let f_metadata = metadata(path).await;

            // Remove the cached entry if the file is not found
            if let Err(e) = f_metadata {
                println!(
                    "Cache file not found for {}, removing cache entry...",
                    &path_str
                );
                self.0.write().await.remove(path);
                return Err(e); // Return the error
            }
            let f_metadata = f_metadata.unwrap();

            // Remove the cached entry if the file has been modified
            if e.last_accessed < f_metadata.modified()? {
                println!(
                    "Cache file expired for {}, removing cache entry...",
                    &path_str
                );
                self.0.write().await.remove(path);
                cached = None; // Set cached to None
            }
        }

        // Return the cached file if it exists and is valid
        if let Some(e) = &cached {
            println!("Cache valid for {}, using cached file...", &path_str);
            return Ok(e.data.clone());
        }

        // Read the file into cache
        println!("Cache miss for {}, reading file...", &path_str);
        let data = fs::read(path).await?;
        let data: Arc<[u8]> = Arc::from(&data[..]);
        self.0.write().await.insert(
            path.into(),
            CacheEntry {
                data: data.clone(),
                last_accessed: SystemTime::now(),
            },
        );
        Ok(data)
    }
}
