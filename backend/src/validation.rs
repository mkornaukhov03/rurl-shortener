use url::Url;

pub(crate) fn is_valid_url(url: &str) -> bool {
    Url::parse(url).is_ok()
}

pub(crate) fn is_valid_short_link(short: &str) -> bool {
    short.len() >= 4 && short.len() <= 10 && short.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
}