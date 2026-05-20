use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;

use crate::GlobalArgs;

mod fetch;
mod signature;
use signature::VerifiedArtefact;

/// Exit‐code marker: `--check` found an available update.
#[derive(Debug)]
pub struct UpdateAvailable;

impl std::fmt::Display for UpdateAvailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("update available")
    }
}

impl std::error::Error for UpdateAvailable {}

#[derive(Debug, clap::Args)]
pub struct UpdateArgs {
    /// Check for updates without installing.
    #[arg(long)]
    pub check: bool,

    /// Install a specific version instead of latest.
    #[arg(long, value_name = "VER")]
    pub version: Option<String>,

    /// Reinstall even if already on the latest version.
    #[arg(long)]
    pub force: bool,

    /// Skip signature verification of the downloaded artefact. Dangerous —
    /// only use when the release public key is known to be temporarily
    /// unavailable and the user explicitly accepts the risk. Logs a loud
    /// warning. See ADR-045.
    #[arg(long, hide = true)]
    pub insecure_skip_verify: bool,
}

pub fn run(args: &UpdateArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");

    // 1. Package-manager installs: defer to the package manager so the user
    //    gets the one command that will work, not a menu of options. Detected
    //    via path markers shared with `commands::version` (see
    //    `package_manager_for_exe`).
    if let Some(pm) = detect_package_manager() {
        report_package_manager_install(current, global, pm);
        return Ok(());
    }

    // 2. Try sidecar binary
    if let Some(sidecar) = find_sidecar() {
        if global.verbose {
            eprintln!("Using sidecar: {}", sidecar.display());
        }
        return run_sidecar(&sidecar, args);
    }

    // 3. Library fallback
    run_library_update(args, global)
}

// ── Package-manager detection ───────────────────────────────────────

/// A package manager that owns the installed `anvil` binary. The upgrade
/// must go through it, not through `anvil update`'s in-process replace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackageManager {
    Homebrew,
    Winget,
    Scoop,
}

const HOMEBREW_PREFIXES: &[&str] = &["/opt/homebrew/", "/usr/local/Cellar/", "/home/linuxbrew/"];
// Mirrors the markers in `commands::version` — kept in sync intentionally.
// Winget installs land under `%LOCALAPPDATA%\Microsoft\WindowsApps\eddacraft...`,
// scoop installs under `%USERPROFILE%\scoop\apps\anvil\<version>\`.
const WINGET_MARKERS: &[&str] = &["WindowsApps\\eddacraft", "WindowsApps/eddacraft"];
const SCOOP_MARKERS: &[&str] = &["scoop\\apps\\anvil\\", "scoop/apps/anvil/"];

fn detect_package_manager() -> Option<PackageManager> {
    let exe = std::env::current_exe().ok()?;
    package_manager_for_exe(&exe)
}

fn package_manager_for_exe(path: &Path) -> Option<PackageManager> {
    let s = path.to_str()?;
    if HOMEBREW_PREFIXES.iter().any(|p| s.starts_with(p)) {
        return Some(PackageManager::Homebrew);
    }
    if WINGET_MARKERS.iter().any(|m| s.contains(m)) {
        return Some(PackageManager::Winget);
    }
    if SCOOP_MARKERS.iter().any(|m| s.contains(m)) {
        return Some(PackageManager::Scoop);
    }
    None
}

fn report_package_manager_install(current: &str, global: &GlobalArgs, pm: PackageManager) {
    let (method, upgrade_cmd) = match pm {
        PackageManager::Homebrew => ("homebrew", "brew upgrade eddacraft/tap/anvil"),
        PackageManager::Winget => ("winget", "winget upgrade --id eddacraft.anvil"),
        PackageManager::Scoop => ("scoop", "scoop update anvil"),
    };
    let display = match pm {
        PackageManager::Homebrew => "Homebrew",
        PackageManager::Winget => "WinGet",
        PackageManager::Scoop => "Scoop",
    };
    if global.json {
        println!(
            "{}",
            serde_json::json!({
                "current_version": current,
                "install_method": method,
                "message": format!("Installed via {display}. Run `{upgrade_cmd}` instead."),
                "upgrade_command": upgrade_cmd,
            })
        );
    } else {
        println!("anvil was installed via {display}. Run `{upgrade_cmd}` instead.");
    }
}

// ── Sidecar resolution ─────────────────────────────────────────────

const SIDECAR_NAME: &str = if cfg!(windows) {
    "eddacraft-anvil-update.exe"
} else {
    "eddacraft-anvil-update"
};

fn find_sidecar() -> Option<PathBuf> {
    // Adjacent to the current executable
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let adjacent = dir.join(SIDECAR_NAME);
        if adjacent.is_file() {
            return Some(adjacent);
        }
    }

    // On PATH
    find_on_path(SIDECAR_NAME)
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn run_sidecar(path: &Path, args: &UpdateArgs) -> anyhow::Result<()> {
    let mut cmd = Command::new(path);

    if args.check {
        cmd.arg("--check");
    }
    if let Some(ver) = &args.version {
        cmd.args(["--version", ver]);
    }
    if args.force {
        cmd.arg("--force");
    }

    let status = cmd
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if status.success() {
        Ok(())
    } else {
        let code = status.code().unwrap_or(1);
        // For --check, exit code 1 means "update available" — propagate it
        if args.check && code == 1 {
            anyhow::bail!(UpdateAvailable);
        }
        anyhow::bail!("updater exited with code {code}");
    }
}

// ── Library fallback (axoupdater) ───────────────────────────────────

/// GitHub release source for manual configuration when no install receipt exists.
const GITHUB_OWNER: &str = "eddacraft";
const GITHUB_REPO: &str = "anvil";

fn run_library_update(args: &UpdateArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");

    // Windows holds an exclusive file lock on a running executable, so the
    // in-process axoupdater replace cannot overwrite `anvil.exe`. The sidecar
    // path (`find_sidecar`) is the only safe Windows updater; it is not
    // shipped in this release (`install-updater = false` in
    // dist-workspace.toml, gated on missing aarch64-pc-windows-msvc
    // axoupdater binaries). `--check` is read-only, so let it through.
    if cfg!(windows) && !args.check {
        report_windows_unsupported(current, global);
        return Ok(());
    }

    let mut updater = axoupdater::AxoUpdater::new_for("anvil");

    // Try loading the cargo-dist install receipt (created by shell/powershell installers).
    // If missing (dev build, manual install), configure the release source manually.
    if updater.load_receipt().is_err() {
        if global.verbose {
            eprintln!("No install receipt found, configuring GitHub release source manually");
        }
        let current_version: axoupdater::Version = current
            .parse()
            .map_err(|e| anyhow::anyhow!("failed to parse current version: {e}"))?;
        updater.set_current_version(current_version)?;
        updater.set_release_source(axoupdater::ReleaseSource {
            release_type: axoupdater::ReleaseSourceType::GitHub,
            owner: GITHUB_OWNER.to_string(),
            name: GITHUB_REPO.to_string(),
            app_name: "anvil".to_string(),
        });
    }

    if args.force {
        updater.always_update(true);
    }

    if let Some(ver) = &args.version {
        updater
            .configure_version_specifier(axoupdater::UpdateRequest::SpecificVersion(ver.clone()));
    }

    let update_needed = updater.is_update_needed_sync()?;

    if args.check {
        return report_check(current, update_needed, global);
    }

    if !update_needed && !args.force && args.version.is_none() {
        report_up_to_date(current, global);
        return Ok(());
    }

    if !global.json {
        println!("Current version: {current}");
        println!("Downloading update...");
    }

    // Verification preflight: download the installer + its detached
    // `.minisig` ourselves, verify against the embedded public key, and
    // hand the verified file to axoupdater via `configure_installer_path`
    // so it skips its own download step. See ADR-045.
    let _verified_tempdir = match verify_pending_install(args, global) {
        Ok(Some((tempdir, verified_path, verified))) => {
            let path_str = verified_path
                .to_str()
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "verified installer path is not valid UTF-8: {}",
                        verified_path.display()
                    )
                })?
                .to_string();
            updater.configure_installer_path(path_str);
            if !global.json {
                println!("Verified signature ({}).", verified.trusted_comment);
            }
            Some(tempdir)
        }
        Ok(None) => None,
        Err(e) => return Err(e),
    };

    updater.enable_installer_output();
    perform_update(updater, current, global)
}

/// Loud warning emitted when `--insecure-skip-verify` is passed. ADR-045
/// requires this deviation to be visible even without `--verbose`.
fn write_skip_verify_warning<W: std::io::Write>(w: &mut W) -> std::io::Result<()> {
    writeln!(
        w,
        "WARNING: --insecure-skip-verify: skipping signature verification on the update artefact."
    )?;
    writeln!(
        w,
        "         The downloaded installer will run without proof it came from the Anvil release key."
    )?;
    Ok(())
}

/// Loud warning emitted when the binary was built without
/// `ANVIL_RELEASE_PUBLIC_KEY` (development build). Council CRITICAL per
/// ADR-045: must surface even without `--verbose`, otherwise the entire
/// verification feature can ship as a silent no-op.
fn write_dev_key_warning<W: std::io::Write>(w: &mut W) -> std::io::Result<()> {
    writeln!(
        w,
        "WARNING: This anvil binary was built without ANVIL_RELEASE_PUBLIC_KEY (development build)."
    )?;
    writeln!(
        w,
        "         Skipping signature verification — release builds enforce it. See ADR-045."
    )?;
    writeln!(
        w,
        "         If you got this binary from an official release, please re-install from a trusted source."
    )?;
    Ok(())
}

/// Run the signature-verification preflight for the library-fallback
/// path. Returns the tempdir that must live for the duration of the
/// install (dropped tempdirs delete their contents), the verified
/// installer path, and the verified-artefact metadata.
///
/// Returns `Ok(None)` when verification is skipped — either because the
/// binary was built without a real release public key (development build),
/// or because `--insecure-skip-verify` was passed. Both cases log a loud
/// warning so the deviation is visible.
fn verify_pending_install(
    args: &UpdateArgs,
    _global: &GlobalArgs,
) -> anyhow::Result<Option<(tempfile::TempDir, PathBuf, VerifiedArtefact)>> {
    if args.insecure_skip_verify {
        // `eprintln!` semantics: best-effort; ignore writer errors.
        let _ = write_skip_verify_warning(&mut std::io::stderr().lock());
        return Ok(None);
    }

    if signature::is_using_dev_public_key() {
        let _ = write_dev_key_warning(&mut std::io::stderr().lock());
        return Ok(None);
    }

    let tempdir = tempfile::tempdir().context("creating tempdir for verified installer")?;
    let source =
        fetch::github_release_source(GITHUB_OWNER, GITHUB_REPO, "anvil", args.version.as_deref());
    let (path, verified) = fetch::fetch_and_verify(&source, tempdir.path())?;
    // Council CRITICAL: bind the verified artefact to the requested
    // version. A signed installer for v0.6.0 must not be accepted in
    // response to `anvil update --version v0.7.0` (downgrade vector).
    if let Some(requested) = args.version.as_deref() {
        check_trusted_comment_matches_version(&verified.trusted_comment, requested)?;
    }
    Ok(Some((tempdir, path, verified)))
}

/// Parse the `tag=<vX.Y.Z>` field from the minisign trusted comment and
/// assert it matches `requested`. The trusted comment is set by the
/// release-sign workflow to `tag=<TAG>;commit=<SHA>;built=<DATE>`.
fn check_trusted_comment_matches_version(
    trusted_comment: &str,
    requested: &str,
) -> anyhow::Result<()> {
    let normalised_requested = requested.trim_start_matches('v');
    let tag = trusted_comment
        .split(';')
        .find_map(|kv| kv.trim().strip_prefix("tag="))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "signed artefact's trusted comment has no `tag=` field: {trusted_comment:?}"
            )
        })?;
    let normalised_tag = tag.trim_start_matches('v');
    if normalised_tag != normalised_requested {
        anyhow::bail!(
            "signed artefact is for tag `{tag}` but you requested version `{requested}` — refusing to install. \
             This protects against signature-replay / downgrade. \
             If you really meant tag `{tag}`, run: anvil update --version {tag}"
        );
    }
    Ok(())
}

fn report_check(current: &str, update_needed: bool, global: &GlobalArgs) -> anyhow::Result<()> {
    if global.json {
        println!(
            "{}",
            serde_json::json!({
                "current_version": current,
                "update_available": update_needed,
                "action": "check"
            })
        );
    } else if update_needed {
        println!("Current version: {current}");
        println!("Update available. Run `anvil update` to install.");
    } else {
        println!("Current version: {current}");
        println!("Already up to date.");
    }

    if update_needed {
        anyhow::bail!(UpdateAvailable);
    }
    Ok(())
}

fn report_windows_unsupported(current: &str, global: &GlobalArgs) {
    let message = windows_unsupported_message();
    if global.json {
        println!(
            "{}",
            serde_json::json!({
                "current_version": current,
                "platform": "windows",
                "action": "unsupported",
                "message": message,
                "alternatives": [
                    "winget upgrade --id eddacraft.anvil",
                    "irm https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.ps1 | iex",
                ],
            })
        );
    } else {
        println!("Current version: {current}");
        println!("{message}");
    }
}

fn windows_unsupported_message() -> &'static str {
    "Self-update is not supported on Windows in this release.\n\
     \n\
     To upgrade, use one of:\n  \
     - winget upgrade --id eddacraft.anvil\n  \
     - Re-run the PowerShell installer:\n      \
         irm https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.ps1 | iex\n\
     \n\
     If the installer fails with \"file is being used by another process\",\n\
     close any editor running the Anvil MCP server (Cursor, Claude Code)\n\
     and try again."
}

fn report_up_to_date(current: &str, global: &GlobalArgs) {
    if global.json {
        println!(
            "{}",
            serde_json::json!({
                "current_version": current,
                "update_available": false,
                "action": "none"
            })
        );
    } else {
        println!("Current version: {current}");
        println!("Already up to date.");
    }
}

fn perform_update(
    mut updater: axoupdater::AxoUpdater,
    current: &str,
    global: &GlobalArgs,
) -> anyhow::Result<()> {
    match updater.run_sync() {
        Ok(Some(result)) => {
            if global.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "current_version": current,
                        "new_version": result.new_version_tag,
                        "action": "updated"
                    })
                );
            } else {
                println!("Updated successfully to {}.", result.new_version_tag);
            }
            Ok(())
        }
        Ok(None) => {
            if global.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "current_version": current,
                        "action": "none",
                        "message": "no update performed"
                    })
                );
            } else {
                println!("No update performed.");
            }
            Ok(())
        }
        Err(e) => {
            anyhow::bail!(
                "Update failed: {e}\n\n\
                 To update manually:\n  \
                 - Install script: curl -fsSL https://install.eddacraft.ai | sh\n  \
                 - From source: cargo build --release"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Package-manager detection ───────────────────────────────────

    #[test]
    fn homebrew_opt_prefix_detected() {
        let path = Path::new("/opt/homebrew/bin/anvil");
        assert_eq!(
            package_manager_for_exe(path),
            Some(PackageManager::Homebrew)
        );
    }

    #[test]
    fn homebrew_cellar_prefix_detected() {
        let path = Path::new("/usr/local/Cellar/anvil/0.3.1/bin/anvil");
        assert_eq!(
            package_manager_for_exe(path),
            Some(PackageManager::Homebrew)
        );
    }

    #[test]
    fn homebrew_linuxbrew_prefix_detected() {
        let path = Path::new("/home/linuxbrew/.linuxbrew/bin/anvil");
        assert_eq!(
            package_manager_for_exe(path),
            Some(PackageManager::Homebrew)
        );
    }

    #[test]
    fn winget_backslash_path_detected() {
        let path = Path::new(
            r"C:\Users\Alice\AppData\Local\Microsoft\WindowsApps\eddacraft.anvil_8wekyb3d8bbwe\anvil.exe",
        );
        assert_eq!(package_manager_for_exe(path), Some(PackageManager::Winget));
    }

    #[test]
    fn winget_forward_slash_path_detected() {
        // Some Windows toolchains normalise to forward slashes.
        let path = Path::new(
            "C:/Users/Alice/AppData/Local/Microsoft/WindowsApps/eddacraft.anvil_8wekyb3d8bbwe/anvil.exe",
        );
        assert_eq!(package_manager_for_exe(path), Some(PackageManager::Winget));
    }

    #[test]
    fn scoop_backslash_path_detected() {
        let path = Path::new(r"C:\Users\Alice\scoop\apps\anvil\0.6.1\anvil.exe");
        assert_eq!(package_manager_for_exe(path), Some(PackageManager::Scoop));
    }

    #[test]
    fn scoop_forward_slash_path_detected() {
        let path = Path::new("C:/Users/Alice/scoop/apps/anvil/0.6.1/anvil.exe");
        assert_eq!(package_manager_for_exe(path), Some(PackageManager::Scoop));
    }

    #[test]
    fn cargo_bin_path_has_no_package_manager() {
        let path = Path::new("/home/user/.cargo/bin/anvil");
        assert_eq!(package_manager_for_exe(path), None);
    }

    #[test]
    fn windows_cargo_bin_path_has_no_package_manager() {
        let path = Path::new(r"C:\Users\121\.cargo\bin\anvil.exe");
        assert_eq!(package_manager_for_exe(path), None);
    }

    #[test]
    fn non_homebrew_usr_local_bin() {
        let path = Path::new("/usr/local/bin/anvil");
        assert_eq!(package_manager_for_exe(path), None);
    }

    // ── Trusted-comment tag check ──────────────────────────────────

    #[test]
    fn trusted_comment_matches_requested_version_exact() {
        check_trusted_comment_matches_version(
            "tag=v0.7.0-beta;commit=deadbeef;built=2026-05-14",
            "v0.7.0-beta",
        )
        .expect("matching tag must succeed");
    }

    #[test]
    fn trusted_comment_matches_requested_version_normalises_v_prefix() {
        // The user can pass `--version 0.7.0-beta` and the trusted
        // comment carries `tag=v0.7.0-beta` (or vice versa). Both forms
        // must match.
        check_trusted_comment_matches_version("tag=v0.7.0-beta;commit=x", "0.7.0-beta").unwrap();
        check_trusted_comment_matches_version("tag=0.7.0-beta;commit=x", "v0.7.0-beta").unwrap();
    }

    #[test]
    fn trusted_comment_refuses_mismatched_version() {
        // The signature-replay / downgrade case: a legitimate signed
        // installer for v0.6.0 is served in response to a request for
        // v0.7.0. Verification of the signature itself succeeds; the
        // tag-check is what catches the swap.
        let err = check_trusted_comment_matches_version(
            "tag=v0.6.0;commit=cafe;built=2026-04-01",
            "v0.7.0",
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("v0.6.0") && msg.contains("v0.7.0"),
            "error must name both tags, got: {msg}"
        );
        assert!(
            msg.contains("downgrade") || msg.contains("refusing to install"),
            "error must be loud and actionable, got: {msg}"
        );
    }

    #[test]
    fn trusted_comment_without_tag_field_is_rejected() {
        let err =
            check_trusted_comment_matches_version("timestamp:1234567890", "v0.7.0").unwrap_err();
        assert!(err.to_string().contains("no `tag=` field"));
    }

    // ── Sidecar resolution ──────────────────────────────────────────

    #[test]
    fn find_on_path_returns_none_for_missing_binary() {
        // Verify that a non-existent binary is not found on PATH
        assert!(find_on_path("eddacraft-anvil-update-nonexistent-test").is_none());
    }

    #[test]
    fn sidecar_name_matches_platform() {
        if cfg!(windows) {
            assert!(SIDECAR_NAME.to_ascii_lowercase().ends_with(".exe"));
        } else {
            assert!(!SIDECAR_NAME.contains('.'));
        }
    }

    // ── Sidecar command building ────────────────────────────────────

    fn args_with(check: bool, version: Option<&str>, force: bool) -> UpdateArgs {
        UpdateArgs {
            check,
            version: version.map(str::to_string),
            force,
            insecure_skip_verify: false,
        }
    }

    #[test]
    fn sidecar_command_check_flag() {
        // Verify the command would be built with --check
        let mut cmd = Command::new("fake-updater");
        let args = args_with(true, None, false);
        if args.check {
            cmd.arg("--check");
        }
        // Command::get_args is available — verify the arg is present
        let cmd_args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert_eq!(cmd_args, vec!["--check"]);
    }

    #[test]
    fn sidecar_command_version_flag() {
        let mut cmd = Command::new("fake-updater");
        let args = args_with(false, Some("0.4.0"), false);
        if let Some(ver) = &args.version {
            cmd.args(["--version", ver]);
        }
        let cmd_args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert_eq!(cmd_args, vec!["--version", "0.4.0"]);
    }

    #[test]
    fn sidecar_command_force_flag() {
        let mut cmd = Command::new("fake-updater");
        let args = args_with(false, None, true);
        if args.force {
            cmd.arg("--force");
        }
        let cmd_args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert_eq!(cmd_args, vec!["--force"]);
    }

    #[test]
    fn windows_unsupported_message_lists_alternatives() {
        let msg = windows_unsupported_message();
        assert!(msg.contains("winget upgrade --id eddacraft.anvil"));
        assert!(msg.contains("eddacraft-anvil-installer.ps1"));
        assert!(msg.contains("Anvil MCP server"));
    }

    #[test]
    fn sidecar_command_no_flags() {
        let cmd = Command::new("fake-updater");
        let cmd_args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert!(cmd_args.is_empty());
    }

    // ── Warning emission on the skip-verify paths (CLAWP-001) ───────

    #[test]
    fn skip_verify_warning_writes_both_loud_lines() {
        let mut buf = Vec::new();
        write_skip_verify_warning(&mut buf).expect("infallible for Vec");
        let text = std::str::from_utf8(&buf).expect("utf-8");

        assert!(
            text.contains("WARNING: --insecure-skip-verify"),
            "header line missing; got:\n{text}"
        );
        assert!(
            text.contains("without proof it came from the Anvil release key"),
            "rationale line missing; got:\n{text}"
        );
        // Operators read this on stderr; a missing trailing newline
        // merges the warning into the next log line.
        assert!(text.ends_with('\n'), "must end with newline; got: {text:?}");
        // Two distinct lines, not one wrapped line.
        assert_eq!(
            text.matches('\n').count(),
            2,
            "expected 2 lines, got:\n{text}"
        );
    }

    #[test]
    fn dev_key_warning_writes_three_loud_lines_and_cites_adr_045() {
        let mut buf = Vec::new();
        write_dev_key_warning(&mut buf).expect("infallible for Vec");
        let text = std::str::from_utf8(&buf).expect("utf-8");

        assert!(
            text.contains("ANVIL_RELEASE_PUBLIC_KEY (development build)"),
            "header line missing; got:\n{text}"
        );
        assert!(
            text.contains("release builds enforce it. See ADR-045"),
            "ADR-045 citation missing; got:\n{text}"
        );
        assert!(
            text.contains("re-install from a trusted source"),
            "recovery line missing; got:\n{text}"
        );
        assert_eq!(
            text.matches('\n').count(),
            3,
            "expected 3 lines, got:\n{text}"
        );
    }

    // ── Clap parsing of the hidden --insecure-skip-verify flag ──────

    /// Tiny wrapper to exercise [`UpdateArgs`] through clap without
    /// booting the full CLI. `UpdateArgs` is `clap::Args` (a subcommand
    /// args group); wrapping it in a `clap::Parser` is the documented
    /// pattern for unit-testing flag parsing.
    #[derive(clap::Parser, Debug)]
    #[command(name = "anvil-update-test", no_binary_name = true)]
    struct UpdateArgsParser {
        #[command(flatten)]
        args: UpdateArgs,
    }

    #[test]
    fn insecure_skip_verify_parses_even_though_hidden() {
        use clap::Parser;
        // `hide = true` only suppresses the flag from --help; clap must
        // still accept it when typed.
        let parsed = UpdateArgsParser::try_parse_from(["--check", "--insecure-skip-verify"])
            .expect("hidden flag must parse");

        assert!(parsed.args.check, "--check should be set");
        assert!(
            parsed.args.insecure_skip_verify,
            "--insecure-skip-verify should be set"
        );
        assert!(parsed.args.version.is_none(), "no --version expected");
        assert!(!parsed.args.force, "no --force expected");
    }

    #[test]
    fn unknown_flag_is_rejected_with_unknown_argument_kind() {
        use clap::Parser;
        let err = UpdateArgsParser::try_parse_from(["--this-flag-does-not-exist"])
            .expect_err("unknown flags must error");

        assert!(
            matches!(err.kind(), clap::error::ErrorKind::UnknownArgument),
            "expected UnknownArgument, got {:?}",
            err.kind()
        );
    }

    #[test]
    fn defaults_match_safe_posture() {
        use clap::Parser;
        // Defence against an accidental `#[arg(default_value_t = true)]`
        // on a security-relevant flag.
        let parsed = UpdateArgsParser::try_parse_from(std::iter::empty::<&str>())
            .expect("zero-arg form must parse");

        assert!(
            !parsed.args.insecure_skip_verify,
            "skip-verify defaults off"
        );
        assert!(!parsed.args.force, "force defaults off");
        assert!(!parsed.args.check, "check defaults off");
        assert!(parsed.args.version.is_none(), "version defaults to None");
    }
}
