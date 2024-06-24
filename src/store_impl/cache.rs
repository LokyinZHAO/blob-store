use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
};

use lru::LruCache;

use crate::{error::Result, BlobMeta, BlobRange, BlobStore, Key};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct PageIndex(usize);

type PageId = (Key, PageIndex);
type Page = Vec<u8>;
pub struct MemoryCache<S, const PAGE_SIZE: usize>
where
    S: BlobStore,
{
    store: S,
    meta_cache: RefCell<HashMap<Key, (BlobMeta, std::collections::BTreeSet<PageIndex>)>>, // cache meta and in-cache pages' index
    cache: RefCell<LruCache<PageId, Page>>,
    free_pages: RefCell<Vec<Page>>,
}

impl<S, const PAGE_SIZE: usize> MemoryCache<S, PAGE_SIZE>
where
    S: BlobStore,
{
    /// create cache with capacity in number of pages
    pub fn with_capacity(store: S, size: std::num::NonZeroUsize) -> Self {
        let cache = RefCell::new(LruCache::new(size));
        let free_pages = RefCell::new(vec![vec![0_u8; PAGE_SIZE]; size.get() + 16]);
        Self {
            store,
            meta_cache: Default::default(),
            cache,
            free_pages,
        }
    }

    #[inline]
    fn range_to_page_idx(range: &BlobRange) -> PageIndex {
        PageIndex(range.start / PAGE_SIZE)
    }

    #[inline]
    fn page_idx_to_range(idx: PageIndex) -> BlobRange {
        let idx = idx.0;
        (idx * PAGE_SIZE)..((idx + 1) * PAGE_SIZE)
    }

    /// FIX: blob is smaller than a page size
    fn split_range<const SIZE: usize>(
        range: BlobRange,
        blob_size: usize,
    ) -> impl Iterator<Item = (PageIndex, BlobRange)> {
        let mut aligned = range.clone();
        let head = if range.start % SIZE != 0 && SIZE <= blob_size {
            let idx = Self::range_to_page_idx(&range);
            let end = usize::try_from(idx.0 + 1).unwrap() * SIZE;
            aligned.start = end;
            Some((idx, (range.start..end)))
        } else {
            None
        };
        let tail = if range.end % SIZE != 0 {
            let idx = Self::range_to_page_idx(&range);
            let start = usize::try_from(idx.0).unwrap() * SIZE;
            aligned.end = start;
            Some((idx, (start..range.end)))
        } else {
            None
        };
        let aligned = (aligned.start / PAGE_SIZE..aligned.end / PAGE_SIZE)
            .map(|idx| (PageIndex(idx), Self::page_idx_to_range(PageIndex(idx))));
        head.into_iter().chain(aligned).chain(tail)
    }
}

impl<S, const PAGE_SIZE: usize> MemoryCache<S, PAGE_SIZE>
where
    S: BlobStore,
{
    fn free_page(&self, key: Key, range: BlobRange, page: Page) -> crate::error::Result<()> {
        self.store
            .put(key, &page[range.clone()], crate::PutOpt::Replace(range))?;
        self.free_pages.borrow_mut().push(page);
        Ok(())
    }

    fn cache_page(
        &self,
        cache: &mut RefMut<'_, LruCache<PageId, Page>>,
        key: Key,
        page_idx: PageIndex,
        page: Page,
    ) -> crate::error::Result<()> {
        let mut meta_cache = self.meta_cache.borrow_mut();
        if let Some((_, pages_set)) = meta_cache.get_mut(&key) {
            pages_set.insert(page_idx);
        } else {
            let mut page_set = std::collections::BTreeSet::new();
            page_set.insert(page_idx);
            meta_cache.insert(key, (self.store.meta(key)?, page_set));
        }
        let evict = cache.push((key, page_idx), page);
        if let Some(((key, page_idx), evict_page)) = evict {
            let (_, page_set) = meta_cache.get_mut(&key).unwrap();
            page_set.remove(&page_idx);
            if page_set.is_empty() {
                meta_cache.remove(&key);
            }
            let mut evict_range = Self::page_idx_to_range(page_idx);
            drop(meta_cache);
            let blob_size = self.meta(key)?.size;
            if evict_range.end > blob_size {
                evict_range.end = blob_size;
            }
            self.free_page(key, evict_range, evict_page)?;
        }
        Ok(())
    }
}

impl<S, const PAGE_SIZE: usize> BlobStore for MemoryCache<S, PAGE_SIZE>
where
    S: BlobStore,
{
    fn contains(&self, key: Key) -> crate::error::Result<bool> {
        Ok(self.meta_cache.borrow().contains_key(&key) || self.store.contains(key)?)
    }

    fn meta(&self, key: Key) -> crate::error::Result<crate::BlobMeta> {
        if let Some(meta) = self
            .meta_cache
            .borrow()
            .get(&key)
            .map(|(meta, _)| meta.clone())
        {
            Ok(meta)
        } else {
            self.store.meta(key)
        }
    }

    fn put(&self, key: Key, value: &[u8], opt: crate::PutOpt) -> crate::error::Result<()> {
        let range = match &opt {
            // do not cache the newly created page
            crate::PutOpt::Create => {
                return self.store.put(key, value, crate::PutOpt::Create);
            }
            crate::PutOpt::Replace(range) => range.clone(),
        };
        // check range
        let meta = self.meta(key)?;
        if range.len() != value.len() {
            return Err(crate::error::BlobError::RangeError.into());
        }
        let valid_range = 0..meta.size;
        if !crate::store_impl::helpers::range_contains(&valid_range, &range) {
            return Err(crate::error::BlobError::RangeError.into());
        }
        let mut cache = self.cache.borrow_mut();
        let mut buf_offset: usize = 0;
        Self::split_range::<PAGE_SIZE>(range, meta.size).try_for_each(
            |(page_idx, in_blob_range)| -> Result<()> {
                let in_buf_range = buf_offset..(buf_offset + in_blob_range.len());
                let in_cache_range = in_blob_range.start % PAGE_SIZE..in_blob_range.end % PAGE_SIZE;
                if let Some(cached_page) = cache.get_mut(&(key, page_idx)) {
                    // cache hit
                    cached_page[in_cache_range].copy_from_slice(&value[in_buf_range]);
                } else {
                    //cache miss
                    let mut page = self.free_pages.borrow_mut().pop().expect("no free page");
                    self.store
                        .get(key, &mut page, crate::GetOpt::Range(in_blob_range.clone()))?;
                    page[in_cache_range].copy_from_slice(&value[in_buf_range]);
                    self.cache_page(&mut cache, key, page_idx, page)?;
                }
                buf_offset += in_blob_range.len();
                Ok(())
            },
        )
    }

    fn get(&self, key: Key, buf: &mut [u8], opt: crate::GetOpt) -> crate::error::Result<()> {
        let meta = self.meta(key)?;
        let range = match &opt {
            crate::GetOpt::All => {
                let size = meta.size;
                0..size
            }
            crate::GetOpt::Range(range) => range.clone(),
        };
        // check range
        if range.len() != buf.len() {
            return Err(crate::error::BlobError::RangeError.into());
        }
        let valid_range = 0..meta.size;
        if !crate::store_impl::helpers::range_contains(&valid_range, &range) {
            return Err(crate::error::BlobError::RangeError.into());
        }
        let mut cache = self.cache.borrow_mut();
        let mut buf_offset = 0;
        Self::split_range::<PAGE_SIZE>(range, meta.size).try_for_each(
            |(page_idx, in_blob_range)| -> Result<()> {
                let in_buf_range = buf_offset..(buf_offset + in_blob_range.len());
                let in_cache_range = in_blob_range.start % PAGE_SIZE..in_blob_range.end % PAGE_SIZE;
                debug_assert!(in_blob_range.len() < PAGE_SIZE);
                if let Some(cached_page) = cache.get(&(key, page_idx)) {
                    // cache hit
                    buf[in_buf_range].copy_from_slice(&cached_page[in_cache_range]);
                } else {
                    // cache miss, load from store
                    // let aligned_cache_range = in_blob_range.start / PAGE_SIZE * PAGE_SIZE
                    //     ..in_blob_range.start / PAGE_SIZE * PAGE_SIZE + PAGE_SIZE;
                    let mut page = self.free_pages.borrow_mut().pop().expect("no free page");
                    self.store.get(
                        key,
                        &mut page[in_cache_range.clone()],
                        crate::GetOpt::Range(in_blob_range.clone()),
                    )?;
                    buf[in_buf_range].copy_from_slice(&page[in_cache_range]);
                    self.cache_page(&mut cache, key, page_idx, page)?;
                }
                buf_offset += in_blob_range.len();
                Ok(())
            },
        )
    }

    fn delete(&self, key: Key, opt: crate::DeleteOpt) -> crate::error::Result<Option<Vec<u8>>> {
        if let crate::DeleteOpt::Interest(_) = opt {
            unimplemented!("delete with interest is not supported yet")
        }
        let mut cache = self.cache.borrow_mut();
        let mut free_pages = self.free_pages.borrow_mut();
        if let Some((meta, page_set)) = self.meta_cache.borrow_mut().remove(&key) {
            for page_idx in page_set {
                let page = cache.pop(&(key, page_idx)).unwrap();
                let mut range = Self::page_idx_to_range(page_idx);
                if range.end > meta.size {
                    range.end = meta.size;
                }
                self.store.put(key, &page, crate::PutOpt::Replace(range))?;
                free_pages.push(page);
            }
        }
        self.store.delete(key, opt)
    }
}

impl<S, const PAGE_SIZE: usize> Drop for MemoryCache<S, PAGE_SIZE>
where
    S: BlobStore,
{
    fn drop(&mut self) {
        let mut cache = self.cache.borrow_mut();
        while let Some(((key, page_idx), page)) = cache.pop_lru() {
            let blob_size = self.store.meta(key).unwrap().size;
            let mut range = Self::page_idx_to_range(page_idx);
            if range.end > blob_size {
                range.end = blob_size;
            }
            self.store
                .put(key, &page, crate::PutOpt::Replace(range))
                .unwrap();
        }
    }
}
