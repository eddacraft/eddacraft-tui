# Wave 1: Foundation + Independent Surfaces — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development
> (if subagents available) or superpowers:executing-plans to implement this plan.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish the KERN Phase 1 runtime (watcher + parser + symbol extraction)
and port the three simplest Ink TUI surfaces to Ratatui, delivering the first
usable Rust kernel infrastructure and TUI surfaces.

**Architecture:** Two independent tracks that share only `anvil-kernel-types`.
Track A builds the kernel's watch/parse/extract pipeline in `crates/anvil-kernel/`.
Track B builds Ratatui surface ports in `crates/anvil-tui/` using mock data
(no kernel dependency). A new `crates/anvil-kernel/` crate is created for the
kernel; surfaces live in `crates/anvil-tui/`.

**Tech Stack:** Rust 2024 edition, tree-sitter (0.26), notify (8), petgraph (0.8),
ratatui (0.30), crossterm (0.29), tokio (1), insta (1) for snapshot testing,
criterion (0.5) for benchmarks.

**APS Work Items:** KERN-005, KERN-010, KERN-011, KERN-012, KERN-013, PORT-010,
PORT-011, PORT-012, RATS-004.

---

## Chunk 1: Track A — Kernel Phase 1

### Task 1: Create `anvil-kernel` crate scaffold (KERN-005 prep)

**Files:**
- Create: `crates/anvil-kernel/Cargo.toml`
- Create: `crates/anvil-kernel/src/lib.rs`
- Create: `crates/anvil-kernel/src/watcher/mod.rs`
- Create: `crates/anvil-kernel/src/parser/mod.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Create crate directory and Cargo.toml**

```toml
# crates/anvil-kernel/Cargo.toml
[package]
name = "anvil-kernel"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Anvil Rust kernel — watcher, parser, semantic graph, policy engine"

[dependencies]
anvil-kernel-types = { path = "../anvil-kernel-types" }
tree-sitter = { workspace = true }
tree-sitter-typescript = { workspace = true }
tree-sitter-javascript = { workspace = true }
notify = { workspace = true }
petgraph = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }

[dev-dependencies]
insta = { workspace = true }
tempfile = "3"

[lints]
workspace = true
```

- [ ] **Step 2: Create lib.rs with module stubs**

```rust
// crates/anvil-kernel/src/lib.rs
pub mod parser;
pub mod watcher;
```

- [ ] **Step 3: Create module stubs**

```rust
// crates/anvil-kernel/src/watcher/mod.rs
// File watcher with debounce and backpressure

// crates/anvil-kernel/src/parser/mod.rs
// Incremental tree-sitter parsing with AST cache
```

- [ ] **Step 4: Add to workspace members**

Add `"crates/anvil-kernel"` to the `members` array in root `Cargo.toml`.

- [ ] **Step 5: Verify build**

Run: `cargo check -p eddacraft-anvil-kernel`
Expected: compiles with no errors

- [ ] **Step 6: Commit**

```bash
git add crates/anvil-kernel/ Cargo.toml
git commit -m "chore(kern): scaffold anvil-kernel crate with watcher and parser modules"
```

---

### Task 2: notify-rs watcher with debounce + backpressure (KERN-010)

**Files:**
- Create: `crates/anvil-kernel/src/watcher/debounce.rs`
- Create: `crates/anvil-kernel/src/watcher/events.rs`
- Modify: `crates/anvil-kernel/src/watcher/mod.rs`

- [ ] **Step 1: Define watcher event types**

```rust
// crates/anvil-kernel/src/watcher/events.rs
use std::path::PathBuf;
use std::time::Instant;

/// A coalesced batch of file changes after debouncing.
#[derive(Debug, Clone)]
pub struct ChangeBatch {
    pub changes: Vec<FileChange>,
    pub received_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Created,
    Modified,
    Removed,
}
```

- [ ] **Step 2: Write failing test for debouncer**

```rust
// crates/anvil-kernel/src/watcher/debounce.rs
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::events::{ChangeBatch, ChangeKind, FileChange};

/// Coalesces rapid file changes within a configurable window.
///
/// Files changed multiple times within the debounce window are
/// collapsed into a single change. Backpressure is applied by
/// bounding the pending change map to `max_pending` entries —
/// if exceeded, the oldest batch is flushed immediately.
pub struct Debouncer {
    window: Duration,
    max_pending: usize,
    pending: HashMap<PathBuf, (ChangeKind, Instant)>,
}

impl Debouncer {
    pub fn new(window: Duration, max_pending: usize) -> Self {
        Self {
            window,
            max_pending,
            pending: HashMap::new(),
        }
    }

    /// Record a file change. Returns a batch if the debounce window
    /// has elapsed for any pending changes or backpressure triggers.
    pub fn record(&mut self, change: FileChange) -> Option<ChangeBatch> {
        let now = Instant::now();
        self.pending.insert(change.path, (change.kind, now));

        if self.pending.len() > self.max_pending {
            return Some(self.flush(now));
        }
        None
    }

    /// Check for changes whose debounce window has elapsed.
    /// Call this on a timer tick (e.g. every 10-50ms).
    pub fn tick(&mut self) -> Option<ChangeBatch> {
        let now = Instant::now();
        let ready: Vec<PathBuf> = self
            .pending
            .iter()
            .filter(|(_, (_, ts))| now.duration_since(*ts) >= self.window)
            .map(|(p, _)| p.clone())
            .collect();

        if ready.is_empty() {
            return None;
        }

        let changes: Vec<FileChange> = ready
            .into_iter()
            .filter_map(|p| {
                self.pending
                    .remove(&p)
                    .map(|(kind, _)| FileChange { path: p, kind })
            })
            .collect();

        Some(ChangeBatch {
            changes,
            received_at: now,
        })
    }

    /// Flush all pending changes immediately.
    pub fn flush(&mut self, now: Instant) -> ChangeBatch {
        let changes: Vec<FileChange> = self
            .pending
            .drain()
            .map(|(path, (kind, _))| FileChange { path, kind })
            .collect();
        ChangeBatch {
            changes,
            received_at: now,
        }
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesces_rapid_changes_to_same_file() {
        let mut d = Debouncer::new(Duration::from_millis(50), 100);

        let path = PathBuf::from("src/main.rs");
        // Two rapid changes to the same file
        assert!(d
            .record(FileChange {
                path: path.clone(),
                kind: ChangeKind::Modified,
            })
            .is_none());
        assert!(d
            .record(FileChange {
                path: path.clone(),
                kind: ChangeKind::Modified,
            })
            .is_none());

        // Only one pending entry
        assert_eq!(d.pending_count(), 1);
    }

    #[test]
    fn backpressure_flushes_when_max_exceeded() {
        let mut d = Debouncer::new(Duration::from_millis(50), 2);

        d.record(FileChange {
            path: PathBuf::from("a.rs"),
            kind: ChangeKind::Modified,
        });
        d.record(FileChange {
            path: PathBuf::from("b.rs"),
            kind: ChangeKind::Modified,
        });
        // Third change exceeds max_pending=2
        let batch = d.record(FileChange {
            path: PathBuf::from("c.rs"),
            kind: ChangeKind::Modified,
        });

        assert!(batch.is_some());
        let batch = batch.unwrap();
        assert_eq!(batch.changes.len(), 3);
        assert_eq!(d.pending_count(), 0);
    }

    #[test]
    fn tick_emits_after_window_elapses() {
        let mut d = Debouncer::new(Duration::from_millis(0), 100);

        d.record(FileChange {
            path: PathBuf::from("a.rs"),
            kind: ChangeKind::Created,
        });

        // Window is 0ms so tick should emit immediately
        let batch = d.tick();
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().changes.len(), 1);
        assert_eq!(d.pending_count(), 0);
    }

    #[test]
    fn tick_does_not_emit_within_window() {
        let mut d = Debouncer::new(Duration::from_secs(60), 100);

        d.record(FileChange {
            path: PathBuf::from("a.rs"),
            kind: ChangeKind::Modified,
        });

        // Window is 60s, tick should not emit
        let batch = d.tick();
        assert!(batch.is_none());
        assert_eq!(d.pending_count(), 1);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p eddacraft-anvil-kernel -- watcher::debounce`
Expected: all 4 tests pass

- [ ] **Step 4: Implement the file watcher**

```rust
// crates/anvil-kernel/src/watcher/mod.rs
pub mod debounce;
pub mod events;

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use self::debounce::Debouncer;
use self::events::{ChangeBatch, ChangeKind, FileChange};

/// Configuration for the file watcher.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Root directory to watch.
    pub root: PathBuf,
    /// Debounce window for coalescing rapid changes.
    pub debounce_window: Duration,
    /// Maximum pending changes before backpressure flush.
    pub max_pending: usize,
    /// Tick interval for checking debounce expiry.
    pub tick_interval: Duration,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            debounce_window: Duration::from_millis(50),
            max_pending: 500,
            tick_interval: Duration::from_millis(20),
        }
    }
}

/// Error type for watcher operations.
#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("channel receive error: {0}")]
    Recv(#[from] mpsc::RecvTimeoutError),
}

/// Starts watching the given directory and sends `ChangeBatch` events
/// to the returned receiver. Runs until the watcher handle is dropped.
pub fn start_watcher(
    config: WatcherConfig,
) -> Result<(RecommendedWatcher, mpsc::Receiver<ChangeBatch>), WatcherError> {
    let (raw_tx, raw_rx) = mpsc::channel::<notify::Result<Event>>();
    let (batch_tx, batch_rx) = mpsc::channel::<ChangeBatch>();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = raw_tx.send(res);
        },
        notify::Config::default(),
    )?;

    watcher.watch(&config.root, RecursiveMode::Recursive)?;

    let debounce_window = config.debounce_window;
    let max_pending = config.max_pending;
    let tick_interval = config.tick_interval;

    std::thread::spawn(move || {
        let mut debouncer = Debouncer::new(debounce_window, max_pending);

        loop {
            match raw_rx.recv_timeout(tick_interval) {
                Ok(Ok(event)) => {
                    for path in event.paths {
                        let kind = match event.kind {
                            EventKind::Create(_) => ChangeKind::Created,
                            EventKind::Modify(_) => ChangeKind::Modified,
                            EventKind::Remove(_) => ChangeKind::Removed,
                            _ => continue,
                        };
                        if let Some(batch) = debouncer.record(FileChange { path, kind }) {
                            if batch_tx.send(batch).is_err() {
                                return;
                            }
                        }
                    }
                }
                Ok(Err(_)) => continue,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Check debounce expiry
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }

            if let Some(batch) = debouncer.tick() {
                if batch_tx.send(batch).is_err() {
                    return;
                }
            }
        }
    });

    Ok((watcher, batch_rx))
}
```

- [ ] **Step 5: Add thiserror dependency to Cargo.toml**

Add `thiserror = "2"` to `[dependencies]` in `crates/anvil-kernel/Cargo.toml`
and add `thiserror = "2"` to `[workspace.dependencies]` in root `Cargo.toml`.

- [ ] **Step 6: Write integration test for watcher**

```rust
// crates/anvil-kernel/tests/watcher_integration.rs
use std::fs;
use std::time::Duration;

use anvil_kernel::watcher::{start_watcher, WatcherConfig};

#[test]
fn detects_file_creation() {
    let dir = tempfile::tempdir().unwrap();
    let config = WatcherConfig {
        root: dir.path().to_path_buf(),
        debounce_window: Duration::from_millis(10),
        max_pending: 100,
        tick_interval: Duration::from_millis(5),
    };

    let (_watcher, rx) = start_watcher(config).unwrap();

    // Give the watcher time to start
    std::thread::sleep(Duration::from_millis(50));

    // Create a file
    fs::write(dir.path().join("test.rs"), "fn main() {}").unwrap();

    // Wait for the batch
    let batch = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(!batch.changes.is_empty());
    assert!(batch
        .changes
        .iter()
        .any(|c| c.path.ends_with("test.rs")));
}
```

- [ ] **Step 7: Run integration test**

Run: `cargo test -p eddacraft-anvil-kernel --test watcher_integration`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add crates/anvil-kernel/ Cargo.toml
git commit -m "feat(kern): add file watcher with debounce and backpressure (KERN-010)"
```

---

### Task 3: tree-sitter incremental parsing with AST cache (KERN-011)

**Files:**
- Create: `crates/anvil-kernel/src/parser/cache.rs`
- Create: `crates/anvil-kernel/src/parser/languages.rs`
- Modify: `crates/anvil-kernel/src/parser/mod.rs`

- [ ] **Step 1: Define language support**

```rust
// crates/anvil-kernel/src/parser/languages.rs
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
            Self::JavaScript | Self::Jsx => tree_sitter_javascript::LANGUAGE_JAVASCRIPT.into(),
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
```

- [ ] **Step 2: Run language tests**

Run: `cargo test -p eddacraft-anvil-kernel -- parser::languages`
Expected: all 4 tests pass

- [ ] **Step 3: Implement AST cache**

```rust
// crates/anvil-kernel/src/parser/cache.rs
use std::collections::HashMap;
use std::path::PathBuf;

/// Content-addressed AST cache keyed by file path.
///
/// Stores parsed ASTs alongside a content hash. On reparse,
/// the hash is compared — if unchanged, the cached tree is returned.
pub struct AstCache {
    entries: HashMap<PathBuf, CacheEntry>,
}

struct CacheEntry {
    content_hash: u64,
    tree: tree_sitter::Tree,
}

impl AstCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Get a cached tree if the content hash matches.
    pub fn get(&self, path: &PathBuf, content_hash: u64) -> Option<&tree_sitter::Tree> {
        self.entries
            .get(path)
            .filter(|e| e.content_hash == content_hash)
            .map(|e| &e.tree)
    }

    /// Insert or update a cached tree.
    pub fn insert(&mut self, path: PathBuf, content_hash: u64, tree: tree_sitter::Tree) {
        self.entries.insert(path, CacheEntry { content_hash, tree });
    }

    /// Remove a file from the cache (e.g. on deletion).
    pub fn remove(&mut self, path: &PathBuf) -> bool {
        self.entries.remove(path).is_some()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Simple FNV-1a hash for content addressing.
pub fn hash_content(content: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in content {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_hit_on_same_content() {
        let mut cache = AstCache::new();
        let path = PathBuf::from("test.ts");
        let content = b"const x = 1;";
        let hash = hash_content(content);

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        let tree = parser.parse(content, None).unwrap();

        cache.insert(path.clone(), hash, tree);
        assert!(cache.get(&path, hash).is_some());
    }

    #[test]
    fn cache_miss_on_different_content() {
        let mut cache = AstCache::new();
        let path = PathBuf::from("test.ts");

        let content1 = b"const x = 1;";
        let content2 = b"const x = 2;";
        let hash1 = hash_content(content1);
        let hash2 = hash_content(content2);

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        let tree = parser.parse(content1, None).unwrap();

        cache.insert(path.clone(), hash1, tree);
        assert!(cache.get(&path, hash2).is_none());
    }

    #[test]
    fn remove_clears_entry() {
        let mut cache = AstCache::new();
        let path = PathBuf::from("test.ts");
        let hash = hash_content(b"x");

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        let tree = parser.parse(b"const x = 1;", None).unwrap();

        cache.insert(path.clone(), hash, tree);
        assert!(cache.remove(&path));
        assert!(cache.get(&path, hash).is_none());
    }

    #[test]
    fn hash_content_is_deterministic() {
        let content = b"hello world";
        assert_eq!(hash_content(content), hash_content(content));
    }

    #[test]
    fn hash_content_differs_for_different_input() {
        assert_ne!(hash_content(b"hello"), hash_content(b"world"));
    }
}
```

- [ ] **Step 4: Run cache tests**

Run: `cargo test -p eddacraft-anvil-kernel -- parser::cache`
Expected: all 5 tests pass

- [ ] **Step 5: Implement the parser**

```rust
// crates/anvil-kernel/src/parser/mod.rs
pub mod cache;
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
    pub fn parse_bytes(
        &mut self,
        path: &Path,
        content: &[u8],
    ) -> Result<ParseResult, ParseError> {
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
        let content =
            std::fs::read(path).map_err(|e| ParseError::Io {
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
        let r2 = parser
            .parse_bytes(path, b"const x = 42;")
            .unwrap();
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
```

- [ ] **Step 6: Run parser tests**

Run: `cargo test -p eddacraft-anvil-kernel -- parser::tests`
Expected: all 6 tests pass

- [ ] **Step 7: Commit**

```bash
git add crates/anvil-kernel/
git commit -m "feat(kern): add tree-sitter parser with AST cache (KERN-011)"
```

---

### Task 4: Symbol extraction (KERN-012)

**Files:**
- Create: `crates/anvil-kernel/src/parser/extract.rs`
- Create: `crates/anvil-kernel/src/parser/queries/typescript.scm`
- Create: `crates/anvil-kernel/src/parser/queries/javascript.scm`
- Modify: `crates/anvil-kernel/src/parser/mod.rs`

- [ ] **Step 1: Create tree-sitter query files**

```scm
;; crates/anvil-kernel/src/parser/queries/typescript.scm
;; Functions (named + arrow)
(function_declaration
  name: (identifier) @name) @function

(lexical_declaration
  (variable_declarator
    name: (identifier) @name
    value: (arrow_function))) @function

;; Classes
(class_declaration
  name: (type_identifier) @name) @class

;; Exports
(export_statement) @export

;; Imports
(import_statement
  source: (string) @source) @import
```

```scm
;; crates/anvil-kernel/src/parser/queries/javascript.scm
;; Functions
(function_declaration
  name: (identifier) @name) @function

(lexical_declaration
  (variable_declarator
    name: (identifier) @name
    value: (arrow_function))) @function

;; Classes
(class_declaration
  name: (identifier) @name) @class

;; Exports (CJS)
(expression_statement
  (assignment_expression
    left: (member_expression
      object: (identifier) @_obj
      property: (property_identifier) @_prop)
    (#eq? @_obj "module")
    (#eq? @_prop "exports"))) @export

;; Exports (ESM)
(export_statement) @export

;; Imports (ESM)
(import_statement
  source: (string) @source) @import

;; Imports (CJS)
(lexical_declaration
  (variable_declarator
    value: (call_expression
      function: (identifier) @_fn
      arguments: (arguments (string) @source)
      (#eq? @_fn "require")))) @import
```

- [ ] **Step 2: Implement symbol extractor**

```rust
// crates/anvil-kernel/src/parser/extract.rs
use std::path::Path;

use anvil_kernel_types::{SymbolKind, SymbolNode, Visibility};

/// Extracted symbols from a single file.
#[derive(Debug, Clone)]
pub struct FileSymbols {
    pub file: String,
    pub symbols: Vec<SymbolNode>,
    pub imports: Vec<ImportEdge>,
}

/// An import edge from one file to another.
#[derive(Debug, Clone)]
pub struct ImportEdge {
    pub from_file: String,
    pub to_source: String,
}

/// Extract symbols from a tree-sitter AST.
pub fn extract_symbols(
    tree: &tree_sitter::Tree,
    source: &[u8],
    file_path: &Path,
    id_offset: u64,
) -> FileSymbols {
    let file = file_path.to_string_lossy().to_string();
    let root = tree.root_node();
    let mut symbols = Vec::new();
    let mut imports = Vec::new();
    let mut next_id = id_offset;

    extract_from_node(
        root,
        source,
        &file,
        &mut symbols,
        &mut imports,
        &mut next_id,
    );

    FileSymbols {
        file,
        symbols,
        imports,
    }
}

fn extract_from_node(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    imports: &mut Vec<ImportEdge>,
    next_id: &mut u64,
) {
    match node.kind() {
        "function_declaration" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = node_text(name_node, source);
                symbols.push(SymbolNode {
                    id: *next_id,
                    kind: SymbolKind::Function,
                    name,
                    visibility: Visibility::Internal,
                    file: file.to_string(),
                    trust_level: Default::default(),
                });
                *next_id += 1;
            }
        }
        "class_declaration" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = node_text(name_node, source);
                symbols.push(SymbolNode {
                    id: *next_id,
                    kind: SymbolKind::Class,
                    name,
                    visibility: Visibility::Internal,
                    file: file.to_string(),
                    trust_level: Default::default(),
                });
                *next_id += 1;
            }
        }
        "export_statement" => {
            // Mark exported declarations as Public
            if let Some(decl) = node.child_by_field_name("declaration") {
                extract_from_node(decl, source, file, symbols, imports, next_id);
                // Mark the last added symbol as public
                if let Some(last) = symbols.last_mut() {
                    last.visibility = Visibility::Public;
                }
            } else {
                symbols.push(SymbolNode {
                    id: *next_id,
                    kind: SymbolKind::Export,
                    name: String::from("*"),
                    visibility: Visibility::Public,
                    file: file.to_string(),
                    trust_level: Default::default(),
                });
                *next_id += 1;
            }
        }
        "import_statement" => {
            if let Some(source_node) = node.child_by_field_name("source") {
                let raw = node_text(source_node, source);
                let module_path = raw.trim_matches(|c| c == '\'' || c == '"');
                imports.push(ImportEdge {
                    from_file: file.to_string(),
                    to_source: module_path.to_string(),
                });
            }
        }
        "lexical_declaration" => {
            // Check for arrow functions and require() calls
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i) {
                    if child.kind() == "variable_declarator" {
                        if let Some(name_node) = child.child_by_field_name("name") {
                            if let Some(value) = child.child_by_field_name("value") {
                                if value.kind() == "arrow_function" {
                                    let name = node_text(name_node, source);
                                    symbols.push(SymbolNode {
                                        id: *next_id,
                                        kind: SymbolKind::Function,
                                        name,
                                        visibility: Visibility::Internal,
                                        file: file.to_string(),
                                        trust_level: Default::default(),
                                    });
                                    *next_id += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    // Recurse into children (except for nodes we've already handled)
    if !matches!(
        node.kind(),
        "function_declaration"
            | "class_declaration"
            | "lexical_declaration"
    ) {
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i) {
                extract_from_node(child, source, file, symbols, imports, next_id);
            }
        }
    }
}

fn node_text(node: tree_sitter::Node, source: &[u8]) -> String {
    node.utf8_text(source).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn extracts_functions_from_typescript() {
        let source = b"
function greet(name: string): string {
    return name;
}

const add = (a: number, b: number) => a + b;
";
        let mut parser = Parser::new();
        let result = parser
            .parse_bytes(Path::new("test.ts"), source)
            .unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let fns: Vec<&str> = symbols
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .map(|s| s.name.as_str())
            .collect();

        assert!(fns.contains(&"greet"));
        assert!(fns.contains(&"add"));
    }

    #[test]
    fn extracts_classes() {
        let source = b"
class Greeter {
    greet() { return 'hello'; }
}
";
        let mut parser = Parser::new();
        let result = parser
            .parse_bytes(Path::new("test.ts"), source)
            .unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let classes: Vec<&str> = symbols
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .map(|s| s.name.as_str())
            .collect();

        assert!(classes.contains(&"Greeter"));
    }

    #[test]
    fn marks_exports_as_public() {
        let source = b"
export function greet() {}
function internal() {}
";
        let mut parser = Parser::new();
        let result = parser
            .parse_bytes(Path::new("test.ts"), source)
            .unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let greet = symbols
            .symbols
            .iter()
            .find(|s| s.name == "greet")
            .unwrap();
        let internal = symbols
            .symbols
            .iter()
            .find(|s| s.name == "internal")
            .unwrap();

        assert_eq!(greet.visibility, Visibility::Public);
        assert_eq!(internal.visibility, Visibility::Internal);
    }

    #[test]
    fn extracts_imports() {
        let source = b"
import { something } from './module';
import * as fs from 'node:fs';
";
        let mut parser = Parser::new();
        let result = parser
            .parse_bytes(Path::new("test.ts"), source)
            .unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let import_sources: Vec<&str> = symbols
            .imports
            .iter()
            .map(|i| i.to_source.as_str())
            .collect();

        assert!(import_sources.contains(&"./module"));
        assert!(import_sources.contains(&"node:fs"));
    }

    #[test]
    fn assigns_unique_ids() {
        let source = b"
function a() {}
function b() {}
class C {}
";
        let mut parser = Parser::new();
        let result = parser
            .parse_bytes(Path::new("test.ts"), source)
            .unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 100);
        let ids: Vec<u64> = symbols.symbols.iter().map(|s| s.id).collect();

        // All unique
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(ids.len(), unique.len());

        // Starting from offset
        assert!(ids.iter().all(|&id| id >= 100));
    }
}
```

- [ ] **Step 3: Update parser mod.rs to include extract module**

Add `pub mod extract;` to `crates/anvil-kernel/src/parser/mod.rs`.

- [ ] **Step 4: Run extraction tests**

Run: `cargo test -p eddacraft-anvil-kernel -- parser::extract`
Expected: all 5 tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/anvil-kernel/src/parser/
git commit -m "feat(kern): add symbol extraction from tree-sitter ASTs (KERN-012)"
```

---

### Task 5: Ignore patterns + git-aware filtering (KERN-013)

**Files:**
- Create: `crates/anvil-kernel/src/watcher/filter.rs`
- Modify: `crates/anvil-kernel/src/watcher/mod.rs`

- [ ] **Step 1: Implement file filter**

```rust
// crates/anvil-kernel/src/watcher/filter.rs
use std::path::Path;

/// Determines whether a file path should be processed or ignored.
pub struct FileFilter {
    ignore_patterns: Vec<String>,
}

impl FileFilter {
    pub fn new(ignore_patterns: Vec<String>) -> Self {
        Self { ignore_patterns }
    }

    /// Default ignore patterns for typical projects.
    pub fn default_patterns() -> Vec<String> {
        vec![
            "node_modules".to_string(),
            ".git".to_string(),
            "target".to_string(),
            "dist".to_string(),
            "build".to_string(),
            ".next".to_string(),
            ".turbo".to_string(),
            ".nx".to_string(),
            "coverage".to_string(),
            ".anvil".to_string(),
        ]
    }

    /// Check if a path should be ignored.
    pub fn should_ignore(&self, path: &Path) -> bool {
        for component in path.components() {
            let name = component.as_os_str().to_string_lossy();
            if self.ignore_patterns.iter().any(|p| p == name.as_ref()) {
                return true;
            }
        }
        false
    }

    /// Check if a file has a parseable extension.
    pub fn is_parseable(&self, path: &Path) -> bool {
        matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs")
        )
    }

    /// Combined check: not ignored AND has a parseable extension.
    pub fn should_process(&self, path: &Path) -> bool {
        !self.should_ignore(path) && self.is_parseable(path)
    }
}

impl Default for FileFilter {
    fn default() -> Self {
        Self::new(Self::default_patterns())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_node_modules() {
        let filter = FileFilter::default();
        assert!(filter.should_ignore(Path::new("node_modules/foo/bar.ts")));
        assert!(filter.should_ignore(Path::new("packages/core/node_modules/x.js")));
    }

    #[test]
    fn ignores_git_directory() {
        let filter = FileFilter::default();
        assert!(filter.should_ignore(Path::new(".git/objects/abc")));
    }

    #[test]
    fn ignores_build_outputs() {
        let filter = FileFilter::default();
        assert!(filter.should_ignore(Path::new("target/debug/anvil")));
        assert!(filter.should_ignore(Path::new("dist/index.js")));
        assert!(filter.should_ignore(Path::new("build/output.js")));
    }

    #[test]
    fn allows_source_files() {
        let filter = FileFilter::default();
        assert!(!filter.should_ignore(Path::new("src/main.ts")));
        assert!(!filter.should_ignore(Path::new("packages/core/src/lib.ts")));
    }

    #[test]
    fn detects_parseable_extensions() {
        let filter = FileFilter::default();
        assert!(filter.is_parseable(Path::new("main.ts")));
        assert!(filter.is_parseable(Path::new("App.tsx")));
        assert!(filter.is_parseable(Path::new("index.js")));
        assert!(filter.is_parseable(Path::new("config.mjs")));
        assert!(filter.is_parseable(Path::new("util.cjs")));
        assert!(!filter.is_parseable(Path::new("README.md")));
        assert!(!filter.is_parseable(Path::new("Cargo.toml")));
    }

    #[test]
    fn should_process_combines_checks() {
        let filter = FileFilter::default();
        assert!(filter.should_process(Path::new("src/main.ts")));
        assert!(!filter.should_process(Path::new("node_modules/foo.ts")));
        assert!(!filter.should_process(Path::new("src/README.md")));
    }

    #[test]
    fn custom_patterns() {
        let filter = FileFilter::new(vec!["vendor".to_string(), "tmp".to_string()]);
        assert!(filter.should_ignore(Path::new("vendor/lib.ts")));
        assert!(filter.should_ignore(Path::new("tmp/scratch.ts")));
        assert!(!filter.should_ignore(Path::new("node_modules/x.ts")));
    }
}
```

- [ ] **Step 2: Update watcher mod.rs to export filter**

Add `pub mod filter;` to `crates/anvil-kernel/src/watcher/mod.rs`.

- [ ] **Step 3: Run filter tests**

Run: `cargo test -p eddacraft-anvil-kernel -- watcher::filter`
Expected: all 7 tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/anvil-kernel/src/watcher/filter.rs crates/anvil-kernel/src/watcher/mod.rs
git commit -m "feat(kern): add ignore patterns and file filtering (KERN-013)"
```

---

### Task 6: Rust CI pipeline (KERN-005)

**Files:**
- Create: `.github/workflows/rust.yml`

- [ ] **Step 1: Create CI workflow**

```yaml
# .github/workflows/rust.yml
name: Rust

on:
  push:
    branches: [main, dev, 'rust-*']
    paths:
      - 'crates/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
      - '.github/workflows/rust.yml'
  pull_request:
    paths:
      - 'crates/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
      - '.github/workflows/rust.yml'

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -Dwarnings

jobs:
  check:
    name: Check + Clippy + Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets --all-features
      - run: cargo test --all
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/rust.yml
git commit -m "ci(kern): add Rust CI pipeline for cargo test, clippy, fmt (KERN-005)"
```

---

## Chunk 2: Track B — TUI Surface Ports

### Task 7: Create `anvil-tui` crate scaffold

**Files:**
- Create: `crates/anvil-tui/Cargo.toml`
- Create: `crates/anvil-tui/src/lib.rs`
- Create: `crates/anvil-tui/src/surfaces/mod.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Create crate directory and Cargo.toml**

```toml
# crates/anvil-tui/Cargo.toml
[package]
name = "anvil-tui"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Anvil TUI surfaces built on eddacraft-tui — watch dashboard, gate explorer, wizard"

[dependencies]
eddacraft-tui = { path = "../eddacraft-tui" }
anvil-kernel-types = { path = "../anvil-kernel-types" }
ratatui = { workspace = true }
crossterm = { workspace = true }

[dev-dependencies]
insta = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 2: Create lib.rs and module stubs**

```rust
// crates/anvil-tui/src/lib.rs
pub mod surfaces;
```

```rust
// crates/anvil-tui/src/surfaces/mod.rs
pub mod welcome;
pub mod doctor;
pub mod status;
```

- [ ] **Step 3: Add to workspace members**

Add `"crates/anvil-tui"` to the `members` array in root `Cargo.toml`.

- [ ] **Step 4: Verify build**

Run: `cargo check -p eddacraft-anvil-tui`
Expected: compiles with no errors

- [ ] **Step 5: Commit**

```bash
git add crates/anvil-tui/ Cargo.toml
git commit -m "chore(rats): scaffold anvil-tui crate for TUI surface ports"
```

---

### Task 8: Port welcome surface (PORT-010)

**Files:**
- Create: `crates/anvil-tui/src/surfaces/welcome/mod.rs`
- Create: `crates/anvil-tui/src/surfaces/welcome/render.rs`

The welcome surface is the simplest — static content with a quick-start menu.
No service dependencies, no live data.

**Ink reference:** `apps/anvil-cli/src/tui/commands/welcome/Welcome.tsx`

- [ ] **Step 1: Define welcome types and state**

```rust
// crates/anvil-tui/src/surfaces/welcome/mod.rs
pub mod render;

use eddacraft_tui::prelude::*;

/// Quick-start options shown on the welcome screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickStartOption {
    RunGate,
    StartWatch,
    RunTutorial,
    ViewDocs,
}

impl QuickStartOption {
    pub const ALL: [Self; 4] = [
        Self::RunGate,
        Self::StartWatch,
        Self::RunTutorial,
        Self::ViewDocs,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::RunGate => "Run gate checks",
            Self::StartWatch => "Start watch mode",
            Self::RunTutorial => "Run interactive tutorial",
            Self::ViewDocs => "View documentation",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::RunGate => "Check your project against configured quality gates",
            Self::StartWatch => "Monitor files and run checks on every save",
            Self::RunTutorial => "Learn Anvil with a guided walkthrough",
            Self::ViewDocs => "Open the Anvil documentation in your browser",
        }
    }
}

/// State for the welcome surface.
pub struct WelcomeState {
    pub selected: usize,
    pub should_quit: bool,
    pub chosen: Option<QuickStartOption>,
}

impl WelcomeState {
    pub fn new() -> Self {
        Self {
            selected: 0,
            should_quit: false,
            chosen: None,
        }
    }

    pub fn handle_key(&mut self, action: Action) {
        match action {
            Action::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            Action::Down => {
                if self.selected < QuickStartOption::ALL.len() - 1 {
                    self.selected += 1;
                }
            }
            Action::Select => {
                self.chosen = Some(QuickStartOption::ALL[self.selected]);
            }
            Action::Quit => {
                self.should_quit = true;
            }
            _ => {}
        }
    }
}

impl Default for WelcomeState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let state = WelcomeState::new();
        assert_eq!(state.selected, 0);
        assert!(!state.should_quit);
        assert!(state.chosen.is_none());
    }

    #[test]
    fn navigate_down_and_up() {
        let mut state = WelcomeState::new();
        state.handle_key(Action::Down);
        assert_eq!(state.selected, 1);
        state.handle_key(Action::Down);
        assert_eq!(state.selected, 2);
        state.handle_key(Action::Up);
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn does_not_go_below_zero() {
        let mut state = WelcomeState::new();
        state.handle_key(Action::Up);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn does_not_exceed_max() {
        let mut state = WelcomeState::new();
        for _ in 0..10 {
            state.handle_key(Action::Down);
        }
        assert_eq!(state.selected, QuickStartOption::ALL.len() - 1);
    }

    #[test]
    fn select_sets_chosen() {
        let mut state = WelcomeState::new();
        state.handle_key(Action::Down); // StartWatch
        state.handle_key(Action::Select);
        assert_eq!(state.chosen, Some(QuickStartOption::StartWatch));
    }

    #[test]
    fn quit_sets_flag() {
        let mut state = WelcomeState::new();
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }
}
```

- [ ] **Step 2: Run welcome state tests**

Run: `cargo test -p eddacraft-anvil-tui -- surfaces::welcome`
Expected: all 6 tests pass

- [ ] **Step 3: Implement welcome render**

```rust
// crates/anvil-tui/src/surfaces/welcome/render.rs
use eddacraft_tui::prelude::*;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::{QuickStartOption, WelcomeState};

const LOGO: &str = r"
   _____              .__.__
  /  _  \   _______  _|__|  |
 /  /_\  \ /    \  \/ /  |  |
/    |    \   |  \   /|  |  |__
\____|__  /___|  /\_/ |__|____/
        \/     \/
";

const TAGLINE: &str = "Structural governance for AI-assisted development";

pub fn render(frame: &mut Frame, area: Rect, state: &WelcomeState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Length(8),  // Logo
        Constraint::Length(2),  // Tagline
        Constraint::Length(1),  // Spacer
        Constraint::Min(6),    // Menu
        Constraint::Length(2),  // Help text
    ])
    .split(area);

    // Logo
    let logo = Paragraph::new(Text::raw(LOGO))
        .style(Style::default().fg(theme.accent()));
    frame.render_widget(logo, chunks[0]);

    // Tagline
    let tagline = Paragraph::new(TAGLINE)
        .style(Style::default().fg(theme.muted()));
    frame.render_widget(tagline, chunks[1]);

    // Menu
    let menu_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.muted()))
        .title(" Quick Start ");

    let menu_area = menu_block.inner(chunks[3]);
    frame.render_widget(menu_block, chunks[3]);

    let items: Vec<Line> = QuickStartOption::ALL
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let indicator = if i == state.selected { "▸ " } else { "  " };
            let label_style = if i == state.selected {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };
            let desc_style = Style::default().fg(theme.muted());

            Line::from(vec![
                Span::styled(indicator, label_style),
                Span::styled(opt.label(), label_style),
                Span::styled("  ", Style::default()),
                Span::styled(opt.description(), desc_style),
            ])
        })
        .collect();

    let menu = Paragraph::new(Text::from(items));
    frame.render_widget(menu, menu_area);

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("j/k", Style::default().fg(theme.accent())),
        Span::styled(" navigate  ", Style::default().fg(theme.muted())),
        Span::styled("enter", Style::default().fg(theme.accent())),
        Span::styled(" select  ", Style::default().fg(theme.muted())),
        Span::styled("q", Style::default().fg(theme.accent())),
        Span::styled(" quit", Style::default().fg(theme.muted())),
    ]));
    frame.render_widget(help, chunks[4]);
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p eddacraft-anvil-tui`
Expected: compiles with no errors

- [ ] **Step 5: Commit**

```bash
git add crates/anvil-tui/src/surfaces/welcome/
git commit -m "feat(port): add welcome surface port to Ratatui (PORT-010)"
```

---

### Task 9: Port doctor surface (PORT-011)

**Files:**
- Create: `crates/anvil-tui/src/surfaces/doctor/mod.rs`
- Create: `crates/anvil-tui/src/surfaces/doctor/render.rs`

The doctor surface displays diagnostic check results with pass/fail/warn status.
Receives data as input — no service coupling.

**Ink reference:** `apps/anvil-cli/src/tui/commands/doctor/Diagnostics.tsx`

- [ ] **Step 1: Define doctor types and state**

```rust
// crates/anvil-tui/src/surfaces/doctor/mod.rs
pub mod render;

use eddacraft_tui::prelude::*;

/// Status of a single diagnostic check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Fail,
    Warn,
    Skipped,
    Running,
}

impl CheckStatus {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Pass => "◆",
            Self::Fail => "✖",
            Self::Warn => "◈",
            Self::Skipped => "○",
            Self::Running => "●",
        }
    }
}

/// A single diagnostic check result.
#[derive(Debug, Clone)]
pub struct DiagnosticCheck {
    pub name: String,
    pub category: String,
    pub status: CheckStatus,
    pub message: String,
    pub details: Option<String>,
    pub auto_fixable: bool,
}

/// Aggregate summary of all checks.
#[derive(Debug, Clone, Default)]
pub struct DiagnosticSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
    pub skipped: usize,
}

impl DiagnosticSummary {
    pub fn from_checks(checks: &[DiagnosticCheck]) -> Self {
        let mut summary = Self {
            total: checks.len(),
            ..Default::default()
        };
        for check in checks {
            match check.status {
                CheckStatus::Pass => summary.passed += 1,
                CheckStatus::Fail => summary.failed += 1,
                CheckStatus::Warn => summary.warnings += 1,
                CheckStatus::Skipped => summary.skipped += 1,
                CheckStatus::Running => {}
            }
        }
        summary
    }
}

/// State for the doctor surface.
pub struct DoctorState {
    pub checks: Vec<DiagnosticCheck>,
    pub selected: usize,
    pub expanded: bool,
    pub should_quit: bool,
}

impl DoctorState {
    pub fn new(checks: Vec<DiagnosticCheck>) -> Self {
        Self {
            checks,
            selected: 0,
            expanded: false,
            should_quit: false,
        }
    }

    pub fn summary(&self) -> DiagnosticSummary {
        DiagnosticSummary::from_checks(&self.checks)
    }

    pub fn handle_key(&mut self, action: Action) {
        match action {
            Action::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.expanded = false;
                }
            }
            Action::Down => {
                if self.selected < self.checks.len().saturating_sub(1) {
                    self.selected += 1;
                    self.expanded = false;
                }
            }
            Action::Select => {
                self.expanded = !self.expanded;
            }
            Action::Quit => {
                self.should_quit = true;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_checks() -> Vec<DiagnosticCheck> {
        vec![
            DiagnosticCheck {
                name: "Node.js".to_string(),
                category: "Runtime".to_string(),
                status: CheckStatus::Pass,
                message: "v22.0.0 found".to_string(),
                details: Some("Path: /usr/bin/node".to_string()),
                auto_fixable: false,
            },
            DiagnosticCheck {
                name: "ESLint config".to_string(),
                category: "Linting".to_string(),
                status: CheckStatus::Fail,
                message: "No .eslintrc found".to_string(),
                details: Some("Run `npx eslint --init` to create one".to_string()),
                auto_fixable: true,
            },
            DiagnosticCheck {
                name: "Git hooks".to_string(),
                category: "Hooks".to_string(),
                status: CheckStatus::Warn,
                message: "Hooks not installed".to_string(),
                details: None,
                auto_fixable: true,
            },
        ]
    }

    #[test]
    fn summary_counts_correctly() {
        let checks = sample_checks();
        let summary = DiagnosticSummary::from_checks(&checks);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.warnings, 1);
    }

    #[test]
    fn navigate_and_expand() {
        let mut state = DoctorState::new(sample_checks());
        assert_eq!(state.selected, 0);
        assert!(!state.expanded);

        state.handle_key(Action::Select);
        assert!(state.expanded);

        state.handle_key(Action::Down);
        assert_eq!(state.selected, 1);
        assert!(!state.expanded); // collapsed on navigation
    }

    #[test]
    fn bounds_checking() {
        let mut state = DoctorState::new(sample_checks());
        state.handle_key(Action::Up); // already at 0
        assert_eq!(state.selected, 0);

        for _ in 0..10 {
            state.handle_key(Action::Down);
        }
        assert_eq!(state.selected, 2); // max index
    }
}
```

- [ ] **Step 2: Run doctor tests**

Run: `cargo test -p eddacraft-anvil-tui -- surfaces::doctor`
Expected: all 3 tests pass

- [ ] **Step 3: Implement doctor render**

```rust
// crates/anvil-tui/src/surfaces/doctor/render.rs
use eddacraft_tui::prelude::*;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::{CheckStatus, DoctorState};

pub fn render(frame: &mut Frame, area: Rect, state: &DoctorState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Length(3),  // Header + summary
        Constraint::Min(4),    // Check list
        Constraint::Length(4),  // Detail panel (when expanded)
        Constraint::Length(2),  // Help text
    ])
    .split(area);

    // Summary header
    let summary = state.summary();
    let summary_line = Line::from(vec![
        Span::styled("Diagnostics  ", Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("◆ {} passed", summary.passed),
            Style::default().fg(theme.success()),
        ),
        Span::styled("  ", Style::default()),
        Span::styled(
            format!("✖ {} failed", summary.failed),
            Style::default().fg(theme.error()),
        ),
        Span::styled("  ", Style::default()),
        Span::styled(
            format!("◈ {} warnings", summary.warnings),
            Style::default().fg(theme.warning()),
        ),
    ]);
    frame.render_widget(Paragraph::new(summary_line), chunks[0]);

    // Check list
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.muted()))
        .title(" Checks ");

    let list_area = list_block.inner(chunks[1]);
    frame.render_widget(list_block, chunks[1]);

    let items: Vec<Line> = state
        .checks
        .iter()
        .enumerate()
        .map(|(i, check)| {
            let selected = i == state.selected;
            let indicator = if selected { "▸ " } else { "  " };
            let icon_colour = match check.status {
                CheckStatus::Pass => theme.success(),
                CheckStatus::Fail => theme.error(),
                CheckStatus::Warn => theme.warning(),
                CheckStatus::Skipped => theme.muted(),
                CheckStatus::Running => theme.accent(),
            };
            let name_style = if selected {
                Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };

            Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(
                    format!("{} ", check.status.icon()),
                    Style::default().fg(icon_colour),
                ),
                Span::styled(&check.name, name_style),
                Span::styled(
                    format!("  [{}]", check.category),
                    Style::default().fg(theme.muted()),
                ),
                Span::styled(
                    format!("  {}", check.message),
                    Style::default().fg(theme.muted()),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(items)), list_area);

    // Detail panel (when expanded)
    if state.expanded {
        if let Some(check) = state.checks.get(state.selected) {
            let detail_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent()))
                .title(format!(" {} ", check.name));

            let detail_area = detail_block.inner(chunks[2]);
            frame.render_widget(detail_block, chunks[2]);

            let detail_text = check
                .details
                .as_deref()
                .unwrap_or("No additional details");
            let mut lines = vec![Line::from(Span::styled(
                detail_text,
                Style::default().fg(theme.fg()),
            ))];
            if check.auto_fixable {
                lines.push(Line::from(Span::styled(
                    "Auto-fixable: press 'f' to fix",
                    Style::default().fg(theme.accent()),
                )));
            }
            frame.render_widget(Paragraph::new(Text::from(lines)), detail_area);
        }
    }

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("j/k", Style::default().fg(theme.accent())),
        Span::styled(" navigate  ", Style::default().fg(theme.muted())),
        Span::styled("enter", Style::default().fg(theme.accent())),
        Span::styled(" details  ", Style::default().fg(theme.muted())),
        Span::styled("q", Style::default().fg(theme.accent())),
        Span::styled(" quit", Style::default().fg(theme.muted())),
    ]));
    frame.render_widget(help, chunks[3]);
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p eddacraft-anvil-tui`
Expected: compiles with no errors

- [ ] **Step 5: Commit**

```bash
git add crates/anvil-tui/src/surfaces/doctor/
git commit -m "feat(port): add doctor diagnostics surface port to Ratatui (PORT-011)"
```

---

### Task 10: Port status dashboard surface (PORT-012)

**Files:**
- Create: `crates/anvil-tui/src/surfaces/status/mod.rs`
- Create: `crates/anvil-tui/src/surfaces/status/render.rs`

The status dashboard shows three panels: hooks, profile, and results.
Receives `StatusData` as input — no service coupling.

**Ink reference:** `apps/anvil-cli/src/tui/commands/status/StatusDashboard.tsx`

- [ ] **Step 1: Define status types and state**

```rust
// crates/anvil-tui/src/surfaces/status/mod.rs
pub mod render;

use eddacraft_tui::prelude::*;

/// Status of a single hook.
#[derive(Debug, Clone)]
pub struct HookStatus {
    pub name: String,
    pub active: bool,
    pub path: String,
}

/// Configuration profile information.
#[derive(Debug, Clone)]
pub struct ProfileInfo {
    pub name: String,
    pub checks: Vec<String>,
    pub path: String,
}

/// Result of a recent gate run.
#[derive(Debug, Clone)]
pub struct GateRunResult {
    pub timestamp: String,
    pub passed: bool,
    pub score: f64,
    pub checks_run: usize,
    pub checks_passed: usize,
    pub duration_ms: u64,
}

/// All data needed by the status dashboard.
#[derive(Debug, Clone)]
pub struct StatusData {
    pub hooks: Vec<HookStatus>,
    pub profile: ProfileInfo,
    pub recent_runs: Vec<GateRunResult>,
}

/// Which panel is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusPanel {
    Hooks,
    Profile,
    Results,
}

impl StatusPanel {
    pub fn next(self) -> Self {
        match self {
            Self::Hooks => Self::Profile,
            Self::Profile => Self::Results,
            Self::Results => Self::Hooks,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Hooks => Self::Results,
            Self::Profile => Self::Hooks,
            Self::Results => Self::Profile,
        }
    }
}

/// State for the status dashboard surface.
pub struct StatusState {
    pub data: StatusData,
    pub focused_panel: StatusPanel,
    pub selected_item: usize,
    pub should_quit: bool,
}

impl StatusState {
    pub fn new(data: StatusData) -> Self {
        Self {
            data,
            focused_panel: StatusPanel::Hooks,
            selected_item: 0,
            should_quit: false,
        }
    }

    fn max_items_in_panel(&self) -> usize {
        match self.focused_panel {
            StatusPanel::Hooks => self.data.hooks.len(),
            StatusPanel::Profile => self.data.profile.checks.len(),
            StatusPanel::Results => self.data.recent_runs.len(),
        }
    }

    pub fn handle_key(&mut self, action: Action) {
        match action {
            Action::Up => {
                if self.selected_item > 0 {
                    self.selected_item -= 1;
                }
            }
            Action::Down => {
                let max = self.max_items_in_panel().saturating_sub(1);
                if self.selected_item < max {
                    self.selected_item += 1;
                }
            }
            Action::Right | Action::PageDown => {
                self.focused_panel = self.focused_panel.next();
                self.selected_item = 0;
            }
            Action::Left | Action::PageUp => {
                self.focused_panel = self.focused_panel.prev();
                self.selected_item = 0;
            }
            Action::Quit => {
                self.should_quit = true;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> StatusData {
        StatusData {
            hooks: vec![
                HookStatus {
                    name: "pre-commit".to_string(),
                    active: true,
                    path: ".husky/pre-commit".to_string(),
                },
                HookStatus {
                    name: "commit-msg".to_string(),
                    active: false,
                    path: ".husky/commit-msg".to_string(),
                },
            ],
            profile: ProfileInfo {
                name: "dev".to_string(),
                checks: vec![
                    "eslint".to_string(),
                    "secret-scan".to_string(),
                    "architecture".to_string(),
                ],
                path: ".anvil/profiles/dev.yaml".to_string(),
            },
            recent_runs: vec![GateRunResult {
                timestamp: "2026-03-16T10:00:00Z".to_string(),
                passed: true,
                score: 0.95,
                checks_run: 5,
                checks_passed: 5,
                duration_ms: 2400,
            }],
        }
    }

    #[test]
    fn panel_navigation() {
        let mut state = StatusState::new(sample_data());
        assert_eq!(state.focused_panel, StatusPanel::Hooks);

        state.handle_key(Action::Right);
        assert_eq!(state.focused_panel, StatusPanel::Profile);
        assert_eq!(state.selected_item, 0); // reset on switch

        state.handle_key(Action::PageDown);
        assert_eq!(state.focused_panel, StatusPanel::Results);

        state.handle_key(Action::Right);
        assert_eq!(state.focused_panel, StatusPanel::Hooks); // wraps
    }

    #[test]
    fn item_navigation_within_panel() {
        let mut state = StatusState::new(sample_data());
        // Hooks panel has 2 items
        state.handle_key(Action::Down);
        assert_eq!(state.selected_item, 1);
        state.handle_key(Action::Down); // at max
        assert_eq!(state.selected_item, 1);
        state.handle_key(Action::Up);
        assert_eq!(state.selected_item, 0);
    }

    #[test]
    fn panel_switch_resets_selection() {
        let mut state = StatusState::new(sample_data());
        state.handle_key(Action::Down);
        assert_eq!(state.selected_item, 1);

        state.handle_key(Action::Right); // switch to Profile
        assert_eq!(state.selected_item, 0);
    }
}
```

- [ ] **Step 2: Run status tests**

Run: `cargo test -p eddacraft-anvil-tui -- surfaces::status`
Expected: all 3 tests pass

- [ ] **Step 3: Implement status render**

```rust
// crates/anvil-tui/src/surfaces/status/render.rs
use eddacraft_tui::prelude::*;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::{StatusPanel, StatusState};

pub fn render(frame: &mut Frame, area: Rect, state: &StatusState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Length(1),  // Title
        Constraint::Ratio(1, 3), // Hooks panel
        Constraint::Ratio(1, 3), // Profile panel
        Constraint::Ratio(1, 3), // Results panel
        Constraint::Length(2),  // Help text
    ])
    .split(area);

    // Title
    let title = Paragraph::new(Line::from(Span::styled(
        "Anvil Status",
        Style::default()
            .fg(theme.fg())
            .add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(title, chunks[0]);

    // Hooks panel
    render_hooks_panel(frame, chunks[1], state, theme);

    // Profile panel
    render_profile_panel(frame, chunks[2], state, theme);

    // Results panel
    render_results_panel(frame, chunks[3], state, theme);

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("j/k", Style::default().fg(theme.accent())),
        Span::styled(" navigate  ", Style::default().fg(theme.muted())),
        Span::styled("h/l", Style::default().fg(theme.accent())),
        Span::styled(" switch panel  ", Style::default().fg(theme.muted())),
        Span::styled("q", Style::default().fg(theme.accent())),
        Span::styled(" quit", Style::default().fg(theme.muted())),
    ]));
    frame.render_widget(help, chunks[4]);
}

fn panel_block<'a>(title: &'a str, focused: bool, theme: &EddaCraftTheme) -> Block<'a> {
    let border_colour = if focused { theme.accent() } else { theme.muted() };
    let border_style = if focused {
        Borders::ALL
    } else {
        Borders::ALL
    };
    Block::default()
        .borders(border_style)
        .border_style(Style::default().fg(border_colour))
        .title(format!(" {title} "))
        .title_style(if focused {
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.muted())
        })
}

fn render_hooks_panel(frame: &mut Frame, area: Rect, state: &StatusState, theme: &EddaCraftTheme) {
    let focused = state.focused_panel == StatusPanel::Hooks;
    let block = panel_block("Hooks", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = state
        .data
        .hooks
        .iter()
        .enumerate()
        .map(|(i, hook)| {
            let selected = focused && i == state.selected_item;
            let indicator = if selected { "▸ " } else { "  " };
            let status_icon = if hook.active { "◆" } else { "○" };
            let status_colour = if hook.active {
                theme.success()
            } else {
                theme.muted()
            };

            Line::from(vec![
                Span::styled(indicator, Style::default().fg(theme.fg())),
                Span::styled(
                    format!("{status_icon} "),
                    Style::default().fg(status_colour),
                ),
                Span::styled(
                    &hook.name,
                    if selected {
                        Style::default()
                            .fg(theme.fg())
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg())
                    },
                ),
                Span::styled(
                    format!("  {}", hook.path),
                    Style::default().fg(theme.muted()),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_profile_panel(
    frame: &mut Frame,
    area: Rect,
    state: &StatusState,
    theme: &EddaCraftTheme,
) {
    let focused = state.focused_panel == StatusPanel::Profile;
    let block = panel_block("Profile", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![Line::from(vec![
        Span::styled("Active: ", Style::default().fg(theme.muted())),
        Span::styled(
            &state.data.profile.name,
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    for (i, check) in state.data.profile.checks.iter().enumerate() {
        let selected = focused && i == state.selected_item;
        let indicator = if selected { "▸ " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(indicator, Style::default().fg(theme.fg())),
            Span::styled(
                format!("◆ {check}"),
                if selected {
                    Style::default()
                        .fg(theme.fg())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg())
                },
            ),
        ]));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_results_panel(
    frame: &mut Frame,
    area: Rect,
    state: &StatusState,
    theme: &EddaCraftTheme,
) {
    let focused = state.focused_panel == StatusPanel::Results;
    let block = panel_block("Recent Runs", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = state
        .data
        .recent_runs
        .iter()
        .enumerate()
        .map(|(i, run)| {
            let selected = focused && i == state.selected_item;
            let indicator = if selected { "▸ " } else { "  " };
            let status_icon = if run.passed { "◆" } else { "✖" };
            let status_colour = if run.passed {
                theme.success()
            } else {
                theme.error()
            };

            Line::from(vec![
                Span::styled(indicator, Style::default().fg(theme.fg())),
                Span::styled(
                    format!("{status_icon} "),
                    Style::default().fg(status_colour),
                ),
                Span::styled(
                    format!(
                        "{}/{} checks  ",
                        run.checks_passed, run.checks_run
                    ),
                    if selected {
                        Style::default()
                            .fg(theme.fg())
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg())
                    },
                ),
                Span::styled(
                    format!("{}ms  ", run.duration_ms),
                    Style::default().fg(theme.muted()),
                ),
                Span::styled(&run.timestamp, Style::default().fg(theme.muted())),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p eddacraft-anvil-tui`
Expected: compiles with no errors

- [ ] **Step 5: Commit**

```bash
git add crates/anvil-tui/src/surfaces/status/
git commit -m "feat(port): add status dashboard surface port to Ratatui (PORT-012)"
```

---

### Task 11: APS onboarding wizard (RATS-004)

**Files:**
- Create: `crates/anvil-tui/src/surfaces/wizard/mod.rs`
- Create: `crates/anvil-tui/src/surfaces/wizard/render.rs`
- Modify: `crates/anvil-tui/src/surfaces/mod.rs`

The APS onboarding wizard is a new Ratatui surface (not an Ink port).
Multi-step flow: template selection → configuration → scaffold generation.

- [ ] **Step 1: Define wizard types and state**

```rust
// crates/anvil-tui/src/surfaces/wizard/mod.rs
pub mod render;

use eddacraft_tui::prelude::*;

/// Template available for scaffolding.
#[derive(Debug, Clone)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
}

/// Wizard step progression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    TemplateSelect,
    ProjectName,
    Configure,
    Summary,
}

impl WizardStep {
    pub fn label(self) -> &'static str {
        match self {
            Self::TemplateSelect => "Select Template",
            Self::ProjectName => "Project Name",
            Self::Configure => "Configure",
            Self::Summary => "Summary",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::TemplateSelect => 0,
            Self::ProjectName => 1,
            Self::Configure => 2,
            Self::Summary => 3,
        }
    }

    pub const TOTAL: usize = 4;

    pub fn next(self) -> Option<Self> {
        match self {
            Self::TemplateSelect => Some(Self::ProjectName),
            Self::ProjectName => Some(Self::Configure),
            Self::Configure => Some(Self::Summary),
            Self::Summary => None,
        }
    }

    pub fn prev(self) -> Option<Self> {
        match self {
            Self::TemplateSelect => None,
            Self::ProjectName => Some(Self::TemplateSelect),
            Self::Configure => Some(Self::ProjectName),
            Self::Summary => Some(Self::Configure),
        }
    }
}

/// Configuration options set during the wizard.
#[derive(Debug, Clone, Default)]
pub struct WizardConfig {
    pub project_name: String,
    pub template_id: Option<String>,
    pub enable_watch: bool,
    pub enable_hooks: bool,
}

/// State for the APS onboarding wizard.
pub struct WizardState {
    pub step: WizardStep,
    pub templates: Vec<Template>,
    pub template_selected: usize,
    pub config: WizardConfig,
    pub text_input: TextInputState,
    pub should_quit: bool,
    pub confirmed: bool,
}

impl WizardState {
    pub fn new(templates: Vec<Template>) -> Self {
        Self {
            step: WizardStep::TemplateSelect,
            templates,
            template_selected: 0,
            config: WizardConfig::default(),
            text_input: TextInputState::default(),
            should_quit: false,
            confirmed: false,
        }
    }

    pub fn handle_key(&mut self, action: Action) {
        match self.step {
            WizardStep::TemplateSelect => self.handle_template_key(action),
            WizardStep::ProjectName => self.handle_name_key(action),
            WizardStep::Configure => self.handle_configure_key(action),
            WizardStep::Summary => self.handle_summary_key(action),
        }
    }

    fn handle_template_key(&mut self, action: Action) {
        match action {
            Action::Up => {
                if self.template_selected > 0 {
                    self.template_selected -= 1;
                }
            }
            Action::Down => {
                if self.template_selected < self.templates.len().saturating_sub(1) {
                    self.template_selected += 1;
                }
            }
            Action::Select => {
                if let Some(t) = self.templates.get(self.template_selected) {
                    self.config.template_id = Some(t.id.clone());
                    self.step = WizardStep::ProjectName;
                }
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_name_key(&mut self, action: Action) {
        match action {
            Action::Select => {
                self.config.project_name = self.text_input.value.clone();
                if !self.config.project_name.is_empty() {
                    self.step = WizardStep::Configure;
                }
            }
            Action::Back => {
                self.step = WizardStep::TemplateSelect;
            }
            Action::Quit => self.should_quit = true,
            _ => {
                // Text input handles character input
            }
        }
    }

    fn handle_configure_key(&mut self, action: Action) {
        match action {
            Action::Select => {
                self.step = WizardStep::Summary;
            }
            Action::Back => {
                self.step = WizardStep::ProjectName;
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_summary_key(&mut self, action: Action) {
        match action {
            Action::Select => {
                self.confirmed = true;
            }
            Action::Back => {
                self.step = WizardStep::Configure;
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_templates() -> Vec<Template> {
        vec![
            Template {
                id: "basic".to_string(),
                name: "Basic".to_string(),
                description: "Minimal setup with core checks".to_string(),
                tags: vec!["starter".to_string()],
            },
            Template {
                id: "full".to_string(),
                name: "Full".to_string(),
                description: "All checks, hooks, and watch mode".to_string(),
                tags: vec!["recommended".to_string()],
            },
        ]
    }

    #[test]
    fn starts_at_template_select() {
        let state = WizardState::new(sample_templates());
        assert_eq!(state.step, WizardStep::TemplateSelect);
    }

    #[test]
    fn template_selection_advances_to_name() {
        let mut state = WizardState::new(sample_templates());
        state.handle_key(Action::Select);
        assert_eq!(state.step, WizardStep::ProjectName);
        assert_eq!(state.config.template_id, Some("basic".to_string()));
    }

    #[test]
    fn back_navigation() {
        let mut state = WizardState::new(sample_templates());
        state.step = WizardStep::Configure;
        state.handle_key(Action::Back);
        assert_eq!(state.step, WizardStep::ProjectName);
    }

    #[test]
    fn summary_confirm() {
        let mut state = WizardState::new(sample_templates());
        state.step = WizardStep::Summary;
        state.handle_key(Action::Select);
        assert!(state.confirmed);
    }

    #[test]
    fn step_progression() {
        assert_eq!(
            WizardStep::TemplateSelect.next(),
            Some(WizardStep::ProjectName)
        );
        assert_eq!(
            WizardStep::ProjectName.next(),
            Some(WizardStep::Configure)
        );
        assert_eq!(WizardStep::Configure.next(), Some(WizardStep::Summary));
        assert_eq!(WizardStep::Summary.next(), None);
    }

    #[test]
    fn step_regression() {
        assert_eq!(WizardStep::TemplateSelect.prev(), None);
        assert_eq!(
            WizardStep::ProjectName.prev(),
            Some(WizardStep::TemplateSelect)
        );
    }
}
```

- [ ] **Step 2: Run wizard tests**

Run: `cargo test -p eddacraft-anvil-tui -- surfaces::wizard`
Expected: all 6 tests pass

- [ ] **Step 3: Add wizard render stub**

```rust
// crates/anvil-tui/src/surfaces/wizard/render.rs
use eddacraft_tui::prelude::*;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::{WizardState, WizardStep};

pub fn render(frame: &mut Frame, area: Rect, state: &WizardState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Progress bar
        Constraint::Min(6),   // Step content
        Constraint::Length(2), // Help text
    ])
    .split(area);

    // Step progress indicator
    render_progress(frame, chunks[0], state, theme);

    // Step content
    match state.step {
        WizardStep::TemplateSelect => render_template_step(frame, chunks[1], state, theme),
        WizardStep::ProjectName => render_name_step(frame, chunks[1], state, theme),
        WizardStep::Configure => render_configure_step(frame, chunks[1], state, theme),
        WizardStep::Summary => render_summary_step(frame, chunks[1], state, theme),
    }

    // Help text
    let help_text = match state.step {
        WizardStep::TemplateSelect => "j/k navigate  enter select  q quit",
        WizardStep::ProjectName => "type name  enter confirm  esc back  q quit",
        WizardStep::Configure => "enter next  esc back  q quit",
        WizardStep::Summary => "enter confirm  esc back  q quit",
    };
    let help = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(theme.muted()),
    )));
    frame.render_widget(help, chunks[2]);
}

fn render_progress(frame: &mut Frame, area: Rect, state: &WizardState, theme: &EddaCraftTheme) {
    let steps: Vec<Span> = (0..WizardStep::TOTAL)
        .map(|i| {
            let label = match i {
                0 => "Template",
                1 => "Name",
                2 => "Configure",
                3 => "Summary",
                _ => "",
            };
            let style = if i == state.step.index() {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else if i < state.step.index() {
                Style::default().fg(theme.success())
            } else {
                Style::default().fg(theme.muted())
            };
            let separator = if i < WizardStep::TOTAL - 1 {
                " → "
            } else {
                ""
            };
            vec![
                Span::styled(label, style),
                Span::styled(separator, Style::default().fg(theme.muted())),
            ]
        })
        .flatten()
        .collect();

    frame.render_widget(Paragraph::new(Line::from(steps)), area);
}

fn render_template_step(
    frame: &mut Frame,
    area: Rect,
    state: &WizardState,
    theme: &EddaCraftTheme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Select a Template ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<Line> = state
        .templates
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let selected = i == state.template_selected;
            let indicator = if selected { "▸ " } else { "  " };
            let name_style = if selected {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };

            Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(&t.name, name_style),
                Span::styled(
                    format!("  {}", t.description),
                    Style::default().fg(theme.muted()),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(items)), inner);
}

fn render_name_step(
    frame: &mut Frame,
    area: Rect,
    state: &WizardState,
    theme: &EddaCraftTheme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Project Name ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let prompt = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            "Enter a name for your project:",
            Style::default().fg(theme.fg()),
        )),
        Line::from(Span::styled(
            format!("▸ {}_", state.text_input.value),
            Style::default().fg(theme.accent()),
        )),
    ]));
    frame.render_widget(prompt, inner);
}

fn render_configure_step(
    frame: &mut Frame,
    area: Rect,
    state: &WizardState,
    theme: &EddaCraftTheme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Configure ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let watch_icon = if state.config.enable_watch { "◆" } else { "○" };
    let hooks_icon = if state.config.enable_hooks { "◆" } else { "○" };

    let content = Paragraph::new(Text::from(vec![
        Line::from(vec![
            Span::styled(
                format!("{watch_icon} "),
                Style::default().fg(if state.config.enable_watch {
                    theme.success()
                } else {
                    theme.muted()
                }),
            ),
            Span::styled("Enable watch mode", Style::default().fg(theme.fg())),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{hooks_icon} "),
                Style::default().fg(if state.config.enable_hooks {
                    theme.success()
                } else {
                    theme.muted()
                }),
            ),
            Span::styled("Install git hooks", Style::default().fg(theme.fg())),
        ]),
    ]));
    frame.render_widget(content, inner);
}

fn render_summary_step(
    frame: &mut Frame,
    area: Rect,
    state: &WizardState,
    theme: &EddaCraftTheme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Summary ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let template_name = state
        .config
        .template_id
        .as_ref()
        .and_then(|id| state.templates.iter().find(|t| &t.id == id))
        .map(|t| t.name.as_str())
        .unwrap_or("none");

    let content = Paragraph::new(Text::from(vec![
        Line::from(vec![
            Span::styled("Project:  ", Style::default().fg(theme.muted())),
            Span::styled(&state.config.project_name, Style::default().fg(theme.fg())),
        ]),
        Line::from(vec![
            Span::styled("Template: ", Style::default().fg(theme.muted())),
            Span::styled(template_name, Style::default().fg(theme.fg())),
        ]),
        Line::from(vec![
            Span::styled("Watch:    ", Style::default().fg(theme.muted())),
            Span::styled(
                if state.config.enable_watch {
                    "enabled"
                } else {
                    "disabled"
                },
                Style::default().fg(theme.fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled("Hooks:    ", Style::default().fg(theme.muted())),
            Span::styled(
                if state.config.enable_hooks {
                    "enabled"
                } else {
                    "disabled"
                },
                Style::default().fg(theme.fg()),
            ),
        ]),
        Line::default(),
        Line::from(Span::styled(
            "Press enter to confirm and create the project",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )),
    ]));
    frame.render_widget(content, inner);
}
```

- [ ] **Step 4: Update surfaces/mod.rs**

Add `pub mod wizard;` to `crates/anvil-tui/src/surfaces/mod.rs`.

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p eddacraft-anvil-tui`
Expected: compiles with no errors

- [ ] **Step 6: Commit**

```bash
git add crates/anvil-tui/src/surfaces/wizard/ crates/anvil-tui/src/surfaces/mod.rs
git commit -m "feat(rats): add APS onboarding wizard surface (RATS-004)"
```

---

### Task 12: Final build verification + APS status update

- [ ] **Step 1: Run all tests**

Run: `cargo test --all`
Expected: all tests pass

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features`
Expected: no warnings (clippy deny is set in workspace)

- [ ] **Step 3: Run fmt check**

Run: `cargo fmt --all -- --check`
Expected: no formatting issues

- [ ] **Step 4: Update APS module statuses**

Update `plans/modules/rust-kernel.aps.md`:
- KERN-005: Draft → Done
- KERN-010: Draft → Done
- KERN-011: Draft → Done
- KERN-012: Draft → Done
- KERN-013: Draft → Done
- Phase 1 status: Draft → Done

Update `plans/modules/ratatui-tui.aps.md`:
- RATS-004: Draft → Done

Update `plans/modules/ink-to-ratatui-port.aps.md`:
- PORT-010: Draft → Done
- PORT-011: Draft → Done
- PORT-012: Draft → Done
- Phase 2 status: Draft → Done

- [ ] **Step 5: Commit APS updates**

```bash
git add plans/modules/
git commit -m "chore(plans): update KERN, RATS, PORT module statuses for Wave 1 completion"
```
