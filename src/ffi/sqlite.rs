use crate::prelude::*;

fn blob_store_connect(path: &cxx::CxxString) -> crate::error::Result<Box<SqliteBlobStore>> {
    SqliteBlobStore::connect(path.to_str().unwrap()).map(|obj| Box::new(obj))
}

#[cxx::bridge(namespace = "blob_store::sqlite")]
mod ffi {

    extern "Rust" {
        type SqliteBlobStore;
        fn blob_store_connect(path: &CxxString) -> Result<Box<SqliteBlobStore>>;
        // fn create(store: &SqliteBlobStore, key: Key, value: &[u8]) -> Result<()>;
        // fn put(store: &SqliteBlobStore, key: Key, value: &[u8], offset: usize) -> Result<()>;
        // fn get(store: &SqliteBlobStore, key: Key, buf: &mut [u8]) -> Result<()>;
    }
}
