//! GHCLIAUTH-011: end-to-end acceptance test for the brokered GitHub
//! device flow (ADR-066). This is the regression guard for the original
//! bug — a *headless* `anvil auth login` used to be un-completable because
//! the pre-broker flow needed a browser confirm page that #1779 broke.
//!
//! Each test drives the real `anvil` binary (no library shortcuts) against
//! a wiremock server standing in for `anvil-api`, asserts the credentials
//! the CLI actually persisted to disk, and — on the happy path — proves
//! `anvil auth whoami` resolves the identity back through the mocked broker.
//!
//! ## Why these run headless
//!
//! The device flow takes no stdin at all; the OTP flow reads two lines
//! (email, then code) which we pipe in. None of them open a browser or a
//! TUI. `--no-tui`, `ANVIL_SKIP_WELCOME`, `ANVIL_NO_PROMPT` and an empty
//! `XDG_CONFIG_HOME`/`HOME` keep the run isolated and non-interactive.
//!
//! ## Fast wall-clock
//!
//! The poll loop sleeps on the server-relayed `interval`. Every mock here
//! returns `interval: 1` (the client's `clamp_interval` floor) and uses
//! wiremock's `up_to_n_times` sequencing so the loop exits after one or two
//! polls — the whole file runs in a couple of seconds.

use std::collections::HashSet;
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// A per-test isolated environment: a tempdir that doubles as `HOME`,
/// `XDG_CONFIG_HOME`, and `ANVIL_HOME`. The explicit `ANVIL_HOME` re-roots
/// credentials to `<home>/anvil-home/user/credentials.json` (DISTRIB-006) on
/// every platform — without it the Windows build writes under %APPDATA% and
/// would escape the tempdir. Nothing on the developer's machine is read or
/// written.
struct TestEnv {
    home: tempfile::TempDir,
    workdir: tempfile::TempDir,
}

impl TestEnv {
    fn new() -> Self {
        Self {
            home: tempfile::tempdir().unwrap(),
            workdir: tempfile::tempdir().unwrap(),
        }
    }

    /// Path the CLI writes credentials to under this env's `ANVIL_HOME`.
    fn credentials_path(&self) -> std::path::PathBuf {
        self.home
            .path()
            .join("anvil-home")
            .join("user")
            .join("credentials.json")
    }

    /// Base `anvil` command pointed at the mock broker, fully isolated and
    /// non-interactive. `api_base` is the wiremock `server.uri()`; `args` are
    /// the CLI arguments (e.g. `["--no-tui", "auth", "login"]`).
    fn command(&self, api_base: &str, args: &[&str]) -> Command {
        let mut cmd = Command::new(ANVIL_BIN);
        cmd.args(args)
            .current_dir(self.workdir.path())
            .env("HOME", self.home.path())
            .env("USERPROFILE", self.home.path())
            // XDG path == HOME so config reads resolve under the tempdir.
            .env("XDG_CONFIG_HOME", self.home.path())
            // Re-root credentials deterministically on ALL platforms — the
            // Windows fallback otherwise writes under %APPDATA%.
            .env("ANVIL_HOME", self.home.path().join("anvil-home"))
            // wiremock binds 127.0.0.1; api_url() allows http for localhost.
            .env("ANVIL_API_URL", api_base)
            .env("ANVIL_SKIP_WELCOME", "1")
            .env("ANVIL_NO_PROMPT", "1")
            .env("ANVIL_LOG", "off")
            // No dev bypass, no ambient licence — exercise the real flow.
            .env_remove("ANVIL_DEV")
            .env_remove("ANVIL_LICENSE")
            .env_remove("RUST_LOG");
        cmd
    }

    /// Read and parse the persisted credentials, or `None` if the CLI wrote
    /// nothing.
    fn saved_credentials(&self) -> Option<Value> {
        let raw = std::fs::read_to_string(self.credentials_path()).ok()?;
        Some(serde_json::from_str(&raw).expect("credentials.json must be valid JSON"))
    }
}

/// Run a prepared command to completion off the async worker so wiremock can
/// keep serving requests on its background task while the CLI subprocess
/// blocks. `stdin` is fed to the child's standard input (used by the OTP
/// flow, which prompts for email then code).
async fn run(mut cmd: Command, stdin: Option<&'static str>) -> Output {
    tokio::task::spawn_blocking(move || {
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().expect("failed to spawn anvil binary");
        if let Some(input) = stdin {
            use std::io::Write as _;
            child
                .stdin
                .take()
                .expect("child stdin")
                .write_all(input.as_bytes())
                .expect("write child stdin");
        }
        child.wait_with_output().expect("wait for anvil binary")
    })
    .await
    .expect("join blocking task")
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Mount the device-flow `start` endpoint returning a tight 1s interval.
async fn mount_device_start(server: &MockServer, poll_token: &str) {
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/github-device/start"))
        .and(body_json(serde_json::json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "pollToken": poll_token,
            "userCode": "TEST-CODE",
            "verificationUri": "https://github.com/login/device",
            "expiresIn": 600,
            "interval": 1
        })))
        .expect(1)
        .mount(server)
        .await;
}

// ── Happy path: start → pending poll → confirmed ──────────────────────────

#[tokio::test]
async fn device_flow_e2e_confirmed_saves_credentials_and_whoami_resolves() {
    let env = TestEnv::new();
    let server = MockServer::start().await;

    mount_device_start(&server, "tok-happy").await;

    // First poll is still pending; the second confirms. Sequencing keeps the
    // loop to two iterations (~2s of 1s sleeps) instead of spinning.
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/github-device/poll"))
        .and(body_json(serde_json::json!({"pollToken": "tok-happy"})))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "pending"})),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/github-device/poll"))
        .and(body_json(serde_json::json!({"pollToken": "tok-happy"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "confirmed",
            "license": "lic-headless",
            "refreshToken": "rt-headless",
            "expiresAt": "2099-01-01T00:00:00Z"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // whoami → POST /auth/verify with the saved licence; resolve identity.
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/verify"))
        .and(body_json(serde_json::json!({"token": "lic-headless"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "valid": true,
            "isEdict": false,
            "user": {"email": "headless@example.com", "plan": "pro"}
        })))
        .mount(&server)
        .await;

    // 1. Headless login completes end to end.
    let login = run(
        env.command(&server.uri(), &["--no-tui", "auth", "login"]),
        None,
    )
    .await;
    assert!(
        login.status.success(),
        "headless `anvil auth login` must complete: stderr=\n{}",
        stderr_of(&login)
    );

    // 2. Credentials were persisted to disk from the confirmed poll.
    let creds = env
        .saved_credentials()
        .expect("confirmed login must persist credentials.json");
    assert_eq!(creds["license"], "lic-headless");
    assert_eq!(creds["refreshToken"], "rt-headless");
    assert_eq!(creds["expiresAt"], "2099-01-01T00:00:00Z");

    // 3. `anvil auth whoami` resolves the identity through the broker.
    let whoami = run(
        env.command(&server.uri(), &["--no-tui", "--json", "auth", "whoami"]),
        None,
    )
    .await;
    assert!(
        whoami.status.success(),
        "whoami must resolve after a confirmed login: stderr=\n{}",
        stderr_of(&whoami)
    );
    let parsed: Value = serde_json::from_str(&String::from_utf8_lossy(&whoami.stdout))
        .expect("whoami --json must emit one JSON object");
    assert_eq!(parsed["email"], "headless@example.com");
    assert_eq!(parsed["plan"], "pro");
}

// ── slow_down is honoured (back off and continue, never fatal) ────────────

#[tokio::test]
async fn device_flow_e2e_slow_down_backs_off_then_confirms() {
    let env = TestEnv::new();
    let server = MockServer::start().await;

    mount_device_start(&server, "tok-slow").await;

    // First poll: 429 slow_down (retryAfter clamps to 1 in the client). The
    // pre-ADR-066 flow treated any 429 as fatal — this asserts the CLI backs
    // off and keeps polling rather than bailing.
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/github-device/poll"))
        .and(body_json(serde_json::json!({"pollToken": "tok-slow"})))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "error": "slow_down",
            "retryAfter": 1
        })))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/github-device/poll"))
        .and(body_json(serde_json::json!({"pollToken": "tok-slow"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "confirmed",
            "license": "lic-slow",
            "refreshToken": "rt-slow",
            "expiresAt": "2099-01-01T00:00:00Z"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let login = run(
        env.command(&server.uri(), &["--no-tui", "auth", "login"]),
        None,
    )
    .await;

    assert!(
        login.status.success(),
        "a slow_down (429) must be a back-off, not a fatal bail: stderr=\n{}",
        stderr_of(&login)
    );
    let creds = env
        .saved_credentials()
        .expect("login must still confirm and persist after a slow_down");
    assert_eq!(creds["license"], "lic-slow");
}

// ── expired terminal state ────────────────────────────────────────────────

#[tokio::test]
async fn device_flow_e2e_expired_fails_without_saving_credentials() {
    let env = TestEnv::new();
    let server = MockServer::start().await;

    mount_device_start(&server, "tok-expired").await;
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/github-device/poll"))
        .and(body_json(serde_json::json!({"pollToken": "tok-expired"})))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "expired"})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let login = run(
        env.command(&server.uri(), &["--no-tui", "auth", "login"]),
        None,
    )
    .await;

    assert!(
        !login.status.success(),
        "an expired device code must fail the login"
    );
    assert!(
        stderr_of(&login).contains("expired"),
        "expired must surface a clear message: stderr=\n{}",
        stderr_of(&login)
    );
    assert!(
        env.saved_credentials().is_none(),
        "a failed (expired) login must not persist credentials"
    );
}

// ── declined terminal state ───────────────────────────────────────────────

#[tokio::test]
async fn device_flow_e2e_declined_fails_without_saving_credentials() {
    let env = TestEnv::new();
    let server = MockServer::start().await;

    mount_device_start(&server, "tok-declined").await;
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/github-device/poll"))
        .and(body_json(serde_json::json!({"pollToken": "tok-declined"})))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "declined"})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let login = run(
        env.command(&server.uri(), &["--no-tui", "auth", "login"]),
        None,
    )
    .await;

    assert!(
        !login.status.success(),
        "a declined sign-in must fail the login"
    );
    assert!(
        stderr_of(&login).contains("declined"),
        "declined must surface a clear message: stderr=\n{}",
        stderr_of(&login)
    );
    assert!(
        env.saved_credentials().is_none(),
        "a failed (declined) login must not persist credentials"
    );
}

// ── --otp flow still works end to end ─────────────────────────────────────

#[tokio::test]
async fn device_flow_e2e_otp_request_verify_saves_credentials() {
    let env = TestEnv::new();
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/auth/otp/request"))
        .and(body_json(serde_json::json!({"email": "otp@example.com"})))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"message": "Code sent"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/otp/verify"))
        .and(body_json(
            serde_json::json!({"email": "otp@example.com", "code": "123456"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "license": "lic-otp",
            "refreshToken": "rt-otp",
            "expiresAt": "2099-06-01T00:00:00Z"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // The OTP flow prompts for email then code on stdin.
    let login = run(
        env.command(&server.uri(), &["--no-tui", "auth", "login", "--otp"]),
        Some("otp@example.com\n123456\n"),
    )
    .await;

    assert!(
        login.status.success(),
        "`anvil auth login --otp` must still complete: stderr=\n{}",
        stderr_of(&login)
    );
    let creds = env
        .saved_credentials()
        .expect("OTP login must persist credentials.json");
    assert_eq!(creds["license"], "lic-otp");
    assert_eq!(creds["refreshToken"], "rt-otp");
    assert_eq!(creds["email"], "otp@example.com");
}

/// Mock `/session/refresh` that rotates a token on first use and treats a
/// second use of the same token as family-revoking reuse — the server
/// contract that makes concurrent unguarded refresh fatal.
struct RotatingRefresh {
    used: Arc<Mutex<HashSet<String>>>,
}

impl Respond for RotatingRefresh {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
        let token = body
            .get("refreshToken")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let mut used = self.used.lock().expect("rotating-refresh lock");
        if !used.insert(token.clone()) {
            return ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": "Token reuse detected"
            }));
        }
        let next = format!("{token}.next");
        ResponseTemplate::new(200)
            .set_delay(Duration::from_millis(150))
            .set_body_json(serde_json::json!({
                "license": format!("lic-{next}"),
                "refreshToken": next,
                "expiresAt": "2099-12-31T23:59:59Z"
            }))
    }
}

// Concurrent `anvil auth refresh` against one rotating token must serialise
// the load → exchange → save transaction. Without an inter-process lock
// both processes submit the same token and the family is revoked.
#[tokio::test]
async fn concurrent_auth_refresh_serialises_rotating_token() {
    let env = TestEnv::new();
    let server = MockServer::start().await;
    let used = Arc::new(Mutex::new(HashSet::new()));

    Mock::given(method("POST"))
        .and(path("/api/v1/auth/session/refresh"))
        .respond_with(RotatingRefresh {
            used: Arc::clone(&used),
        })
        .mount(&server)
        .await;

    let creds_path = env.credentials_path();
    std::fs::create_dir_all(creds_path.parent().expect("credentials parent")).unwrap();
    std::fs::write(
        &creds_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "license": "lic-shared",
            "refreshToken": "rt-shared",
            "email": "user@example.com",
            "expiresAt": "2099-12-31T23:59:59Z",
            "isEdict": false
        }))
        .unwrap(),
    )
    .unwrap();

    let (first, second) = tokio::join!(
        run(
            env.command(&server.uri(), &["--no-tui", "--json", "auth", "refresh"]),
            None,
        ),
        run(
            env.command(&server.uri(), &["--no-tui", "--json", "auth", "refresh"]),
            None,
        ),
    );

    assert!(
        first.status.success(),
        "first concurrent refresh must succeed: stderr=\n{}",
        stderr_of(&first)
    );
    assert!(
        second.status.success(),
        "second concurrent refresh must succeed after waiting for the lock: stderr=\n{}",
        stderr_of(&second)
    );

    let submitted = used.lock().expect("rotating-refresh lock");
    assert!(
        submitted.contains("rt-shared"),
        "the original token must be exchanged exactly once: {submitted:?}"
    );
    // Serialised refresh either rotates once (waiter re-reads the new token
    // and exchanges that) or twice; reuse of the same token never occurs.
    assert!(
        submitted.len() <= 2,
        "expected at most two distinct tokens, got {submitted:?}"
    );

    let creds = env
        .saved_credentials()
        .expect("refresh must leave credentials on disk");
    let saved = creds["refreshToken"].as_str().expect("saved refresh token");
    assert_ne!(
        saved, "rt-shared",
        "the stored refresh token must have rotated"
    );
    assert!(
        saved.starts_with("rt-shared"),
        "saved token should be in the rotated family, got {saved}"
    );
}
