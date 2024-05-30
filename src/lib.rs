use std::vec;

pub mod error;
mod store_impl;

pub mod prelude {
    pub use super::error::Error as BlobError;
    pub use super::error::Result as BlobResult;
    pub use super::store_impl::prelude::*;
    pub use super::*;
}

pub struct BlobMeta {
    pub size: usize,
}

pub type BlobRange = std::ops::Range<usize>;
pub type Offset = usize;

pub type Key = [u8; 20];

pub trait KeyLike {
    fn as_key(&self) -> Key;
}

impl KeyLike for u64 {
    fn as_key(&self) -> Key {
        let mut key = [0; 20];
        key[0..8].copy_from_slice(&self.to_be_bytes());
        key
    }
}

impl KeyLike for Key {
    fn as_key(&self) -> Key {
        *self
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PutOpt {
    /// Create the blob if it doesn't exist, fail if it does.
    Create,
    /// Replace the blob content if it exists, fail if it doesn't.
    Replace(Offset),
}

#[derive(Debug, Clone)]
pub enum GetOpt {
    /// Get all the content of the blob.
    All,
    /// Get a range of the blob.
    Range(BlobRange),
}

pub enum DeleteOpt {
    /// Delete the blob and return its content.
    Interest(BlobRange),
    /// Delete the blob and discard its content.
    Discard,
}

pub trait BlobStore {
    fn contains(&self, key: Key) -> error::Result<bool>;
    fn meta(&self, key: Key) -> error::Result<BlobMeta>;
    fn put(&self, key: Key, value: &[u8], opt: PutOpt) -> error::Result<()>;
    fn get(&self, key: Key, buf: &mut [u8], opt: GetOpt) -> error::Result<()>;
    fn get_owned(&self, key: Key, opt: GetOpt) -> error::Result<Vec<u8>> {
        let len = match &opt {
            GetOpt::All => self.meta(key)?.size,
            GetOpt::Range(range) => range.end - range.start,
        };
        let mut buf = vec![0_u8; len];
        self.get(key, &mut buf, opt).map(|_| buf)
    }
    fn delete(&self, key: Key, opt: DeleteOpt) -> error::Result<Option<Vec<u8>>>;
}
