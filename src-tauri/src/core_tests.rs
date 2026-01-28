#[cfg(test)]
mod tests {
    use super::super::core::normalize_timestamp;

    #[test]
    fn normalize_timestamp_works() {
        let ts = normalize_timestamp("2003.05.05 0:01:00").unwrap();
        assert_eq!(ts, "2003-05-05T0:01:00Z");
    }
}
