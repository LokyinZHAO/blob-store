use std::{fs::File, num::NonZeroUsize, path::PathBuf};

use crate::error::Result;

pub struct FilePool {
    pool: dashmap::DashMap<PathBuf, File>,
    lru: parking_lot::Mutex<lru::LruCache<PathBuf, ()>>,
}

const DEFAULT_FILE_POOL_SIZE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(32) };

impl Default for FilePool {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_FILE_POOL_SIZE)
    }
}

impl FilePool {
    pub fn new() -> FilePool {
        Default::default()
    }

    pub fn with_capacity(capacity: NonZeroUsize) -> FilePool {
        Self {
            pool: dashmap::DashMap::with_capacity(capacity.get()),
            lru: parking_lot::Mutex::new(lru::LruCache::new(capacity)),
        }
    }

    pub fn get_or_insert_with<'a, F>(
        &'a self,
        path: PathBuf,
        default: F,
    ) -> Result<dashmap::mapref::entry::Entry<'a, PathBuf, File>>
    where
        F: FnOnce(&PathBuf) -> Result<File>,
    {
        let mut lru = self.lru.lock();
        if lru.get(&path).is_some() {
            return Ok(self.pool.entry(path));
        }
        let file = default(&path)?;
        lru.push(path.clone(), ())
            .map(|(evict, _)| self.pool.remove(&evict));
        self.pool.insert(path.clone(), file);
        Ok(self.pool.entry(path))
    }
}
