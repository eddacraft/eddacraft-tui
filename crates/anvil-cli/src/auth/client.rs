use std::fmt::Write as _;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::StatusCode;
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

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub body: String,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.body.is_empty() {
            write!(f, "API response: {}", self.status)
        } else {
            write!(f, "API response: {}: {}", self.status, self.body)
        }
    }
}

impl std::error::Error for ApiError {}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct WaitlistResponse {
    pub total: u32,
    pub items: Vec<WaitlistItem>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct WaitlistItem {
    pub email: String,
    pub name: Option<String>,
    pub source: String,
    pub created_at: String,
    pub approved_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct AuditResponse {
    pub total: u32,
    pub items: Vec<AuditItem>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct AuditItem {
    pub id: String,
    pub action: String,
    pub actor: String,
    pub metadata: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FleetOverviewResponse {
    pub schema_version: String,
    pub as_of: String,
    pub active_installs: FleetActiveInstalls,
    pub distributions: FleetDistributions,
    pub feature_adoption: Vec<FleetFeatureAdoption>,
    pub retention_cohorts: Vec<FleetRetentionCohort>,
    pub historical_aggregates: FleetHistoricalAggregates,
    pub notes: FleetNotes,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct FleetActiveInstalls {
    pub daily: u64,
    pub weekly: u64,
    pub monthly: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FleetDistributions {
    pub versions: Vec<FleetDistributionEntry>,
    pub install_methods: Vec<FleetDistributionEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct FleetDistributionEntry {
    pub value: String,
    pub installs: u64,
    pub share: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FleetFeatureAdoption {
    pub feature_key: String,
    pub installs: u64,
    pub share: f64,
    pub usage_count: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FleetRetentionCohort {
    pub cohort_start: String,
    pub cohort_size: u64,
    pub periods: Vec<FleetRetentionPeriod>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct FleetRetentionPeriod {
    pub week: u8,
    pub retained: u64,
    pub share: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FleetHistoricalAggregates {
    pub daily_install_dimensions: Vec<FleetHistoricalInstallDimension>,
    pub daily_feature_usage: Vec<FleetHistoricalFeatureUsage>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FleetHistoricalInstallDimension {
    pub day: String,
    pub version: String,
    pub install_method: String,
    pub platform: String,
    pub channel: String,
    pub distinct_installs: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FleetHistoricalFeatureUsage {
    pub day: String,
    pub feature_key: String,
    pub installs: u64,
    pub usage_count: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FleetNotes {
    pub activity_definition: String,
    pub raw_retention_days: u16,
    pub current_metrics_source: String,
    pub historical_metrics_source: String,
    pub data_quality: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShowUserResponse {
    pub user: ShowUser,
    pub tokens: Vec<ShowToken>,
    pub recent_audit: Vec<AuditItem>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub audit_error: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct ShowUser {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct ShowToken {
    pub id: String,
    pub scopes: Vec<String>,
    #[serde(default)]
    pub is_edict: bool,
    pub expires_at: String,
    pub revoked_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RevokeResponse {
    pub revoked: u32,
    /// SEC-007 / GH #1672: refresh sessions revoked alongside the access
    /// tokens. `None` when talking to a pre-fix server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_sessions_revoked: Option<u32>,
    /// SEC-007 / GH #1672: account-level revoke (by email) flipped the user
    /// from `active` to `suspended`. `None` on grant-level revoke (by token)
    /// or pre-fix servers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_suspended: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationPreviewResponse {
    pub dry_run: bool,
    pub source: String,
    pub count: u32,
    pub recipients: Vec<MigrationRecipient>,
    pub preview_token: String,
    pub expires_at: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct MigrationRecipient {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct MigrationSendResponse {
    pub source: String,
    pub total: u32,
    pub sent: u32,
    pub failed: u32,
    pub results: Vec<MigrationSendResult>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct MigrationSendResult {
    pub email: String,
    pub sent: bool,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmailUpdateResponse {
    pub user: EmailUpdateUser,
    pub previous_email: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct EmailUpdateUser {
    pub id: String,
    pub email: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct WhoamiResponse {
    pub email: String,
    pub plan: Option<String>,
    #[allow(dead_code)]
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VerifyResponse {
    valid: bool,
    #[serde(rename = "isEdict", default)]
    is_edict: bool,
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

    pub async fn verify_edict(&self) -> Result<()> {
        let token = self
            .token
            .as_deref()
            .context("Not authenticated. Run: anvil auth login")?;

        let verify: VerifyResponse = self.post("/auth/verify", VerifyBody { token }).await?;
        if !verify.valid {
            bail!("Stored credentials are invalid or expired")
        }
        if !verify.is_edict {
            bail!("Stored credentials are not edict credentials")
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}/api/v1{}", self.api_url, path);
        let mut req = self.http.get(&url);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let res = req.send().await.context("API request")?;
        if matches!(res.status().as_u16(), 401 | 403) {
            return Err(crate::output::AuthRequired.into());
        }
        parse_response(res).await
    }

    pub async fn get_with_query<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let mut url = format!("{}/api/v1{}", self.api_url, path);
        if !query.is_empty() {
            let pairs = query
                .iter()
                .map(|(key, value)| {
                    format!(
                        "{}={}",
                        encode_path_segment(key),
                        encode_path_segment(value)
                    )
                })
                .collect::<Vec<_>>()
                .join("&");
            url.push('?');
            url.push_str(&pairs);
        }
        let mut req = self.http.get(&url);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let res = req.send().await.context("API request")?;
        if matches!(res.status().as_u16(), 401 | 403) {
            return Err(crate::output::AuthRequired.into());
        }
        parse_response(res).await
    }

    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: impl Serialize) -> Result<T> {
        let url = format!("{}/api/v1{}", self.api_url, path);
        let mut req = self.http.post(&url).json(&body);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let res = req.send().await.context("API request")?;
        if matches!(res.status().as_u16(), 401 | 403) {
            return Err(crate::output::AuthRequired.into());
        }
        parse_response(res).await
    }

    pub async fn list_waitlist(
        &self,
        status: Option<&str>,
        source: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<WaitlistResponse> {
        let mut query = Vec::new();
        if let Some(status) = status {
            query.push(("status", status.to_string()));
        }
        if let Some(source) = source {
            query.push(("source", source.to_string()));
        }
        if let Some(limit) = limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(offset) = offset {
            query.push(("offset", offset.to_string()));
        }
        self.get_with_query("/admin/waitlist", &query).await
    }

    pub async fn get_user(&self, email: &str) -> Result<ShowUserResponse> {
        self.get(&format!("/admin/user/{}", encode_path_segment(email)))
            .await
    }

    pub async fn get_fleet_overview(&self) -> Result<FleetOverviewResponse> {
        self.get("/admin/fleet").await
    }

    pub async fn revoke_email(&self, email: &str) -> Result<RevokeResponse> {
        #[derive(Serialize)]
        struct Body<'a> {
            email: &'a str,
        }
        self.post("/admin/revoke", Body { email }).await
    }

    pub async fn revoke_token(&self, token: &str) -> Result<RevokeResponse> {
        #[derive(Serialize)]
        struct Body<'a> {
            token: &'a str,
        }
        self.post("/admin/revoke", Body { token }).await
    }

    pub async fn list_audit(
        &self,
        action: Option<&str>,
        actor: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<AuditResponse> {
        let mut query = Vec::new();
        if let Some(action) = action {
            query.push(("action", action.to_string()));
        }
        if let Some(actor) = actor {
            query.push(("actor", actor.to_string()));
        }
        if let Some(limit) = limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(offset) = offset {
            query.push(("offset", offset.to_string()));
        }
        self.get_with_query("/admin/audit", &query).await
    }

    pub async fn send_migration_dry_run(
        &self,
        source: &str,
        limit: u32,
    ) -> Result<MigrationPreviewResponse> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            source: &'a str,
            dry_run: bool,
            limit: u32,
        }
        self.post(
            "/admin/send-migration",
            Body {
                source,
                dry_run: true,
                limit,
            },
        )
        .await
    }

    pub async fn send_migration_commit(
        &self,
        source: &str,
        limit: u32,
        preview_token: &str,
    ) -> Result<MigrationSendResponse> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            source: &'a str,
            dry_run: bool,
            limit: u32,
            preview_token: &'a str,
        }
        self.post(
            "/admin/send-migration",
            Body {
                source,
                dry_run: false,
                limit,
                preview_token,
            },
        )
        .await
    }

    pub async fn update_user_email(
        &self,
        current_email: &str,
        new_email: &str,
    ) -> Result<EmailUpdateResponse> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            current_email: &'a str,
            new_email: &'a str,
        }
        self.post(
            "/admin/user/email-update",
            Body {
                current_email,
                new_email,
            },
        )
        .await
    }

    pub async fn whoami(&self) -> Result<WhoamiResponse> {
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

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
    }
    encoded
}

async fn parse_response<T: DeserializeOwned>(res: reqwest::Response) -> Result<T> {
    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(ApiError { status, body }.into());
    }
    res.json().await.context("parsing response")
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, header, method, path, query_param};
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

    #[tokio::test]
    async fn verify_edict_accepts_valid_edict_token() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/verify"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "valid": true,
                "isEdict": true
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("edict-token"));
        client.verify_edict().await.unwrap();
    }

    #[tokio::test]
    async fn verify_edict_rejects_valid_non_edict_token() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/verify"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "valid": true,
                "isEdict": false
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("regular-token"));
        let err = client.verify_edict().await.unwrap_err();
        assert!(err.to_string().contains("not edict"));
    }

    #[tokio::test]
    async fn verify_edict_treats_missing_marker_as_non_edict() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/verify"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "valid": true
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("legacy-token"));
        let err = client.verify_edict().await.unwrap_err();
        assert!(err.to_string().contains("not edict"));
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
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("bad-key"));
        let err = client.invite_user("x@y.com", None, None).await.unwrap_err();
        assert!(err.to_string().contains("API response"));
    }

    #[tokio::test]
    async fn list_waitlist_sends_filters_and_bearer_auth() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/admin/waitlist"))
            .and(query_param("status", "approved"))
            .and(query_param("source", "manual"))
            .and(query_param("limit", "25"))
            .and(query_param("offset", "10"))
            .and(header("authorization", "Bearer admin-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total": 1,
                "items": [{
                    "email": "a@example.com",
                    "name": null,
                    "source": "manual",
                    "created_at": "2026-01-01T00:00:00Z",
                    "approved_at": null
                }]
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("admin-key"));
        let result = client
            .list_waitlist(Some("approved"), Some("manual"), Some(25), Some(10))
            .await
            .unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].email, "a@example.com");
    }

    #[tokio::test]
    async fn get_user_url_encodes_email() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/admin/user/a%2Bb%40example.com"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user": {
                    "id": "u1",
                    "email": "a+b@example.com",
                    "name": null,
                    "status": "active",
                    "notes": null,
                    "created_at": "2026-01-01T00:00:00Z",
                    "updated_at": "2026-01-02T00:00:00Z"
                },
                "tokens": [],
                "recentAudit": []
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("admin-key"));
        let result = client.get_user("a+b@example.com").await.unwrap();
        assert_eq!(result.user.email, "a+b@example.com");
    }

    #[tokio::test]
    async fn get_fleet_overview_uses_admin_path_and_preserves_json_contract() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "schemaVersion": "anvil.fleet-overview.v1",
            "asOf": "2026-07-16",
            "activeInstalls": { "daily": 1, "weekly": 2, "monthly": 3 },
            "distributions": {
                "versions": [{ "value": "1.0.0", "installs": 3, "share": 1.0 }],
                "installMethods": [{ "value": "homebrew", "installs": 2, "share": 2.0 / 3.0 }]
            },
            "featureAdoption": [{
                "featureKey": "alpha",
                "installs": 1,
                "share": 1.0 / 3.0,
                "usageCount": 4
            }],
            "retentionCohorts": [{
                "cohortStart": "2026-06-01",
                "cohortSize": 2,
                "periods": [{ "week": 0, "retained": 2, "share": 1.0 }]
            }],
            "historicalAggregates": {
                "dailyInstallDimensions": [{
                    "day": "2026-01-01",
                    "version": "1.0.0",
                    "installMethod": "homebrew",
                    "platform": "aarch64-apple-darwin",
                    "channel": "stable",
                    "distinctInstalls": 3
                }],
                "dailyFeatureUsage": [{
                    "day": "2026-01-01",
                    "featureKey": "alpha",
                    "installs": 2,
                    "usageCount": 7
                }]
            },
            "notes": {
                "activityDefinition": "beacon observed",
                "rawRetentionDays": 90,
                "currentMetricsSource": "retained raw beacons",
                "historicalMetricsSource": "indefinite daily aggregates",
                "dataQuality": "anonymous, unverified beacons; directional evidence only, not audit-grade"
            }
        });

        Mock::given(method("GET"))
            .and(path("/api/v1/admin/fleet"))
            .and(header("authorization", "Bearer admin-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("admin-key"));
        let result = client.get_fleet_overview().await.unwrap();

        assert_eq!(result.schema_version, "anvil.fleet-overview.v1");
        assert_eq!(serde_json::to_value(result).unwrap(), body);
    }

    #[tokio::test]
    async fn get_fleet_overview_maps_forbidden_to_auth_required() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/admin/fleet"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("bad-key"));
        let err = client.get_fleet_overview().await.unwrap_err();
        assert!(err.is::<crate::output::AuthRequired>());
    }

    #[tokio::test]
    async fn revoke_token_posts_token_body() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/admin/revoke"))
            .and(body_json(serde_json::json!({"token": "raw-token"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "revoked": 1
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("admin-key"));
        let result = client.revoke_token("raw-token").await.unwrap();
        assert_eq!(result.revoked, 1);
        // Pre-fix server response: optional SEC-007 fields absent.
        assert_eq!(result.refresh_sessions_revoked, None);
        assert_eq!(result.account_suspended, None);
    }

    // SEC-007 / GH #1672: account-level revoke surfaces refresh-session
    // and account-suspension counters when the server provides them.
    #[tokio::test]
    async fn revoke_email_surfaces_refresh_and_suspended_counters() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/admin/revoke"))
            .and(body_json(serde_json::json!({"email": "alice@example.com"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "revoked": 2,
                "refreshSessionsRevoked": 3,
                "accountSuspended": true
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("admin-key"));
        let result = client.revoke_email("alice@example.com").await.unwrap();
        assert_eq!(result.revoked, 2);
        assert_eq!(result.refresh_sessions_revoked, Some(3));
        assert_eq!(result.account_suspended, Some(true));
    }

    #[tokio::test]
    async fn list_audit_maps_filter_actor_to_actor_query() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/admin/audit"))
            .and(query_param("action", "user.approved"))
            .and(query_param("actor", "ops@example.com"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total": 0,
                "items": []
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("admin-key"));
        let result = client
            .list_audit(Some("user.approved"), Some("ops@example.com"), None, None)
            .await
            .unwrap();
        assert_eq!(result.total, 0);
    }

    #[tokio::test]
    async fn send_migration_uses_preview_token_flow() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/admin/send-migration"))
            .and(body_json(serde_json::json!({
                "source": "import",
                "dryRun": true,
                "limit": 20
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "dryRun": true,
                "source": "import",
                "count": 1,
                "recipients": [{"email": "a@example.com", "name": null}],
                "previewToken": "snap-token",
                "expiresAt": "2026-01-01T00:10:00Z"
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/v1/admin/send-migration"))
            .and(body_json(serde_json::json!({
                "source": "import",
                "dryRun": false,
                "limit": 20,
                "previewToken": "snap-token"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "source": "import",
                "total": 1,
                "sent": 1,
                "failed": 0,
                "results": [{"email": "a@example.com", "sent": true}]
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("admin-key"));
        let preview = client.send_migration_dry_run("import", 20).await.unwrap();
        assert_eq!(preview.preview_token, "snap-token");
        let sent = client
            .send_migration_commit("import", 20, &preview.preview_token)
            .await
            .unwrap();
        assert_eq!(sent.sent, 1);
    }

    #[tokio::test]
    async fn update_user_email_posts_camel_case_body() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/admin/user/email-update"))
            .and(body_json(serde_json::json!({
                "currentEmail": "old@example.com",
                "newEmail": "new@example.com"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user": {"id": "u1", "email": "new@example.com", "status": "active"},
                "previousEmail": "old@example.com"
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("admin-key"));
        let result = client
            .update_user_email("old@example.com", "new@example.com")
            .await
            .unwrap();
        assert_eq!(result.previous_email, "old@example.com");
        assert_eq!(result.user.email, "new@example.com");
    }

    #[tokio::test]
    async fn admin_unauthorized_maps_to_auth_required() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/admin/waitlist"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = mock_client(&server.uri(), Some("bad-key"));
        let err = client
            .list_waitlist(None, None, None, None)
            .await
            .unwrap_err();
        assert!(err.is::<crate::output::AuthRequired>());
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
        assert!(err.is::<crate::output::AuthRequired>());
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
