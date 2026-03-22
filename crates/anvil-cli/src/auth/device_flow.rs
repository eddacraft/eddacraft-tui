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
    sent: bool,
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

fn api_url() -> String {
    let raw =
        std::env::var("ANVIL_API_URL").unwrap_or_else(|_| "https://api.eddacraft.ai".to_string());
    raw.trim_end_matches('/').to_string()
}

fn prompt_input(label: &str) -> Result<String> {
    eprint!("{label}");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .context("reading stdin")?;
    Ok(input.trim().to_string())
}

pub async fn login_device_flow() -> Result<()> {
    let url = api_url();
    let email = prompt_input("Email: ")?;
    if email.is_empty() {
        bail!("Email is required");
    }

    eprintln!("Starting device code flow...");

    let client = reqwest::Client::new();
    let start: DeviceStartResponse = client
        .post(format!("{url}/api/v1/auth/device/start"))
        .json(&DeviceStartRequest { email: &email })
        .send()
        .await
        .context("device code start request")?
        .error_for_status()
        .context("device code start response")?
        .json()
        .await
        .context("parsing device code start response")?;

    eprintln!();
    eprintln!("To authenticate, open this URL:");
    eprintln!("  {}", start.verification_url);
    eprintln!();
    eprintln!("And enter code: {}", start.user_code);
    eprintln!();
    eprintln!("Waiting for confirmation...");

    let poll_interval = std::time::Duration::from_secs(5);
    let max_attempts = (start.expires_in / 5).max(1);

    for _ in 0..max_attempts {
        tokio::time::sleep(poll_interval).await;

        let poll: DevicePollResponse = client
            .post(format!("{url}/api/v1/auth/device/poll"))
            .json(&DevicePollRequest {
                poll_token: &start.poll_token,
            })
            .send()
            .await
            .context("device poll request")?
            .error_for_status()
            .context("device poll response")?
            .json()
            .await
            .context("parsing device poll response")?;

        match poll.status.as_str() {
            "confirmed" => {
                let token = poll.license.context("server returned no license")?;
                let refresh = poll
                    .refresh_token
                    .context("server returned no refresh token")?;
                let expires = poll.expires_at.context("server returned no expiry")?;

                credentials::save(&Credentials {
                    token: token.clone(),
                    refresh_token: Some(refresh),
                    email: Some(email.clone()),
                    expires_at: Some(expires),
                })?;

                eprintln!();
                eprintln!("\u{2713} Authenticated as {email}");
                let path = credentials::credentials_path();
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
    let url = api_url();
    let email = prompt_input("Email: ")?;
    if email.is_empty() {
        bail!("Email is required");
    }

    eprintln!("Requesting verification code...");

    let client = reqwest::Client::new();
    let resp: OtpRequestResponse = client
        .post(format!("{url}/api/v1/auth/otp/send"))
        .json(&OtpSendRequest { email: &email })
        .send()
        .await
        .context("OTP send request")?
        .error_for_status()
        .context("OTP send response")?
        .json()
        .await
        .context("parsing OTP send response")?;

    let _ = resp.sent;

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

        match client
            .post(format!("{url}/api/v1/auth/otp/verify"))
            .json(&OtpVerifyRequest {
                email: &email,
                code: &code,
            })
            .send()
            .await
        {
            Ok(res) => match res.error_for_status() {
                Ok(ok) => {
                    let result: OtpVerifyResponse =
                        ok.json().await.context("parsing OTP verify response")?;

                    credentials::save(&Credentials {
                        token: result.license.clone(),
                        refresh_token: Some(result.refresh_token),
                        email: Some(email.clone()),
                        expires_at: Some(result.expires_at),
                    })?;

                    eprintln!();
                    eprintln!("\u{2713} Authenticated as {email}");
                    let path = credentials::credentials_path();
                    eprintln!("  Credentials saved to {}", path.display());
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("Verification failed: {e}");
                    if attempt < 3 {
                        eprintln!("{} attempt(s) remaining", 3 - attempt);
                    }
                }
            },
            Err(e) => {
                eprintln!("Request failed: {e}");
                if attempt < 3 {
                    eprintln!("{} attempt(s) remaining", 3 - attempt);
                }
            }
        }
    }

    bail!("Maximum attempts reached. Please try again.");
}
