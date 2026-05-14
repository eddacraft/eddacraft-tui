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
//!
//! ## Bounded capacity (MLP2-057)
//!
//! The cache is capped at [`DEFAULT_RULE_SET_CACHE_CAPACITY`] entries
//! by default (sized against INTD-016's session cap of ~1024 live
//! worktrees on a daemon). On insert at capacity the
//! least-recently-used entry is evicted and a `tracing::warn!` event
//! fires so operators can detect cache pressure before MLP2-058 wires
//! the richer status surface. Two counters back the eviction surface:
//!
//! - [`RuleSetCache::len`] — current entry count (`cache.entries_count`).
//! - [`RuleSetCache::evictions`] — cumulative LRU evictions
//!   (`cache.evictions`).
//!
//! Recency is tracked by a monotonic generation counter bumped on
//! every successful `lookup` and `get_or_resolve` hit. Linear scan to
//! find the LRU is O(n) where n ≤ capacity (1024); the constant is
//! small enough that the simpler primitive beats the dependency cost
//! of a `LinkedHashMap` or `lru` crate.
//!
//! ## Session-lifetime coupling (MLP2-057)
//!
//! The cache no longer accumulates stale entries across the daemon's
//! lifetime. [`SessionRegistry::unregister`] and
//! [`SessionRegistry::evict_stale`] both fire a worktree-unregister
//! hook that the daemon wires to [`RuleSetCache::invalidate`]; a
//! register/unregister cycle therefore leaves no cache residue. The
//! hook is opt-in (defaults to no-op) so embedded-mode tests that
//! never construct a cache aren't forced to plumb one through.

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

/// MLP2-057: default cache capacity, sized against INTD-016's
/// per-daemon session cap. Operators with substantially more
/// concurrent worktrees can construct via
/// [`RuleSetCache::with_capacity`] from `Resolved::rule_cache_max`
/// (deferred config knob).
pub const DEFAULT_RULE_SET_CACHE_CAPACITY: usize = 1024;

/// MLP2-057: internal cache state — sessions, LRU recency
/// generation, and the cumulative-evictions counter. Pulled into a
/// dedicated struct so [`RuleSetCache::lock`] hands callers a single
/// guard regardless of how many fields back the cache.
#[derive(Debug, Default)]
struct CacheInner {
    map: HashMap<WorktreeKey, RuleSetEntryWithRecency>,
    /// Monotonic generation counter, bumped on every successful
    /// access (lookup hit, `get_or_resolve` hit, or fresh insert).
    /// An entry's `last_used` is the generation at its most recent
    /// access; the lowest `last_used` in the map is the LRU entry.
    next_generation: u64,
    /// Cumulative LRU evictions since cache construction. The
    /// counter resets only on a new cache instance — operators
    /// inspect rate of change rather than absolute value.
    evictions: u64,
    /// MLP2-058: cumulative effective invalidations since cache
    /// construction. Counts only entries that were actually dropped
    /// — `invalidate` on a missing key, or `invalidate_on_change`
    /// on a non-config file or non-matching worktree, is a no-op
    /// and does NOT bump this counter. Operators reading rate of
    /// change see only the meaningful pressure (watcher-driven
    /// config edits, registry unregister hooks).
    invalidations: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct RuleSetEntryWithRecency {
    entry: RuleSetEntry,
    last_used: u64,
}

/// Thread-safe rule-set cache. Stores at most one entry per
/// canonicalised worktree path up to a fixed capacity; on insert at
/// capacity the least-recently-used entry is evicted with a
/// `tracing::warn!` event and the [`RuleSetCache::evictions`] counter
/// bumps. Invalidation drops an entry and forces a re-resolve on the
/// next access.
///
/// The internal map is wrapped in a `Mutex` rather than a
/// `RwLock`: rule-set resolution happens on the file-watcher
/// thread and on the enforcement pipeline thread, both at low
/// frequency relative to the work they hand off. The contention
/// surface is dominated by hash-map mutations, not reads, so the
/// simpler primitive wins.
#[derive(Debug)]
pub struct RuleSetCache {
    inner: Mutex<CacheInner>,
    capacity: usize,
}

impl Default for RuleSetCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleSetCache {
    /// Empty cache with the default capacity
    /// ([`DEFAULT_RULE_SET_CACHE_CAPACITY`]).
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_RULE_SET_CACHE_CAPACITY)
    }

    /// Empty cache with a custom capacity. `capacity` is clamped to a
    /// minimum of 1 — a zero-capacity cache would refuse every
    /// `get_or_resolve` insert and so disable the memoisation layer
    /// the cache exists for. Tests targeting eviction behaviour use
    /// small capacities (2, 4) to drive the LRU path on a tractable
    /// fixture.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(CacheInner::default()),
            capacity: capacity.max(1),
        }
    }

    /// Maximum number of entries the cache will hold before LRU
    /// eviction kicks in. Pinned at construction; not mutable.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Read the entry for `key` without populating on miss. Returns
    /// a `Hit` carrying a clone of the cached entry, or `Miss`. A hit
    /// bumps the entry's recency so a subsequent capacity-eviction
    /// pass treats this as the most recently used worktree.
    pub fn lookup(&self, key: &WorktreeKey) -> CacheOutcome {
        let mut guard = self.lock();
        let next = guard.next_generation;
        let Some(entry) = guard.map.get_mut(key) else {
            return CacheOutcome::Miss;
        };
        entry.last_used = next;
        let cloned = entry.entry.clone();
        guard.next_generation = next.wrapping_add(1);
        CacheOutcome::Hit(cloned)
    }

    /// Read the entry for `key` and on miss call `resolver` to build
    /// one, store it, and return the freshly-built entry.
    ///
    /// `resolver` is fallible — anvil-config parsing or anvil-rules
    /// computation can fail (malformed YAML, invalid rule id, etc.).
    /// Errors bypass the cache: a failed resolution does **not**
    /// poison the entry, so a follow-up call after the operator
    /// fixes the file recomputes and succeeds.
    ///
    /// MLP2-057: a successful resolve at capacity evicts the LRU
    /// entry before insert, fires `tracing::warn!`, and bumps
    /// [`RuleSetCache::evictions`].
    pub fn get_or_resolve<F, E>(&self, key: &WorktreeKey, resolver: F) -> Result<RuleSetEntry, E>
    where
        F: FnOnce(&WorktreeKey) -> Result<RuleSetEntry, E>,
    {
        {
            let mut guard = self.lock();
            let next = guard.next_generation;
            if let Some(existing) = guard.map.get_mut(key) {
                existing.last_used = next;
                let cloned = existing.entry.clone();
                guard.next_generation = next.wrapping_add(1);
                return Ok(cloned);
            }
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
        // MLP2-058: log cache miss before resolve so a config-parse
        // failure (resolver returns Err) is observable as
        // "miss + no insert" rather than silently dropped. Debug
        // level because misses are the steady-state rate (every
        // first request for a worktree); operators bump to `debug`
        // when they need cache-shape visibility.
        tracing::debug!(
            target: "anvil_intercept::rule_cache",
            worktree = %key.as_path().display(),
            "rule_cache miss; resolving",
        );
        let fresh = resolver(key)?;
        self.insert_with_eviction(key.clone(), fresh.clone());
        Ok(fresh)
    }

    /// Drop the entry for `key`. Returns `true` when an entry was
    /// present, `false` otherwise — useful for telemetry that
    /// counts effective invalidations vs no-ops.
    ///
    /// MLP2-058: an effective drop bumps
    /// [`RuleSetCache::invalidations`].
    pub fn invalidate(&self, key: &WorktreeKey) -> bool {
        let mut guard = self.lock();
        let dropped = guard.map.remove(key).is_some();
        if dropped {
            guard.invalidations = guard.invalidations.saturating_add(1);
        }
        dropped
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
                .map
                .keys()
                .filter(|key| key.as_path() == parent_canon)
                .cloned()
                .collect()
        } else {
            // Canonicalise failed — usually a transient mid-burst
            // delete. Conservatively flush every entry; the next
            // access re-resolves and re-populates. Cheaper than a
            // permanently stale cache.
            guard.map.keys().cloned().collect()
        };
        for key in to_drop {
            if guard.map.remove(&key).is_some() {
                hits.push(key);
            }
        }
        if !hits.is_empty() {
            // MLP2-058: bump the invalidation counter for the
            // operator-visible status surface and emit a single
            // `tracing::info!` event so a configuration-edit storm
            // shows up in daemon logs without per-key spam. Counter
            // moves by the exact number of effective drops, so the
            // status surface reflects pressure honestly. We log
            // path + count here rather than per key.
            let drops = u64::try_from(hits.len()).unwrap_or(u64::MAX);
            guard.invalidations = guard.invalidations.saturating_add(drops);
            let total_invalidations = guard.invalidations;
            tracing::info!(
                target: "anvil_intercept::rule_cache",
                anvil_config_path = %path.display(),
                invalidated = hits.len(),
                cache_invalidations_total = total_invalidations,
                "rule_cache invalidated by config-edit",
            );
        }
        hits
    }

    /// Number of cached entries. The `cache.entries_count`
    /// counter surfaced for MLP2-058. Useful for tests and telemetry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lock().map.len()
    }

    /// `true` when no entries are cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lock().map.is_empty()
    }

    /// Cumulative LRU evictions since this cache instance was
    /// constructed. The `cache.evictions` counter surfaced for
    /// MLP2-058 — a steady non-zero rate of change indicates the
    /// cache is sized too small for the active worktree set.
    #[must_use]
    pub fn evictions(&self) -> u64 {
        self.lock().evictions
    }

    /// MLP2-058: cumulative effective invalidations since this
    /// cache instance was constructed.
    ///
    /// Counts entries actually dropped by [`Self::invalidate`]
    /// (registry-unregister hooks) + [`Self::invalidate_on_change`]
    /// (watcher config-edit hits). No-op invalidate calls do NOT
    /// contribute. Surfaced via the `cache_invalidations_total`
    /// field on `DaemonStatusV1` so operators reading rate of
    /// change see only meaningful pressure.
    #[must_use]
    pub fn invalidations(&self) -> u64 {
        self.lock().invalidations
    }

    /// MLP2-057: insert with capacity enforcement. Called from the
    /// `get_or_resolve` miss path. Locks once, checks capacity,
    /// evicts LRU if needed, then inserts. Two-phase locks would
    /// risk a torn state if a second writer slipped between the
    /// eviction and the insert.
    fn insert_with_eviction(&self, key: WorktreeKey, entry: RuleSetEntry) {
        use std::collections::hash_map::Entry;

        let mut guard = self.lock();
        let next = guard.next_generation;
        // If the key already exists we're just refreshing it —
        // capacity is unchanged, recency bumps, no eviction.
        if let Entry::Occupied(mut existing) = guard.map.entry(key.clone()) {
            existing.insert(RuleSetEntryWithRecency {
                entry,
                last_used: next,
            });
            guard.next_generation = next.wrapping_add(1);
            return;
        }
        if guard.map.len() >= self.capacity {
            // Linear scan for the LRU entry. n ≤ capacity (1024 by
            // default); the constant is small enough that an
            // intrusive LRU list would not pay off here.
            if let Some(victim_key) = guard
                .map
                .iter()
                .min_by_key(|(_, v)| v.last_used)
                .map(|(k, _)| k.clone())
            {
                guard.map.remove(&victim_key);
                guard.evictions = guard.evictions.saturating_add(1);
                let total_evictions = guard.evictions;
                let capacity = self.capacity;
                tracing::warn!(
                    target: "anvil_intercept::rule_cache",
                    evicted_worktree = %victim_key.as_path().display(),
                    cache_capacity = capacity,
                    cache_evictions_total = total_evictions,
                    "rule_cache LRU eviction; cache at capacity",
                );
            }
        }
        guard.map.insert(
            key,
            RuleSetEntryWithRecency {
                entry,
                last_used: next,
            },
        );
        guard.next_generation = next.wrapping_add(1);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, CacheInner> {
        // Mutex poisoning here would indicate a panic mid-resolve;
        // the cache's invariants are simple (no torn writes — every
        // mutation is a `HashMap::insert` or `::remove`) so taking
        // the poisoned guard is safe. A second-tier recovery
        // strategy is unnecessary for this surface.
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // MLP2-058 Council #C-025: surface poisoned-recovery
                // once per acquisition. Operators investigating a
                // misbehaving daemon see the recovery rather than
                // having to reverse-engineer "the cache silently
                // kept running after a panic". `tracing::warn!` is
                // the right severity — recovery is safe (poisoned
                // state cannot tear the cache's HashMap-only
                // invariants) but the upstream panic is itself a
                // bug that needs investigation.
                tracing::warn!(
                    target: "anvil_intercept::rule_cache",
                    "rule_cache mutex poisoned; recovering inner state",
                );
                poisoned.into_inner()
            }
        }
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
        const ITERATIONS: usize = 200;
        let cache = Arc::new(RuleSetCache::new());
        let dir = TempDir::new().unwrap();
        let k = key(&dir);

        let writer = {
            let cache = Arc::clone(&cache);
            let k = k.clone();
            thread::spawn(move || {
                for i in 0..ITERATIONS {
                    // Bind the formatted value in an outer let so the
                    // closure captures the produced `String` rather
                    // than the loop variable directly — keeps CodeQL's
                    // "unused variable" lint quiet on the inlined
                    // `format!("v{i}")` pattern (PR #1522 review).
                    let sha = format!("v{i}");
                    let _ = cache.get_or_resolve::<_, ()>(&k, |_| Ok(entry(&sha)));
                }
            })
        };
        let invalidator = {
            let cache = Arc::clone(&cache);
            let k = k.clone();
            thread::spawn(move || {
                for _ in 0..ITERATIONS {
                    cache.invalidate(&k);
                }
            })
        };

        writer.join().unwrap();
        invalidator.join().unwrap();
        // The cache must be convergent under concurrency. Two
        // assertions, both addressing pragmatic-lead Council finding
        // #C-003 by lifting the test from "Mutex does not deadlock"
        // (stdlib-guaranteed) to "the cache only ever holds a value
        // the writer inserted":
        // 1) Cache size is at most 1 (single-key cache property).
        // 2) Any final entry's `rules_sha` must be one of the
        //    `v0..v200` values the writer produced — never a torn
        //    value, never something the writer never inserted.
        let final_len = cache.len();
        assert!(final_len <= 1);
        if let CacheOutcome::Hit(entry) = cache.lookup(&k) {
            let valid: Vec<String> = (0..ITERATIONS).map(|i| format!("v{i}")).collect();
            assert!(
                valid.contains(&entry.rules_sha),
                "final rules_sha must be one of the writer's inserts, got {:?}",
                entry.rules_sha
            );
        }
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

    // MLP2-057 — bounded capacity + LRU eviction.

    /// `RuleSetCache::new()` pins the default capacity at
    /// [`DEFAULT_RULE_SET_CACHE_CAPACITY`]. Operators that need a
    /// different value go through [`RuleSetCache::with_capacity`]; this
    /// pin guards against an accidental default-change drive-by.
    #[test]
    fn default_capacity_is_pinned() {
        let cache = RuleSetCache::new();
        assert_eq!(cache.capacity(), DEFAULT_RULE_SET_CACHE_CAPACITY);
        assert_eq!(cache.evictions(), 0);
        assert!(cache.is_empty());
    }

    /// Zero-capacity caches are clamped to 1. A literal-zero cap
    /// would refuse every insert, disabling the cache entirely;
    /// treating it as a typo and clamping to 1 is the safer default.
    #[test]
    fn with_capacity_clamps_zero_to_one() {
        let cache = RuleSetCache::with_capacity(0);
        assert_eq!(cache.capacity(), 1);
    }

    /// Fill the cache to capacity, then insert one more — the
    /// least-recently-used key (the first one inserted, since no
    /// later access bumped its recency) is evicted, and the
    /// evictions counter increments.
    #[test]
    fn lru_evicts_oldest_entry_at_capacity_plus_one() {
        let cache = RuleSetCache::with_capacity(2);
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let dir_c = TempDir::new().unwrap();
        let k_a = key(&dir_a);
        let k_b = key(&dir_b);
        let k_c = key(&dir_c);

        cache
            .get_or_resolve::<_, ()>(&k_a, |_| Ok(entry("a")))
            .unwrap();
        cache
            .get_or_resolve::<_, ()>(&k_b, |_| Ok(entry("b")))
            .unwrap();
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.evictions(), 0);

        cache
            .get_or_resolve::<_, ()>(&k_c, |_| Ok(entry("c")))
            .unwrap();
        assert_eq!(cache.len(), 2, "cache must not exceed capacity");
        assert_eq!(cache.evictions(), 1, "evictions counter incremented");
        assert_eq!(cache.lookup(&k_a), CacheOutcome::Miss, "k_a was LRU");
        assert!(matches!(cache.lookup(&k_b), CacheOutcome::Hit(_)));
        assert!(matches!(cache.lookup(&k_c), CacheOutcome::Hit(_)));
    }

    /// A `lookup` hit bumps the entry's recency. Insert a, b; lookup
    /// a so a is now MRU; insert c; b should be the eviction victim,
    /// not a.
    #[test]
    fn lookup_bumps_recency_so_it_is_not_lru() {
        let cache = RuleSetCache::with_capacity(2);
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let dir_c = TempDir::new().unwrap();
        let k_a = key(&dir_a);
        let k_b = key(&dir_b);
        let k_c = key(&dir_c);

        cache
            .get_or_resolve::<_, ()>(&k_a, |_| Ok(entry("a")))
            .unwrap();
        cache
            .get_or_resolve::<_, ()>(&k_b, |_| Ok(entry("b")))
            .unwrap();
        // Touch a — now b is the LRU.
        assert!(matches!(cache.lookup(&k_a), CacheOutcome::Hit(_)));
        cache
            .get_or_resolve::<_, ()>(&k_c, |_| Ok(entry("c")))
            .unwrap();

        assert_eq!(cache.lookup(&k_b), CacheOutcome::Miss, "k_b was LRU");
        assert!(matches!(cache.lookup(&k_a), CacheOutcome::Hit(_)));
        assert!(matches!(cache.lookup(&k_c), CacheOutcome::Hit(_)));
    }

    /// A `get_or_resolve` hit bumps recency the same way `lookup`
    /// does. Insert a, b; resolve a (hit, recency bumps); insert c;
    /// b is evicted.
    #[test]
    fn get_or_resolve_hit_bumps_recency_so_it_is_not_lru() {
        let cache = RuleSetCache::with_capacity(2);
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let dir_c = TempDir::new().unwrap();
        let k_a = key(&dir_a);
        let k_b = key(&dir_b);
        let k_c = key(&dir_c);

        cache
            .get_or_resolve::<_, ()>(&k_a, |_| Ok(entry("a")))
            .unwrap();
        cache
            .get_or_resolve::<_, ()>(&k_b, |_| Ok(entry("b")))
            .unwrap();
        // Re-resolve a; the resolver must NOT fire (hit path).
        cache
            .get_or_resolve::<_, ()>(&k_a, |_| panic!("resolver must not run on hit"))
            .unwrap();
        cache
            .get_or_resolve::<_, ()>(&k_c, |_| Ok(entry("c")))
            .unwrap();

        assert_eq!(cache.lookup(&k_b), CacheOutcome::Miss);
        assert!(matches!(cache.lookup(&k_a), CacheOutcome::Hit(_)));
        assert!(matches!(cache.lookup(&k_c), CacheOutcome::Hit(_)));
    }

    /// Evictions counter increments once per LRU eviction across a
    /// burst that pushes the cache past capacity multiple times. The
    /// counter is cumulative, not per-call.
    #[test]
    fn evictions_counter_increments_per_eviction() {
        let cache = RuleSetCache::with_capacity(2);
        let dirs: Vec<TempDir> = (0..5).map(|_| TempDir::new().unwrap()).collect();
        let keys: Vec<WorktreeKey> = dirs.iter().map(key).collect();

        for (idx, k) in keys.iter().enumerate() {
            let sha = format!("v{idx}");
            cache
                .get_or_resolve::<_, ()>(k, |_| Ok(entry(&sha)))
                .unwrap();
        }
        // Capacity 2, inserted 5 → 3 evictions.
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.evictions(), 3);
    }

    /// Re-inserting an existing key is a refresh, not a capacity
    /// pressure event — the evictions counter must NOT bump.
    #[test]
    fn re_inserting_existing_key_is_not_an_eviction() {
        let cache = RuleSetCache::with_capacity(2);
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let k_a = key(&dir_a);
        let k_b = key(&dir_b);

        cache
            .get_or_resolve::<_, ()>(&k_a, |_| Ok(entry("a-v1")))
            .unwrap();
        cache
            .get_or_resolve::<_, ()>(&k_b, |_| Ok(entry("b")))
            .unwrap();
        // Invalidate a then re-insert via the resolver — that's
        // still a refresh of "logical" key a, but the contains_key
        // path doesn't fire because invalidate dropped it. So an
        // eviction WOULD happen here if a were the LRU... but
        // capacity is 2 and we only have 1 entry after invalidate.
        // No eviction.
        cache.invalidate(&k_a);
        cache
            .get_or_resolve::<_, ()>(&k_a, |_| Ok(entry("a-v2")))
            .unwrap();
        assert_eq!(
            cache.evictions(),
            0,
            "invalidate + re-insert when below capacity must not be a pressure event",
        );
    }

    /// `invalidate` is a deliberate user-driven drop, not a pressure
    /// event — the evictions counter MUST NOT increment. Operators
    /// reading `cache.evictions` see only the capacity-driven LRU
    /// pressure.
    #[test]
    fn invalidate_does_not_count_as_eviction() {
        let cache = RuleSetCache::with_capacity(4);
        let dir = TempDir::new().unwrap();
        let k = key(&dir);
        cache
            .get_or_resolve::<_, ()>(&k, |_| Ok(entry("v")))
            .unwrap();
        assert!(cache.invalidate(&k));
        assert_eq!(
            cache.evictions(),
            0,
            "invalidate is a deliberate drop, not a pressure event",
        );
    }

    // MLP2-057 — registry → cache hook integration.

    /// End-to-end pin of the registry-side hook driving cache
    /// invalidation. A worktree is registered (cache hit), then
    /// unregistered through `SessionRegistry::unregister`. The
    /// daemon-style hook bridges `unregister` to
    /// `RuleSetCache::invalidate`, so the next cache lookup returns
    /// `Miss`. Pins the wire-up that the APS task's Validation
    /// criterion calls out ("register-then-unregister a worktree →
    /// cache returns Miss after unregister").
    #[test]
    fn registry_unregister_invalidates_cache_via_hook() {
        use crate::registry::SessionRegistry;
        use anvil_intercept_proto::SessionId;
        use std::time::Instant;

        let dir = TempDir::new().unwrap();
        let canonical_key = WorktreeKey::canonicalise(dir.path()).unwrap();
        let cache = Arc::new(RuleSetCache::new());

        // Daemon-style wire-up: a clone of the cache lands in the
        // hook closure; the registry holds it via `Arc<dyn Fn ...>`.
        let cache_for_hook = Arc::clone(&cache);
        let registry =
            SessionRegistry::new().with_unregister_hook(Arc::new(move |worktree_path| {
                let key = WorktreeKey::from_canonical(worktree_path.to_path_buf());
                cache_for_hook.invalidate(&key);
            }));

        // Populate the cache against the worktree.
        cache
            .get_or_resolve::<_, ()>(&canonical_key, |_| Ok(entry("v1")))
            .unwrap();
        assert!(matches!(cache.lookup(&canonical_key), CacheOutcome::Hit(_)));

        // Register, then unregister — the hook fires and clears the
        // cache entry behind the scenes.
        let sid = SessionId::new("hook-int-test");
        registry
            .register(&sid, dir.path(), None, Instant::now())
            .unwrap();
        assert!(
            matches!(cache.lookup(&canonical_key), CacheOutcome::Hit(_)),
            "registration alone does not touch the cache",
        );
        registry.unregister(&sid).unwrap();
        assert_eq!(
            cache.lookup(&canonical_key),
            CacheOutcome::Miss,
            "post-unregister lookup must miss"
        );
    }

    /// Concurrent inserts driving evictions and concurrent
    /// invalidations against arbitrary keys must not deadlock.
    /// Pins the `insert_with_eviction` single-lock design against a
    /// future refactor that splits eviction and insert into separate
    /// lock acquisitions.
    #[test]
    fn concurrent_insert_and_evict_do_not_deadlock() {
        const ITERATIONS: usize = 200;
        let cache = Arc::new(RuleSetCache::with_capacity(8));
        let dirs: Vec<TempDir> = (0..32).map(|_| TempDir::new().unwrap()).collect();
        let keys: Vec<WorktreeKey> = dirs.iter().map(key).collect();

        let inserter_keys = keys.clone();
        let inserter_cache = Arc::clone(&cache);
        let inserter = thread::spawn(move || {
            for i in 0..ITERATIONS {
                let k = &inserter_keys[i % inserter_keys.len()];
                let sha = format!("v{i}");
                let _ = inserter_cache.get_or_resolve::<_, ()>(k, |_| Ok(entry(&sha)));
            }
        });
        let invalidator_keys = keys.clone();
        let invalidator_cache = Arc::clone(&cache);
        let invalidator = thread::spawn(move || {
            for i in 0..ITERATIONS {
                let k = &invalidator_keys[i % invalidator_keys.len()];
                invalidator_cache.invalidate(k);
            }
        });

        inserter.join().unwrap();
        invalidator.join().unwrap();
        // Final state: cache must respect its bound, eviction
        // counter must be a sane (non-decreasing) value.
        assert!(cache.len() <= 8, "capacity bound holds under contention");
    }

    // MLP2-058: invalidation counter + tracing-event surface.

    /// `invalidate` on a present key bumps the invalidation counter
    /// by exactly 1. A no-op invalidate (missing key) does NOT bump.
    /// Pin so the counter only reflects effective drops — that's the
    /// signal operators read for pressure.
    #[test]
    fn invalidate_counter_only_counts_effective_drops() {
        let cache = RuleSetCache::new();
        let dir = TempDir::new().unwrap();
        let k = key(&dir);
        cache
            .get_or_resolve::<_, ()>(&k, |_| Ok(entry("v1")))
            .unwrap();
        assert_eq!(cache.invalidations(), 0);

        // Effective drop -> counter bumps.
        assert!(cache.invalidate(&k));
        assert_eq!(cache.invalidations(), 1);

        // No-op invalidate -> counter unchanged.
        assert!(!cache.invalidate(&k));
        assert_eq!(cache.invalidations(), 1);
    }

    /// `invalidate_on_change` against a recognised `.anvil.*` file
    /// that hits one entry bumps the counter by 1. A burst that hits
    /// N entries (e.g. cache-wide flush on canonicalise failure)
    /// bumps by exactly N — the field is cumulative, not per-call.
    #[test]
    fn invalidate_on_change_counter_matches_dropped_entries() {
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
        assert_eq!(cache.invalidations(), 0);

        let touched_a = dir_a.path().join(".anvil.yaml");
        std::fs::write(&touched_a, b"{}").unwrap();
        let hits = cache.invalidate_on_change(&touched_a);
        assert_eq!(hits.len(), 1);
        assert_eq!(cache.invalidations(), 1);

        // Phantom path -> over-flush both entries; counter += 1
        // (only one survives the previous step).
        let phantom = Path::new("/nonexistent/anvil-mlp2-058-test/.anvil.yaml");
        let hits = cache.invalidate_on_change(phantom);
        assert_eq!(hits.len(), 1, "k_b is the only remaining entry");
        assert_eq!(cache.invalidations(), 2);
    }

    /// `invalidate_on_change` against an unrelated file (not a
    /// `.anvil.*`) is a hard no-op and MUST NOT bump the counter.
    /// Closes the spam-the-counter hole an attacker writing
    /// arbitrary files could otherwise exploit.
    #[test]
    fn invalidate_on_change_unrelated_file_does_not_bump_counter() {
        let cache = RuleSetCache::new();
        let dir = TempDir::new().unwrap();
        let k = key(&dir);
        cache
            .get_or_resolve::<_, ()>(&k, |_| Ok(entry("v1")))
            .unwrap();
        let unrelated = dir.path().join("Cargo.toml");
        std::fs::write(&unrelated, b"# unrelated").unwrap();
        let hits = cache.invalidate_on_change(&unrelated);
        assert!(hits.is_empty());
        assert_eq!(cache.invalidations(), 0);
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
