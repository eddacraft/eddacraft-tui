//! Build script for `eddacraft-anvil-checks`.
//!
//! `registry_loader::EMBEDDED_REGISTRY` pulls the compiled pattern catalogue
//! into the binary via `include_str!`. Cargo only re-runs the embed when
//! its own source files change, so without this `rerun-if-changed`
//! directive a fresh `patterns:compile` run (which rewrites
//! `patterns/compiled/registry.json`) would not be picked up by
//! incremental builds — the binary would ship a stale catalogue until
//! `cargo clean` or an unrelated source edit forced a rebuild.

fn main() {
    println!("cargo:rerun-if-changed=../../patterns/compiled/registry.json");
}
