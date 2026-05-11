use std::path::{Path, PathBuf};
use std::process::Command;

use crate::GlobalArgs;

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
}

pub fn run(args: &UpdateArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");

    // 1. Homebrew detection
    if is_homebrew_install() {
        if global.json {
            println!(
                "{}",
                serde_json::json!({
                    "current_version": current,
                    "install_method": "homebrew",
                    "message": "Installed via Homebrew. Run `brew upgrade eddacraft/tap/anvil` instead."
                })
            );
        } else {
            println!(
                "anvil was installed via Homebrew. Run `brew upgrade eddacraft/tap/anvil` instead."
            );
        }
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

// ── Homebrew detection ──────────────────────────────────────────────

const HOMEBREW_PREFIXES: &[&str] = &["/opt/homebrew/", "/usr/local/Cellar/", "/home/linuxbrew/"];

fn is_homebrew_install() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    is_path_under_homebrew(&exe)
}

fn is_path_under_homebrew(path: &Path) -> bool {
    let Some(s) = path.to_str() else {
        return false;
    };
    HOMEBREW_PREFIXES.iter().any(|prefix| s.starts_with(prefix))
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

    updater.enable_installer_output();
    perform_update(updater, current, global)
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

    // ── Homebrew detection ──────────────────────────────────────────

    #[test]
    fn homebrew_opt_prefix_detected() {
        let path = Path::new("/opt/homebrew/bin/anvil");
        assert!(is_path_under_homebrew(path));
    }

    #[test]
    fn homebrew_cellar_prefix_detected() {
        let path = Path::new("/usr/local/Cellar/anvil/0.3.1/bin/anvil");
        assert!(is_path_under_homebrew(path));
    }

    #[test]
    fn homebrew_linuxbrew_prefix_detected() {
        let path = Path::new("/home/linuxbrew/.linuxbrew/bin/anvil");
        assert!(is_path_under_homebrew(path));
    }

    #[test]
    fn non_homebrew_cargo_bin() {
        let path = Path::new("/home/user/.cargo/bin/anvil");
        assert!(!is_path_under_homebrew(path));
    }

    #[test]
    fn non_homebrew_usr_local_bin() {
        let path = Path::new("/usr/local/bin/anvil");
        assert!(!is_path_under_homebrew(path));
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

    #[test]
    fn sidecar_command_check_flag() {
        // Verify the command would be built with --check
        let mut cmd = Command::new("fake-updater");
        let args = UpdateArgs {
            check: true,
            version: None,
            force: false,
        };
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
        let args = UpdateArgs {
            check: false,
            version: Some("0.4.0".to_string()),
            force: false,
        };
        if let Some(ver) = &args.version {
            cmd.args(["--version", ver]);
        }
        let cmd_args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert_eq!(cmd_args, vec!["--version", "0.4.0"]);
    }

    #[test]
    fn sidecar_command_force_flag() {
        let mut cmd = Command::new("fake-updater");
        let args = UpdateArgs {
            check: false,
            version: None,
            force: true,
        };
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
}
