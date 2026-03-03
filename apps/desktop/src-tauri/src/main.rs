#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod import;
mod workspace;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::read_dir,
            commands::read_file,
            commands::write_file,
            commands::analyze_sql,
            commands::read_csv,
            commands::read_parquet,
            commands::read_json,
            commands::read_jsonl,
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
