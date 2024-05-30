#![allow(unused)]
fn main() {
    use blob_store::BlobStore;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn foo(id: usize) -> usize {
        id
    }

    pub fn read(c: &mut Criterion) {
        c.bench_function("fib 20", |b| b.iter(|| foo(black_box(20))));
    }

    criterion_group!(benches, read);
    criterion_main!(benches);
}
