use arrow::array::Array;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;
use tauri::State;

use crate::file_cache::{FileCacheState, read_csv_page, read_jsonl_page};

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
    sorted.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
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

#[tauri::command]
pub fn read_csv(
    path: String,
    limit: usize,
    offset: usize,
    cache: State<'_, FileCacheState>,
) -> Result<DataPreview, String> {
    let index = cache.0.get_or_build_csv(&path)?;
    let page = if limit > 0 { offset / limit } else { 0 };
    let rows = read_csv_page(&path, &index, page)?;
    Ok(DataPreview {
        columns: index.columns.clone(),
        rows,
        total_rows: index.total_rows,
    })
}

#[tauri::command]
pub fn read_parquet(path: String, limit: usize, offset: usize) -> Result<DataPreview, String> {
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

    let file = File::open(&path).map_err(|e| e.to_string())?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(|e| e.to_string())?;

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

    Ok(DataPreview {
        columns,
        rows,
        total_rows,
    })
}

#[tauri::command]
pub fn read_json(path: String, limit: usize, offset: usize) -> Result<DataPreview, String> {
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let value: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let records = match value {
        Value::Array(arr) => arr,
        Value::Object(obj) => vec![Value::Object(obj)],
        _ => return Err("JSON must be an array or object".to_string()),
    };

    if records.is_empty() {
        return Ok(DataPreview {
            columns: vec![],
            rows: vec![],
            total_rows: 0,
        });
    }

    let columns = extract_json_columns(&records[0]);
    let total_rows = records.len();

    let rows: Vec<Vec<String>> = records
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|record| extract_json_row(&record, &columns))
        .collect();

    Ok(DataPreview {
        columns,
        rows,
        total_rows,
    })
}

#[tauri::command]
pub fn read_jsonl(
    path: String,
    limit: usize,
    offset: usize,
    cache: State<'_, FileCacheState>,
) -> Result<DataPreview, String> {
    let index = cache.0.get_or_build_jsonl(&path)?;
    let page = if limit > 0 { offset / limit } else { 0 };
    let rows = read_jsonl_page(&path, &index, page)?;
    Ok(DataPreview {
        columns: index.columns.clone(),
        rows,
        total_rows: index.total_rows,
    })
}

fn extract_json_columns(value: &Value) -> Vec<String> {
    match value {
        Value::Object(map) => map.keys().cloned().collect(),
        _ => vec!["value".to_string()],
    }
}

fn extract_json_row(value: &Value, columns: &[String]) -> Vec<String> {
    match value {
        Value::Object(map) => columns
            .iter()
            .map(|col| format_json_value(map.get(col)))
            .collect(),
        _ => vec![format_json_value(Some(value))],
    }
}

fn format_json_value(value: Option<&Value>) -> String {
    match value {
        None => "NULL".to_string(),
        Some(Value::Null) => "NULL".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Array(arr)) => serde_json::to_string(arr).unwrap_or_default(),
        Some(Value::Object(obj)) => serde_json::to_string(obj).unwrap_or_default(),
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
use std::sync::Arc;

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
