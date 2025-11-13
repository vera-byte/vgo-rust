pub fn build_link_header(base_url: &str, page: u32, per_page: u32, total: Option<u64>) -> String {
    let mut links: Vec<String> = Vec::new();
    let next = page.saturating_add(1);
    let prev = page.saturating_sub(1);
    let first = 1u32;
    let last = total.map(|t| {
        if per_page == 0 { 1 } else { ((t as f64) / (per_page as f64)).ceil() as u32 }
    });

    links.push(format!("<{}?page={}&per_page={}>; rel=\"first\"", base_url, first, per_page));
    if let Some(l) = last {
        links.push(format!("<{}?page={}&per_page={}>; rel=\"last\"", base_url, l, per_page));
    }
    links.push(format!("<{}?page={}&per_page={}>; rel=\"next\"", base_url, next, per_page));
    if page > 1 {
        links.push(format!("<{}?page={}&per_page={}>; rel=\"prev\"", base_url, prev, per_page));
    }
    links.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_header_build() {
        let h = build_link_header("https://api.example/users", 2, 30, Some(95));
        assert!(h.contains("rel=\"next\""));
        assert!(h.contains("rel=\"prev\""));
        assert!(h.contains("rel=\"last\""));
        assert!(h.contains("page=3"));
        assert!(h.contains("page=1"));
    }
}
