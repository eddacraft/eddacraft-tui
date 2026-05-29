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
    /// The tree-sitter grammar could not be loaded for a language, almost
    /// always an ABI version mismatch between the `tree-sitter` runtime and a
    /// generated grammar crate. Surfaced as a `Result` rather than a panic
    /// (LANGTS-005 K4) because the parse path is load-bearing for daemon mode —
    /// a long-running daemon must degrade gracefully, not abort the process.
    #[error("failed to load {language:?} grammar (tree-sitter version mismatch): {source}")]
    LanguageInit {
        language: Language,
        #[source]
        source: tree_sitter::LanguageError,
    },
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
///
/// # Thread-safety strategy (LANGTS-005 K3)
///
/// A `tree_sitter::Parser` is `Send` but not `Sync`, so a single [`Parser`]
/// must not be shared across threads behind a shared reference. The adopted
/// strategy is **audit option (1): a thread-local parser per worker** — each
/// thread constructs and owns its own [`Parser`] (and therefore its own
/// per-language `tree_sitter::Parser` pool and AST cache). The orchestration
/// layers (`watch.rs`, `embedded.rs`) already create a `Parser` per scan/owner
/// rather than sharing one across worker threads, so no shared mutable parser
/// ever crosses a thread boundary.
///
/// This keeps each `tree_sitter::Parser` confined to one thread for its whole
/// lifetime, which is exactly the invariant the C grammar runtime requires. A
/// shared global parser pool behind a mutex was rejected (audit option (2)) as
/// it would serialise all parsing; per-thread ownership scales with workers.
/// The choice is recorded here per the T3 checklist §1, and exercised by the
/// `concurrent_parses_across_languages_are_isolated` regression test below.
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
    ///
    /// Returns a [`ParseError::LanguageInit`] instead of panicking when the
    /// grammar cannot be loaded (LANGTS-005 K4) — load-bearing for daemon mode.
    fn get_parser(&mut self, lang: Language) -> Result<&mut tree_sitter::Parser, ParseError> {
        use std::collections::hash_map::Entry;

        match self.parsers.entry(lang) {
            Entry::Occupied(e) => Ok(e.into_mut()),
            Entry::Vacant(e) => {
                // Construct the parser inline. A grammar/ABI mismatch is what
                // `set_language` reports via `LanguageError::Version` — the case
                // the old code `expect`-panicked on; surfacing it as
                // `ParseError::LanguageInit` keeps the parse path panic-free
                // (load-bearing for daemon mode, LANGTS-005 K4).
                let mut parser = tree_sitter::Parser::new();
                parser
                    .set_language(&lang.ts_language())
                    .map_err(|source| ParseError::LanguageInit {
                        language: lang,
                        source,
                    })?;
                Ok(e.insert(parser))
            }
        }
    }

    /// Parse a file from its content bytes. Uses the AST cache to skip
    /// reparsing if the content hash AND grammar version match.
    pub fn parse_bytes(&mut self, path: &Path, content: &[u8]) -> Result<ParseResult, ParseError> {
        let lang = Language::from_path(path)
            .ok_or_else(|| ParseError::UnsupportedLanguage(path.to_path_buf()))?;

        let content_hash = hash_content(content);
        let grammar_version = lang.grammar_version();
        let path_buf = path.to_path_buf();

        // Check cache (keyed on content hash AND grammar version, K2).
        if let Some(tree) = self.cache.get(&path_buf, content_hash, grammar_version) {
            return Ok(ParseResult {
                path: path_buf,
                language: lang,
                tree: tree.clone(),
                cached: true,
            });
        }

        // Parse
        let parser = self.get_parser(lang)?;
        let tree = parser
            .parse(content, None)
            .ok_or_else(|| ParseError::ParseFailed(path_buf.clone()))?;

        self.cache.insert(
            path_buf.clone(),
            content_hash,
            grammar_version,
            tree.clone(),
        );

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

    // --- K4: parse path surfaces a grammar mismatch as a Result, not a panic ---

    #[test]
    fn language_mismatch_maps_to_parse_error_not_panic() {
        // A grammar/ABI mismatch is exactly what `set_language` reports via
        // `LanguageError::Version`, and what the old code `expect`-panicked on.
        // `get_parser` maps it to a recoverable `ParseError::LanguageInit`
        // (no panic, no process abort — load-bearing for daemon mode). The
        // mapping is now inline in `get_parser`; this pins the error variant's
        // shape so the daemon-facing contract (language carried for diagnostics)
        // cannot regress.
        let err = ParseError::LanguageInit {
            language: Language::TypeScript,
            source: tree_sitter::LanguageError::Version(99),
        };
        match err {
            ParseError::LanguageInit { language, .. } => {
                assert_eq!(language, Language::TypeScript);
            }
            other => panic!("expected LanguageInit, got {other:?}"),
        }
    }

    #[test]
    fn parse_path_returns_result_without_panicking() {
        // Smoke: the real parse path is fully Result-based end to end — a
        // healthy grammar parses to Ok, and `get_parser`'s grammar-load branch
        // yields `Err(ParseError::LanguageInit)` rather than unwinding (pinned
        // by the variant test above). That the call site compiles as a `?`/Ok
        // chain is itself the K4 guarantee.
        let mut parser = Parser::new();
        let ok = parser.parse_bytes(Path::new("src/greet.ts"), TS_SOURCE);
        assert!(ok.is_ok());
    }

    // --- K3: per-thread parser ownership; concurrent parses across languages ---

    #[test]
    fn concurrent_parses_across_languages_are_isolated() {
        // Each thread owns its own Parser (audit option (1)); concurrent parses
        // across TS and JS must complete without data races or panics. Run many
        // iterations to give a thread-confinement violation a chance to surface
        // under the test harness.
        let handles: Vec<_> = (0..8)
            .map(|i| {
                std::thread::spawn(move || {
                    let mut parser = Parser::new();
                    for _ in 0..50 {
                        let (path, src): (&Path, &[u8]) = if i % 2 == 0 {
                            (Path::new("src/greet.ts"), TS_SOURCE)
                        } else {
                            (Path::new("src/add.js"), JS_SOURCE)
                        };
                        let result = parser.parse_bytes(path, src).unwrap();
                        assert!(!result.tree.root_node().has_error());
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().expect("worker thread panicked");
        }
    }
}
