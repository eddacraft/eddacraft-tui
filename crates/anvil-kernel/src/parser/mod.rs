pub mod cache;
pub mod extract;
pub mod languages;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use self::cache::{AstCache, hash_content};
use self::languages::Language;

/// Error type for parser operations.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("unsupported file type: {0}")]
    UnsupportedLanguage(PathBuf),
    #[error("tree-sitter parse failed for {0}")]
    ParseFailed(PathBuf),
    #[error("IO error reading {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Result of parsing a single file.
#[derive(Debug)]
pub struct ParseResult {
    pub path: PathBuf,
    pub language: Language,
    pub tree: tree_sitter::Tree,
    pub cached: bool,
}

/// Incremental parser with per-language tree-sitter instances and AST cache.
pub struct Parser {
    parsers: HashMap<Language, tree_sitter::Parser>,
    cache: AstCache,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            parsers: HashMap::new(),
            cache: AstCache::new(),
        }
    }

    /// Get or create a tree-sitter parser for the given language.
    fn get_parser(&mut self, lang: Language) -> &mut tree_sitter::Parser {
        self.parsers.entry(lang).or_insert_with(|| {
            let mut parser = tree_sitter::Parser::new();
            parser
                .set_language(&lang.ts_language())
                .expect("language version mismatch");
            parser
        })
    }

    /// Parse a file from its content bytes. Uses the AST cache to skip
    /// reparsing if the content hash matches.
    pub fn parse_bytes(&mut self, path: &Path, content: &[u8]) -> Result<ParseResult, ParseError> {
        let lang = Language::from_path(path)
            .ok_or_else(|| ParseError::UnsupportedLanguage(path.to_path_buf()))?;

        let content_hash = hash_content(content);
        let path_buf = path.to_path_buf();

        // Check cache
        if let Some(tree) = self.cache.get(&path_buf, content_hash) {
            return Ok(ParseResult {
                path: path_buf,
                language: lang,
                tree: tree.clone(),
                cached: true,
            });
        }

        // Parse
        let parser = self.get_parser(lang);
        let tree = parser
            .parse(content, None)
            .ok_or_else(|| ParseError::ParseFailed(path_buf.clone()))?;

        self.cache
            .insert(path_buf.clone(), content_hash, tree.clone());

        Ok(ParseResult {
            path: path_buf,
            language: lang,
            tree,
            cached: false,
        })
    }

    /// Parse a file from disk.
    pub fn parse_file(&mut self, path: &Path) -> Result<ParseResult, ParseError> {
        let content = std::fs::read(path).map_err(|e| ParseError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        self.parse_bytes(path, &content)
    }

    /// Remove a file from the cache.
    pub fn invalidate(&mut self, path: &Path) {
        self.cache.remove(&path.to_path_buf());
    }

    /// Number of cached ASTs.
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TS_SOURCE: &[u8] = b"
export function greet(name: string): string {
    return `Hello, ${name}!`;
}

export class Greeter {
    private name: string;
    constructor(name: string) {
        this.name = name;
    }
    greet(): string {
        return `Hello, ${this.name}!`;
    }
}
";

    const JS_SOURCE: &[u8] = b"
function add(a, b) {
    return a + b;
}
module.exports = { add };
";

    #[test]
    fn parses_typescript() {
        let mut parser = Parser::new();
        let result = parser
            .parse_bytes(Path::new("src/greet.ts"), TS_SOURCE)
            .unwrap();
        assert_eq!(result.language, Language::TypeScript);
        assert!(!result.cached);
        assert!(!result.tree.root_node().has_error());
    }

    #[test]
    fn parses_javascript() {
        let mut parser = Parser::new();
        let result = parser
            .parse_bytes(Path::new("src/add.js"), JS_SOURCE)
            .unwrap();
        assert_eq!(result.language, Language::JavaScript);
        assert!(!result.tree.root_node().has_error());
    }

    #[test]
    fn returns_cached_on_same_content() {
        let mut parser = Parser::new();
        let path = Path::new("src/greet.ts");

        let r1 = parser.parse_bytes(path, TS_SOURCE).unwrap();
        assert!(!r1.cached);

        let r2 = parser.parse_bytes(path, TS_SOURCE).unwrap();
        assert!(r2.cached);
    }

    #[test]
    fn reparses_on_changed_content() {
        let mut parser = Parser::new();
        let path = Path::new("src/greet.ts");

        parser.parse_bytes(path, TS_SOURCE).unwrap();
        let r2 = parser.parse_bytes(path, b"const x = 42;").unwrap();
        assert!(!r2.cached);
    }

    #[test]
    fn rejects_unsupported_extension() {
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("README.md"), b"# Hello");
        assert!(matches!(result, Err(ParseError::UnsupportedLanguage(_))));
    }

    #[test]
    fn invalidate_clears_cache() {
        let mut parser = Parser::new();
        let path = Path::new("src/greet.ts");

        parser.parse_bytes(path, TS_SOURCE).unwrap();
        assert_eq!(parser.cache_size(), 1);

        parser.invalidate(path);
        assert_eq!(parser.cache_size(), 0);
    }
}
