use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::credentials;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct AnvilClient {
    http: reqwest::Client,
    api_url: String,
    token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WhoamiResponse {
    pub email: String,
    pub plan: Option<String>,
    #[allow(dead_code)]
    pub created_at: Option<String>,
}

impl AnvilClient {
    pub fn new() -> Result<Self> {
        let api_url = super::api_url()?;
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .context("building HTTP client")?;
        Ok(Self {
            http,
            api_url,
            token: None,
        })
    }

    #[allow(dead_code)]
    pub fn authenticated() -> Result<Self> {
        let mut client = Self::new()?;
        let creds = credentials::load()?.context("Not authenticated. Run: anvil auth login")?;
        client.token = Some(creds.license);
        Ok(client)
    }

    pub fn with_token(token: String) -> Result<Self> {
        let mut client = Self::new()?;
        client.token = Some(token);
        Ok(client)
    }

    #[allow(dead_code)]
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
            plan: Option<String>,
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

        let (email, plan) = match verify.user {
            Some(u) => (u.email, u.plan),
            None => ("unknown".to_string(), None),
        };

        Ok(WhoamiResponse {
            email,
            plan,
            created_at: None,
        })
    }

    pub async fn invite_user(
        &self,
        email: &str,
        name: Option<&str>,
        notes: Option<&str>,
    ) -> Result<()> {
        #[derive(Serialize)]
        struct InviteBody<'a> {
            email: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            name: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            notes: Option<&'a str>,
        }
        #[derive(Deserialize)]
        struct InviteResponse {
            user: InviteUser,
        }
        #[derive(Deserialize)]
        struct InviteUser {
            email: String,
        }

        let resp: InviteResponse = self
            .post("/admin/invite", InviteBody { email, name, notes })
            .await?;
        let _ = resp.user.email;
        Ok(())
    }

    pub async fn invite_user_token(
        &self,
        email: &str,
        name: Option<&str>,
        notes: Option<&str>,
        edict: bool,
    ) -> Result<String> {
        #[derive(Serialize)]
        struct InviteBody<'a> {
            email: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            name: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            notes: Option<&'a str>,
            #[serde(rename = "tokenOnly")]
            token_only: bool,
            edict: bool,
        }
        #[derive(Deserialize)]
        struct TokenResponse {
            token: String,
        }

        let resp: TokenResponse = self
            .post(
                "/admin/invite",
                InviteBody {
                    email,
                    name,
                    notes,
                    token_only: true,
                    edict,
                },
            )
            .await?;
        Ok(resp.token)
    }

    pub async fn approve_user(&self, email: &str) -> Result<()> {
        #[derive(Serialize)]
        struct ApproveBody<'a> {
            email: &'a str,
        }
        #[derive(Deserialize)]
        struct ApproveResponse {
            approved: Vec<serde_json::Value>,
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

        let result: BatchResponse = self
            .post("/admin/approve", BatchBody { batch: count })
            .await?;
        Ok(result.approved.into_iter().map(|e| e.email).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Create a client pointing at the given mock server URL.
    fn mock_client(base_url: &str, token: Option<&str>) -> AnvilClient {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        AnvilClient {
            http,
            api_url: base_url.to_string(),
            token: token.map(String::from),
        }
    }

    // --- whoami ---

    #[tokio::test]
    async fn whoami_valid_with_user() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/verify"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "valid": true,
                "user": {
                    "email": "dev@example.com",
                    "plan": "pro"
                }
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("test-token"));
        let result = client.whoami().await.unwrap();
        assert_eq!(result.email, "dev@example.com");
        assert_eq!(result.plan.as_deref(), Some("pro"));
    }

    #[tokio::test]
    async fn whoami_valid_without_user() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/verify"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "valid": true,
                "user": null
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("test-token"));
        let result = client.whoami().await.unwrap();
        assert_eq!(result.email, "unknown");
        assert!(result.plan.is_none());
    }

    #[tokio::test]
    async fn whoami_invalid_credentials() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/verify"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "valid": false
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("bad-token"));
        let err = client.whoami().await.unwrap_err();
        assert!(err.to_string().contains("invalid or expired"));
    }

    #[tokio::test]
    async fn whoami_user_missing_email_field() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/verify"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "valid": true,
                "user": {}
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("test-token"));
        let err = client.whoami().await.unwrap_err();
        assert!(err.to_string().contains("parsing response"));
    }

    #[tokio::test]
    async fn whoami_without_token_errors() {
        let client = mock_client("http://localhost:1", None);
        let err = client.whoami().await.unwrap_err();
        assert!(err.to_string().contains("Not authenticated"));
    }

    #[tokio::test]
    async fn whoami_sends_bearer_auth() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/verify"))
            .and(header("authorization", "Bearer my-secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "valid": true,
                "user": { "email": "a@b.com" }
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("my-secret"));
        let result = client.whoami().await.unwrap();
        assert_eq!(result.email, "a@b.com");
    }

    // --- approve_user ---

    #[tokio::test]
    async fn approve_user_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/admin/approve"))
            .and(body_json(serde_json::json!({"email": "user@example.com"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "approved": [{"email": "user@example.com"}]
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("admin-key"));
        client.approve_user("user@example.com").await.unwrap();
    }

    #[tokio::test]
    async fn approve_user_empty_result_errors() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/admin/approve"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "approved": []
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("admin-key"));
        let err = client.approve_user("nobody@example.com").await.unwrap_err();
        assert!(err.to_string().contains("No users approved"));
    }

    // --- approve_batch ---

    #[tokio::test]
    async fn approve_batch_returns_emails() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/admin/approve"))
            .and(body_json(serde_json::json!({"batch": 2})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "approved": [
                    {"email": "a@example.com"},
                    {"email": "b@example.com"}
                ]
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("admin-key"));
        let emails = client.approve_batch(2).await.unwrap();
        assert_eq!(emails, vec!["a@example.com", "b@example.com"]);
    }

    #[tokio::test]
    async fn approve_batch_empty_returns_empty_vec() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/admin/approve"))
            .and(body_json(serde_json::json!({"batch": 5})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "approved": []
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("admin-key"));
        let emails = client.approve_batch(5).await.unwrap();
        assert!(emails.is_empty());
    }

    // --- invite_user ---

    #[tokio::test]
    async fn invite_user_default_flow() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/admin/invite"))
            .and(body_json(serde_json::json!({
                "email": "new@example.com"
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "user": {"email": "new@example.com", "id": "uuid-1"},
                "expiresAt": "2026-07-13T00:00:00.000Z",
                "scopes": ["beta"]
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("admin-key"));
        client
            .invite_user("new@example.com", None, None)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn invite_user_with_name_and_notes() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/admin/invite"))
            .and(body_json(serde_json::json!({
                "email": "vip@example.com",
                "name": "VIP User",
                "notes": "Priority access"
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "user": {"email": "vip@example.com", "id": "uuid-2"},
                "expiresAt": "2026-07-13T00:00:00.000Z",
                "scopes": ["beta"]
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("admin-key"));
        client
            .invite_user("vip@example.com", Some("VIP User"), Some("Priority access"))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn invite_user_token_returns_token() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/admin/invite"))
            .and(body_json(serde_json::json!({
                "email": "ci@example.com",
                "tokenOnly": true,
                "edict": false
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "token": "anvil_beta_abc123",
                "user": {"email": "ci@example.com", "id": "uuid-3"},
                "expiresAt": "2026-07-13T00:00:00.000Z",
                "scopes": ["beta"]
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("admin-key"));
        let token = client
            .invite_user_token("ci@example.com", None, None, false)
            .await
            .unwrap();
        assert_eq!(token, "anvil_beta_abc123");
    }

    #[tokio::test]
    async fn invite_user_propagates_http_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/admin/invite"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("bad-key"));
        let err = client.invite_user("x@y.com", None, None).await.unwrap_err();
        assert!(err.to_string().contains("API response"));
    }

    // --- HTTP error handling ---

    #[tokio::test]
    async fn post_propagates_http_errors() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/verify"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("tok"));
        let err = client.whoami().await.unwrap_err();
        assert!(err.to_string().contains("API response"));
    }

    #[tokio::test]
    async fn post_propagates_401() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/admin/approve"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("bad-key"));
        let err = client.approve_user("x@y.com").await.unwrap_err();
        assert!(err.to_string().contains("API response"));
    }

    // --- URL construction ---

    #[tokio::test]
    async fn url_construction_includes_api_prefix() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/verify"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "valid": true,
                "user": { "email": "test@test.com" }
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("tok"));
        client.whoami().await.unwrap();
    }

    // --- WhoamiResponse ---

    #[test]
    fn whoami_response_deserialises() {
        let json = r#"{"email": "a@b.com", "plan": "free", "created_at": "2024-01-01"}"#;
        let resp: WhoamiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.email, "a@b.com");
        assert_eq!(resp.plan.as_deref(), Some("free"));
    }

    #[test]
    fn whoami_response_optional_fields() {
        let json = r#"{"email": "a@b.com"}"#;
        let resp: WhoamiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.email, "a@b.com");
        assert!(resp.plan.is_none());
        assert!(resp.created_at.is_none());
    }
}
