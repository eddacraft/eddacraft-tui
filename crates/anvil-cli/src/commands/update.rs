use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::Context;

use crate::GlobalArgs;
use crate::commands::version::InstallMethod;

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

// These are independent CLI switches, not coupled state; modelling them as a
// state machine would obscure clap's composable flag surface.
#[allow(clippy::struct_excessive_bools)]
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

    /// Consent to a package-manager-owned update without prompting.
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Skip signature verification of the downloaded artefact. Dangerous —
    /// only use when the release public key is known to be temporarily
    /// unavailable and the user explicitly accepts the risk. Logs a
    /// loud warning.
    #[arg(long, hide = true)]
    pub insecure_skip_verify: bool,
}

pub fn run(args: &UpdateArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");

    // 1. Package-manager installs: defer to the package manager so the user
    //    gets the one command that will work, not a menu of options. Detected
    //    through the canonical install-method detector in `commands::version`.
    if let Some(command) =
        package_manager_command_for(crate::commands::version::detect_install_method())
    {
        return run_package_manager_update(current, args, global, command);
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

/// A package-manager operation that anvil may execute after explicit consent.
///
/// Keeping the executable and arguments as separate static fields makes this an
/// allowlist, not a shell-command surface. [`Command`] receives these values
/// directly; the human-readable form is never parsed or executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PackageManagerCommand {
    install_method: &'static str,
    display_name: &'static str,
    display_command: &'static str,
    executable: &'static str,
    argv: &'static [&'static str],
}

impl PackageManagerCommand {
    fn display(self) -> &'static str {
        self.display_command
    }
}

#[derive(Debug, PartialEq, Eq)]
struct PackageManagerExecution {
    success: bool,
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl PackageManagerExecution {
    #[cfg(test)]
    fn success(stdout: Vec<u8>, stderr: Vec<u8>) -> Self {
        Self {
            success: true,
            exit_code: Some(0),
            stdout,
            stderr,
        }
    }
}

fn package_manager_command_for(method: InstallMethod) -> Option<PackageManagerCommand> {
    match method {
        InstallMethod::Homebrew => Some(PackageManagerCommand {
            install_method: "homebrew",
            display_name: "Homebrew",
            display_command: "brew upgrade eddacraft/tap/anvil",
            executable: "brew",
            argv: &["upgrade", "eddacraft/tap/anvil"],
        }),
        InstallMethod::Winget => Some(PackageManagerCommand {
            install_method: "winget",
            display_name: "WinGet",
            display_command: "winget upgrade --id eddacraft.anvil",
            executable: "winget",
            argv: &["upgrade", "--id", "eddacraft.anvil"],
        }),
        InstallMethod::Scoop => Some(PackageManagerCommand {
            install_method: "scoop",
            display_name: "Scoop",
            display_command: "scoop update anvil",
            executable: "powershell.exe",
            argv: &[
                "-NoProfile",
                "-Command",
                "scoop update anvil; $anvilSucceeded = $?; $anvilExitCode = $LASTEXITCODE; if ($anvilSucceeded) { exit 0 }; if ($null -ne $anvilExitCode -and $anvilExitCode -ne 0) { exit $anvilExitCode }; exit 1",
            ],
        }),
        InstallMethod::CargoDist
        | InstallMethod::CargoInstall
        | InstallMethod::DevBuild
        | InstallMethod::Unknown => None,
    }
}

#[cfg(test)]
type PackageManager = InstallMethod;

#[cfg(test)]
fn package_manager_for_exe(path: &Path) -> Option<PackageManager> {
    let method = crate::commands::version::classify_exe_path(path);
    package_manager_command_for(method).map(|_| method)
}

fn run_package_manager_update(
    current: &str,
    args: &UpdateArgs,
    global: &GlobalArgs,
    command: PackageManagerCommand,
) -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    run_package_manager_update_with(
        current,
        args,
        global.json,
        command,
        &mut stdin.lock(),
        &mut stdout.lock(),
        &mut stderr.lock(),
        execute_package_manager_command,
    )
}

fn execute_package_manager_command(
    command: PackageManagerCommand,
    capture: bool,
) -> std::io::Result<PackageManagerExecution> {
    let mut child = Command::new(command.executable);
    child.args(command.argv);
    if capture {
        // Structured mode is non-interactive. A manager that unexpectedly
        // asks for more input receives EOF instead of corrupting or hanging a
        // JSON caller's pipeline.
        let output = child.stdin(Stdio::null()).output()?;
        Ok(PackageManagerExecution {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    } else {
        let status = child
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;
        Ok(PackageManagerExecution {
            success: status.success(),
            exit_code: status.code(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }
}

fn validate_package_manager_options(
    args: &UpdateArgs,
    command: PackageManagerCommand,
) -> anyhow::Result<()> {
    if args.version.is_some() {
        anyhow::bail!(
            "`--version` is not supported for {}-owned installs; this package manager can only select its configured latest version. Run `{}` instead.",
            command.display_name,
            command.display()
        );
    }
    if args.force {
        anyhow::bail!(
            "`--force` is not supported for {}-owned installs; anvil will not map it to a different package-manager operation. Run `{}` instead.",
            command.display_name,
            command.display()
        );
    }
    Ok(())
}

fn write_package_manager_check<W: Write, E: Write>(
    current: &str,
    json: bool,
    command: PackageManagerCommand,
    stdout: &mut W,
    stderr: &mut E,
) -> std::io::Result<()> {
    if json {
        writeln!(
            stdout,
            "{}",
            serde_json::json!({
                "current_version": current,
                "install_method": command.install_method,
                "action": "check",
                "message": format!(
                    "Installed via {}. Updates are managed by the package manager.",
                    command.display_name
                ),
                "upgrade_command": command.display(),
            })
        )
    } else {
        writeln!(
            stderr,
            "Installed via {}. Updates are managed by the package manager.",
            command.display_name
        )?;
        writeln!(stderr, "Upgrade command: {}", command.display())
    }
}

fn write_package_manager_execution_json<W: Write>(
    current: &str,
    command: PackageManagerCommand,
    action: &str,
    attempted: &str,
    execution: &PackageManagerExecution,
    stdout: &mut W,
) -> std::io::Result<()> {
    writeln!(
        stdout,
        "{}",
        serde_json::json!({
            "current_version": current,
            "install_method": command.install_method,
            "action": action,
            "upgrade_command": attempted,
            "exit_code": execution.exit_code,
            "manager_stdout": String::from_utf8_lossy(&execution.stdout),
            "manager_stderr": String::from_utf8_lossy(&execution.stderr),
        })
    )
}

fn write_package_manager_error_json<W: Write>(
    current: &str,
    command: PackageManagerCommand,
    attempted: &str,
    error: &str,
    stdout: &mut W,
) -> std::io::Result<()> {
    writeln!(
        stdout,
        "{}",
        serde_json::json!({
            "current_version": current,
            "install_method": command.install_method,
            "action": "failed",
            "upgrade_command": attempted,
            "error": error,
        })
    )
}

#[allow(clippy::too_many_arguments)]
fn run_package_manager_update_with<R, W, E, F>(
    current: &str,
    args: &UpdateArgs,
    json: bool,
    command: PackageManagerCommand,
    input: &mut R,
    stdout: &mut W,
    stderr: &mut E,
    execute: F,
) -> anyhow::Result<()>
where
    R: BufRead,
    W: Write,
    E: Write,
    F: FnOnce(PackageManagerCommand, bool) -> std::io::Result<PackageManagerExecution>,
{
    if let Err(error) = validate_package_manager_options(args, command) {
        if json {
            write_package_manager_error_json(
                current,
                command,
                command.display(),
                &error.to_string(),
                stdout,
            )?;
            return Err(crate::output::AlreadyReported.into());
        }
        return Err(error);
    }

    if args.check {
        write_package_manager_check(current, json, command, stdout, stderr)?;
        return Ok(());
    }

    if json && !args.yes {
        let error = "package-manager updates in JSON mode require explicit consent; rerun with `anvil update --yes --json`";
        write_package_manager_error_json(current, command, command.display(), error, stdout)?;
        return Err(crate::output::AlreadyReported.into());
    }

    if !json {
        writeln!(stderr, "Detected package manager: {}", command.display_name)?;
        writeln!(stderr, "Command: {}", command.display())?;
    }

    if !args.yes {
        write!(stderr, "Run it now? [y/N] ")?;
        stderr.flush()?;
        let mut answer = String::new();
        input.read_line(&mut answer)?;
        if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
            writeln!(stderr, "Update declined; no changes made.")?;
            return Ok(());
        }
    }

    let attempted = command.display();
    let execution = match execute(command, json) {
        Ok(execution) => execution,
        Err(error) => {
            if json {
                write_package_manager_error_json(
                    current,
                    command,
                    attempted,
                    &error.to_string(),
                    stdout,
                )?;
                return Err(crate::output::AlreadyReported.into());
            }
            return Err(error)
                .with_context(|| format!("failed to run package-manager command `{attempted}`"));
        }
    };
    if !execution.success {
        let code = execution.exit_code.map_or_else(
            || "terminated by signal".to_string(),
            |code| format!("exit code {code}"),
        );
        if json {
            write_package_manager_execution_json(
                current, command, "failed", attempted, &execution, stdout,
            )?;
            return Err(crate::output::AlreadyReported.into());
        }
        anyhow::bail!("package-manager command `{attempted}` failed with {code}");
    }

    if json {
        write_package_manager_execution_json(
            current,
            command,
            "manager_completed",
            attempted,
            &execution,
            stdout,
        )?;
    } else {
        writeln!(
            stderr,
            "Update completed successfully via {}.",
            command.display_name
        )?;
    }
    Ok(())
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

/// Build the sidecar updater command.
///
/// The sidecar (`eddacraft-anvil-update`, a cargo-dist updater) understands the
/// standard `--check` / `--version` / `--force` flags but NOT anvil's custom
/// `--insecure-skip-verify`, so that flag is deliberately *not* forwarded here —
/// forwarding an unknown flag would make the updater error out. `run_sidecar`
/// warns the operator instead (issue #1735). Extracted so the forwarded arg
/// surface is unit-testable without spawning the real updater.
fn build_sidecar_command(path: &Path, args: &UpdateArgs) -> Command {
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
    cmd
}

/// Loud warning when `--insecure-skip-verify` is passed but the sidecar updater
/// cannot honour it (it has no such flag).
///
/// `--insecure-skip-verify` only controls anvil's *own* signature check on the
/// library-fallback path; the sidecar (`eddacraft-anvil-update`) runs its own
/// updater logic and never anvil's signature step, so the flag is meaningless
/// there. Per ADR-045 and the operator-config silent-default class (#1735) we
/// never drop an operator-supplied security flag silently — so say so loudly
/// and name the sidecar so the operator can remove it to reach the path where
/// the flag does apply.
fn write_sidecar_skip_verify_ignored_warning<W: std::io::Write>(
    path: &Path,
    w: &mut W,
) -> std::io::Result<()> {
    writeln!(
        w,
        "WARNING: --insecure-skip-verify is not supported by the sidecar updater ({SIDECAR_NAME}); the flag has no effect on this path."
    )?;
    writeln!(
        w,
        "         It only governs anvil's own signature check, which the sidecar does not run. To use it, remove the sidecar ({}) so anvil's built-in updater runs instead.",
        path.display()
    )?;
    Ok(())
}

/// Emit the sidecar skip-verify warning only when it is actually meaningful:
/// the flag was set AND this is a real install. `--check` downloads nothing, so
/// the flag is a no-op there on every path and a warning would be pure noise.
/// Split from `run_sidecar` so the call-site decision is unit-testable without
/// spawning the updater.
fn maybe_warn_sidecar_skip_verify<W: std::io::Write>(
    path: &Path,
    args: &UpdateArgs,
    w: &mut W,
) -> std::io::Result<()> {
    if args.insecure_skip_verify && !args.check {
        write_sidecar_skip_verify_ignored_warning(path, w)?;
    }
    Ok(())
}

fn run_sidecar(path: &Path, args: &UpdateArgs) -> anyhow::Result<()> {
    // Surface a dropped --insecure-skip-verify loudly rather than silently
    // (#1735). `.expect` matches the library path's ADR-045 convention: the
    // security warning is a contract, so a failed stderr write is fatal.
    maybe_warn_sidecar_skip_verify(path, args, &mut std::io::stderr().lock())
        .expect("stderr write for ADR-045 sidecar skip-verify warning");

    let status = build_sidecar_command(path, args)
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
/// App name must match cargo-dist / install receipt (`eddacraft-anvil`), not
/// the binary short name `anvil` (CIB-229).
pub(crate) const DIST_APP_NAME: &str = "eddacraft-anvil";
/// Receipt name written before the package was renamed. Probed as a fallback
/// so an install that predates the rename still reports its real provenance.
pub(crate) const LEGACY_DIST_APP_NAME: &str = "anvil";

/// Load the cargo-dist install receipt (written by the shell / `PowerShell`
/// installers) into `updater`, preferring the current app name and falling
/// back to the legacy one. Returns whether either succeeded.
///
/// This is the **single** receipt lookup in the CLI, shared with
/// [`crate::commands::version::detect_install_method`]. `version` used to
/// resolve the receipt path itself via `dirs::config_dir()`, which is not
/// where the receipt is written on Windows (`%LOCALAPPDATA%`, read as an
/// environment variable) or macOS (`~/.config`) — so `version` reported
/// `cargo install` for the very installs `update --check` could update, and
/// handed a toolchain-only upgrade command to people with no toolchain
/// (CIB-315). Routing both surfaces through axoupdater makes that
/// divergence unrepresentable rather than merely fixed.
pub(crate) fn load_dist_receipt(updater: &mut axoupdater::AxoUpdater) -> bool {
    updater.load_receipt().is_ok() || updater.load_receipt_as(LEGACY_DIST_APP_NAME).is_ok()
}

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

    let mut updater = axoupdater::AxoUpdater::new_for(DIST_APP_NAME);

    // Try loading the cargo-dist install receipt (created by shell/powershell installers).
    // Prefer the package name; also try legacy `anvil` receipt paths.
    let receipt_loaded = load_dist_receipt(&mut updater);

    // If missing (dev build, manual install), configure the release source
    // manually *and* set install_prefix so is_update_needed is configured
    // (axoupdater NotConfigured when prefix is absent).
    if !receipt_loaded {
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
            app_name: DIST_APP_NAME.to_string(),
        });
        // Prefix = parent of the running binary when available (often …/bin).
        if let Ok(exe) = std::env::current_exe()
            && let Some(parent) = exe.parent()
            && let Some(s) = parent.to_str()
        {
            updater.set_install_dir(s);
        }
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
        "         The downloaded installer will run without proof it came from the anvil release key."
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
        // Match the prior `eprintln!` panic-on-stderr-failure contract —
        // ADR-045 requires this warning to surface. Silent drop would
        // be the same bug ADR-045 calls "verification ships as a silent
        // no-op". Holding the lock across both writelns is intentional:
        // atomic multi-line output, no interleave with concurrent stderr
        // writers (axoupdater / tracing).
        write_skip_verify_warning(&mut std::io::stderr().lock())
            .expect("stderr write must succeed for ADR-045 skip-verify warning");
        return Ok(None);
    }

    if signature::is_using_dev_public_key() {
        // Same panic-on-fail contract — Council CRITICAL per ADR-045.
        write_dev_key_warning(&mut std::io::stderr().lock())
            .expect("stderr write must succeed for ADR-045 dev-key warning");
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
                    crate::commands::version::WINDOWS_INSTALLER_UPGRADE,
                ],
            })
        );
    } else {
        println!("Current version: {current}");
        println!("{message}");
    }
}

/// Exposed to `version` so a test can prove the two surfaces print the same
/// Windows upgrade command; a user who runs both must not get two answers.
pub(crate) fn windows_unsupported_message() -> &'static str {
    "Self-update is not supported on Windows in this release.\n\
     \n\
     To upgrade, use one of:\n  \
     - winget upgrade --id eddacraft.anvil\n  \
     - Re-run the PowerShell installer:\n      \
         irm https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.ps1 | iex\n\
     \n\
     If the installer fails with \"file is being used by another process\",\n\
     close any editor running the anvil MCP server (Cursor, Claude Code)\n\
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

    // Package-manager command allowlist.

    #[test]
    fn package_manager_commands_are_static_executable_and_argv_pairs() {
        use crate::commands::version::InstallMethod;

        let cases = [
            (
                InstallMethod::Homebrew,
                "Homebrew",
                "brew",
                &["upgrade", "eddacraft/tap/anvil"][..],
                "brew upgrade eddacraft/tap/anvil",
            ),
            (
                InstallMethod::Winget,
                "WinGet",
                "winget",
                &["upgrade", "--id", "eddacraft.anvil"][..],
                "winget upgrade --id eddacraft.anvil",
            ),
            (
                InstallMethod::Scoop,
                "Scoop",
                "powershell.exe",
                &[
                    "-NoProfile",
                    "-Command",
                    "scoop update anvil; $anvilSucceeded = $?; $anvilExitCode = $LASTEXITCODE; if ($anvilSucceeded) { exit 0 }; if ($null -ne $anvilExitCode -and $anvilExitCode -ne 0) { exit $anvilExitCode }; exit 1",
                ][..],
                "scoop update anvil",
            ),
        ];

        for (method, display_name, executable, argv, display_command) in cases {
            let command = package_manager_command_for(method).expect("supported manager");
            assert_eq!(command.display_name, display_name);
            assert_eq!(command.executable, executable);
            assert_eq!(command.argv, argv);
            assert_eq!(command.display(), display_command);
        }
    }

    #[cfg(windows)]
    #[test]
    fn scoop_powershell_launcher_resolves_on_windows() {
        let command = package_manager_command_for(InstallMethod::Scoop).unwrap();
        let status = Command::new(command.executable)
            .args(["-NoProfile", "-Command", "exit 0"])
            .status()
            .expect("the selected Scoop PowerShell launcher must resolve");
        assert!(status.success());
    }

    #[cfg(windows)]
    #[test]
    fn scoop_powershell_wrapper_preserves_exit_semantics() {
        let command = package_manager_command_for(InstallMethod::Scoop).unwrap();
        let script = command.argv.get(2).expect("Scoop PowerShell script");
        let forwarding = script
            .strip_prefix("scoop update anvil; ")
            .expect("fixed Scoop command precedes exit forwarding");
        let run_probe = |probe: &str| {
            let script = format!("{probe}; {forwarding}");
            Command::new(command.executable)
                .args(["-NoProfile", "-Command", &script])
                .status()
                .expect("PowerShell must run the exit-code probe")
                .code()
        };

        assert_eq!(run_probe("cmd.exe /D /C exit 37"), Some(37));
        assert_eq!(
            run_probe("cmd.exe /D /C exit 37; $anvilProbe = 1"),
            Some(0),
            "PowerShell success must override a stale native exit code"
        );
        assert_eq!(
            run_probe("Write-Error 'probe' -ErrorAction SilentlyContinue"),
            Some(1),
            "PowerShell-only failure must use the fallback exit code"
        );
    }

    #[test]
    fn non_package_manager_install_methods_have_no_command() {
        use crate::commands::version::InstallMethod;

        for method in [
            InstallMethod::CargoDist,
            InstallMethod::CargoInstall,
            InstallMethod::DevBuild,
            InstallMethod::Unknown,
        ] {
            assert!(package_manager_command_for(method).is_none());
        }
    }

    fn consent_test_args(yes: bool) -> UpdateArgs {
        UpdateArgs {
            check: false,
            version: None,
            force: false,
            yes,
            insecure_skip_verify: false,
        }
    }

    #[test]
    fn interactive_explicit_yes_prompts_then_executes() {
        let command = package_manager_command_for(InstallMethod::Homebrew).unwrap();
        let mut input = "yes\n".as_bytes();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let executed = std::cell::Cell::new(false);

        run_package_manager_update_with(
            "0.9.0-beta",
            &consent_test_args(false),
            false,
            command,
            &mut input,
            &mut stdout,
            &mut stderr,
            |actual, capture| {
                executed.set(true);
                assert_eq!(actual, command);
                assert!(!capture, "human mode must stream manager output");
                Ok(PackageManagerExecution::success(Vec::new(), Vec::new()))
            },
        )
        .expect("explicit yes should execute");

        assert!(executed.get());
        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stderr.contains("Detected package manager: Homebrew"));
        assert!(stderr.contains("Command: brew upgrade eddacraft/tap/anvil"));
        assert!(stderr.contains("Run it now? [y/N]"));
        assert!(stderr.contains("Update completed successfully via Homebrew."));
        assert!(stdout.is_empty(), "human results belong on stderr");
    }

    #[test]
    fn decline_and_eof_are_clean_no_ops() {
        let command = package_manager_command_for(InstallMethod::Homebrew).unwrap();
        for answer in ["n\n", "", "not now\n"] {
            let mut input = answer.as_bytes();
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let executed = std::cell::Cell::new(false);
            run_package_manager_update_with(
                "0.9.0-beta",
                &consent_test_args(false),
                false,
                command,
                &mut input,
                &mut stdout,
                &mut stderr,
                |_, _| {
                    executed.set(true);
                    Ok(PackageManagerExecution::success(Vec::new(), Vec::new()))
                },
            )
            .expect("decline and EOF are clean no-ops");

            assert!(!executed.get(), "answer {answer:?} must not spawn");
            assert!(stdout.is_empty());
            assert!(
                String::from_utf8(stderr)
                    .unwrap()
                    .contains("no changes made")
            );
        }
    }

    #[test]
    fn yes_flag_skips_prompt_and_executes_for_non_interactive_callers() {
        let command = package_manager_command_for(InstallMethod::Scoop).unwrap();
        let mut input = "no\n".as_bytes();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let executed = std::cell::Cell::new(false);

        run_package_manager_update_with(
            "0.9.0-beta",
            &consent_test_args(true),
            false,
            command,
            &mut input,
            &mut stdout,
            &mut stderr,
            |_, capture| {
                executed.set(true);
                assert!(!capture);
                Ok(PackageManagerExecution::success(Vec::new(), Vec::new()))
            },
        )
        .unwrap();

        assert!(executed.get());
        let stderr = String::from_utf8(stderr).unwrap();
        assert!(!stderr.contains("Run it now?"));
        assert!(stderr.contains("Update completed successfully via Scoop."));
        assert!(stdout.is_empty(), "human results belong on stderr");
    }

    #[test]
    fn check_with_yes_is_read_only_and_reports_package_manager_guidance() {
        let command = package_manager_command_for(InstallMethod::Winget).unwrap();
        let args = UpdateArgs {
            check: true,
            yes: true,
            ..consent_test_args(false)
        };
        let mut input = "yes\n".as_bytes();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let executed = std::cell::Cell::new(false);

        run_package_manager_update_with(
            "0.9.0-beta",
            &args,
            false,
            command,
            &mut input,
            &mut stdout,
            &mut stderr,
            |_, _| {
                executed.set(true);
                Ok(PackageManagerExecution::success(Vec::new(), Vec::new()))
            },
        )
        .expect("check should report without executing");

        assert!(!executed.get(), "--check must never spawn");
        assert!(stdout.is_empty(), "human results belong on stderr");
        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stderr.contains("Installed via WinGet"));
        assert!(stderr.contains("winget upgrade --id eddacraft.anvil"));
        assert!(!stderr.contains("Run it now?"));
    }

    #[test]
    fn json_without_yes_refuses_without_prompting_or_executing() {
        let command = package_manager_command_for(InstallMethod::Scoop).unwrap();
        let mut input = "yes\n".as_bytes();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let executed = std::cell::Cell::new(false);

        let err = run_package_manager_update_with(
            "0.9.0-beta",
            &consent_test_args(false),
            true,
            command,
            &mut input,
            &mut stdout,
            &mut stderr,
            |_, _| {
                executed.set(true);
                Ok(PackageManagerExecution::success(Vec::new(), Vec::new()))
            },
        )
        .expect_err("JSON execution requires --yes");

        assert!(err.is::<crate::output::AlreadyReported>());
        assert!(!executed.get());
        let stdout = String::from_utf8(stdout).unwrap();
        let document: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(document["action"], "failed");
        assert_eq!(document["install_method"], "scoop");
        assert!(document["error"].as_str().unwrap().contains("--yes"));
        assert_eq!(stdout.lines().count(), 1);
        assert!(stderr.is_empty(), "JSON failure suppresses human stderr");
    }

    #[test]
    fn json_with_yes_captures_child_output_in_one_document() {
        let command = package_manager_command_for(InstallMethod::Scoop).unwrap();
        let mut input = "ignored\n".as_bytes();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let captured = std::cell::Cell::new(false);

        run_package_manager_update_with(
            "0.9.0-beta",
            &consent_test_args(true),
            true,
            command,
            &mut input,
            &mut stdout,
            &mut stderr,
            |actual, capture| {
                assert_eq!(actual, command);
                captured.set(capture);
                Ok(PackageManagerExecution::success(
                    b"manager stdout\n".to_vec(),
                    b"manager stderr\n".to_vec(),
                ))
            },
        )
        .expect("JSON --yes should execute");

        assert!(captured.get(), "JSON mode must capture manager output");
        let stdout = String::from_utf8(stdout).unwrap();
        let document: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(document["action"], "manager_completed");
        assert_eq!(document["install_method"], "scoop");
        assert_eq!(document["manager_stdout"], "manager stdout\n");
        assert_eq!(document["manager_stderr"], "manager stderr\n");
        assert_eq!(stdout.lines().count(), 1, "stdout must be one JSON line");
        assert!(stderr.is_empty(), "JSON success suppresses human stderr");
    }

    #[test]
    fn json_check_emits_one_document_and_no_human_stderr() {
        let command = package_manager_command_for(InstallMethod::Winget).unwrap();
        let args = UpdateArgs {
            check: true,
            ..consent_test_args(false)
        };
        let mut input = "ignored\n".as_bytes();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        run_package_manager_update_with(
            "0.9.0-beta",
            &args,
            true,
            command,
            &mut input,
            &mut stdout,
            &mut stderr,
            |_, _| panic!("--check must not execute"),
        )
        .unwrap();

        let stdout = String::from_utf8(stdout).unwrap();
        let document: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(document["action"], "check");
        assert_eq!(stdout.lines().count(), 1);
        assert!(stderr.is_empty());
    }

    #[test]
    fn package_manager_paths_reject_version_and_force_actionably() {
        let command = package_manager_command_for(InstallMethod::Homebrew).unwrap();
        let cases = [
            UpdateArgs {
                version: Some("0.8.1-beta".to_string()),
                ..consent_test_args(true)
            },
            UpdateArgs {
                force: true,
                ..consent_test_args(true)
            },
        ];

        for args in cases {
            let mut input = "".as_bytes();
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let executed = std::cell::Cell::new(false);
            let err = run_package_manager_update_with(
                "0.9.0-beta",
                &args,
                false,
                command,
                &mut input,
                &mut stdout,
                &mut stderr,
                |_, _| {
                    executed.set(true);
                    Ok(PackageManagerExecution::success(Vec::new(), Vec::new()))
                },
            )
            .expect_err("unsupported package-manager option must fail");

            let message = err.to_string();
            let flag = if args.version.is_some() {
                "--version"
            } else {
                "--force"
            };
            assert!(message.contains(flag), "missing {flag}: {message}");
            assert!(message.contains("Homebrew"), "missing owner: {message}");
            assert!(
                message.contains("brew upgrade eddacraft/tap/anvil"),
                "missing actionable command: {message}"
            );
            assert!(!executed.get());
        }
    }

    #[test]
    fn json_unsupported_option_emits_one_failure_document() {
        let command = package_manager_command_for(InstallMethod::Homebrew).unwrap();
        let args = UpdateArgs {
            force: true,
            ..consent_test_args(true)
        };
        let mut input = "".as_bytes();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let err = run_package_manager_update_with(
            "0.9.0-beta",
            &args,
            true,
            command,
            &mut input,
            &mut stdout,
            &mut stderr,
            |_, _| panic!("invalid options must not execute"),
        )
        .expect_err("--force must fail on package-manager paths");

        assert!(err.is::<crate::output::AlreadyReported>());
        let stdout = String::from_utf8(stdout).unwrap();
        let document: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(document["action"], "failed");
        assert!(document["error"].as_str().unwrap().contains("--force"));
        assert_eq!(stdout.lines().count(), 1);
        assert!(stderr.is_empty(), "JSON failure suppresses human stderr");
    }

    #[test]
    fn json_nonzero_exit_emits_one_failure_document_and_propagates() {
        let command = package_manager_command_for(InstallMethod::Winget).unwrap();
        let mut input = "".as_bytes();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let err = run_package_manager_update_with(
            "0.9.0-beta",
            &consent_test_args(true),
            true,
            command,
            &mut input,
            &mut stdout,
            &mut stderr,
            |_, capture| {
                assert!(capture);
                Ok(PackageManagerExecution {
                    success: false,
                    exit_code: Some(23),
                    stdout: b"partial output\n".to_vec(),
                    stderr: b"manager failure\n".to_vec(),
                })
            },
        )
        .expect_err("non-zero child exit must propagate");

        assert!(err.is::<crate::output::AlreadyReported>());
        let stdout = String::from_utf8(stdout).unwrap();
        let document: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(document["action"], "failed");
        assert_eq!(document["exit_code"], 23);
        assert_eq!(document["manager_stdout"], "partial output\n");
        assert_eq!(document["manager_stderr"], "manager failure\n");
        assert_eq!(stdout.lines().count(), 1);
        assert!(stderr.is_empty(), "JSON failure suppresses human stderr");
    }

    #[test]
    fn missing_manager_executable_is_named_and_emits_json_failure() {
        let command = package_manager_command_for(InstallMethod::Homebrew).unwrap();
        let mut input = "".as_bytes();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let err = run_package_manager_update_with(
            "0.9.0-beta",
            &consent_test_args(true),
            true,
            command,
            &mut input,
            &mut stdout,
            &mut stderr,
            |_, _| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "executable not found",
                ))
            },
        )
        .expect_err("missing executable must propagate");

        assert!(err.is::<crate::output::AlreadyReported>());
        let stdout = String::from_utf8(stdout).unwrap();
        let document: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(document["action"], "failed");
        assert!(
            document["error"]
                .as_str()
                .unwrap()
                .contains("executable not found")
        );
        assert_eq!(stdout.lines().count(), 1);
        assert!(stderr.is_empty(), "JSON failure suppresses human stderr");
    }

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
            yes: false,
            insecure_skip_verify: false,
        }
    }

    fn sidecar_args(args: &UpdateArgs) -> Vec<String> {
        build_sidecar_command(std::path::Path::new("fake-updater"), args)
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn sidecar_command_check_flag() {
        assert_eq!(sidecar_args(&args_with(true, None, false)), vec!["--check"]);
    }

    #[test]
    fn sidecar_command_version_flag() {
        assert_eq!(
            sidecar_args(&args_with(false, Some("0.4.0"), false)),
            vec!["--version", "0.4.0"]
        );
    }

    #[test]
    fn sidecar_command_force_flag() {
        assert_eq!(sidecar_args(&args_with(false, None, true)), vec!["--force"]);
    }

    #[test]
    fn sidecar_command_omits_insecure_skip_verify() {
        // The sidecar updater has no such flag; forwarding an unknown flag
        // would break it, so it must never be forwarded (#1735). Supported
        // flags alongside it still are.
        let args = UpdateArgs {
            check: false,
            version: None,
            force: true,
            yes: false,
            insecure_skip_verify: true,
        };
        let got = sidecar_args(&args);
        assert!(
            !got.iter().any(|a| a == "--insecure-skip-verify"),
            "insecure-skip-verify must not be forwarded to the sidecar: {got:?}"
        );
        assert!(
            got.iter().any(|a| a == "--force"),
            "supported flags still forwarded: {got:?}"
        );
    }

    fn warn_output(path: &str, args: &UpdateArgs) -> String {
        let mut buf = Vec::new();
        maybe_warn_sidecar_skip_verify(std::path::Path::new(path), args, &mut buf)
            .expect("infallible for Vec");
        String::from_utf8(buf).expect("utf-8")
    }

    #[test]
    fn sidecar_skip_verify_ignored_warning_is_loud() {
        let mut buf = Vec::new();
        write_sidecar_skip_verify_ignored_warning(
            std::path::Path::new("/opt/anvil/eddacraft-anvil-update"),
            &mut buf,
        )
        .expect("infallible for Vec");
        let text = std::str::from_utf8(&buf).expect("utf-8");
        assert!(
            text.contains("WARNING: --insecure-skip-verify"),
            "header line missing; got:\n{text}"
        );
        assert!(
            text.contains("not supported by the sidecar updater"),
            "reason missing; got:\n{text}"
        );
        assert!(
            text.contains(SIDECAR_NAME),
            "should name the sidecar binary; got:\n{text}"
        );
        assert!(
            text.contains("/opt/anvil/eddacraft-anvil-update"),
            "should name the resolved sidecar path so the operator can remove it; got:\n{text}"
        );
    }

    #[test]
    fn warns_on_real_install_with_skip_verify() {
        // flag set + actual install (not --check) → loud warning.
        let args = UpdateArgs {
            check: false,
            version: None,
            force: false,
            yes: false,
            insecure_skip_verify: true,
        };
        assert!(
            warn_output("/p/eddacraft-anvil-update", &args)
                .contains("WARNING: --insecure-skip-verify"),
            "expected a warning on the install + skip-verify sidecar path"
        );
    }

    #[test]
    fn silent_on_check_even_with_skip_verify() {
        // --check downloads nothing, so the flag is a no-op on every path — no
        // warning noise.
        let args = UpdateArgs {
            check: true,
            version: None,
            force: false,
            yes: false,
            insecure_skip_verify: true,
        };
        assert!(
            warn_output("/p/eddacraft-anvil-update", &args).is_empty(),
            "--check is read-only; skip-verify warning must not fire"
        );
    }

    #[test]
    fn silent_when_skip_verify_absent() {
        assert!(
            warn_output("/p/eddacraft-anvil-update", &args_with(false, None, true)).is_empty(),
            "no warning when --insecure-skip-verify is not set"
        );
    }

    #[test]
    fn windows_unsupported_message_lists_alternatives() {
        let msg = windows_unsupported_message();
        assert!(msg.contains("winget upgrade --id eddacraft.anvil"));
        assert!(msg.contains("eddacraft-anvil-installer.ps1"));
        assert!(msg.contains("anvil MCP server"));
    }

    #[test]
    fn sidecar_command_no_flags() {
        assert!(sidecar_args(&args_with(false, None, false)).is_empty());
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
            text.contains("without proof it came from the anvil release key"),
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

        // Pin the `WARNING:` token at the header so a tone-downgrade
        // (e.g. `note:` / `info:`) is caught — log scrapers and
        // ADR-045 compliance grep patterns rely on this prefix.
        assert!(
            text.starts_with("WARNING:"),
            "header must lead with `WARNING:`; got:\n{text}"
        );
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

    /// Simplified parser wrapper that approximates the production flag
    /// set for `UpdateArgs` plus the `global = true` flags from
    /// `GlobalArgs`, both flattened at the same level. This is **not**
    /// a faithful reproduction of the production mount shape — in
    /// `main.rs` the real CLI has `GlobalArgs` flattened on the
    /// top-level `Cli` and `UpdateArgs` mounted as a `Commands::Update`
    /// subcommand. Flag *parsing* of the combinations exercised here
    /// reaches the same `bool`/`Option` slots, but subcommand parse
    /// semantics (flag-placement-before-subcommand, subcommand-level
    /// conflicts, etc.) are not exercised. Those land on the
    /// integration tests against the real binary.
    ///
    /// Note: `hide = true` posture on `--insecure-skip-verify` is
    /// pinned by `update_help_advertises_insecure_skip_verify_only_implicitly`
    /// in `tests/update_resolution_chain.rs` — clap's `try_parse_from`
    /// can't differentiate hidden vs. visible flags at parse time.
    #[derive(clap::Parser, Debug)]
    #[command(name = "anvil-update-test", no_binary_name = true)]
    struct UpdateArgsParser {
        #[command(flatten)]
        global: GlobalArgs,
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
    fn insecure_skip_verify_composes_with_global_args() {
        use clap::Parser;
        // The realistic operator-typed form. `--json` is a `global = true`
        // arg on `GlobalArgs`; if the test wrapper only had `UpdateArgs`,
        // clap would (correctly) reject it as `UnknownArgument` and this
        // test would be lying about coverage.
        let parsed =
            UpdateArgsParser::try_parse_from(["--json", "--check", "--insecure-skip-verify"])
                .expect("global args must compose with update-specific flags");

        assert!(parsed.global.json, "--json should propagate");
        assert!(parsed.args.check, "--check should be set");
        assert!(parsed.args.insecure_skip_verify);
    }

    #[test]
    fn insecure_skip_verify_composes_with_force() {
        use clap::Parser;
        // Highest-stakes combination: bypass signature AND force update
        // past the `is_update_needed` short-circuit. Pinning this so a
        // future refactor cannot accidentally invert the precedence or
        // make `--force` mutually exclusive with `--insecure-skip-verify`.
        let parsed = UpdateArgsParser::try_parse_from(["--force", "--insecure-skip-verify"])
            .expect("force + skip-verify must compose");

        assert!(parsed.args.force);
        assert!(parsed.args.insecure_skip_verify);
    }

    #[test]
    fn insecure_skip_verify_composes_with_version() {
        use clap::Parser;
        // `--insecure-skip-verify` returns `Ok(None)` from
        // `verify_pending_install` before the trusted-comment
        // downgrade-vector guard runs. That is intentional — operator
        // opted out — but the parser-level composition must not become
        // mutually exclusive by accident.
        let parsed =
            UpdateArgsParser::try_parse_from(["--insecure-skip-verify", "--version", "v0.7.0"])
                .expect("version + skip-verify must compose");

        assert!(parsed.args.insecure_skip_verify);
        assert_eq!(parsed.args.version.as_deref(), Some("v0.7.0"));
    }

    #[test]
    fn unknown_flag_is_rejected_with_unknown_argument_kind() {
        use clap::Parser;
        // Narrower than the binary-level `update_unknown_flag_is_rejected_by_clap`
        // in `tests/update_resolution_chain.rs` — this pins the
        // `ErrorKind` discriminant; the integration test pins the
        // emitted stderr text on the real binary.
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
        assert!(!parsed.args.yes, "yes defaults off");
        assert!(parsed.args.version.is_none(), "version defaults to None");
    }
}
