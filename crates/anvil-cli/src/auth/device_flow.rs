use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::credentials::{self, Credentials};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceStartResponse {
    poll_token: String,
    user_code: String,
    verification_url: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DevicePollResponse {
    status: String,
    license: Option<String>,
    refresh_token: Option<String>,
    expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtpRequestResponse {
    #[allow(dead_code)]
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtpVerifyResponse {
    license: String,
    refresh_token: String,
    expires_at: String,
}

#[derive(Debug, Serialize)]
struct DeviceStartRequest<'a> {
    email: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DevicePollRequest<'a> {
    poll_token: &'a str,
}

#[derive(Debug, Serialize)]
struct OtpSendRequest<'a> {
    email: &'a str,
}

#[derive(Debug, Serialize)]
struct OtpVerifyRequest<'a> {
    email: &'a str,
    code: &'a str,
}

fn api_url() -> anyhow::Result<String> {
    super::api_url()
}

fn build_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .context("building HTTP client")
}

fn prompt_input(label: &str) -> Result<String> {
    use std::io::Write;
    eprint!("{label}");
    std::io::stderr().flush().context("flushing stderr")?;
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .context("reading stdin")?;
    Ok(input.trim().to_string())
}

// ── HTTP helpers (extracted for testability) ──────────────────────────

async fn device_start(
    client: &reqwest::Client,
    url: &str,
    email: &str,
) -> Result<DeviceStartResponse> {
    client
        .post(format!("{url}/api/v1/auth/device/start"))
        .json(&DeviceStartRequest { email })
        .send()
        .await
        .context("device code start request")?
        .error_for_status()
        .context("device code start response")?
        .json()
        .await
        .context("parsing device code start response")
}

async fn device_poll(
    client: &reqwest::Client,
    url: &str,
    poll_token: &str,
) -> Result<DevicePollResponse> {
    client
        .post(format!("{url}/api/v1/auth/device/poll"))
        .json(&DevicePollRequest { poll_token })
        .send()
        .await
        .context("device poll request")?
        .error_for_status()
        .context("device poll response")?
        .json()
        .await
        .context("parsing device poll response")
}

async fn otp_request(
    client: &reqwest::Client,
    url: &str,
    email: &str,
) -> Result<OtpRequestResponse> {
    client
        .post(format!("{url}/api/v1/auth/otp/request"))
        .json(&OtpSendRequest { email })
        .send()
        .await
        .context("OTP send request")?
        .error_for_status()
        .context("OTP send response")?
        .json()
        .await
        .context("parsing OTP send response")
}

async fn otp_verify(
    client: &reqwest::Client,
    url: &str,
    email: &str,
    code: &str,
) -> Result<OtpVerifyResponse> {
    client
        .post(format!("{url}/api/v1/auth/otp/verify"))
        .json(&OtpVerifyRequest { email, code })
        .send()
        .await
        .context("OTP verify request")?
        .error_for_status()
        .context("OTP verify response")?
        .json()
        .await
        .context("parsing OTP verify response")
}

// ── Public entry points (thin wrappers adding I/O) ────────────────────

pub async fn login_device_flow() -> Result<()> {
    let url = api_url()?;
    let email = prompt_input("Email: ")?;
    if email.is_empty() {
        bail!("Email is required");
    }

    eprintln!("Starting device code flow...");

    let client = build_client()?;
    let start = device_start(&client, &url, &email).await?;

    eprintln!();
    eprintln!("To authenticate, open this URL:");
    eprintln!("  {}", start.verification_url);
    eprintln!();
    eprintln!("And enter code: {}", start.user_code);
    eprintln!();
    eprintln!("Waiting for confirmation...");

    let poll_interval = std::time::Duration::from_secs(5);
    let max_attempts = (start.expires_in / poll_interval.as_secs()).max(1);

    for _ in 0..max_attempts {
        tokio::time::sleep(poll_interval).await;

        let poll = device_poll(&client, &url, &start.poll_token).await?;

        match poll.status.as_str() {
            "confirmed" => {
                let license = poll.license.context("server returned no licence")?;
                let refresh = poll
                    .refresh_token
                    .context("server returned no refresh token")?;
                let expires = poll.expires_at.context("server returned no expiry")?;

                credentials::save(&Credentials {
                    license,
                    refresh_token: Some(refresh),
                    email: Some(email.clone()),
                    expires_at: Some(expires),
                })?;

                eprintln!();
                eprintln!("✓ Authenticated as {email}");
                let path = credentials::credentials_path()?;
                eprintln!("  Credentials saved to {}", path.display());
                return Ok(());
            }
            "expired" => bail!("Device code has expired. Please try again."),
            _ => eprint!("."),
        }
    }

    bail!("Timed out waiting for confirmation. Please try again.");
}

pub async fn login_otp_flow() -> Result<()> {
    let url = api_url()?;
    let email = prompt_input("Email: ")?;
    if email.is_empty() {
        bail!("Email is required");
    }

    eprintln!("Requesting verification code...");

    let client = build_client()?;
    let _ = otp_request(&client, &url, &email).await?;

    eprintln!();
    eprintln!("A verification code has been sent to your email.");

    for attempt in 1..=3 {
        let code = prompt_input("Enter code: ")?;
        if code.is_empty() {
            eprintln!("Code is required");
            if attempt < 3 {
                eprintln!("{} attempt(s) remaining", 3 - attempt);
            }
            continue;
        }

        match otp_verify(&client, &url, &email, &code).await {
            Ok(result) => {
                credentials::save(&Credentials {
                    license: result.license,
                    refresh_token: Some(result.refresh_token),
                    email: Some(email.clone()),
                    expires_at: Some(result.expires_at),
                })?;

                eprintln!();
                eprintln!("✓ Authenticated as {email}");
                let path = credentials::credentials_path()?;
                eprintln!("  Credentials saved to {}", path.display());
                return Ok(());
            }
            Err(e) => {
                eprintln!("Verification failed: {e}");
                if attempt < 3 {
                    eprintln!("{} attempt(s) remaining", 3 - attempt);
                }
            }
        }
    }

    bail!("Maximum attempts reached. Please try again.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // ── Serde: response deserialisation ───────────────────────────────

    #[test]
    fn deserialise_device_start_response() {
        let json = r#"{
            "pollToken": "tok-abc",
            "userCode": "ABCD-1234",
            "verificationUrl": "https://example.com/activate",
            "expiresIn": 300
        }"#;
        let resp: DeviceStartResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.poll_token, "tok-abc");
        assert_eq!(resp.user_code, "ABCD-1234");
        assert_eq!(resp.verification_url, "https://example.com/activate");
        assert_eq!(resp.expires_in, 300);
    }

    #[test]
    fn deserialise_device_poll_confirmed() {
        let json = r#"{
            "status": "confirmed",
            "license": "lic-123",
            "refreshToken": "ref-456",
            "expiresAt": "2099-12-31T23:59:59Z"
        }"#;
        let resp: DevicePollResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, "confirmed");
        assert_eq!(resp.license.as_deref(), Some("lic-123"));
        assert_eq!(resp.refresh_token.as_deref(), Some("ref-456"));
        assert_eq!(resp.expires_at.as_deref(), Some("2099-12-31T23:59:59Z"));
    }

    #[test]
    fn deserialise_device_poll_pending() {
        let json = r#"{"status": "pending"}"#;
        let resp: DevicePollResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, "pending");
        assert!(resp.license.is_none());
        assert!(resp.refresh_token.is_none());
        assert!(resp.expires_at.is_none());
    }

    #[test]
    fn deserialise_device_poll_expired() {
        let json = r#"{"status": "expired"}"#;
        let resp: DevicePollResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, "expired");
    }

    #[test]
    fn deserialise_otp_request_response_with_message() {
        let json = r#"{"message": "Code sent"}"#;
        let resp: OtpRequestResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.message.as_deref(), Some("Code sent"));
    }

    #[test]
    fn deserialise_otp_request_response_without_message() {
        let json = r"{}";
        let resp: OtpRequestResponse = serde_json::from_str(json).unwrap();
        assert!(resp.message.is_none());
    }

    #[test]
    fn deserialise_otp_verify_response() {
        let json = r#"{
            "license": "lic-otp",
            "refreshToken": "ref-otp",
            "expiresAt": "2099-06-15T12:00:00Z"
        }"#;
        let resp: OtpVerifyResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.license, "lic-otp");
        assert_eq!(resp.refresh_token, "ref-otp");
        assert_eq!(resp.expires_at, "2099-06-15T12:00:00Z");
    }

    // ── Serde: request serialisation ──────────────────────────────────

    #[test]
    fn serialise_device_start_request() {
        let req = DeviceStartRequest {
            email: "user@example.com",
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["email"], "user@example.com");
    }

    #[test]
    fn serialise_device_poll_request_uses_camel_case() {
        let req = DevicePollRequest {
            poll_token: "tok-abc",
        };
        let json = serde_json::to_value(&req).unwrap();
        let obj = json.as_object().expect("expected JSON object");
        assert!(obj.contains_key("pollToken"), "should use camelCase");
        assert!(!obj.contains_key("poll_token"), "should not use snake_case");
    }

    #[test]
    fn serialise_otp_send_request() {
        let req = OtpSendRequest {
            email: "user@example.com",
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["email"], "user@example.com");
    }

    #[test]
    fn serialise_otp_verify_request() {
        let req = OtpVerifyRequest {
            email: "user@example.com",
            code: "123456",
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["email"], "user@example.com");
        assert_eq!(json["code"], "123456");
    }

    // ── build_client smoke test ───────────────────────────────────────

    #[test]
    fn build_client_succeeds() {
        assert!(build_client().is_ok());
    }

    // ── Wiremock: device_start ────────────────────────────────────────

    #[tokio::test]
    async fn device_start_sends_correct_request() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/device/start"))
            .and(body_json(serde_json::json!({"email": "dev@example.com"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "pollToken": "poll-xyz",
                "userCode": "TEST-9999",
                "verificationUrl": "https://example.com/verify",
                "expiresIn": 600
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let resp = device_start(&client, &server.uri(), "dev@example.com")
            .await
            .unwrap();

        assert_eq!(resp.poll_token, "poll-xyz");
        assert_eq!(resp.user_code, "TEST-9999");
        assert_eq!(resp.verification_url, "https://example.com/verify");
        assert_eq!(resp.expires_in, 600);
    }

    #[tokio::test]
    async fn device_start_propagates_server_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/device/start"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = device_start(&client, &server.uri(), "dev@example.com")
            .await
            .unwrap_err();

        assert!(
            err.to_string().contains("device code start response"),
            "expected context about start response, got: {err}"
        );
    }

    // ── Wiremock: device_poll ─────────────────────────────────────────

    #[tokio::test]
    async fn device_poll_confirmed() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/device/poll"))
            .and(body_json(serde_json::json!({"pollToken": "tok-1"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "confirmed",
                "license": "lic-confirmed",
                "refreshToken": "rt-confirmed",
                "expiresAt": "2099-01-01T00:00:00Z"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let resp = device_poll(&client, &server.uri(), "tok-1").await.unwrap();

        assert_eq!(resp.status, "confirmed");
        assert_eq!(resp.license.as_deref(), Some("lic-confirmed"));
        assert_eq!(resp.refresh_token.as_deref(), Some("rt-confirmed"));
    }

    #[tokio::test]
    async fn device_poll_pending() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/device/poll"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "pending"})),
            )
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let resp = device_poll(&client, &server.uri(), "tok-1").await.unwrap();

        assert_eq!(resp.status, "pending");
        assert!(resp.license.is_none());
    }

    #[tokio::test]
    async fn device_poll_expired_status() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/device/poll"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "expired"})),
            )
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let resp = device_poll(&client, &server.uri(), "tok-1").await.unwrap();

        assert_eq!(resp.status, "expired");
    }

    #[tokio::test]
    async fn device_poll_propagates_server_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/device/poll"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = device_poll(&client, &server.uri(), "tok-bad")
            .await
            .unwrap_err();

        assert!(
            err.to_string().contains("device poll response"),
            "expected context about poll response, got: {err}"
        );
    }

    // ── Wiremock: otp_request ─────────────────────────────────────────

    #[tokio::test]
    async fn otp_request_sends_correct_body() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/otp/request"))
            .and(body_json(serde_json::json!({"email": "user@test.com"})))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"message": "Code sent"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let resp = otp_request(&client, &server.uri(), "user@test.com")
            .await
            .unwrap();

        assert_eq!(resp.message.as_deref(), Some("Code sent"));
    }

    #[tokio::test]
    async fn otp_request_propagates_server_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/otp/request"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = otp_request(&client, &server.uri(), "user@test.com")
            .await
            .unwrap_err();

        assert!(
            err.to_string().contains("OTP send response"),
            "expected context about send response, got: {err}"
        );
    }

    // ── Wiremock: otp_verify ──────────────────────────────────────────

    #[tokio::test]
    async fn otp_verify_sends_correct_body() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/otp/verify"))
            .and(body_json(
                serde_json::json!({"email": "user@test.com", "code": "123456"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "license": "lic-otp-ok",
                "refreshToken": "rt-otp-ok",
                "expiresAt": "2099-06-01T00:00:00Z"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let resp = otp_verify(&client, &server.uri(), "user@test.com", "123456")
            .await
            .unwrap();

        assert_eq!(resp.license, "lic-otp-ok");
        assert_eq!(resp.refresh_token, "rt-otp-ok");
        assert_eq!(resp.expires_at, "2099-06-01T00:00:00Z");
    }

    #[tokio::test]
    async fn otp_verify_wrong_code_returns_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/otp/verify"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = otp_verify(&client, &server.uri(), "user@test.com", "000000")
            .await
            .unwrap_err();

        assert!(
            err.to_string().contains("OTP verify response"),
            "expected context about verify response, got: {err}"
        );
    }

    // ── Credentials construction ──────────────────────────────────────

    #[test]
    fn credentials_from_device_poll_confirmed() {
        let poll = DevicePollResponse {
            status: "confirmed".to_string(),
            license: Some("lic-dev".to_string()),
            refresh_token: Some("rt-dev".to_string()),
            expires_at: Some("2099-01-01T00:00:00Z".to_string()),
        };

        let creds = Credentials {
            license: poll.license.unwrap(),
            refresh_token: poll.refresh_token,
            email: Some("dev@example.com".to_string()),
            expires_at: poll.expires_at,
        };

        assert_eq!(creds.license, "lic-dev");
        assert_eq!(creds.refresh_token.as_deref(), Some("rt-dev"));
        assert_eq!(creds.email.as_deref(), Some("dev@example.com"));
        assert_eq!(creds.expires_at.as_deref(), Some("2099-01-01T00:00:00Z"));
    }

    #[test]
    fn credentials_from_otp_verify() {
        let verify = OtpVerifyResponse {
            license: "lic-otp".to_string(),
            refresh_token: "rt-otp".to_string(),
            expires_at: "2099-06-15T12:00:00Z".to_string(),
        };

        let creds = Credentials {
            license: verify.license,
            refresh_token: Some(verify.refresh_token),
            email: Some("otp@example.com".to_string()),
            expires_at: Some(verify.expires_at),
        };

        assert_eq!(creds.license, "lic-otp");
        assert_eq!(creds.refresh_token.as_deref(), Some("rt-otp"));
        assert_eq!(creds.email.as_deref(), Some("otp@example.com"));
    }

    // ── Boundary: expires_in zero ─────────────────────────────────────

    #[test]
    fn deserialise_device_start_expires_in_zero() {
        let json = r#"{
            "pollToken": "tok-zero",
            "userCode": "ZERO-0000",
            "verificationUrl": "https://example.com/activate",
            "expiresIn": 0
        }"#;
        let resp: DeviceStartResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.expires_in, 0);
        // login_device_flow clamps: (0 / 5).max(1) == 1
        let max_attempts = (resp.expires_in / 5).max(1);
        assert_eq!(max_attempts, 1, "zero expires_in should clamp to 1 attempt");
    }

    // ── Malformed JSON response paths ─────────────────────────────────

    #[tokio::test]
    async fn device_start_malformed_json_returns_parse_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/device/start"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"unexpected": "shape"})),
            )
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = device_start(&client, &server.uri(), "dev@example.com")
            .await
            .unwrap_err();

        assert!(
            err.to_string()
                .contains("parsing device code start response"),
            "expected parse context, got: {err}"
        );
    }

    #[tokio::test]
    async fn otp_verify_malformed_json_returns_parse_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/otp/verify"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"unexpected": "shape"})),
            )
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = otp_verify(&client, &server.uri(), "user@test.com", "123456")
            .await
            .unwrap_err();

        assert!(
            err.to_string().contains("parsing OTP verify response"),
            "expected parse context, got: {err}"
        );
    }
}
