use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Re-export of [`anvil_kernel::watcher::filter::is_ignored_dir_name`] —
/// the canonical denylist lives in `anvil-kernel` so the watcher and the
/// cli command surfaces (`audit`, `baseline`, `check`, `drift`, `gate`)
/// cannot drift. Add new entries in `anvil-kernel/src/watcher/filter.rs`,
/// not here.
pub(crate) use anvil_kernel::watcher::filter::is_ignored_dir_name;

/// Build the [`anvil_checks::secret::SecretCheckConfig`] used by the
/// secret-scan surfaces (`audit`, `check`, `gate`) for a project rooted at
/// `root`.
///
/// This is the **single seam** through which project-level secret-scan
/// configuration flows. Today it returns the defaults, but consolidating the
/// three call sites here means the planned `.anvilrc` allowlist/exclude
/// surface becomes a change to *this one function* — the commands never need
/// to touch it again.
///
/// When that surface lands, map the project config's allowlist entries into
/// `SecretCheckConfig::custom_allowlist`. Suppressions from those entries are
/// already recorded with `AllowlistProvenance::Custom` and surfaced at scan
/// time (see [`secret_suppression_note`]), so an `.anvilrc` opt-out can never
/// silently hide a real credential — the operator sees every allowlisted match
/// called out, with the pattern that suppressed it.
#[must_use]
pub(crate) fn secret_check_config(_root: &Path) -> anvil_checks::secret::SecretCheckConfig {
    // EXTENSION POINT: load `.anvilrc` from `_root` and fold its secret-scan
    // allowlist into `custom_allowlist` here.
    anvil_checks::secret::SecretCheckConfig::default()
}

/// Render a one-line, non-noisy callout summarising allowlist suppressions
/// from a secret scan, or `None` when nothing was suppressed. Keeps the raw
/// total terse but breaks out the operator-configured (`.anvilrc`) count,
/// since those are the suppressions that can mask a genuine credential and so
/// must never pass unseen. Full per-entry provenance lives in the structured
/// `SecretCheckResult::suppressions` for callers that surface it.
#[must_use]
pub(crate) fn secret_suppression_note(
    suppressions: &[anvil_checks::secret::Suppression],
) -> Option<String> {
    if suppressions.is_empty() {
        return None;
    }
    let operator = suppressions
        .iter()
        .filter(|s| s.provenance.is_operator_configured())
        .count();
    let detail = if operator > 0 {
        format!(" ({operator} via project allowlist)")
    } else {
        String::new()
    };
    Some(format!(
        "ℹ {} match(es) withheld by allowlist (not flagged){detail}",
        suppressions.len()
    ))
}

/// [`secret_suppression_note`] rendered as a message suffix: a `\n\n`-separated
/// block ready to append to a check message, or an empty string when nothing
/// was suppressed.
#[must_use]
pub(crate) fn secret_suppression_suffix(
    suppressions: &[anvil_checks::secret::Suppression],
) -> String {
    secret_suppression_note(suppressions).map_or(String::new(), |note| format!("\n\n{note}"))
}

/// Resolve the user's home directory, honouring the platform's home
/// environment variable before the OS known-folder API.
///
/// `dirs::home_dir()` on Windows reads `FOLDERID_Profile` via the Known
/// Folder API and **ignores `%USERPROFILE%`**; on Unix it honours `$HOME`.
/// anvil reads and writes editor MCP config under the home dir
/// (`~/.cursor/mcp.json`, `~/.claude.json`) and detects installed clients
/// from there, so a home that diverges from the one the user's shell and
/// editor actually use makes anvil install to — and report on — the wrong
/// location. On Windows `%USERPROFILE%` can differ from the known-folder
/// profile (redirected/roaming/relocated profiles), which surfaced as
/// activation over-claims and "anvil installed it but my editor can't see
/// it" reports. Preferring the platform home env var keeps anvil aligned
/// with the user's environment, and lets tests isolate home via
/// `USERPROFILE`/`HOME`. Falls back to `dirs::home_dir()` when the env var
/// is unset or empty.
pub fn user_home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let from_env = std::env::var_os("USERPROFILE");
    #[cfg(not(windows))]
    let from_env = std::env::var_os("HOME");

    from_env
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
}

/// Resolve the workspace root via `git rev-parse --show-toplevel`.
///
/// Canonicalises the git result to collapse symlinks. Falls back to
/// the current directory (returned as-is, not canonicalised). Returns
/// an error only when no usable path can be determined.
pub fn workspace_root() -> Result<PathBuf> {
    let git_failure = match std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        Ok(output) if output.status.success() => {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                let root = PathBuf::from(stdout.trim());
                if let Ok(canonical) = root.canonicalize() {
                    return Ok(canonical);
                }
                return Ok(root);
            }

            Some("git rev-parse returned non-UTF-8 output".to_string())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                Some(format!(
                    "git rev-parse failed with status {}",
                    output.status
                ))
            } else {
                Some(format!(
                    "git rev-parse failed with status {}: {}",
                    output.status, stderr
                ))
            }
        }
        Err(err) => Some(format!("failed to run git rev-parse: {err}")),
    };

    std::env::current_dir().with_context(|| {
        if let Some(reason) = &git_failure {
            format!("failed to determine workspace root: {reason}; current directory unresolvable")
        } else {
            "failed to determine workspace root: current directory unresolvable".to_string()
        }
    })
}

/// Format an `anyhow::Error` for user-facing display with path-leakage guardrails.
///
/// - Default (`verbose = false`): prints only the outermost context (`{err}`).
///   Relies on the outer context being a programmer-written, path-free string
///   (e.g. `"starting engine watcher"`).
/// - Verbose (`verbose = true`): prints the full anyhow chain (`{err:#}`),
///   which may include absolute paths from `notify::Error`, `std::io::Error`,
///   or similar filesystem-origin errors.
///
/// **Blind spot**: if a caller constructs context strings that embed paths
/// (e.g. `.with_context(|| format!("reading {}", path.display()))`), those
/// paths are part of the outermost message and WILL appear even at
/// `verbose = false`. The convention in `docs/guides/cli-output-streams.md`
/// forbids path-embedding context strings on error chains routed through
/// this helper — this function does not redact them automatically.
pub fn format_user_error(err: &anyhow::Error, verbose: bool) -> String {
    if verbose {
        format!("{err:#}")
    } else {
        format!("{err}")
    }
}

/// CIB-199: return the subset of `paths` that `.gitattributes` marks as
/// `linguist-generated` (value `true` or `set`), via one batched
/// `git check-attr`. The returned strings are exactly the input strings that
/// matched, so callers can filter with set membership whether `paths` are
/// workspace-relative or absolute (git echoes each path back verbatim).
///
/// Best-effort: any git failure (no repo, git absent, non-zero exit) yields an
/// empty set, so anti-pattern scanning behaves exactly as before wherever the
/// attribute is unused.
pub(crate) fn git_generated_paths(
    root: &Path,
    paths: &[String],
) -> std::collections::HashSet<String> {
    use std::process::{Command, Stdio};

    let mut generated = std::collections::HashSet::new();
    if paths.is_empty() {
        return generated;
    }

    let Ok(mut child) = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["check-attr", "--stdin", "-z", "linguist-generated"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return generated;
    };

    if let Some(mut stdin) = child.stdin.take() {
        let mut buf = Vec::new();
        for path in paths {
            buf.extend_from_slice(path.as_bytes());
            buf.push(0);
        }
        // Ignore write errors — a broken pipe surfaces as a non-success exit,
        // which we already treat as "no exclusions".
        let _ = stdin.write_all(&buf);
    }

    let output = match child.wait_with_output() {
        Ok(output) if output.status.success() => output,
        _ => return generated,
    };

    // `-z` output is NUL-separated triples: <path>\0<attr>\0<value>\0…
    let mut fields = output.stdout.split(|&byte| byte == 0);
    while let (Some(path), Some(_attr), Some(value)) = (fields.next(), fields.next(), fields.next())
    {
        let value = std::str::from_utf8(value).unwrap_or_default();
        if value == "true" || value == "set" {
            generated.insert(String::from_utf8_lossy(path).into_owned());
        }
    }

    generated
}

/// Write `data` to `path` atomically by writing to a uniquely-named temporary
/// file in the same directory and then renaming. This prevents partial/corrupt
/// state files if the process crashes or is interrupted mid-write.
///
/// Uses `tempfile` for unpredictable filenames (prevents symlink attacks).
/// On Unix the temp file is created with mode 0o600.
///
/// Note: this provides process-crash atomicity, not power-loss durability
/// (no `fsync` before rename). Callers that need a stricter guard against
/// symlinked parent directories (e.g. the MCP install path, where the
/// target file lives in `$HOME` and a redirected parent would leak auth
/// tokens to an unintended directory) should call
/// [`refuse_if_parent_is_symlink`] before invoking this function. The
/// function intentionally does NOT enforce that guard itself — broad
/// callers (`.anvilrc`, baseline snapshots, etc.) legitimately run inside
/// symlinked workspace roots and must not be blocked.
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));

    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut builder = tempfile::Builder::new();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o600));
    }

    let mut tmp = builder
        .tempfile_in(dir)
        .with_context(|| format!("creating temp file in {}", dir.display()))?;

    tmp.write_all(data)
        .with_context(|| format!("writing temp file for {}", path.display()))?;
    tmp.flush()
        .with_context(|| format!("flushing temp file for {}", path.display()))?;

    let tmp_path = tmp.into_temp_path();
    let tmp_display = tmp_path.display().to_string();

    // On Windows, TempPath::persist uses std::fs::rename under the hood, which
    // fails if the destination already exists. Remove the existing file first.
    #[cfg(windows)]
    {
        if let Err(err) = std::fs::remove_file(path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                return Err(err)
                    .with_context(|| format!("removing existing file {}", path.display()));
            }
        }
    }

    tmp_path
        .persist(path)
        .with_context(|| format!("persisting {tmp_display} -> {}", path.display()))?;

    // On Windows, restrict the file to the current user only (matching Unix 0o600).
    // icacls is available on all modern Windows (Vista+).
    #[cfg(windows)]
    {
        restrict_windows_permissions(path);
    }

    Ok(())
}

/// Refuse if the immediate parent directory of `target` exists and is a
/// symlink.
///
/// **Threat model (LAUNCH-009.5):** the MCP install path writes editor
/// config files at `~/.cursor/mcp.json` and `~/.claude.json`. POSIX
/// `rename(2)` replaces a symlink at the *target* path safely (the
/// symlink is destroyed, not followed), but the *temp file* used by
/// `atomic_write`'s `tempfile_in(parent)` writes through the parent's
/// symlink. A `~/.cursor` symlink pointing outside `$HOME` would let
/// the install path land a sensitive config file (e.g. `.claude.json`
/// carries auth tokens) in an unintended directory.
///
/// **Scoping:** this guard is opt-in. `atomic_write` and `write_new`
/// do NOT enforce it themselves — broad callers (`.anvilrc`, baseline
/// snapshots, credential caches) legitimately run inside symlinked
/// workspace roots and refusing those would break ordinary developer
/// workflows. Only the MCP install path
/// (`activation::orchestrator::install`) calls this guard before
/// writing.
///
/// **Granularity:** stricter than necessary (a HOME-containment check
/// would be finer-grained but platform-fragile). Users with
/// intentionally-symlinked editor config dirs should resolve the
/// symlink (`mv` the real dir into place) before running
/// `anvil start`.
pub fn refuse_if_parent_is_symlink(target: &Path) -> Result<()> {
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    match std::fs::symlink_metadata(dir) {
        Ok(md) if md.file_type().is_symlink() => {
            anyhow::bail!(
                "refusing to write {} — its parent directory {} is a symlink. \
                 Resolve the symlink (move the real directory into place) \
                 and re-run.",
                target.display(),
                dir.display(),
            )
        }
        _ => Ok(()),
    }
}

/// Create `path` exclusively and write `data`. Fails with `AlreadyExists` if
/// the file is already present — use this instead of `atomic_write` when the
/// caller has already decided "only create, do not overwrite", so the
/// check-then-write window cannot be exploited by a concurrent writer.
///
/// On Unix the file is created with mode 0o600.
pub fn write_new(path: &Path, data: &[u8]) -> Result<()> {
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts
        .open(path)
        .with_context(|| format!("creating {}", path.display()))?;
    file.write_all(data)
        .with_context(|| format!("writing {}", path.display()))?;
    file.flush()
        .with_context(|| format!("flushing {}", path.display()))?;

    // On Windows, restrict the file to the current user only (matching the
    // Unix 0o600 set at creation time). Best-effort; emits a warning rather
    // than failing the write if icacls is unavailable.
    #[cfg(windows)]
    {
        restrict_windows_permissions(path);
    }

    Ok(())
}

#[cfg(windows)]
fn current_user_sid() -> Result<String> {
    let output = std::process::Command::new("whoami")
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .context("failed to run whoami /user")?;

    if !output.status.success() {
        anyhow::bail!("whoami /user exited with status {}", output.status);
    }

    let stdout =
        String::from_utf8(output.stdout).context("whoami /user returned non-UTF-8 output")?;

    let line = stdout
        .lines()
        .find(|l| !l.trim().is_empty())
        .context("whoami returned no user information")?;

    // CSV output: "DOMAIN\User","S-1-5-21-..."
    let trimmed = line.trim().trim_matches('"');
    let sid = trimmed
        .rsplit("\",\"")
        .next()
        .context("whoami CSV output missing SID")?;

    let is_valid_sid = sid.starts_with("S-")
        && sid
            .as_bytes()
            .iter()
            .skip(2)
            .all(|b| b.is_ascii_digit() || *b == b'-');
    if !is_valid_sid {
        anyhow::bail!("whoami returned an invalid SID: {sid}");
    }

    Ok(sid.to_string())
}

/// Restrict a file to the current user only on Windows via `icacls`.
///
/// Uses the current user's SID (via `whoami /user`) instead of the
/// USERNAME environment variable to avoid granting permissions to
/// well-known group names like "Everyone" that happen to be
/// alphanumeric. Best-effort: emits a warning to the `tracing`
/// stream if the restriction cannot be applied but does not fail
/// the write operation. This mirrors the Unix 0o600 set at creation
/// time.
#[cfg(windows)]
fn restrict_windows_permissions(path: &Path) {
    let sid = match current_user_sid() {
        Ok(sid) => sid,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "cannot restrict file permissions: could not determine current user SID",
            );
            return;
        }
    };

    let status = std::process::Command::new("icacls")
        .arg(path)
        .args(["/inheritance:r", "/grant:r"])
        .arg(format!("*{sid}:(F)"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => tracing::warn!(
            path = %path.display(),
            exit = %s,
            "failed to restrict file permissions: icacls exited non-zero",
        ),
        Err(e) => tracing::warn!(
            path = %path.display(),
            error = %e,
            "failed to run icacls",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_checks::secret::{AllowlistProvenance, Suppression};
    use anvil_kernel::watcher::filter::IGNORE_DIRS;

    #[test]
    fn git_generated_paths_honours_linguist_generated() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        let initialised = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["init", "-q"])
            .status()
            .is_ok_and(|s| s.success());
        if !initialised {
            return; // git unavailable — the helper is best-effort, so skip.
        }
        std::fs::write(
            root.join(".gitattributes"),
            "*.gen.ts linguist-generated=true\nsrc/api.ts linguist-generated\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        for rel in ["routeTree.gen.ts", "src/api.ts", "src/app.ts"] {
            std::fs::write(root.join(rel), "x\n").unwrap();
        }

        let paths = vec![
            "routeTree.gen.ts".to_string(),
            "src/api.ts".to_string(),
            "src/app.ts".to_string(),
        ];
        let generated = git_generated_paths(root, &paths);
        assert!(
            generated.contains("routeTree.gen.ts"),
            "linguist-generated=true must be treated as generated"
        );
        assert!(
            generated.contains("src/api.ts"),
            "bare `linguist-generated` (set) must be treated as generated"
        );
        assert!(
            !generated.contains("src/app.ts"),
            "an unmarked file must not be excluded"
        );
    }

    #[test]
    fn git_generated_paths_empty_for_empty_input() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(git_generated_paths(tmp.path(), &[]).is_empty());
    }

    fn suppression(provenance: AllowlistProvenance) -> Suppression {
        Suppression {
            file: "src/x.rs".to_string(),
            line: 1,
            rule_name: "High Entropy String".to_string(),
            redacted_match: "ab...yz".to_string(),
            provenance,
        }
    }

    #[test]
    fn suppression_note_is_none_when_empty() {
        assert!(secret_suppression_note(&[]).is_none());
    }

    #[test]
    fn suppression_note_counts_total_and_omits_operator_detail_for_builtins() {
        let note = secret_suppression_note(&[
            suppression(AllowlistProvenance::BuiltinShape),
            suppression(AllowlistProvenance::BuiltinKeyword),
        ])
        .expect("note for non-empty suppressions");
        assert!(note.contains("2 match(es)"), "got: {note}");
        assert!(
            !note.contains("project allowlist"),
            "built-in-only suppressions must not claim a project allowlist: {note}"
        );
    }

    #[test]
    fn suppression_note_breaks_out_operator_configured_count() {
        let note = secret_suppression_note(&[
            suppression(AllowlistProvenance::BuiltinShape),
            suppression(AllowlistProvenance::Custom {
                pattern: "diag_".to_string(),
            }),
        ])
        .expect("note");
        assert!(note.contains("2 match(es)"), "got: {note}");
        assert!(
            note.contains("1 via project allowlist"),
            "operator-configured suppressions must be called out: {note}"
        );
    }

    #[test]
    fn is_ignored_dir_name_matches_full_list() {
        for entry in IGNORE_DIRS {
            assert!(is_ignored_dir_name(entry), "expected {entry} to be ignored");
        }
    }

    #[test]
    fn is_ignored_dir_name_rejects_unknown() {
        for name in ["src", "tests", "lib", "node_modules.bak", "Target", ""] {
            assert!(
                !is_ignored_dir_name(name),
                "expected {name} to not be ignored"
            );
        }
    }

    /// ADOPT-004: cli's `is_ignored_dir_name` is the public re-export of
    /// the kernel-owned canonical list. They must agree on every entry so
    /// audit/baseline/check/drift/gate (cli consumers) and the watcher
    /// (kernel consumer) cannot diverge.
    #[test]
    fn cli_helper_matches_kernel_canonical() {
        for entry in IGNORE_DIRS {
            assert!(
                is_ignored_dir_name(entry),
                "cli is_ignored_dir_name disagrees with kernel canonical for {entry}",
            );
        }
        for name in ["src", "tests", "lib", "node_modules.bak", "Target"] {
            assert!(
                !is_ignored_dir_name(name),
                "cli is_ignored_dir_name should not match {name}",
            );
        }
    }

    #[test]
    fn atomic_write_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");

        atomic_write(&path, b"hello").unwrap();

        assert!(path.exists());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn atomic_write_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        std::fs::write(&path, "old").unwrap();

        atomic_write(&path, b"new").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn refuse_if_parent_is_symlink_blocks_symlinked_parent() {
        // LAUNCH-009.5 council remediation: a symlinked parent dir would
        // let `tempfile_in` write through the link to an unintended
        // location. Callers that need this guard (currently the MCP
        // install path) opt in before invoking atomic_write.
        use std::os::unix::fs::symlink;

        let real = tempfile::tempdir().unwrap();
        let real_dir = real.path().join("real-config-dir");
        std::fs::create_dir(&real_dir).unwrap();

        let staging = tempfile::tempdir().unwrap();
        let symlinked_parent = staging.path().join("editor-config");
        symlink(&real_dir, &symlinked_parent).unwrap();

        let path = symlinked_parent.join("mcp.json");
        let err = refuse_if_parent_is_symlink(&path).expect_err("symlinked parent must be refused");
        let msg = format!("{err:#}");
        assert!(msg.contains("symlink"), "error must mention symlink: {msg}");
    }

    #[test]
    fn refuse_if_parent_is_symlink_passes_through_real_dir() {
        // Regression guard: the helper must not produce false positives
        // on ordinary directories. atomic_write is used by `.anvilrc`,
        // baseline snapshots, etc. — those paths are NOT installed
        // through this guard, but we still verify the helper itself
        // behaves correctly on real dirs in case a future caller opts in.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.json");
        refuse_if_parent_is_symlink(&path).unwrap();
    }

    #[test]
    fn refuse_if_parent_is_symlink_passes_when_parent_does_not_exist() {
        // The helper should not error when the parent dir doesn't exist
        // yet — the caller will create it via create_dir_all and then
        // re-check (or the subsequent write will surface the I/O error).
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent").join("file.json");
        refuse_if_parent_is_symlink(&path).unwrap();
    }

    #[test]
    fn atomic_write_succeeds_in_subdirectory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("subdir").join("file.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        atomic_write(&path, b"ok").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "ok");
    }

    #[test]
    fn write_new_creates_file_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fresh.rc");

        write_new(&path, b"hello").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn write_new_errors_when_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.rc");
        std::fs::write(&path, "keep-me").unwrap();

        let err = write_new(&path, b"overwrite").unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("creating") || msg.contains("exists"),
            "error message should mention creation failure: {msg}"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "keep-me",
            "write_new must not overwrite an existing file"
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_new_sets_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.rc");

        write_new(&path, b"secret").unwrap();

        let perms = std::fs::metadata(&path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_sets_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.json");

        atomic_write(&path, b"secret").unwrap();

        let perms = std::fs::metadata(&path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600);
    }

    #[test]
    fn format_user_error_default_omits_chain() {
        let inner = anyhow::anyhow!("inotify: /home/victim/secret-project");
        let outer = inner.context("starting engine watcher");

        let msg = format_user_error(&outer, false);

        assert!(
            msg.contains("starting engine watcher"),
            "default mode should include the outer context: {msg}"
        );
        assert!(
            !msg.contains("/home/victim/secret-project"),
            "default mode must not leak wrapped root-cause paths: {msg}"
        );
    }

    #[test]
    fn format_user_error_verbose_includes_chain() {
        let inner = anyhow::anyhow!("inotify: /home/victim/secret-project");
        let outer = inner.context("starting engine watcher");

        let msg = format_user_error(&outer, true);

        assert!(msg.contains("starting engine watcher"), "verbose: {msg}");
        assert!(
            msg.contains("/home/victim/secret-project"),
            "verbose must include the full chain for debugging: {msg}"
        );
    }

    /// Documents the blind spot: paths embedded in the OUTER context string
    /// itself (via `.with_context(|| format!("reading {}", p.display()))`)
    /// are part of the outermost message and will leak even at
    /// `verbose = false`. The convention in `cli-output-streams.md`
    /// forbids this pattern on sites routed through `format_user_error`.
    /// This test locks the behaviour in so a future change to the helper
    /// that silently widened the contract would trip the assertion and
    /// force an explicit convention update.
    #[test]
    fn format_user_error_does_not_redact_paths_in_outer_context() {
        let err = anyhow::anyhow!("io error")
            .context(format!("reading {}", "/home/victim/secret-project"));

        let msg = format_user_error(&err, false);

        assert!(
            msg.contains("/home/victim/secret-project"),
            "path in outer context is NOT redacted — callers must avoid this pattern: {msg}"
        );
    }

    #[test]
    fn workspace_root_returns_absolute_path() {
        let root = workspace_root().unwrap();
        assert!(
            root.is_absolute(),
            "workspace root should be absolute, got: {root:?}"
        );
    }

    #[test]
    fn workspace_root_is_canonical() {
        let root = workspace_root().unwrap();
        if let Ok(canonical) = root.canonicalize() {
            assert_eq!(root, canonical);
        }
    }
}
