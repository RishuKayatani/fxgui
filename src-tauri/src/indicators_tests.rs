#[cfg(test)]
mod tests {
    use super::super::indicators::{ma, rsi, macd};

    #[test]
    fn ma_basic() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let out = ma(&values, 3);
        assert_eq!(out[2], Some(2.0));
        assert_eq!(out[4], Some(4.0));
    }

    #[test]
    fn rsi_basic() {
        let values = vec![1.0, 2.0, 3.0, 2.0, 3.0, 4.0];
        let out = rsi(&values, 3);
        assert!(out.iter().any(|v| v.is_some()));
    }

    #[test]
    fn macd_basic() {
        let values: Vec<f64> = (1..50).map(|v| v as f64).collect();
        let (macd, signal, hist) = macd(&values, 12, 26, 9);
        assert_eq!(macd.len(), values.len());
        assert_eq!(signal.len(), values.len());
        assert_eq!(hist.len(), values.len());
    }
}
