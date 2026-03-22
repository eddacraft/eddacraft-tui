#![allow(dead_code)]
use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct AdminArgs {
    #[command(subcommand)]
    command: AdminCommand,
}

#[derive(Debug, clap::Subcommand)]
enum AdminCommand {
    /// Approve a waitlisted user by email
    Approve {
        /// Email address to approve
        #[arg(conflicts_with = "batch", required_unless_present = "batch")]
        email: Option<String>,

        /// Approve the oldest N unapproved waitlist entries
        #[arg(long, conflicts_with = "email", required_unless_present = "email")]
        batch: Option<u32>,
    },
}

#[derive(Debug, Serialize)]
struct ApproveResult {
    approved: Vec<String>,
}

pub fn run(args: &AdminArgs, global: &GlobalArgs) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().context("creating tokio runtime")?;
    let admin_key = std::env::var("ANVIL_ADMIN_KEY")
        .context("ANVIL_ADMIN_KEY environment variable is required for admin commands")?;
    let client = crate::auth::client::AnvilClient::with_token(admin_key);

    match &args.command {
        AdminCommand::Approve { email, batch } => {
            if let Some(email) = email {
                rt.block_on(client.approve_user(email))?;
                let result = ApproveResult {
                    approved: vec![email.clone()],
                };
                if global.json {
                    crate::output::json::print(&result)?;
                } else {
                    println!();
                    println!("\u{2713} Approved {email}");
                }
            } else if let Some(count) = batch {
                let approved = rt.block_on(client.approve_batch(*count))?;
                let result = ApproveResult {
                    approved: approved.clone(),
                };
                if global.json {
                    crate::output::json::print(&result)?;
                } else {
                    println!();
                    if approved.is_empty() {
                        println!("No users to approve");
                    } else {
                        for email in &approved {
                            println!("\u{2713} Approved {email}");
                        }
                        println!();
                        println!("{} user(s) approved", approved.len());
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Debug, Parser)]
    struct Wrapper {
        #[command(flatten)]
        inner: AdminArgs,
    }

    #[test]
    fn args_rejects_approve_without_email_or_batch() {
        let err = Wrapper::try_parse_from(["test", "approve"]).unwrap_err();
        assert_ne!(err.exit_code(), 0);
    }

    #[test]
    fn args_parses_approve_email() {
        let w = Wrapper::try_parse_from(["test", "approve", "user@example.com"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_approve_batch() {
        let w = Wrapper::try_parse_from(["test", "approve", "--batch", "5"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_rejects_approve_with_email_and_batch() {
        let err = Wrapper::try_parse_from(["test", "approve", "user@example.com", "--batch", "5"])
            .unwrap_err();
        assert_ne!(err.exit_code(), 0);
    }
}
