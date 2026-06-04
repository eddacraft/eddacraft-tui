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

    // Only `crate`/`self`/`super` reference this workspace; `std`, external
    // crates and bare identifiers are out.
    if !matches!(*anchor, "crate" | "self" | "super") {
        return None;
    }

    let src_root = crate_src_root(workspace_root, from_file)?;
    // Intra-crate module resolution is only sound when the importing file lives
    // inside the crate's `src/` module tree. A file outside it — `benches/`,
    // `tests/`, `examples/`, a loose script, or a bare filename — is its own
    // compilation unit (or not part of the tree at all), so `module_dir_of` /
    // the `super` walk would be meaningless. Skip conservatively rather than
    // risk resolving against the wrong tree (e.g. a workspace-root `src/`).
    if !Path::new(from_file).starts_with(&src_root) {
        return None;
    }

    match *anchor {
        "crate" => resolve_module_file(workspace_root, &src_root, rest),
        "self" => {
            let base = module_dir_of(from_file)?;
            resolve_module_file(workspace_root, &base, rest)
        }
        "super" => {
            // `supers` counts the whole leading run including the anchor, so the
            // tail to resolve is `rest[(supers - 1)..]` (rest already excludes
            // the anchor).
            let supers = 1 + rest.iter().take_while(|s| **s == "super").count();
            let tail = &rest[supers - 1..];
            let mut base = module_dir_of(from_file)?;
            for _ in 0..supers {
                // The parent module's children live one directory up; never
                // climb above the crate `src/` root (guaranteed terminating
                // because `from_file` is under `src_root`).
                if base == src_root {
                    return None;
                }
                base = base.parent()?.to_path_buf();
            }
            resolve_module_file(workspace_root, &base, tail)
        }
        _ => unreachable!("anchor pre-filtered above"),
    }
}

/// The `src/` directory of the crate that owns `from_file`: walk up `from_file`'s
/// ancestors to the nearest directory containing a `Cargo.toml`, then append
/// `src`. Workspace-relative.
fn crate_src_root(workspace_root: &Path, from_file: &str) -> Option<PathBuf> {
    // The candidate is only accepted if `<crate>/src` actually exists, so a
    // crate with no `src/` (or a non-Rust tree) yields `None` rather than a
    // phantom root.
    let accept = |src: PathBuf| workspace_root.join(&src).is_dir().then_some(src);
    let mut dir = Path::new(from_file).parent();
    while let Some(d) = dir {
        if workspace_root.join(d).join("Cargo.toml").is_file() {
            return accept(d.join("src"));
        }
        dir = d.parent();
    }
    // Single-crate repo with the manifest at the root (`from_file = src/…`).
    if workspace_root.join("Cargo.toml").is_file() {
        return accept(PathBuf::from("src"));
    }
    None
}

/// The directory in which `from_file`'s **child** modules live: the same
/// directory for a crate root (`lib.rs`/`main.rs`) or a `mod.rs`, else a
/// directory named after the file stem (`a/b.rs` → `a/b/`).
fn module_dir_of(from_file: &str) -> Option<PathBuf> {
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
    let has_parent = path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir));
    // A `..` here would mean an upstream bug (resolution should never produce
    // one); fail loud under test, degrade to a dropped edge in release.
    debug_assert!(
        !has_parent,
        "resolved path escaped via `..`: {}",
        path.display()
    );
    if has_parent {
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
    fn import_from_outside_src_is_not_resolved() {
        let tmp = tempfile::TempDir::new().unwrap();
        single_crate(tmp.path());
        // A bench/test/script file lives outside the crate `src/` tree; its
        // intra-crate paths are skipped conservatively (no resolving against
        // a workspace-root `src/`).
        touch(tmp.path(), "benches/bench.rs");
        touch(tmp.path(), "scripts/migrate.rs");
        assert_eq!(
            resolve_rust_import(tmp.path(), "benches/bench.rs", "crate::config::Settings"),
            None
        );
        assert_eq!(
            resolve_rust_import(tmp.path(), "scripts/migrate.rs", "super::config"),
            None
        );
    }

    #[test]
    fn bare_filename_from_file_is_not_resolved() {
        let tmp = tempfile::TempDir::new().unwrap();
        single_crate(tmp.path());
        // A bare `lib.rs` (no directory) is not under `src/` → conservative skip,
        // never a climb above the root.
        assert_eq!(
            resolve_rust_import(tmp.path(), "lib.rs", "super::whatever"),
            None
        );
        assert_eq!(
            resolve_rust_import(tmp.path(), "lib.rs", "crate::config"),
            None
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
