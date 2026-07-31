//! PYLAN-006: best-effort Python module string → workspace `.py` resolution.

use std::path::{Path, PathBuf};

use crate::util::relative_slash;

/// Resolve a Python import's module string to a workspace-relative `.py` file,
/// or `None` if it is external / unresolvable. `from_file` is workspace-relative
/// (forward-slash); `workspace_root` is the absolute repo root used only to
/// probe which candidate files exist.
#[must_use]
pub fn resolve_python_import(
    workspace_root: &Path,
    from_file: &str,
    module: &str,
) -> Option<String> {
    if module.is_empty() {
        return None;
    }
    // Defence in depth: `from_file` is expected workspace-relative. Refuse an
    // absolute or `..`-bearing path up front so no candidate is ever probed
    // outside `workspace_root` (the `relative_slash` strip_prefix would drop
    // the result anyway, but reject early rather than rely on the last layer —
    // matching `rust_resolve`'s `from_file.starts_with(src_root)` precheck).
    if !is_workspace_relative_slash_path(from_file) {
        return None;
    }
    if module.starts_with('.') {
        resolve_relative(workspace_root, from_file, module)
    } else {
        resolve_absolute(workspace_root, module)
    }
}

/// Resolve an absolute dotted module against the flat and `src/` package roots.
///
/// The flat candidate is tried before the `src/` one, deterministically. In the
/// rare repo that carries the *same* package both flat and under `src/` (e.g. a
/// stale top-level copy beside the real `src/` layout), an absolute import is
/// attributed to the flat file. Per the conservative posture this can only
/// mis-target an edge to the wrong in-workspace file, never escape the
/// workspace — and a genuinely ambiguous layout is itself the smell to fix.
fn resolve_absolute(workspace_root: &Path, module: &str) -> Option<String> {
    if !is_dotted_identifier(module) {
        return None;
    }
    let rel = module.replace('.', "/");
    existing_module_file(workspace_root, &PathBuf::from(&rel))
        .or_else(|| existing_module_file(workspace_root, &Path::new("src").join(&rel)))
}

/// Resolve a relative import (`.`, `.x`, `..pkg.sub`) against the importing
/// file's package.
fn resolve_relative(workspace_root: &Path, from_file: &str, module: &str) -> Option<String> {
    let dots = module.chars().take_while(|c| *c == '.').count();
    let remainder = &module[dots..];
    // The remainder, if present, must be a dotted identifier chain.
    if !remainder.is_empty() && !is_dotted_identifier(remainder) {
        return None;
    }

    // Dot 1 = the importing file's own package (its containing directory); each
    // further dot climbs one parent package.
    let mut base = Path::new(from_file).parent()?.to_path_buf();
    for _ in 0..dots.saturating_sub(1) {
        base = base.parent()?.to_path_buf();
    }

    let target = if remainder.is_empty() {
        base
    } else {
        base.join(remainder.replace('.', "/"))
    };

    // Climbing above the workspace root (a `..` component) is unresolvable.
    if target
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }
    existing_module_file(workspace_root, &target)
}

/// The workspace-relative path of `<rel>.py` or `<rel>/__init__.py`, whichever
/// exists on disk first, or `None`.
fn existing_module_file(workspace_root: &Path, rel: &Path) -> Option<String> {
    let as_file = rel.with_extension("py");
    if workspace_root.join(&as_file).is_file() {
        return relative_slash(workspace_root, &workspace_root.join(&as_file));
    }
    let as_pkg = rel.join("__init__.py");
    if workspace_root.join(&as_pkg).is_file() {
        return relative_slash(workspace_root, &workspace_root.join(&as_pkg));
    }
    None
}

fn is_workspace_relative_slash_path(path: &str) -> bool {
    if path.starts_with('/') || path.starts_with('\\') || has_windows_drive_prefix(path) {
        return false;
    }
    !Path::new(path)
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic()
}

/// Whether `s` is a non-empty `.`-separated chain of ASCII Python identifiers.
fn is_dotted_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.split('.').all(|seg| {
            let mut chars = seg.chars();
            match chars.next() {
                Some(c) if c == '_' || c.is_ascii_alphabetic() => {
                    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
                }
                _ => false,
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(root: &Path, rel: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"# fixture\n").unwrap();
    }

    #[test]
    fn absolute_flat_layout_module_and_package() {
        let tmp = tempfile::TempDir::new().unwrap();
        touch(tmp.path(), "app/main.py");
        touch(tmp.path(), "app/config.py");
        touch(tmp.path(), "app/db/__init__.py");

        // module-as-file
        assert_eq!(
            resolve_python_import(tmp.path(), "app/main.py", "app.config"),
            Some("app/config.py".to_string())
        );
        // module-as-package (__init__.py)
        assert_eq!(
            resolve_python_import(tmp.path(), "app/main.py", "app.db"),
            Some("app/db/__init__.py".to_string())
        );
    }

    #[test]
    fn absolute_src_layout() {
        let tmp = tempfile::TempDir::new().unwrap();
        touch(tmp.path(), "src/app/main.py");
        touch(tmp.path(), "src/app/config.py");

        assert_eq!(
            resolve_python_import(tmp.path(), "src/app/main.py", "app.config"),
            Some("src/app/config.py".to_string())
        );
    }

    #[test]
    fn absolute_prefers_flat_over_src_when_both_exist() {
        let tmp = tempfile::TempDir::new().unwrap();
        touch(tmp.path(), "src/mymod.py");
        // The same package exists both flat and under src/ — resolution is
        // deterministic (flat first), documented on `resolve_absolute`.
        touch(tmp.path(), "app/config.py");
        touch(tmp.path(), "src/app/config.py");

        assert_eq!(
            resolve_python_import(tmp.path(), "src/mymod.py", "app.config"),
            Some("app/config.py".to_string())
        );
    }

    #[test]
    fn external_module_is_dropped() {
        let tmp = tempfile::TempDir::new().unwrap();
        touch(tmp.path(), "app/main.py");
        // stdlib / third-party — exists nowhere in the tree.
        assert_eq!(resolve_python_import(tmp.path(), "app/main.py", "os"), None);
        assert_eq!(
            resolve_python_import(tmp.path(), "app/main.py", "numpy.linalg"),
            None
        );
    }

    #[test]
    fn relative_single_dot_with_remainder() {
        let tmp = tempfile::TempDir::new().unwrap();
        touch(tmp.path(), "app/handlers/user.py");
        touch(tmp.path(), "app/handlers/base.py");

        // `from .base import X` in app/handlers/user.py → app/handlers/base.py
        assert_eq!(
            resolve_python_import(tmp.path(), "app/handlers/user.py", ".base"),
            Some("app/handlers/base.py".to_string())
        );
    }

    #[test]
    fn relative_bare_dot_is_current_package_init() {
        let tmp = tempfile::TempDir::new().unwrap();
        touch(tmp.path(), "app/handlers/user.py");
        touch(tmp.path(), "app/handlers/__init__.py");

        // `from . import x` → the current package's __init__.py
        assert_eq!(
            resolve_python_import(tmp.path(), "app/handlers/user.py", "."),
            Some("app/handlers/__init__.py".to_string())
        );
    }

    #[test]
    fn relative_double_dot_parent_package() {
        let tmp = tempfile::TempDir::new().unwrap();
        touch(tmp.path(), "app/handlers/user.py");
        touch(tmp.path(), "app/models.py");
        touch(tmp.path(), "app/util/__init__.py");

        // `from ..models import M` in app/handlers/user.py → app/models.py
        assert_eq!(
            resolve_python_import(tmp.path(), "app/handlers/user.py", "..models"),
            Some("app/models.py".to_string())
        );
        // `from ..util import U` → app/util/__init__.py
        assert_eq!(
            resolve_python_import(tmp.path(), "app/handlers/user.py", "..util"),
            Some("app/util/__init__.py".to_string())
        );
    }

    #[test]
    fn relative_climb_above_root_is_none() {
        let tmp = tempfile::TempDir::new().unwrap();
        touch(tmp.path(), "a.py");
        touch(tmp.path(), "b.py");
        // `a.py` is at the root; `..b` climbs above the workspace → unresolvable.
        assert_eq!(resolve_python_import(tmp.path(), "a.py", "..b"), None);
    }

    #[test]
    fn star_import_module_resolves_like_any_other() {
        let tmp = tempfile::TempDir::new().unwrap();
        touch(tmp.path(), "app/main.py");
        touch(tmp.path(), "app/api.py");
        // `from app.api import *` records module `app.api`.
        assert_eq!(
            resolve_python_import(tmp.path(), "app/main.py", "app.api"),
            Some("app/api.py".to_string())
        );
    }

    #[test]
    fn absolute_or_parent_bearing_from_file_is_refused() {
        // Defence-in-depth: a malformed `from_file` must never probe outside
        // the workspace, regardless of the module form.
        let tmp = tempfile::TempDir::new().unwrap();
        touch(tmp.path(), "app/x.py");
        assert_eq!(
            resolve_python_import(tmp.path(), "/etc/passwd", "app.x"),
            None
        );
        assert_eq!(
            resolve_python_import(tmp.path(), "\\etc\\passwd", "app.x"),
            None
        );
        assert_eq!(
            resolve_python_import(tmp.path(), "C:\\outside\\mod.py", "app.x"),
            None
        );
        assert_eq!(
            resolve_python_import(tmp.path(), "../outside/mod.py", ".sibling"),
            None
        );
    }

    #[test]
    fn malformed_module_is_dropped() {
        let tmp = tempfile::TempDir::new().unwrap();
        touch(tmp.path(), "app/main.py");
        touch(tmp.path(), "app/x.py");
        // Non-identifier segments never reach the filesystem.
        assert_eq!(
            resolve_python_import(tmp.path(), "app/main.py", "app..x"),
            None
        );
        assert_eq!(
            resolve_python_import(tmp.path(), "app/main.py", "9bad"),
            None
        );
        assert_eq!(resolve_python_import(tmp.path(), "app/main.py", ""), None);
    }

    #[test]
    fn prefers_module_file_over_package_at_same_name() {
        let tmp = tempfile::TempDir::new().unwrap();
        touch(tmp.path(), "app/main.py");
        // Both app/svc.py and app/svc/__init__.py exist; the .py file wins
        // (Python would actually error on this, but resolution must be
        // deterministic — file before package, documented).
        touch(tmp.path(), "app/svc.py");
        touch(tmp.path(), "app/svc/__init__.py");
        assert_eq!(
            resolve_python_import(tmp.path(), "app/main.py", "app.svc"),
            Some("app/svc.py".to_string())
        );
    }
}
