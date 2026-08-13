use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::credentials::{self, Credentials};

/// Response from `POST /api/v1/auth/github-device/start` — the server-side
/// broker for GitHub's Device Authorisation Grant (RFC 8628, ADR-066).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceStartResponse {
    poll_token: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    /// Server-relayed GitHub poll interval (RFC 8628 §3.2) — the initial
    /// sleep between polls; `slow_down` responses raise it.
    interval: u64,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshSessionResponse {
    license: String,
    refresh_token: String,
    expires_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RefreshSessionRequest<'a> {
    refresh_token: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EdictVerifyResponse {
    valid: bool,
    /// Server-side assertion that this token is an early-access edict, not a
    /// regular beta access token. Defaults to `false` for older servers that
    /// do not yet expose the field — the CLI treats that as "not an edict".
    #[serde(default)]
    is_edict: bool,
    user: Option<EdictUser>,
    expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EdictUser {
    email: String,
}

/// Strict empty start body — the endpoint rejects any field, by design
/// (no caller-supplied identity, ADR-066). An empty braced struct
/// serialises to `{}`; a unit struct would serialise to `null`.
#[derive(Debug, Serialize)]
struct DeviceStartRequest {}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DevicePollRequest<'a> {
    poll_token: &'a str,
}

/// Body of a 429 from `/github-device/poll` — both the broker's own
/// cross-instance gate and GitHub's relayed `slow_down` use this shape.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SlowDownBody {
    retry_after: Option<u64>,
}

/// Outcome of one poll round-trip. `SlowDown` is a back-off instruction, not
/// an error — the pre-ADR-066 flow treated every 429 as fatal, which made the
/// CLI bail the moment the broker asked it to slow down.
#[derive(Debug)]
enum DevicePoll {
    Status(DevicePollResponse),
    SlowDown { retry_after: Option<u64> },
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

#[derive(Debug, Serialize)]
struct EdictVerifyRequest<'a> {
    token: &'a str,
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

/// Summarise a network-level reqwest error into a short user-visible hint.
fn friendly_network_error(e: &reqwest::Error) -> &'static str {
    if e.is_connect() {
        "could not connect"
    } else if e.is_timeout() {
        "timed out"
    } else if e.is_redirect() {
        "too many redirects"
    } else {
        "network error"
    }
}

/// Convert an HTTP error status into a user-friendly message.
fn friendly_http_error(status: reqwest::StatusCode, context: &str) -> String {
    match status.as_u16() {
        401 => format!("{context}: authorisation failed. Please try again."),
        403 => format!("{context}: access denied. Check that your account is approved."),
        404 => format!("{context}: auth service not found. Check your ANVIL_API_URL setting."),
        429 => format!("{context}: too many requests. Please wait a moment and try again."),
        500..=599 => format!(
            "{context}: the auth server is temporarily unavailable. Please try again in a few minutes."
        ),
        _ => format!("{context}: unexpected error (HTTP {status})."),
    }
}

/// Check response status and return a user-friendly error on failure.
fn check_status(response: reqwest::Response, context: &str) -> Result<reqwest::Response> {
    let status = response.status();
    if status.is_client_error() || status.is_server_error() {
        bail!(friendly_http_error(status, context));
    }
    Ok(response)
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

/// Begin a brokered GitHub device-flow session. The endpoint takes a strict
/// empty body — no email, no user reference; the signed-in user is derived
/// solely from the GitHub authorisation when the device code is approved
/// (ADR-066).
async fn device_start(client: &reqwest::Client, url: &str) -> Result<DeviceStartResponse> {
    let resp = client
        .post(format!("{url}/api/v1/auth/github-device/start"))
        .json(&DeviceStartRequest {})
        .send()
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Could not reach the auth server ({}). Check your network connection.",
                friendly_network_error(&e)
            )
        })?;
    check_status(resp, "Login failed")?
        .json()
        .await
        .context("Login failed: unexpected response from the auth server.")
}

async fn device_poll(client: &reqwest::Client, url: &str, poll_token: &str) -> Result<DevicePoll> {
    let resp = client
        .post(format!("{url}/api/v1/auth/github-device/poll"))
        .json(&DevicePollRequest { poll_token })
        .send()
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Could not reach the auth server while checking login status ({}).",
                friendly_network_error(&e)
            )
        })?;
    // 429 is a back-off instruction (RFC 8628 slow_down / the broker's poll
    // gate), never fatal. Anything else non-2xx is a real failure.
    if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry_after = resp
            .json::<SlowDownBody>()
            .await
            .ok()
            .and_then(|b| b.retry_after);
        return Ok(DevicePoll::SlowDown { retry_after });
    }
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        bail!("GitHub sign-in failed. Run `anvil auth login` to try again.");
    }
    check_status(resp, "Login check failed")?
        .json()
        .await
        .map(DevicePoll::Status)
        .context("Login check failed: unexpected response from the auth server.")
}

/// Terminal poll states and their user-facing messages. `None` means the
/// state is non-terminal (pending / unknown) and the CLI keeps waiting.
fn poll_failure_message(status: &str) -> Option<&'static str> {
    match status {
        "expired" => Some(
            "The sign-in request expired before it was approved. Run `anvil auth login` to start \
             again.",
        ),
        "declined" => Some("GitHub sign-in was declined. Run `anvil auth login` to try again."),
        "awaiting_approval" => Some(
            "Signed in to GitHub, but your anvil account is awaiting approval — you'll receive an \
             email when it's ready. If you were invited by email, run `anvil auth login --otp` \
             instead.",
        ),
        _ => None,
    }
}

async fn otp_request(
    client: &reqwest::Client,
    url: &str,
    email: &str,
) -> Result<OtpRequestResponse> {
    let resp = client
        .post(format!("{url}/api/v1/auth/otp/request"))
        .json(&OtpSendRequest { email })
        .send()
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Could not reach the auth server ({}). Check your network connection.",
                friendly_network_error(&e)
            )
        })?;
    check_status(resp, "Verification code request failed")?
        .json()
        .await
        .context("Verification code request failed: unexpected response from the auth server.")
}

async fn otp_verify(
    client: &reqwest::Client,
    url: &str,
    email: &str,
    code: &str,
) -> Result<OtpVerifyResponse> {
    let resp = client
        .post(format!("{url}/api/v1/auth/otp/verify"))
        .json(&OtpVerifyRequest { email, code })
        .send()
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Could not reach the auth server ({}). Check your network connection.",
                friendly_network_error(&e)
            )
        })?;
    check_status(resp, "Invalid or expired code")?
        .json()
        .await
        .context("Verification failed: unexpected response from the auth server.")
}

async fn edict_verify(
    client: &reqwest::Client,
    url: &str,
    edict: &str,
) -> Result<EdictVerifyResponse> {
    let resp = client
        .post(format!("{url}/api/v1/auth/verify"))
        .json(&EdictVerifyRequest { token: edict })
        .send()
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Could not reach the auth server ({}). Check your network connection.",
                friendly_network_error(&e)
            )
        })?;
    check_status(resp, "Edict verification failed")?
        .json()
        .await
        .context("Edict verification failed: unexpected response from the auth server.")
}

async fn refresh_session(
    client: &reqwest::Client,
    url: &str,
    refresh_token: &str,
) -> Result<RefreshSessionResponse> {
    let resp = client
        .post(format!("{url}/api/v1/auth/session/refresh"))
        .json(&RefreshSessionRequest { refresh_token })
        .send()
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Could not reach the auth server ({}). Check your network connection.",
                friendly_network_error(&e)
            )
        })?;
    let status = resp.status();
    if status.is_client_error() || status.is_server_error() {
        let body = resp.text().await.unwrap_or_default();
        bail!(refresh_error_message(status, &body));
    }
    resp.json()
        .await
        .context("Session refresh failed: unexpected response from the auth server.")
}

/// Distinct prefixes the refresh path uses to signal "this token will never
/// work again — the user must run `anvil auth login`". Anything not on this
/// list is treated as transient.
const PERMANENT_REFRESH_PREFIXES: &[&str] = &[
    "Refresh token expired",
    "Refresh token is invalid or revoked",
    "Refresh-token reuse detected",
    "Your anvil account is not active",
    "Session refresh rejected by the auth server",
];

/// Returns true when the error message originated from
/// [`refresh_error_message`] for a definitive 401. Used by the silent-refresh
/// caller to decide whether to surface the message and skip the generic
/// "Session expired" line, vs swallow the failure as transient.
pub fn is_permanent_refresh_failure(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    PERMANENT_REFRESH_PREFIXES
        .iter()
        .any(|prefix| msg.starts_with(prefix))
}

/// Pick the most actionable message for a /session/refresh failure.
///
/// The server returns distinct `{ "error": "…" }` strings for the four
/// terminal cases (expired refresh token, revoked, family theft detected,
/// inactive account); surface them so the user knows whether a fresh
/// `anvil auth login` will fix things or whether support is needed.
fn refresh_error_message(status: reqwest::StatusCode, body: &str) -> String {
    let server_reason = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v.get("error")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });
    match (status.as_u16(), server_reason.as_deref()) {
        (401, Some("Refresh token expired")) => {
            "Refresh token expired. Run `anvil auth login` to re-authenticate.".into()
        }
        (401, Some("Token reuse detected")) => {
            "Refresh-token reuse detected — your session family was revoked for security. \
             Run `anvil auth login`."
                .into()
        }
        (401, Some("Invalid refresh token")) => {
            "Refresh token is invalid or revoked. Run `anvil auth login`.".into()
        }
        (401, Some("User account is not active")) => {
            "Your anvil account is not active. Contact support.".into()
        }
        (401, _) => "Session refresh rejected by the auth server. Run `anvil auth login`.".into(),
        _ => friendly_http_error(status, "Session refresh failed"),
    }
}

/// Attempt to refresh an expired licence using a stored refresh token.
///
/// Returns the new `Credentials` (caller is responsible for persisting via
/// `credentials::save`). Carries `email` and `is_edict` forward from the
/// existing credentials since `/session/refresh` does not echo them.
pub async fn try_refresh_credentials(creds: &Credentials) -> Result<Credentials> {
    let refresh_token = creds
        .refresh_token
        .as_deref()
        .context("no refresh token stored")?;
    let url = api_url()?;
    let client = build_client()?;
    let resp = refresh_session(&client, &url, refresh_token).await?;
    Ok(Credentials {
        license: resp.license,
        refresh_token: Some(resp.refresh_token),
        email: creds.email.clone(),
        expires_at: Some(resp.expires_at),
        is_edict: creds.is_edict,
    })
}

/// End-to-end refresh: lock the credential file, re-read stored credentials,
/// exchange the refresh token, persist the new credentials, and return them.
///
/// The exclusive lock serialises concurrent CLI processes so two refreshes
/// cannot submit the same rotating token (which the server treats as reuse
/// and answers by revoking the session family).
pub async fn refresh_command() -> Result<Credentials> {
    let _lock = credentials::CredentialRefreshLock::acquire()?;
    let creds = credentials::load()?
        .context("Not authenticated. Run `anvil auth login` to authenticate.")?;
    if creds.refresh_token.is_none() {
        bail!(
            "No refresh token on disk. This usually means credentials were minted by an older \
             version that did not persist refresh tokens; run `anvil auth login` to mint a fresh \
             pair."
        );
    }
    let new_creds = try_refresh_credentials(&creds).await?;
    credentials::save(&new_creds)?;
    Ok(new_creds)
}

// ── Public entry points (thin wrappers adding I/O) ────────────────────

/// Clamp the server-relayed poll interval (RFC 8628 §3.5) so a broken value
/// can neither spin-loop (0) nor stall the terminal (huge).
fn clamp_interval(seconds: u64) -> u64 {
    seconds.clamp(1, 3_600)
}

/// Bound the device-code lifetime to a sane window. Without the upper bound a
/// hostile `expiresIn` (e.g. `u64::MAX`) panics in `Instant + Duration`.
fn deadline_window(expires_in: u64) -> Duration {
    Duration::from_secs(expires_in.clamp(1, 86_400))
}

/// Strip control characters from a server-supplied string before printing it
/// to the terminal — ANSI/OSC sequences in a hostile response must not be
/// able to forge hyperlinks, clear the screen, or retitle the window.
fn sanitize_for_terminal(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}

/// Sign in via the brokered GitHub device flow (ADR-066). Headless-friendly:
/// no email prompt, no local browser required — the verification URL can be
/// opened on any device.
pub async fn login_device_flow() -> Result<()> {
    let url = api_url()?;
    let client = build_client()?;

    eprintln!("Starting GitHub sign-in...");

    let start = device_start(&client, &url).await?;

    eprintln!();
    eprintln!("To sign in, open this URL on any device:");
    eprintln!("  {}", sanitize_for_terminal(&start.verification_uri));
    eprintln!();
    eprintln!(
        "And enter code: {}",
        sanitize_for_terminal(&start.user_code)
    );
    eprintln!();
    eprintln!("Waiting for authorisation...");

    let deadline = std::time::Instant::now() + deadline_window(start.expires_in);
    // Initial sleep between polls; slow_down responses raise it.
    let mut interval = clamp_interval(start.interval);

    loop {
        // Never sleep past the deadline — a late slow_down must not stall
        // the terminal for its full interval before the timeout message.
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        tokio::time::sleep(remaining.min(Duration::from_secs(interval))).await;

        match device_poll(&client, &url, &start.poll_token).await? {
            DevicePoll::SlowDown { retry_after } => {
                interval = clamp_interval(retry_after.unwrap_or(interval + 5));
                eprint!(".");
            }
            DevicePoll::Status(poll) => match poll.status.as_str() {
                "confirmed" => {
                    let license = poll.license.context("server returned no licence")?;
                    let refresh = poll
                        .refresh_token
                        .context("server returned no refresh token")?;
                    let expires = poll.expires_at.context("server returned no expiry")?;

                    // No email at mint time by design — identity came from
                    // GitHub server-side; `anvil auth whoami` resolves the
                    // account email from the server.
                    credentials::save(&Credentials {
                        license,
                        refresh_token: Some(refresh),
                        email: None,
                        expires_at: Some(expires),
                        is_edict: Some(false),
                    })?;

                    eprintln!();
                    eprintln!("✓ Authenticated via GitHub");
                    let path = credentials::credentials_path()?;
                    eprintln!("  Credentials saved to {}", path.display());
                    eprintln!("  Run `anvil auth whoami` to see your account.");
                    return Ok(());
                }
                status => {
                    if let Some(message) = poll_failure_message(status) {
                        bail!(message);
                    }
                    eprint!(".");
                }
            },
        }
    }

    bail!("Timed out waiting for authorisation. Run `anvil auth login` to try again.");
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
                    is_edict: Some(false),
                })?;

                eprintln!();
                eprintln!("✓ Authenticated as {email}");
                let path = credentials::credentials_path()?;
                eprintln!("  Credentials saved to {}", path.display());
                return Ok(());
            }
            Err(e) => {
                eprintln!("{e}");
                if attempt < 3 {
                    eprintln!("{} attempt(s) remaining", 3 - attempt);
                }
            }
        }
    }

    bail!("Maximum attempts reached. Please try again.");
}

pub async fn login_edict_flow() -> Result<()> {
    let url = api_url()?;
    let edict = prompt_input("Early-access edict: ")?;
    if edict.is_empty() {
        bail!("Early-access edict is required");
    }

    eprintln!("Verifying edict...");

    let client = build_client()?;
    let result = edict_verify(&client, &url, &edict).await?;
    if !result.valid {
        bail!("Invalid or expired early-access edict");
    }
    if !result.is_edict {
        // The token verified as a generic beta access token, not as an
        // early-access edict. Reject it on the edict path so a regular
        // service / CI token cannot be redeemed via `--edict` and gain
        // edict-only privileges downstream.
        bail!(
            "That token is not an early-access edict. Use `anvil auth login` instead, \
             or contact support if you believe this is wrong."
        );
    }

    let email = result.user.map(|user| user.email);
    credentials::save(&Credentials {
        license: edict,
        refresh_token: None,
        email: email.clone(),
        expires_at: result.expires_at,
        is_edict: Some(true),
    })?;

    eprintln!();
    if let Some(email) = email {
        eprintln!("✓ Authenticated as {email}");
    } else {
        eprintln!("✓ Edict accepted");
    }
    let path = credentials::credentials_path()?;
    eprintln!("  Credentials saved to {}", path.display());
    Ok(())
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
            "userCode": "WDJB-MJHT",
            "verificationUri": "https://github.com/login/device",
            "expiresIn": 899,
            "interval": 5
        }"#;
        let resp: DeviceStartResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.poll_token, "tok-abc");
        assert_eq!(resp.user_code, "WDJB-MJHT");
        assert_eq!(resp.verification_uri, "https://github.com/login/device");
        assert_eq!(resp.expires_in, 899);
        assert_eq!(resp.interval, 5);
    }

    #[test]
    fn deserialise_device_poll_declined_and_awaiting_approval() {
        for status in ["declined", "awaiting_approval"] {
            let json = format!(r#"{{"status": "{status}"}}"#);
            let resp: DevicePollResponse = serde_json::from_str(&json).unwrap();
            assert_eq!(resp.status, status);
            assert!(resp.license.is_none());
        }
    }

    #[test]
    fn deserialise_slow_down_body_with_and_without_retry_after() {
        let with: SlowDownBody =
            serde_json::from_str(r#"{"error": "slow_down", "retryAfter": 10}"#).unwrap();
        assert_eq!(with.retry_after, Some(10));

        let without: SlowDownBody =
            serde_json::from_str(r#"{"error": "Too many requests"}"#).unwrap();
        assert_eq!(without.retry_after, None);
    }

    // ── Terminal-state messages ───────────────────────────────────────

    #[test]
    fn poll_failure_message_maps_terminal_states() {
        let expired = poll_failure_message("expired").unwrap();
        assert!(expired.contains("expired"), "got: {expired}");
        assert!(expired.contains("anvil auth login"), "got: {expired}");

        let declined = poll_failure_message("declined").unwrap();
        assert!(declined.contains("declined"), "got: {declined}");

        let awaiting = poll_failure_message("awaiting_approval").unwrap();
        assert!(awaiting.contains("awaiting approval"), "got: {awaiting}");
        assert!(
            awaiting.contains("--otp"),
            "awaiting-approval should point at the OTP fallback, got: {awaiting}"
        );
    }

    #[test]
    fn poll_failure_message_keeps_waiting_on_non_terminal_states() {
        assert!(poll_failure_message("pending").is_none());
        assert!(poll_failure_message("confirmed").is_none());
        assert!(poll_failure_message("something_new").is_none());
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

    // ── Wiremock: device_start (brokered GitHub device flow) ──────────

    #[tokio::test]
    async fn device_start_sends_strict_empty_body_to_github_device_endpoint() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/github-device/start"))
            .and(body_json(serde_json::json!({})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "pollToken": "poll-xyz",
                "userCode": "TEST-9999",
                "verificationUri": "https://github.com/login/device",
                "expiresIn": 600,
                "interval": 7
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let resp = device_start(&client, &server.uri()).await.unwrap();

        assert_eq!(resp.poll_token, "poll-xyz");
        assert_eq!(resp.user_code, "TEST-9999");
        assert_eq!(resp.verification_uri, "https://github.com/login/device");
        assert_eq!(resp.expires_in, 600);
        assert_eq!(resp.interval, 7, "interval must round-trip from the server");
    }

    #[tokio::test]
    async fn device_start_propagates_server_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/github-device/start"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = device_start(&client, &server.uri()).await.unwrap_err();

        assert!(
            err.to_string().contains("Login failed"),
            "expected user-friendly error, got: {err}"
        );
    }

    // ── Wiremock: device_poll ─────────────────────────────────────────

    #[tokio::test]
    async fn device_poll_confirmed() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/github-device/poll"))
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
        let poll = device_poll(&client, &server.uri(), "tok-1").await.unwrap();

        let DevicePoll::Status(resp) = poll else {
            panic!("expected Status, got slow_down");
        };
        assert_eq!(resp.status, "confirmed");
        assert_eq!(resp.license.as_deref(), Some("lic-confirmed"));
        assert_eq!(resp.refresh_token.as_deref(), Some("rt-confirmed"));
    }

    #[tokio::test]
    async fn device_poll_pending() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/github-device/poll"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "pending"})),
            )
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let poll = device_poll(&client, &server.uri(), "tok-1").await.unwrap();

        let DevicePoll::Status(resp) = poll else {
            panic!("expected Status, got slow_down");
        };
        assert_eq!(resp.status, "pending");
        assert!(resp.license.is_none());
    }

    #[tokio::test]
    async fn device_poll_terminal_statuses_deserialise() {
        for status in ["expired", "declined", "awaiting_approval"] {
            let server = MockServer::start().await;

            Mock::given(method("POST"))
                .and(path("/api/v1/auth/github-device/poll"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": status})),
                )
                .mount(&server)
                .await;

            let client = build_client().unwrap();
            let poll = device_poll(&client, &server.uri(), "tok-1").await.unwrap();

            let DevicePoll::Status(resp) = poll else {
                panic!("expected Status for {status}, got slow_down");
            };
            assert_eq!(resp.status, status);
        }
    }

    #[tokio::test]
    async fn device_poll_429_is_back_off_not_fatal() {
        // The pre-ADR-066 flow bailed on any 429 (`friendly_http_error(429)`),
        // killing the login the moment GitHub said slow_down. A 429 must come
        // back as a SlowDown instruction carrying the server's retryAfter.
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/github-device/poll"))
            .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
                "error": "slow_down",
                "retryAfter": 10
            })))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let poll = device_poll(&client, &server.uri(), "tok-1").await.unwrap();

        let DevicePoll::SlowDown { retry_after } = poll else {
            panic!("expected SlowDown, got status");
        };
        assert_eq!(retry_after, Some(10));
    }

    #[tokio::test]
    async fn device_poll_429_without_retry_after_still_backs_off() {
        // A bare 429 (e.g. the per-IP limiter, no retryAfter field) must also
        // be a back-off, with the caller choosing the fallback interval.
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/github-device/poll"))
            .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
                "error": "Too many requests, please try again later"
            })))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let poll = device_poll(&client, &server.uri(), "tok-1").await.unwrap();

        let DevicePoll::SlowDown { retry_after } = poll else {
            panic!("expected SlowDown, got status");
        };
        assert_eq!(retry_after, None);
    }

    #[tokio::test]
    async fn device_poll_401_maps_to_clear_github_failure() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/github-device/poll"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": "github_authentication_failed"
            })))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = device_poll(&client, &server.uri(), "tok-1")
            .await
            .unwrap_err();
        let msg = err.to_string();

        assert!(msg.contains("GitHub sign-in failed"), "got: {msg}");
        assert!(msg.contains("anvil auth login"), "got: {msg}");
    }

    #[tokio::test]
    async fn device_poll_propagates_server_error() {
        let server = MockServer::start().await;

        // 503 (not 403/401/429): those statuses now have dedicated branches;
        // this exercises the generic check_status error path.
        Mock::given(method("POST"))
            .and(path("/api/v1/auth/github-device/poll"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = device_poll(&client, &server.uri(), "tok-bad")
            .await
            .unwrap_err();

        assert!(
            err.to_string().contains("Login check failed"),
            "expected user-friendly error, got: {err}"
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
            err.to_string().contains("Verification code request failed"),
            "expected user-friendly error, got: {err}"
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
    async fn edict_verify_parses_valid_response() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/verify"))
            .and(body_json(serde_json::json!({"token": "anvil_beta_edict"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "valid": true,
                "isEdict": true,
                "user": {"email": "early@example.com"},
                "expiresAt": "2099-07-01T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let resp = edict_verify(&client, &server.uri(), "anvil_beta_edict")
            .await
            .unwrap();

        assert!(resp.valid);
        assert!(resp.is_edict);
        assert_eq!(resp.user.unwrap().email, "early@example.com");
        assert_eq!(resp.expires_at.as_deref(), Some("2099-07-01T00:00:00Z"));
    }

    #[tokio::test]
    async fn edict_verify_defaults_is_edict_false_when_field_absent() {
        // Older API servers do not return `isEdict`. The CLI must default
        // to `false` so a non-edict token cannot be redeemed via the
        // edict-login path against an outdated server.
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/verify"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "valid": true,
                "user": {"email": "service@example.com"},
                "expiresAt": "2099-07-01T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let resp = edict_verify(&client, &server.uri(), "anvil_beta_service")
            .await
            .unwrap();

        assert!(resp.valid);
        assert!(!resp.is_edict);
    }

    #[tokio::test]
    async fn refresh_session_returns_new_credentials_on_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/session/refresh"))
            .and(body_json(serde_json::json!({"refreshToken": "rt-old"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "license": "lic-new",
                "refreshToken": "rt-new",
                "expiresAt": "2099-12-31T23:59:59Z"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let resp = refresh_session(&client, &server.uri(), "rt-old")
            .await
            .unwrap();

        assert_eq!(resp.license, "lic-new");
        assert_eq!(resp.refresh_token, "rt-new");
        assert_eq!(resp.expires_at, "2099-12-31T23:59:59Z");
    }

    #[tokio::test]
    async fn refresh_session_surfaces_expired_reason() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/session/refresh"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": "Refresh token expired"
            })))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = refresh_session(&client, &server.uri(), "rt-stale")
            .await
            .unwrap_err();
        let msg = err.to_string();

        assert!(
            msg.contains("Refresh token expired"),
            "expected expired-token wording, got: {msg}"
        );
        assert!(
            msg.contains("anvil auth login"),
            "expected actionable next step, got: {msg}"
        );
    }

    #[tokio::test]
    async fn refresh_session_surfaces_token_reuse_reason() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/session/refresh"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": "Token reuse detected"
            })))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = refresh_session(&client, &server.uri(), "rt-cloned")
            .await
            .unwrap_err();
        let msg = err.to_string();

        assert!(
            msg.contains("reuse detected"),
            "expected reuse wording, got: {msg}"
        );
        assert!(
            msg.contains("anvil auth login"),
            "expected actionable next step, got: {msg}"
        );
    }

    #[tokio::test]
    async fn refresh_session_surfaces_invalid_refresh_reason() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/session/refresh"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": "Invalid refresh token"
            })))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = refresh_session(&client, &server.uri(), "rt-bogus")
            .await
            .unwrap_err();
        let msg = err.to_string();

        assert!(
            msg.contains("invalid or revoked"),
            "expected invalid/revoked wording, got: {msg}"
        );
    }

    #[tokio::test]
    async fn refresh_session_surfaces_inactive_user_reason() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/session/refresh"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": "User account is not active"
            })))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = refresh_session(&client, &server.uri(), "rt-inactive")
            .await
            .unwrap_err();
        let msg = err.to_string();

        assert!(
            msg.contains("not active"),
            "expected inactive-account wording, got: {msg}"
        );
        assert!(
            msg.contains("support"),
            "inactive-account error should point at support, got: {msg}"
        );
    }

    #[tokio::test]
    async fn refresh_session_falls_back_for_unrecognised_401_reason() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/session/refresh"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": "Some future server reason"
            })))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = refresh_session(&client, &server.uri(), "rt-x")
            .await
            .unwrap_err();
        let msg = err.to_string();

        assert!(
            msg.contains("Session refresh rejected") || msg.contains("rejected"),
            "expected generic 401 fallback, got: {msg}"
        );
    }

    #[tokio::test]
    async fn refresh_session_propagates_5xx() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/session/refresh"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = refresh_session(&client, &server.uri(), "rt-x")
            .await
            .unwrap_err();

        assert!(
            err.to_string().contains("Session refresh failed"),
            "expected user-friendly error, got: {err}"
        );
    }

    #[tokio::test]
    async fn try_refresh_credentials_errors_when_no_refresh_token_stored() {
        let creds = Credentials {
            license: "lic-old".to_string(),
            refresh_token: None,
            email: Some("user@example.com".to_string()),
            expires_at: Some("2020-01-01T00:00:00Z".to_string()),
            is_edict: Some(false),
        };
        let err = try_refresh_credentials(&creds).await.unwrap_err();
        assert!(
            err.to_string().contains("no refresh token stored"),
            "expected explicit no-refresh-token message, got: {err}"
        );
    }

    #[test]
    fn is_permanent_refresh_failure_recognises_known_reasons() {
        for msg in [
            "Refresh token expired. Run `anvil auth login` to re-authenticate.",
            "Refresh token is invalid or revoked. Run `anvil auth login`.",
            "Refresh-token reuse detected — your session family was revoked for security. \
             Run `anvil auth login`.",
            "Your anvil account is not active. Contact support.",
            "Session refresh rejected by the auth server. Run `anvil auth login`.",
        ] {
            let err = anyhow::anyhow!("{msg}");
            assert!(
                is_permanent_refresh_failure(&err),
                "expected `{msg}` to be classified as permanent"
            );
        }
    }

    #[test]
    fn is_permanent_refresh_failure_treats_network_errors_as_transient() {
        for msg in [
            "Could not reach the auth server (connect refused). Check your network connection.",
            "Session refresh failed: the auth server is temporarily unavailable. \
             Please try again in a few minutes.",
            "Session refresh failed: too many requests. Please wait a moment and try again.",
            "no refresh token stored",
        ] {
            let err = anyhow::anyhow!("{msg}");
            assert!(
                !is_permanent_refresh_failure(&err),
                "expected `{msg}` to be classified as transient"
            );
        }
    }

    #[test]
    fn deserialise_refresh_session_response() {
        let json = r#"{
            "license": "lic-rs",
            "refreshToken": "rt-rs",
            "expiresAt": "2099-09-09T09:09:09Z"
        }"#;
        let resp: RefreshSessionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.license, "lic-rs");
        assert_eq!(resp.refresh_token, "rt-rs");
        assert_eq!(resp.expires_at, "2099-09-09T09:09:09Z");
    }

    #[test]
    fn serialise_refresh_session_request() {
        let req = RefreshSessionRequest {
            refresh_token: "rt-ser",
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["refreshToken"], "rt-ser");
        assert!(
            json.get("refresh_token").is_none(),
            "refresh-session request must use camelCase"
        );
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
            err.to_string().contains("Invalid or expired code"),
            "expected user-friendly error, got: {err}"
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

        // The GitHub device flow stores no email — identity was derived
        // server-side from the GitHub token; whoami resolves it on demand.
        let creds = Credentials {
            license: poll.license.unwrap(),
            refresh_token: poll.refresh_token,
            email: None,
            expires_at: poll.expires_at,
            is_edict: Some(false),
        };

        assert_eq!(creds.license, "lic-dev");
        assert_eq!(creds.refresh_token.as_deref(), Some("rt-dev"));
        assert!(creds.email.is_none());
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
            is_edict: Some(false),
        };

        assert_eq!(creds.license, "lic-otp");
        assert_eq!(creds.refresh_token.as_deref(), Some("rt-otp"));
        assert_eq!(creds.email.as_deref(), Some("otp@example.com"));
    }

    // ── friendly_http_error coverage ────────────────────────────────────

    #[test]
    fn friendly_http_error_401() {
        let msg = friendly_http_error(reqwest::StatusCode::UNAUTHORIZED, "Login");
        assert_eq!(msg, "Login: authorisation failed. Please try again.");
    }

    #[test]
    fn friendly_http_error_403() {
        let msg = friendly_http_error(reqwest::StatusCode::FORBIDDEN, "Login");
        assert_eq!(
            msg,
            "Login: access denied. Check that your account is approved."
        );
    }

    #[test]
    fn friendly_http_error_404() {
        let msg = friendly_http_error(reqwest::StatusCode::NOT_FOUND, "Login");
        assert_eq!(
            msg,
            "Login: auth service not found. Check your ANVIL_API_URL setting."
        );
    }

    #[test]
    fn friendly_http_error_429() {
        let msg = friendly_http_error(reqwest::StatusCode::TOO_MANY_REQUESTS, "Login");
        assert_eq!(
            msg,
            "Login: too many requests. Please wait a moment and try again."
        );
    }

    #[test]
    fn friendly_http_error_500() {
        let msg = friendly_http_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "Login");
        assert!(msg.contains("temporarily unavailable"), "got: {msg}");
    }

    #[test]
    fn friendly_http_error_502() {
        let msg = friendly_http_error(reqwest::StatusCode::BAD_GATEWAY, "Login");
        assert!(msg.contains("temporarily unavailable"), "got: {msg}");
    }

    #[test]
    fn friendly_http_error_other() {
        let msg = friendly_http_error(reqwest::StatusCode::CONFLICT, "Login");
        assert_eq!(msg, "Login: unexpected error (HTTP 409 Conflict).");
    }

    // ── Boundary: hostile/broken server values are clamped ────────────

    #[test]
    fn clamp_interval_bounds_hostile_values() {
        assert_eq!(clamp_interval(0), 1, "zero must not spin-loop");
        assert_eq!(clamp_interval(5), 5);
        assert_eq!(clamp_interval(u64::MAX), 3_600, "huge must not stall");
    }

    #[test]
    fn deadline_window_bounds_hostile_expires_in() {
        // u64::MAX seconds would panic in `Instant + Duration`; the window
        // must be bounded on both ends.
        assert_eq!(deadline_window(0), Duration::from_secs(1));
        assert_eq!(deadline_window(899), Duration::from_secs(899));
        assert_eq!(deadline_window(u64::MAX), Duration::from_hours(24));
    }

    #[test]
    fn sanitize_for_terminal_strips_control_sequences() {
        // OSC 8 hyperlink forgery, screen clear, and window retitle must all
        // come out inert; plain URLs and codes pass through unchanged.
        assert_eq!(
            sanitize_for_terminal("\x1b]8;;https://evil.example\x07click\x1b]8;;\x07"),
            "]8;;https://evil.exampleclick]8;;"
        );
        assert_eq!(sanitize_for_terminal("\x1b[2J\x1b[Hcode"), "[2J[Hcode");
        assert_eq!(
            sanitize_for_terminal("https://github.com/login/device"),
            "https://github.com/login/device"
        );
        assert_eq!(sanitize_for_terminal("WDJB-MJHT"), "WDJB-MJHT");
    }

    #[test]
    fn deserialise_device_start_expires_in_zero() {
        let json = r#"{
            "pollToken": "tok-zero",
            "userCode": "ZERO-0000",
            "verificationUri": "https://github.com/login/device",
            "expiresIn": 0,
            "interval": 5
        }"#;
        let resp: DeviceStartResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.expires_in, 0);
        assert_eq!(deadline_window(resp.expires_in), Duration::from_secs(1));
    }

    // ── Malformed JSON response paths ─────────────────────────────────

    #[tokio::test]
    async fn device_start_malformed_json_returns_parse_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/github-device/start"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"unexpected": "shape"})),
            )
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let err = device_start(&client, &server.uri()).await.unwrap_err();

        assert!(
            err.to_string()
                .contains("unexpected response from the auth server"),
            "expected user-friendly parse error, got: {err}"
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
            err.to_string()
                .contains("unexpected response from the auth server"),
            "expected user-friendly parse error, got: {err}"
        );
    }
}
