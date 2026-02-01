use serde::{Deserialize, Serialize};
use rusqlite::OptionalExtension;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use crate::logger;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Candle {
    pub ts_utc: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataSet {
    pub source_path: String,
    pub candles: Vec<Candle>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IngestResult {
    pub dataset: DataSet,
    pub used_cache: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheStatus {
    pub path: String,
    pub files: u64,
    pub bytes: u64,
}

fn cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    Ok(base.join("cache"))
}

fn cache_key(path: &Path) -> Result<String, String> {
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    let mtime = meta.modified().map_err(|e| e.to_string())?;
    let mtime = mtime
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    Ok(format!("{}_{}", path.to_string_lossy(), mtime))
}

fn cache_path(app: &AppHandle, key: &str) -> Result<PathBuf, String> {
    Ok(cache_dir(app)?.join(format!("{}.sqlite", blake3::hash(key.as_bytes()))))
}

pub fn clear_cache(app: &AppHandle) -> Result<u64, String> {
    let dir = cache_dir(app)?;
    if !dir.exists() {
        return Ok(0);
    }
    let mut removed = 0;
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("sqlite") {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
            removed += 1;
        }
    }
    Ok(removed)
}

pub fn cache_status(app: &AppHandle) -> Result<CacheStatus, String> {
    let dir = cache_dir(app)?;
    if !dir.exists() {
        return Ok(CacheStatus {
            path: dir.to_string_lossy().to_string(),
            files: 0,
            bytes: 0,
        });
    }
    let mut files = 0;
    let mut bytes = 0;
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("sqlite") {
            files += 1;
            let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
            bytes += meta.len();
        }
    }
    Ok(CacheStatus {
        path: dir.to_string_lossy().to_string(),
        files,
        bytes,
    })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatasetHistory {
    pub path: String,
    pub last_used: i64,
}

fn history_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    Ok(base.join("dataset_history.json"))
}

pub fn list_dataset_history(app: &AppHandle) -> Result<Vec<DatasetHistory>, String> {
    let path = history_path(app)?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let history: Vec<DatasetHistory> = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(history)
}

pub fn record_dataset_history(app: &AppHandle, path: &str) -> Result<(), String> {
    let mut history = list_dataset_history(app)?;
    history.retain(|item| item.path != path);
    history.insert(
        0,
        DatasetHistory {
            path: path.to_string(),
            last_used: chrono::Utc::now().timestamp(),
        },
    );
    if history.len() > 10 {
        history.truncate(10);
    }
    let path = history_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(&history).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_csv_or_tsv(app: &AppHandle, path: &str) -> Result<IngestResult, String> {
    let start = std::time::Instant::now();
    let _ = logger::log_event(app, &format!("ingest start {}", path));
    let path = PathBuf::from(path);
    if !path.exists() {
        let _ = logger::log_event(app, "ingest error file not found");
        return Err("file not found".to_string());
    }

    let key = cache_key(&path)?;
    let cache_path = cache_path(app, &key)?;

    if cache_path.exists() {
        let cache_start = std::time::Instant::now();
        let dataset = load_from_cache(&cache_path)?;
        let _ = logger::log_event(
            app,
            &format!("ingest cache load {}ms", cache_start.elapsed().as_millis()),
        );
        let _ = logger::log_event(app, "ingest cache hit");
        let _ = logger::log_event(
            app,
            &format!("ingest total {}ms", start.elapsed().as_millis()),
        );
        return Ok(IngestResult {
            dataset,
            used_cache: true,
        });
    }

    let parse_start = std::time::Instant::now();
    let dataset = parse_csv_like(&path)?;
    let _ = logger::log_event(
        app,
        &format!("ingest parse {}ms", parse_start.elapsed().as_millis()),
    );
    let cache_write_start = std::time::Instant::now();
    save_to_cache(&cache_path, &dataset)?;
    let _ = logger::log_event(
        app,
        &format!(
            "ingest cache write {}ms",
            cache_write_start.elapsed().as_millis()
        ),
    );
    let _ = logger::log_event(app, "ingest success");
    let _ = logger::log_event(
        app,
        &format!("ingest total {}ms", start.elapsed().as_millis()),
    );

    Ok(IngestResult {
        dataset,
        used_cache: false,
    })
}

fn cache_path_for_source(app: &AppHandle, source_path: &str) -> Result<Option<PathBuf>, String> {
    if source_path.trim().is_empty() {
        return Ok(None);
    }
    let path = PathBuf::from(source_path);
    if !path.exists() {
        return Ok(None);
    }
    let key = cache_key(&path)?;
    Ok(Some(cache_path(app, &key)?))
}

pub fn load_resample_cache(
    app: &AppHandle,
    source_path: &str,
    target: &str,
) -> Result<Option<DataSet>, String> {
    let cache_path = match cache_path_for_source(app, source_path)? {
        Some(path) => path,
        None => return Ok(None),
    };
    if !cache_path.exists() {
        return Ok(None);
    }
    let conn = rusqlite::Connection::open(cache_path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS resample_meta (target TEXT PRIMARY KEY, count INTEGER);\n\
         CREATE TABLE IF NOT EXISTS resample_candles (\n\
           target TEXT,\n\
           idx INTEGER,\n\
           ts_utc TEXT,\n\
           open REAL,\n\
           high REAL,\n\
           low REAL,\n\
           close REAL,\n\
           volume REAL\n\
         );\n\
         CREATE INDEX IF NOT EXISTS idx_resample_target_idx ON resample_candles(target, idx);",
    )
    .map_err(|e| e.to_string())?;

    let count: Option<i64> = conn
        .query_row(
            "SELECT count FROM resample_meta WHERE target = ?1",
            [target],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let count = match count {
        Some(v) if v > 0 => v as usize,
        _ => return Ok(None),
    };

    let mut stmt = conn
        .prepare(
            "SELECT ts_utc, open, high, low, close, volume\n\
             FROM resample_candles WHERE target = ?1 ORDER BY idx ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([target], |row| {
            Ok(Candle {
                ts_utc: row.get(0)?,
                open: row.get(1)?,
                high: row.get(2)?,
                low: row.get(3)?,
                close: row.get(4)?,
                volume: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut candles = Vec::with_capacity(count);
    for row in rows {
        candles.push(row.map_err(|e| e.to_string())?);
    }
    if candles.len() != count {
        return Ok(None);
    }

    Ok(Some(DataSet {
        source_path: source_path.to_string(),
        candles,
    }))
}

pub fn save_resample_cache(
    app: &AppHandle,
    source_path: &str,
    target: &str,
    dataset: &DataSet,
) -> Result<(), String> {
    let cache_path = match cache_path_for_source(app, source_path)? {
        Some(path) => path,
        None => return Ok(()),
    };
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut conn = rusqlite::Connection::open(cache_path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS resample_meta (target TEXT PRIMARY KEY, count INTEGER);\n\
         CREATE TABLE IF NOT EXISTS resample_candles (\n\
           target TEXT,\n\
           idx INTEGER,\n\
           ts_utc TEXT,\n\
           open REAL,\n\
           high REAL,\n\
           low REAL,\n\
           close REAL,\n\
           volume REAL\n\
         );\n\
         CREATE INDEX IF NOT EXISTS idx_resample_target_idx ON resample_candles(target, idx);",
    )
    .map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM resample_candles WHERE target = ?1", [target])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM resample_meta WHERE target = ?1", [target])
        .map_err(|e| e.to_string())?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO resample_candles (target, idx, ts_utc, open, high, low, close, volume)\n\
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )
            .map_err(|e| e.to_string())?;
        for (idx, c) in dataset.candles.iter().enumerate() {
            stmt.execute((
                target,
                idx as i64,
                &c.ts_utc,
                c.open,
                c.high,
                c.low,
                c.close,
                c.volume,
            ))
            .map_err(|e| e.to_string())?;
        }
    }
    tx.execute(
        "INSERT INTO resample_meta (target, count) VALUES (?1, ?2)",
        (target, dataset.candles.len() as i64),
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_indicator_cache(
    app: &AppHandle,
    source_path: &str,
    expected_len: usize,
    indicators: &[&str],
) -> Result<Option<serde_json::Value>, String> {
    let cache_path = match cache_path_for_source(app, source_path)? {
        Some(path) => path,
        None => return Ok(None),
    };
    if !cache_path.exists() {
        return Ok(None);
    }
    let conn = rusqlite::Connection::open(cache_path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS indicator_meta (indicator TEXT PRIMARY KEY, count INTEGER);\n\
         CREATE TABLE IF NOT EXISTS indicator_values (\n\
           indicator TEXT,\n\
           idx INTEGER,\n\
           value REAL\n\
         );\n\
         CREATE INDEX IF NOT EXISTS idx_indicator_idx ON indicator_values(indicator, idx);",
    )
    .map_err(|e| e.to_string())?;

    let mut result = serde_json::Map::new();
    for name in indicators {
        let count: Option<i64> = conn
            .query_row(
                "SELECT count FROM indicator_meta WHERE indicator = ?1",
                [*name],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let count = match count {
            Some(v) if v as usize == expected_len => v as usize,
            _ => return Ok(None),
        };

        let mut stmt = conn
            .prepare(
                "SELECT idx, value FROM indicator_values WHERE indicator = ?1 ORDER BY idx ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([*name], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<f64>>(1)?)))
            .map_err(|e| e.to_string())?;

        let mut series = vec![serde_json::Value::Null; count];
        for row in rows {
            let (idx, value) = row.map_err(|e| e.to_string())?;
            let idx = idx as usize;
            if idx < series.len() {
                series[idx] = match value {
                    Some(v) => serde_json::Value::from(v),
                    None => serde_json::Value::Null,
                };
            }
        }
        result.insert((*name).to_string(), serde_json::Value::Array(series));
    }

    Ok(Some(serde_json::Value::Object(result)))
}

pub fn save_indicator_cache(
    app: &AppHandle,
    source_path: &str,
    indicators: &[(&str, Vec<Option<f64>>)],
) -> Result<(), String> {
    let cache_path = match cache_path_for_source(app, source_path)? {
        Some(path) => path,
        None => return Ok(()),
    };
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut conn = rusqlite::Connection::open(cache_path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS indicator_meta (indicator TEXT PRIMARY KEY, count INTEGER);\n\
         CREATE TABLE IF NOT EXISTS indicator_values (\n\
           indicator TEXT,\n\
           idx INTEGER,\n\
           value REAL\n\
         );\n\
         CREATE INDEX IF NOT EXISTS idx_indicator_idx ON indicator_values(indicator, idx);",
    )
    .map_err(|e| e.to_string())?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for (name, series) in indicators {
        tx.execute("DELETE FROM indicator_values WHERE indicator = ?1", [*name])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM indicator_meta WHERE indicator = ?1", [*name])
            .map_err(|e| e.to_string())?;
        let mut stmt = tx
            .prepare(
                "INSERT INTO indicator_values (indicator, idx, value) VALUES (?1, ?2, ?3)",
            )
            .map_err(|e| e.to_string())?;
        for (idx, value) in series.iter().enumerate() {
            stmt.execute((*name, idx as i64, value))
                .map_err(|e| e.to_string())?;
        }
        tx.execute(
            "INSERT INTO indicator_meta (indicator, count) VALUES (?1, ?2)",
            (*name, series.len() as i64),
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

fn parse_csv_like(path: &Path) -> Result<DataSet, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut candles = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Allow comments or header-like lines
        let lower = line.to_lowercase();
        if idx == 0 && (lower.contains("timestamp") || (lower.contains("date") && lower.contains("time"))) {
            continue;
        }
        let parts = split_line(line);
        if parts.len() < 5 {
            return Err(format!("invalid column count at line {}", idx + 1));
        }
        let (ts_raw, start_idx) = if parts.len() >= 6 && looks_like_date(parts[0]) && looks_like_time(parts[1]) {
            (format!("{} {}", parts[0].trim(), parts[1].trim()), 2)
        } else {
            (parts[0].trim().to_string(), 1)
        };
        let ts_utc = normalize_timestamp(&ts_raw).map_err(|e| format!("{} at line {}", e, idx + 1))?;
        if parts.len() < start_idx + 4 {
            return Err(format!("invalid column count at line {}", idx + 1));
        }
        let open = parse_f64(parts[start_idx]).map_err(|e| format!("{} at line {}", e, idx + 1))?;
        let high = parse_f64(parts[start_idx + 1]).map_err(|e| format!("{} at line {}", e, idx + 1))?;
        let low = parse_f64(parts[start_idx + 2]).map_err(|e| format!("{} at line {}", e, idx + 1))?;
        let close = parse_f64(parts[start_idx + 3]).map_err(|e| format!("{} at line {}", e, idx + 1))?;
        let volume = if parts.len() > start_idx + 4 {
            parse_f64(parts[start_idx + 4]).map_err(|e| format!("{} at line {}", e, idx + 1))?
        } else {
            0.0
        };

        candles.push(Candle {
            ts_utc,
            open,
            high,
            low,
            close,
            volume,
        });
    }

    Ok(DataSet {
        source_path: path.to_string_lossy().to_string(),
        candles,
    })
}

fn split_line(line: &str) -> Vec<&str> {
    if line.contains('\t') {
        line.split('\t').collect()
    } else if line.contains(',') {
        line.split(',').collect()
    } else {
        line.split_whitespace().collect()
    }
}

fn looks_like_date(s: &str) -> bool {
    let mut parts = s.split('.');
    let y = parts.next().unwrap_or("");
    let m = parts.next().unwrap_or("");
    let d = parts.next().unwrap_or("");
    !y.is_empty()
        && !m.is_empty()
        && !d.is_empty()
        && y.chars().all(|c| c.is_ascii_digit())
        && m.chars().all(|c| c.is_ascii_digit())
        && d.chars().all(|c| c.is_ascii_digit())
}

fn looks_like_time(s: &str) -> bool {
    let mut parts = s.split(':');
    let h = parts.next().unwrap_or("");
    let m = parts.next().unwrap_or("");
    let sec = parts.next().unwrap_or("");
    !h.is_empty()
        && !m.is_empty()
        && !sec.is_empty()
        && h.chars().all(|c| c.is_ascii_digit())
        && m.chars().all(|c| c.is_ascii_digit())
        && sec.chars().all(|c| c.is_ascii_digit())
}

pub fn normalize_timestamp(s: &str) -> Result<String, String> {
    // expected: YYYY.MM.DD H:MM:SS (UTC)
    if let Some((date, time)) = s.split_once(' ') {
        let date = date.replace('.', "-");
        let time = normalize_time(time)?;
        return Ok(format!("{}T{}Z", date, time));
    }
    if let Some((date, time)) = s.split_once('\t') {
        let date = date.replace('.', "-");
        let time = normalize_time(time)?;
        return Ok(format!("{}T{}Z", date, time));
    }
    Err("invalid timestamp format".to_string())
}

fn normalize_time(time: &str) -> Result<String, String> {
    let mut parts = time.trim().split(':');
    let h = parts.next().ok_or_else(|| "invalid time format".to_string())?;
    let m = parts.next().ok_or_else(|| "invalid time format".to_string())?;
    let s = parts.next().ok_or_else(|| "invalid time format".to_string())?;
    if !h.chars().all(|c| c.is_ascii_digit())
        || !m.chars().all(|c| c.is_ascii_digit())
        || !s.chars().all(|c| c.is_ascii_digit())
    {
        return Err("invalid time format".to_string());
    }
    let h = h.parse::<u32>().map_err(|_| "invalid time format".to_string())?;
    let m = m.parse::<u32>().map_err(|_| "invalid time format".to_string())?;
    let s = s.parse::<u32>().map_err(|_| "invalid time format".to_string())?;
    if h > 23 || m > 59 || s > 59 {
        return Err("invalid time format".to_string());
    }
    Ok(format!("{:02}:{:02}:{:02}", h, m, s))
}

fn parse_f64(s: &str) -> Result<f64, String> {
    let cleaned = s.trim().replace(',', "");
    cleaned
        .parse::<f64>()
        .map_err(|_| format!("invalid number: {}", s))
}

fn save_to_cache(path: &Path, dataset: &DataSet) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut conn = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS dataset_meta (source_path TEXT);\n         CREATE TABLE IF NOT EXISTS candles (ts_utc TEXT, open REAL, high REAL, low REAL, close REAL, volume REAL);",
    )
    .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM dataset_meta", []).map_err(|e| e.to_string())?;
    conn.execute("INSERT INTO dataset_meta (source_path) VALUES (?1)", [&dataset.source_path])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM candles", []).map_err(|e| e.to_string())?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    {
        let mut stmt = tx
            .prepare("INSERT INTO candles (ts_utc, open, high, low, close, volume) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
            .map_err(|e| e.to_string())?;
        for c in &dataset.candles {
            stmt.execute((
                &c.ts_utc,
                c.open,
                c.high,
                c.low,
                c.close,
                c.volume,
            ))
            .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

fn load_from_cache(path: &Path) -> Result<DataSet, String> {
    let conn = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
    let source_path: String = conn
        .query_row("SELECT source_path FROM dataset_meta LIMIT 1", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT ts_utc, open, high, low, close, volume FROM candles ORDER BY ROWID ASC")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Candle {
                ts_utc: row.get(0)?,
                open: row.get(1)?,
                high: row.get(2)?,
                low: row.get(3)?,
                close: row.get(4)?,
                volume: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut candles = Vec::new();
    for row in rows {
        candles.push(row.map_err(|e| e.to_string())?);
    }

    Ok(DataSet { source_path, candles })
}
