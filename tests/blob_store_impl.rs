mod common;

use std::sync::Arc;

use blob_store::prelude::*;

#[test]
fn test_local_fs() {
    // write read
    let tmp_dir = tempfile::tempdir().unwrap();
    let store = LocalFileSystemBlobStore::connect(tmp_dir.path()).unwrap();
    common::write_read(&store);
    // dump
    let tmp_dir = tempfile::tempdir().unwrap();
    common::dump(|| {
        LocalFileSystemBlobStore::connect(tmp_dir.path())
            .map(|obj| -> Box<dyn BlobStore> { Box::new(obj) })
            .map_err(Into::into)
    });
    // concurrecy
    let tmp_dir = tempfile::tempdir().unwrap();
    let store = Arc::new(LocalFileSystemBlobStore::connect(tmp_dir.path()).unwrap());
    common::concurrent(store);
}

#[test]
#[cfg(feature = "sqlite")]
fn test_sqlite() {
    // write read
    let tmp_dir = tempfile::tempdir().unwrap();
    let store = SqliteBlobStore::connect(tmp_dir.path()).unwrap();
    common::write_read(&store);
    // dump
    let tmp_dir = tempfile::tempdir().unwrap();
    common::dump(|| {
        SqliteBlobStore::connect(tmp_dir.path())
            .map(|obj| -> Box<dyn BlobStore> { Box::new(obj) })
            .map_err(Into::into)
    });
    // concurrecy
    let tmp_dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SqliteBlobStore::connect(tmp_dir.path()).unwrap());
    common::concurrent(store);
}
