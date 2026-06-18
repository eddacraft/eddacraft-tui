use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::util::is_ignored_dir_name;

const CACHE_SCHEMA: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct WatchWarmupCache {
    schema: u32,
    root: String,
    entries: Vec<WarmupEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WarmupEntry {
    path: String,
    len: u64,
    modified_unix_secs: Option<u64>,
}

pub fn write_watch_warmup_cache(root: &Path) -> anyhow::Result<()> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let entries = collect_entries(&root)?;
    let cache = WatchWarmupCache {
        schema: CACHE_SCHEMA,
        root: root.to_string_lossy().into_owned(),
        entries,
    };
    let path = cache_path(&root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(&cache).context("serialising watch warm-up cache")?;
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

pub fn load_watch_warmup_cache(root: &Path) -> anyhow::Result<Option<Vec<String>>> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let path = cache_path(&root);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };
    let cache: WatchWarmupCache = match serde_json::from_str(&text) {
        Ok(cache) => cache,
        Err(_) => return Ok(None),
    };
    if cache.schema != CACHE_SCHEMA || cache.root != root.to_string_lossy() {
        return Ok(None);
    }

    let current_entries = collect_entries(&root)?;
    if !same_entries(&cache.entries, &current_entries) {
        return Ok(None);
    }

    let mut paths = Vec::with_capacity(cache.entries.len());
    for entry in cache.entries {
        let path = root.join(&entry.path);
        let Ok(metadata) = std::fs::metadata(&path) else {
            return Ok(None);
        };
        if !metadata.is_file()
            || metadata.len() != entry.len
            || modified_unix_secs(&metadata) != entry.modified_unix_secs
        {
            return Ok(None);
        }
        paths.push(entry.path);
    }
    Ok(Some(paths))
}

fn same_entries(left: &[WarmupEntry], right: &[WarmupEntry]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.path == right.path
                && left.len == right.len
                && left.modified_unix_secs == right.modified_unix_secs
        })
}

fn cache_path(root: &Path) -> std::path::PathBuf {
    root.join(".anvil").join("cache").join("watch-warmup.json")
}

fn collect_entries(root: &Path) -> anyhow::Result<Vec<WarmupEntry>> {
    let walker = ignore::WalkBuilder::new(root)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .filter_entry(|entry| {
            if entry.file_type().is_some_and(|ft| ft.is_dir())
                && let Some(name) = entry.file_name().to_str()
            {
                return entry.depth() == 0 || !is_ignored_dir_name(name);
            }
            true
        })
        .build();

    let mut entries = Vec::new();
    for entry in walker.filter_map(Result::ok) {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) || !is_parseable(entry.path()) {
            continue;
        }
        let metadata = entry
            .metadata()
            .with_context(|| format!("reading metadata for {}", entry.path().display()))?;
        let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
        entries.push(WarmupEntry {
            path: rel.to_string_lossy().into_owned(),
            len: metadata.len(),
            modified_unix_secs: modified_unix_secs(&metadata),
        });
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

fn is_parseable(path: &Path) -> bool {
    // Single source of truth with the kernel parser — every supported anchor
    // and tail-wave language (LANGTAIL) warms the cache, not just JS/TS.
    anvil_kernel::parser::languages::Language::from_path(path).is_some()
}

fn modified_unix_secs(metadata: &std::fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn writes_and_loads_existing_parseable_paths() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("main.ts"), "export const x = 1;").unwrap();

        write_watch_warmup_cache(tmp.path()).unwrap();

        let paths = load_watch_warmup_cache(tmp.path()).unwrap().unwrap();
        assert_eq!(paths, vec!["main.ts"]);
    }

    #[test]
    fn stale_cache_falls_back_to_none() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("main.ts");
        std::fs::write(&path, "export const x = 1;").unwrap();
        write_watch_warmup_cache(tmp.path()).unwrap();
        std::fs::remove_file(path).unwrap();

        assert!(load_watch_warmup_cache(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn new_parseable_file_invalidates_cache() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("main.ts"), "export const x = 1;").unwrap();
        write_watch_warmup_cache(tmp.path()).unwrap();
        std::fs::write(tmp.path().join("later.ts"), "export const y = 2;").unwrap();

        assert!(load_watch_warmup_cache(tmp.path()).unwrap().is_none());
    }
}
