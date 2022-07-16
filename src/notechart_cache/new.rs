use std::path::Path;
use std::collections::HashSet;
use walkdir::WalkDir;
use crate::notechart_cache;
use notechart_cache::{CacheError, CachedNoteChart};
use rusqlite::{Connection, Result, params};

pub fn find_new(dir_path: &str, conn: &mut Connection) -> Result<(), CacheError>{
    let notecharts = match get_notechart_paths(dir_path) {
        Ok(n) => n,
        Err(_) => return Err(CacheError::DirReadError)
    };

    println!("Found {} notecharts", notecharts.len());

    let mut notecharts_to_insert: Vec<String> = vec![];
    let mut existing_hashes: HashSet<String> = HashSet::new();
    
    for path in notecharts {
        let hash = match notechart_cache::get_hash(&path) {
            Ok(h) => h,
            Err(_) => {
                println!("Failed to read file {}", path);
                return Err(CacheError::FileReadError)
            }
        };
        
        let is_exists = match is_notechart_exists(&hash, &conn) {
            Ok(b) => b,
            Err(_) => return Err(CacheError::DbSelectError)
        };

        if is_exists || existing_hashes.contains(&hash) {
            continue
        }

        notecharts_to_insert.push(path);
        existing_hashes.insert(hash);
    }

    let tr = match conn.transaction() {
        Ok(t) => t,
        Err(_) => return Err(CacheError::TransactionError)
    };

    for path in notecharts_to_insert {
        let notechart = match get_notechart_data(&path) {
            Ok(n) => n,
            Err(_) => continue
        };

        let result = tr.execute(
            "INSERT INTO notecharts (hash, artist, title, version, path, audio, background) 
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                notechart.hash, 
                notechart.artist, 
                notechart.title, 
                notechart.version, 
                notechart.path, 
                notechart.audio, 
                notechart.background
            ],
        );

        if let Err(e) = result {
            return Err(CacheError::DbInsertError(e))
        }
    }

    if let Err(_) = tr.commit() {
        return Err(CacheError::TransactionError)
    }

    return Ok(())
}

fn get_notechart_data(path: &str) -> Result<CachedNoteChart, CacheError>{
    let info = match osu_parser::import_info(&path) {
        Ok(i) => i,
        Err(_) => return Err(CacheError::FileReadError)
    };

    let hash = match notechart_cache::get_hash(&path) {
        Ok(h) => h,
        Err(_) => return Err(CacheError::FileReadError)
    };

    let artist = info.metadata.artist;
    let title = info.metadata.title;
    let version = info.metadata.version;
    let audio = info.general.audio_filename;
    let path = path.to_string();

    let mut events_iter = info.events.data.iter();
    let background: String;

    if let Some(e) = events_iter.find(|e| e.e_type == "0") {
        background = e.params[0].clone();
    } else {
        background = String::new();
    }

    return Ok(CachedNoteChart { 
        id: 0, hash, artist, title, version, path, background, audio
    })
}

fn get_notechart_paths(dir_path: &str) -> Result<Vec<String>, std::io::Error> {
    let mut notecharts: Vec<String> = vec![];

    for file in WalkDir::new(dir_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|file| file.ok()) 
        {
        if file.metadata()?.is_file() {
            let path = file.path();
            if is_notechart(path){
                notecharts.push(
                    path.display().to_string() // idiot
                )
            }
        }
    }

    return Ok(notecharts)
}

fn is_notechart(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let is_notechart = match ext {
            _ if ext == "osu" => true,
            _ if ext == "sm" => false,
            _ if ext == "ssc" => false,
            _ => false,
        };

        return is_notechart
    }

    return false;
}

pub fn is_notechart_exists(hash: &str, conn: &rusqlite::Connection) -> Result<bool, rusqlite::Error> {
    let query = format!("SELECT EXISTS (SELECT 1 FROM notecharts WHERE hash = '{}')", hash);
    let mut stmt = conn.prepare(&query)?;
    let mut rows = stmt.query([])?;
    
    if let Some(row) = rows.next()? {
        let is_exists: bool = row.get(0)?;
        if is_exists {
            return Ok(true)
        }
    }

    return Ok(false)
}