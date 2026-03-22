pub mod client;
pub mod credentials;
pub mod device_flow;

use anyhow::{Result, bail};

/// Resolve the API URL from `ANVIL_API_URL` or the default, rejecting
/// insecure (non-HTTPS) URLs unless they target localhost.
pub fn api_url() -> Result<String> {
    let url = std::env::var("ANVIL_API_URL")
        .unwrap_or_else(|_| "https://api.eddacraft.ai".to_string())
        .trim_end_matches('/')
        .to_string();

    let is_https = url.starts_with("https://");
    let is_localhost = url.starts_with("http://localhost")
        || url.starts_with("http://127.0.0.1")
        || url.starts_with("http://[::1]");

    if !is_https && !is_localhost {
        bail!(
            "ANVIL_API_URL must use HTTPS (or target localhost for development). \
             Got: {url}"
        );
    }

    Ok(url)
}
