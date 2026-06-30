use std::fmt;

/// Error from a Plane `/api/v1` call. Never carries the API token.
#[derive(Debug)]
pub enum ApiError {
    /// Missing or invalid client configuration (base URL or API key).
    Config(String),
    /// Non-2xx HTTP response from the Plane API.
    Http {
        status: u16,
        url: String,
        body: String,
    },
    /// Transport-level failure (connection, DNS, TLS, ...).
    Transport { url: String, source: String },
    /// Response body was not the expected JSON.
    Decode {
        url: String,
        source: String,
        body: String,
    },
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Config(message) => write!(f, "{message}"),
            ApiError::Http { status, url, body } => write!(
                f,
                "Plane API {url} returned HTTP {status}: {}",
                summarize(body)
            ),
            ApiError::Transport { url, source } => write!(f, "failed to reach {url}: {source}"),
            ApiError::Decode { url, source, body } => write!(
                f,
                "Plane API {url} returned invalid JSON: {source}; body: {}",
                summarize(body)
            ),
        }
    }
}

impl std::error::Error for ApiError {}

fn summarize(body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        return "<empty body>".to_string();
    }
    const LIMIT: usize = 500;
    if body.chars().count() <= LIMIT {
        body.to_string()
    } else {
        format!("{}...", body.chars().take(LIMIT).collect::<String>())
    }
}
