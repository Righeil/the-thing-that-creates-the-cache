use std::io::Error as IoError;
use rusqlite::Error as DbError;

#[derive(Debug)]
pub enum CacheError {
    DirReadError(IoError),
    ConnectionError(DbError),
    TransactionError(DbError),
    FileReadError(IoError),
    DbInsertError(DbError),
    DbSelectError(DbError),
    DbDeleteError(DbError),
    DbUpdateError(DbError)
}

impl std::error::Error for CacheError {}
impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CacheError::DirReadError(e) => write!(f, "Failed to read directory: {}", e),
            CacheError::ConnectionError(e) => write!(f, "Failed to open connection: {}", e),
            CacheError::TransactionError(e) => write!(f, "Failed to do transaction: {}", e),
            CacheError::FileReadError(e) => write!(f, "Failed to read file: {}", e),
            CacheError::DbInsertError(e) => write!(f, "Failed to insert row: {}", e),
            CacheError::DbSelectError(e) => write!(f, "Failed to select row: {}", e),
            CacheError::DbDeleteError(e) => write!(f, "Failed to delete row: {}", e),
            CacheError::DbUpdateError(e) => write!(f, "Failed to update row: {}", e)
        }
    }
}