use std::collections::HashMap;
use std::path::PathBuf;

/// Content-addressed AST cache keyed by file path.
///
/// Stores parsed ASTs alongside a content hash. On reparse,
/// the hash is compared -- if unchanged, the cached tree is returned.
pub struct AstCache {
    entries: HashMap<PathBuf, CacheEntry>,
}

struct CacheEntry {
    content_hash: u64,
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
