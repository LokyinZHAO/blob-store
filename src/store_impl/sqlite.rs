use std::{
    io::prelude::{Seek, Write},
    path,
};

use rusqlite::blob::{Blob, ZeroBlob};

use crate::{
    error::{Error, Result},
    BlobStore, Key,
};

type RowID = i64;
type Mutex<T> = parking_lot::Mutex<T>;
type Lock<'a, T> = parking_lot::MutexGuard<'a, T>;
type Map<K, V> = dashmap::DashMap<K, V>;
type KeyToRowIDMap = Map<Key, RowID>;
type ConnLock<'a> = Lock<'a, rusqlite::Connection>;

pub struct SqliteBlobStore {
    root: path::PathBuf,
    conn: Mutex<rusqlite::Connection>,
    key_to_row_map: KeyToRowIDMap,
}

impl SqliteBlobStore {
    const DATABASE_NAME: rusqlite::DatabaseName<'static> = rusqlite::MAIN_DB;
    const TABLE_NAME: &'static str = "blobs";
    const COLUMN_NAME: &'static str = "content";
    const SQL_INSERT: &'static str = concat!("INSERT INTO blobs (content) VALUES (?)",);
    const SQL_DELETE: &'static str = concat!("DELETE FROM blobs WHERE rowid = (?)",);
    const SQL_CREATE_TABLE: &'static str =
        concat!("CREATE TABLE IF NOT EXISTS blobs ( content BLOB NOT NULL )",);
    const DB_FILE: &'static str = "blobs.db";
    const MAP_FILE: &'static str = "blobs.map.dump";

    pub fn connect(path: &path::Path) -> Result<Self> {
        let db_path = {
            let mut path = path.to_path_buf();
            path.push(Self::DB_FILE);
            path
        };
        let map_path = {
            let mut path = path.to_path_buf();
            path.push(Self::MAP_FILE);
            path
        };
        let conn = rusqlite::Connection::open(db_path.as_path())?;
        conn.execute(Self::SQL_CREATE_TABLE, [])?;
        let map = if map_path.exists() {
            bincode::deserialize_from(std::fs::File::open(map_path)?)
                .map_err(|e| anyhow::Error::new(e))?
        } else {
            KeyToRowIDMap::new()
        };
        Ok(Self {
            conn: Mutex::new(conn),
            key_to_row_map: map,
            root: path.into(),
        })
    }

    fn open_blob<'l>(
        map: &KeyToRowIDMap,
        conn_lock: &'l ConnLock<'_>,
        key: &Key,
        read_only: bool,
    ) -> Result<Blob<'l>> {
        map.get(key)
            .map(|row_id| *row_id.value())
            .map(|row_id| {
                conn_lock
                    .blob_open(
                        Self::DATABASE_NAME,
                        Self::TABLE_NAME,
                        Self::COLUMN_NAME,
                        row_id,
                        read_only,
                    )
                    .map_err(Error::from)
            })
            .ok_or_else(|| crate::error::BlobError::NotFound)
            .map_err(Error::from)
            .and_then(std::convert::identity)
    }
}

impl BlobStore for SqliteBlobStore {
    fn contains(&self, key: Key) -> crate::error::Result<bool> {
        Ok(self.key_to_row_map.contains_key(&key))
    }

    fn meta(&self, key: Key) -> crate::error::Result<crate::BlobMeta> {
        let conn_lock = self.conn.lock();
        let size = Self::open_blob(&self.key_to_row_map, &conn_lock, &key, true)?.len();
        Ok(crate::BlobMeta { size })
    }

    fn put(&self, key: Key, value: &[u8], opt: crate::PutOpt) -> crate::error::Result<()> {
        let conn_lock = self.conn.lock();
        let mut blob = match &opt {
            crate::PutOpt::Create => {
                if self.key_to_row_map.contains_key(&key) {
                    return Err(crate::error::BlobError::AlreadyExists.into());
                }
                conn_lock.execute(
                    Self::SQL_INSERT,
                    [ZeroBlob(value.len().try_into().unwrap())],
                )?;
                let row_id = conn_lock.last_insert_rowid();
                self.key_to_row_map.insert(key, row_id);
                Self::open_blob(&self.key_to_row_map, &conn_lock, &key, false)?
            }
            crate::PutOpt::Replace(range) => {
                let mut blob = Self::open_blob(&self.key_to_row_map, &conn_lock, &key, false)?;
                // check range
                let size = blob.len();
                let valid_range = 0..size;
                if !valid_range.contains(&range.start) || !valid_range.contains(&range.end) {
                    return Err(crate::error::BlobError::RangeError.into());
                }
                if value.len() != range.len() {
                    return Err(crate::error::BlobError::RangeError.into());
                }
                blob.seek(std::io::SeekFrom::Start((range.start).try_into().unwrap()))?;
                blob
            }
        };
        blob.write(value)?;
        Ok(())
    }

    fn get(&self, key: Key, buf: &mut [u8], opt: crate::GetOpt) -> crate::error::Result<()> {
        let key = key;
        let conn_lock = self.conn.lock();
        let mut blob = Self::open_blob(&self.key_to_row_map, &conn_lock, &key, true)?;
        match &opt {
            crate::GetOpt::All => {
                if blob.len() != buf.len() {
                    return Err(crate::error::BlobError::RangeError.into());
                }
                blob.read_at_exact(buf, 0)?;
            }
            crate::GetOpt::Range(range) => {
                let len = range.end - range.start;
                if len != buf.len() {
                    return Err(crate::error::BlobError::RangeError.into());
                }
                let valid_range = 0..blob.len();
                if !valid_range.contains(&range.start) || !valid_range.contains(&range.end) {
                    return Err(crate::error::BlobError::RangeError.into());
                }
                blob.seek(std::io::SeekFrom::Start(range.start.try_into().unwrap()))?;
                blob.read_at_exact(buf, range.start)?;
            }
        }
        Ok(())
    }

    fn delete(&self, key: Key, opt: crate::DeleteOpt) -> crate::error::Result<Option<Vec<u8>>> {
        let key = key;
        let row_id = match self.key_to_row_map.remove(&key) {
            Some((_, row_id)) => row_id,
            None => return Err(crate::error::BlobError::NotFound.into()),
        };
        if let crate::DeleteOpt::Interest(_) = &opt {
            unimplemented!("Interest delete not implemented, use \"get\" before delete instead");
            // let mut blob = self.open_blob(&key, false)?;
            // let size = blob.len();
            // interest.reserve_exact(size - interest.len());
            // blob.raw_read_at_exact()?;
        }
        self.conn.lock().execute(Self::SQL_DELETE, [row_id])?;
        Ok(None)
    }
}

impl Drop for SqliteBlobStore {
    fn drop(&mut self) {
        let map_path = {
            let mut path = self.root.clone();
            path.push(Self::MAP_FILE);
            path
        };

        bincode::serialize_into(
            std::fs::File::options()
                .truncate(true)
                .read(true)
                .write(true)
                .create(true)
                .open(map_path)
                .expect("failed to open map file"),
            &self.key_to_row_map,
        )
        .unwrap();
    }
}
