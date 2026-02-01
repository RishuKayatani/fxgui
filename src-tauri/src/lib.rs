// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod core;
mod core_tests;
mod indicators;
mod indicators_tests;
mod resample;
mod presets;
mod logger;

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
fn cache_status(app: tauri::AppHandle) -> Result<core::CacheStatus, String> {
    core::cache_status(&app)
}

#[tauri::command]
fn list_dataset_history(app: tauri::AppHandle) -> Result<Vec<core::DatasetHistory>, String> {
    core::list_dataset_history(&app)
}

#[tauri::command]
fn record_dataset_history(app: tauri::AppHandle, path: &str) -> Result<(), String> {
    core::record_dataset_history(&app, path)
}

#[derive(serde::Serialize)]
struct RangeResult {
    candles: Vec<core::Candle>,
}

#[derive(serde::Serialize)]
struct IndicatorRangeResult {
    series: Vec<Option<f64>>,
}

#[derive(serde::Serialize)]
struct QuickIngestResult {
    source_path: String,
    total: usize,
    used_cache: bool,
    initial: Vec<core::Candle>,
}

#[tauri::command]
fn ingest_csv_quick(
    app: tauri::AppHandle,
    path: String,
    initial_limit: usize,
) -> Result<QuickIngestResult, String> {
    let total = match core::cached_bar_count(&app, &path) {
        Ok(Some(count)) => Some(count),
        _ => None,
    };
    if let Some(count) = total {
        let initial = core::load_range_from_cache(&app, &path, 0, initial_limit)?
            .unwrap_or_default();
        return Ok(QuickIngestResult {
            source_path: path,
            total: count,
            used_cache: true,
            initial,
        });
    }

    let initial = core::load_range_from_path(&path, 0, initial_limit)?;
    Ok(QuickIngestResult {
        source_path: path,
        total: initial.len(),
        used_cache: false,
        initial,
    })
}

#[tauri::command]
fn dataset_range(
    app: tauri::AppHandle,
    source_path: String,
    offset: usize,
    limit: usize,
) -> Result<RangeResult, String> {
    if let Ok(Some(candles)) = core::load_range_from_cache(&app, &source_path, offset, limit) {
        return Ok(RangeResult { candles });
    }
    let candles = core::load_range_from_path(&source_path, offset, limit)?;
    Ok(RangeResult { candles })
}

#[tauri::command]
fn indicator_range(
    app: tauri::AppHandle,
    source_path: String,
    offset: usize,
    limit: usize,
    indicator: String,
) -> Result<IndicatorRangeResult, String> {
    let data = core::load_csv_or_tsv(&app, &source_path)?;
    let full = compute_indicators(app.clone(), data.dataset)?;
    let key = indicator.to_lowercase();
    let series = full
        .get(&key)
        .or_else(|| full.get("ma"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| "indicator not found".to_string())?;
    let total = series.len();
    let start = offset.min(total);
    let end = (start + limit).min(total);
    let mut out = Vec::with_capacity(end - start);
    for value in &series[start..end] {
        if value.is_null() {
            out.push(None);
        } else {
            out.push(value.as_f64());
        }
    }
    Ok(IndicatorRangeResult { series: out })
}

#[tauri::command]
fn compute_indicators(
    app: tauri::AppHandle,
    dataset: core::DataSet,
) -> Result<serde_json::Value, String> {
    let use_cache = !dataset.source_path.trim().is_empty();
    if use_cache {
        if let Ok(Some(cached)) = core::load_indicator_cache(
            &app,
            &dataset.source_path,
            dataset.candles.len(),
            &["ma", "rsi", "macd", "signal", "hist"],
        ) {
            return Ok(cached);
        }
    }

    let start = std::time::Instant::now();
    let closes = indicators::closes_from_candles(&dataset.candles);
    let t_closes = start.elapsed().as_millis();
    let ma = indicators::ma(&closes, 14);
    let t_ma = start.elapsed().as_millis();
    let rsi = indicators::rsi(&closes, 14);
    let t_rsi = start.elapsed().as_millis();
    let (macd, signal, hist) = indicators::macd(&closes, 12, 26, 9);
    let t_macd = start.elapsed().as_millis();

    let total = start.elapsed().as_millis();
    if cfg!(debug_assertions) {
        println!(
            "[perf] indicators closes={}ms ma={}ms rsi={}ms macd={}ms total={}ms",
            t_closes, t_ma, t_rsi, t_macd, total
        );
    }

    if use_cache {
        let _ = core::save_indicator_cache(
            &app,
            &dataset.source_path,
            &[
                ("ma", ma.clone()),
                ("rsi", rsi.clone()),
                ("macd", macd.clone()),
                ("signal", signal.clone()),
                ("hist", hist.clone()),
            ],
        );
    }

    Ok(serde_json::json!({
        "ma": ma,
        "rsi": rsi,
        "macd": macd,
        "signal": signal,
        "hist": hist,
    }))
}

#[tauri::command]
fn resample_dataset(
    app: tauri::AppHandle,
    dataset: core::DataSet,
    target: String,
) -> Result<core::DataSet, String> {
    let start = std::time::Instant::now();
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

    if let Ok(Some(cached)) = core::load_resample_cache(&app, &dataset.source_path, &target) {
        return Ok(cached);
    }

    let resampled = resample::resample(&dataset, interval)?;
    let _ = core::save_resample_cache(&app, &dataset.source_path, &target, &resampled);
    if cfg!(debug_assertions) {
        println!(
            "[perf] resample {} total={}ms",
            target,
            start.elapsed().as_millis()
        );
    }
    Ok(resampled)
}

#[tauri::command]
fn list_presets(app: tauri::AppHandle) -> Result<Vec<presets::Preset>, String> {
    presets::list_presets(&app)
}

#[tauri::command]
fn save_preset(app: tauri::AppHandle, preset: presets::Preset) -> Result<(), String> {
    presets::save_preset(&app, preset)
}

#[tauri::command]
fn delete_preset(app: tauri::AppHandle, name: &str) -> Result<bool, String> {
    presets::delete_preset(&app, name)
}

#[tauri::command]
fn load_preset(app: tauri::AppHandle, name: &str) -> Result<presets::Preset, String> {
    presets::load_preset(&app, name)
}

#[tauri::command]
fn save_playback_state(
    app: tauri::AppHandle,
    state: presets::PlaybackState,
) -> Result<(), String> {
    presets::save_playback(&app, state)
}

#[tauri::command]
fn load_playback_state(app: tauri::AppHandle) -> Result<Option<presets::PlaybackState>, String> {
    presets::load_playback(&app)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            ingest_csv,
            ingest_csv_quick,
            clear_cache,
            cache_status,
            list_dataset_history,
            record_dataset_history,
            dataset_range,
            indicator_range,
            compute_indicators,
            resample_dataset,
            list_presets,
            save_preset,
            delete_preset,
            load_preset,
            save_playback_state,
            load_playback_state
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
