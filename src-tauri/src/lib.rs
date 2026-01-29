// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod core;
mod core_tests;
mod indicators;
mod indicators_tests;
mod resample;

#[allow(dead_code)]
fn _infer_interval_unused() {
    // placeholder to keep infer_interval referenced for now
}

#[tauri::command]
fn ingest_csv(path: &str, app: tauri::AppHandle) -> Result<core::IngestResult, String> {
    core::load_csv_or_tsv(&app, path)
}

#[tauri::command]
fn clear_cache(app: tauri::AppHandle) -> Result<u64, String> {
    core::clear_cache(&app)
}

#[tauri::command]
fn compute_indicators(dataset: core::DataSet) -> Result<serde_json::Value, String> {
    let closes = indicators::closes_from_candles(&dataset.candles);
    let ma = indicators::ma(&closes, 14);
    let rsi = indicators::rsi(&closes, 14);
    let (macd, signal, hist) = indicators::macd(&closes, 12, 26, 9);

    Ok(serde_json::json!({
        "ma": ma,
        "rsi": rsi,
        "macd": macd,
        "signal": signal,
        "hist": hist,
    }))
}

#[tauri::command]
fn resample_dataset(dataset: core::DataSet, target: String) -> Result<core::DataSet, String> {
    let interval = match target.as_str() {
        "M1" => resample::Interval::M1,
        "M5" => resample::Interval::M5,
        "M15" => resample::Interval::M15,
        "M30" => resample::Interval::M30,
        "H1" => resample::Interval::H1,
        "H4" => resample::Interval::H4,
        "D1" => resample::Interval::D1,
        _ => return Err("invalid interval".to_string()),
    };

    resample::resample(&dataset, interval)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            ingest_csv,
            clear_cache,
            compute_indicators,
            resample_dataset
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
