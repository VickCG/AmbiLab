use dashmap::DashMap;
use duckdb::Connection;
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

pub struct DuckDbState {
    /// Arc so inner connection can be cloned into spawn_blocking closures.
    pub conn: Arc<Mutex<Connection>>,
    /// Tracks paths currently being converted to avoid duplicate background builds.
    pub building: Arc<DashMap<String, ()>>,
    /// Session-level row count cache: path → total_rows.
    /// Populated on first full-scan; valid for the lifetime of the process.
    pub row_counts: Arc<DashMap<String, usize>>,
    /// Schema cache: path → [(col_name, is_complex)].
    /// Avoids repeated DESCRIBE on every pagination request.
    pub schema_cache: Arc<DashMap<String, Vec<(String, bool)>>>,
}

impl DuckDbState {
    pub fn new() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;

        let threads = std::thread::available_parallelism()
            .map(|n| (n.get()).saturating_sub(1).max(1))
            .unwrap_or(1);

        conn.execute_batch(&format!(
            "PRAGMA threads = {threads}; \
             SET memory_limit = '4GB'; \
             SET enable_object_cache = true; \
             SET enable_progress_bar = false;"
        ))
        .map_err(|e| e.to_string())?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            building: Arc::new(DashMap::new()),
            row_counts: Arc::new(DashMap::new()),
            schema_cache: Arc::new(DashMap::new()),
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
