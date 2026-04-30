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
