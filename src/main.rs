use blob_store::Key;
use rand::Rng;

type LoadRecord = (blob_store::prelude::Key, BlobOps);

fn main() {
    const HELP_MSG: &str = "Usage: blobstore <device path> <test load file>";
    const MAX_LOAD: usize = 1024 * 1024;
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() != 3 {
        panic!("{}", HELP_MSG);
    }
    let device_path = std::path::PathBuf::from(args[1].as_str());
    let test_load_file = args[2].as_str();
    println!("benchmark blob store");
    println!("warming up");
    let mut rng = rand::thread_rng();
    let f = std::fs::File::open(test_load_file).unwrap();
    let load = csv::Reader::from_reader(std::io::BufReader::new(f))
        .records()
        .take(MAX_LOAD)
        .filter_map(Result::ok)
        .map(parse_record)
        .map(|Record { blob, read, .. }| {
            let blob_id = blob.blob_name.as_bytes().chunks(Key::default().len()).fold(
                Key::default(),
                |mut acc, x| {
                    acc.iter_mut().zip(x).for_each(|(acc, x)| *acc ^= x);
                    acc
                },
            );
            let ops = if read {
                BlobOps::Read
            } else {
                BlobOps::Write((0..blob.blob_bytes).map(|_| rng.gen::<u8>()).collect())
            };
            (blob_id, ops)
        })
        .collect::<Vec<_>>();
    #[cfg(feature = "local_fs")]
    {
        let path = {
            let mut p = device_path.clone();
            p.push("local_fs");
            if p.exists() {
                std::fs::remove_dir_all(p.as_path()).unwrap();
            }
            p
        };
        std::fs::create_dir_all(path.as_path()).unwrap();
        let blob_store = blob_store::prelude::LocalFileSystemBlobStore::connect(path).unwrap();
        let result = bench_backend(&blob_store, &load);
        println!("local fs benchmark:\n{result}");
    }
    #[cfg(feature = "memmap")]
    {
        let path = {
            let mut p = device_path.clone();
            p.push("memmap");
            if p.exists() {
                std::fs::remove_dir_all(p.as_path()).unwrap();
            }
            p
        };
        std::fs::create_dir_all(path.as_path()).unwrap();
        let blob_store = blob_store::prelude::MemMapStore::connect(path).unwrap();
        let result = bench_backend(&blob_store, &load);
        println!("memmap benchmark:\n{result}");
    }
    #[cfg(feature = "sqlite")]
    {
        let path = {
            let mut p = device_path.clone();
            p.push("sqlite");
            if p.exists() {
                std::fs::remove_dir_all(p.as_path()).unwrap();
            }
            p
        };
        std::fs::create_dir_all(path.as_path()).unwrap();
        let blob_store = blob_store::prelude::SqliteBlobStore::connect(path).unwrap();
        let result = bench_backend(&blob_store, &load);
        println!("sqlite benchmark:\n{result}");
    }
}

fn parse_record(record: csv::StringRecord) -> Record {
    // csv HEAD format
    // Timestamp,AnonRegion,AnonUserId,AnonAppName,AnonFunctionInvocationId,AnonBlobName,BlobType,AnonBlobETag,BlobBytes,Read,Write
    let timestamp = std::time::UNIX_EPOCH
        + std::time::Duration::from_millis(record.get(0).unwrap().parse::<u64>().unwrap());
    let region = record.get(1).unwrap().to_string();
    let user_id = record.get(2).unwrap().parse::<usize>().unwrap();
    let app_name = record.get(3).unwrap().to_string();
    let func_id = record.get(4).unwrap().parse::<usize>().unwrap();
    let blob = BlobRecord {
        blob_name: record.get(5).unwrap().to_string(),
        blob_type: record.get(6).unwrap().to_string(),
        blob_tag: record.get(7).unwrap().to_string(),
        blob_bytes: record
            .get(8)
            .unwrap()
            .parse::<f64>()
            .unwrap_or_default()
            .round() as usize,
    };
    let read = record
        .get(9)
        .unwrap()
        .to_lowercase()
        .parse::<bool>()
        .unwrap();
    let write = record
        .get(10)
        .unwrap()
        .to_lowercase()
        .parse::<bool>()
        .unwrap();
    Record {
        timestamp,
        region,
        user_id,
        app_name,
        func_id,
        blob,
        read,
        write,
    }
}

fn bench_backend(blob_store: &dyn blob_store::BlobStore, load: &[LoadRecord]) -> BenchResult {
    let mut result = BenchResult::default();
    for (blob_id, ops) in load {
        match ops {
            BlobOps::Read => {
                let start = std::time::Instant::now();
                if blob_store.contains(*blob_id).unwrap() {
                    let val = blob_store
                        .get_owned(*blob_id, blob_store::GetOpt::All)
                        .unwrap();
                    result.read += 1;
                    result.read_size += val.len();
                } else {
                    result.read_non_exist += 1;
                }
                result.read_time += start.elapsed();
            }
            BlobOps::Write(data) => {
                let start = std::time::Instant::now();
                if blob_store.contains(*blob_id).unwrap() {
                    let size = std::cmp::min(blob_store.meta(*blob_id).unwrap().size, data.len());
                    blob_store
                        .put(
                            *blob_id,
                            &data[0..size],
                            blob_store::PutOpt::Replace(0..size),
                        )
                        .unwrap();
                    result.replace += 1;
                    result.write_size += size;
                } else {
                    blob_store
                        .put(*blob_id, &data, blob_store::PutOpt::Create)
                        .unwrap();
                    result.create += 1;
                    result.write_size += data.len();
                }
                result.write_time += start.elapsed();
            }
        }
    }
    result
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct Record {
    timestamp: std::time::SystemTime,
    region: String,
    user_id: usize,
    app_name: String,
    func_id: usize,
    blob: BlobRecord,
    read: bool,
    write: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct BlobRecord {
    blob_name: String,
    blob_type: String,
    blob_tag: String,
    blob_bytes: usize,
}

enum BlobOps {
    Read,
    Write(Vec<u8>),
}

#[derive(Default)]
struct BenchResult {
    read: usize,
    read_non_exist: usize,
    create: usize,
    replace: usize,
    read_time: std::time::Duration,
    write_time: std::time::Duration,
    read_size: usize,
    write_size: usize,
}

impl std::fmt::Display for BenchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let thrput = bench_throughput(self);
        let str = format!(
            "[LOAD] read: {}; read non-exist: {}; create: {}, replace: {}\n[SIZE] read: {:.2}MB; write: {:.2}MB\n[TIME] read: {:.3} s; write: {:.3} s\n[THRPUT] read: {:.3} MB/s; write: {:.3} MB/s",
            self.read, self.read_non_exist, self.create, self.replace
            , f64::try_from(self.read_size as u32).unwrap() , f64::try_from(self.write_size as u32).unwrap(), self.read_time.as_secs_f64(), self.write_time.as_secs_f64()
            ,thrput.0, thrput.1
        );
        f.write_str(&str)
    }
}

/// calculate benchmark (read, write) throughput (in MB/s)
fn bench_throughput(bench_result: &BenchResult) -> (f64, f64) {
    (
        f64::try_from(bench_result.read_size as u32).unwrap()
            / 1024_f64
            / 1024_f64
            / bench_result.read_time.as_secs_f64(),
        f64::try_from(bench_result.write_size as u32).unwrap()
            / 1024_f64
            / 1024_f64
            / bench_result.write_time.as_secs_f64(),
    )
}
