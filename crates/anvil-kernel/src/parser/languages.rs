use std::path::Path;

/// Languages supported by the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    TypeScript,
    Tsx,
    JavaScript,
    Jsx,
}

impl Language {
    /// Determine language from file extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "ts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "js" | "mjs" | "cjs" => Some(Self::JavaScript),
            "jsx" => Some(Self::Jsx),
            _ => None,
        }
    }

    /// Get the tree-sitter language for this language.
    pub fn ts_language(&self) -> tree_sitter::Language {
        match self {
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::JavaScript | Self::Jsx => tree_sitter_javascript::LANGUAGE.into(),
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
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for part in [
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
    fn returns_none_for_unknown() {
        assert_eq!(Language::from_path(Path::new("README.md")), None);
        assert_eq!(Language::from_path(Path::new("Cargo.toml")), None);
    }
}
