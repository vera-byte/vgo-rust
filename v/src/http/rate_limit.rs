pub fn rate_limit_headers(limit: u32, remaining: u32, reset_epoch: u64) -> Vec<(String, String)> {
    vec![
        ("X-RateLimit-Limit".to_string(), limit.to_string()),
        ("X-RateLimit-Remaining".to_string(), remaining.to_string()),
        ("X-RateLimit-Reset".to_string(), reset_epoch.to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_headers() {
        let hs = rate_limit_headers(60, 59, 1700000000);
        assert_eq!(hs.len(), 3);
        assert!(hs.iter().any(|(k, _)| k == "X-RateLimit-Limit"));
        assert!(hs.iter().any(|(k, _)| k == "X-RateLimit-Remaining"));
        assert!(hs.iter().any(|(k, _)| k == "X-RateLimit-Reset"));
    }
}
