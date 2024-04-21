use const_format::concatcp;

pub const BASE_URLS: &[&str] = if cfg!(debug_assertions) {
    &["http://localhost:9000"]
} else {
    &["https://www.stream-stash.com", "https://stream-stash.com"]
};

pub const REDIRECT_URL: &str = concatcp!(BASE_URLS[0], "/loginRedirect");
