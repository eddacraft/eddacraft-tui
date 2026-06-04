//! Best-effort Rust module-path → file resolution for boundary analysis
//! (RSTLAN-005).
//!
//! The kernel Rust extractor (RSTLAN-002) emits import edges whose target is a
//! module *path* string — `crate::config::Settings`, `super::sibling::thing`,
//! `self::local::Item`, `std::collections::HashMap`. The architecture validator
//! matches edges to layers by *file* path. This module bridges the two: it maps
//! a Rust module path, relative to the importing file, to the workspace-relative
//! `.rs` file that defines the referenced module.
//!
//! It is **conservative by construction**: when a path can't be resolved to a
//! file on disk (an external crate like `std`/`serde`, a macro-generated module,
//! a `#[path]` redirect, or a crate-root symbol) it returns `None` and the edge
//! is simply dropped. A missed edge is a missed drift signal, never a false
//! boundary violation — matching Anvil's "warnings over blocks, new edges only"
//! posture. `#[path]`, re-exports beyond the file target, and proc-macro modules
//! are out of scope (documented in the RSTLAN module).
//!
//! Resolution rules:
//! - `crate::a::b::Item` → the importing file's owning crate `src/` root, then
//!   `a/b.rs` or `a/b/mod.rs` (trailing symbol `Item` dropped).
//! - `self::a::Item` → the importing file's own module directory, then `a`.
//! - `super::a` (and repeated `super::super::…`) → walk up that many parent
//!   module directories, then `a`, never escaping the crate `src/` root.
//! - anything else (`std`, an external crate, a bare identifier) → `None`.

use std::path::{Path, PathBuf};

/// Resolve a Rust `use` module path to a workspace-relative `.rs` file, or
/// `None` if it is external / unresolvable. `from_file` is workspace-relative
/// (forward-slash); `workspace_root` is the absolute repo root used only to
/// probe which candidate files exist.
#[must_use]
pub fn resolve_rust_import(
    workspace_root: &Path,
    from_file: &str,
    module_path: &str,
) -> Option<String> {
    let segments: Vec<&str> = module_path.split("::").filter(|s| !s.is_empty()).collect();
    let (anchor, rest) = segments.split_first()?;

    match *anchor {
        "crate" => {
            let base = crate_src_root(workspace_root, from_file)?;
            resolve_module_file(workspace_root, &base, rest)
        }
        "self" => {
            let base = module_dir_of(workspace_root, from_file)?;
            resolve_module_file(workspace_root, &base, rest)
        }
        "super" => {
            // Count the leading run of `super` (`super::super::x`).
            let supers = 1 + rest.iter().take_while(|s| **s == "super").count();
            let tail = &segments[supers..];
            let src_root = crate_src_root(workspace_root, from_file)?;
            let mut base = module_dir_of(workspace_root, from_file)?;
            for _ in 0..supers {
                // The parent module's children live one directory up; never
                // climb above the crate `src/` root.
                if base == src_root {
                    return None;
                }
                base = base.parent()?.to_path_buf();
            }
            resolve_module_file(workspace_root, &base, tail)
        }
        // `std`, an external crate, or a bare identifier — not in this workspace.
        _ => None,
    }
}

/// The `src/` directory of the crate that owns `from_file`: walk up `from_file`'s
/// ancestors to the nearest directory containing a `Cargo.toml`, then append
/// `src`. Workspace-relative.
fn crate_src_root(workspace_root: &Path, from_file: &str) -> Option<PathBuf> {
    let mut dir = Path::new(from_file).parent();
    while let Some(d) = dir {
        if workspace_root.join(d).join("Cargo.toml").is_file() {
            return Some(d.join("src"));
        }
        dir = d.parent();
    }
    // Single-crate repo with the manifest at the root (`from_file = src/…`).
    if workspace_root.join("Cargo.toml").is_file() {
        return Some(PathBuf::from("src"));
    }
    None
}

/// The directory in which `from_file`'s **child** modules live: the same
/// directory for a crate root (`lib.rs`/`main.rs`) or a `mod.rs`, else a
/// directory named after the file stem (`a/b.rs` → `a/b/`).
fn module_dir_of(_workspace_root: &Path, from_file: &str) -> Option<PathBuf> {
    let path = Path::new(from_file);
    let parent = path.parent()?;
    let stem = path.file_stem()?.to_str()?;
    if matches!(stem, "lib" | "main" | "mod") {
        Some(parent.to_path_buf())
    } else {
        Some(parent.join(stem))
    }
}

/// Find the file defining the module reached by `segments` under `base_dir`,
/// dropping trailing segments that turn out to be symbols rather than modules
/// (`config::Settings` → `config.rs`). Returns a workspace-relative,
/// forward-slash path, or `None` when nothing on disk matches.
fn resolve_module_file(
    workspace_root: &Path,
    base_dir: &Path,
    segments: &[&str],
) -> Option<String> {
    let mut segs = segments;
    while let Some((last, parents)) = segs.split_last() {
        let mut dir = base_dir.to_path_buf();
        for p in parents {
            dir.push(p);
        }
        let as_file = dir.join(format!("{last}.rs"));
        if workspace_root.join(&as_file).is_file() {
            return rel_slash(&as_file);
        }
        let as_mod = dir.join(last).join("mod.rs");
        if workspace_root.join(&as_mod).is_file() {
            return rel_slash(&as_mod);
        }
        // `last` was a symbol (type/fn/const), not a module file — drop it.
        segs = parents;
    }
    None
}

/// Render an already-workspace-relative path with forward slashes, refusing any
/// `..` component (defence in depth — module resolution should never produce one).
fn rel_slash(path: &Path) -> Option<String> {
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }
    Some(path.to_str()?.replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(root: &Path, rel: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, "// fixture\n").unwrap();
    }

    /// Single-crate repo fixture: manifest at root, sources under `src/`.
    fn single_crate(root: &Path) {
        touch(root, "Cargo.toml");
        touch(root, "src/lib.rs");
        touch(root, "src/config.rs");
        touch(root, "src/handlers/mod.rs");
        touch(root, "src/handlers/user.rs");
        touch(root, "src/handlers/admin.rs");
        touch(root, "src/db/pool.rs");
    }

    #[test]
    fn crate_path_drops_trailing_symbol_to_module_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        single_crate(tmp.path());
        // `Settings` is a type in module `config` → resolves to src/config.rs.
        assert_eq!(
            resolve_rust_import(
                tmp.path(),
                "src/handlers/user.rs",
                "crate::config::Settings"
            ),
            Some("src/config.rs".to_string())
        );
    }

    #[test]
    fn crate_path_to_mod_rs() {
        let tmp = tempfile::TempDir::new().unwrap();
        single_crate(tmp.path());
        // `crate::handlers::user` → the module file; but `handlers` itself is a
        // mod.rs directory module.
        assert_eq!(
            resolve_rust_import(tmp.path(), "src/db/pool.rs", "crate::handlers"),
            Some("src/handlers/mod.rs".to_string())
        );
    }

    #[test]
    fn crate_nested_module_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        single_crate(tmp.path());
        assert_eq!(
            resolve_rust_import(tmp.path(), "src/lib.rs", "crate::handlers::user::Handler"),
            Some("src/handlers/user.rs".to_string())
        );
    }

    #[test]
    fn self_path_resolves_in_own_module_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        single_crate(tmp.path());
        // From handlers/mod.rs, `self::user` is src/handlers/user.rs.
        assert_eq!(
            resolve_rust_import(tmp.path(), "src/handlers/mod.rs", "self::user::User"),
            Some("src/handlers/user.rs".to_string())
        );
    }

    #[test]
    fn super_path_resolves_sibling_module() {
        let tmp = tempfile::TempDir::new().unwrap();
        single_crate(tmp.path());
        // From handlers/user.rs, `super::admin` is the sibling src/handlers/admin.rs.
        assert_eq!(
            resolve_rust_import(tmp.path(), "src/handlers/user.rs", "super::admin::Admin"),
            Some("src/handlers/admin.rs".to_string())
        );
    }

    #[test]
    fn super_from_mod_rs_reaches_crate_root_siblings() {
        let tmp = tempfile::TempDir::new().unwrap();
        single_crate(tmp.path());
        // From handlers/mod.rs (the `handlers` module), `super::config` is the
        // crate-root sibling src/config.rs.
        assert_eq!(
            resolve_rust_import(tmp.path(), "src/handlers/mod.rs", "super::config::Settings"),
            Some("src/config.rs".to_string())
        );
    }

    #[test]
    fn super_does_not_escape_crate_root() {
        let tmp = tempfile::TempDir::new().unwrap();
        single_crate(tmp.path());
        // lib.rs is the crate root — it has no parent module; `super` is invalid.
        assert_eq!(
            resolve_rust_import(tmp.path(), "src/lib.rs", "super::whatever"),
            None
        );
    }

    #[test]
    fn external_crate_paths_are_unresolved() {
        let tmp = tempfile::TempDir::new().unwrap();
        single_crate(tmp.path());
        for ext in [
            "std::collections::HashMap",
            "serde::Deserialize",
            "anyhow::Result",
            "tokio::spawn",
        ] {
            assert_eq!(
                resolve_rust_import(tmp.path(), "src/lib.rs", ext),
                None,
                "external path {ext} must not resolve"
            );
        }
    }

    #[test]
    fn unresolvable_in_workspace_path_is_none_not_a_guess() {
        let tmp = tempfile::TempDir::new().unwrap();
        single_crate(tmp.path());
        // `crate::nonexistent::Thing` has no file — conservative skip.
        assert_eq!(
            resolve_rust_import(tmp.path(), "src/lib.rs", "crate::nonexistent::Thing"),
            None
        );
    }

    #[test]
    fn crate_path_resolves_per_owning_crate_in_a_workspace() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Two crates; `crate::` in api must resolve to api/src, not core/src.
        touch(tmp.path(), "Cargo.toml"); // virtual workspace root
        touch(tmp.path(), "crates/api/Cargo.toml");
        touch(tmp.path(), "crates/api/src/lib.rs");
        touch(tmp.path(), "crates/api/src/routes.rs");
        touch(tmp.path(), "crates/core/Cargo.toml");
        touch(tmp.path(), "crates/core/src/lib.rs");
        touch(tmp.path(), "crates/core/src/routes.rs");

        assert_eq!(
            resolve_rust_import(tmp.path(), "crates/api/src/lib.rs", "crate::routes::Router"),
            Some("crates/api/src/routes.rs".to_string()),
            "crate:: must resolve within the importing file's own crate"
        );
    }

    #[test]
    fn double_super_climbs_two_levels() {
        let tmp = tempfile::TempDir::new().unwrap();
        touch(tmp.path(), "Cargo.toml");
        touch(tmp.path(), "src/lib.rs");
        touch(tmp.path(), "src/a/mod.rs");
        touch(tmp.path(), "src/a/shared.rs");
        touch(tmp.path(), "src/a/b/mod.rs");
        touch(tmp.path(), "src/a/b/c.rs");
        touch(tmp.path(), "src/top.rs");
        // src/a/b/c.rs is module crate::a::b::c. `super` = b, `super::super` = a,
        // so super::super::shared is crate::a::shared = src/a/shared.rs.
        assert_eq!(
            resolve_rust_import(tmp.path(), "src/a/b/c.rs", "super::super::shared::S"),
            Some("src/a/shared.rs".to_string())
        );
        // Three supers reaches the crate root: crate::top = src/top.rs.
        assert_eq!(
            resolve_rust_import(tmp.path(), "src/a/b/c.rs", "super::super::super::top::T"),
            Some("src/top.rs".to_string())
        );
    }

    #[test]
    fn determinism_same_input_same_output() {
        let tmp = tempfile::TempDir::new().unwrap();
        single_crate(tmp.path());
        let a = resolve_rust_import(
            tmp.path(),
            "src/handlers/user.rs",
            "crate::config::Settings",
        );
        let b = resolve_rust_import(
            tmp.path(),
            "src/handlers/user.rs",
            "crate::config::Settings",
        );
        assert_eq!(a, b);
    }
}
