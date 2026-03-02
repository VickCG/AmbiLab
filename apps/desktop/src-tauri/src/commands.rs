use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
