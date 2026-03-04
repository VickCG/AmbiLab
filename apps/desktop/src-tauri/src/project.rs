use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

const PROJECT_FILE: &str = "project.json";
const PROJECT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Active,
    Stable,
    Archived,
}

/// A reference to a dataset used by this project.
/// `path` is relative to the project folder (e.g. `datasets/sales.csv`)
/// or prefixed with `../../datasets/` for workspace-shared datasets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRef {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub datasets: Vec<DatasetRef>,
    pub status: ProjectStatus,
    pub version: u32,
}

impl ProjectConfig {
    fn new(name: &str) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        Self {
            id: format!("proj_{:x}{:x}", dur.as_secs(), dur.subsec_nanos()),
            name: name.to_string(),
            created_at: Utc::now().to_rfc3339(),
            datasets: Vec::new(),
            status: ProjectStatus::Active,
            version: PROJECT_VERSION,
        }
    }

    pub fn project_dir(workspace_path: &str, name: &str) -> PathBuf {
        PathBuf::from(workspace_path).join("projects").join(name)
    }

    fn config_path(workspace_path: &str, name: &str) -> PathBuf {
        Self::project_dir(workspace_path, name).join(PROJECT_FILE)
    }

    pub async fn load(workspace_path: &str, name: &str) -> Result<Self, String> {
        let path = Self::config_path(workspace_path, name);
        let content = fs::read_to_string(&path).await.map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    pub async fn save(&self, workspace_path: &str) -> Result<(), String> {
        let path = Self::config_path(workspace_path, &self.name);
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("tmp");
        let mut file = fs::File::create(&tmp).await.map_err(|e| e.to_string())?;
        file.write_all(json.as_bytes()).await.map_err(|e| e.to_string())?;
        file.sync_all().await.map_err(|e| e.to_string())?;
        fs::rename(&tmp, &path).await.map_err(|e| e.to_string())
    }
}

async fn scaffold_project_dirs(dir: &Path) -> Result<(), String> {
    for subdir in &["datasets", "queries", "charts", "notes"] {
        fs::create_dir_all(dir.join(subdir))
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn register_in_workspace(workspace_path: &str, project_name: &str) -> Result<(), String> {
    use crate::workspace_root::WorkspaceRoot;
    let mut root = WorkspaceRoot::load(workspace_path).await?;
    if root.projects.contains(&project_name.to_string()) {
        return Ok(());
    }
    root.projects.push(project_name.to_string());
    root.save(workspace_path).await
}

/// Creates a new project under `workspace_path/projects/<name>/`.
/// Scaffolds `datasets/`, `queries/`, `charts/`, `notes/` subdirectories
/// and writes `project.json`. Idempotent — loads existing config if project already exists.
#[tauri::command]
pub async fn create_project(workspace_path: String, name: String) -> Result<ProjectConfig, String> {
    let dir = ProjectConfig::project_dir(&workspace_path, &name);
    if dir.exists() {
        return ProjectConfig::load(&workspace_path, &name).await;
    }
    fs::create_dir_all(&dir).await.map_err(|e| e.to_string())?;
    scaffold_project_dirs(&dir).await?;
    let config = ProjectConfig::new(&name);
    config.save(&workspace_path).await?;
    let has_manifest = PathBuf::from(&workspace_path).join("workspace.json").exists();
    if has_manifest {
        register_in_workspace(&workspace_path, &name).await?;
    }
    Ok(config)
}

#[tauri::command]
pub async fn load_project(workspace_path: String, name: String) -> Result<ProjectConfig, String> {
    ProjectConfig::load(&workspace_path, &name).await
}

#[tauri::command]
pub async fn save_project(workspace_path: String, config: ProjectConfig) -> Result<(), String> {
    config.save(&workspace_path).await
}

/// Lists all projects that have a valid `project.json` under `workspace_path/projects/`.
#[tauri::command]
pub async fn list_projects(workspace_path: String) -> Result<Vec<ProjectConfig>, String> {
    let projects_dir = PathBuf::from(&workspace_path).join("projects");
    if !projects_dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = fs::read_dir(&projects_dir).await.map_err(|e| e.to_string())?;
    let mut projects = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        let meta = entry.metadata().await.map_err(|e| e.to_string())?;
        if !meta.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if let Ok(cfg) = ProjectConfig::load(&workspace_path, &name).await {
            projects.push(cfg);
        }
    }
    Ok(projects)
}

/// Marks a project as `archived` without moving files.
#[tauri::command]
pub async fn archive_project(workspace_path: String, name: String) -> Result<(), String> {
    let mut config = ProjectConfig::load(&workspace_path, &name).await?;
    config.status = ProjectStatus::Archived;
    config.save(&workspace_path).await
}

/// Transitions a project from `active` → `stable`.
#[tauri::command]
pub async fn finalize_project(workspace_path: String, name: String) -> Result<(), String> {
    let mut config = ProjectConfig::load(&workspace_path, &name).await?;
    config.status = ProjectStatus::Stable;
    config.save(&workspace_path).await
}
