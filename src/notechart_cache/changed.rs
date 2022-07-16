use rusqlite::{params, Connection};
use osu_parser;
use crate::notechart_cache;
use notechart_cache::CacheError;

#[derive(Debug, Clone)]
struct ChangedNotechart {
    id: u64,
    hash: String,
    path: String
}

pub fn fix_changed(conn: &mut Connection) -> Result<(), CacheError> {
    let (changed, removed) = find_changed_notecharts(&conn)?;

    let tr = match conn.transaction() {
        Ok(t) => t,
        Err(_) => return Err(CacheError::TransactionError)
    };

    for notechart in &changed {
        let info = match osu_parser::import_info(&notechart.path) {
            Ok(i) => i,
            Err(_) => return Err(CacheError::FileReadError)
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
            hash=?1,
            artist = ?2, 
            title = ?3, 
            version = ?4, 
            audio = ?5, 
            background = ?6
            WHERE id=?7",
            params![notechart.hash, artist, title, version, audio, background, notechart.id],
        );

        if let Err(_) = result {
            return Err(CacheError::DbUpdateError)
        }
    }

    for notechart in &removed {
        if let Err(_) = tr.execute("DELETE FROM notecharts WHERE id=?1", [notechart.id]) {
            return Err(CacheError::DbDeleteError)
        };
    }

    if let Err(_) = tr.commit() {
        return Err(CacheError::TransactionError)
    }

    return Ok(())
}

fn find_changed_notecharts(conn: &Connection) -> Result<(Vec<ChangedNotechart>, Vec<ChangedNotechart>), CacheError> {
    let mut changed_notecharts: Vec<ChangedNotechart> = vec![];
    let mut removed_notecharts: Vec<ChangedNotechart> = vec![];

    let query = "SELECT id, hash, path FROM notecharts";
    let mut stmt = match conn.prepare(query) {
        Ok(s) => s,
        Err(_) => return Err(CacheError::DbSelectError)
    };

    let notechart_iter = stmt.query_map([], |row| {
        Ok(ChangedNotechart {
            id: row.get(0)?,
            hash: row.get(1)?,
            path: row.get(2)?
        })
    });

    let notechart_iter = match notechart_iter {
        Ok(n) => n,
        Err(_) => return Err(CacheError::DbSelectError)
    };

    for notechart in notechart_iter {
        let notechart = match notechart {
            Ok(n) => n,
            Err(_) => continue
        };

        let hash = match notechart_cache::get_hash(&notechart.path) {
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