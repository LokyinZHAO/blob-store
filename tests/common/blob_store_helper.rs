use blob_store::error::Result as BlobResult;
use blob_store::prelude::*;
use rand::prelude::Rng;

use crate::common::gen_random;

const LOAD: usize = 4096;
const BLOB_SIZE_RANGE: std::ops::Range<usize> = 0..4096;

fn put_blobs(blob_store: &dyn BlobStore) -> Vec<(Key, Vec<u8>)> {
    let mut rng = rand::thread_rng();
    (0..LOAD)
        .map(|_| gen_random(rng.gen_range(BLOB_SIZE_RANGE.clone())))
        .inspect(|(key, data)| blob_store.put(*key, &data, PutOpt::Create).unwrap())
        .collect::<Vec<_>>()
}

fn check_blobs(blob_store: &dyn BlobStore, expect: &[(Key, Vec<u8>)]) {
    expect.iter().for_each(|(key, expect)| {
        let received = blob_store.get_owned(*key, GetOpt::All).unwrap();
        assert_eq!(expect, &received);
    });
}

/// expected to receive a clean store
pub fn write_read(blob_store: &dyn BlobStore) {
    let expect = put_blobs(blob_store);
    check_blobs(blob_store, &expect);
}

pub fn dump<F>(open: F)
where
    F: Fn() -> BlobResult<Box<dyn BlobStore>>,
{
    let store = open().unwrap();
    let expect = put_blobs(&*store);
    drop(store);
    // reopen
    let store = open().unwrap();
    check_blobs(&*store, &expect);
}
