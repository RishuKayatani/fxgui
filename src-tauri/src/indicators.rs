use super::core::Candle;

pub fn ma(values: &[f64], period: usize) -> Vec<Option<f64>> {
    if period == 0 {
        return vec![None; values.len()];
    }
    let mut out = Vec::with_capacity(values.len());
    let mut sum = 0.0;
    for i in 0..values.len() {
        sum += values[i];
        if i + 1 > period {
            sum -= values[i - period];
        }
        if i + 1 >= period {
            out.push(Some(sum / period as f64));
        } else {
            out.push(None);
        }
    }
    out
}

pub fn ema(values: &[f64], period: usize) -> Vec<Option<f64>> {
    if period == 0 {
        return vec![None; values.len()];
    }
    let mut out = Vec::with_capacity(values.len());
    let k = 2.0 / (period as f64 + 1.0);
    let mut ema_prev = 0.0;
    for i in 0..values.len() {
        if i + 1 < period {
            out.push(None);
            ema_prev += values[i];
            continue;
        }
        if i + 1 == period {
            ema_prev = (ema_prev + values[i]) / period as f64;
            out.push(Some(ema_prev));
            continue;
        }
        ema_prev = values[i] * k + ema_prev * (1.0 - k);
        out.push(Some(ema_prev));
    }
    out
}

pub fn rsi(values: &[f64], period: usize) -> Vec<Option<f64>> {
    if period == 0 || values.len() < 2 {
        return vec![None; values.len()];
    }
    let mut out = vec![None; values.len()];
    let mut gain = 0.0;
    let mut loss = 0.0;

    for i in 1..=period.min(values.len() - 1) {
        let delta = values[i] - values[i - 1];
        if delta >= 0.0 {
            gain += delta;
        } else {
            loss -= delta;
        }
    }

    if values.len() > period {
        let mut avg_gain = gain / period as f64;
        let mut avg_loss = loss / period as f64;
        out[period] = Some(calc_rsi(avg_gain, avg_loss));

        for i in (period + 1)..values.len() {
            let delta = values[i] - values[i - 1];
            let (g, l) = if delta >= 0.0 { (delta, 0.0) } else { (0.0, -delta) };
            avg_gain = (avg_gain * (period as f64 - 1.0) + g) / period as f64;
            avg_loss = (avg_loss * (period as f64 - 1.0) + l) / period as f64;
            out[i] = Some(calc_rsi(avg_gain, avg_loss));
        }
    }

    out
}

fn calc_rsi(avg_gain: f64, avg_loss: f64) -> f64 {
    if avg_loss == 0.0 {
        return 100.0;
    }
    let rs = avg_gain / avg_loss;
    100.0 - (100.0 / (1.0 + rs))
}

pub fn macd(values: &[f64], fast: usize, slow: usize, signal: usize) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let ema_fast = ema(values, fast);
    let ema_slow = ema(values, slow);
    let mut macd_line = Vec::with_capacity(values.len());
    for i in 0..values.len() {
        match (ema_fast[i], ema_slow[i]) {
            (Some(f), Some(s)) => macd_line.push(Some(f - s)),
            _ => macd_line.push(None),
        }
    }

    let macd_vals: Vec<f64> = macd_line.iter().map(|v| v.unwrap_or(0.0)).collect();
    let signal_line = ema(&macd_vals, signal);

    let mut hist = Vec::with_capacity(values.len());
    for i in 0..values.len() {
        match (macd_line[i], signal_line[i]) {
            (Some(m), Some(s)) => hist.push(Some(m - s)),
            _ => hist.push(None),
        }
    }

    (macd_line, signal_line, hist)
}

pub fn closes_from_candles(candles: &[Candle]) -> Vec<f64> {
    candles.iter().map(|c| c.close).collect()
}
