use super::core::{Candle, DataSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interval {
    M1,
    M5,
    M15,
    M30,
    H1,
    H4,
    D1,
}

impl Interval {
    pub fn seconds(self) -> i64 {
        match self {
            Interval::M1 => 60,
            Interval::M5 => 300,
            Interval::M15 => 900,
            Interval::M30 => 1800,
            Interval::H1 => 3600,
            Interval::H4 => 14400,
            Interval::D1 => 86400,
        }
    }
}

#[allow(dead_code)]
pub fn infer_interval(candles: &[Candle]) -> Option<Interval> {
    if candles.len() < 2 {
        return None;
    }
    let t0 = parse_ts(&candles[0].ts_utc).ok()?;
    let t1 = parse_ts(&candles[1].ts_utc).ok()?;
    let diff = (t1 - t0).abs();
    match diff {
        60 => Some(Interval::M1),
        300 => Some(Interval::M5),
        900 => Some(Interval::M15),
        1800 => Some(Interval::M30),
        3600 => Some(Interval::H1),
        14400 => Some(Interval::H4),
        86400 => Some(Interval::D1),
        _ => None,
    }
}

pub fn resample(dataset: &DataSet, target: Interval) -> Result<DataSet, String> {
    let mut out = Vec::new();
    if dataset.candles.is_empty() {
        return Ok(DataSet {
            source_path: dataset.source_path.clone(),
            candles: out,
        });
    }
    let mut bucket_start = parse_ts(&dataset.candles[0].ts_utc)?;
    let bucket = target.seconds();
    bucket_start = bucket_start - (bucket_start % bucket);

    let mut current: Option<Candle> = None;

    for c in &dataset.candles {
        let ts = parse_ts(&c.ts_utc)?;
        let bucket_time = ts - (ts % bucket);
        if bucket_time != bucket_start {
            if let Some(acc) = current.take() {
                out.push(acc);
            }
            bucket_start = bucket_time;
        }
        current = Some(merge_candle(current, c, bucket_start));
    }

    if let Some(acc) = current.take() {
        out.push(acc);
    }

    Ok(DataSet {
        source_path: dataset.source_path.clone(),
        candles: out,
    })
}

fn merge_candle(current: Option<Candle>, incoming: &Candle, bucket_start: i64) -> Candle {
    match current {
        None => Candle {
            ts_utc: format_ts(bucket_start),
            open: incoming.open,
            high: incoming.high,
            low: incoming.low,
            close: incoming.close,
            volume: incoming.volume,
        },
        Some(mut acc) => {
            acc.high = acc.high.max(incoming.high);
            acc.low = acc.low.min(incoming.low);
            acc.close = incoming.close;
            acc.volume += incoming.volume;
            acc
        }
    }
}

fn parse_ts(ts: &str) -> Result<i64, String> {
    // expects: YYYY-MM-DDTHH:MM:SSZ
    let ts = ts.trim_end_matches('Z');
    let parts: Vec<&str> = ts.split('T').collect();
    if parts.len() != 2 {
        return Err("invalid timestamp".to_string());
    }
    let date = parts[0];
    let time = parts[1];
    let d: Vec<i64> = date
        .split('-')
        .map(|p| p.parse::<i64>().map_err(|_| "invalid date".to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let t: Vec<i64> = time
        .split(':')
        .map(|p| p.parse::<i64>().map_err(|_| "invalid time".to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    if d.len() != 3 || t.len() < 2 {
        return Err("invalid timestamp".to_string());
    }
    let (year, month, day) = (d[0], d[1], d[2]);
    let (hour, min, sec) = (t[0], t[1], *t.get(2).unwrap_or(&0));
    Ok(to_epoch(year, month, day, hour, min, sec))
}

fn format_ts(epoch: i64) -> String {
    let (year, month, day, hour, min, sec) = from_epoch(epoch);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, hour, min, sec)
}

fn to_epoch(year: i64, month: i64, day: i64, hour: i64, min: i64, sec: i64) -> i64 {
    // naive conversion, good enough for UTC data without leap seconds
    let mut y = year;
    let mut m = month;
    let d = day;
    if m <= 2 {
        y -= 1;
        m += 12;
    }
    let era = y / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m - 3) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468; // days since 1970-01-01
    days * 86400 + hour * 3600 + min * 60 + sec
}

fn from_epoch(epoch: i64) -> (i64, i64, i64, i64, i64, i64) {
    let days = epoch.div_euclid(86400);
    let secs = epoch.rem_euclid(86400);
    let z = days + 719468;
    let era = (z).div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096).div_euclid(365);
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2).div_euclid(153);
    let d = doy - (153 * mp + 2).div_euclid(5) + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    let hour = secs / 3600;
    let min = (secs % 3600) / 60;
    let sec = secs % 60;
    (year, m, d, hour, min, sec)
}
