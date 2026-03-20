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
