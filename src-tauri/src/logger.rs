use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub fn log_event(app: &AppHandle, message: &str) -> Result<(), String> {
    let path = log_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let line = format!("{} {}\n", chrono::Utc::now().to_rfc3339(), message);
    fs::write(&path, append_line(&path, line)?).map_err(|e| e.to_string())?;
    Ok(())
}

fn append_line(path: &Path, line: String) -> Result<Vec<u8>, String> {
    if path.exists() {
        let mut data = fs::read(path).map_err(|e| e.to_string())?;
        data.extend_from_slice(line.as_bytes());
        Ok(data)
    } else {
        Ok(line.into_bytes())
    }
}

fn log_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    Ok(base.join("logs").join("fxgui.log"))
}
