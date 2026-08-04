//! Canonical rendering of user-facing file paths (CIB-237).
//!
//! Before this module, four surfaces rendered locations four ways: `check`
//! emitted repo-relative `src/app.py`, the secret scanner emitted a
//! leading-slash `/.env`, `workspace_root()` leaked Windows NT-extended
//! `\\?\C:\...` prefixes whenever a prefix strip missed, and `skill install`
//! joined forward-slash literals onto native roots.
//!
//! The rule this module implements is one style per platform:
//!
//! - a path inside the workspace renders **repo-relative with `/`
//!   separators** — the style the majority of surfaces already emitted, and
//!   the one editors and terminal linkifiers resolve;
//! - any other path renders as a **normalised absolute path** in native
//!   separators, never carrying a `\\?\` verbatim prefix.
//!
//! Prefix stripping is deliberately done on normalised *strings* rather than
//! via [`Path::strip_prefix`]. On Windows `git rev-parse --show-toplevel`
//! yields `C:/a/b` while the directory walker yields `C:\a\b`; a component
//! comparison misses, and every affected finding silently degraded to an
//! absolute path. Normalising both sides first is what makes those two agree,
//! and it keeps the logic testable on every platform rather than only on
//! Windows CI.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// The Windows NT-extended ("verbatim") prefix, e.g. `\\?\C:\project`.
const VERBATIM_PREFIX: &str = r"\\?\";
/// The verbatim UNC prefix, e.g. `\\?\UNC\server\share`.
const VERBATIM_UNC_PREFIX: &str = r"\\?\UNC\";

/// Remove a Windows NT-extended prefix so it never reaches user-facing output.
///
/// `\\?\UNC\server\share` becomes `\\server\share`; `\\?\C:\x` becomes `C:\x`.
/// Any other input is returned untouched, so this is a no-op on Unix paths.
///
/// Returns [`Cow`] because the UNC form has to regain the leading `\\` that
/// the verbatim prefix subsumes — without it, `\\?\UNC\server\share` would
/// render as `server\share`, which reads as a relative path.
pub fn strip_verbatim_prefix(path: &str) -> Cow<'_, str> {
    if let Some(rest) = path.strip_prefix(VERBATIM_UNC_PREFIX) {
        return Cow::Owned(format!(r"\\{rest}"));
    }
    Cow::Borrowed(path.strip_prefix(VERBATIM_PREFIX).unwrap_or(path))
}

/// Normalise separators to `/` for display of workspace-relative paths.
fn to_display_separators(path: &str) -> String {
    path.replace('\\', "/")
}

/// Compare path segments for prefix purposes.
///
/// Windows path comparison is case-insensitive, so `C:/Project` and
/// `c:/project` name the same directory and must strip. Unix is
/// case-sensitive and folding there would strip prefixes that do not match.
///
/// Known limitation: the fold is ASCII-only, while NTFS folds Unicode. A root
/// containing non-ASCII letters whose case differs between `git rev-parse` and
/// the directory walker still degrades to an absolute path on Windows. Fixing
/// it needs a Unicode-aware fold; the failure mode is a less tidy path, not a
/// wrong one.
fn segments_match(left: &str, right: &str) -> bool {
    if cfg!(windows) {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

/// Strip `root` from `path` when `path` lies inside it, honouring component
/// boundaries so `/a/b` is not treated as a prefix of `/a/bc`.
///
/// Both sides are separator-normalised first. Returns `None` when `path` is
/// not inside `root`.
fn strip_root(path: &str, root: &str) -> Option<String> {
    let path = to_display_separators(path);
    let root = to_display_separators(root);
    let root = root.trim_end_matches('/');
    if root.is_empty() || root == "." {
        return Some(path.trim_start_matches("./").to_string());
    }

    let candidate = path.get(..root.len())?;
    if !segments_match(candidate, root) {
        return None;
    }
    let rest = &path[root.len()..];
    // The remainder must start at a component boundary, or be empty when the
    // path *is* the root.
    if rest.is_empty() {
        return Some(String::new());
    }
    let rest = rest.strip_prefix('/')?;
    // Collapse a leading `./`, which `Path::strip_prefix` used to drop for us.
    // `check` feeds this value to a `git check-attr` set lookup, so a stray
    // `./` would miss the generated-file set and un-suppress a finding.
    Some(rest.trim_start_matches("./").to_string())
}

/// Render a finding location path for human and JSON output.
///
/// Returns a repo-relative path with `/` separators when `path` is inside
/// `root`, and a normalised absolute path otherwise. A path that is already
/// relative is passed through with its separators normalised.
pub fn render(path: &str, root: Option<&Path>) -> String {
    let path = strip_verbatim_prefix(path);
    let path = path.as_ref();

    if let Some(root) = root {
        let root = root.to_string_lossy();
        let root = strip_verbatim_prefix(&root);
        if let Some(relative) = strip_root(path, root.as_ref())
            && !relative.is_empty()
        {
            return relative;
        }
    }

    if is_absolute_display(path) {
        // Outside the workspace: keep the platform's own absolute form, minus
        // any verbatim prefix already removed above.
        return path.to_string();
    }

    to_display_separators(path)
}

/// Whether a path string should be shown as absolute.
///
/// Checked on the string rather than via [`Path::is_absolute`] so a Windows
/// path evaluated on a Unix host (tests, cross-platform fixtures) is still
/// classified correctly.
fn is_absolute_display(path: &str) -> bool {
    if path.starts_with('/') || path.starts_with('\\') {
        return true;
    }
    let bytes = path.as_bytes();
    // Drive-letter form: `C:\x` or `C:/x`.
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

/// Whether the secret scanner, given `root`, would report `scanned` as `emitted`.
///
/// Mirrors `anvil_checks::secret::normalise_file_path`, which is private to
/// that crate: it returns `/{relative}` when it can strip the root and the raw
/// path when it cannot.
fn scanner_would_report(scanned: &str, emitted: &str, root: Option<&Path>) -> bool {
    if scanned == emitted {
        return true;
    }
    let Some(root) = root else { return false };
    let root = root.to_string_lossy();
    strip_root(
        strip_verbatim_prefix(scanned).as_ref(),
        strip_verbatim_prefix(&root).as_ref(),
    )
    .is_some_and(|relative| {
        !relative.is_empty() && emitted.strip_prefix('/') == Some(relative.as_str())
    })
}

/// Render a secret-scanner finding path, resolved against the files that were
/// actually scanned.
///
/// The scanner emits two shapes: `/{relative}` when it could strip the
/// workspace root, and the raw **absolute** path when it could not. From the
/// string alone these are indistinguishable — `/etc/secrets/prod.env` is a
/// valid instance of either — so stripping the leading `/` unconditionally
/// turns a genuine absolute path into a relative one pointing somewhere else
/// entirely. On a secret-reporting surface that is worse than the
/// inconsistency it fixes.
///
/// `scanned` settles it: whichever path produced this finding is the truth, and
/// it then renders like any other path. This mirrors how `audit` maps findings
/// back through its own `rel_by_abs` index. When nothing matches, the fallback
/// never strips a leading `/`, because an unproven marker is not a marker.
///
/// The scanner's `/`-prefix contract is corrected here rather than at source
/// because `activation::baseline` keys stored findings on it.
pub fn render_secret_finding(file: &str, scanned: &[&str], root: Option<&Path>) -> String {
    if let Some(origin) = scanned
        .iter()
        .copied()
        .find(|candidate| scanner_would_report(candidate, file, root))
    {
        return render(origin, root);
    }
    render(file, root)
}

/// Join a `/`-separated relative literal onto `base`, one component at a time.
///
/// [`Path::join`] treats the literal as opaque and keeps its forward slashes,
/// so a native root plus `".claude/skills"` yields
/// `C:\Users\dev\.claude/skills` on Windows — a usable path that displays with
/// mixed separators (CIB-237). Pushing components lets the platform choose its
/// own separator throughout.
pub fn join_relative(base: &Path, relative: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    for component in relative.split('/').filter(|c| !c.is_empty()) {
        path.push(component);
    }
    path
}

/// Canonicalise without producing a Windows NT-extended prefix.
///
/// [`std::fs::canonicalize`] returns `\\?\C:\...` on Windows, which leaks into
/// every surface that prints or prefix-strips against the workspace root.
/// `dunce` returns the ordinary form whenever the path is representable as
/// one, and is identical to `std::fs::canonicalize` on Unix.
pub fn canonicalise(path: &Path) -> std::io::Result<PathBuf> {
    dunce::canonicalize(path)
}

/// Format a location suffix, omitting the line when it is the whole-file
/// sentinel.
///
/// `anvil audit` uses `line: 0` to mean "this finding is about the file
/// itself" (a committed `.env`, say) rather than a numbered line. The SARIF
/// adapter already omits the region for it; rendering it as `.env:0` in the
/// other surfaces implied a zero-based line number that no anvil surface uses.
pub fn format_location(file: &str, line: usize) -> String {
    if line == 0 {
        file.to_string()
    } else {
        format!("{file}:{line}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_windows_verbatim_prefix() {
        assert_eq!(
            strip_verbatim_prefix(r"\\?\C:\project\src"),
            r"C:\project\src"
        );
    }

    #[test]
    fn strips_verbatim_unc_prefix_to_usable_unc_path() {
        assert_eq!(
            strip_verbatim_prefix(r"\\?\UNC\server\share\file.txt"),
            r"\\server\share\file.txt",
            "a UNC path must keep its leading `\\\\` or it reads as relative"
        );
        assert!(is_absolute_display(&strip_verbatim_prefix(
            r"\\?\UNC\server\share\file.txt"
        )));
    }

    #[test]
    fn leaves_ordinary_paths_untouched() {
        assert_eq!(
            strip_verbatim_prefix("/home/dev/project"),
            "/home/dev/project"
        );
        assert_eq!(strip_verbatim_prefix("src/app.py"), "src/app.py");
    }

    #[test]
    fn renders_path_inside_root_as_repo_relative() {
        let root = PathBuf::from("/home/dev/project");
        assert_eq!(
            render("/home/dev/project/src/app.py", Some(&root)),
            "src/app.py"
        );
    }

    #[test]
    fn renders_windows_path_inside_root_with_forward_slashes() {
        let root = PathBuf::from(r"C:\project");
        assert_eq!(render(r"C:\project\src\app.py", Some(&root)), "src/app.py");
    }

    /// The regression that made findings degrade to absolute paths on Windows:
    /// git reports the toplevel with `/`, the walker yields `\`.
    #[test]
    fn strips_root_despite_mixed_separators() {
        let root = PathBuf::from("C:/project");
        assert_eq!(render(r"C:\project\src\app.py", Some(&root)), "src/app.py");
    }

    #[test]
    fn strips_verbatim_root_against_ordinary_path() {
        let root = PathBuf::from(r"\\?\C:\project");
        assert_eq!(render(r"C:\project\src\app.py", Some(&root)), "src/app.py");
    }

    #[test]
    fn does_not_strip_a_sibling_sharing_a_name_prefix() {
        let root = PathBuf::from("/home/dev/project");
        // `/home/dev/project-old` must not become `-old/src/app.py`.
        assert_eq!(
            render("/home/dev/project-old/src/app.py", Some(&root)),
            "/home/dev/project-old/src/app.py"
        );
    }

    #[test]
    fn renders_path_outside_root_as_normalised_absolute() {
        let root = PathBuf::from("/home/dev/project");
        assert_eq!(render("/etc/hosts", Some(&root)), "/etc/hosts");
        assert_eq!(
            render(r"\\?\C:\elsewhere\file.txt", Some(&root)),
            r"C:\elsewhere\file.txt"
        );
    }

    #[test]
    fn passes_relative_paths_through_with_normalised_separators() {
        assert_eq!(render(r"src\app.py", None), "src/app.py");
        assert_eq!(render("src/app.py", None), "src/app.py");
    }

    #[test]
    fn treats_dot_root_as_no_prefix() {
        let root = PathBuf::from(".");
        assert_eq!(render("./src/app.py", Some(&root)), "src/app.py");
    }

    /// The two styles that appeared side by side in one gate run.
    #[test]
    fn secret_and_antipattern_findings_render_in_the_same_style() {
        let root = PathBuf::from("/home/dev/project");
        let scanned = ["/home/dev/project/.env"];
        let secret = render_secret_finding("/.env", &scanned, Some(&root));
        let antipattern = render("src/app.py", Some(&root));
        assert_eq!(secret, ".env");
        assert_eq!(antipattern, "src/app.py");
        assert!(
            !secret.starts_with('/'),
            "secret path must not lead with a slash"
        );
    }

    #[test]
    fn secret_finding_with_absolute_path_still_relativises() {
        let root = PathBuf::from("/home/dev/project");
        let scanned = ["/home/dev/project/src/keys.ts"];
        assert_eq!(
            render_secret_finding("/home/dev/project/src/keys.ts", &scanned, Some(&root)),
            "src/keys.ts"
        );
    }

    /// A secret found OUTSIDE the workspace must keep its absolute path.
    /// Stripping the leading `/` as if it were the scanner's marker would
    /// rewrite `/etc/secrets/prod.env` into a relative path naming a
    /// different file — on a secret-reporting surface.
    #[test]
    fn secret_finding_outside_the_root_stays_absolute() {
        let root = PathBuf::from("/home/dev/project");
        let scanned = ["/etc/secrets/prod.env"];
        assert_eq!(
            render_secret_finding("/etc/secrets/prod.env", &scanned, Some(&root)),
            "/etc/secrets/prod.env"
        );
    }

    /// The symlinked-root case that makes the scanner fall back to a raw
    /// absolute path: `/tmp` -> `/private/tmp` on macOS, or a symlinked
    /// worktree. The finding must not be mangled into a relative path.
    #[test]
    fn secret_finding_survives_a_root_form_mismatch() {
        let root = PathBuf::from("/private/tmp/real");
        let scanned = ["/tmp/link/src/keys.ts"];
        assert_eq!(
            render_secret_finding("/tmp/link/src/keys.ts", &scanned, Some(&root)),
            "/tmp/link/src/keys.ts"
        );
    }

    /// With nothing to resolve against, an unproven marker is not a marker.
    #[test]
    fn secret_finding_never_strips_an_unproven_leading_slash() {
        let root = PathBuf::from("/home/dev/project");
        assert_eq!(
            render_secret_finding("/etc/passwd", &[], Some(&root)),
            "/etc/passwd"
        );
    }

    /// CIB-199 generated-file filtering keys on this string, so an interior
    /// `.` component must not survive into the lookup.
    #[test]
    fn collapses_a_leading_dot_segment() {
        let root = PathBuf::from("/home/dev/project");
        assert_eq!(render("/home/dev/project/./src/a.rs", Some(&root)), "src/a.rs");
    }

    #[test]
    fn omits_the_whole_file_sentinel_line() {
        assert_eq!(format_location(".env", 0), ".env");
    }

    #[test]
    fn renders_real_line_numbers() {
        assert_eq!(format_location("src/app.py", 12), "src/app.py:12");
        assert_eq!(format_location(".env", 1), ".env:1");
    }

    #[test]
    fn join_relative_uses_native_separators_throughout() {
        let base = PathBuf::from("/home/dev");
        let joined = join_relative(&base, ".claude/skills");
        // Compare STRINGS, not `PathBuf`s. `Path` equality is component-wise
        // and Windows accepts `/` as a separator, so
        // `C:\\x\\.claude/skills == C:\\x\\.claude\\skills` compares equal and
        // the mixed-separator bug is invisible to it. Only the string form
        // distinguishes them.
        assert_eq!(
            joined.to_str(),
            base.join(".claude").join("skills").to_str(),
            "join_relative must produce the same STRING as a component-wise join"
        );
    }

    #[test]
    fn join_relative_handles_single_and_empty_segments() {
        let base = PathBuf::from("/home/dev");
        assert_eq!(
            join_relative(&base, "manifest.json"),
            base.join("manifest.json")
        );
        assert_eq!(join_relative(&base, ""), base);
    }

    #[test]
    fn join_relative_does_not_absorb_traversal_segments() {
        // The traversal guards run before this helper; it must not quietly
        // resolve or drop `..`, or those guards would be inspecting a
        // different path than the one opened.
        let base = PathBuf::from("/home/dev");
        assert_eq!(
            join_relative(&base, "a/../b"),
            base.join("a").join("..").join("b")
        );
    }

    #[test]
    fn segment_comparison_follows_platform_case_rules() {
        if cfg!(windows) {
            assert!(segments_match("C:/Project", "c:/project"));
        } else {
            assert!(!segments_match("/home/Project", "/home/project"));
        }
        assert!(segments_match("/home/project", "/home/project"));
    }
}
