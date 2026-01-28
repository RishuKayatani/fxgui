// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod core;
mod core_tests;

#[tauri::command]
fn ingest_csv(path: &str, app: tauri::AppHandle) -> Result<core::IngestResult, String> {
    core::load_csv_or_tsv(&app, path)
}

#[tauri::command]
fn clear_cache(app: tauri::AppHandle) -> Result<u64, String> {
    core::clear_cache(&app)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![ingest_csv, clear_cache])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
