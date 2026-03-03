#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autosave;
mod commands;
mod file_cache;
mod import;
mod workspace;
mod workspace_config;

use autosave::AutoSaveManager;
use file_cache::{FileCache, FileCacheState};
use std::sync::Arc;

fn main() {
    let autosave_manager = Arc::new(AutoSaveManager::new());

    tauri::Builder::default()
        .setup({
            let autosave_manager = autosave_manager.clone();
            move |_app| {
                autosave_manager.start_background_task();
                Ok(())
            }
        })
        .manage(commands::AutoSaveState(autosave_manager))
        .manage(FileCacheState(Arc::new(FileCache::new())))
        .invoke_handler(tauri::generate_handler![
            commands::read_dir,
            commands::read_file,
            commands::write_file,
            commands::analyze_sql,
            commands::read_csv,
            commands::read_parquet,
            commands::read_json,
            commands::read_jsonl,
            commands::load_workspace_config,
            commands::save_workspace_config,
            commands::workspace_config_exists,
            commands::update_workspace_config,
            commands::create_workspace_with_config,
            import::import_files,
            import::import_folder,
            import::import_folders,
            import::get_supported_extensions,
            workspace::create_workspace,
            workspace::create_workspaces,
            workspace::get_workspace_info,
            workspace::rename_workspace,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
