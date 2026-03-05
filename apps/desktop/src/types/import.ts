export type FileType =
  | "csv"
  | "tsv"
  | "json"
  | "jsonlines"
  | "parquet"
  | "arrow"
  | "excel"
  | "sql"
  | "sqlite"
  | "unknown";

export interface ImportedFile {
  path: string;
  name: string;
  extension: string;
  size_bytes: number;
  file_type: FileType;
}

export interface ImportResult {
  files: ImportedFile[];
  total_count: number;
  skipped_count: number;
}

export type ImportMode = "files" | "folder" | "folders";

export const FILE_TYPE_LABELS: Record<FileType, string> = {
  csv: "CSV",
  tsv: "TSV",
  json: "JSON",
  jsonlines: "JSON Lines",
  parquet: "Parquet",
  arrow: "Arrow/Feather",
  excel: "Excel",
  sql: "SQL",
  sqlite: "SQLite",
  unknown: "Unknown",
};

export const FILE_TYPE_COLORS: Record<FileType, string> = {
  csv: "#4caf50",
  tsv: "#4caf50",
  json: "#ff9800",
  jsonlines: "#ff9800",
  parquet: "#2196f3",
  arrow: "#9c27b0",
  excel: "#1d6f42",
  sql: "#e38c00",
  sqlite: "#003b57",
  unknown: "#969696",
};

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}
