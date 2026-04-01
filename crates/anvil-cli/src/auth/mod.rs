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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_url_when_env_unset() {
        temp_env::with_var_unset("ANVIL_API_URL", || {
            let url = api_url().unwrap();
            assert_eq!(url, "https://api.eddacraft.ai");
        });
    }

    #[test]
    fn custom_https_url() {
        temp_env::with_var("ANVIL_API_URL", Some("https://custom.example.com"), || {
            let url = api_url().unwrap();
            assert_eq!(url, "https://custom.example.com");
        });
    }

    #[test]
    fn strips_trailing_slash() {
        temp_env::with_var("ANVIL_API_URL", Some("https://api.example.com/"), || {
            let url = api_url().unwrap();
            assert!(!url.ends_with('/'));
        });
    }

    #[test]
    fn allows_localhost_http() {
        temp_env::with_var("ANVIL_API_URL", Some("http://localhost:3000"), || {
            let url = api_url().unwrap();
            assert_eq!(url, "http://localhost:3000");
        });
    }

    #[test]
    fn allows_127_0_0_1_http() {
        temp_env::with_var("ANVIL_API_URL", Some("http://127.0.0.1:8080"), || {
            let url = api_url().unwrap();
            assert_eq!(url, "http://127.0.0.1:8080");
        });
    }

    #[test]
    fn rejects_insecure_remote_http() {
        temp_env::with_var("ANVIL_API_URL", Some("http://evil.example.com"), || {
            let err = api_url().unwrap_err();
            assert!(err.to_string().contains("HTTPS"));
        });
    }

    #[test]
    fn rejects_invalid_url() {
        temp_env::with_var("ANVIL_API_URL", Some("not a url"), || {
            assert!(api_url().is_err());
        });
    }
}
