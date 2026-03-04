use dashmap::DashMap;
use duckdb::Connection;
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

pub struct DuckDbState {
    pub conn: Mutex<Connection>,
    /// Tracks paths currently being converted to avoid duplicate background builds.
    pub building: Arc<DashMap<String, ()>>,
}

impl DuckDbState {
    pub fn new() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        conn.execute_batch("SET memory_limit = '512MB';")
            .map_err(|e| e.to_string())?;
        Ok(Self {
            conn: Mutex::new(conn),
            building: Arc::new(DashMap::new()),
        })
    }
}

/// Returns a Parquet cache path keyed by source path + mtime.
/// Cache is automatically invalidated when the source file changes.
pub fn parquet_cache_path(source: &str) -> Result<PathBuf, String> {
    let meta = std::fs::metadata(source).map_err(|e| e.to_string())?;
    let mtime = meta
        .modified()
        .map_err(|e| e.to_string())?
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis();

    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update(mtime.to_le_bytes());
    let hash = format!("{:x}", hasher.finalize());

    let cache_dir = std::env::temp_dir().join("ambilab_cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;

    Ok(cache_dir.join(format!("{}.parquet", &hash[..16])))
}
