use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    BlobError(#[from] BlobError),
    #[error("OS I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(feature = "sqlite")]
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum BlobError {
    #[error("blob not found")]
    NotFound,
    #[error("blob already exists")]
    AlreadyExists,
    #[error("blob range out of bounds")]
    RangeError,
}
