use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use serde::Serialize;

use crate::GlobalArgs;
use crate::auth::{credentials, device_flow};
use crate::feature_flags::evaluate_cli_licence_gate;

#[derive(Debug, Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommand,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Authenticate with the Anvil service
    Login {
        /// Use email OTP instead of device code flow
        #[arg(long)]
        otp: bool,

        /// Redeem a revokable early-access edict
        #[arg(long, conflicts_with = "otp")]
        edict: bool,
    },
    /// Remove stored credentials
    Logout,
    /// Show current authenticated user
    Whoami,
    /// Exchange the stored refresh token for a fresh licence without
    /// re-running the device-code flow. Useful when the JWT has lapsed
    /// but the refresh token is still valid (it's good for 90 days).
    // NB: the "90 days" in the help text above mirrors `REFRESH_WINDOW_DAYS`
    // (and `docs/architecture/auth-as-built.md`). clap help is a literal
    // doc-comment so it can't interpolate the constant — if the refresh
    // window changes, update all three sites together.
    Refresh,
}

#[derive(Debug, Serialize)]
struct WhoamiData {
    email: String,
    plan: Option<String>,
    expires_at: Option<String>,
    /// FLAGS-008: shared licence-gate resolution for this session (enabled|disabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    licence_gate: Option<String>,
}

/// The licence model issues short-lived access tokens (JWTs) backed by a
/// long-lived refresh token. The refresh token is valid for this many days;
/// `anvil auth refresh` exchanges it for a fresh access token (and rotates
/// the refresh token) without a full device-flow re-login. Surfaced in the
/// refresh output so a healthy refresh is not mistaken for a short-lived-only
/// session (GH #1921).
///
/// This is a client-side mirror of the server's refresh-token lifetime, not a
/// value read from the refresh response (which carries only the access-token
/// expiry). Three sites encode this window and must move together if the
/// server changes it: this constant, the `AuthCommand::Refresh` `--help`
/// doc-comment ("good for 90 days"), and `docs/architecture/auth-as-built.md`
/// (§"7-day access / 90-day refresh").
const REFRESH_WINDOW_DAYS: u32 = 90;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RefreshData {
    refreshed: bool,
    email: Option<String>,
    expires_at: Option<String>,
}

/// Render the human-readable lines for a successful `anvil auth refresh`.
///
/// Shows the new access-token expiry AND the refresh window. Before GH
/// #1921 the output printed only a bare `Expires: <~7d>` line, which
/// contradicted the `--help` text advertising a 90-day window and made a
/// correct refresh look like it only bought 7 days.
fn format_refresh_human(data: &RefreshData) -> String {
    use std::fmt::Write as _;
    let mut out = String::from("Session refreshed\n");
    if let Some(email) = &data.email {
        let _ = writeln!(out, "  Email:          {email}");
    }
    if let Some(expires) = &data.expires_at {
        let _ = writeln!(out, "  Access expires: {expires}");
    }
    let _ = writeln!(
        out,
        "  You can refresh without re-login for up to {REFRESH_WINDOW_DAYS} days."
    );
    out
}

pub fn run(args: &AuthArgs, global: &GlobalArgs) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().context("creating tokio runtime")?;

    match &args.command {
        AuthCommand::Login { otp, edict } => {
            if *otp {
                rt.block_on(device_flow::login_otp_flow())
            } else if *edict {
                rt.block_on(device_flow::login_edict_flow())
            } else {
                rt.block_on(device_flow::login_device_flow())
            }
        }
        AuthCommand::Logout => {
            credentials::clear()?;
            if global.json {
                crate::output::json::print(&serde_json::json!({"logged_out": true}))?;
            } else {
                println!("Credentials removed");
            }
            Ok(())
        }
        AuthCommand::Refresh => {
            let new_creds = rt.block_on(device_flow::refresh_command())?;
            let data = RefreshData {
                refreshed: true,
                email: new_creds.email.clone(),
                expires_at: new_creds.expires_at.clone(),
            };
            if global.json {
                crate::output::json::print(&data)?;
            } else {
                print!("{}", format_refresh_human(&data));
            }
            Ok(())
        }
        AuthCommand::Whoami => {
            let creds = credentials::load()?.context("Not authenticated. Run: anvil auth login")?;

            let client = crate::auth::client::AnvilClient::with_token(creds.license.clone())?;

            match rt.block_on(client.whoami()) {
                Ok(whoami) => {
                    let gate =
                        evaluate_cli_licence_gate(whoami.email.as_str(), whoami.plan.as_deref());
                    let data = WhoamiData {
                        email: whoami.email,
                        plan: whoami.plan,
                        expires_at: creds.expires_at,
                        licence_gate: Some(gate.variant),
                    };
                    if global.json {
                        crate::output::json::print(&data)?;
                    } else {
                        println!();
                        println!("Authenticated");
                        println!("  Email:   {}", data.email);
                        if let Some(plan) = &data.plan {
                            println!("  Plan:    {plan}");
                        }
                        if let Some(expires) = &data.expires_at {
                            println!("  Expires: {expires}");
                        }
                        if let Some(gate) = &data.licence_gate {
                            println!("  Gate:    cli.licence-gate = {gate}");
                        }
                    }
                    Ok(())
                }
                Err(e) if is_network_error(&e) => {
                    let data = WhoamiData {
                        email: creds.email.unwrap_or_else(|| "unknown".to_string()),
                        plan: None,
                        expires_at: creds.expires_at,
                        licence_gate: None,
                    };
                    if global.json {
                        crate::output::json::print(&data)?;
                    } else {
                        println!();
                        println!("Authenticated (offline)");
                        println!("  Email: {}", data.email);
                    }
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
    }
}

/// Check whether an error is a network/connectivity failure (timeout, DNS,
/// connection refused) so we can fall back to offline credential display.
fn is_network_error(err: &anyhow::Error) -> bool {
    for cause in err.chain() {
        if let Some(re) = cause.downcast_ref::<reqwest::Error>()
            && (re.is_connect() || re.is_timeout())
        {
            return true;
        }
    }
    false
}

// -------------------------------------------------------------------------
// Top-level aliases (anvil login, anvil logout, anvil whoami)
// -------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct LoginArgs {
    /// Use email OTP instead of device code flow
    #[arg(long)]
    otp: bool,

    /// Redeem a revokable early-access edict
    #[arg(long, conflicts_with = "otp")]
    edict: bool,
}

#[derive(Debug, Args)]
pub struct LogoutArgs {}

#[derive(Debug, Args)]
pub struct WhoamiArgs {}

pub fn run_login(args: &LoginArgs, global: &GlobalArgs) -> Result<()> {
    let auth_args = AuthArgs {
        command: AuthCommand::Login {
            otp: args.otp,
            edict: args.edict,
        },
    };
    run(&auth_args, global)
}

pub fn run_logout(_args: &LogoutArgs, global: &GlobalArgs) -> Result<()> {
    let auth_args = AuthArgs {
        command: AuthCommand::Logout,
    };
    run(&auth_args, global)
}

pub fn run_whoami(_args: &WhoamiArgs, global: &GlobalArgs) -> Result<()> {
    let auth_args = AuthArgs {
        command: AuthCommand::Whoami,
    };
    run(&auth_args, global)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        inner: AuthArgs,
    }

    #[test]
    fn args_parses_login() {
        let w = Wrapper::try_parse_from(["test", "login"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_login_otp() {
        let w = Wrapper::try_parse_from(["test", "login", "--otp"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_login_edict() {
        let w = Wrapper::try_parse_from(["test", "login", "--edict"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_rejects_login_otp_and_edict_together() {
        assert!(Wrapper::try_parse_from(["test", "login", "--otp", "--edict"]).is_err());
    }

    #[test]
    fn args_parses_logout() {
        let w = Wrapper::try_parse_from(["test", "logout"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_whoami() {
        let w = Wrapper::try_parse_from(["test", "whoami"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_refresh() {
        let w = Wrapper::try_parse_from(["test", "refresh"]).unwrap();
        assert!(matches!(w.inner.command, AuthCommand::Refresh));
    }

    #[test]
    fn args_rejects_refresh_with_flags() {
        // Refresh takes no flags; surface that as a parse error so a typo
        // like `anvil auth refresh --otp` doesn't silently fall through.
        assert!(Wrapper::try_parse_from(["test", "refresh", "--otp"]).is_err());
    }

    #[test]
    fn refresh_human_output_surfaces_access_expiry_and_refresh_window() {
        // GH #1921: a correct refresh buys a short-lived access token but
        // renews the long-lived refresh window. The output must surface
        // BOTH so a healthy refresh is not mistaken for a 7-day-only
        // session — the `--help` text already advertises the 90-day
        // window, and a bare `Expires: <7d>` line contradicted it.
        let data = RefreshData {
            refreshed: true,
            email: Some("user@example.com".to_string()),
            expires_at: Some("2026-06-01T00:00:00Z".to_string()),
        };
        let out = format_refresh_human(&data);
        assert!(out.contains("Session refreshed"), "got: {out}");
        assert!(out.contains("user@example.com"), "got: {out}");
        assert!(
            out.contains("2026-06-01T00:00:00Z"),
            "access expiry must still be shown, got: {out}"
        );
        assert!(
            out.contains(&REFRESH_WINDOW_DAYS.to_string()),
            "the 90-day refresh window must be surfaced, got: {out}"
        );
        assert!(
            out.to_lowercase().contains("without re-login"),
            "must explain you can refresh without re-login, got: {out}"
        );
    }

    #[test]
    fn refresh_human_output_omits_email_line_when_absent() {
        let data = RefreshData {
            refreshed: true,
            email: None,
            expires_at: Some("2026-06-01T00:00:00Z".to_string()),
        };
        let out = format_refresh_human(&data);
        assert!(!out.contains("Email:"), "got: {out}");
        assert!(out.contains("Session refreshed"), "got: {out}");
    }

    #[test]
    fn refresh_human_output_handles_missing_access_expiry() {
        // The server response is the only source of the access expiry; if
        // it is absent we must not fabricate a date. The 90-day refresh
        // window is a known client-side constant, so that line still shows.
        let data = RefreshData {
            refreshed: true,
            email: Some("user@example.com".to_string()),
            expires_at: None,
        };
        let out = format_refresh_human(&data);
        assert!(out.contains("Session refreshed"), "got: {out}");
        assert!(!out.contains("Access expires:"), "got: {out}");
        assert!(
            out.contains(&REFRESH_WINDOW_DAYS.to_string()),
            "refresh window line must still render, got: {out}"
        );
    }

    // --- Top-level alias tests ---

    #[derive(Parser)]
    struct LoginWrapper {
        #[command(flatten)]
        inner: LoginArgs,
    }

    #[derive(Parser)]
    struct LogoutWrapper {
        #[command(flatten)]
        inner: LogoutArgs,
    }

    #[derive(Parser)]
    struct WhoamiWrapper {
        #[command(flatten)]
        inner: WhoamiArgs,
    }

    #[test]
    fn alias_login_parses() {
        let w = LoginWrapper::try_parse_from(["test"]).unwrap();
        assert!(!w.inner.otp);
        assert!(!w.inner.edict);
    }

    #[test]
    fn alias_login_otp_parses() {
        let w = LoginWrapper::try_parse_from(["test", "--otp"]).unwrap();
        assert!(w.inner.otp);
    }

    #[test]
    fn alias_login_edict_parses() {
        let w = LoginWrapper::try_parse_from(["test", "--edict"]).unwrap();
        assert!(w.inner.edict);
    }

    #[test]
    fn alias_logout_parses() {
        let _ = LogoutWrapper::try_parse_from(["test"]).unwrap();
    }

    #[test]
    fn alias_whoami_parses() {
        let _ = WhoamiWrapper::try_parse_from(["test"]).unwrap();
    }
}
