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

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

// Match the public release source used by `commands::update.rs` and
// the canonical `install.sh` URL — the public-facing repo is
// `eddacraft/anvil`, not `eddacraft/anvil-001` (which is the private
// development repo). Pointing the latest-version probe at the wrong
// repo would silently report wrong update availability.
const GITHUB_OWNER: &str = "eddacraft";
const GITHUB_REPO: &str = "anvil";

#[derive(Debug, Args)]
pub struct VersionArgs {
    /// Skip the network probe for the latest release version. Useful
    /// in CI / sandboxed environments where outbound HTTPS is blocked.
    #[arg(long)]
    pub offline: bool,

    /// Probe the releases feed for security advisories attached to the
    /// running version, in addition to the latest-version check.
    /// Requires network unless `--offline`. When offline, advisories
    /// are reported as `unavailable` rather than empty so the user
    /// knows the absence is not a positive result.
    ///
    /// `anvil status` and the watch TUI also probe for the same hint
    /// on every invocation (rate-limited to once per 24h per
    /// advertised version). Opt out of that ambient probe with
    /// `ANVIL_DISABLE_UPDATE_HINT=1` in the environment.
    #[arg(long)]
    pub check: bool,
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

    // GH #1920: detect other `anvil` binaries on PATH besides the one
    // running now. Multiple installs (e.g. cargo-dist `~/.eddacraft/bin`
    // vs Scoop `~/scoop/shims`) can co-exist; `scoop update` then reports
    // success while a stale copy earlier in PATH keeps winning, so `anvil`
    // runs old code with no signal. Listing the extras turns that silent
    // wrong-version into a one-line diagnosis.
    let shadow_paths: Vec<String> = find_shadowing_anvil_binaries(
        std::env::var_os("PATH").as_deref(),
        std::env::current_exe().ok().as_deref(),
    )
    .into_iter()
    .map(|p| p.display().to_string())
    .collect();

    let latest = if args.offline {
        None
    } else {
        fetch_latest_version_quiet()
    };
    let update_available = match latest.as_deref() {
        Some(l) => is_newer_semver(l, &current),
        None => false,
    };

    // `--check` opts in to a second network call that retrieves the
    // running version's release body and extracts any
    // `Security-Advisory: GHSA-…` entries. Skipping this on every
    // `anvil version` invocation keeps the default path's latency
    // bounded by one HTTP round-trip; users who specifically want to
    // know about advisories ask via `--check`.
    //
    // Three states: NotProbed (no --check), Unavailable (--check
    // requested but offline OR network failure), Probed(items)
    // (probe succeeded, items may be empty). The distinction matters
    // because "we checked and there are none" reassures the user,
    // while "we tried and couldn't ask" must NOT be presented as
    // reassurance.
    let advisories = if !args.check {
        AdvisoryProbe::NotProbed
    } else if args.offline {
        AdvisoryProbe::Unavailable
    } else {
        match fetch_advisories_for_version(&current) {
            Some(items) => AdvisoryProbe::Probed(items),
            None => AdvisoryProbe::Unavailable,
        }
    };

    if global.json {
        let payload = VersionJson {
            current_version: &current,
            latest_version: latest.as_deref(),
            update_available,
            install_method: install_method.label(),
            upgrade_command,
            advisories: advisories.json_shape(),
            shadowed_by: &shadow_paths,
        };
        let out = serde_json::to_string_pretty(&payload)?;
        println!("{out}");
    } else {
        print_human(
            &current,
            latest.as_deref(),
            update_available,
            install_method,
            &advisories,
            &shadow_paths,
        );
    }

    Ok(())
}

/// GH #1920: find `anvil` executables on `PATH` other than the running one.
///
/// On Windows especially, the cargo-dist installer (`~/.eddacraft/bin`) and
/// Scoop (`~/scoop/shims`) install to different directories that can both be
/// on `PATH`. After `scoop update`, the Scoop copy is fresh but a stale
/// cargo-dist copy earlier in `PATH` keeps resolving — so `anvil` runs the
/// old binary while `scoop` reports the new version. This enumerates the
/// other copies so the version surface can warn about the shadowing.
///
/// Pure over its inputs (the `PATH` value and the running exe path) so it is
/// unit-testable without touching the process environment.
///
/// Reports at most one binary **per PATH directory**, excluding the directory
/// the running binary lives in. This is deliberate: a single install can drop
/// several wrappers in one directory (a Scoop shims dir holds `anvil.exe`,
/// `anvil.cmd`, and `anvil.ps1` for the *same* install), so counting files
/// would flag one install as shadowing itself. Distinct PATH directories each
/// holding an `anvil` is the real "more than one install" signal. The
/// reported path is the on-disk wrapper location — for shim-based installers
/// it is the shim, not the underlying binary; `where`/`which -a` disambiguate.
fn find_shadowing_anvil_binaries(
    path_var: Option<&OsStr>,
    current_exe: Option<&Path>,
) -> Vec<PathBuf> {
    let Some(path_var) = path_var else {
        return Vec::new();
    };
    // Without knowing which binary is running we cannot tell a shadow from
    // the real one — don't guess and risk flagging the running install.
    // This also covers the case where `current_exe` is given but cannot be
    // canonicalised (deleted, /proc unavailable, permissions): without a
    // canonical form we can exclude neither the running binary nor its
    // directory, so every PATH copy — including the real one — would look
    // like a shadow. Bail rather than emit a false warning.
    let Some(current_exe) = current_exe else {
        return Vec::new();
    };
    let Ok(current_canon) = std::fs::canonicalize(current_exe) else {
        return Vec::new();
    };
    let current_dir = current_canon.parent().map(Path::to_path_buf);

    let exe_names: &[&str] = if cfg!(windows) {
        &["anvil.exe", "anvil.cmd", "anvil.bat", "anvil.ps1"]
    } else {
        &["anvil"]
    };

    let mut found = Vec::new();
    let mut seen_dirs = std::collections::HashSet::new();
    for dir in std::env::split_paths(path_var) {
        // Canonicalise the directory so repeated / symlinked PATH entries and
        // the running binary's own directory are recognised regardless of
        // spelling. A directory we cannot canonicalise (missing, unreadable,
        // or an invalid path) cannot hold a usable shadow we can name — skip
        // it.
        let Ok(canon_dir) = std::fs::canonicalize(&dir) else {
            continue;
        };
        if current_dir.as_ref() == Some(&canon_dir) {
            continue; // the running install's directory
        }
        if !seen_dirs.insert(canon_dir) {
            continue; // already inspected this directory under another spelling
        }
        for name in exe_names {
            let candidate = dir.join(name);
            let Ok(canon) = std::fs::canonicalize(&candidate) else {
                continue; // missing / transient
            };
            if canon == current_canon {
                continue; // the running binary reached via another PATH entry
            }
            if !is_executable_file(&canon) {
                continue;
            }
            found.push(candidate);
            break; // one entry per directory
        }
    }
    found
}

/// True when `path` is a regular file the current platform would execute.
/// On Unix this requires an execute bit; on other platforms a regular file
/// is sufficient (Windows keys execution off the extension, already filtered
/// by the candidate name list).
fn is_executable_file(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    meta.is_file() && has_execute_permission(&meta)
}

#[cfg(unix)]
fn has_execute_permission(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn has_execute_permission(_meta: &std::fs::Metadata) -> bool {
    // Windows keys execution off the file extension, already constrained by
    // the candidate name list.
    true
}

/// Outcome of the `--check` advisory probe.
///
/// `NotProbed` means `--check` was not requested — distinct from
/// `Probed(vec![])` (probed, none attached) so the JSON consumer can
/// tell "we don't know" from "we know there are none". `Unavailable`
/// means `--check` was requested but `--offline` blocked the probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdvisoryProbe {
    NotProbed,
    Unavailable,
    Probed(Vec<AdvisoryTag>),
}

impl AdvisoryProbe {
    fn json_shape(&self) -> AdvisoryJson<'_> {
        match self {
            AdvisoryProbe::NotProbed => AdvisoryJson {
                checked: false,
                available: false,
                items: &[],
            },
            AdvisoryProbe::Unavailable => AdvisoryJson {
                checked: true,
                available: false,
                items: &[],
            },
            AdvisoryProbe::Probed(items) => AdvisoryJson {
                checked: true,
                available: true,
                items,
            },
        }
    }
}

/// One security advisory tag attached to a release. The `id` follows
/// GitHub's GHSA scheme (`GHSA-xxxx-yyyy-zzzz`); other identifiers
/// (CVE, RUSTSEC) are accepted verbatim. `summary` is the free-text
/// remainder after the colon if present, trimmed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdvisoryTag {
    pub id: String,
    /// Optional human description after the ID (e.g. " — credential
    /// leak in update flow"). Empty string when no summary is given.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub summary: String,
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
    /// DISTRIB-002 advisory surface. Only populated when `--check`
    /// was passed; otherwise `checked` is false and consumers should
    /// treat `items` as "unknown" rather than "none".
    advisories: AdvisoryJson<'a>,
    /// GH #1920: other `anvil` executables found on `PATH` besides the
    /// running one. Empty in the common single-install case. When
    /// non-empty, `PATH` order — not the install method shown above —
    /// decides which binary a fresh shell runs.
    shadowed_by: &'a [String],
}

#[derive(Serialize)]
struct AdvisoryJson<'a> {
    checked: bool,
    available: bool,
    items: &'a [AdvisoryTag],
}

fn print_human(
    current: &str,
    latest: Option<&str>,
    update_available: bool,
    install_method: InstallMethod,
    advisories: &AdvisoryProbe,
    shadowed_by: &[String],
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
    match advisories {
        AdvisoryProbe::NotProbed => {}
        AdvisoryProbe::Unavailable => {
            println!("Advisories: unavailable (offline; rerun without --offline to probe)");
        }
        AdvisoryProbe::Probed(items) if items.is_empty() => {
            println!("Advisories: none attached to running version");
        }
        AdvisoryProbe::Probed(items) => {
            // Surface each advisory on its own line so a release with
            // multiple tags does not collapse into one ambiguous string.
            // The ID is the actionable token a user pastes into
            // GitHub's advisory search.
            println!("Advisories: {} attached to running version", items.len());
            for adv in items {
                if adv.summary.is_empty() {
                    println!("  - {}", adv.id);
                } else {
                    println!("  - {}: {}", adv.id, adv.summary);
                }
            }
        }
    }
    if !shadowed_by.is_empty() {
        // GH #1920: more than one `anvil` is on PATH. PATH order — not the
        // install method above — decides which one a fresh shell runs, so a
        // per-manager update (e.g. `scoop update`) can succeed while a stale
        // copy keeps winning. Name the extras and point at the resolver.
        println!();
        println!(
            "Warning: {} other `anvil` executable(s) found on PATH besides the one running now:",
            shadowed_by.len()
        );
        for path in shadowed_by {
            println!("  - {path}");
        }
        println!(
            "PATH order decides which runs; a per-manager update may not change it. \
             Inspect with `where.exe anvil` or `Get-Command anvil -All` (Windows) or \
             `which -a anvil` (Unix), then remove the stale install or fix PATH order."
        );
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
    upgrade_command_for_platform(m, cfg!(windows))
}

/// [`upgrade_command_for`] with the host platform injected, so both branches
/// are reachable from any CI leg.
///
/// The parameter is not decoration: the Windows arm below became reachable for
/// the first time when receipt detection was fixed (CIB-315), and a `cfg!`
/// inside the match would have made the advice Windows users actually receive
/// untestable everywhere except the nightly Windows leg. That is the same
/// shape as the defect this item exists to fix — a production value no test on
/// the default legs can supply.
pub(crate) fn upgrade_command_for_platform(m: InstallMethod, windows: bool) -> &'static str {
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
        //
        // Platform-split on purpose: a Windows user cannot run
        // `curl … | sh`, and `anvil update` cannot self-update on Windows
        // either (`install-updater = false`), so re-running the PowerShell
        // installer is the only working upgrade path there. Without this
        // split, fixing detection would have swapped one unrunnable
        // instruction (`cargo install --git …`, needs a toolchain) for
        // another (`curl … | sh`, needs a POSIX shell).
        InstallMethod::CargoDist => {
            if windows {
                WINDOWS_INSTALLER_UPGRADE
            } else {
                UNIX_INSTALLER_UPGRADE
            }
        }
        // The `eddacraft-anvil` crate is `publish = false`, so a
        // bare `cargo install eddacraft-anvil` will fail. The only
        // working `cargo install` form is the git-source variant
        // pointing at the public repo. Users who installed via a
        // local `--path` clone should re-run their original command.
        InstallMethod::CargoInstall => {
            "cargo install --git https://github.com/eddacraft/anvil --force eddacraft-anvil"
        }
        InstallMethod::DevBuild => "",
        InstallMethod::Unknown => {
            "rerun your installer or download the latest release from https://github.com/eddacraft/anvil/releases"
        }
    }
}

// ─── Install method detection ────────────────────────────────────────

/// The only working upgrade path for a receipt install on Windows: re-run the
/// PowerShell installer. Kept identical to the line `anvil update` prints when
/// it declines to self-update on Windows, so the two surfaces cannot suggest
/// different commands for the same install.
pub(crate) const WINDOWS_INSTALLER_UPGRADE: &str = "irm https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.ps1 | iex";

/// The shell-installer equivalent for every other platform.
pub(crate) const UNIX_INSTALLER_UPGRADE: &str = "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh | sh";

const HOMEBREW_PREFIXES: &[&str] = &["/opt/homebrew/", "/usr/local/Cellar/", "/home/linuxbrew/"];
const SCOOP_MARKERS: &[&str] = &["scoop\\apps\\anvil\\", "scoop/apps/anvil/"];
const WINGET_MARKERS: &[&str] = &["WindowsApps\\eddacraft", "WindowsApps/eddacraft"];

pub fn detect_install_method() -> InstallMethod {
    let Ok(exe) = std::env::current_exe() else {
        return InstallMethod::Unknown;
    };
    classify_exe_path(&exe)
}

/// CIB-197: process-cached [`detect_install_method`], for callers that
/// run on every command (the usage producer stamps the install method
/// onto each `command.invoked` row). Detection is one `current_exe`
/// read, a few path checks, and at most one receipt-file existence
/// probe — cheap, but the `OnceLock` keeps it one-shot per process. The
/// answer cannot change while the same binary is running, so caching is
/// lossless.
pub fn detect_install_method_cached() -> InstallMethod {
    static CACHE: std::sync::OnceLock<InstallMethod> = std::sync::OnceLock::new();
    *CACHE.get_or_init(detect_install_method)
}

/// Pure helper exposed for unit testing — `detect_install_method`'s
/// only side-effect is reading `current_exe()`. By taking a path
/// the helper is deterministic, so fixture tests do not need to
/// fork or set environment variables.
///
/// Detection order matters (CIB-229):
/// 1. Package managers by absolute / fragment path (Homebrew, Scoop, `WinGet`).
/// 2. cargo-dist receipt (must beat `$CARGO_HOME/bin` — cargo-dist default layout).
/// 3. Developer build under `target/{debug,release}/`.
/// 4. `cargo install` location (`$CARGO_HOME/bin` or `~/.cargo/bin`) without receipt.
/// 5. Fall through to `Unknown`.
pub(crate) fn classify_exe_path(exe: &Path) -> InstallMethod {
    // Package-manager markers first so production never pays for the receipt
    // probe (a file read plus a JSON parse) on Homebrew, Scoop, or WinGet
    // paths. Receipt lookup is deferred until the remaining classify path
    // needs it.
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
    classify_exe_path_with_receipt(exe, dist_receipt_present())
}

/// Whether cargo-dist's install receipt is present for this install.
///
/// Delegates to the shared [`crate::commands::update::load_dist_receipt`]
/// rather than resolving the receipt path here. Resolving it here is what
/// broke: `dirs::config_dir()` is `%APPDATA%` on Windows and
/// `~/Library/Application Support` on macOS, while cargo-dist writes to
/// `%LOCALAPPDATA%` and `~/.config` respectively, so the receipt branch was
/// unreachable on both platforms and every official-installer user there was
/// classified as `cargo install` (CIB-315). Only Linux ever agreed.
///
/// Note this is a load, not an existence probe: a receipt that cannot be
/// parsed no longer counts as provenance. That is the intent — `update`
/// cannot use an unparseable receipt either, and the two surfaces must not
/// disagree about the same install.
fn dist_receipt_present() -> bool {
    let mut updater = axoupdater::AxoUpdater::new_for(crate::commands::update::DIST_APP_NAME);
    crate::commands::update::load_dist_receipt(&mut updater)
}

/// Same as [`classify_exe_path`], but whether a cargo-dist receipt was found
/// is injected.
///
/// Production reaches this after the package-manager early returns and passes
/// [`dist_receipt_present`]. Tests pass the flag directly, so they exercise
/// the ordering without needing a receipt on disk.
///
/// Note what this signature deliberately does *not* take: a receipt **root**.
/// The previous shape injected one, which let every unit test pass a temp dir
/// and pass — while production passed a root the receipt writer never uses on
/// two of three platforms. A defect no test could express is not a tested
/// defect (CIB-315). Root resolution now lives in exactly one place, shared
/// with `anvil update`.
///
/// Package-manager checks are repeated here so injected-flag unit tests keep a
/// single full-order classifier (they call this helper directly).
pub(crate) fn classify_exe_path_with_receipt(exe: &Path, receipt_present: bool) -> InstallMethod {
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
    // Receipt before CARGO_HOME/bin: shell/PowerShell installers use that path.
    if receipt_present {
        return InstallMethod::CargoDist;
    }
    if is_dev_build(exe) {
        return InstallMethod::DevBuild;
    }
    if is_cargo_install(exe) {
        return InstallMethod::CargoInstall;
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
    let Some(user_home) = crate::util::user_home_dir() else {
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

// ─── Advisory probe (DISTRIB-002) ────────────────────────────────────

/// Best-effort fetch of advisories attached to a specific version's
/// release body. Returns `None` when the release cannot be fetched
/// (private repo, deleted release, network failure, malformed JSON);
/// returns `Some(vec![])` when the release exists but carries no
/// advisory tags. Two outcomes look the same in human output but
/// differ in JSON via [`AdvisoryProbe`].
fn fetch_advisories_for_version(version: &str) -> Option<Vec<AdvisoryTag>> {
    let tag = if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{version}")
    };
    let url =
        format!("https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases/tags/{tag}");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    runtime.block_on(fetch_advisories_from(&url))
}

/// Async core, extracted so unit tests can point it at a wiremock
/// server. Returns `None` on any failure (HTTP error, JSON parse,
/// missing field); the caller treats `None` as "advisory feed
/// unavailable for this version" and continues without false claims.
async fn fetch_advisories_from(url: &str) -> Option<Vec<AdvisoryTag>> {
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
    let release_body = body.get("body").and_then(|b| b.as_str()).unwrap_or("");
    Some(parse_advisory_tags(release_body))
}

// ─── Bridge: probe → UpdateHint DTO (DISTRIB-002) ─────────────────

/// Compose the optional `UpdateHint` for `anvil status` and the watch
/// TUI. Returns `None` when no update is available, the probe failed,
/// or the rate-limit gate suppressed the hint.
///
/// Probes latest version synchronously (3s timeout, silent on failure
/// — see [`fetch_latest_version_quiet`]). When `include_advisories`
/// is true, also probes the running version's release body for
/// advisory tags. The rate-limit gate at
/// [`crate::update_hint::record_if_due`] persists state so successive
/// invocations across surfaces share the 24h budget.
pub fn compute_update_hint(include_advisories: bool) -> Option<anvil_tui::surfaces::UpdateHint> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let latest = fetch_latest_version_quiet()?;
    if !is_newer_semver(&latest, &current) {
        return None;
    }
    let state_path = crate::update_hint::state_file_path()?;
    if !crate::update_hint::record_if_due(
        &state_path,
        &latest,
        std::time::SystemTime::now(),
        crate::update_hint::DEFAULT_HINT_TTL,
    ) {
        return None;
    }
    let advisory_ids = if include_advisories {
        fetch_advisories_for_version(&current)
            .unwrap_or_default()
            .into_iter()
            .map(|a| a.id)
            .collect()
    } else {
        Vec::new()
    };
    Some(anvil_tui::surfaces::UpdateHint {
        latest_version: latest,
        current_version: current,
        advisory_ids,
    })
}

/// Parse `Security-Advisory: <ID>[: <summary>]` lines from a release
/// body. The line may appear anywhere in the body. Header recognition
/// is case-insensitive on the prefix; the ID is captured verbatim
/// (typically `GHSA-xxxx-yyyy-zzzz`, but `CVE-*` and `RUSTSEC-*` are
/// also accepted because the convention is "whatever the maintainer
/// wrote after the colon").
///
/// Returns advisories in source order, de-duplicated by ID so a
/// release body that mentions the same advisory twice does not
/// double-count it.
pub fn parse_advisory_tags(release_body: &str) -> Vec<AdvisoryTag> {
    const PREFIX: &str = "security-advisory:";
    let mut out: Vec<AdvisoryTag> = Vec::new();
    for raw_line in release_body.lines() {
        let line = raw_line.trim_start_matches(['-', '*', ' ', '\t']);
        let lower = line.to_ascii_lowercase();
        let Some(rest) = lower.strip_prefix(PREFIX) else {
            continue;
        };
        // Index into the original-case string at the same boundary as
        // the lowercase match so identifier casing is preserved
        // (`GHSA-` upper-case is convention).
        let rest_start = PREFIX.len();
        let rest_original = &line[rest_start..];
        let _ = rest; // explicit drop — `lower` only used for prefix detection
        let trimmed = rest_original.trim();
        let (id, summary) = split_id_and_summary(trimmed);
        if !is_recognised_advisory_id(id) {
            // Reject lines like `Security-Advisory: see our linked
            // security policy` — they contain the prefix but no
            // actionable advisory identifier. Surfaces would render
            // them as malformed IDs in the red advisory style.
            continue;
        }
        if out.iter().any(|adv| adv.id == id) {
            continue;
        }
        out.push(AdvisoryTag {
            id: id.to_string(),
            summary: summary.to_string(),
        });
    }
    out
}

/// True when `id` looks like a recognised security-advisory identifier.
/// Anchored to the three schemes the spec mentions (`GHSA-`, `CVE-`,
/// `RUSTSEC-`) so prose lines that happen to start with
/// `Security-Advisory:` don't get parsed as advisories. Comparison is
/// case-insensitive on the prefix; the rest must contain only
/// digits, ASCII letters, and dashes — characters present in every
/// real GHSA/CVE/RUSTSEC identifier.
fn is_recognised_advisory_id(id: &str) -> bool {
    let upper = id.to_ascii_uppercase();
    let known = ["GHSA-", "CVE-", "RUSTSEC-"];
    if !known.iter().any(|prefix| upper.starts_with(prefix)) {
        return false;
    }
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn split_id_and_summary(s: &str) -> (&str, &str) {
    // Allow either `ID: summary` or `ID — summary` or just `ID`.
    // The dash form is common in release notes where the colon is
    // already used to delimit the header.
    for sep in [": ", " — ", " - "] {
        if let Some(idx) = s.find(sep) {
            let id = s[..idx].trim();
            let summary = s[idx + sep.len()..].trim();
            return (id, summary);
        }
    }
    (s.trim(), "")
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

    // --- GH #1920: PATH-shadow detection ---

    fn anvil_exe_name() -> &'static str {
        if cfg!(windows) { "anvil.exe" } else { "anvil" }
    }

    /// Create a file the platform will treat as an executable (exec bit on
    /// Unix; any regular file on Windows).
    fn write_exe(path: &Path) {
        std::fs::write(path, b"x").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mut perms = std::fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(path, perms).unwrap();
        }
    }

    #[test]
    fn shadow_detection_finds_other_anvil_on_path() {
        let d1 = tempfile::tempdir().unwrap();
        let d2 = tempfile::tempdir().unwrap();
        let running = d1.path().join(anvil_exe_name());
        let other = d2.path().join(anvil_exe_name());
        write_exe(&running);
        write_exe(&other);

        let path = std::env::join_paths([d1.path(), d2.path()]).unwrap();
        let found = find_shadowing_anvil_binaries(Some(path.as_os_str()), Some(&running));
        assert_eq!(found.len(), 1, "found: {found:?}");
        assert_eq!(
            std::fs::canonicalize(&found[0]).unwrap(),
            std::fs::canonicalize(&other).unwrap()
        );
    }

    #[test]
    fn shadow_detection_excludes_the_running_binarys_directory() {
        let d = tempfile::tempdir().unwrap();
        let running = d.path().join(anvil_exe_name());
        write_exe(&running);

        let path = std::env::join_paths([d.path()]).unwrap();
        let found = find_shadowing_anvil_binaries(Some(path.as_os_str()), Some(&running));
        assert!(found.is_empty(), "found: {found:?}");
    }

    #[test]
    fn shadow_detection_empty_without_path() {
        let d = tempfile::tempdir().unwrap();
        let running = d.path().join(anvil_exe_name());
        write_exe(&running);
        assert!(find_shadowing_anvil_binaries(None, Some(&running)).is_empty());
    }

    #[test]
    fn shadow_detection_empty_when_current_exe_unknown() {
        // Cannot distinguish the running install from shadows without knowing
        // which binary is running — must not flag everything.
        let d = tempfile::tempdir().unwrap();
        write_exe(&d.path().join(anvil_exe_name()));
        let path = std::env::join_paths([d.path()]).unwrap();
        let found = find_shadowing_anvil_binaries(Some(path.as_os_str()), None);
        assert!(found.is_empty(), "found: {found:?}");
    }

    #[test]
    fn shadow_detection_empty_when_current_exe_uncanonicalisable() {
        // If the running exe path cannot be canonicalised (deleted, etc.),
        // we cannot exclude the running install — bail rather than flag the
        // real binary as a shadow.
        let d = tempfile::tempdir().unwrap();
        write_exe(&d.path().join(anvil_exe_name()));
        let path = std::env::join_paths([d.path()]).unwrap();
        let missing = d.path().join("does-not-exist").join(anvil_exe_name());
        let found = find_shadowing_anvil_binaries(Some(path.as_os_str()), Some(&missing));
        assert!(found.is_empty(), "found: {found:?}");
    }

    #[test]
    fn shadow_detection_dedups_repeated_dirs() {
        let d1 = tempfile::tempdir().unwrap();
        let d2 = tempfile::tempdir().unwrap();
        let running = d1.path().join(anvil_exe_name());
        write_exe(&running);
        write_exe(&d2.path().join(anvil_exe_name()));

        // d2 listed twice must not double-count the same directory.
        let path = std::env::join_paths([d2.path(), d1.path(), d2.path()]).unwrap();
        let found = find_shadowing_anvil_binaries(Some(path.as_os_str()), Some(&running));
        assert_eq!(found.len(), 1, "found: {found:?}");
    }

    #[cfg(unix)]
    #[test]
    fn shadow_detection_skips_non_executable_file() {
        let d1 = tempfile::tempdir().unwrap();
        let d2 = tempfile::tempdir().unwrap();
        let running = d1.path().join(anvil_exe_name());
        write_exe(&running);
        // A non-executable `anvil` file the shell would never run.
        std::fs::write(d2.path().join(anvil_exe_name()), b"not exec").unwrap();

        let path = std::env::join_paths([d1.path(), d2.path()]).unwrap();
        let found = find_shadowing_anvil_binaries(Some(path.as_os_str()), Some(&running));
        assert!(found.is_empty(), "found: {found:?}");
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
                "github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer."
            ),
            "CargoDist upgrade command must point at the canonical \
             release URL: {cargo_dist_cmd}"
        );
        // Round-2 council: `eddacraft-anvil` crate is `publish =
        // false`, so the bare `cargo install eddacraft-anvil` form
        // would fail. The hint must use the git-source variant
        // pointing at the public repo, not crates.io.
        let cargo_install_cmd = upgrade_command_for(InstallMethod::CargoInstall);
        assert!(
            cargo_install_cmd.contains("--git https://github.com/eddacraft/anvil"),
            "CargoInstall hint must use --git form (crate is publish=false): {cargo_install_cmd}"
        );
        assert!(
            !cargo_install_cmd.starts_with("cargo install eddacraft-anvil "),
            "CargoInstall hint must NOT use the bare crates.io form: {cargo_install_cmd}"
        );
        assert_eq!(upgrade_command_for(InstallMethod::DevBuild), "");
        // Unknown hint must point at the PUBLIC repo (eddacraft/anvil),
        // not the private dev repo (eddacraft/anvil-001).
        let unknown_cmd = upgrade_command_for(InstallMethod::Unknown);
        assert!(unknown_cmd.contains("releases"));
        assert!(
            unknown_cmd.contains("eddacraft/anvil/releases"),
            "Unknown hint must point at the public repo: {unknown_cmd}"
        );
        assert!(
            !unknown_cmd.contains("eddacraft/anvil-001/releases"),
            "Unknown hint must NOT point at the private repo: {unknown_cmd}"
        );
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
    fn detect_install_method_cached_is_stable_across_calls() {
        // CIB-197: the OnceLock cache must hand back the same variant on
        // every call within a process (the usage producer may race the
        // `anvil version` surface for first detection).
        let first = detect_install_method_cached();
        let second = detect_install_method_cached();
        assert_eq!(first, second);
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
        let result = temp_env::with_vars(
            [
                ("CARGO_HOME", Some(dir.path().as_os_str())),
                ("HOME", Some(dir.path().as_os_str())),
            ],
            || classify_exe_path_with_receipt(&bin, false),
        );
        assert_eq!(result, InstallMethod::CargoInstall);
    }

    #[test]
    fn classify_exe_path_cargo_home_with_receipt_is_cargo_dist() {
        // CIB-229: cargo-dist's default layout is CARGO_HOME/bin + a receipt,
        // so a found receipt must beat the CargoInstall path check. This
        // covers the ordering only — that the receipt is found where the
        // installer actually writes it is
        // `dist_receipt_present_reads_the_path_cargo_dist_writes`.
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("cargo").join("bin").join("anvil");
        let result = temp_env::with_vars(
            [
                ("CARGO_HOME", Some(dir.path().join("cargo").as_os_str())),
                ("HOME", Some(dir.path().as_os_str())),
            ],
            || classify_exe_path_with_receipt(&bin, true),
        );
        assert_eq!(result, InstallMethod::CargoDist);
    }

    #[test]
    fn cargo_dist_upgrade_advice_is_runnable_on_the_platform_it_targets() {
        // CIB-315. Fixing receipt detection made the CargoDist arm reachable
        // on Windows for the first time. Before this, the arm only ever
        // returned the shell installer — so a Windows user would have been
        // moved off `cargo install --git …` (needs a toolchain they do not
        // have) onto `curl … | sh` (needs a shell they do not have). Both
        // branches are asserted from every CI leg, because the Windows leg
        // is nightly-only and this is precisely the advice Windows users see.
        let win = upgrade_command_for_platform(InstallMethod::CargoDist, true);
        assert!(
            win.contains("eddacraft-anvil-installer.ps1"),
            "Windows must be pointed at the PowerShell installer: {win}"
        );
        assert!(
            !win.contains("| sh") && !win.contains("curl "),
            "Windows must not be told to pipe into sh: {win}"
        );

        let unix = upgrade_command_for_platform(InstallMethod::CargoDist, false);
        assert!(
            unix.contains("eddacraft-anvil-installer.sh"),
            "non-Windows must be pointed at the shell installer: {unix}"
        );
        assert!(
            !unix.contains("irm "),
            "non-Windows must not be told to use irm: {unix}"
        );

        // `anvil update` declines to self-update on Windows and prints its own
        // upgrade line. If these two drift, one install gets two different
        // instructions depending on which command you happened to run.
        assert!(
            crate::commands::update::windows_unsupported_message()
                .contains(WINDOWS_INSTALLER_UPGRADE),
            "`version` and `update` must print the same Windows upgrade command"
        );
    }

    #[test]
    fn dist_receipt_present_reads_the_path_cargo_dist_writes() {
        // CIB-315. The claim under test is agreement with the *writer*, so
        // the receipt is planted through axoupdater's own path override
        // rather than through a root this module chooses — a root of our own
        // choosing is exactly what was wrong, and a test that picks one can
        // never fail on it.
        //
        // RED against the previous implementation on every platform:
        // `dirs::config_dir()` does not read AXOUPDATER_CONFIG_PATH. RED on
        // Windows and macOS for the shipped reason too — `%APPDATA%` and
        // `~/Library/Application Support` are not where the receipt lands.
        //
        // HOME and XDG_CONFIG_HOME are redirected so the developer's own
        // `~/.config/eddacraft-anvil` receipt cannot satisfy the positive
        // case under a broken implementation. Without that, the assertion
        // goes green on any machine with anvil installed — which is every
        // machine likely to run it.
        let home = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("eddacraft-anvil-receipt.json"),
            r#"{"install_prefix":"/tmp","binaries":["anvil"],"source":{"app_name":"eddacraft-anvil","name":"anvil","owner":"eddacraft","release_type":"github"},"version":"0.9.3-beta","provider":{"source":"cargo-dist","version":"0.31.0"}}"#,
        )
        .unwrap();

        let found = temp_env::with_vars(
            [
                ("AXOUPDATER_CONFIG_PATH", Some(dir.path().as_os_str())),
                ("XDG_CONFIG_HOME", Some(home.path().as_os_str())),
                ("HOME", Some(home.path().as_os_str())),
            ],
            dist_receipt_present,
        );
        assert!(
            found,
            "receipt written where cargo-dist writes it must be found by `version`"
        );

        let empty = tempfile::tempdir().unwrap();
        let missing = temp_env::with_vars(
            [
                ("AXOUPDATER_CONFIG_PATH", Some(empty.path().as_os_str())),
                ("XDG_CONFIG_HOME", Some(home.path().as_os_str())),
                ("HOME", Some(home.path().as_os_str())),
            ],
            dist_receipt_present,
        );
        assert!(!missing, "no receipt must not report cargo-dist provenance");
    }

    #[test]
    fn dist_receipt_present_accepts_the_legacy_app_name() {
        // Installs predating the eddacraft-anvil rename wrote `anvil`. The
        // fallback exists in `update`; `version` must agree, or the two
        // surfaces disagree about the same install again. HOME and
        // XDG_CONFIG_HOME are redirected for the same reason as above.
        let home = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("anvil-receipt.json"),
            r#"{"install_prefix":"/tmp","binaries":["anvil"],"source":{"app_name":"anvil","name":"anvil","owner":"eddacraft","release_type":"github"},"version":"0.8.1-beta","provider":{"source":"cargo-dist","version":"0.31.0"}}"#,
        )
        .unwrap();

        let found = temp_env::with_vars(
            [
                ("AXOUPDATER_CONFIG_PATH", Some(dir.path().as_os_str())),
                ("XDG_CONFIG_HOME", Some(home.path().as_os_str())),
                ("HOME", Some(home.path().as_os_str())),
            ],
            dist_receipt_present,
        );
        assert!(found, "legacy `anvil` receipt must still report cargo-dist");
    }

    #[test]
    fn classify_exe_path_falls_through_to_unknown_for_arbitrary_path() {
        // No package-manager marker, not under target/, not in
        // CARGO_HOME — falls through to Unknown. Inject "no receipt" so a
        // host cargo-dist install on the developer machine running the tests
        // cannot tip this into CargoDist.
        let dir = tempfile::tempdir().unwrap();
        let result = temp_env::with_vars(
            [
                ("HOME", Some(dir.path().as_os_str())),
                // Defensively unset CARGO_HOME so the test isolates
                // the unknown path even when run on a developer machine.
                ("CARGO_HOME", None),
            ],
            || classify_exe_path_with_receipt(&PathBuf::from("/opt/some/random/anvil"), false),
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

    // ─── DISTRIB-002 advisory parser ──────────────────────────────

    #[test]
    fn parse_advisory_tags_extracts_single_ghsa_with_colon_summary() {
        let body = "## Notes\n\nSecurity-Advisory: GHSA-aaaa-bbbb-cccc: credential leak\n";
        let advisories = parse_advisory_tags(body);
        assert_eq!(advisories.len(), 1);
        assert_eq!(advisories[0].id, "GHSA-aaaa-bbbb-cccc");
        assert_eq!(advisories[0].summary, "credential leak");
    }

    #[test]
    fn parse_advisory_tags_extracts_bare_id_without_summary() {
        let body = "Security-Advisory: GHSA-aaaa-bbbb-cccc\n";
        let advisories = parse_advisory_tags(body);
        assert_eq!(advisories.len(), 1);
        assert_eq!(advisories[0].id, "GHSA-aaaa-bbbb-cccc");
        assert!(advisories[0].summary.is_empty());
    }

    #[test]
    fn parse_advisory_tags_handles_em_dash_summary_separator() {
        // Release notes commonly use an em-dash because the colon is
        // already part of the `Security-Advisory:` header.
        let body = "- Security-Advisory: GHSA-aaaa-bbbb-cccc — update flow CVE-style leak\n";
        let advisories = parse_advisory_tags(body);
        assert_eq!(advisories.len(), 1);
        assert_eq!(advisories[0].id, "GHSA-aaaa-bbbb-cccc");
        assert_eq!(advisories[0].summary, "update flow CVE-style leak");
    }

    #[test]
    fn parse_advisory_tags_is_case_insensitive_on_header() {
        let body = "security-advisory: GHSA-aaaa-bbbb-cccc\n";
        let advisories = parse_advisory_tags(body);
        assert_eq!(advisories.len(), 1);
        assert_eq!(advisories[0].id, "GHSA-aaaa-bbbb-cccc");
    }

    #[test]
    fn parse_advisory_tags_accepts_cve_and_rustsec_ids() {
        let body = "\
Security-Advisory: CVE-2026-1234: deps issue
Security-Advisory: RUSTSEC-2026-0001
";
        let advisories = parse_advisory_tags(body);
        assert_eq!(advisories.len(), 2);
        assert_eq!(advisories[0].id, "CVE-2026-1234");
        assert_eq!(advisories[1].id, "RUSTSEC-2026-0001");
    }

    #[test]
    fn parse_advisory_tags_deduplicates_repeated_ids() {
        let body = "\
Security-Advisory: GHSA-aaaa-bbbb-cccc: first mention
Security-Advisory: GHSA-aaaa-bbbb-cccc: second mention
";
        let advisories = parse_advisory_tags(body);
        assert_eq!(advisories.len(), 1, "duplicate IDs must collapse");
        assert_eq!(advisories[0].summary, "first mention");
    }

    #[test]
    fn parse_advisory_tags_ignores_unrelated_lines() {
        let body = "## Notes\n\nNothing security here.\n\n## Other\n\nstill nothing\n";
        assert!(parse_advisory_tags(body).is_empty());
    }

    #[test]
    fn parse_advisory_tags_handles_list_item_prefixes() {
        let body = "\
- Security-Advisory: GHSA-1111-2222-3333: dash bullet
* Security-Advisory: GHSA-4444-5555-6666: asterisk bullet
  Security-Advisory: GHSA-7777-8888-9999: indented
";
        let ids: Vec<_> = parse_advisory_tags(body)
            .into_iter()
            .map(|a| a.id)
            .collect();
        assert_eq!(
            ids,
            vec![
                "GHSA-1111-2222-3333".to_string(),
                "GHSA-4444-5555-6666".to_string(),
                "GHSA-7777-8888-9999".to_string(),
            ]
        );
    }

    #[test]
    fn parse_advisory_tags_drops_lines_with_empty_id() {
        // A malformed `Security-Advisory: ` (no ID) must not produce
        // a phantom advisory.
        let body = "Security-Advisory:\nSecurity-Advisory: GHSA-aaaa-bbbb-cccc\n";
        let advisories = parse_advisory_tags(body);
        assert_eq!(advisories.len(), 1);
        assert_eq!(advisories[0].id, "GHSA-aaaa-bbbb-cccc");
    }

    #[test]
    fn parse_advisory_tags_rejects_prose_after_prefix() {
        // Council MAJOR: a release body where someone wrote
        // `Security-Advisory: see our linked security policy` must
        // not register "see our linked security policy" as an
        // advisory ID. Only GHSA / CVE / RUSTSEC identifiers count.
        let body = "Security-Advisory: see our linked security policy for details\n";
        assert!(parse_advisory_tags(body).is_empty());
    }

    #[test]
    fn parse_advisory_tags_rejects_unknown_scheme() {
        let body = "Security-Advisory: OWNTRACKER-12345-XYZ\n";
        assert!(parse_advisory_tags(body).is_empty());
    }

    #[test]
    fn parse_advisory_tags_rejects_id_with_spaces() {
        // `is_recognised_advisory_id` requires alphanumerics + dashes;
        // an embedded space disqualifies even a GHSA-looking prefix.
        let body = "Security-Advisory: GHSA-aaaa bbbb-cccc\n";
        assert!(parse_advisory_tags(body).is_empty());
    }

    // ─── Advisory probe wiring ───────────────────────────────────

    #[tokio::test(flavor = "current_thread")]
    async fn check_surfaces_advisory_from_release_body() {
        // This is the spec's named validation test
        // (`commands::version::tests::check_surfaces_advisory`):
        // fetch the running version's release, parse its body,
        // return the advisory tag.
        let server = wiremock::MockServer::start().await;
        let payload = serde_json::json!({
            "tag_name": "v0.6.2-beta",
            "body": "## Notes\n\nSecurity-Advisory: GHSA-aaaa-bbbb-cccc: credential leak in update flow\n",
        })
        .to_string();
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/releases/tags/v0.6.2-beta"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(payload))
            .mount(&server)
            .await;
        let url = format!("{}/releases/tags/v0.6.2-beta", server.uri());
        let result = fetch_advisories_from(&url)
            .await
            .expect("probe must succeed");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "GHSA-aaaa-bbbb-cccc");
        assert_eq!(result[0].summary, "credential leak in update flow");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn check_returns_empty_vec_when_release_has_no_advisories() {
        let server = wiremock::MockServer::start().await;
        let payload = serde_json::json!({
            "tag_name": "v0.6.2-beta",
            "body": "Routine release. No security issues.",
        })
        .to_string();
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(payload))
            .mount(&server)
            .await;
        let url = format!("{}/releases/tags/v0.6.2-beta", server.uri());
        let result = fetch_advisories_from(&url)
            .await
            .expect("probe must succeed");
        assert!(
            result.is_empty(),
            "release with no advisory tags must return empty vec, not None"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn check_returns_none_on_404() {
        // A deleted release / private repo / wrong tag must propagate
        // as `None` so the caller renders "unavailable" rather than
        // misclaiming "no advisories".
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let url = format!("{}/releases/tags/v0.6.2-beta", server.uri());
        let result = fetch_advisories_from(&url).await;
        assert!(result.is_none(), "404 must yield None, got {result:?}");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn check_returns_none_on_malformed_json() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("{not json"))
            .mount(&server)
            .await;
        let url = format!("{}/releases/tags/v0.6.2-beta", server.uri());
        assert!(fetch_advisories_from(&url).await.is_none());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn check_returns_empty_when_body_field_missing() {
        // Some releases publish without a body. That's a valid
        // response — no advisories — not a network failure.
        let server = wiremock::MockServer::start().await;
        let payload = serde_json::json!({"tag_name": "v0.6.2-beta"}).to_string();
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(payload))
            .mount(&server)
            .await;
        let url = format!("{}/releases/tags/v0.6.2-beta", server.uri());
        let result = fetch_advisories_from(&url)
            .await
            .expect("probe must succeed");
        assert!(result.is_empty());
    }

    // ─── JSON shape ──────────────────────────────────────────────

    #[test]
    fn advisory_probe_json_shape_distinguishes_not_probed_from_empty() {
        let not_probed = AdvisoryProbe::NotProbed;
        let not_probed_shape = not_probed.json_shape();
        assert!(!not_probed_shape.checked);
        assert!(!not_probed_shape.available);

        let probed_empty = AdvisoryProbe::Probed(vec![]);
        let probed_empty_shape = probed_empty.json_shape();
        assert!(probed_empty_shape.checked);
        assert!(probed_empty_shape.available);
        assert!(probed_empty_shape.items.is_empty());

        let unavailable = AdvisoryProbe::Unavailable;
        let unavailable_shape = unavailable.json_shape();
        assert!(unavailable_shape.checked);
        assert!(!unavailable_shape.available);
    }
}
