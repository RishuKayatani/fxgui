#[cfg(test)]
mod tests {
    use super::super::core::normalize_timestamp;

    #[test]
    fn normalize_timestamp_works() {
        let ts = normalize_timestamp("2003.05.05 0:01:00").unwrap();
        assert_eq!(ts, "2003-05-05T00:01:00Z");
    }

    #[test]
    fn normalize_timestamp_is_utc() {
        let ts = normalize_timestamp("2003.05.05 23:59:59").unwrap();
        assert!(ts.ends_with('Z'));
    }
}
