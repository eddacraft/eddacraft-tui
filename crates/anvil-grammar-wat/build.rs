//! Compiles the vendored WebAssembly-text (`.wat`/`.wast`) tree-sitter grammar
//! (LTW2-002, ADR-093).
//!
//! The grammar has **no published crate** and **no external scanner**, so a
//! single generated `parser.c` is compiled and linked; `src/lib.rs` binds the
//! resulting `tree_sitter_wat()` symbol. Source + provenance live under
//! `vendor/tree-sitter-wat/`.

use std::path::Path;

fn main() {
    let dir = Path::new("vendor/tree-sitter-wat");
    let parser = dir.join("parser.c");

    cc::Build::new()
        .include(dir)
        .file(&parser)
        // Third-party generated C: silence its warnings so they don't drown the
        // workspace's own build output. Not our source to fix.
        .warnings(false)
        .compile("tree_sitter_wat");

    println!("cargo:rerun-if-changed={}", parser.display());
}
