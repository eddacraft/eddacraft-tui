pub mod client;
pub mod credentials;
pub mod device_flow;

use anyhow::{Result, bail};
use reqwest::Url;

/// Resolve the API URL from `ANVIL_API_URL` or the default, rejecting
/// insecure (non-HTTPS) URLs unless they target localhost.
pub fn api_url() -> Result<String> {
    let raw = std::env::var("ANVIL_API_URL")
        .unwrap_or_else(|_| "https://api.eddacraft.ai".to_string())
        .trim_end_matches('/')
        .to_string();

    let parsed = Url::parse(&raw).map_err(|e| anyhow::anyhow!("invalid ANVIL_API_URL: {e}"))?;

    let is_https = parsed.scheme() == "https";
    let is_localhost = parsed.scheme() == "http"
        && matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));

    if !is_https && !is_localhost {
        bail!(
            "ANVIL_API_URL must use HTTPS (or target localhost for development). \
             Got: {raw}"
        );
    }

    Ok(raw)
}
