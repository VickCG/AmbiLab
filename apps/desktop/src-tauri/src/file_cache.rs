use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::sync::Arc;

pub const PAGE_SIZE: usize = 100;

pub struct PageIndex {
    pub page_offsets: Vec<u64>,
    pub total_rows: usize,
    pub columns: Vec<String>,
}

pub struct FileCache {
    csv: RwLock<HashMap<String, Arc<PageIndex>>>,
    jsonl: RwLock<HashMap<String, Arc<PageIndex>>>,
}

pub struct FileCacheState(pub Arc<FileCache>);

impl FileCache {
    pub fn new() -> Self {
        Self {
            csv: RwLock::new(HashMap::new()),
            jsonl: RwLock::new(HashMap::new()),
        }
    }

    pub fn get_or_build_csv(&self, path: &str) -> Result<Arc<PageIndex>, String> {
        if let Some(idx) = self.csv.read().get(path).cloned() {
            return Ok(idx);
        }
        let arc = Arc::new(build_csv_index(path)?);
        self.csv.write().insert(path.to_string(), arc.clone());
        Ok(arc)
    }

    pub fn get_or_build_jsonl(&self, path: &str) -> Result<Arc<PageIndex>, String> {
        if let Some(idx) = self.jsonl.read().get(path).cloned() {
            return Ok(idx);
        }
        let arc = Arc::new(build_jsonl_index(path)?);
        self.jsonl.write().insert(path.to_string(), arc.clone());
        Ok(arc)
    }
}

impl Default for FileCache {
    fn default() -> Self {
        Self::new()
    }
}

fn build_csv_index(path: &str) -> Result<PageIndex, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut reader = csv::Reader::from_reader(BufReader::new(file));

    let columns: Vec<String> = reader
        .headers()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|h| h.to_string())
        .collect();

    // position().byte() after headers = start of first data row
    let mut page_offsets = vec![reader.position().byte()];
    let mut total_rows = 0usize;
    let mut record = csv::StringRecord::new();

    loop {
        let has_more = reader.read_record(&mut record).map_err(|e| e.to_string())?;
        if !has_more {
            break;
        }
        total_rows += 1;
        if total_rows % PAGE_SIZE == 0 {
            // position().byte() after record N = start of record N+1
            page_offsets.push(reader.position().byte());
        }
    }

    Ok(PageIndex { page_offsets, total_rows, columns })
}

pub fn read_csv_page(path: &str, index: &PageIndex, page: usize) -> Result<Vec<Vec<String>>, String> {
    let &offset = index.page_offsets.get(page).ok_or("page out of range")?;

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;

    // has_headers(false): we seeked past the header already
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(BufReader::new(file));

    let mut rows: Vec<Vec<String>> = Vec::with_capacity(PAGE_SIZE);
    for result in reader.records().take(PAGE_SIZE) {
        let record = result.map_err(|e| e.to_string())?;
        rows.push(record.iter().map(|f| f.to_string()).collect());
    }

    Ok(rows)
}

fn build_jsonl_index(path: &str) -> Result<PageIndex, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(file);
    let mut columns = Vec::new();
    let mut page_offsets: Vec<u64> = Vec::new();
    let mut total_rows = 0usize;
    let mut byte_offset = 0u64;
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        if !line.trim().is_empty() {
            if total_rows == 0 {
                let v: Value = serde_json::from_str(line.trim()).map_err(|e| e.to_string())?;
                columns = jsonl_columns(&v);
                page_offsets.push(byte_offset);
            } else if total_rows % PAGE_SIZE == 0 {
                page_offsets.push(byte_offset);
            }
            total_rows += 1;
        }
        byte_offset += n as u64;
    }

    Ok(PageIndex { page_offsets, total_rows, columns })
}

pub fn read_jsonl_page(path: &str, index: &PageIndex, page: usize) -> Result<Vec<Vec<String>>, String> {
    let &offset = index.page_offsets.get(page).ok_or("page out of range")?;

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;

    let reader = BufReader::new(file);
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(PAGE_SIZE);

    for line_result in reader.lines() {
        if rows.len() >= PAGE_SIZE {
            break;
        }
        let line = line_result.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        rows.push(jsonl_row(&value, &index.columns));
    }

    Ok(rows)
}

fn jsonl_columns(value: &Value) -> Vec<String> {
    match value {
        Value::Object(map) => map.keys().cloned().collect(),
        _ => vec!["value".to_string()],
    }
}

fn jsonl_row(value: &Value, columns: &[String]) -> Vec<String> {
    match value {
        Value::Object(map) => columns.iter().map(|c| jsonl_value(map.get(c))).collect(),
        _ => vec![jsonl_value(Some(value))],
    }
}

fn jsonl_value(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "NULL".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Array(arr)) => serde_json::to_string(arr).unwrap_or_default(),
        Some(Value::Object(obj)) => serde_json::to_string(obj).unwrap_or_default(),
    }
}
