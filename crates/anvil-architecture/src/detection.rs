//! Rust entry-point detection (RSTLAN-004).
//!
//! Surfaces the roots of a Rust workspace — binary `src/main.rs` / `[[bin]]`
//! targets (including `src/bin/*.rs` auto-discovery), library roots, and
//! explicit `[[example]]` binaries across every workspace member — as
//! [`EntryPoint`]s, so baseline creation and the `anvil architecture` surfaces
//! treat a pure-Rust or mixed repo's Rust roots the way they already treat a TS
//! package's `bin` / `main`.
//!
//! Detection is `Cargo.toml`- and filesystem-driven and deterministic (members
//! and targets are sorted, output is de-duplicated by path). It parses no Rust
//! source — that is the kernel extractor's job (RSTLAN-002) — and resolves no
//! imports (RSTLAN-005). A missing or unparseable root `Cargo.toml` yields an
//! empty list: a non-Rust or virtual-only tree is not an error.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::types::{DetectionConfidence, EntryPoint, EntryPointType};

/// Detect Rust entry points under `workspace_root`.
///
/// Reads the root `Cargo.toml`. If it declares a `[workspace]`, every member
/// (glob patterns expanded) is scanned, plus the root itself when it also
/// carries `[package]` (a non-virtual workspace or a single-crate repo).
/// Returns entry points with workspace-root-relative, forward-slash paths,
/// sorted and de-duplicated by path for determinism.
#[must_use]
pub fn detect_rust_entry_points(workspace_root: &Path) -> Vec<EntryPoint> {
    let Some(root_manifest) = read_manifest(&workspace_root.join("Cargo.toml")) else {
        return Vec::new();
    };

    let mut crate_dirs: Vec<PathBuf> = Vec::new();
    if let Some(ws) = &root_manifest.workspace {
        for member_glob in &ws.members {
            crate_dirs.extend(expand_member_glob(workspace_root, member_glob));
        }
        if !ws.exclude.is_empty() {
            crate_dirs.retain(|dir| !is_excluded(workspace_root, dir, &ws.exclude));
        }
    }
    // The root is itself a crate when it carries `[package]` (a non-virtual
    // workspace, or a single-crate repo with no `[workspace]` table at all).
    if root_manifest.package.is_some() {
        crate_dirs.push(workspace_root.to_path_buf());
    }
    crate_dirs.sort();
    crate_dirs.dedup();

    let mut entries: Vec<EntryPoint> = Vec::new();
    for crate_dir in &crate_dirs {
        collect_crate_entry_points(workspace_root, crate_dir, &mut entries);
    }

    // Deterministic order + one entry per path (an explicit `[[bin]]` pointing at
    // `src/main.rs` and the implicit default bin collapse to one).
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries.dedup_by(|a, b| a.path == b.path);
    entries
}

/// Append the entry points of a single crate at `crate_dir` to `out`.
fn collect_crate_entry_points(workspace_root: &Path, crate_dir: &Path, out: &mut Vec<EntryPoint>) {
    let Some(manifest) = read_manifest(&crate_dir.join("Cargo.toml")) else {
        return;
    };

    let push = |out: &mut Vec<EntryPoint>, abs: PathBuf, ty: EntryPointType, conf| {
        if abs.is_file()
            && let Some(rel) = relative_slash(workspace_root, &abs)
        {
            out.push(EntryPoint {
                path: rel,
                entry_type: ty,
                confidence: conf,
                exports: None,
            });
        }
    };

    // Library root: explicit `[lib].path` or the conventional `src/lib.rs`.
    let lib_path = manifest
        .lib
        .as_ref()
        .and_then(|t| t.path.clone())
        .unwrap_or_else(|| "src/lib.rs".to_string());
    push(
        out,
        crate_dir.join(&lib_path),
        EntryPointType::Package,
        DetectionConfidence::High,
    );

    // Explicit `[[bin]]` targets, resolved per Cargo's rules: an explicit
    // `path` wins; otherwise the name (its own, or the package name when the
    // table omits it) maps to `src/main.rs` when it equals the package name,
    // else `src/bin/<name>.rs`.
    let pkg_name = manifest.package.as_ref().and_then(|p| p.name.clone());
    for bin in &manifest.bin {
        let rel = if let Some(path) = bin.path.clone() {
            Some(path)
        } else {
            bin.name.clone().or_else(|| pkg_name.clone()).map(|name| {
                if Some(name.as_str()) == pkg_name.as_deref() {
                    "src/main.rs".to_string()
                } else {
                    format!("src/bin/{name}.rs")
                }
            })
        };
        if let Some(rel) = rel {
            push(
                out,
                crate_dir.join(rel),
                EntryPointType::Application,
                DetectionConfidence::High,
            );
        }
    }

    // Implicit default binary: `src/main.rs` is a bin target even with no
    // `[[bin]]` table (dedup collapses it with an explicit one that points here).
    push(
        out,
        crate_dir.join("src/main.rs"),
        EntryPointType::Application,
        DetectionConfidence::High,
    );

    // Auto-discovered binaries: every `src/bin/*.rs` is a Cargo binary target,
    // unless the package opts out with `autobins = false` (then `src/bin/` holds
    // helper modules, not targets).
    let autobins = manifest.package.as_ref().is_none_or(|p| p.autobins);
    if autobins && let Ok(read_dir) = std::fs::read_dir(crate_dir.join("src").join("bin")) {
        let mut bin_files: Vec<PathBuf> = read_dir
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "rs"))
            .collect();
        bin_files.sort();
        for abs in bin_files {
            push(
                out,
                abs,
                EntryPointType::Application,
                DetectionConfidence::High,
            );
        }
    }

    // Explicit `[[example]]` binaries: peripheral roots, medium confidence.
    for example in &manifest.example {
        let rel = example.path.clone().or_else(|| {
            example
                .name
                .as_ref()
                .map(|name| format!("examples/{name}.rs"))
        });
        if let Some(rel) = rel {
            push(
                out,
                crate_dir.join(rel),
                EntryPointType::Application,
                DetectionConfidence::Medium,
            );
        }
    }
}

/// Expand a `[workspace].members` entry (literal or glob, e.g. `crates/*`) to the
/// member directories that contain a `Cargo.toml`.
fn expand_member_glob(workspace_root: &Path, member: &str) -> Vec<PathBuf> {
    // Cargo never uses a recursive `**` in workspace members; refuse it so a
    // pathological pattern can't trigger a full-tree walk (incl. `target/`).
    if member.contains("**") {
        return Vec::new();
    }
    let pattern = workspace_root.join(member);
    let Some(pattern) = pattern.to_str() else {
        return Vec::new();
    };
    let Ok(paths) = glob::glob(pattern) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = paths
        .flatten()
        .filter(|p| p.join("Cargo.toml").is_file())
        .collect();
    dirs.sort();
    dirs
}

/// Whether a member directory is dropped by `[workspace].exclude` (literal
/// root-relative path or a glob pattern).
fn is_excluded(root: &Path, dir: &Path, exclude: &[String]) -> bool {
    let Ok(rel) = dir.strip_prefix(root) else {
        return false;
    };
    let rel = rel.to_string_lossy().replace('\\', "/");
    exclude.iter().any(|ex| {
        let ex = ex.trim_end_matches('/');
        rel == ex || glob::Pattern::new(ex).is_ok_and(|p| p.matches(&rel))
    })
}

/// Workspace-root-relative path with forward slashes, or `None` if `abs` is not
/// under `root`, escapes it through an un-normalised `..` component, or is not
/// valid UTF-8. The `..` guard stops a `..`-bearing `members`/`[[bin]] path`
/// from persisting a path that points outside the workspace.
fn relative_slash(root: &Path, abs: &Path) -> Option<String> {
    let rel = abs.strip_prefix(root).ok()?;
    if rel
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }
    Some(rel.to_str()?.replace('\\', "/"))
}

fn read_manifest(path: &Path) -> Option<Manifest> {
    let text = std::fs::read_to_string(path).ok()?;
    toml::from_str(&text).ok()
}

#[derive(Deserialize)]
struct Manifest {
    package: Option<PackageSection>,
    workspace: Option<WorkspaceSection>,
    lib: Option<TargetSection>,
    #[serde(default)]
    bin: Vec<TargetSection>,
    #[serde(default)]
    example: Vec<TargetSection>,
}

#[derive(Deserialize)]
struct PackageSection {
    name: Option<String>,
    /// Cargo's `[package] autobins` — when `false`, `src/bin/*.rs` are not
    /// auto-discovered binary targets. Defaults to `true`.
    #[serde(default = "default_true")]
    autobins: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
struct WorkspaceSection {
    #[serde(default)]
    members: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

#[derive(Deserialize)]
struct TargetSection {
    name: Option<String>,
    path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, rel: &str, body: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    fn paths(entries: &[EntryPoint]) -> Vec<&str> {
        entries.iter().map(|e| e.path.as_str()).collect()
    }

    #[test]
    fn single_package_binary_main() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"app\"\n");
        write(tmp.path(), "src/main.rs", "fn main() {}\n");

        let entries = detect_rust_entry_points(tmp.path());
        assert_eq!(paths(&entries), ["src/main.rs"]);
        assert_eq!(entries[0].entry_type, EntryPointType::Application);
        assert_eq!(entries[0].confidence, DetectionConfidence::High);
    }

    #[test]
    fn single_package_library_root() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"lib\"\n");
        write(tmp.path(), "src/lib.rs", "pub fn f() {}\n");

        let entries = detect_rust_entry_points(tmp.path());
        assert_eq!(paths(&entries), ["src/lib.rs"]);
        assert_eq!(entries[0].entry_type, EntryPointType::Package);
    }

    #[test]
    fn explicit_bin_and_default_main_dedup_by_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        // An explicit [[bin]] pointing at src/main.rs must not double-count with
        // the implicit default bin.
        write(
            tmp.path(),
            "Cargo.toml",
            "[package]\nname = \"app\"\n\n[[bin]]\nname = \"app\"\npath = \"src/main.rs\"\n",
        );
        write(tmp.path(), "src/main.rs", "fn main() {}\n");

        let entries = detect_rust_entry_points(tmp.path());
        assert_eq!(paths(&entries), ["src/main.rs"], "deduped by path");
    }

    #[test]
    fn auto_discovers_src_bin_targets() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"tools\"\n");
        write(tmp.path(), "src/lib.rs", "pub fn f() {}\n");
        write(tmp.path(), "src/bin/one.rs", "fn main() {}\n");
        write(tmp.path(), "src/bin/two.rs", "fn main() {}\n");

        let entries = detect_rust_entry_points(tmp.path());
        assert_eq!(
            paths(&entries),
            ["src/bin/one.rs", "src/bin/two.rs", "src/lib.rs"]
        );
    }

    #[test]
    fn workspace_members_and_globs() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/*\"]\n",
        );
        write(
            tmp.path(),
            "crates/api/Cargo.toml",
            "[package]\nname = \"api\"\n",
        );
        write(tmp.path(), "crates/api/src/main.rs", "fn main() {}\n");
        write(
            tmp.path(),
            "crates/core/Cargo.toml",
            "[package]\nname = \"core\"\n",
        );
        write(tmp.path(), "crates/core/src/lib.rs", "pub fn f() {}\n");

        let entries = detect_rust_entry_points(tmp.path());
        assert_eq!(
            paths(&entries),
            ["crates/api/src/main.rs", "crates/core/src/lib.rs"]
        );
    }

    #[test]
    fn non_virtual_workspace_includes_root_package() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "Cargo.toml",
            "[package]\nname = \"root\"\n\n[workspace]\nmembers = [\"crates/sub\"]\n",
        );
        write(tmp.path(), "src/lib.rs", "pub fn f() {}\n");
        write(
            tmp.path(),
            "crates/sub/Cargo.toml",
            "[package]\nname = \"sub\"\n",
        );
        write(tmp.path(), "crates/sub/src/main.rs", "fn main() {}\n");

        let entries = detect_rust_entry_points(tmp.path());
        assert_eq!(paths(&entries), ["crates/sub/src/main.rs", "src/lib.rs"]);
    }

    #[test]
    fn explicit_example_is_medium_confidence() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "Cargo.toml",
            "[package]\nname = \"demo\"\n\n[[example]]\nname = \"hello\"\n",
        );
        write(tmp.path(), "src/lib.rs", "pub fn f() {}\n");
        write(tmp.path(), "examples/hello.rs", "fn main() {}\n");

        let entries = detect_rust_entry_points(tmp.path());
        assert_eq!(paths(&entries), ["examples/hello.rs", "src/lib.rs"]);
        let example = entries
            .iter()
            .find(|e| e.path == "examples/hello.rs")
            .unwrap();
        assert_eq!(example.confidence, DetectionConfidence::Medium);
    }

    #[test]
    fn workspace_exclude_is_respected() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"crates/generated\"]\n",
        );
        write(
            tmp.path(),
            "crates/keep/Cargo.toml",
            "[package]\nname = \"keep\"\n",
        );
        write(tmp.path(), "crates/keep/src/lib.rs", "pub fn f() {}\n");
        write(
            tmp.path(),
            "crates/generated/Cargo.toml",
            "[package]\nname = \"generated\"\n",
        );
        write(tmp.path(), "crates/generated/src/main.rs", "fn main() {}\n");

        let entries = detect_rust_entry_points(tmp.path());
        assert_eq!(paths(&entries), ["crates/keep/src/lib.rs"]);
    }

    #[test]
    fn parent_dir_member_does_not_escape_the_workspace() {
        let tmp = tempfile::TempDir::new().unwrap();
        // A sibling crate referenced via `..` must never produce a `../…` path
        // in the output (it would escape the workspace root in the baseline).
        write(
            tmp.path(),
            "root/Cargo.toml",
            "[workspace]\nmembers = [\"../sibling\"]\n",
        );
        write(
            tmp.path(),
            "sibling/Cargo.toml",
            "[package]\nname = \"sibling\"\n",
        );
        write(tmp.path(), "sibling/src/main.rs", "fn main() {}\n");

        let entries = detect_rust_entry_points(&tmp.path().join("root"));
        assert!(
            entries.iter().all(|e| !e.path.contains("..")),
            "no entry path may contain `..`, got {:?}",
            paths(&entries)
        );
    }

    #[test]
    fn autobins_false_suppresses_src_bin_discovery() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "Cargo.toml",
            "[package]\nname = \"lib\"\nautobins = false\n",
        );
        write(tmp.path(), "src/lib.rs", "pub fn f() {}\n");
        write(tmp.path(), "src/bin/helper.rs", "fn main() {}\n");

        let entries = detect_rust_entry_points(tmp.path());
        assert_eq!(
            paths(&entries),
            ["src/lib.rs"],
            "autobins=false: src/bin/* are not targets"
        );
    }

    #[test]
    fn unnamed_bin_falls_back_to_package_name_main() {
        let tmp = tempfile::TempDir::new().unwrap();
        // `[[bin]]` with neither name nor path → Cargo infers package name →
        // src/main.rs (collapses with the implicit default bin).
        write(
            tmp.path(),
            "Cargo.toml",
            "[package]\nname = \"app\"\n\n[[bin]]\n",
        );
        write(tmp.path(), "src/main.rs", "fn main() {}\n");

        let entries = detect_rust_entry_points(tmp.path());
        assert_eq!(paths(&entries), ["src/main.rs"]);
    }

    #[test]
    fn recursive_glob_member_is_refused() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "Cargo.toml",
            "[workspace]\nmembers = [\"**\"]\n",
        );
        write(
            tmp.path(),
            "crates/a/Cargo.toml",
            "[package]\nname = \"a\"\n",
        );
        write(tmp.path(), "crates/a/src/main.rs", "fn main() {}\n");
        // `**` is refused (no full-tree walk); nothing is discovered through it.
        assert!(detect_rust_entry_points(tmp.path()).is_empty());
    }

    #[test]
    fn missing_manifest_is_empty_not_an_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), "src/main.rs", "fn main() {}\n");
        assert!(detect_rust_entry_points(tmp.path()).is_empty());
    }

    #[test]
    fn virtual_workspace_emits_no_root_entry() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Virtual manifest: [workspace] but no [package] and no members on disk.
        write(
            tmp.path(),
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/*\"]\n",
        );
        write(tmp.path(), "src/main.rs", "fn main() {}\n");
        assert!(
            detect_rust_entry_points(tmp.path()).is_empty(),
            "root src/main.rs is not an entry point of a virtual workspace"
        );
    }

    #[test]
    fn deterministic_across_runs() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"app\"\n");
        write(tmp.path(), "src/main.rs", "fn main() {}\n");
        write(tmp.path(), "src/lib.rs", "pub fn f() {}\n");
        write(tmp.path(), "src/bin/z.rs", "fn main() {}\n");
        write(tmp.path(), "src/bin/a.rs", "fn main() {}\n");

        let first = detect_rust_entry_points(tmp.path());
        let second = detect_rust_entry_points(tmp.path());
        assert_eq!(paths(&first), paths(&second));
        assert_eq!(
            paths(&first),
            ["src/bin/a.rs", "src/bin/z.rs", "src/lib.rs", "src/main.rs"]
        );
    }
}
