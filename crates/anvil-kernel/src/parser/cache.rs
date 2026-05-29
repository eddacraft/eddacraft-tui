use std::collections::HashMap;
use std::path::PathBuf;

/// Content-addressed AST cache keyed by file path.
///
/// Stores parsed ASTs alongside a content hash AND a grammar-version
/// fingerprint. On reparse both are compared -- the cached tree is returned
/// only if the content is unchanged *and* it was parsed by the same grammar
/// build. The grammar-version component (LANGTS-005 K2) closes a latent
/// staleness bug: a tree-sitter grammar bump changes the parse output for the
/// same bytes, so keying on content hash alone could serve a tree built by the
/// old grammar after an upgrade.
pub struct AstCache {
    entries: HashMap<PathBuf, CacheEntry>,
}

struct CacheEntry {
    content_hash: u64,
    grammar_version: u64,
    tree: tree_sitter::Tree,
}

impl Default for AstCache {
    fn default() -> Self {
        Self::new()
    }
}

impl AstCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Get a cached tree if the content hash AND grammar version both match.
    ///
    /// A mismatch on either component is a miss: changed bytes invalidate the
    /// tree (as before), and a grammar bump invalidates every entry parsed by
    /// the prior grammar build even when the bytes are unchanged (K2).
    pub fn get(
        &self,
        path: &PathBuf,
        content_hash: u64,
        grammar_version: u64,
    ) -> Option<&tree_sitter::Tree> {
        self.entries
            .get(path)
            .filter(|e| e.content_hash == content_hash && e.grammar_version == grammar_version)
            .map(|e| &e.tree)
    }

    /// Insert or update a cached tree.
    pub fn insert(
        &mut self,
        path: PathBuf,
        content_hash: u64,
        grammar_version: u64,
        tree: tree_sitter::Tree,
    ) {
        self.entries.insert(
            path,
            CacheEntry {
                content_hash,
                grammar_version,
                tree,
            },
        );
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

    /// Fixed grammar-version stand-in for tests that don't exercise K2.
    const GV: u64 = 0xABCD;

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

        cache.insert(path.clone(), hash, GV, tree);
        assert!(cache.get(&path, hash, GV).is_some());
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

        cache.insert(path.clone(), hash1, GV, tree);
        assert!(cache.get(&path, hash2, GV).is_none());
    }

    #[test]
    fn cache_miss_on_grammar_version_change() {
        // K2: same path, same content hash, but a different grammar version
        // must be a miss -- a tree parsed by an old grammar cannot be served
        // after a grammar bump.
        let mut cache = AstCache::new();
        let path = PathBuf::from("test.ts");
        let content = b"const x = 1;";
        let hash = hash_content(content);

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        let tree = parser.parse(content, None).unwrap();

        let old_gv = 1_u64;
        let new_gv = 2_u64;
        cache.insert(path.clone(), hash, old_gv, tree);

        // Same content hash, new grammar version -> miss.
        assert!(
            cache.get(&path, hash, new_gv).is_none(),
            "a grammar-version change must invalidate the cached tree"
        );
        // Same content hash, original grammar version -> still a hit.
        assert!(cache.get(&path, hash, old_gv).is_some());
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

        cache.insert(path.clone(), hash, GV, tree);
        assert!(cache.remove(&path));
        assert!(cache.get(&path, hash, GV).is_none());
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
