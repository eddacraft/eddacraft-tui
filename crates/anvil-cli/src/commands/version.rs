//! `anvil version` — install-method-aware version surface (LAUNCH-013).
//!
//! Prints the current binary version, the latest available release
//! version (if reachable), update availability, the detected install
//! method, and the recommended upgrade command. Network failures are
//! non-fatal — the local version is always printed.
//!
//! Detection covers Homebrew, Scoop, `WinGet`, the `cargo-dist`
//! installer / `PowerShell` installer (via the install receipt), and
//! unknown / manual installs. Older direct installs that predate `anvil update`
//! are advised to rerun the latest installer rather than pointing at
//! a missing subcommand.

use std::path::Path;

use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

const GITHUB_OWNER: &str = "eddacraft";
const GITHUB_REPO: &str = "anvil-001";

#[derive(Debug, Args)]
pub struct VersionArgs {
    /// Skip the network probe for the latest release version. Useful
    /// in CI / sandboxed environments where outbound HTTPS is blocked.
    #[arg(long)]
    pub offline: bool,
}

/// Install method detected for the running binary. The variant is
/// part of the JSON output's `install_method` field; surfaces map it
/// to a recommended upgrade command via [`upgrade_command_for`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallMethod {
    Homebrew,
    Scoop,
    Winget,
    /// cargo-dist installer (shell or PowerShell) — detected via the
    /// install receipt that those installers write next to the binary.
    CargoDist,
    /// `cargo install eddacraft-anvil` — binary lives under the
    /// user's `CARGO_HOME/bin` (typically `~/.cargo/bin/`). Detected
    /// by path prefix; cargo does not write a receipt the way
    /// cargo-dist does.
    CargoInstall,
    /// Build artefact under a `target/` directory — almost always a
    /// developer build. Treated separately from `unknown` so the
    /// upgrade hint nudges the user toward `cargo build` rather than
    /// claiming "rerun the installer".
    DevBuild,
    /// Direct download / manual placement / older install that
    /// predates the cargo-dist receipt.
    Unknown,
}

impl InstallMethod {
    pub fn label(self) -> &'static str {
        match self {
            InstallMethod::Homebrew => "homebrew",
            InstallMethod::Scoop => "scoop",
            InstallMethod::Winget => "winget",
            InstallMethod::CargoDist => "cargo_dist",
            InstallMethod::CargoInstall => "cargo_install",
            InstallMethod::DevBuild => "dev_build",
            InstallMethod::Unknown => "unknown",
        }
    }
}

pub fn run(args: &VersionArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let install_method = detect_install_method();
    let upgrade_command = upgrade_command_for(install_method);

    let latest = if args.offline {
        None
    } else {
        fetch_latest_version_quiet()
    };
    let update_available = match latest.as_deref() {
        Some(l) => is_newer_semver(l, &current),
        None => false,
    };

    if global.json {
        let payload = VersionJson {
            current_version: &current,
            latest_version: latest.as_deref(),
            update_available,
            install_method: install_method.label(),
            upgrade_command,
        };
        let out = serde_json::to_string_pretty(&payload)?;
        println!("{out}");
    } else {
        print_human(
            &current,
            latest.as_deref(),
            update_available,
            install_method,
        );
    }

    Ok(())
}

/// JSON shape locked for tooling consumers. Adding fields is allowed;
/// renames or removals are breaking.
#[derive(Serialize)]
struct VersionJson<'a> {
    current_version: &'a str,
    /// `None` when the latest-version lookup was skipped (offline) or
    /// failed. Distinguishable from "no update needed" via
    /// `update_available`.
    latest_version: Option<&'a str>,
    update_available: bool,
    install_method: &'static str,
    /// Empty string when no upgrade command applies (e.g.
    /// `dev_build`). Tooling should treat empty as "no automatic
    /// upgrade — see docs."
    upgrade_command: &'static str,
}

fn print_human(
    current: &str,
    latest: Option<&str>,
    update_available: bool,
    install_method: InstallMethod,
) {
    println!("anvil {current}");
    match latest {
        Some(l) if update_available => println!("Latest: {l} (update available)"),
        Some(l) => println!("Latest: {l} (up to date)"),
        None => println!("Latest: unavailable (network probe skipped or failed)"),
    }
    println!("Installed via: {}", install_method_display(install_method));
    let cmd = upgrade_command_for(install_method);
    if !cmd.is_empty() {
        if update_available {
            println!("Upgrade: {cmd}");
        } else {
            println!("Upgrade command (when needed): {cmd}");
        }
    }
}

fn install_method_display(m: InstallMethod) -> &'static str {
    match m {
        InstallMethod::Homebrew => "Homebrew",
        InstallMethod::Scoop => "Scoop",
        InstallMethod::Winget => "WinGet",
        InstallMethod::CargoDist => "cargo-dist installer",
        InstallMethod::CargoInstall => "cargo install (CARGO_HOME/bin)",
        InstallMethod::DevBuild => "developer build (cargo)",
        InstallMethod::Unknown => "unknown / manual",
    }
}

/// Recommended upgrade command per install method. Strings are stable
/// — tooling can pin against them.
pub fn upgrade_command_for(m: InstallMethod) -> &'static str {
    match m {
        InstallMethod::Homebrew => "brew upgrade eddacraft/tap/anvil",
        InstallMethod::Scoop => "scoop update anvil",
        InstallMethod::Winget => "winget upgrade --id eddacraft.anvil",
        // Older direct installs (pre-`anvil update`) won't have the
        // subcommand, so the honest recommendation is to rerun the
        // installer. Mirrors the canonical URL in `install.sh`'s
        // header comment so a user pasting the printed line into
        // their shell hits the same release artefact `install.sh`
        // resolves.
        InstallMethod::CargoDist => {
            "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh | sh"
        }
        InstallMethod::CargoInstall => "cargo install eddacraft-anvil --force",
        InstallMethod::DevBuild => "",
        InstallMethod::Unknown => {
            "rerun your installer or download the latest release from https://github.com/eddacraft/anvil-001/releases"
        }
    }
}

// ─── Install method detection ────────────────────────────────────────

const HOMEBREW_PREFIXES: &[&str] = &["/opt/homebrew/", "/usr/local/Cellar/", "/home/linuxbrew/"];
const SCOOP_MARKERS: &[&str] = &["scoop\\apps\\anvil\\", "scoop/apps/anvil/"];
const WINGET_MARKERS: &[&str] = &["WindowsApps\\eddacraft", "WindowsApps/eddacraft"];

pub fn detect_install_method() -> InstallMethod {
    let Ok(exe) = std::env::current_exe() else {
        return InstallMethod::Unknown;
    };
    classify_exe_path(&exe)
}

/// Pure helper exposed for unit testing — `detect_install_method`'s
/// only side-effect is reading `current_exe()`. By taking a path
/// the helper is deterministic, so fixture tests do not need to
/// fork or set environment variables.
///
/// Detection order matters:
/// 1. Package managers by absolute / fragment path (Homebrew, Scoop, `WinGet`).
/// 2. `cargo install` location (`$CARGO_HOME/bin` or `~/.cargo/bin`).
/// 3. Developer build under `target/{debug,release}/`.
/// 4. cargo-dist receipt presence.
/// 5. Fall through to `Unknown`.
pub(crate) fn classify_exe_path(exe: &Path) -> InstallMethod {
    let s = exe.to_string_lossy();

    if HOMEBREW_PREFIXES.iter().any(|p| s.starts_with(p)) {
        return InstallMethod::Homebrew;
    }
    if SCOOP_MARKERS.iter().any(|m| s.contains(m)) {
        return InstallMethod::Scoop;
    }
    if WINGET_MARKERS.iter().any(|m| s.contains(m)) {
        return InstallMethod::Winget;
    }
    if is_cargo_install(exe) {
        return InstallMethod::CargoInstall;
    }
    if is_dev_build(exe) {
        return InstallMethod::DevBuild;
    }
    if has_cargo_dist_receipt(exe) {
        return InstallMethod::CargoDist;
    }

    InstallMethod::Unknown
}

fn is_cargo_install(exe: &Path) -> bool {
    // CARGO_HOME defaults to ~/.cargo. The binary ends up at
    // $CARGO_HOME/bin/<name>. Match either the explicit env var or
    // the default home-dir path so users with custom CARGO_HOME
    // (CI runners, multi-user systems) are still detected.
    let cargo_home = std::env::var_os("CARGO_HOME").map(std::path::PathBuf::from);
    if let Some(home) = cargo_home
        && exe.starts_with(home.join("bin"))
    {
        return true;
    }
    let Some(user_home) = dirs::home_dir() else {
        return false;
    };
    exe.starts_with(user_home.join(".cargo").join("bin"))
}

fn is_dev_build(exe: &Path) -> bool {
    // A `target/{debug,release}/anvil` path is the canonical dev
    // build location across platforms. Match by component rather
    // than substring so a user-installed binary at
    // `/opt/anvil-target/anvil` isn't false-positive flagged.
    let mut components: Vec<_> = exe.components().collect();
    components.pop(); // drop the binary name
    let Some(parent) = components.last() else {
        return false;
    };
    let parent_name = parent.as_os_str().to_string_lossy();
    if !matches!(parent_name.as_ref(), "debug" | "release") {
        return false;
    }
    components.pop();
    let Some(grandparent) = components.last() else {
        return false;
    };
    grandparent.as_os_str() == "target"
}

fn has_cargo_dist_receipt(exe: &Path) -> bool {
    // cargo-dist's install receipt is `~/.config/anvil/anvil-receipt.json`
    // (the platform user-config dir per `dirs::config_dir`). The
    // axoupdater crate's `load_receipt` consults the same location;
    // we just check existence rather than parsing.
    let _ = exe; // path may be useful in future heuristics
    let Some(config_dir) = dirs::config_dir() else {
        return false;
    };
    config_dir.join("anvil").join("anvil-receipt.json").exists()
}

// ─── Latest-version probe ────────────────────────────────────────────

/// Best-effort fetch of the latest GitHub release tag. Errors and
/// timeouts are silently swallowed — the caller surfaces "unavailable"
/// rather than failing the command.
///
/// Wraps an async `reqwest::Client` in a fresh single-thread tokio
/// runtime, matching the pattern axoupdater uses for its blocking
/// API. Avoids enabling the workspace-wide `reqwest/blocking` feature
/// just for this probe.
fn fetch_latest_version_quiet() -> Option<String> {
    let url = format!("https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases/latest");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    runtime.block_on(fetch_latest_version_from(&url))
}

/// The probe extracted to take an arbitrary URL so unit tests can
/// point it at a wiremock server. `fetch_latest_version_quiet` is
/// the only production caller and always passes the GitHub URL.
async fn fetch_latest_version_from(url: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .user_agent(concat!("anvil/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    let tag = body.get("tag_name")?.as_str()?;
    Some(tag.trim_start_matches('v').to_string())
}

/// True when `latest` is a strictly higher `SemVer` than `current`.
/// Returns `false` for unparseable versions — better to under-claim
/// "update available" than to nag on a parse glitch.
fn is_newer_semver(latest: &str, current: &str) -> bool {
    use std::cmp::Ordering;

    let Some(l) = parse_version(latest) else {
        return false;
    };
    let Some(c) = parse_version(current) else {
        return false;
    };
    matches!(l.cmp(&c), Ordering::Greater)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SimpleVersion {
    major: u64,
    minor: u64,
    patch: u64,
    /// Pre-release identifier as a sortable string; `None` means a
    /// stable (non-pre-release) version which sorts AFTER any
    /// pre-release of the same `major.minor.patch` per `SemVer`
    /// rules. Encoded explicitly via [`Self::cmp_key`].
    pre: Option<String>,
}

impl SimpleVersion {
    /// Pre-release identifiers sort by `SemVer` 2.0 §11 rules: numeric
    /// identifiers compare numerically, alphanumeric identifiers
    /// compare lexicographically, numeric < alphanumeric. Returns
    /// `None` for stable (no pre-release).
    fn pre_identifiers(&self) -> Option<Vec<PreIdentifier>> {
        self.pre
            .as_deref()
            .map(|p| p.split('.').map(PreIdentifier::from_str).collect())
    }
}

/// One dot-separated identifier within a pre-release string. Numeric
/// identifiers (all-ASCII-digits) compare numerically; non-numeric
/// identifiers compare lexicographically; numeric always sorts BEFORE
/// alphanumeric, per `SemVer` 2.0 §11.4.3.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PreIdentifier {
    Numeric(u64),
    Alphanumeric(String),
}

impl PreIdentifier {
    fn from_str(s: &str) -> Self {
        if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
            // Saturate on overflow rather than fail — `2^64 - 1` is
            // already absurd as a pre-release identifier.
            if let Ok(n) = s.parse::<u64>() {
                return PreIdentifier::Numeric(n);
            }
        }
        PreIdentifier::Alphanumeric(s.to_string())
    }
}

impl std::cmp::Ord for PreIdentifier {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match (self, other) {
            (PreIdentifier::Numeric(a), PreIdentifier::Numeric(b)) => a.cmp(b),
            (PreIdentifier::Alphanumeric(a), PreIdentifier::Alphanumeric(b)) => a.cmp(b),
            // Numeric always sorts before alphanumeric.
            (PreIdentifier::Numeric(_), PreIdentifier::Alphanumeric(_)) => Ordering::Less,
            (PreIdentifier::Alphanumeric(_), PreIdentifier::Numeric(_)) => Ordering::Greater,
        }
    }
}

impl std::cmp::PartialOrd for PreIdentifier {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for SimpleVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        // Compare core components first.
        let core_cmp =
            (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch));
        if core_cmp != Ordering::Equal {
            return core_cmp;
        }
        // SemVer §11.3: stable > any pre-release of same core.
        match (self.pre_identifiers(), other.pre_identifiers()) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => {
                // SemVer §11.4: identifier-by-identifier comparison.
                // A larger set of identifiers (with all earlier ones
                // equal) wins.
                a.cmp(&b)
            }
        }
    }
}

impl std::cmp::PartialOrd for SimpleVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn parse_version(raw: &str) -> Option<SimpleVersion> {
    // Strip leading `v`. Split on `-` to separate version core from
    // pre-release identifier. Then split core on `.`.
    let trimmed = raw.trim_start_matches('v');
    let (core, pre) = match trimmed.split_once('-') {
        Some((c, p)) => (c, Some(p.to_string())),
        None => (trimmed, None),
    };
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    Some(SimpleVersion {
        major: parts[0].parse().ok()?,
        minor: parts[1].parse().ok()?,
        patch: parts[2].parse().ok()?,
        pre,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn cmp(a: &str, b: &str) -> std::cmp::Ordering {
        parse_version(a).unwrap().cmp(&parse_version(b).unwrap())
    }

    #[test]
    fn parse_strips_leading_v() {
        let v = parse_version("v1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert!(v.pre.is_none());
    }

    #[test]
    fn parse_handles_pre_release() {
        let v = parse_version("0.5.1-beta").unwrap();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 5);
        assert_eq!(v.patch, 1);
        assert_eq!(v.pre.as_deref(), Some("beta"));
    }

    #[test]
    fn parse_rejects_non_semver() {
        assert!(parse_version("1.2").is_none());
        assert!(parse_version("not-a-version").is_none());
        assert!(parse_version("1.2.x").is_none());
    }

    #[test]
    fn ordering_handles_core_versions() {
        assert_eq!(cmp("1.0.0", "1.0.0"), std::cmp::Ordering::Equal);
        assert_eq!(cmp("1.0.1", "1.0.0"), std::cmp::Ordering::Greater);
        assert_eq!(cmp("1.1.0", "1.0.99"), std::cmp::Ordering::Greater);
        assert_eq!(cmp("2.0.0", "1.999.0"), std::cmp::Ordering::Greater);
    }

    #[test]
    fn ordering_treats_stable_as_greater_than_pre_release() {
        // 1.0.0 (stable) > 1.0.0-beta — semver semantics.
        assert_eq!(cmp("1.0.0", "1.0.0-beta"), std::cmp::Ordering::Greater);
        assert_eq!(cmp("0.5.1-beta", "0.5.0"), std::cmp::Ordering::Greater);
        // Council remediation guard: pre-release of higher core
        // beats stable of lower core.
        assert_eq!(cmp("0.6.0-rc.1", "0.5.99"), std::cmp::Ordering::Greater);
    }

    #[test]
    fn is_newer_semver_basic_cases() {
        assert!(is_newer_semver("0.5.2-beta", "0.5.1-beta"));
        assert!(!is_newer_semver("0.5.1-beta", "0.5.2-beta"));
        assert!(!is_newer_semver("0.5.1-beta", "0.5.1-beta"));
        // Unparseable inputs return false (don't nag).
        assert!(!is_newer_semver("not-a-version", "0.5.1-beta"));
        assert!(!is_newer_semver("0.5.2-beta", "not-a-version"));
    }

    #[test]
    fn upgrade_command_per_install_method_is_stable() {
        assert_eq!(
            upgrade_command_for(InstallMethod::Homebrew),
            "brew upgrade eddacraft/tap/anvil"
        );
        assert_eq!(
            upgrade_command_for(InstallMethod::Scoop),
            "scoop update anvil"
        );
        assert!(upgrade_command_for(InstallMethod::Winget).contains("winget upgrade"));
        // Round-1 council: the cargo-dist URL must match the
        // canonical URL in `install.sh` so a user pasting the
        // printed line into their shell hits the same release
        // artefact.
        let cargo_dist_cmd = upgrade_command_for(InstallMethod::CargoDist);
        assert!(
            cargo_dist_cmd.contains(
                "github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh"
            ),
            "CargoDist upgrade command must point at the canonical \
             release URL: {cargo_dist_cmd}"
        );
        assert_eq!(
            upgrade_command_for(InstallMethod::CargoInstall),
            "cargo install eddacraft-anvil --force"
        );
        assert_eq!(upgrade_command_for(InstallMethod::DevBuild), "");
        assert!(upgrade_command_for(InstallMethod::Unknown).contains("releases"));
    }

    #[test]
    fn install_method_label_is_snake_case() {
        for m in [
            InstallMethod::Homebrew,
            InstallMethod::Scoop,
            InstallMethod::Winget,
            InstallMethod::CargoDist,
            InstallMethod::CargoInstall,
            InstallMethod::DevBuild,
            InstallMethod::Unknown,
        ] {
            let l = m.label();
            assert!(
                l.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "label `{l}` is not snake_case"
            );
        }
    }

    #[test]
    fn is_dev_build_recognises_target_debug() {
        let p = PathBuf::from("/repo/target/debug/anvil");
        assert!(is_dev_build(&p));
    }

    #[test]
    fn is_dev_build_recognises_target_release() {
        let p = PathBuf::from("/repo/target/release/anvil");
        assert!(is_dev_build(&p));
    }

    #[test]
    fn is_dev_build_does_not_match_lookalike_paths() {
        // A binary at `/opt/foo-target/anvil` is NOT a dev build.
        let p = PathBuf::from("/opt/foo-target/anvil");
        assert!(!is_dev_build(&p));
        // `/usr/local/bin/anvil` is not a dev build.
        let p = PathBuf::from("/usr/local/bin/anvil");
        assert!(!is_dev_build(&p));
    }

    #[test]
    fn classify_exe_path_recognises_homebrew_prefixes() {
        // Apple Silicon
        assert_eq!(
            classify_exe_path(&PathBuf::from("/opt/homebrew/bin/anvil")),
            InstallMethod::Homebrew
        );
        // Intel macOS
        assert_eq!(
            classify_exe_path(&PathBuf::from("/usr/local/Cellar/anvil/0.5.1/bin/anvil")),
            InstallMethod::Homebrew
        );
        // Linuxbrew
        assert_eq!(
            classify_exe_path(&PathBuf::from("/home/linuxbrew/.linuxbrew/bin/anvil")),
            InstallMethod::Homebrew
        );
    }

    #[test]
    fn classify_exe_path_recognises_scoop_marker() {
        // Both forward and backslash variants — the marker list
        // covers Windows native and POSIX-style separators in case
        // a Windows tool reports paths with `/`.
        assert_eq!(
            classify_exe_path(&PathBuf::from(
                "C:\\Users\\me\\scoop\\apps\\anvil\\current\\anvil.exe"
            )),
            InstallMethod::Scoop
        );
    }

    #[test]
    fn classify_exe_path_recognises_winget_marker() {
        assert_eq!(
            classify_exe_path(&PathBuf::from(
                "C:\\Users\\me\\AppData\\Local\\Microsoft\\WindowsApps\\eddacraft.anvil_xyz\\anvil.exe"
            )),
            InstallMethod::Winget
        );
    }

    #[test]
    fn classify_exe_path_recognises_cargo_install_via_default_home() {
        // Use `temp_env::with_var` to avoid the `unsafe` block that
        // direct `std::env::set_var` requires under the workspace's
        // `unsafe_code = "forbid"` lint. The closure scope guarantees
        // the env var is restored after the test.
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin").join("anvil");
        let result = temp_env::with_var("CARGO_HOME", Some(dir.path().as_os_str()), || {
            classify_exe_path(&bin)
        });
        assert_eq!(result, InstallMethod::CargoInstall);
    }

    #[test]
    fn classify_exe_path_falls_through_to_unknown_for_arbitrary_path() {
        // No package-manager marker, not under target/, not in
        // CARGO_HOME — falls through to Unknown. Set both `HOME` and
        // `XDG_CONFIG_HOME` to a temp dir so the cargo-dist receipt
        // probe sees no file. `temp_env::with_vars` handles cleanup.
        let dir = tempfile::tempdir().unwrap();
        let result = temp_env::with_vars(
            [
                ("HOME", Some(dir.path().as_os_str())),
                (
                    "XDG_CONFIG_HOME",
                    Some(dir.path().join("config").as_os_str()),
                ),
                // Defensively unset CARGO_HOME so the test isolates
                // the unknown path even when run on a developer machine.
                ("CARGO_HOME", None),
            ],
            || classify_exe_path(&PathBuf::from("/opt/some/random/anvil")),
        );
        assert_eq!(result, InstallMethod::Unknown);
    }

    #[test]
    fn semver_numeric_pre_release_identifiers_sort_numerically() {
        // SemVer 2.0 §11.4.1: numeric identifiers compare numerically.
        // The hand-rolled parser previously used lex-order which
        // would have placed `rc.10 < rc.2` — broken. The numeric-
        // identifier path now produces the correct order.
        assert_eq!(
            cmp("0.5.0-rc.10", "0.5.0-rc.2"),
            std::cmp::Ordering::Greater,
            "rc.10 should sort AFTER rc.2"
        );
        assert_eq!(cmp("0.5.0-rc.2", "0.5.0-rc.10"), std::cmp::Ordering::Less);
        assert_eq!(cmp("0.5.0-rc.10", "0.5.0-rc.10"), std::cmp::Ordering::Equal);
        // Numeric < alphanumeric (SemVer §11.4.3):
        // 0.5.0-1 < 0.5.0-alpha
        assert_eq!(cmp("0.5.0-1", "0.5.0-alpha"), std::cmp::Ordering::Less);
    }

    #[test]
    fn is_newer_semver_handles_numeric_pre_release() {
        // Council remediation: bumping rc.2 → rc.10 must show
        // "update available", not be silently suppressed.
        assert!(is_newer_semver("0.5.0-rc.10", "0.5.0-rc.2"));
        assert!(!is_newer_semver("0.5.0-rc.2", "0.5.0-rc.10"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fetch_latest_version_handles_404_gracefully() {
        // Council remediation: prove the network-failure path
        // returns None instead of bubbling up an error.
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let url = format!("{}/releases/latest", server.uri());
        let result = fetch_latest_version_from(&url).await;
        assert!(
            result.is_none(),
            "404 response must yield None, got {result:?}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fetch_latest_version_handles_malformed_json_gracefully() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("{not json"))
            .mount(&server)
            .await;
        let url = format!("{}/releases/latest", server.uri());
        let result = fetch_latest_version_from(&url).await;
        assert!(result.is_none(), "malformed JSON must yield None");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fetch_latest_version_handles_missing_tag_name_gracefully() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_string(r#"{"name": "v0.5.2-beta"}"#),
            )
            .mount(&server)
            .await;
        let url = format!("{}/releases/latest", server.uri());
        let result = fetch_latest_version_from(&url).await;
        // No `tag_name` field → return None rather than guessing.
        assert!(
            result.is_none(),
            "JSON without tag_name must yield None, got {result:?}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fetch_latest_version_strips_leading_v_from_tag_name() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_string(r#"{"tag_name": "v0.5.2-beta"}"#),
            )
            .mount(&server)
            .await;
        let url = format!("{}/releases/latest", server.uri());
        let result = fetch_latest_version_from(&url).await;
        assert_eq!(result.as_deref(), Some("0.5.2-beta"));
    }
}
