use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use thiserror::Error;
use tokio::fs;
use tokio::io::AsyncWriteExt;

const CONFIG_DIR: &str = ".ambilab";
const CONFIG_FILE: &str = "workspace.json";
const CURRENT_VERSION: u32 = 1;

#[derive(Error, Debug)]
pub enum WorkspaceConfigError {
    #[error("IO error: {0}")]
    Io(std::io::Error),

    #[error("JSON serialization error: {0}")]
    Serialization(serde_json::Error),

    #[error("workspace config not found at {0}")]
    NotFound(PathBuf),

    #[error("invalid workspace path: {0}")]
    InvalidPath(String),

    #[error("config version mismatch: got {found}, expected {expected}")]
    VersionMismatch { found: u32, expected: u32 },
}

impl From<std::io::Error> for WorkspaceConfigError {
    fn from(err: std::io::Error) -> Self {
        WorkspaceConfigError::Io(err)
    }
}

impl From<serde_json::Error> for WorkspaceConfigError {
    fn from(err: serde_json::Error) -> Self {
        WorkspaceConfigError::Serialization(err)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub metadata: WorkspaceMetadata,
    pub editor_state: EditorState,
    pub tree_state: TreeState,
    pub workspace_settings: WorkspaceSettings,
    pub ui_state: UiState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMetadata {
    pub id: String,
    pub name: String,
    pub path: String,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_opened: DateTime<Utc>,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorState {
    pub open_files: Vec<OpenFileState>,
    pub active_file_index: i32,
    pub closed_tabs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenFileState {
    pub path: String,
    pub relative_path: String,
    pub content_hash: String,
    pub cursor_position: CursorPosition,
    pub scroll_position: ScrollPosition,
    pub is_dirty: bool,
    pub last_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollPosition {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeState {
    pub expanded_folders: Vec<String>,
    pub pinned_items: Vec<String>,
    pub collapsed_workspaces: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    pub default_query_template: String,
    pub auto_save_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiState {
    pub sidebar_width: u32,
    pub chat_width: u32,
    pub panel_height: u32,
}

impl WorkspaceConfig {
    pub fn new(id: String, name: String, path: String, color: Option<String>) -> Self {
        Self {
            metadata: WorkspaceMetadata {
                id,
                name,
                path,
                color,
                created_at: Utc::now(),
                last_opened: Utc::now(),
                version: CURRENT_VERSION,
            },
            editor_state: EditorState {
                open_files: Vec::with_capacity(10),
                active_file_index: -1,
                closed_tabs: Vec::with_capacity(5),
            },
            tree_state: TreeState {
                expanded_folders: Vec::with_capacity(20),
                pinned_items: Vec::with_capacity(5),
                collapsed_workspaces: Vec::with_capacity(5),
            },
            workspace_settings: WorkspaceSettings {
                default_query_template: "sql".to_string(),
                auto_save_interval_ms: 2000,
            },
            ui_state: UiState {
                sidebar_width: 280,
                chat_width: 350,
                panel_height: 200,
            },
        }
    }

    fn get_config_dir(workspace_path: &str) -> PathBuf {
        PathBuf::from(workspace_path).join(CONFIG_DIR)
    }

    fn get_config_path(workspace_path: &str) -> PathBuf {
        Self::get_config_dir(workspace_path).join(CONFIG_FILE)
    }

    pub async fn load(workspace_path: &str) -> Result<Self, WorkspaceConfigError> {
        let config_path = Self::get_config_path(workspace_path);

        if !config_path.exists() {
            return Err(WorkspaceConfigError::NotFound(config_path));
        }

        let content = fs::read_to_string(&config_path).await?;
        let config: Self = serde_json::from_str(&content)?;

        if config.metadata.version != CURRENT_VERSION {
            return Err(WorkspaceConfigError::VersionMismatch {
                found: config.metadata.version,
                expected: CURRENT_VERSION,
            });
        }

        Ok(config)
    }

    pub async fn save(&self, workspace_path: &str) -> Result<(), WorkspaceConfigError> {
        let config_dir = Self::get_config_dir(workspace_path);
        let config_path = Self::get_config_path(workspace_path);

        fs::create_dir_all(&config_dir).await?;

        let json = serde_json::to_string_pretty(self)?;

        let temp_path = config_path.with_extension("tmp");

        let mut file = fs::File::create(&temp_path).await?;
        file.write_all(json.as_bytes()).await?;
        file.sync_all().await?;

        fs::rename(&temp_path, &config_path).await?;

        Ok(())
    }

    pub fn exists(workspace_path: &str) -> bool {
        Self::get_config_path(workspace_path).exists()
    }

    pub fn touch(&mut self) {
        self.metadata.last_opened = Utc::now();
    }
}

pub fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("sha256:{:x}", result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_and_load_config() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let path = temp_dir.path().to_str().expect("failed to get path");

        let config = WorkspaceConfig::new(
            "test_id".to_string(),
            "Test".to_string(),
            path.to_string(),
            None,
        );

        config.save(path).await.expect("failed to save");

        let loaded = WorkspaceConfig::load(path)
            .await
            .expect("failed to load");
        assert_eq!(loaded.metadata.id, "test_id");
    }

    #[tokio::test]
    async fn test_atomic_write() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let path = temp_dir.path().to_str().expect("failed to get path");

        let config = WorkspaceConfig::new(
            "test_id".to_string(),
            "Test".to_string(),
            path.to_string(),
            None,
        );

        config.save(path).await.expect("failed to save");

        let temp_path = WorkspaceConfig::get_config_path(path).with_extension("tmp");
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_serialization() {
        let config = WorkspaceConfig::new(
            "test_id".to_string(),
            "Test".to_string(),
            "/test/path".to_string(),
            Some("#4caf50".to_string()),
        );

        let json = serde_json::to_string(&config).expect("failed to serialize");
        let deserialized: WorkspaceConfig =
            serde_json::from_str(&json).expect("failed to deserialize");

        assert_eq!(config.metadata.id, deserialized.metadata.id);
    }

    #[test]
    fn test_content_hash() {
        let content = "SELECT * FROM users";
        let hash = compute_content_hash(content);
        assert!(hash.starts_with("sha256:"));
    }

    #[tokio::test]
    async fn test_missing_directory() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let path = temp_dir.path().to_str().expect("failed to get path");

        let config = WorkspaceConfig::new(
            "test_id".to_string(),
            "Test".to_string(),
            path.to_string(),
            None,
        );

        config.save(path).await.expect("failed to save");

        let config_dir = PathBuf::from(path).join(".ambilab");
        assert!(config_dir.exists());
    }
}
