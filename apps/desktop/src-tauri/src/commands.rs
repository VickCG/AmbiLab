use arrow::array::Array;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::BufReader;
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
pub fn read_csv(path: String, limit: usize, offset: usize) -> Result<DataPreview, String> {
    let file = File::open(&path).map_err(|e| e.to_string())?;
    let mut reader = csv::Reader::from_reader(BufReader::new(file));

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| e.to_string())?;
        let row: Vec<String> = record.iter().map(|f| f.to_string()).collect();
        all_rows.push(row);
    }

    let total_rows = all_rows.len();
    let rows: Vec<Vec<String>> = all_rows
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    Ok(DataPreview {
        columns: headers,
        rows,
        total_rows,
    })
}

#[tauri::command]
pub fn read_parquet(path: String, limit: usize, offset: usize) -> Result<DataPreview, String> {
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

    let file = File::open(&path).map_err(|e| e.to_string())?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(|e| e.to_string())?;

    let metadata = builder.metadata();
    let total_rows = metadata.file_metadata().num_rows() as usize;

    let reader = builder.build().map_err(|e| e.to_string())?;

    let mut columns: Vec<String> = Vec::new();
    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut columns_set = false;

    for batch_result in reader {
        let batch = batch_result.map_err(|e| e.to_string())?;

        if !columns_set {
            columns = batch
                .schema()
                .fields()
                .iter()
                .map(|f| f.name().clone())
                .collect();
            columns_set = true;
        }

        let num_rows = batch.num_rows();
        for row_idx in 0..num_rows {
            let mut row: Vec<String> = Vec::with_capacity(batch.num_columns());
            for col_idx in 0..batch.num_columns() {
                let col = batch.column(col_idx);
                let value = format_array_value(col.as_ref(), row_idx);
                row.push(value);
            }
            all_rows.push(row);
        }
    }

    let rows: Vec<Vec<String>> = all_rows
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    Ok(DataPreview {
        columns,
        rows,
        total_rows,
    })
}

fn format_array_value(array: &dyn Array, idx: usize) -> String {
    use arrow::array::{
        Float32Array, Float64Array, Int32Array, Int64Array, StringArray, BooleanArray,
    };
    use arrow::datatypes::DataType;

    if array.is_null(idx) {
        return "NULL".to_string();
    }

    match array.data_type() {
        DataType::Utf8 => {
            let arr = array.as_any().downcast_ref::<StringArray>().unwrap();
            arr.value(idx).to_string()
        }
        DataType::Int32 => {
            let arr = array.as_any().downcast_ref::<Int32Array>().unwrap();
            arr.value(idx).to_string()
        }
        DataType::Int64 => {
            let arr = array.as_any().downcast_ref::<Int64Array>().unwrap();
            arr.value(idx).to_string()
        }
        DataType::Float32 => {
            let arr = array.as_any().downcast_ref::<Float32Array>().unwrap();
            arr.value(idx).to_string()
        }
        DataType::Float64 => {
            let arr = array.as_any().downcast_ref::<Float64Array>().unwrap();
            arr.value(idx).to_string()
        }
        DataType::Boolean => {
            let arr = array.as_any().downcast_ref::<BooleanArray>().unwrap();
            arr.value(idx).to_string()
        }
        _ => format!("{:?}", array.data_type()),
    }
}
