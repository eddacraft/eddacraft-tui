//! MLP2-001: daemon-side resolved rule-set cache with `.anvil.*`
//! watcher invalidation.
//!
//! Each registered worktree gets at most one cached
//! [`ResolvedRuleSet`] keyed by [`WorktreeKey`]. The entry carries
//! the `rules_sha` that was active when the entry was built so a
//! caller chasing a witness `rules_sha` can confirm the cached
//! resolution still matches without parsing again.
//!
//! ## Why a separate module
//!
//! The session registry (`registry.rs`) is identity-focused: it
//! attributes filesystem writes to a session. Rule resolution is a
//! separate concern that does not need session-level granularity —
//! every agent inside the same worktree sees the same rule set, so
//! the cache key is the worktree path, not the session. MLP2-023
//! extends the *session registry* key to `(WorktreeKey, AgentTag)`;
//! this cache stays worktree-scoped because rules are not
//! per-agent. The newtype [`WorktreeKey`] keeps both concerns
//! lined up without making one depend on the other.
//!
//! ## What this module does NOT do
//!
//! - **Does not parse config.** Cache misses call back into a
//!   resolver supplied by the caller — typically [`resolve_for_worktree`]
//!   in this same module, which uses [`anvil_config::parse_file`] +
//!   [`anvil_rules::rules_sha`]. Keeping the cache agnostic of
//!   those crates makes the data structure easy to test in
//!   isolation.
//! - **Does not enforce.** Rule evaluation lives in the L4 / hook
//!   path (MLP2-016 onwards). The cache is a memoisation layer.
//! - **Does not pin in-flight evaluations.** MLP2-002 owns the
//!   scheduler-level pinning so a mid-evaluation config write
//!   does not swap the rule set under a running call. The cache's
//!   invalidate semantics are eventual, not abortive.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anvil_config::{CanonicalError, ParseError, canonical_json_bytes, discover, parse_file};
use anvil_rules::{RulesShaError, RulesShaInput};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Canonicalised worktree path used as the cache key.
///
/// Newtype rather than a bare `PathBuf` so the type system catches
/// "passed a path that wasn't canonicalised" mistakes. The wrapping
/// type is intentionally cheap (one allocation per construction,
/// identical to `PathBuf::from`).
///
/// **Forward-compat note (MLP2-023):** the registry session key
/// will extend to `(WorktreeKey, AgentTag)`. This cache stays keyed
/// on `WorktreeKey` alone because every agent inside the same
/// worktree resolves to the same rule set, so MLP2-023 lands
/// without touching this type.
///
/// **Trust-model note:** the path is canonicalised once at
/// construction; a post-registration symlink swap will desync this
/// key from the registry's view of the worktree. The cache is a
/// memoisation layer rather than an attribution surface, so the
/// downside of drift is a stale cache entry until the next
/// `.anvil.*` event, not an attribution bypass. Long-term remediation
/// is keying on `(dev, inode)` — tracked outside MLP2-001 (Council
/// 2026-05-14 #C-022).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorktreeKey(PathBuf);

impl WorktreeKey {
    /// Wrap a path that the caller has already canonicalised. Useful
    /// when the registry has already canonicalised the same value and
    /// we want to avoid hitting the filesystem twice.
    #[must_use]
    pub fn from_canonical(path: PathBuf) -> Self {
        Self(path)
    }

    /// Canonicalise the given path and wrap it. Surfaces the OS error
    /// directly so the caller can decide whether to treat a missing
    /// directory as "no cache entry" or an error.
    pub fn canonicalise(path: &Path) -> std::io::Result<Self> {
        std::fs::canonicalize(path).map(Self)
    }

    /// Borrow the underlying canonical path.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for WorktreeKey {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

/// Resolved rule-set payload cached against a worktree.
///
/// `config` is the raw `serde_json::Value` returned by
/// [`anvil_config::parse_file`]. Downstream consumers (L4 engine in
/// MLP2-016, hook in MLP-003) project it into their own shapes; the
/// cache stores the canonical parsed form so multiple consumers
/// share one parse.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedRuleSet {
    /// Parsed `.anvil.*` payload, post-format-dispatch but
    /// pre-policy-projection.
    pub config: Value,
}

/// Cached entry: the resolved rule set plus the `rules_sha` that
/// identifies it. Callers chasing a witness `rules_sha` can use the
/// recorded value to confirm the cached entry still matches their
/// expected version.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleSetEntry {
    pub rules_sha: String,
    pub resolved: ResolvedRuleSet,
}

/// Outcome of a cache lookup. Tests assert on the variant to
/// separate hit/miss without scraping internal counters.
#[derive(Debug, Clone, PartialEq)]
pub enum CacheOutcome {
    Hit(RuleSetEntry),
    Miss,
}

/// Thread-safe rule-set cache. Stores at most one entry per
/// canonicalised worktree path; invalidation drops the entry and
/// forces a re-resolve on the next access.
///
/// The internal map is wrapped in a `Mutex` rather than a
/// `RwLock`: rule-set resolution happens on the file-watcher
/// thread and on the enforcement pipeline thread, both at low
/// frequency relative to the work they hand off. The contention
/// surface is dominated by hash-map mutations, not reads, so the
/// simpler primitive wins.
#[derive(Debug, Default)]
pub struct RuleSetCache {
    inner: Mutex<HashMap<WorktreeKey, RuleSetEntry>>,
}

impl RuleSetCache {
    /// Empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the entry for `key` without populating on miss. Returns
    /// a `Hit` carrying a clone of the cached entry, or `Miss`.
    pub fn lookup(&self, key: &WorktreeKey) -> CacheOutcome {
        match self.lock().get(key) {
            Some(entry) => CacheOutcome::Hit(entry.clone()),
            None => CacheOutcome::Miss,
        }
    }

    /// Read the entry for `key` and on miss call `resolver` to build
    /// one, store it, and return the freshly-built entry.
    ///
    /// `resolver` is fallible — anvil-config parsing or anvil-rules
    /// computation can fail (malformed YAML, invalid rule id, etc.).
    /// Errors bypass the cache: a failed resolution does **not**
    /// poison the entry, so a follow-up call after the operator
    /// fixes the file recomputes and succeeds.
    pub fn get_or_resolve<F, E>(&self, key: &WorktreeKey, resolver: F) -> Result<RuleSetEntry, E>
    where
        F: FnOnce(&WorktreeKey) -> Result<RuleSetEntry, E>,
    {
        if let Some(existing) = self.lock().get(key).cloned() {
            return Ok(existing);
        }
        // Resolve outside the lock so a slow resolver does not block
        // sibling worktree look-ups. Trade-offs:
        // - Two concurrent resolvers on the same key both compute and
        //   the second insert wins. If the config file is stable across
        //   both reads they produce identical entries; if it changes,
        //   the later insert wins with the newer content — also correct.
        // - A watcher invalidate that arrives between this miss-check
        //   and the insert below is a no-op (the entry is not yet in
        //   the map), so the fresh insert here can re-populate a
        //   stale entry. This is consistent with the module-level
        //   "invalidate semantics are eventual" contract: the *next*
        //   `.anvil.*` watcher event clears the re-inserted entry, and
        //   in-flight evaluations are independently pinned by
        //   MLP2-002 so they never see the stale data either.
        let fresh = resolver(key)?;
        self.lock().insert(key.clone(), fresh.clone());
        Ok(fresh)
    }

    /// Drop the entry for `key`. Returns `true` when an entry was
    /// present, `false` otherwise — useful for telemetry that
    /// counts effective invalidations vs no-ops.
    pub fn invalidate(&self, key: &WorktreeKey) -> bool {
        self.lock().remove(key).is_some()
    }

    /// Invalidate every entry whose worktree contains the changed
    /// `path`, when the change touched a `.anvil.{yaml,yml,json,toml}`
    /// file. Returns the worktree keys that were invalidated so
    /// callers can emit per-worktree telemetry.
    ///
    /// Recognition rule: the basename must be exactly `.anvil` and
    /// the extension must be exactly `yaml`/`yml`/`json`/`toml` in
    /// lowercase. This matches `anvil_config::discover` —
    /// invalidating on `.anvil.YAML` while `discover` cannot
    /// re-resolve a mixed-case filename would silently collapse the
    /// rule set to empty, so the two recognisers are deliberately
    /// kept in lock-step (Council 2026-05-14 #C-019 / #C-028).
    /// Touches of unrelated files — even those that happen to share
    /// a worktree — are a no-op.
    ///
    /// If `std::fs::canonicalize` fails on the changed file's parent
    /// directory (e.g. it was deleted between the watcher event and
    /// the lookup), the cache cannot match the raw path against its
    /// canonical keys. In that case we conservatively invalidate
    /// **every** entry rather than risk a silent miss — the
    /// re-resolve cost is bounded by the cache size, and serving
    /// stale rules after a `.anvil.*` write is the regression
    /// MLP2-001 exists to prevent (Council 2026-05-14 #C-020 /
    /// #C-035).
    pub fn invalidate_on_change(&self, path: &Path) -> Vec<WorktreeKey> {
        if !is_anvil_config_file(path) {
            return Vec::new();
        }
        let Some(parent) = path.parent() else {
            return Vec::new();
        };
        let mut hits = Vec::new();
        let mut guard = self.lock();
        let to_drop: Vec<WorktreeKey> = if let Ok(parent_canon) = std::fs::canonicalize(parent) {
            guard
                .keys()
                .filter(|key| key.as_path() == parent_canon)
                .cloned()
                .collect()
        } else {
            // Canonicalise failed — usually a transient mid-burst
            // delete. Conservatively flush every entry; the next
            // access re-resolves and re-populates. Cheaper than a
            // permanently stale cache.
            guard.keys().cloned().collect()
        };
        for key in to_drop {
            if guard.remove(&key).is_some() {
                hits.push(key);
            }
        }
        hits
    }

    /// Number of cached entries. Useful for tests and telemetry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// `true` when no entries are cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<WorktreeKey, RuleSetEntry>> {
        // Mutex poisoning here would indicate a panic mid-resolve;
        // the cache's invariants are simple (no torn writes — every
        // mutation is a `HashMap::insert` or `::remove`) so taking
        // the poisoned guard is safe. A second-tier recovery
        // strategy is unnecessary for this surface.
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Errors raised while resolving a rule set for a cache miss.
#[derive(Debug, Error)]
pub enum ResolveError {
    /// Filesystem error walking the worktree to find a config file.
    #[error("rule-set resolve io error in {path}: {source}")]
    Discover {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// `.anvil.*` parsing failed (malformed YAML / JSON / TOML).
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Canonical-JSON encoding refused the parsed value. In practice
    /// only fires when the config contains non-finite floats; the
    /// parser already rejects those for TOML, so this is a
    /// last-line-of-defence error.
    #[error(transparent)]
    Canonical(#[from] CanonicalError),
    /// Rule-set hashing rejected the assembled input (bad config-sha
    /// shape or invalid rule id). Effectively impossible from this
    /// call site — we feed our own SHA-256 hex — but kept typed.
    #[error(transparent)]
    RulesSha(#[from] RulesShaError),
}

/// Cache-miss resolver: discover the `.anvil.*` config under the
/// worktree, parse it, and compute the `rules_sha` for the supplied
/// runtime context.
///
/// `anvil_version` is the running anvil binary's semver; pass
/// `env!("CARGO_PKG_VERSION")` at the call site so the value reflects
/// the actual daemon, not this crate. `opa_runtime_version` is the
/// pinned regorus / OPA version; the L4 engine (MLP2-016) supplies
/// it. `rules` enumerates the rule identifiers that the daemon will
/// hold the operator to — for v1 callers may pass an empty iterator,
/// in which case the `rules_sha` is keyed purely on
/// `(anvil_version, config_sha, opa_runtime_version)`. MLP2-014 wires
/// the real rule list through.
///
/// When the worktree has no `.anvil.*` file the function returns an
/// entry built against an empty JSON object — operators running
/// without explicit policy still get a deterministic `rules_sha` so
/// witness lines stay verifiable.
pub fn resolve_for_worktree<I, S>(
    worktree: &WorktreeKey,
    anvil_version: &str,
    opa_runtime_version: &str,
    rules: I,
) -> Result<RuleSetEntry, ResolveError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let dir = worktree.as_path();
    let discovered = discover(dir, ".anvil").map_err(|source| ResolveError::Discover {
        path: dir.to_path_buf(),
        source,
    })?;

    let config = match discovered.as_ref() {
        Some(found) => parse_file(&found.path)?,
        None => Value::Object(serde_json::Map::new()),
    };

    let canonical = canonical_json_bytes(&config)?;
    let mut hasher = Sha256::new();
    hasher.update(&canonical);
    let config_sha = hex_lower(hasher.finalize().as_slice());

    let input = RulesShaInput::try_new(
        anvil_version.to_string(),
        opa_runtime_version.to_string(),
        rules,
        config_sha,
    )?;
    let rules_sha = input.compute()?;

    Ok(RuleSetEntry {
        rules_sha,
        resolved: ResolvedRuleSet { config },
    })
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit(u32::from(b >> 4), 16).expect("nibble"));
        s.push(char::from_digit(u32::from(b & 0x0f), 16).expect("nibble"));
    }
    s
}

/// Recognise `.anvil.yaml`, `.anvil.yml`, `.anvil.json`, `.anvil.toml`
/// with both the basename and the extension matched
/// **case-sensitively in lowercase**. `anvil_config::discover` only
/// matches lowercase extensions; recognising a mixed-case
/// `.anvil.YAML` here would invalidate the cache but the subsequent
/// `discover` call would not find the file, leaving the cache to
/// re-populate with an empty rule set and silently dropping the
/// operator's policy (Council 2026-05-14 #C-019 / #C-028). The two
/// recognisers are deliberately kept in lock-step.
fn is_anvil_config_file(path: &Path) -> bool {
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    if stem != ".anvil" {
        return false;
    }
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(ext, "yaml" | "yml" | "json" | "toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    use serde_json::json;
    use tempfile::TempDir;

    fn entry(sha: &str) -> RuleSetEntry {
        RuleSetEntry {
            rules_sha: sha.to_string(),
            resolved: ResolvedRuleSet { config: json!({}) },
        }
    }

    fn key(dir: &TempDir) -> WorktreeKey {
        WorktreeKey::canonicalise(dir.path()).unwrap()
    }

    #[test]
    fn lookup_on_empty_cache_returns_miss() {
        let cache = RuleSetCache::new();
        let dir = TempDir::new().unwrap();
        assert_eq!(cache.lookup(&key(&dir)), CacheOutcome::Miss);
    }

    #[test]
    fn get_or_resolve_populates_on_miss() {
        let cache = RuleSetCache::new();
        let dir = TempDir::new().unwrap();
        let k = key(&dir);

        let resolved = cache
            .get_or_resolve::<_, ()>(&k, |_| Ok(entry("abc")))
            .unwrap();

        assert_eq!(resolved.rules_sha, "abc");
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.lookup(&k), CacheOutcome::Hit(entry("abc")));
    }

    #[test]
    fn get_or_resolve_skips_resolver_on_hit() {
        let cache = RuleSetCache::new();
        let dir = TempDir::new().unwrap();
        let k = key(&dir);

        cache
            .get_or_resolve::<_, ()>(&k, |_| Ok(entry("first")))
            .unwrap();

        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let calls_clone = Arc::clone(&calls);
        let second = cache
            .get_or_resolve::<_, ()>(&k, move |_| {
                calls_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(entry("never"))
            })
            .unwrap();

        assert_eq!(second.rules_sha, "first");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn resolver_failure_does_not_poison_cache() {
        let cache = RuleSetCache::new();
        let dir = TempDir::new().unwrap();
        let k = key(&dir);

        let err = cache
            .get_or_resolve::<_, &'static str>(&k, |_| Err("boom"))
            .unwrap_err();
        assert_eq!(err, "boom");
        assert!(cache.is_empty());

        let ok = cache
            .get_or_resolve::<_, &'static str>(&k, |_| Ok(entry("recovered")))
            .unwrap();
        assert_eq!(ok.rules_sha, "recovered");
    }

    #[test]
    fn invalidate_returns_true_when_present() {
        let cache = RuleSetCache::new();
        let dir = TempDir::new().unwrap();
        let k = key(&dir);
        cache
            .get_or_resolve::<_, ()>(&k, |_| Ok(entry("v1")))
            .unwrap();

        assert!(cache.invalidate(&k));
        assert!(cache.is_empty());
        assert!(!cache.invalidate(&k));
    }

    #[test]
    fn invalidate_on_change_drops_matching_worktree() {
        let cache = RuleSetCache::new();
        let dir = TempDir::new().unwrap();
        let k = key(&dir);
        cache
            .get_or_resolve::<_, ()>(&k, |_| Ok(entry("v1")))
            .unwrap();

        for ext in ["yaml", "yml", "json", "toml"] {
            cache
                .get_or_resolve::<_, ()>(&k, |_| Ok(entry("seed")))
                .unwrap();
            let touched = dir.path().join(format!(".anvil.{ext}"));
            std::fs::write(&touched, b"{}").unwrap();
            let hits = cache.invalidate_on_change(&touched);
            assert_eq!(hits, vec![k.clone()], ".anvil.{ext} should invalidate");
            assert!(cache.is_empty());
        }
    }

    #[test]
    fn invalidate_on_change_ignores_unrelated_files() {
        let cache = RuleSetCache::new();
        let dir = TempDir::new().unwrap();
        let k = key(&dir);
        cache
            .get_or_resolve::<_, ()>(&k, |_| Ok(entry("v1")))
            .unwrap();

        let stranger = dir.path().join("Cargo.toml");
        std::fs::write(&stranger, b"# unrelated").unwrap();
        assert!(cache.invalidate_on_change(&stranger).is_empty());
        assert_eq!(cache.len(), 1, "unrelated file must not invalidate");
    }

    /// MLP2-001 Council fix #C-019 / #C-028: the cache invalidator
    /// must mirror `anvil_config::discover`'s lowercase-only rule, or
    /// a mixed-case touch evicts the entry but `discover` cannot
    /// re-find the file — the cache would silently re-populate with
    /// an empty rule set.
    #[test]
    fn invalidate_on_change_rejects_mixed_case_extension() {
        let cache = RuleSetCache::new();
        let dir = TempDir::new().unwrap();
        let k = key(&dir);
        cache
            .get_or_resolve::<_, ()>(&k, |_| Ok(entry("v1")))
            .unwrap();
        let touched = dir.path().join(".anvil.YAML");
        std::fs::write(&touched, b"{}").unwrap();
        assert!(
            cache.invalidate_on_change(&touched).is_empty(),
            ".anvil.YAML must not invalidate; discover() is lowercase-only"
        );
        assert_eq!(cache.len(), 1, "entry must survive the mixed-case write");
    }

    /// MLP2-001 Council fix #C-020 / #C-035: if `canonicalize` fails
    /// on the changed file's parent, the invalidator over-approximates
    /// and clears the entire cache rather than silently missing the
    /// invalidation against canonical keys.
    #[test]
    fn invalidate_on_change_canonicalise_fail_flushes_all_entries() {
        let cache = RuleSetCache::new();
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let k_a = key(&dir_a);
        let k_b = key(&dir_b);
        cache
            .get_or_resolve::<_, ()>(&k_a, |_| Ok(entry("a")))
            .unwrap();
        cache
            .get_or_resolve::<_, ()>(&k_b, |_| Ok(entry("b")))
            .unwrap();

        // Construct a phantom `.anvil.yaml` path whose parent never
        // existed — `std::fs::canonicalize` returns NotFound, which
        // is the same Err shape the cache sees when a parent is
        // deleted mid-burst by the watcher.
        let phantom = Path::new("/nonexistent/anvil-rule-cache-test/.anvil.yaml");
        let hits = cache.invalidate_on_change(phantom);
        assert_eq!(hits.len(), 2, "both worktree entries must be flushed");
        assert!(hits.contains(&k_a));
        assert!(hits.contains(&k_b));
        assert!(cache.is_empty(), "cache must be empty after over-flush");
    }

    /// MLP2-001 Council fix #C-033: a `.anvil.yaml` in a subdirectory
    /// of a worktree must NOT invalidate the worktree-level cache
    /// entry. Only the worktree's *direct* `.anvil.*` files are
    /// considered authoritative.
    #[test]
    fn invalidate_on_change_ignores_anvil_config_in_subdirectory() {
        let cache = RuleSetCache::new();
        let dir = TempDir::new().unwrap();
        let subdir = dir.path().join("nested");
        std::fs::create_dir_all(&subdir).unwrap();
        let k = key(&dir);
        cache
            .get_or_resolve::<_, ()>(&k, |_| Ok(entry("v1")))
            .unwrap();

        let nested = subdir.join(".anvil.yaml");
        std::fs::write(&nested, b"{}").unwrap();
        assert!(
            cache.invalidate_on_change(&nested).is_empty(),
            "nested .anvil.yaml must not invalidate worktree-root entry"
        );
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn invalidate_on_change_only_hits_owning_worktree() {
        let cache = RuleSetCache::new();
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let k_a = key(&dir_a);
        let k_b = key(&dir_b);
        cache
            .get_or_resolve::<_, ()>(&k_a, |_| Ok(entry("a")))
            .unwrap();
        cache
            .get_or_resolve::<_, ()>(&k_b, |_| Ok(entry("b")))
            .unwrap();

        let touched = dir_a.path().join(".anvil.yaml");
        std::fs::write(&touched, b"{}").unwrap();
        let hits = cache.invalidate_on_change(&touched);
        assert_eq!(hits, vec![k_a]);
        assert_eq!(cache.lookup(&k_b), CacheOutcome::Hit(entry("b")));
    }

    #[test]
    fn concurrent_invalidate_and_store_do_not_race() {
        let cache = Arc::new(RuleSetCache::new());
        let dir = TempDir::new().unwrap();
        let k = key(&dir);

        let writer = {
            let cache = Arc::clone(&cache);
            let k = k.clone();
            thread::spawn(move || {
                for i in 0..200 {
                    let _ = cache.get_or_resolve::<_, ()>(&k, |_| Ok(entry(&format!("v{i}"))));
                }
            })
        };
        let invalidator = {
            let cache = Arc::clone(&cache);
            let k = k.clone();
            thread::spawn(move || {
                for _ in 0..200 {
                    cache.invalidate(&k);
                }
            })
        };

        writer.join().unwrap();
        invalidator.join().unwrap();
        // No assertion on final state — convergent under concurrency.
        // The point is that `lock()` does not deadlock or panic and
        // the cache stays internally consistent (size 0 or 1, both
        // valid).
        let final_len = cache.len();
        assert!(final_len <= 1);
    }

    #[test]
    fn resolve_for_worktree_with_no_config_returns_deterministic_entry() {
        let dir = TempDir::new().unwrap();
        let k = key(&dir);
        let a = resolve_for_worktree::<_, &str>(&k, "0.7.0-beta", "0.10.0", []).unwrap();
        let b = resolve_for_worktree::<_, &str>(&k, "0.7.0-beta", "0.10.0", []).unwrap();
        assert_eq!(
            a.rules_sha, b.rules_sha,
            "deterministic on identical inputs"
        );
        assert_eq!(
            a.resolved.config,
            json!({}),
            "missing config => empty object"
        );
    }

    #[test]
    fn resolve_for_worktree_picks_up_yaml_config() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".anvil.yaml"), "mode: warn\n").unwrap();
        let k = key(&dir);
        let entry =
            resolve_for_worktree::<_, &str>(&k, "0.7.0-beta", "0.10.0", ["rule-a"]).unwrap();
        assert_eq!(entry.resolved.config, json!({"mode": "warn"}));
    }

    #[test]
    fn resolve_for_worktree_is_format_agnostic_for_rules_sha() {
        let dir_yaml = TempDir::new().unwrap();
        std::fs::write(dir_yaml.path().join(".anvil.yaml"), "mode: warn\n").unwrap();
        let dir_json = TempDir::new().unwrap();
        std::fs::write(dir_json.path().join(".anvil.json"), r#"{"mode":"warn"}"#).unwrap();

        let yaml =
            resolve_for_worktree::<_, &str>(&key(&dir_yaml), "0.7.0-beta", "0.10.0", ["rule-a"])
                .unwrap();
        let json_entry =
            resolve_for_worktree::<_, &str>(&key(&dir_json), "0.7.0-beta", "0.10.0", ["rule-a"])
                .unwrap();
        assert_eq!(
            yaml.rules_sha, json_entry.rules_sha,
            "yaml and json with identical content must hash identically"
        );
    }

    #[test]
    fn resolve_for_worktree_changes_when_config_changes() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".anvil.yaml"), "mode: warn\n").unwrap();
        let k = key(&dir);
        let before = resolve_for_worktree::<_, &str>(&k, "0.7.0-beta", "0.10.0", []).unwrap();

        std::fs::write(dir.path().join(".anvil.yaml"), "mode: fence\n").unwrap();
        let after = resolve_for_worktree::<_, &str>(&k, "0.7.0-beta", "0.10.0", []).unwrap();
        assert_ne!(before.rules_sha, after.rules_sha);
        assert_ne!(before.resolved.config, after.resolved.config);
    }

    #[test]
    fn is_anvil_config_file_recognises_lowercase_only() {
        assert!(is_anvil_config_file(Path::new("/work/.anvil.yaml")));
        assert!(is_anvil_config_file(Path::new("/work/.anvil.yml")));
        assert!(is_anvil_config_file(Path::new("/work/.anvil.json")));
        assert!(is_anvil_config_file(Path::new("/work/.anvil.toml")));
        // Lock-step with `anvil_config::discover` (lowercase-only).
        assert!(
            !is_anvil_config_file(Path::new("/work/.anvil.JSON")),
            "mixed-case ext must be rejected (Council #C-019 / #C-028)"
        );
        assert!(!is_anvil_config_file(Path::new("/work/.anvil.YAML")));
        assert!(!is_anvil_config_file(Path::new("/work/.Anvil.yaml")));
        assert!(!is_anvil_config_file(Path::new("/work/.anvilrc")));
        assert!(!is_anvil_config_file(Path::new("/work/anvil.yaml")));
        assert!(!is_anvil_config_file(Path::new("/work/.anvil")));
    }
}
