use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaneState {
    pub id: usize,
    pub pair: String,
    pub timeframe: String,
    pub indicator: String,
    pub view_bars: i64,
    pub view_offset: i64,
    pub playing: bool,
    pub speed: f64,
    pub seek: i64,
    pub bars: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Preset {
    pub name: String,
    pub split: usize,
    pub panes: Vec<PaneState>,
}

fn presets_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    Ok(base.join("presets.json"))
}

fn load_all(app: &AppHandle) -> Result<Vec<Preset>, String> {
    let path = presets_path(app)?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let presets: Vec<Preset> = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(presets)
}

fn save_all(app: &AppHandle, presets: &[Preset]) -> Result<(), String> {
    let path = presets_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(presets).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_presets(app: &AppHandle) -> Result<Vec<Preset>, String> {
    load_all(app)
}

pub fn save_preset(app: &AppHandle, preset: Preset) -> Result<(), String> {
    if preset.name.trim().is_empty() {
        return Err("preset name is required".to_string());
    }
    let mut presets = load_all(app)?;
    presets.retain(|p| p.name != preset.name);
    presets.push(preset);
    save_all(app, &presets)
}

pub fn delete_preset(app: &AppHandle, name: &str) -> Result<bool, String> {
    let mut presets = load_all(app)?;
    let before = presets.len();
    presets.retain(|p| p.name != name);
    if presets.len() == before {
        return Ok(false);
    }
    save_all(app, &presets)?;
    Ok(true)
}

pub fn load_preset(app: &AppHandle, name: &str) -> Result<Preset, String> {
    let presets = load_all(app)?;
    presets
        .into_iter()
        .find(|p| p.name == name)
        .ok_or_else(|| "preset not found".to_string())
}
