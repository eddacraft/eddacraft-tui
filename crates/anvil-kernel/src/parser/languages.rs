use std::path::Path;

/// Languages supported by the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    TypeScript,
    Tsx,
    JavaScript,
    Jsx,
    Rust,
    Python,
}

impl Language {
    /// Determine language from file extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "ts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "js" | "mjs" | "cjs" => Some(Self::JavaScript),
            "jsx" => Some(Self::Jsx),
            "rs" => Some(Self::Rust),
            "py" | "pyi" => Some(Self::Python),
            _ => None,
        }
    }

    /// Get the tree-sitter language for this language.
    pub fn ts_language(&self) -> tree_sitter::Language {
        match self {
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::JavaScript | Self::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
        }
    }

    /// A grammar-version fingerprint for this language's tree-sitter grammar.
    ///
    /// Used as part of the AST cache key (LANGTS-005 K2) so that a tree-sitter
    /// grammar bump can never serve a tree parsed by an older grammar on the
    /// same content hash. The fingerprint folds the grammar's ABI version with
    /// its structural counts (`node_kind_count`, `field_count`,
    /// `parse_state_count`) — any of which shifts when the grammar's `.scm` /
    /// generated tables change across a version bump — into a single `u64`.
    ///
    /// It is derived purely from the compiled `tree_sitter::Language` and is
    /// therefore deterministic for a given grammar build (same input → same
    /// output, per the determinism principle). It is a cache *discriminator*,
    /// not a semantic version: equality means "same grammar build", and that is
    /// exactly the invariant the cache needs.
    pub fn grammar_version(&self) -> u64 {
        let lang = self.ts_language();
        // Distinct FNV-1a seed from `cache::hash_content` so the two `u64`
        // cache-key fields (content hash vs grammar version) are never built
        // from the same constants — they occupy separate slots and must not be
        // accidentally interchangeable. The `Language` discriminant is folded in
        // first so variants that share one tree-sitter grammar (JavaScript/Jsx)
        // — or any two grammars that happen to share structural counts — still
        // produce distinct fingerprints.
        let mut hash: u64 = 0x517c_c1b7_2722_0a95;
        for part in [
            *self as u64,
            lang.abi_version() as u64,
            lang.node_kind_count() as u64,
            lang.field_count() as u64,
            lang.parse_state_count() as u64,
        ] {
            hash ^= part;
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammar_versions_are_distinct_per_language() {
        // The fingerprint is a cache *discriminator*: every supported language
        // must hash to a different value, including JavaScript vs Jsx which
        // share one tree-sitter grammar (distinguished only by the folded
        // discriminant). A collision would let a cached tree for one grammar be
        // served for another on the same content hash — the exact K2 bug.
        let versions = [
            Language::TypeScript.grammar_version(),
            Language::Tsx.grammar_version(),
            Language::JavaScript.grammar_version(),
            Language::Jsx.grammar_version(),
            Language::Rust.grammar_version(),
            Language::Python.grammar_version(),
        ];
        for (i, a) in versions.iter().enumerate() {
            for b in &versions[i + 1..] {
                assert_ne!(a, b, "grammar_version collision between languages");
            }
        }
    }

    #[test]
    fn grammar_version_is_deterministic() {
        assert_eq!(
            Language::TypeScript.grammar_version(),
            Language::TypeScript.grammar_version(),
            "same grammar build must hash identically (determinism principle)"
        );
    }

    #[test]
    fn detects_typescript() {
        assert_eq!(
            Language::from_path(Path::new("src/main.ts")),
            Some(Language::TypeScript)
        );
    }

    #[test]
    fn detects_tsx() {
        assert_eq!(
            Language::from_path(Path::new("App.tsx")),
            Some(Language::Tsx)
        );
    }

    #[test]
    fn detects_javascript_variants() {
        assert_eq!(
            Language::from_path(Path::new("index.js")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            Language::from_path(Path::new("config.mjs")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            Language::from_path(Path::new("require.cjs")),
            Some(Language::JavaScript)
        );
    }

    #[test]
    fn detects_rust() {
        assert_eq!(
            Language::from_path(Path::new("crates/anvil-kernel/src/main.rs")),
            Some(Language::Rust)
        );
        assert_eq!(
            Language::from_path(Path::new("lib.rs")),
            Some(Language::Rust)
        );
    }

    #[test]
    fn detects_python() {
        assert_eq!(
            Language::from_path(Path::new("pkg/mod.py")),
            Some(Language::Python)
        );
        assert_eq!(
            Language::from_path(Path::new("stubs/types.pyi")),
            Some(Language::Python)
        );
    }

    #[test]
    fn returns_none_for_unknown() {
        assert_eq!(Language::from_path(Path::new("README.md")), None);
        assert_eq!(Language::from_path(Path::new("Cargo.toml")), None);
    }

    #[test]
    fn rust_grammar_parses_real_source() {
        // RSTLAN-001: the bound grammar must produce a non-error tree for
        // representative Rust source. This is the grammar-wiring acceptance —
        // symbol extraction is RSTLAN-002. A grammar/ABI mismatch surfaces here
        // as a `set_language` failure rather than silently downstream.
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&Language::Rust.ts_language())
            .expect("tree-sitter-rust grammar must bind (ABI compatible)");
        let source = r"
            use std::collections::HashMap;
            pub mod inner;

            pub fn main() {
                let _m: HashMap<String, u32> = HashMap::new();
            }
        ";
        let tree = parser.parse(source, None).expect("parse must yield a tree");
        assert!(
            !tree.root_node().has_error(),
            "well-formed Rust source must parse without errors"
        );
        assert_eq!(tree.root_node().kind(), "source_file");
    }
}
