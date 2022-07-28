use std::path::Path;
use walkdir::WalkDir;
use crate::notechart_cache;
use notechart_cache::{CacheError, CachedNoteChart};
use rusqlite::{Connection, Result, params, Error, Transaction};

struct NotechartSetDir {
    pub id: i64,
    pub path: String,
    pub file_paths: Vec<String>
}

pub fn find_new(dir_path: &str, conn: &mut Connection) -> Result<(), CacheError> {
    let mut sets = match get_sets_and_notecharts(dir_path) {
        Ok(n) => n,
        Err(e) => return Err(CacheError::DirReadError(e))
    };

    println!("Found {} notechart sets", sets.len());

    let mut new_sets: Vec<&mut NotechartSetDir> = vec![]; 

    for set in &mut sets {
        let res_opt_id = get_set_id(conn, &set.path);
        if let Ok(opt) = res_opt_id {
            match opt {
                Some(id) => set.id = id,
                None => new_sets.push(set)
            }
        }
    }

    let tr = match conn.transaction() {
        Ok(t) => t,
        Err(e) => return Err(CacheError::TransactionError(e))
    };

    for set in new_sets {
        if let Ok(opt) = new_set(&tr, &set.path) {
            match opt {
                Some(id) => set.id = id,
                None => panic!("HOW????")
            }
        }
    }

    let res = tr.commit();

    if let Err(e) = res {
        return Err(CacheError::TransactionError(e))
    }

    let tr = match conn.transaction() {
        Ok(t) => t,
        Err(e) => return Err(CacheError::TransactionError(e))
    };

    for set in sets {
        for notechart_filename in set.file_paths {
            let mut path = set.path.to_string();
            path.push_str("/");
            path.push_str(&notechart_filename);

            let hash = match notechart_cache::get_hash(&path) {
                Ok(h) => h,
                Err(e) => return Err(CacheError::FileReadError(e))
            };
            
            let is_exists = match is_notechart_exists(&hash, &tr) {
                Ok(is) => is,
                Err(_) => continue
            };

            if is_exists {
                println!("exists");
                continue
            }

            let set_id = set.id;
            let notechart = get_notechart_data(&path)?;

            let result = tr.execute(
                "INSERT INTO notecharts (set_id, hash, artist, title, version, filename, audio, background) 
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    set_id,
                    hash,
                    notechart.artist, 
                    notechart.title, 
                    notechart.version, 
                    notechart_filename, 
                    notechart.audio, 
                    notechart.background
                ],
            );
        
            if let Err(e) = result {
                return Err(CacheError::DbInsertError(e))
            }
        }
    }

    let res = tr.commit();

    if let Err(e) = res {
        return Err(CacheError::TransactionError(e))
    }

    return Ok(());
}

fn get_notechart_data(path: &str) -> Result<CachedNoteChart, CacheError>{
    let info = match osu_parser::import_info(&path) {
        Ok(i) => i,
        Err(e) => return Err(CacheError::FileReadError(e))
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
        id: 0, set_id: None, artist, title, version, path, background, audio
    })
}

fn get_sets_and_notecharts(dir_path: &str) -> Result<Vec<NotechartSetDir>, std::io::Error> {
    let mut notechart_sets: Vec<NotechartSetDir> = vec![];

    fn get_notecharts(dir_path: &Path) -> Result<Option<Vec<String>>, std::io::Error>{
        let paths = std::fs::read_dir(dir_path)?;
        let mut notecharts: Vec<String> = vec![];

        for path in paths {
            let path = path?.path();
            let path = path.as_path();

            if is_notechart(path) {
                let file_name = match path.file_name() {
                    Some(f) => f,
                    None => continue
                };

                if let Some(file_name) = file_name.to_str() {
                    notecharts.push(
                        file_name.to_string()
                    );
                }
            }
        }

        match notecharts.len() {
            0 => return Ok(None),
            _ => return Ok(Some(notecharts))
        }
    }

    for dir in WalkDir::new(dir_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|dir| dir.ok()) 
        {
        if dir.metadata()?.is_dir() {
            let path = dir.path();

            if let Ok(Some(notecharts)) = get_notecharts(path) {
                let path = path.display().to_string();

                notechart_sets.push(
                    NotechartSetDir { id: 0, path, file_paths: notecharts }
                )
            }

        }
    }

    return Ok(notechart_sets)
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

fn get_set_id(conn: &Connection, path: &str) -> Result<Option<i64>, Error> {
    let parent = get_parent(path);
    let query = format!("SELECT id FROM notechart_sets WHERE path='{}'", parent);
    let mut stmt = conn.prepare(&query)?;
    let mut rows = stmt.query([])?;

    if let Some(row) = rows.next()? {
        return Ok(Some(row.get(0)?))
    }

    return Ok(None)
}

fn new_set(tr: &Transaction, path: &str) -> Result<Option<i64>, Error> {
    tr.execute(
        "INSERT INTO notechart_sets (path) VALUES (?)", 
        [path]
    )?;

    let id = tr.last_insert_rowid();

    Ok(Some(id))
}

fn get_parent(path: &str) -> &str {
    let mut pos = 0;
    for (i, &ch) in path.as_bytes().iter().rev().enumerate() {
        if ch == b'/' {
            pos = path.len() - i;
            break
        }
    }
    return &path[..pos];
    // 🤓🤓 atchualy you shoud use Path::parent() !! 🤓🤓
}

fn is_notechart_exists(hash: &str, tr: &rusqlite::Connection) -> Result<bool, rusqlite::Error> {
    let query = format!("SELECT EXISTS (SELECT 1 FROM notecharts WHERE hash = '{}')", hash);
    let mut stmt = tr.prepare(&query)?;
    let mut rows = stmt.query([])?;
    
    if let Some(row) = rows.next()? {
        let is_exists: bool = row.get(0)?;
        if is_exists {
            return Ok(true)
        }
    }

    return Ok(false)
}