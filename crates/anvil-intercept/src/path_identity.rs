//! Worktree path identity for intercept status, claims, and the registry.
//!
//! Mirrors `anvil-cli::display_path` (`same_path` / `strip_verbatim_prefix`
//! / `relative_to`) because this crate cannot depend on the CLI. Do not
//! invent a second comparison here: plain drive paths and Windows `\\?\`
//! / `\\?\UNC\` spellings of the same location must match, including
//! leftover snapshot and registry keys written before the dunce migration.

use std::borrow::Cow;
use std::path::Path;

/// The Windows NT-extended ("verbatim") prefix, e.g. `\\?\C:\project`.
const VERBATIM_PREFIX: &str = r"\\?\";
/// The verbatim UNC prefix, e.g. `\\?\UNC\server\share`.
const VERBATIM_UNC_PREFIX: &str = r"\\?\UNC\";

/// Remove a Windows NT-extended prefix so identity checks see the ordinary form.
///
/// `\\?\UNC\server\share` becomes `\\server\share`; `\\?\C:\x` becomes `C:\x`.
/// Any other input is returned untouched, so this is a no-op on Unix paths.
pub fn strip_verbatim_prefix(path: &str) -> Cow<'_, str> {
    if let Some(rest) = path.strip_prefix(VERBATIM_UNC_PREFIX) {
        return Cow::Owned(format!(r"\\{rest}"));
    }
    Cow::Borrowed(path.strip_prefix(VERBATIM_PREFIX).unwrap_or(path))
}

/// Normalise separators to `/` for identity comparison.
fn to_display_separators(path: &str) -> String {
    path.replace('\\', "/")
}

/// Compare path segments for prefix purposes.
///
/// Windows path comparison is case-insensitive; Unix is case-sensitive.
/// ASCII-only fold, matching `anvil-cli::display_path`.
fn segments_match(left: &str, right: &str) -> bool {
    if cfg!(windows) {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

/// Strip `root` from `path` when `path` lies inside it, honouring component
/// boundaries so `/a/b` is not treated as a prefix of `/a/bc`.
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
    if rest.is_empty() {
        return Some(String::new());
    }
    let rest = rest.strip_prefix('/')?;
    Some(rest.trim_start_matches("./").to_string())
}

/// Strip `root` from `path` when `path` lies inside it.
///
/// Both sides are normalised (verbatim prefix, separators, Windows ASCII
/// case) before comparison, so a dunce path and a `\\?\` canonical path of
/// the same directory still strip. Returns `Some("")` when the two paths
/// name the same location, and `None` when `path` is not inside `root`.
#[must_use]
pub fn relative_to(path: &Path, root: &Path) -> Option<String> {
    let path_text = path.to_string_lossy();
    let root_text = root.to_string_lossy();
    let path = strip_verbatim_prefix(&path_text);
    let root = strip_verbatim_prefix(&root_text);
    strip_root(path.as_ref(), root.as_ref())
}

/// True when `left` and `right` name the same filesystem location for
/// identity checks, even if one side carries a Windows `\\?\` verbatim
/// prefix and the other does not.
#[must_use]
pub fn same_path(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    relative_to(left, right) == Some(String::new())
}

/// Length of the identity form, used to pick the longest matching worktree
/// prefix after stripping a leftover verbatim prefix.
#[must_use]
pub fn identity_len(path: &Path) -> usize {
    strip_verbatim_prefix(&path.to_string_lossy()).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_path_equates_plain_and_verbatim_windows_forms() {
        let plain = Path::new(r"C:\Users\runner\AppData\Local\Temp\proj");
        let verbatim = Path::new(r"\\?\C:\Users\runner\AppData\Local\Temp\proj");
        assert!(same_path(plain, verbatim));
        assert!(same_path(verbatim, plain));
        assert!(!same_path(
            plain,
            Path::new(r"C:\Users\runner\AppData\Local\Temp\other"),
        ));
    }

    #[test]
    fn same_path_equates_unc_verbatim_and_ordinary_unc() {
        let ordinary = Path::new(r"\\server\share\worktree");
        let verbatim = Path::new(r"\\?\UNC\server\share\worktree");
        assert!(same_path(ordinary, verbatim));
        assert_eq!(
            strip_verbatim_prefix(r"\\?\UNC\server\share\worktree"),
            r"\\server\share\worktree"
        );
    }

    #[test]
    fn relative_to_strips_verbatim_child_against_ordinary_root() {
        let root = Path::new(r"C:\Users\dev\project");
        let path = Path::new(r"\\?\C:\Users\dev\project\.anvil\architecture.yaml");
        assert_eq!(
            relative_to(path, root).as_deref(),
            Some(".anvil/architecture.yaml")
        );
    }

    #[test]
    fn strip_verbatim_leaves_unix_paths_untouched() {
        assert_eq!(
            strip_verbatim_prefix("/home/dev/project"),
            "/home/dev/project"
        );
        assert!(same_path(
            Path::new("/home/dev/project"),
            Path::new("/home/dev/project")
        ));
        assert!(!same_path(
            Path::new("/home/dev/project"),
            Path::new("/home/dev/other")
        ));
    }
}
