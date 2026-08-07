// The career vault: a single passphrase-encrypted SQLCipher file (the portable
// `.peregrine`). Event-sourced and append-only — every capture is an immutable
// event, so two machines merge by union later (Phase 8). Nothing here ever
// leaves the machine; the file is the user's to carry.

use crate::egress::now_ms;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

/// The open connection, or None when locked. Connection is Send (not Sync),
/// so it lives behind a Mutex and all vault commands are synchronous.
#[derive(Default)]
pub struct VaultState(pub Mutex<Option<Connection>>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub ts_ms: u64,
    pub device: String,
    pub kind: String,
    pub payload: serde_json::Value,
}

const SCHEMA_VERSION: i64 = 1;
const DEVICE: &str = "local"; // refined when multi-device sync lands (Phase 8)

pub fn vault_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("vault.peregrine"))
}

pub fn exists(app: &tauri::AppHandle) -> bool {
    vault_path(app).map(|p| p.exists()).unwrap_or(false)
}

fn open_encrypted(path: &PathBuf, passphrase: &str) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    let escaped = passphrase.replace('\'', "''");
    conn.execute_batch(&format!("PRAGMA key = '{escaped}';"))
        .map_err(|e| e.to_string())?;
    // Touch the schema to validate the key: a wrong passphrase fails here.
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
        .map_err(|_| "Wrong passphrase, or this isn't a Peregrine vault.".to_string())?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta (k TEXT PRIMARY KEY, v TEXT);
         CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            ts_ms INTEGER NOT NULL,
            device TEXT NOT NULL,
            kind TEXT NOT NULL,
            payload TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_events_ts ON events(ts_ms);",
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR IGNORE INTO meta(k, v) VALUES('schema_version', ?1)",
        [SCHEMA_VERSION.to_string()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Create a brand-new encrypted vault at `path`.
pub fn create(path: &PathBuf, passphrase: &str) -> Result<Connection, String> {
    if path.exists() {
        return Err("A vault already exists.".into());
    }
    if passphrase.trim().len() < 6 {
        return Err("Choose a passphrase of at least 6 characters.".into());
    }
    // Opening creates the file before the schema is written. If anything fails
    // partway (disk full, interrupted write), remove the stub — otherwise a file
    // that "exists" but has no valid schema would block onboarding (looks like an
    // existing vault) AND refuse every passphrase, locking the user out for good.
    let built = (|| {
        let conn = open_encrypted(path, passphrase)?;
        init_schema(&conn)?;
        Ok(conn)
    })();
    if built.is_err() {
        let _ = std::fs::remove_file(path);
    }
    built
}

/// Unlock an existing encrypted vault.
pub fn unlock(path: &PathBuf, passphrase: &str) -> Result<Connection, String> {
    if !path.exists() {
        return Err("No vault found.".into());
    }
    let conn = open_encrypted(path, passphrase)?;
    init_schema(&conn)?; // idempotent; also runs migrations later
    Ok(conn)
}

pub fn add_event(conn: &Connection, kind: &str, payload: &serde_json::Value) -> Result<Event, String> {
    let ev = Event {
        id: uuid::Uuid::new_v4().to_string(),
        ts_ms: now_ms(),
        device: DEVICE.into(),
        kind: kind.to_string(),
        payload: payload.clone(),
    };
    conn.execute(
        "INSERT INTO events(id, ts_ms, device, kind, payload) VALUES(?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![ev.id, ev.ts_ms as i64, ev.device, ev.kind, ev.payload.to_string()],
    )
    .map_err(|e| e.to_string())?;
    Ok(ev)
}

pub fn list_events(conn: &Connection, limit: i64) -> Result<Vec<Event>, String> {
    let mut stmt = conn
        .prepare("SELECT id, ts_ms, device, kind, payload FROM events ORDER BY ts_ms DESC LIMIT ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([limit], |row| {
            let payload_str: String = row.get(4)?;
            Ok(Event {
                id: row.get(0)?,
                ts_ms: row.get::<_, i64>(1)? as u64,
                device: row.get(2)?,
                kind: row.get(3)?,
                payload: serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null),
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Store a small value in the encrypted vault (e.g. the API key). Protected by
/// the vault passphrase — this is what replaces the OS keychain.
pub fn set_meta(conn: &Connection, k: &str, v: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO meta(k, v) VALUES(?1, ?2) ON CONFLICT(k) DO UPDATE SET v = excluded.v",
        rusqlite::params![k, v],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_meta(conn: &Connection, k: &str) -> Option<String> {
    conn.query_row("SELECT v FROM meta WHERE k = ?1", [k], |r| r.get::<_, String>(0))
        .ok()
        .filter(|s| !s.is_empty())
}

/// Merge another vault's events into this one — a lossless union (INSERT OR
/// IGNORE on the event id). This is how work ↔ home sync stays conflict-free:
/// each machine appends events, and merging never loses or duplicates any.
/// Returns the number of new events added.
pub fn merge_from(dest: &Connection, src_path: &PathBuf, passphrase: &str) -> Result<usize, String> {
    let src = open_encrypted(src_path, passphrase)?;
    let events = list_events(&src, i64::MAX)?;
    let mut merged = 0usize;
    for ev in &events {
        let n = dest
            .execute(
                "INSERT OR IGNORE INTO events(id, ts_ms, device, kind, payload) VALUES(?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![ev.id, ev.ts_ms as i64, ev.device, ev.kind, ev.payload.to_string()],
            )
            .map_err(|e| e.to_string())?;
        merged += n;
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_unlock_roundtrip_and_wrong_passphrase() {
        let dir = std::env::temp_dir().join(format!("peregrine-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("v.peregrine");

        // Create + write an event.
        let conn = create(&path, "correct horse battery").unwrap();
        let ev = add_event(&conn, "win", &serde_json::json!({ "text": "shipped it" })).unwrap();
        assert_eq!(ev.kind, "win");
        drop(conn);

        // Wrong passphrase must fail.
        assert!(unlock(&path, "wrong passphrase").is_err());

        // Correct passphrase unlocks and reads the event back.
        let conn2 = unlock(&path, "correct horse battery").unwrap();
        let evs = list_events(&conn2, 10).unwrap();
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].payload["text"], "shipped it");
        drop(conn2);

        // The raw file is encrypted — no plaintext leaks.
        let needle = b"shipped it";
        let bytes = std::fs::read(&path).unwrap();
        assert!(!bytes.windows(needle.len()).any(|w| w == needle));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn merge_is_lossless_union() {
        let dir = std::env::temp_dir().join(format!("peregrine-merge-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("home.peregrine");
        let b = dir.join("work.peregrine");

        let conn_a = create(&a, "shared passphrase").unwrap();
        add_event(&conn_a, "win", &serde_json::json!({ "text": "home win" })).unwrap();

        let conn_b = create(&b, "shared passphrase").unwrap();
        add_event(&conn_b, "win", &serde_json::json!({ "text": "work win" })).unwrap();
        drop(conn_b);

        // Merge work → home: one new event, total two.
        assert_eq!(merge_from(&conn_a, &b, "shared passphrase").unwrap(), 1);
        assert_eq!(list_events(&conn_a, 100).unwrap().len(), 2);
        // Merging again is a no-op (idempotent union).
        assert_eq!(merge_from(&conn_a, &b, "shared passphrase").unwrap(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
