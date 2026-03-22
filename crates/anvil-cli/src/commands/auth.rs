use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use serde::Serialize;

use crate::GlobalArgs;
use crate::auth::{credentials, device_flow};

#[derive(Debug, Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    command: AuthCommand,
}

#[derive(Debug, Subcommand)]
enum AuthCommand {
    /// Authenticate with the Anvil service
    Login {
        /// Use email OTP instead of device code flow
        #[arg(long)]
        otp: bool,
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
}

pub fn run(args: &AuthArgs, global: &GlobalArgs) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().context("creating tokio runtime")?;

    match &args.command {
        AuthCommand::Login { otp } => {
            if *otp {
                rt.block_on(device_flow::login_otp_flow())
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
                    let data = WhoamiData {
                        email: whoami.email,
                        plan: whoami.plan,
                        expires_at: creds.expires_at,
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
                    }
                    Ok(())
                }
                Err(e)
                    if e.to_string().contains("request") || e.to_string().contains("connect") =>
                {
                    let data = WhoamiData {
                        email: creds.email.unwrap_or_else(|| "unknown".to_string()),
                        plan: None,
                        expires_at: creds.expires_at,
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
    fn args_parses_logout() {
        let w = Wrapper::try_parse_from(["test", "logout"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_whoami() {
        let w = Wrapper::try_parse_from(["test", "whoami"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }
}
