use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

const SESSION_FILE: &str = "session.json";

/// Persisted across app restarts in `<workspace>/settings/session.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionConfig {
    pub last_project: Option<String>,
    pub open_files: Vec<String>,
    pub active_dataset: Option<String>,
}

impl SessionConfig {
    fn session_path(workspace_path: &str) -> PathBuf {
        PathBuf::from(workspace_path)
            .join("settings")
            .join(SESSION_FILE)
    }

    pub async fn load(workspace_path: &str) -> Result<Self, String> {
        let path = Self::session_path(workspace_path);
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path).await.map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    pub async fn save(&self, workspace_path: &str) -> Result<(), String> {
        let path = Self::session_path(workspace_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("tmp");
        let mut file = fs::File::create(&tmp).await.map_err(|e| e.to_string())?;
        file.write_all(json.as_bytes()).await.map_err(|e| e.to_string())?;
        file.sync_all().await.map_err(|e| e.to_string())?;
        fs::rename(&tmp, &path).await.map_err(|e| e.to_string())
    }
}

/// Loads the session for `workspace_path`. Returns a default (empty) session
/// if no `session.json` exists yet.
#[tauri::command]
pub async fn load_session(workspace_path: String) -> Result<SessionConfig, String> {
    SessionConfig::load(&workspace_path).await
}

#[tauri::command]
pub async fn save_session(workspace_path: String, config: SessionConfig) -> Result<(), String> {
    config.save(&workspace_path).await
}
