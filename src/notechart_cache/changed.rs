use rusqlite::{params, Connection};
use osu_parser;
use crate::notechart_cache;
use notechart_cache::CacheError;

#[derive(Debug, Clone)]
struct ChangedNotechart {
    id: i64,
    set_id: i64,
    hash: String,
    set_path: String,
    filename: String
}

pub fn fix_changed(conn: &mut Connection) -> Result<(), CacheError> {
    let (changed, removed) = find_changed_notecharts(&conn)?;

    println!("changed: {} | removed: {}", changed.len(), removed.len());

    let tr = match conn.transaction() {
        Ok(t) => t,
        Err(e) => return Err(CacheError::TransactionError(e))
    };

    for notechart in &changed {
        let path = [notechart.set_path.clone(), notechart.filename.clone()].join("/");

        let info = match osu_parser::import_info(&path) {
            Ok(i) => i,
            Err(e) => return Err(CacheError::FileReadError(e))
        };

        let artist = info.metadata.artist;
        let title = info.metadata.title;
        let version = info.metadata.version;
        let audio = info.general.audio_filename;

        let mut events_iter = info.events.data.iter();
        let background: String;

        if let Some(e) = events_iter.find(|e| e.e_type == "0") {
            background = e.params[0].clone();
        } else {
            background = String::new();
        }

        let result = tr.execute(
            "UPDATE notecharts SET 
            set_id=?1,
            hash=?2,
            artist=?3, 
            title=?4, 
            version=?5, 
            audio=?6, 
            background=?7
            WHERE id=?8",
            params![notechart.set_id, notechart.hash, artist, title, version, audio, background, notechart.id],
        );

        if let Err(e) = result {
            return Err(CacheError::DbUpdateError(e))
        }
    }

    for notechart in &removed {
        if let Err(e) = tr.execute("DELETE FROM notecharts WHERE id=?1", [notechart.id]) {
            return Err(CacheError::DbDeleteError(e))
        };
    }

    if let Err(e) = tr.commit() {
        return Err(CacheError::TransactionError(e))
    }

    return Ok(())
}

fn find_changed_notecharts(conn: &Connection) -> Result<(Vec<ChangedNotechart>, Vec<ChangedNotechart>), CacheError> {
    let mut changed_notecharts: Vec<ChangedNotechart> = vec![];
    let mut removed_notecharts: Vec<ChangedNotechart> = vec![];

    let query = "SELECT id, set_id, hash, filename FROM notecharts";
    let mut stmt = match conn.prepare(query) {
        Ok(s) => s,
        Err(e) => return Err(CacheError::DbSelectError(e))
    };

    let notechart_iter = stmt.query_map([], |row| {
        Ok(ChangedNotechart {
            id: row.get(0)?,
            set_id: row.get(1)?,
            hash: row.get(2)?,
            set_path: String::new(),
            filename: row.get(3)?
        })
    });

    let notechart_iter = match notechart_iter {
        Ok(n) => n,
        Err(e) => return Err(CacheError::DbSelectError(e))
    };

    for notechart in notechart_iter {
        let mut notechart = match notechart {
            Ok(n) => n,
            Err(_) => continue
        };

        let query = "SELECT path FROM notechart_sets WHERE id=?1";
        let mut stmt = match conn.prepare(query) {
            Ok(s) => s,
            Err(e) => return Err(CacheError::DbSelectError(e))
        };

        let mut rows = match stmt.query([notechart.set_id]) {
            Ok(s) => s,
            Err(e) => return Err(CacheError::DbSelectError(e))
        };

        if let Ok(row) = rows.next() {
            match row {
                Some(r) => {
                    notechart.set_path = match r.get(0) {
                        Ok(p) => p,
                        Err(e) => return Err(CacheError::DbSelectError(e))
                    };
                }
                None => panic!("?? maybe set is removed idk")
            }
        }
        else {
            panic!("!! maybe set is removed idk")
        }

        let path = [notechart.set_path.clone(), notechart.filename.clone()].join("/");

        let hash = match notechart_cache::get_hash(&path) {
            Ok(h) => h,
            Err(_) => {
                removed_notecharts.push(notechart.clone());
                continue;
            }
        };

        if notechart.hash != hash {
            let mut notechart = notechart.clone();
            notechart.hash = hash;

            changed_notecharts.push(notechart)
        }
    }

    return Ok((changed_notecharts, removed_notecharts))
}