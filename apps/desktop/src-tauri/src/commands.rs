use arrow::array::Array;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Manager, State};

use crate::duckdb_state::{parquet_cache_path, DuckDbState};

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub tables: Vec<String>,
    pub join_count: usize,
    pub estimated_rows: u64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataPreview {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_rows: usize,
}

#[tauri::command]
pub fn read_dir(path: String) -> Result<Vec<FileEntry>, String> {
    let path = PathBuf::from(&path);

    let entries = fs::read_dir(&path)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let metadata = entry.metadata().ok()?;
            let name = entry.file_name().to_string_lossy().to_string();

            if name.starts_with('.') {
                return None;
            }

            Some(FileEntry {
                name,
                path: entry.path().to_string_lossy().to_string(),
                is_dir: metadata.is_dir(),
            })
        })
        .collect::<Vec<_>>();

    let mut sorted = entries;
    sorted.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(sorted)
}

#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn analyze_sql(query: String) -> Result<AnalysisResult, String> {
    use ambilab_core::pipeline::Pipeline;

    let pipeline = Pipeline::new(query);
    let parsed = pipeline.parse().map_err(|e| e.to_string())?;

    let tables: Vec<String> = parsed
        .parsed()
        .map(|ctx| ctx.tables.iter().map(|t| t.name.to_string()).collect())
        .unwrap_or_default();

    Ok(AnalysisResult {
        tables,
        join_count: 0,
        estimated_rows: 0,
        warnings: vec![],
    })
}

/// DuckDB-backed CSV reader.
/// Phase 1 (always fast): DuckDB SIMD scan with LIMIT/OFFSET.
/// Row count: served from session cache; first access does a full COUNT(*) scan.
#[tauri::command]
pub async fn read_csv(
    path: String,
    limit: usize,
    offset: usize,
    db: State<'_, DuckDbState>,
) -> Result<DataPreview, String> {
    let conn = Arc::clone(&db.conn);
    let row_counts = Arc::clone(&db.row_counts);

    tokio::task::spawn_blocking(move || {
        let conn = conn.lock();
        let safe = path.replace('\'', "''");

        let mut stmt = conn
            .prepare(&format!(
                "SELECT * FROM read_csv_auto('{safe}') LIMIT {limit} OFFSET {offset}"
            ))
            .map_err(|e| e.to_string())?;

        let columns = safe_column_names(&stmt);
        let ncols = columns.len();
        let mut rows: Vec<Vec<String>> = Vec::with_capacity(limit);
        let mut result = stmt.query([]).map_err(|e| e.to_string())?;

        while let Some(row) = result.next().map_err(|e| e.to_string())? {
            let mut data_row: Vec<String> = Vec::with_capacity(ncols);
            for i in 0..ncols {
                data_row.push(duckdb_value_to_string(row, i));
            }
            rows.push(data_row);
        }

        // Row count: cache hit is O(1); miss does a full DuckDB SIMD scan once.
        let total_rows = if let Some(count) = row_counts.get(&path) {
            *count
        } else {
            let count: usize = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM read_csv_auto('{safe}')"),
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())? as usize;
            row_counts.insert(path, count);
            count
        };

        Ok(DataPreview { columns, rows, total_rows })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Arrow-backed Parquet reader with spawn_blocking to avoid blocking the tokio runtime.
/// Total row count read from Parquet file footer — O(1), no scan.
#[tauri::command]
pub async fn read_parquet(
    path: String,
    limit: usize,
    offset: usize,
) -> Result<DataPreview, String> {
    tokio::task::spawn_blocking(move || {
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

        let file = File::open(&path).map_err(|e| e.to_string())?;
        let builder =
            ParquetRecordBatchReaderBuilder::try_new(file).map_err(|e| e.to_string())?;

        let total_rows = builder.metadata().file_metadata().num_rows() as usize;
        let columns: Vec<String> = builder
            .schema()
            .fields()
            .iter()
            .map(|f| f.name().clone())
            .collect();

        let reader = builder.build().map_err(|e| e.to_string())?;

        let mut rows: Vec<Vec<String>> = Vec::with_capacity(limit);
        let mut rows_seen: usize = 0;

        'outer: for batch_result in reader {
            let batch = batch_result.map_err(|e| e.to_string())?;
            let num_rows = batch.num_rows();

            if rows_seen + num_rows <= offset {
                rows_seen += num_rows;
                continue;
            }

            for row_idx in 0..num_rows {
                if rows_seen >= offset && rows.len() < limit {
                    let mut row: Vec<String> = Vec::with_capacity(batch.num_columns());
                    for col_idx in 0..batch.num_columns() {
                        row.push(format_array_value(batch.column(col_idx).as_ref(), row_idx));
                    }
                    rows.push(row);
                }
                rows_seen += 1;
                if rows.len() >= limit {
                    break 'outer;
                }
            }
        }

        Ok(DataPreview { columns, rows, total_rows })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Two-phase JSON read:
/// Phase 1 (~50-150ms): return first page immediately from read_json_auto (DuckDB early-exits at LIMIT)
/// Phase 2 (background): convert JSON → Parquet with ZSTD, emit json_index_ready with total_rows
/// All subsequent calls use the Parquet cache: O(1) count, ~20ms reads
#[tauri::command]
pub async fn read_json(
    path: String,
    limit: usize,
    offset: usize,
    db: State<'_, DuckDbState>,
    app: tauri::AppHandle,
) -> Result<DataPreview, String> {
    let cache_path = parquet_cache_path(&path)?;
    let conn = Arc::clone(&db.conn);

    if cache_path.exists() {
        let safe = cache_path.to_string_lossy().replace('\'', "''");
        return tokio::task::spawn_blocking(move || {
            let conn = conn.lock();
            read_parquet_page(&conn, &safe, limit, offset)
        })
        .await
        .map_err(|e| e.to_string())?;
    }

    // First open: return page immediately on a blocking thread, never block tokio.
    let path_for_query = path.clone();
    let schema_cache = Arc::clone(&db.schema_cache);
    let page = tokio::task::spawn_blocking(move || {
        let conn = conn.lock();
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            query_json_auto(&conn, &path_for_query, limit, offset, &schema_cache)
        }))
        .unwrap_or_else(|_| Err("JSON contains unsupported column types".to_string()))
    })
    .await
    .map_err(|e| e.to_string())??;

    // Guard: only one background build per path.
    if db.building.insert(path.clone(), ()).is_none() {
        let (path_c, cache_c, building_c, app_c) = (
            path,
            cache_path,
            Arc::clone(&db.building),
            app,
        );
        tokio::task::spawn_blocking(move || build_and_notify(path_c, cache_c, building_c, app_c));
    }

    Ok(page)
}

fn is_complex_duckdb_type(typ: &str) -> bool {
    let t = typ.to_uppercase();
    t.starts_with("STRUCT")
        || t.starts_with("MAP")
        || t.starts_with("LIST")
        || t.starts_with("UNION")
        || t.ends_with("[]")
        || t == "JSON"
}

fn json_column_schema(
    conn: &duckdb::Connection,
    safe: &str,
    path: &str,
    cache: &DashMap<String, Vec<(String, bool)>>,
) -> Result<Vec<(String, bool)>, String> {
    if let Some(hit) = cache.get(path) {
        return Ok(hit.clone());
    }
    let mut stmt = conn
        .prepare(&format!(
            "DESCRIBE SELECT * FROM read_json_auto('{safe}', maximum_object_size=268435456, sample_size=1000)"
        ))
        .map_err(|e| e.to_string())?;
    let mut result = stmt.query([]).map_err(|e| e.to_string())?;
    let mut cols = Vec::new();
    while let Some(row) = result.next().map_err(|e| e.to_string())? {
        let name: String = row.get(0).map_err(|e| e.to_string())?;
        let typ: String = row.get(1).map_err(|e| e.to_string())?;
        cols.push((name, is_complex_duckdb_type(&typ)));
    }
    cache.insert(path.to_string(), cols.clone());
    Ok(cols)
}

fn build_json_select(schema: &[(String, bool)]) -> String {
    schema
        .iter()
        .map(|(name, is_complex)| {
            let quoted = name.replace('"', "\"\"");
            if *is_complex {
                format!("CAST(\"{quoted}\" AS VARCHAR) AS \"{quoted}\"")
            } else {
                format!("\"{quoted}\"")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn query_json_auto(
    conn: &duckdb::Connection,
    path: &str,
    limit: usize,
    offset: usize,
    schema_cache: &DashMap<String, Vec<(String, bool)>>,
) -> Result<DataPreview, String> {
    let safe = path.replace('\'', "''");
    let schema = json_column_schema(conn, &safe, path, schema_cache)?;
    let select = build_json_select(&schema);
    let columns: Vec<String> = schema.into_iter().map(|(name, _)| name).collect();
    let ncols = columns.len();

    let mut stmt = conn
        .prepare(&format!(
            "SELECT {select} FROM read_json_auto('{safe}', maximum_object_size=268435456, sample_size=1000) LIMIT {limit} OFFSET {offset}"
        ))
        .map_err(|e| e.to_string())?;
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(limit);
    let mut result = stmt.query([]).map_err(|e| e.to_string())?;
    while let Some(row) = result.next().map_err(|e| e.to_string())? {
        let mut data_row: Vec<String> = Vec::with_capacity(ncols);
        for i in 0..ncols {
            data_row.push(duckdb_value_to_string(row, i));
        }
        rows.push(data_row);
    }
    // total_rows=0: signals "loading" — frontend updates via json_index_ready event
    Ok(DataPreview { columns, rows, total_rows: 0 })
}

fn read_parquet_page(
    conn: &duckdb::Connection,
    safe_cache: &str,
    limit: usize,
    offset: usize,
) -> Result<DataPreview, String> {
    // O(1) — reads row count from Parquet file footer, no row scan
    let total_rows: usize = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM read_parquet('{safe_cache}')"),
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| e.to_string())? as usize;
    let mut stmt = conn
        .prepare(&format!(
            "SELECT * FROM read_parquet('{safe_cache}') LIMIT {limit} OFFSET {offset}"
        ))
        .map_err(|e| e.to_string())?;
    let columns = safe_column_names(&stmt);
    let ncols = columns.len();
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(limit);
    let mut result = stmt.query([]).map_err(|e| e.to_string())?;
    while let Some(row) = result.next().map_err(|e| e.to_string())? {
        let mut data_row: Vec<String> = Vec::with_capacity(ncols);
        for i in 0..ncols {
            data_row.push(duckdb_value_to_string(row, i));
        }
        rows.push(data_row);
    }
    Ok(DataPreview { columns, rows, total_rows })
}

/// Runs in a blocking thread: converts JSON → ZSTD Parquet, then emits json_index_ready.
/// Opens its own DuckDB connection — never blocks the foreground query connection.
fn build_and_notify(
    path: String,
    cache_path: PathBuf,
    building: Arc<DashMap<String, ()>>,
    app: tauri::AppHandle,
) {
    let Ok(conn) = duckdb::Connection::open_in_memory() else {
        return;
    };
    let safe_src = path.replace('\'', "''");
    let safe_dst = cache_path.to_string_lossy().replace('\'', "''");
    let ok = conn
        .execute_batch(&format!(
            "COPY (SELECT * FROM read_json_auto('{safe_src}', maximum_object_size=268435456)) \
             TO '{safe_dst}' \
             (FORMAT PARQUET, CODEC 'ZSTD', COMPRESSION_LEVEL 3, ROW_GROUP_SIZE 122880, STATISTICS TRUE);"
        ))
        .is_ok();
    building.remove(&path);
    if !ok {
        return;
    }
    let total: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM read_parquet('{safe_dst}')"),
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    #[derive(Serialize, Clone)]
    struct Payload {
        path: String,
        total_rows: i64,
    }
    let _ = app.emit_all("json_index_ready", Payload { path, total_rows: total });
}

/// DuckDB-backed JSONL (newline-delimited JSON) reader.
/// Uses read_ndjson_auto for explicit NDJSON parsing with SIMD acceleration.
#[tauri::command]
pub async fn read_jsonl(
    path: String,
    limit: usize,
    offset: usize,
    db: State<'_, DuckDbState>,
) -> Result<DataPreview, String> {
    let conn = Arc::clone(&db.conn);
    let row_counts = Arc::clone(&db.row_counts);

    tokio::task::spawn_blocking(move || {
        let conn = conn.lock();
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let safe = path.replace('\'', "''");

            let mut stmt = conn
                .prepare(&format!(
                    "SELECT * FROM read_ndjson_auto('{safe}', maximum_object_size=268435456) LIMIT {limit} OFFSET {offset}"
                ))
                .map_err(|e| e.to_string())?;

            let columns = safe_column_names(&stmt);
            let ncols = columns.len();
            let mut rows: Vec<Vec<String>> = Vec::with_capacity(limit);
            let mut result = stmt.query([]).map_err(|e| e.to_string())?;

            while let Some(row) = result.next().map_err(|e| e.to_string())? {
                let mut data_row: Vec<String> = Vec::with_capacity(ncols);
                for i in 0..ncols {
                    data_row.push(duckdb_value_to_string(row, i));
                }
                rows.push(data_row);
            }

            let total_rows = if let Some(count) = row_counts.get(&path) {
                *count
            } else {
                let count: usize = conn
                    .query_row(
                        &format!("SELECT COUNT(*) FROM read_ndjson_auto('{safe}', maximum_object_size=268435456)"),
                        [],
                        |r| r.get::<_, i64>(0),
                    )
                    .map_err(|e| e.to_string())? as usize;
                row_counts.insert(path, count);
                count
            };

            Ok(DataPreview { columns, rows, total_rows })
        }))
        .unwrap_or_else(|_| Err("JSONL contains unsupported column types".to_string()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Safe alternative to duckdb-rs `Statement::column_names()`.
/// The duckdb-rs method calls `column_name(i).unwrap()` internally, which panics when
/// DuckDB returns a NULL pointer for the column name (empty header, encoding edge case, etc.).
/// This version falls back to "col_N" instead of panicking.
fn safe_column_names(stmt: &duckdb::Statement<'_>) -> Vec<String> {
    let n = stmt.column_count();
    (0..n)
        .map(|i| {
            stmt.column_name(i)
                .map(|s| s.clone())
                .unwrap_or_else(|_| format!("col_{i}"))
        })
        .collect()
}

fn duckdb_value_to_string(row: &duckdb::Row<'_>, idx: usize) -> String {
    use duckdb::types::ValueRef;
    use std::panic;

    // duckdb-rs panics on complex types (STRUCT, LIST, MAP) inferred by read_json_auto.
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| row.get_ref(idx)));

    match result {
        Err(_) => "...".to_string(),
        Ok(Err(_)) => "NULL".to_string(),
        Ok(Ok(vref)) => match vref {
            ValueRef::Null => "NULL".to_string(),
            ValueRef::Boolean(b) => b.to_string(),
            ValueRef::TinyInt(n) => n.to_string(),
            ValueRef::SmallInt(n) => n.to_string(),
            ValueRef::Int(n) => n.to_string(),
            ValueRef::BigInt(n) => n.to_string(),
            ValueRef::HugeInt(n) => n.to_string(),
            ValueRef::UTinyInt(n) => n.to_string(),
            ValueRef::USmallInt(n) => n.to_string(),
            ValueRef::UInt(n) => n.to_string(),
            ValueRef::UBigInt(n) => n.to_string(),
            ValueRef::Float(n) => n.to_string(),
            ValueRef::Double(n) => n.to_string(),
            ValueRef::Decimal(n) => n.to_string(),
            ValueRef::Text(b) => String::from_utf8_lossy(b).into_owned(),
            ValueRef::Blob(b) => format!("<blob:{}>", b.len()),
            _ => "...".to_string(),
        },
    }
}

fn format_array_value(array: &dyn Array, idx: usize) -> String {
    use arrow::array::{
        BooleanArray, Float32Array, Float64Array, Int32Array, Int64Array, StringArray,
    };
    use arrow::datatypes::DataType;

    if array.is_null(idx) {
        return "NULL".to_string();
    }

    match array.data_type() {
        DataType::Utf8 => array
            .as_any()
            .downcast_ref::<StringArray>()
            .map_or_else(|| "ERR".to_string(), |a| a.value(idx).to_string()),
        DataType::Int32 => array
            .as_any()
            .downcast_ref::<Int32Array>()
            .map_or_else(|| "ERR".to_string(), |a| a.value(idx).to_string()),
        DataType::Int64 => array
            .as_any()
            .downcast_ref::<Int64Array>()
            .map_or_else(|| "ERR".to_string(), |a| a.value(idx).to_string()),
        DataType::Float32 => array
            .as_any()
            .downcast_ref::<Float32Array>()
            .map_or_else(|| "ERR".to_string(), |a| a.value(idx).to_string()),
        DataType::Float64 => array
            .as_any()
            .downcast_ref::<Float64Array>()
            .map_or_else(|| "ERR".to_string(), |a| a.value(idx).to_string()),
        DataType::Boolean => array
            .as_any()
            .downcast_ref::<BooleanArray>()
            .map_or_else(|| "ERR".to_string(), |a| a.value(idx).to_string()),
        _ => format!("{:?}", array.data_type()),
    }
}

use crate::autosave::AutoSaveManager;
use crate::workspace_config::WorkspaceConfig;

pub struct AutoSaveState(pub Arc<AutoSaveManager>);

#[tauri::command]
pub async fn load_workspace_config(path: String) -> Result<WorkspaceConfig, String> {
    WorkspaceConfig::load(&path)
        .await
        .map_err(|e| format!("{:?}", e))
}

#[tauri::command]
pub async fn save_workspace_config(path: String, config: WorkspaceConfig) -> Result<(), String> {
    config.save(&path).await.map_err(|e| format!("{:?}", e))
}

#[tauri::command]
pub fn workspace_config_exists(path: String) -> bool {
    WorkspaceConfig::exists(&path)
}

#[tauri::command]
pub async fn update_workspace_config(
    path: String,
    config: WorkspaceConfig,
    autosave: State<'_, AutoSaveState>,
) -> Result<(), String> {
    autosave.0.queue_save(path, config);
    Ok(())
}

#[tauri::command]
pub async fn create_workspace_with_config(
    path: String,
    name: String,
    color: Option<String>,
) -> Result<WorkspaceConfig, String> {
    if WorkspaceConfig::exists(&path) {
        return WorkspaceConfig::load(&path)
            .await
            .map_err(|e| format!("{:?}", e));
    }

    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    let id = format!("ws_{:x}{:x}", duration.as_secs(), duration.subsec_nanos());

    let config = WorkspaceConfig::new(id, name, path.clone(), color);
    config.save(&path).await.map_err(|e| format!("{:?}", e))?;

    Ok(config)
}
