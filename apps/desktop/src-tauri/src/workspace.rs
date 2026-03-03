use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub path: String,
    pub color: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub workspace: Workspace,
    pub file_count: usize,
    pub folder_count: usize,
}

impl Workspace {
    pub fn from_path(path: &str) -> Self {
        let path_obj = Path::new(path);
        let name = path_obj
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Workspace")
            .to_string();

        let id = format!("ws_{}", uuid_simple());

        Self {
            id,
            name,
            path: path.to_string(),
            color: None,
        }
    }
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}{:x}", duration.as_secs(), duration.subsec_nanos())
}

#[tauri::command]
pub fn create_workspace(path: String) -> Result<Workspace, String> {
    let path_obj = Path::new(&path);

    if !path_obj.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    if !path_obj.is_dir() {
        return Err(format!("Path is not a directory: {}", path));
    }

    Ok(Workspace::from_path(&path))
}

#[tauri::command]
pub fn create_workspaces(paths: Vec<String>) -> Result<Vec<Workspace>, String> {
    let mut workspaces = Vec::with_capacity(paths.len());

    for path in paths {
        let path_obj = Path::new(&path);

        if !path_obj.exists() || !path_obj.is_dir() {
            continue;
        }

        workspaces.push(Workspace::from_path(&path));
    }

    Ok(workspaces)
}

#[tauri::command]
pub fn get_workspace_info(path: String) -> Result<WorkspaceInfo, String> {
    use std::fs;

    let path_obj = Path::new(&path);

    if !path_obj.exists() || !path_obj.is_dir() {
        return Err(format!("Invalid workspace path: {}", path));
    }

    let mut file_count = 0;
    let mut folder_count = 0;

    if let Ok(entries) = fs::read_dir(&path) {
        for entry in entries.filter_map(Result::ok) {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    folder_count += 1;
                } else {
                    file_count += 1;
                }
            }
        }
    }

    Ok(WorkspaceInfo {
        workspace: Workspace::from_path(&path),
        file_count,
        folder_count,
    })
}

#[tauri::command]
pub fn rename_workspace(id: String, new_name: String) -> Result<(), String> {
    if new_name.trim().is_empty() {
        return Err("Workspace name cannot be empty".to_string());
    }
    // Note: The actual renaming is handled in frontend state
    // This command just validates the input
    let _ = id;
    Ok(())
}
