use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

const WORKSPACE_FILE: &str = "workspace.json";
const WORKSPACE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRoot {
    pub workspace_version: u32,
    pub created_at: String,
    pub projects: Vec<String>,
}

impl WorkspaceRoot {
    fn new() -> Self {
        Self {
            workspace_version: WORKSPACE_VERSION,
            created_at: Utc::now().to_rfc3339(),
            projects: Vec::new(),
        }
    }

    fn manifest_path(workspace_path: &str) -> PathBuf {
        PathBuf::from(workspace_path).join(WORKSPACE_FILE)
    }

    pub fn exists(workspace_path: &str) -> bool {
        Self::manifest_path(workspace_path).exists()
    }

    pub async fn load(workspace_path: &str) -> Result<Self, String> {
        let path = Self::manifest_path(workspace_path);
        let content = fs::read_to_string(&path).await.map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    pub async fn save(&self, workspace_path: &str) -> Result<(), String> {
        let path = Self::manifest_path(workspace_path);
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("tmp");
        let mut file = fs::File::create(&tmp).await.map_err(|e| e.to_string())?;
        file.write_all(json.as_bytes()).await.map_err(|e| e.to_string())?;
        file.sync_all().await.map_err(|e| e.to_string())?;
        fs::rename(&tmp, &path).await.map_err(|e| e.to_string())
    }
}

async fn create_workspace_dirs(base: &Path) -> Result<(), String> {
    let dirs = [
        "projects",
        "datasets",
        "cache/duckdb",
        "cache/query_results",
        "settings",
    ];
    for dir in &dirs {
        fs::create_dir_all(base.join(dir))
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Initializes a workspace at `path`: creates folder structure and `workspace.json`.
/// Idempotent — safe to call on an existing workspace (loads and returns existing manifest).
#[tauri::command]
pub async fn initialize_workspace(path: String) -> Result<WorkspaceRoot, String> {
    let base = Path::new(&path);
    if !base.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    create_workspace_dirs(base).await?;
    if WorkspaceRoot::exists(&path) {
        return WorkspaceRoot::load(&path).await;
    }
    let root = WorkspaceRoot::new();
    root.save(&path).await?;
    Ok(root)
}

/// Returns `true` if `path` contains a valid `workspace.json`.
#[tauri::command]
pub fn detect_workspace(path: String) -> bool {
    WorkspaceRoot::exists(&path)
}

#[tauri::command]
pub async fn load_workspace_root(path: String) -> Result<WorkspaceRoot, String> {
    WorkspaceRoot::load(&path).await
}

#[tauri::command]
pub async fn save_workspace_root(path: String, root: WorkspaceRoot) -> Result<(), String> {
    root.save(&path).await
}
