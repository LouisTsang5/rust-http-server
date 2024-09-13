use std::{
    collections::HashMap,
    io::Cursor,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    // time::SystemTime,
};
use tokio::{
    fs::File,
    io::{self, AsyncRead, AsyncReadExt},
    sync::{RwLock, RwLockWriteGuard},
};

use crate::{debug, log_ctx, timer};

const FILE_BUFF_INIT_SIZE: usize = crate::BUFF_INIT_SIZE * 8;
log_ctx!("FileCache");

#[derive(Clone, Debug)]
pub struct CacheEntry {
    data: Arc<[u8]>,
    // last_accessed: SystemTime,
}

struct FileCacheInner {
    cache: HashMap<PathBuf, CacheEntry>,
    size_limit: Option<usize>,
    cur_size: usize,
}

pub struct FileCache(RwLock<FileCacheInner>);

struct FileCacheInsertOk {
    new_entry: CacheEntry,
}

enum FileCacheInsertError {
    CacheFull, // Cache is full. The option contains the removed entry on insert attempt
    IoError(io::Error), // IO Error
}

impl From<io::Error> for FileCacheInsertError {
    fn from(e: io::Error) -> Self {
        Self::IoError(e)
    }
}

#[derive(Debug)]
pub enum AbstractFile {
    File(File, usize),
    CacheEntry(Cursor<Arc<[u8]>>, usize),
}

impl AbstractFile {
    pub fn from_file(file: File, size: usize) -> Self {
        Self::File(file, size)
    }

    pub fn len(&self) -> usize {
        match self {
            Self::File(_, s) => *s,
            Self::CacheEntry(_, s) => *s,
        }
    }
}

impl From<Arc<[u8]>> for AbstractFile {
    fn from(data: Arc<[u8]>) -> Self {
        let len = data.len();
        Self::CacheEntry(Cursor::new(data), len)
    }
}

impl AsyncRead for AbstractFile {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::File(f, _) => Pin::new(f).poll_read(cx, buf),
            Self::CacheEntry(c, _) => Pin::new(c).poll_read(cx, buf),
        }
    }
}

impl FileCache {
    pub fn new(size_limit: Option<usize>) -> Self {
        let inner = FileCacheInner {
            cache: HashMap::new(),
            size_limit,
            cur_size: 0,
        };
        Self(RwLock::new(inner))
    }

    async fn get(&self, path: &Path) -> Option<CacheEntry> {
        self.0.read().await.cache.get(path).cloned()
    }

    fn _remove(
        &self,
        path: &Path,
        write_guard: &mut RwLockWriteGuard<FileCacheInner>,
    ) -> Option<CacheEntry> {
        let removed = write_guard.cache.remove(path);
        if let Some(r) = &removed {
            write_guard.cur_size -= r.data.len();
            debug!(
                "Cache entry removed for {}, current cache size: {}.",
                path.display(),
                write_guard.cur_size
            );
        }
        removed
    }

    pub async fn remove(&self, path: &Path) -> Option<CacheEntry> {
        let mut write_guard = self.0.write().await;
        self._remove(path, &mut write_guard)
    }

    async fn insert(
        &self,
        path: &Path,
        file: &mut File,
        f_size: usize,
    ) -> Result<FileCacheInsertOk, FileCacheInsertError> {
        // Obtain write guard
        // Write guard is held until the end of the function to ensure cache size limit is enforced
        let mut write_guard = self.0.write().await;

        // try remove old entry
        let _ = self._remove(path, &mut write_guard);

        // check if new entry can be inserted
        let can_insert = match &write_guard.size_limit {
            Some(limit) => write_guard.cur_size + f_size <= *limit,
            None => true,
        };

        // Return Err if new entry cannot be inserted
        if !can_insert {
            debug!(
                "Cache entry cannot be inserted for {}, cache size limit reached. Current cache size: {}. New entry size: {}.",
                path.display(),
                write_guard.cur_size,
                f_size
            );
            return Err(FileCacheInsertError::CacheFull);
        }

        // Read file to buffer
        let mut buf = Vec::with_capacity(FILE_BUFF_INIT_SIZE);
        file.read_to_end(&mut buf).await?;

        // insert new entry
        write_guard.cur_size += buf.len();
        let new_entry = CacheEntry {
            data: buf.into(),
            // last_accessed: SystemTime::now(),
        };
        write_guard.cache.insert(path.into(), new_entry.clone());

        debug!(
            "Cache entry inserted for {}, current cache size: {}.",
            path.display(),
            write_guard.cur_size
        );

        // return ok
        Ok(FileCacheInsertOk { new_entry })
    }

    pub async fn open(&self, path: &Path) -> io::Result<AbstractFile> {
        timer!("FileCache::open");
        let cached = self.get(path).await;
        let path_str = path.display(); // for logging

        // Return the cached file if it exists and is valid
        if let Some(e) = cached {
            debug!("Cache valid for {}, using cached file...", &path_str);
            return Ok(AbstractFile::from(e.data));
        }

        // Read the file into cache
        debug!("Cache miss for {}, reading file...", &path_str);
        let mut file = File::open(path).await?;
        let f_size = file.metadata().await?.len() as usize;
        let retval = match self.insert(path.into(), &mut file, f_size).await {
            Ok(cached) => Ok(AbstractFile::from(cached.new_entry.data)),
            Err(e) => match e {
                FileCacheInsertError::IoError(e) => Err(e),
                FileCacheInsertError::CacheFull => Ok(AbstractFile::from_file(file, f_size)),
            },
        }?;
        Ok(retval)
    }
}
