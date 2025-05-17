use url::Url;

pub(crate) fn is_valid_url(url: &str) -> bool {
    Url::parse(url).is_ok()
}

pub(crate) fn is_valid_short_link(short: &str) -> bool {
    short.len() >= 4
        && short.len() <= 16
        && short.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use crate::validation::{is_valid_short_link, is_valid_url};

    #[test]
    fn test_url_validation() {
        assert!(is_valid_url("https://google.com"));
        assert!(is_valid_url("http://docs.google.com"));
        assert!(is_valid_url("https://github.com/login"));
        assert!(is_valid_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(is_valid_url("https://example.com/#section1"));
        assert!(is_valid_url("http://a.io"));
        assert!(is_valid_url("http://127.0.0.1"));

        assert!(!is_valid_url("google.com"));
        assert!(!is_valid_url("127.0.0.1/index.html"));
        assert!(!is_valid_url("http://"));
        assert!(!is_valid_url("https://examp le.com/#section1"));
    }

    #[test]
    fn test_short_link_validation() {
        assert!(is_valid_short_link("abcdefg"));
        assert!(is_valid_short_link("1231321"));
        assert!(is_valid_short_link("yandex_disk"));
        assert!(is_valid_short_link("youtube_video"));

        assert!(!is_valid_short_link("111"));
        assert!(!is_valid_short_link("have some spaces"));
        assert!(!is_valid_short_link("exam/ple"));
        assert!(!is_valid_short_link("very_very_very_very_long_link"));
    }
}
