use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "csv", "tsv", "json", "jsonl", "parquet", "arrow", "feather",
    "xlsx", "xls", "sql", "sqlite", "db",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    Csv,
    Tsv,
    Json,
    JsonLines,
    Parquet,
    Arrow,
    Excel,
    Sql,
    Sqlite,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedFile {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size_bytes: u64,
    pub file_type: FileType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub files: Vec<ImportedFile>,
    pub total_count: usize,
    pub skipped_count: usize,
}

fn detect_file_type(extension: &str) -> FileType {
    match extension.to_lowercase().as_str() {
        "csv" => FileType::Csv,
        "tsv" => FileType::Tsv,
        "json" => FileType::Json,
        "jsonl" => FileType::JsonLines,
        "parquet" => FileType::Parquet,
        "arrow" | "feather" => FileType::Arrow,
        "xlsx" | "xls" => FileType::Excel,
        "sql" => FileType::Sql,
        "sqlite" | "db" => FileType::Sqlite,
        _ => FileType::Unknown,
    }
}

fn is_supported_extension(extension: &str) -> bool {
    SUPPORTED_EXTENSIONS.contains(&extension.to_lowercase().as_str())
}

fn create_imported_file(path: &Path) -> Option<ImportedFile> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if !is_supported_extension(&extension) {
        return None;
    }

    let metadata = fs::metadata(path).ok()?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    Some(ImportedFile {
        path: path.to_string_lossy().to_string(),
        name,
        extension: extension.clone(),
        size_bytes: metadata.len(),
        file_type: detect_file_type(&extension),
    })
}

fn scan_directory(path: &Path, recursive: bool) -> Vec<ImportedFile> {
    let mut files = Vec::new();

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return files,
    };

    for entry in entries.filter_map(Result::ok) {
        let entry_path = entry.path();

        if entry_path.is_dir() {
            if recursive {
                files.extend(scan_directory(&entry_path, true));
            }
        } else if let Some(imported) = create_imported_file(&entry_path) {
            files.push(imported);
        }
    }

    files
}

#[tauri::command]
pub fn import_files(paths: Vec<String>) -> Result<ImportResult, String> {
    let mut files = Vec::new();
    let mut skipped = 0;
    let total = paths.len();

    for path_str in paths {
        let path = Path::new(&path_str);

        if !path.exists() {
            skipped += 1;
            continue;
        }

        if path.is_dir() {
            skipped += 1;
            continue;
        }

        match create_imported_file(path) {
            Some(imported) => files.push(imported),
            None => skipped += 1,
        }
    }

    Ok(ImportResult {
        files,
        total_count: total,
        skipped_count: skipped,
    })
}

#[tauri::command]
pub fn import_folder(path: String, recursive: bool) -> Result<ImportResult, String> {
    let folder_path = Path::new(&path);

    if !folder_path.exists() {
        return Err(format!("Folder does not exist: {}", path));
    }

    if !folder_path.is_dir() {
        return Err(format!("Path is not a directory: {}", path));
    }

    let files = scan_directory(folder_path, recursive);
    let total = files.len();

    Ok(ImportResult {
        files,
        total_count: total,
        skipped_count: 0,
    })
}

#[tauri::command]
pub fn import_folders(paths: Vec<String>, recursive: bool) -> Result<ImportResult, String> {
    let mut all_files = Vec::new();
    let mut total_skipped = 0;

    for path_str in &paths {
        let folder_path = Path::new(path_str);

        if !folder_path.exists() || !folder_path.is_dir() {
            total_skipped += 1;
            continue;
        }

        let files = scan_directory(folder_path, recursive);
        all_files.extend(files);
    }

    let total = all_files.len();

    Ok(ImportResult {
        files: all_files,
        total_count: total,
        skipped_count: total_skipped,
    })
}

#[tauri::command]
pub fn get_supported_extensions() -> Vec<String> {
    SUPPORTED_EXTENSIONS.iter().map(|s| s.to_string()).collect()
}
