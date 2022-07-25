use std::path::Path;
use sha2::{Sha256, Digest};
use std::fs::File;
use std::io;
use rusqlite::{Connection, Result, Error};

mod new;
mod changed;

const DB_PATH: &str = "userdata/notechart.db";

#[derive(Debug)]
pub enum CacheError {
    DirReadError,
    ConnectionError(Error),
    TransactionError,
    FileReadError,
    DbInsertError(Error),
    DbSelectError,
    DbDeleteError,
    DbUpdateError
}

impl std::error::Error for CacheError {}
impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CacheError::DirReadError => write!(f, "Failed to read directory"),
            CacheError::ConnectionError(e) => write!(f, "Failed to open connection: {}", e),
            CacheError::TransactionError => write!(f, "Failed to do transaction"),
            CacheError::FileReadError => write!(f, "Failed to read file"),
            CacheError::DbInsertError(e) => write!(f, "Failed to insert row: {}", e),
            CacheError::DbSelectError => write!(f, "Failed to select row"),
            CacheError::DbDeleteError => write!(f, "Failed to delete row"),
            CacheError::DbUpdateError => write!(f, "Failed to update row")
        }
    }
}

pub fn get_connection() -> Result<Connection, Error> {
    let db_path = Path::new(DB_PATH);

    let connection = match db_path.exists() {
        true => Connection::open(db_path)?,
        false => create_default(db_path)?
    };

    return Ok(connection)
}

fn create_default(db_path: &Path) -> Result<Connection, Error> {
    let connection = Connection::open(db_path)?;

    connection.execute(
        "CREATE TABLE notecharts (
            id	                INTEGER NOT NULL UNIQUE,
            set_id              INTEGER NOT NULL,
            hash	            TEXT NOT NULL UNIQUE,
            artist	            TEXT,
            title	            TEXT,
            version	            TEXT,
            path                TEXT NOT NULL,
            audio               TEXT,
            background          TEXT,
            PRIMARY             KEY(id AUTOINCREMENT)
        );",
        []
    )?;

    connection.execute(
        "CREATE TABLE notechart_sets (
            id	    INTEGER NOT NULL UNIQUE,
            path	TEXT UNIQUE,
            PRIMARY KEY(id AUTOINCREMENT)
        );",
        []
    )?;

    return Ok(connection)
}

#[derive(PartialEq)]
pub struct Set {
    pub id: i64,
    pub path: String
}
pub struct CachedNoteChart {
    pub id: i64,
    pub set: Option<Set>,
    pub artist: String,
    pub title: String,
    pub version: String,
    pub path: String,
    pub background: String,
    pub audio: String,
}

pub fn update(directories: &Vec<&str>) -> Result<(), CacheError> {
    let mut conn = match get_connection() {
        Ok(c) => c,
        Err(e) => return Err(CacheError::ConnectionError(e))
    };

    changed::fix_changed(&mut conn)?;

    for dir_path in directories {
        if let Err(e) = new::find_new(dir_path, &mut conn) {
            println!("Failed to process directory: '{}', error: {}.", dir_path, e);
        }
    }

    return Ok(())
}

pub fn get_hash(filepath: &str) -> Result<String, io::Error> {
    let mut hasher = Sha256::new();
    let mut filepath = File::open(filepath)?;

    io::copy(&mut filepath, &mut hasher)?;
    let hash = hasher.finalize();
    return Ok(format!("{:x}", hash))
}