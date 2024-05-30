use blob_store::prelude::Key;

mod blob_store_helper;

pub use blob_store_helper::*;

fn gen_random(size: usize) -> (Key, Vec<u8>) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let data = (&mut rng)
        .sample_iter(rand::distributions::Standard)
        .take(size)
        .collect();
    let mut key: Key = [0; 20];
    rng.fill(&mut key[..]);
    (key, data)
}
