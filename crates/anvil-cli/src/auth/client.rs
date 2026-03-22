use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::credentials;

pub struct AnvilClient {
    http: reqwest::Client,
    api_url: String,
    token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WhoamiResponse {
    pub email: String,
    pub plan: Option<String>,
    pub created_at: Option<String>,
}

impl AnvilClient {
    pub fn new() -> Self {
        let api_url = std::env::var("ANVIL_API_URL")
            .unwrap_or_else(|_| "https://api.eddacraft.ai".to_string())
            .trim_end_matches('/')
            .to_string();
        Self {
            http: reqwest::Client::new(),
            api_url,
            token: None,
        }
    }

    pub fn authenticated() -> Result<Self> {
        let mut client = Self::new();
        let creds = credentials::load()?.context("Not authenticated. Run: anvil auth login")?;
        client.token = Some(creds.license);
        Ok(client)
    }

    pub fn with_token(token: String) -> Self {
        let mut client = Self::new();
        client.token = Some(token);
        client
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}/api/v1{}", self.api_url, path);
        let mut req = self.http.get(&url);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let res = req.send().await.context("API request")?;
        let res = res.error_for_status().context("API response")?;
        res.json().await.context("parsing response")
    }

    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: impl Serialize) -> Result<T> {
        let url = format!("{}/api/v1{}", self.api_url, path);
        let mut req = self.http.post(&url).json(&body);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let res = req.send().await.context("API request")?;
        let res = res.error_for_status().context("API response")?;
        res.json().await.context("parsing response")
    }

    pub async fn whoami(&self) -> Result<WhoamiResponse> {
        #[derive(Debug, Deserialize)]
        struct VerifyResponse {
            valid: bool,
            user: Option<WhoamiResponseUser>,
        }

        #[derive(Debug, Deserialize)]
        struct WhoamiResponseUser {
            email: String,
        }

        #[derive(Debug, Serialize)]
        struct VerifyBody<'a> {
            token: &'a str,
        }

        let token = self
            .token
            .as_deref()
            .context("Not authenticated. Run: anvil auth login")?;

        let verify: VerifyResponse = self.post("/auth/verify", VerifyBody { token }).await?;
        if !verify.valid {
            bail!("Stored credentials are invalid or expired")
        }

        Ok(WhoamiResponse {
            email: verify
                .user
                .map_or_else(|| "unknown".to_string(), |u| u.email),
            plan: Some("beta".to_string()),
            created_at: None,
        })
    }

    pub async fn approve_user(&self, email: &str) -> Result<()> {
        #[derive(Serialize)]
        struct ApproveBody<'a> {
            email: &'a str,
        }
        #[derive(Deserialize)]
        struct ApproveResponse {
            approved: Vec<ApproveEntry>,
        }
        #[derive(Deserialize)]
        struct ApproveEntry {
            email: String,
        }

        let result: ApproveResponse = self.post("/admin/approve", ApproveBody { email }).await?;
        if result.approved.is_empty() {
            bail!("No users approved — email may not be on the waitlist");
        }
        Ok(())
    }

    pub async fn approve_batch(&self, count: u32) -> Result<Vec<String>> {
        #[derive(Serialize)]
        struct BatchBody {
            batch: u32,
        }
        #[derive(Deserialize)]
        struct BatchResponse {
            approved: Vec<ApproveEntry>,
        }
        #[derive(Deserialize)]
        struct ApproveEntry {
            email: String,
        }

        let result: BatchResponse = self.post("/admin/approve", BatchBody { batch: count }).await?;
        Ok(result.approved.into_iter().map(|e| e.email).collect())
    }
}
