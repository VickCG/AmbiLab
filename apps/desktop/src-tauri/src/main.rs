#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::read_dir,
            commands::read_file,
            commands::write_file,
            commands::analyze_sql,
            commands::read_csv,
            commands::read_parquet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
