//! Vendored WebAssembly-text (`.wat`/`.wast`) tree-sitter grammar (LTW2-002,
//! ADR-093).
//!
//! The upstream grammar ([`wasm-lsp/tree-sitter-wasm`](https://github.com/wasm-lsp/tree-sitter-wasm))
//! ships **no crate**, so its generated `parser.c` is vendored here and compiled
//! by `build.rs`. This crate exists solely to isolate the intrinsically-`unsafe`
//! grammar FFI from the `forbid(unsafe_code)` kernel — it exposes one safe
//! function, [`language`]. See `vendor/tree-sitter-wat/README.md` for provenance
//! and the ABI note.

use tree_sitter::Language;
use tree_sitter_language::LanguageFn;

// Provided by the compiled `vendor/tree-sitter-wat/parser.c`.
unsafe extern "C" {
    fn tree_sitter_wat() -> *const ();
}

/// The WebAssembly-text tree-sitter [`Language`].
///
/// Safe wrapper over the vendored grammar's entry point: `build.rs` compiles the
/// matching generated `parser.c`, so `tree_sitter_wat` is a valid tree-sitter
/// language function — the invariant [`LanguageFn::from_raw`] requires.
#[must_use]
pub fn language() -> Language {
    let function = unsafe { LanguageFn::from_raw(tree_sitter_wat) };
    function.into()
}

#[cfg(test)]
mod tests {
    #[test]
    fn binds_and_parses_a_module() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::language())
            .expect("vendored wat grammar must bind (ABI compatible)");
        let tree = parser
            .parse("(module $m\n  (func $f (result i32) i32.const 0))\n", None)
            .expect("parse must yield a tree");
        assert!(!tree.root_node().has_error());
        assert_eq!(tree.root_node().kind(), "ROOT");
    }
}
