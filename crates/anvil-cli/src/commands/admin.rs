use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;
use crate::output::AuthRequired;

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

    /// Invite a user to the beta (records a manual waitlist entry for audit tracking)
    Invite {
        /// Email address to invite
        email: String,

        /// Display name for the user
        #[arg(long)]
        name: Option<String>,

        /// Internal notes (e.g. reason for invite)
        #[arg(long)]
        notes: Option<String>,

        /// Return a raw access token instead of sending an invite email.
        /// Use for CI/service accounts.
        #[arg(long)]
        token: bool,
    },
}

#[derive(Debug, Serialize)]
struct ApproveResult {
    approved: Vec<String>,
}

#[derive(Debug, Serialize)]
struct InviteResult {
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
}

/// Resolve the admin API key from a raw `std::env::var` result.
///
/// Kept pure (no process env access) so unit tests can exercise every
/// branch without the `unsafe { std::env::set_var }` forbidden by the
/// crate-level `-F unsafe-code` lint.
fn resolve_admin_key(raw: Result<String, std::env::VarError>, json: bool) -> Result<String> {
    match raw {
        Ok(value) if !value.is_empty() => Ok(value),
        _ => {
            if json {
                eprintln!(
                    "{}",
                    serde_json::json!({
                        "error": "authentication_required",
                        "detail": "ANVIL_ADMIN_KEY environment variable is required for admin commands"
                    })
                );
            } else {
                eprintln!(
                    "Authentication required: set ANVIL_ADMIN_KEY to an admin token before running admin commands."
                );
            }
            Err(AuthRequired.into())
        }
    }
}

pub fn run(args: &AdminArgs, global: &GlobalArgs) -> Result<()> {
    let admin_key = resolve_admin_key(std::env::var("ANVIL_ADMIN_KEY"), global.json)?;
    let rt = tokio::runtime::Runtime::new().context("creating tokio runtime")?;
    let client = crate::auth::client::AnvilClient::with_token(admin_key)?;

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
            } else {
                unreachable!("clap requires either --email or --batch");
            }
        }
        AdminCommand::Invite {
            email,
            name,
            notes,
            token: token_only,
        } => {
            if *token_only {
                let raw_token = rt.block_on(client.invite_user_token(
                    email,
                    name.as_deref(),
                    notes.as_deref(),
                ))?;
                let result = InviteResult {
                    email: email.clone(),
                    token: Some(raw_token.clone()),
                };
                if global.json {
                    crate::output::json::print(&result)?;
                } else {
                    println!();
                    println!("\u{2713} Invited {email} (token mode)");
                    println!();
                    println!("Token: {raw_token}");
                    println!();
                    println!("This token is shown once and cannot be retrieved.");
                }
            } else {
                rt.block_on(client.invite_user(email, name.as_deref(), notes.as_deref()))?;
                let result = InviteResult {
                    email: email.clone(),
                    token: None,
                };
                if global.json {
                    crate::output::json::print(&result)?;
                } else {
                    println!();
                    println!("\u{2713} Invited {email}");
                    println!("  Invite email sent with device-code activation link.");
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

    #[test]
    fn args_parses_invite_email() {
        let w = Wrapper::try_parse_from(["test", "invite", "user@example.com"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_invite_with_name_and_notes() {
        let w = Wrapper::try_parse_from([
            "test",
            "invite",
            "user@example.com",
            "--name",
            "Jane Doe",
            "--notes",
            "VIP customer",
        ])
        .unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_invite_token_mode() {
        let w = Wrapper::try_parse_from(["test", "invite", "ci@example.com", "--token"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_rejects_invite_without_email() {
        let err = Wrapper::try_parse_from(["test", "invite"]).unwrap_err();
        assert_ne!(err.exit_code(), 0);
    }

    #[test]
    fn resolve_admin_key_missing_returns_auth_required() {
        let err = resolve_admin_key(Err(std::env::VarError::NotPresent), false).unwrap_err();
        assert!(err.is::<AuthRequired>(), "expected AuthRequired, got {err:?}");
    }

    #[test]
    fn resolve_admin_key_empty_returns_auth_required() {
        let err = resolve_admin_key(Ok(String::new()), false).unwrap_err();
        assert!(err.is::<AuthRequired>(), "expected AuthRequired, got {err:?}");
    }

    #[test]
    fn resolve_admin_key_present_returns_value() {
        let key = resolve_admin_key(Ok("secret-token".into()), false).unwrap();
        assert_eq!(key, "secret-token");
    }
}
