use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::GlobalArgs;
use crate::auth::client::{
    ApiError, AuditResponse, EmailUpdateResponse, MigrationPreviewResponse, MigrationSendResponse,
    RevokeResponse, ShowUserResponse, WaitlistResponse,
};
use crate::output::AuthRequired;

#[derive(Debug, Args)]
pub struct AdminArgs {
    #[command(subcommand)]
    command: AdminCommand,
}

#[derive(Debug, clap::Subcommand)]
enum AdminCommand {
    /// List waitlist entries
    List {
        /// Filter by approval status
        #[arg(long, value_enum)]
        status: Option<WaitlistStatus>,

        /// Filter by signup source
        #[arg(long, value_enum)]
        source: Option<WaitlistSource>,

        /// Maximum entries to return
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..=200))]
        limit: Option<u32>,

        /// Entries to skip
        #[arg(long, value_parser = clap::value_parser!(u32).range(0..))]
        offset: Option<u32>,
    },

    /// Show a user, tokens, and recent audit entries
    Show {
        /// Email address to inspect
        email: String,
    },

    /// Revoke all tokens for an email or one raw token
    Revoke {
        /// Email address whose tokens should be revoked
        #[arg(conflicts_with = "token", required_unless_present = "token")]
        email: Option<String>,

        /// Raw token to revoke
        #[arg(long, conflicts_with = "email", required_unless_present = "email")]
        token: Option<String>,

        /// Skip the confirmation prompt
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// List admin audit entries
    Audit {
        /// Filter by action
        #[arg(long)]
        action: Option<String>,

        /// Filter by actor
        #[arg(long = "filter-actor")]
        filter_actor: Option<String>,

        /// Maximum entries to return
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..=200))]
        limit: Option<u32>,

        /// Entries to skip
        #[arg(long, value_parser = clap::value_parser!(u32).range(0..))]
        offset: Option<u32>,
    },

    /// Preview or send waitlist migration email
    #[command(name = "send-migration")]
    SendMigration {
        /// Filter recipients by source
        #[arg(long, value_enum, default_value_t = MigrationSource::Import)]
        source: MigrationSource,

        /// Maximum recipients to preview or send
        #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=100))]
        limit: u32,

        /// Actually send after a fresh preview; default is dry-run only
        #[arg(long = "no-dry-run", default_value_t = false)]
        no_dry_run: bool,

        /// Skip the confirmation prompt for real sends
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Update a beta user's email address
    #[command(name = "email-update")]
    EmailUpdate {
        /// Current email address
        current_email: String,

        /// New email address
        new_email: String,
    },

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

        /// Issue a revokable early-access edict.
        #[arg(long)]
        edict: bool,
    },

    /// Configure how admin commands retrieve the admin API key
    Auth {
        #[command(subcommand)]
        command: AdminAuthCommand,
    },
}

#[derive(Debug, clap::Subcommand)]
enum AdminAuthCommand {
    /// Store an admin credential for future admin commands
    Set {
        /// Credential source backend
        #[arg(value_enum)]
        source: AdminCredentialSourceKind,

        /// For `1password`: an `op://Vault/item/field` reference (not secret).
        /// For `key`: the admin key itself, or `-` to read it from stdin so it
        /// never lands in your shell history.
        reference: String,
    },

    /// Show the configured admin credential source without revealing the key
    Status,

    /// Remove the configured admin credential source
    Unset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum AdminCredentialSourceKind {
    /// Resolve the key from a 1Password reference via the `op` CLI
    #[value(name = "1password")]
    OnePassword,
    /// Store the admin key directly in the local config (mode 0600), so no
    /// per-shell `export ANVIL_ADMIN_KEY` is needed
    #[value(name = "key")]
    Key,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AdminCredentialConfig {
    source: String,
    reference: String,
}

impl AdminCredentialConfig {
    fn one_password(reference: impl Into<String>) -> Self {
        Self {
            source: "1password".to_string(),
            reference: reference.into(),
        }
    }

    fn key(value: impl Into<String>) -> Self {
        Self {
            source: "key".to_string(),
            reference: value.into(),
        }
    }
}

/// Render a credential reference for display without leaking a secret.
///
/// A 1Password reference (`op://…`) is not itself a secret, so it is shown
/// verbatim. For the `key` source the `reference` field IS the admin key, so
/// it is masked to a trailing fingerprint (`****` + last 4) — enough to tell
/// which key is configured, never enough to use it.
fn redact_admin_reference(source: &str, reference: &str) -> String {
    if source == "key" {
        let tail: String = reference
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if reference.chars().count() > 4 {
            format!("****{tail}")
        } else {
            "****".to_string()
        }
    } else {
        reference.to_string()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AdminAuthStatus {
    configured: bool,
    source: Option<String>,
    reference: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum WaitlistStatus {
    Pending,
    Approved,
    All,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum WaitlistSource {
    Manual,
    Website,
    Import,
    All,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum MigrationSource {
    Import,
    Website,
    Manual,
}

impl std::fmt::Display for MigrationSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Import => "import",
            Self::Website => "website",
            Self::Manual => "manual",
        })
    }
}

fn waitlist_status_value(value: WaitlistStatus) -> &'static str {
    match value {
        WaitlistStatus::Pending => "pending",
        WaitlistStatus::Approved => "approved",
        WaitlistStatus::All => "all",
    }
}

fn waitlist_source_value(value: WaitlistSource) -> &'static str {
    match value {
        WaitlistSource::Manual => "manual",
        WaitlistSource::Website => "website",
        WaitlistSource::Import => "import",
        WaitlistSource::All => "all",
    }
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
/// Kept env-independent (no direct process env access) so unit tests can
/// exercise every branch without the `unsafe { std::env::set_var }`
/// forbidden by the crate-level `unsafe_code` lint.
fn admin_credential_config_path() -> Result<PathBuf> {
    // Route through the credentials-dir convention rather than raw
    // `dirs::config_dir()`: the admin key lives under the same roof as the
    // login credentials — honouring the DISTRIB-006 `ANVIL_HOME` re-rooting
    // (a pre-release candidate must not read the production admin key
    // either) and the deliberate XDG-on-macOS policy — and it drops the hard
    // dependency on the Windows known-folder API, which fails in headless /
    // service sessions (observed as `could not determine user config
    // directory` exit-1s on the Windows cross-compile smoke leg). Default
    // Linux and Windows paths are unchanged (`~/.config/anvil/` /
    // `%APPDATA%/anvil/`).
    let credentials_dir = crate::auth::credentials::credentials_dir()
        .context("could not determine user config directory")?;
    Ok(credentials_dir.join("admin-auth.json"))
}

fn load_admin_credential_config(path: &Path) -> Result<Option<AdminCredentialConfig>> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(err).with_context(|| {
                format!(
                    "failed to read admin credential source at {}",
                    path.display()
                )
            });
        }
    };
    let config: AdminCredentialConfig = serde_json::from_str(&raw).with_context(|| {
        format!(
            "failed to parse admin credential source at {}; run `anvil admin auth unset` and configure it again",
            path.display()
        )
    })?;
    Ok(Some(config))
}

fn save_admin_credential_config(path: &Path, config: &AdminCredentialConfig) -> Result<()> {
    if config.reference.trim().is_empty() {
        bail!("admin credential source reference cannot be empty");
    }
    let parent = path
        .parent()
        .context("admin credential source path has no parent")?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    let raw = serde_json::to_string_pretty(config)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        use std::os::unix::fs::PermissionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .with_context(|| {
                format!(
                    "failed to write admin credential source at {}",
                    path.display()
                )
            })?;
        file.write_all(raw.as_bytes()).with_context(|| {
            format!(
                "failed to write admin credential source at {}",
                path.display()
            )
        })?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).with_context(|| {
            format!(
                "failed to restrict admin credential source permissions at {}",
                path.display()
            )
        })?;
    }

    #[cfg(not(unix))]
    fs::write(path, raw).with_context(|| {
        format!(
            "failed to write admin credential source at {}",
            path.display()
        )
    })?;

    Ok(())
}

fn read_1password_reference(reference: &str) -> Result<String> {
    let output = Command::new("op")
        .args(["read", reference])
        .output()
        .context("failed to run `op read`; install and sign in to the 1Password CLI")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "`op read` failed{}{}",
            if stderr.trim().is_empty() { "" } else { ": " },
            stderr.trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string())
}

fn print_auth_required(json: bool, detail: &str) {
    if json {
        // CIB-052: the envelope only exists under `--json`, and structured
        // output belongs on stdout (stream policy,
        // `docs/guides/cli-output-streams.md`) — same contract CIB-049
        // applied to the main pre-dispatch auth gate. Exit-code routing is
        // unchanged (`AuthRequired` → `EXIT_AUTH_REQUIRED`).
        println!(
            "{}",
            serde_json::json!({
                "error": "authentication_required",
                "detail": detail
            })
        );
    } else {
        eprintln!("Authentication required: {detail}");
    }
}

fn resolve_admin_key_with_config<F>(
    raw: Result<String, std::env::VarError>,
    json: bool,
    config_path: &Path,
    read_1password: F,
) -> Result<String>
where
    F: FnOnce(&str) -> Result<String>,
{
    match raw {
        Ok(value) if !value.is_empty() => Ok(value),
        Err(std::env::VarError::NotUnicode(_)) => {
            print_auth_required(
                json,
                "ANVIL_ADMIN_KEY is set but is not valid Unicode; unset it or set it to a valid admin token before running admin commands.",
            );
            Err(AuthRequired.into())
        }
        _ => match load_admin_credential_config(config_path)? {
            Some(config) if config.source == "key" => {
                // trim()-empty (whitespace-only, e.g. a hand-edited config)
                // is rejected too, matching save_admin_credential_config.
                if config.reference.trim().is_empty() {
                    print_auth_required(
                        json,
                        "the stored admin key is empty; run `anvil admin auth set key -` again or `anvil admin auth unset`.",
                    );
                    Err(AuthRequired.into())
                } else {
                    Ok(config.reference)
                }
            }
            Some(config) if config.source == "1password" => match read_1password(&config.reference)
            {
                Ok(value) if !value.is_empty() => Ok(value),
                Ok(_) => {
                    print_auth_required(
                        json,
                        "configured 1Password admin credential resolved to an empty value; check the item reference or run `anvil admin auth set 1password <reference>` again.",
                    );
                    Err(AuthRequired.into())
                }
                Err(err) => {
                    print_auth_required(
                        json,
                        &format!(
                            "could not read configured 1Password admin credential ({err}); run `op signin`, set ANVIL_ADMIN_KEY, or run `anvil admin auth unset`."
                        ),
                    );
                    Err(AuthRequired.into())
                }
            },
            Some(config) => {
                print_auth_required(
                    json,
                    &format!(
                        "unsupported admin credential source `{}`; run `anvil admin auth unset` and configure it again.",
                        config.source
                    ),
                );
                Err(AuthRequired.into())
            }
            None => {
                print_auth_required(
                    json,
                    "no admin credential configured. Run `anvil admin auth set key -` to store it once (paste the key on stdin), or `anvil admin auth set 1password <op://reference>`, or set ANVIL_ADMIN_KEY.",
                );
                Err(AuthRequired.into())
            }
        },
    }
}

fn resolve_admin_key(raw: Result<String, std::env::VarError>, json: bool) -> Result<String> {
    let config_path = admin_credential_config_path()?;
    resolve_admin_key_with_config(raw, json, &config_path, read_1password_reference)
}

fn run_admin_auth(command: &AdminAuthCommand, global: &GlobalArgs) -> Result<()> {
    let path = admin_credential_config_path()?;
    match command {
        AdminAuthCommand::Set { source, reference } => {
            let config = match source {
                AdminCredentialSourceKind::OnePassword => {
                    AdminCredentialConfig::one_password(reference)
                }
                AdminCredentialSourceKind::Key => {
                    // `-` reads the key from stdin so the secret stays out of
                    // shell history and the process list.
                    let value = if reference == "-" {
                        let mut buf = String::new();
                        let read = io::stdin()
                            .read_line(&mut buf)
                            .context("failed to read admin key from stdin")?;
                        if read == 0 {
                            bail!(
                                "no admin key on stdin (EOF). Run `anvil admin auth set key -` and paste the key, or pipe it: `printf %s \"$KEY\" | anvil admin auth set key -`."
                            );
                        }
                        let trimmed = buf.trim().to_string();
                        if trimmed.is_empty() {
                            bail!("admin key read from stdin was empty; nothing stored.");
                        }
                        trimmed
                    } else {
                        reference.clone()
                    };
                    AdminCredentialConfig::key(value)
                }
            };
            save_admin_credential_config(&path, &config)?;
            let shown = redact_admin_reference(&config.source, &config.reference);
            if global.json {
                crate::output::json::print(&serde_json::json!({
                    "configured": true,
                    "source": config.source,
                    "reference": shown,
                    "path": path,
                }))?;
            } else {
                println!("Admin credential source configured");
                println!("  Source:    {}", config.source);
                println!("  Reference: {shown}");
                println!("  Path:      {}", path.display());
            }
        }
        AdminAuthCommand::Status => {
            let config = load_admin_credential_config(&path)?;
            let status = AdminAuthStatus {
                configured: config.is_some(),
                source: config.as_ref().map(|config| config.source.clone()),
                reference: config
                    .as_ref()
                    .map(|config| redact_admin_reference(&config.source, &config.reference)),
            };
            if global.json {
                crate::output::json::print(&status)?;
            } else if status.configured {
                println!("Admin credential source configured");
                println!(
                    "  Source:    {}",
                    status.source.as_deref().unwrap_or("unknown")
                );
                println!(
                    "  Reference: {}",
                    status.reference.as_deref().unwrap_or("unknown")
                );
            } else {
                println!("No admin credential source configured");
                println!("Run one of:");
                println!(
                    "  anvil admin auth set key -                  # paste the key on stdin, stored locally (0600)"
                );
                println!("  anvil admin auth set 1password op://<vault>/<item>/<field>");
                println!(
                    "(or `set key <your-admin-key>` directly, though that puts the key in shell history)"
                );
            }
        }
        AdminAuthCommand::Unset => {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(err).with_context(|| {
                        format!(
                            "failed to remove admin credential source at {}",
                            path.display()
                        )
                    });
                }
            }
            if global.json {
                crate::output::json::print(&serde_json::json!({"configured": false}))?;
            } else {
                println!("Admin credential source removed");
            }
        }
    }
    Ok(())
}

fn render_json<T: Serialize>(value: &T, json: bool) -> Result<bool> {
    if json {
        crate::output::json::print(value)?;
    }
    Ok(json)
}

#[allow(clippy::too_many_lines)]
pub fn run(args: &AdminArgs, global: &GlobalArgs) -> Result<()> {
    if let AdminCommand::Auth { command } = &args.command {
        return run_admin_auth(command, global);
    }

    let admin_key = resolve_admin_key(
        std::env::var(crate::feature_flags::ADMIN_KEY_ENV_VAR),
        global.json,
    )?;
    let rt = tokio::runtime::Runtime::new().context("creating tokio runtime")?;
    let client = crate::auth::client::AnvilClient::with_token(admin_key)?;

    match &args.command {
        AdminCommand::List {
            status,
            source,
            limit,
            offset,
        } => {
            let result = rt.block_on(client.list_waitlist(
                status.map(waitlist_status_value),
                source.map(waitlist_source_value),
                *limit,
                *offset,
            ))?;
            if !render_json(&result, global.json)? {
                print_waitlist(&result);
            }
        }
        AdminCommand::Show { email } => {
            let result = rt.block_on(client.get_user(email))?;
            if result.audit_error {
                eprintln!("warning: audit lookup failed; user and tokens still shown.");
            }
            if !render_json(&result, global.json)? {
                print_user(&result);
            }
        }
        AdminCommand::Revoke { email, token, yes } => {
            let target = email.as_ref().map_or_else(
                || "the supplied token".to_string(),
                |email| format!("all tokens for {email}"),
            );
            if !*yes && !confirm_revoke(&target, global.json)? {
                return Ok(());
            }
            let result = if let Some(email) = email {
                rt.block_on(client.revoke_email(email))?
            } else if let Some(token) = token {
                rt.block_on(client.revoke_token(token))?
            } else {
                unreachable!("clap requires email or --token")
            };
            if !render_json(&result, global.json)? {
                print_revoke(&result, email.as_deref().unwrap_or("token"));
            }
        }
        AdminCommand::Audit {
            action,
            filter_actor,
            limit,
            offset,
        } => {
            let result = rt.block_on(client.list_audit(
                action.as_deref(),
                filter_actor.as_deref(),
                *limit,
                *offset,
            ))?;
            if !render_json(&result, global.json)? {
                print_audit(&result);
            }
        }
        AdminCommand::SendMigration {
            source,
            limit,
            no_dry_run,
            yes,
        } => {
            let source = source.to_string();
            if !*no_dry_run {
                let preview = rt.block_on(client.send_migration_dry_run(&source, *limit))?;
                if !render_json(&preview, global.json)? {
                    print_migration_preview(&preview);
                }
                return Ok(());
            }
            if !*yes && !io::stdin().is_terminal() {
                bail!("refusing to send migration without --yes in a non-interactive session");
            }
            let preview = rt.block_on(client.send_migration_dry_run(&source, *limit))?;
            if preview.count == 0 {
                if global.json {
                    render_json(
                        &MigrationSendResponse {
                            source: source.clone(),
                            total: 0,
                            sent: 0,
                            failed: 0,
                            results: Vec::new(),
                        },
                        true,
                    )?;
                } else {
                    println!("No recipients match the filter. Nothing to send.");
                }
                return Ok(());
            }
            if !*yes && !confirm_send(&preview, global.json)? {
                return Ok(());
            }
            let result = rt
                .block_on(client.send_migration_commit(&source, *limit, &preview.preview_token))
                .map_err(rewrite_migration_error)?;
            if !render_json(&result, global.json)? {
                print_migration_send(&result);
            }
            if result.failed > 0 {
                bail!(
                    "{} of {} recipient(s) failed to send",
                    result.failed,
                    result.total
                );
            }
        }
        AdminCommand::EmailUpdate {
            current_email,
            new_email,
        } => {
            let result = rt.block_on(client.update_user_email(current_email, new_email))?;
            if !render_json(&result, global.json)? {
                print_email_update(&result);
            }
        }
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
            edict,
        } => {
            if *token_only || *edict {
                let raw_token = rt.block_on(client.invite_user_token(
                    email,
                    name.as_deref(),
                    notes.as_deref(),
                    *edict,
                ))?;
                let result = InviteResult {
                    email: email.clone(),
                    token: Some(raw_token.clone()),
                };
                if global.json {
                    crate::output::json::print(&result)?;
                } else {
                    println!();
                    let mode = if *edict { "edict" } else { "token" };
                    println!("\u{2713} Invited {email} ({mode} mode)");
                    println!();
                    if *edict {
                        println!("Edict: {raw_token}");
                    } else {
                        println!("Token: {raw_token}");
                    }
                    println!();
                    println!("This value is shown once and cannot be retrieved.");
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
        AdminCommand::Auth { .. } => unreachable!("admin auth returned before API client setup"),
    }

    Ok(())
}

fn confirm_revoke(target: &str, json: bool) -> Result<bool> {
    if !io::stdin().is_terminal() {
        bail!("refusing to revoke without --yes in a non-interactive session");
    }
    eprintln!("About to revoke {target}.");
    eprintln!("This cannot be undone. Type \"revoke\" to confirm.");
    eprint!("> ");
    io::stderr().flush()?;
    let mut answer = String::new();
    let read = io::stdin().read_line(&mut answer)?;
    if read == 0 || answer.trim() != "revoke" {
        if json {
            eprintln!("Aborted.");
        } else {
            println!("Aborted.");
        }
        return Ok(false);
    }
    Ok(true)
}

fn confirm_send(preview: &MigrationPreviewResponse, json: bool) -> Result<bool> {
    if json {
        eprintln!(
            "preview: {} recipient(s) - pass --yes to skip this prompt",
            preview.count
        );
    } else {
        eprintln!(
            "About to send migration email to {} recipient(s) (source: {}).",
            preview.count, preview.source
        );
        eprintln!("{}", migration_recipient_table(preview));
    }
    eprint!("Continue? [y/N] ");
    io::stderr().flush()?;
    let mut answer = String::new();
    let read = io::stdin().read_line(&mut answer)?;
    let answer = answer.trim().to_lowercase();
    if read == 0 || (answer != "y" && answer != "yes") {
        if json {
            eprintln!("Aborted.");
        } else {
            println!("Aborted.");
        }
        return Ok(false);
    }
    Ok(true)
}

fn print_waitlist(result: &WaitlistResponse) {
    if result.items.is_empty() {
        println!("No waitlist entries.");
        return;
    }
    println!("EMAIL\tNAME\tSOURCE\tCREATED\tAPPROVED");
    for item in &result.items {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            item.email,
            item.name.as_deref().unwrap_or("-"),
            item.source,
            date_only(&item.created_at),
            item.approved_at.as_deref().map_or("-", date_only)
        );
    }
    println!("\nShowing {} of {}", result.items.len(), result.total);
}

fn print_user(result: &ShowUserResponse) {
    println!("USER");
    println!("----");
    println!("email:      {}", result.user.email);
    println!("name:       {}", result.user.name.as_deref().unwrap_or("-"));
    println!("status:     {}", result.user.status);
    println!("id:         {}", result.user.id);
    println!("created:    {}", date_only(&result.user.created_at));
    println!("updated:    {}", date_only(&result.user.updated_at));
    if let Some(notes) = &result.user.notes {
        println!("notes:      {notes}");
    }
    println!("\nTOKENS");
    println!("------");
    if result.tokens.is_empty() {
        println!("(none)");
    } else {
        println!("ID\tSCOPES\tCREATED\tEXPIRES\tREVOKED");
        for token in &result.tokens {
            println!(
                "{}\t{}\t{}\t{}\t{}",
                token.id,
                token.scopes.join(","),
                date_only(&token.created_at),
                date_only(&token.expires_at),
                token.revoked_at.as_deref().map_or("-", date_only)
            );
        }
    }
    println!("\nRECENT AUDIT");
    println!("------------");
    if result.recent_audit.is_empty() {
        println!("(none)");
    } else {
        print_audit_rows(&result.recent_audit);
    }
}

fn print_revoke(result: &RevokeResponse, subject: &str) {
    println!("✓ Revoked {} token(s) for {subject}", result.revoked);
    // SEC-007 / GH #1672: show the refresh-session and account-suspension
    // counters when the server provides them, so operators see that
    // revocation closed every credential surface.
    if let Some(refresh) = result.refresh_sessions_revoked {
        println!("  refresh sessions revoked: {refresh}");
    }
    if result.account_suspended == Some(true) {
        println!("  account suspended (re-approve to restore access)");
    }
}

fn print_audit(result: &AuditResponse) {
    if result.items.is_empty() {
        println!("No audit entries.");
        return;
    }
    print_audit_rows(&result.items);
    println!("\nShowing {} of {}", result.items.len(), result.total);
}

fn print_audit_rows(items: &[crate::auth::client::AuditItem]) {
    println!("WHEN\tACTION\tACTOR\tMETADATA");
    for item in items {
        let metadata = item.metadata.as_object().map_or_else(String::new, |obj| {
            if obj.is_empty() {
                String::new()
            } else {
                serde_json::to_string(&item.metadata).unwrap_or_default()
            }
        });
        println!(
            "{}\t{}\t{}\t{}",
            timestamp(&item.created_at),
            item.action,
            item.actor,
            metadata
        );
    }
}

fn print_migration_preview(result: &MigrationPreviewResponse) {
    if result.count == 0 {
        println!("No recipients match the filter.");
        return;
    }
    println!(
        "Dry run: {} recipient(s) from source \"{}\"",
        result.count, result.source
    );
    println!("{}", migration_recipient_table(result));
    println!(
        "Preview expires {}. Run with --no-dry-run to fetch a fresh preview and send that snapshot.",
        result.expires_at
    );
}

fn migration_recipient_table(result: &MigrationPreviewResponse) -> String {
    let mut out = String::from("EMAIL\tNAME");
    for recipient in &result.recipients {
        let _ = write!(
            out,
            "\n{}\t{}",
            recipient.email,
            recipient.name.as_deref().unwrap_or("")
        );
    }
    out
}

fn print_migration_send(result: &MigrationSendResponse) {
    println!(
        "✓ Sent {}/{} (failed: {})",
        result.sent, result.total, result.failed
    );
    println!("EMAIL\tSENT\tERROR");
    for row in &result.results {
        println!(
            "{}\t{}\t{}",
            row.email,
            if row.sent { "yes" } else { "no" },
            row.error.as_deref().unwrap_or("")
        );
    }
}

fn print_email_update(result: &EmailUpdateResponse) {
    println!(
        "✓ Updated email {} -> {}",
        result.previous_email, result.user.email
    );
}

fn rewrite_migration_error(err: anyhow::Error) -> anyhow::Error {
    let Some(api) = err.downcast_ref::<ApiError>() else {
        return err;
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&api.body) else {
        return err;
    };
    let Some(code) = parsed.get("code").and_then(|code| code.as_str()) else {
        return err;
    };
    match code {
        "cohort_drift" => {
            let added = parsed
                .get("added")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let removed = parsed
                .get("removed")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let mut message = String::from(
                "recipient set changed since preview; re-run with --no-dry-run to preview and retry",
            );
            if !added.is_empty() {
                let _ = write!(message, "\n  added:   {added}");
            }
            if !removed.is_empty() {
                let _ = write!(message, "\n  removed: {removed}");
            }
            anyhow::anyhow!(message)
        }
        "preview_token_expired" => anyhow::anyhow!(
            "preview token expired (10-minute TTL). Re-run with --no-dry-run to preview and retry within 10 minutes."
        ),
        "preview_token_consumed" => anyhow::anyhow!(
            "preview token already used. A prior send may have completed; run without --no-dry-run to verify recipients before retrying."
        ),
        "preview_token_missing" => anyhow::anyhow!(
            "preview token not found. If another operator created the preview, use the same admin identity. Otherwise re-run with --no-dry-run to generate a fresh snapshot."
        ),
        "preview_token_required" => anyhow::anyhow!(
            "server rejected send without a preview token. This usually means the CLI skipped the preview step; re-run with --no-dry-run."
        ),
        _ => err,
    }
}

fn date_only(value: &str) -> &str {
    value.get(..10).unwrap_or(value)
}

fn timestamp(value: &str) -> String {
    value.get(..19).unwrap_or(value).replace('T', " ")
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
    fn args_parses_invite_edict_mode() {
        let w =
            Wrapper::try_parse_from(["test", "invite", "early@example.com", "--edict"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_rejects_invite_without_email() {
        let err = Wrapper::try_parse_from(["test", "invite"]).unwrap_err();
        assert_ne!(err.exit_code(), 0);
    }

    #[test]
    fn args_parses_list_filters() {
        let w = Wrapper::try_parse_from([
            "test", "list", "--status", "approved", "--source", "manual", "--limit", "25",
            "--offset", "10",
        ])
        .unwrap();
        match w.inner.command {
            AdminCommand::List {
                status,
                source,
                limit,
                offset,
            } => {
                assert!(matches!(status, Some(WaitlistStatus::Approved)));
                assert!(matches!(source, Some(WaitlistSource::Manual)));
                assert_eq!(limit, Some(25));
                assert_eq!(offset, Some(10));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn args_rejects_invalid_list_status() {
        let err = Wrapper::try_parse_from(["test", "list", "--status", "bogus"]).unwrap_err();
        assert_ne!(err.exit_code(), 0);
    }

    #[test]
    fn args_rejects_list_limit_outside_supported_range() {
        let low = Wrapper::try_parse_from(["test", "list", "--limit", "0"]).unwrap_err();
        assert_ne!(low.exit_code(), 0);

        let high = Wrapper::try_parse_from(["test", "list", "--limit", "201"]).unwrap_err();
        assert_ne!(high.exit_code(), 0);
    }

    #[test]
    fn args_rejects_list_negative_offset() {
        let err = Wrapper::try_parse_from(["test", "list", "--offset", "-1"]).unwrap_err();
        assert_ne!(err.exit_code(), 0);
    }

    #[test]
    fn args_parses_show_email() {
        let w = Wrapper::try_parse_from(["test", "show", "user@example.com"]).unwrap();
        match w.inner.command {
            AdminCommand::Show { email } => assert_eq!(email, "user@example.com"),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn args_parses_revoke_email_yes() {
        let w = Wrapper::try_parse_from(["test", "revoke", "user@example.com", "-y"]).unwrap();
        match w.inner.command {
            AdminCommand::Revoke { email, token, yes } => {
                assert_eq!(email.as_deref(), Some("user@example.com"));
                assert!(token.is_none());
                assert!(yes);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn args_parses_revoke_token_yes() {
        let w = Wrapper::try_parse_from(["test", "revoke", "--token", "raw", "--yes"]).unwrap();
        match w.inner.command {
            AdminCommand::Revoke { email, token, yes } => {
                assert!(email.is_none());
                assert_eq!(token.as_deref(), Some("raw"));
                assert!(yes);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn args_rejects_revoke_without_target() {
        let err = Wrapper::try_parse_from(["test", "revoke", "-y"]).unwrap_err();
        assert_ne!(err.exit_code(), 0);
    }

    #[test]
    fn args_rejects_revoke_email_and_token() {
        let err =
            Wrapper::try_parse_from(["test", "revoke", "user@example.com", "--token", "raw", "-y"])
                .unwrap_err();
        assert_ne!(err.exit_code(), 0);
    }

    #[test]
    fn args_parses_audit_filters() {
        let w = Wrapper::try_parse_from([
            "test",
            "audit",
            "--action",
            "user.approved",
            "--filter-actor",
            "ops@example.com",
            "--limit",
            "20",
        ])
        .unwrap();
        match w.inner.command {
            AdminCommand::Audit {
                action,
                filter_actor,
                limit,
                offset,
            } => {
                assert_eq!(action.as_deref(), Some("user.approved"));
                assert_eq!(filter_actor.as_deref(), Some("ops@example.com"));
                assert_eq!(limit, Some(20));
                assert!(offset.is_none());
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn args_rejects_audit_limit_outside_supported_range() {
        let low = Wrapper::try_parse_from(["test", "audit", "--limit", "0"]).unwrap_err();
        assert_ne!(low.exit_code(), 0);

        let high = Wrapper::try_parse_from(["test", "audit", "--limit", "201"]).unwrap_err();
        assert_ne!(high.exit_code(), 0);
    }

    #[test]
    fn args_rejects_audit_negative_offset() {
        let err = Wrapper::try_parse_from(["test", "audit", "--offset", "-1"]).unwrap_err();
        assert_ne!(err.exit_code(), 0);
    }

    #[test]
    fn args_parses_send_migration_defaults() {
        let w = Wrapper::try_parse_from(["test", "send-migration"]).unwrap();
        match w.inner.command {
            AdminCommand::SendMigration {
                source,
                limit,
                no_dry_run,
                yes,
            } => {
                assert!(matches!(source, MigrationSource::Import));
                assert_eq!(limit, 20);
                assert!(!no_dry_run);
                assert!(!yes);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn args_parses_send_migration_real_send() {
        let w = Wrapper::try_parse_from([
            "test",
            "send-migration",
            "--source",
            "website",
            "--limit",
            "5",
            "--no-dry-run",
            "-y",
        ])
        .unwrap();
        match w.inner.command {
            AdminCommand::SendMigration {
                source,
                limit,
                no_dry_run,
                yes,
            } => {
                assert!(matches!(source, MigrationSource::Website));
                assert_eq!(limit, 5);
                assert!(no_dry_run);
                assert!(yes);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn args_rejects_send_migration_limit_outside_supported_range() {
        let low = Wrapper::try_parse_from(["test", "send-migration", "--limit", "0"]).unwrap_err();
        assert_ne!(low.exit_code(), 0);

        let high =
            Wrapper::try_parse_from(["test", "send-migration", "--limit", "101"]).unwrap_err();
        assert_ne!(high.exit_code(), 0);
    }

    #[test]
    fn args_rejects_invalid_send_migration_source() {
        let err =
            Wrapper::try_parse_from(["test", "send-migration", "--source", "bogus"]).unwrap_err();
        assert_ne!(err.exit_code(), 0);
    }

    #[test]
    fn args_parses_email_update() {
        let w =
            Wrapper::try_parse_from(["test", "email-update", "old@example.com", "new@example.com"])
                .unwrap();
        match w.inner.command {
            AdminCommand::EmailUpdate {
                current_email,
                new_email,
            } => {
                assert_eq!(current_email, "old@example.com");
                assert_eq!(new_email, "new@example.com");
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn resolve_admin_key_missing_returns_auth_required() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_admin_key_with_config(
            Err(std::env::VarError::NotPresent),
            false,
            &dir.path().join("missing-admin-auth.json"),
            |_| panic!("missing config should not resolve a source"),
        )
        .unwrap_err();
        assert!(
            err.is::<AuthRequired>(),
            "expected AuthRequired, got {err:?}"
        );
    }

    #[test]
    fn resolve_admin_key_empty_returns_auth_required() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_admin_key_with_config(
            Ok(String::new()),
            false,
            &dir.path().join("missing-admin-auth.json"),
            |_| panic!("missing config should not resolve a source"),
        )
        .unwrap_err();
        assert!(
            err.is::<AuthRequired>(),
            "expected AuthRequired, got {err:?}"
        );
    }

    #[test]
    fn resolve_admin_key_present_returns_value() {
        let key =
            resolve_admin_key_with_config(Ok("secret-token".into()), false, Path::new("."), |_| {
                panic!("configured source should not be used when env is set")
            })
            .unwrap();
        assert_eq!(key, "secret-token");
    }

    #[test]
    fn resolve_admin_key_not_unicode_env_does_not_fall_back_to_configured_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("admin-auth.json");
        save_admin_credential_config(
            &path,
            &AdminCredentialConfig::one_password("op://Anvil/admin-key/credential"),
        )
        .unwrap();

        let err = resolve_admin_key_with_config(
            Err(std::env::VarError::NotUnicode("bad".into())),
            false,
            &path,
            |_| panic!("invalid env value must not fall back to configured source"),
        )
        .unwrap_err();

        assert!(
            err.is::<AuthRequired>(),
            "expected AuthRequired, got {err:?}"
        );
    }

    #[test]
    fn args_parses_admin_auth_set_1password() {
        let w = Wrapper::try_parse_from([
            "test",
            "auth",
            "set",
            "1password",
            "op://Anvil/admin-key/credential",
        ])
        .unwrap();
        match w.inner.command {
            AdminCommand::Auth {
                command: AdminAuthCommand::Set { source, reference },
            } => {
                assert_eq!(source, AdminCredentialSourceKind::OnePassword);
                assert_eq!(reference, "op://Anvil/admin-key/credential");
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn admin_credential_source_round_trips_without_plaintext_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("admin-auth.json");
        let config = AdminCredentialConfig::one_password("op://Anvil/admin-key/credential");

        save_admin_credential_config(&path, &config).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();

        assert!(raw.contains("op://Anvil/admin-key/credential"));
        assert!(!raw.contains("sk_admin"));
        assert_eq!(load_admin_credential_config(&path).unwrap(), Some(config));
    }

    #[cfg(unix)]
    #[test]
    fn admin_credential_source_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("admin-auth.json");
        save_admin_credential_config(
            &path,
            &AdminCredentialConfig::one_password("op://Anvil/admin-key/credential"),
        )
        .unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn resolve_admin_key_prefers_env_over_configured_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("admin-auth.json");
        save_admin_credential_config(
            &path,
            &AdminCredentialConfig::one_password("op://Anvil/admin-key/credential"),
        )
        .unwrap();

        let key = resolve_admin_key_with_config(Ok("env-token".to_string()), false, &path, |_| {
            panic!("configured source should not be used when env is set")
        })
        .unwrap();

        assert_eq!(key, "env-token");
    }

    #[test]
    fn args_parses_admin_auth_set_key() {
        let w =
            Wrapper::try_parse_from(["test", "auth", "set", "key", "anvil_admin_abc123"]).unwrap();
        match w.inner.command {
            AdminCommand::Auth {
                command: AdminAuthCommand::Set { source, reference },
            } => {
                assert_eq!(source, AdminCredentialSourceKind::Key);
                assert_eq!(reference, "anvil_admin_abc123");
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn resolve_admin_key_reads_configured_key_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("admin-auth.json");
        save_admin_credential_config(&path, &AdminCredentialConfig::key("stored-admin-token"))
            .unwrap();

        let key = resolve_admin_key_with_config(
            Err(std::env::VarError::NotPresent),
            false,
            &path,
            |_| panic!("the key source must not shell out to 1Password"),
        )
        .unwrap();

        assert_eq!(key, "stored-admin-token");
    }

    #[test]
    fn resolve_admin_key_prefers_env_over_stored_key_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("admin-auth.json");
        save_admin_credential_config(&path, &AdminCredentialConfig::key("stored-admin-token"))
            .unwrap();

        let key = resolve_admin_key_with_config(Ok("env-token".to_string()), false, &path, |_| {
            panic!("stored key must not be used when env is set")
        })
        .unwrap();

        assert_eq!(key, "env-token");
    }

    #[test]
    fn resolve_admin_key_rejects_empty_stored_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("admin-auth.json");
        save_admin_credential_config(&path, &AdminCredentialConfig::key("placeholder")).unwrap();
        // Hand-write an empty stored key (save_ rejects empty references).
        std::fs::write(&path, r#"{"source":"key","reference":""}"#).unwrap();

        let err = resolve_admin_key_with_config(
            Err(std::env::VarError::NotPresent),
            false,
            &path,
            |_| panic!("empty stored key must not reach 1Password"),
        )
        .unwrap_err();

        assert!(
            err.is::<AuthRequired>(),
            "expected AuthRequired, got {err:?}"
        );
    }

    #[test]
    fn resolve_admin_key_rejects_whitespace_only_stored_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("admin-auth.json");
        // A hand-edited config with a whitespace-only key must be rejected,
        // not handed to the server to fail opaquely later.
        std::fs::write(&path, "{\"source\":\"key\",\"reference\":\"   \\n\"}").unwrap();

        let err = resolve_admin_key_with_config(
            Err(std::env::VarError::NotPresent),
            false,
            &path,
            |_| panic!("whitespace-only stored key must not reach 1Password"),
        )
        .unwrap_err();

        assert!(
            err.is::<AuthRequired>(),
            "expected AuthRequired, got {err:?}"
        );
    }

    #[test]
    fn redact_admin_reference_masks_key_but_not_1password() {
        // A 1Password reference is not a secret — shown verbatim.
        assert_eq!(
            redact_admin_reference("1password", "op://Anvil/admin-key/credential"),
            "op://Anvil/admin-key/credential"
        );
        // The key source's reference IS the secret — only a trailing fingerprint.
        assert_eq!(
            redact_admin_reference("key", "anvil_admin_abcdef1234"),
            "****1234"
        );
        assert_eq!(redact_admin_reference("key", "abcd"), "****");
        assert_eq!(redact_admin_reference("key", "xy"), "****");
    }

    #[test]
    fn reads_admin_auth_status_does_not_serialise_the_raw_key() {
        // Guard: the status JSON must carry the masked reference, never the key.
        let status = AdminAuthStatus {
            configured: true,
            source: Some("key".to_string()),
            reference: Some(redact_admin_reference("key", "anvil_admin_topsecret9999")),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("****9999"), "json: {json}");
        assert!(!json.contains("topsecret"), "raw key leaked: {json}");
    }

    #[test]
    fn resolve_admin_key_reads_configured_1password_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("admin-auth.json");
        save_admin_credential_config(
            &path,
            &AdminCredentialConfig::one_password("op://Anvil/admin-key/credential"),
        )
        .unwrap();

        let key = resolve_admin_key_with_config(
            Err(std::env::VarError::NotPresent),
            false,
            &path,
            |reference| {
                assert_eq!(reference, "op://Anvil/admin-key/credential");
                Ok("resolved-token".to_string())
            },
        )
        .unwrap();

        assert_eq!(key, "resolved-token");
    }

    #[test]
    fn resolve_admin_key_rejects_empty_configured_secret() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("admin-auth.json");
        save_admin_credential_config(
            &path,
            &AdminCredentialConfig::one_password("op://Anvil/admin-key/credential"),
        )
        .unwrap();

        let err = resolve_admin_key_with_config(
            Err(std::env::VarError::NotPresent),
            false,
            &path,
            |_| Ok(String::new()),
        )
        .unwrap_err();

        assert!(
            err.is::<AuthRequired>(),
            "expected AuthRequired, got {err:?}"
        );
    }
}
