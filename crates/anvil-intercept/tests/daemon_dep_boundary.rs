//! Dependency-boundary guard for the daemon save-time graph cache (ADR-064 / B5).
//!
//! The resident intercept daemon depends on `eddacraft-anvil-graph-cache`
//! (`petgraph`-only) for the graph state it caches and certifies against, but it
//! must NOT pull in the tree-sitter parser surface — that is the entire
//! dep-weight rationale for extracting the crate instead of depending on the
//! full `anvil-kernel`. This guard locks that property in so a future change
//! that re-introduces a parser dep to the daemon (or to the graph-cache crate)
//! fails loudly here rather than silently bloating the always-resident binary.
//!
//! The check inspects the **normal** (non-dev, non-build) dependency edges only —
//! the crates pulled into the daemon's own build. (Proc-macro crates are normal
//! edges too and so appear here even though they run host-only at compile time;
//! that is immaterial — none of the forbidden parser crates is a proc-macro.)
//! Dev-dependencies of `anvil-intercept` legitimately pull in the kernel (and
//! thus tree-sitter) for integration tests; those are excluded by
//! `--edges normal`. `--prefix none`
//! flattens the tree; cargo prints each package's name on its own line (a
//! deduplicated back-reference still carries the bare name, e.g. `foo v1 (*)`),
//! so a plain substring scan over the output sees every reachable package.
//!
//! Note the asymmetry between the two crates: ADR-064 §2 forbids the *graph-cache*
//! crate from carrying any of `tree-sitter`, `notify`, `walkdir`, `ignore`, or
//! `rayon`, so it is checked against that full list. The *daemon* crate is only
//! checked for `tree-sitter`: it legitimately carries `rayon` (pre-existing, via
//! `anvil-intercept-rules`/`anvil-checks`), so the full list does not apply to it.

use std::process::Command;

/// The parser/heavyweight crates ADR-064 §2 keeps out of `anvil-graph-cache`.
const GRAPH_CACHE_FORBIDDEN: &[&str] = &["tree-sitter", "notify", "walkdir", "ignore", "rayon"];

/// Run `cargo tree` for `package` over its normal-edge dependency tree and
/// return the flattened package list as a single string.
fn normal_dep_tree(package: &str) -> String {
    let output = Command::new(env!("CARGO"))
        .args([
            "tree", "-p", package, "--edges", "normal", "--prefix", "none",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("`cargo tree` should be runnable under the test toolchain");

    assert!(
        output.status.success(),
        "`cargo tree -p {package}` failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("cargo tree output is valid UTF-8")
}

#[test]
fn daemon_does_not_link_tree_sitter() {
    // The daemon crate and the graph-cache crate it depends on must both be
    // free of any `tree-sitter*` package in their shipped (normal-edge) tree —
    // the core dep-weight boundary the crate extraction exists to hold.
    for package in ["eddacraft-anvil-intercept", "eddacraft-anvil-graph-cache"] {
        let tree = normal_dep_tree(package);
        assert!(
            !tree.contains("tree-sitter"),
            "{package} must not link tree-sitter (ADR-064 dep-weight boundary), \
             but its normal dependency tree contains it:\n{tree}"
        );
    }
}

#[test]
fn graph_cache_excludes_full_parser_dep_set() {
    // ADR-064 §2 pins the graph-cache crate's dependencies to
    // `anvil-kernel-types + petgraph + serde + thiserror (+ tracing)` and
    // explicitly forbids the parser/filesystem/parallelism crates. Guard the
    // whole list — not just tree-sitter — so a future addition of e.g. `rayon`
    // or `walkdir` to the parser-free crate fails here rather than silently
    // eroding the boundary.
    let tree = normal_dep_tree("eddacraft-anvil-graph-cache");
    for forbidden in GRAPH_CACHE_FORBIDDEN {
        assert!(
            !tree.contains(forbidden),
            "eddacraft-anvil-graph-cache must not link `{forbidden}` (ADR-064 §2), \
             but its normal dependency tree contains it:\n{tree}"
        );
    }
}

#[test]
fn kernel_still_links_tree_sitter() {
    // Positive control: the extraction must not have accidentally severed the
    // parser from the kernel, which legitimately owns the tree-sitter surface.
    // If this regresses, `daemon_does_not_link_tree_sitter` could pass for the
    // wrong reason (tree-sitter dropped from the whole workspace).
    let tree = normal_dep_tree("eddacraft-anvil-kernel");
    assert!(
        tree.contains("tree-sitter"),
        "eddacraft-anvil-kernel is expected to own the tree-sitter parser \
         surface, but its normal dependency tree no longer contains it:\n{tree}"
    );
}
