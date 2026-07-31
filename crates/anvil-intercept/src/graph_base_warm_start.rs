//! GBASE-006: warm-start a worktree graph from shared base + live overlay.

#![cfg(unix)]

use std::path::Path;

use anvil_graph_cache::compose;

use crate::kernel_cache::KernelGraphCache;
use crate::overlay_scan::compute_overlay;
use crate::rule_cache::WorktreeKey;
use crate::save_time::SymbolParser;
use crate::snapshot_io::base_store::{BaseLoadOutcome, load_base};
use crate::workspace_pool::DosCaps;

/// The outcome of a per-worktree composed warm-start (GBASE-006). Every variant
/// other than [`Self::Composed`] leaves the key cold for the ordinary cold-scan
/// path (all non-fatal, ADR-105 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposeWarmStartOutcome {
    /// A base was loaded, its overlay computed, composed, and installed into the
    /// cache as a **stale** restored stand-in.
    Composed,
    /// The key was already warm (a concurrent scan or restore beat us). The
    /// compare-and-insert in [`KernelGraphCache::restore`] no-oped; the existing
    /// authoritative graph is kept.
    AlreadyWarm,
    /// No base artefact exists for the sha — the normal cold/first-run case.
    ColdBaseAbsent,
    /// A base artefact exists but does not match this build (wrong magic / epoch /
    /// integrity) — ignored on the cold path (ADR-105 §9), left in place for GC.
    ColdBaseIgnored,
    /// The overlay computation failed environmentally (the workspace anchor could
    /// not open). Serving the composed result would be destructive (every base
    /// file would read as deleted), so serve cold instead.
    ColdOverlayError,
    /// The base payload replayed inconsistently (a duplicate id / dangling edge) —
    /// treated as a corrupt base, i.e. cold rebuild.
    ColdComposeError,
}

/// Materialise `key`'s resident graph by composing the shared base for `sha` with
/// the worktree's live overlay, and install it **stale** into `cache` (GBASE-006,
/// ADR-105 §1/§3).
///
/// `base_dir` is the shared base store (`<graph-cache>/base`); `sha` is the
/// worktree's resolved merge-base commit (supplied by the caller — see the module
/// scope note); `root` is the worktree root; `parser` is the injected
/// [`SymbolParser`] (the daemon never parses on its own — ADR-061/064); `caps` are
/// the standard [`DosCaps`].
///
/// Routes on [`BaseLoadOutcome`] (absent/ignored ⇒ cold, no compose), treats a
/// [`compute_overlay`] error as cold, and installs through
/// [`KernelGraphCache::restore`] so the composed workspace comes up **stale**
/// (never `Certified` pre-reconcile). Deterministic and side-effect-free beyond
/// the single cache insert.
#[must_use]
pub fn compose_worktree_from_base(
    cache: &KernelGraphCache,
    key: &WorktreeKey,
    base_dir: &Path,
    sha: &str,
    root: &Path,
    parser: &dyn SymbolParser,
    caps: &DosCaps,
) -> ComposeWarmStartOutcome {
    // Every log on this path carries `sha` + `base_dir` (council MAJOR 3 / ops):
    // the shared base is keyed by merge-base sha, so an operator correlating a
    // worktree's warm-start with a base-production/GC event needs both.
    //
    // Fast pre-check (the authoritative compare-and-insert is `restore` below): a
    // key already warm is left to its authoritative graph, never clobbered.
    if cache.contains(key) {
        tracing::debug!(
            target: "anvil_intercept::graph_base_warm_start",
            workspace_root = %root.display(),
            sha = %sha,
            "warm-start: key already warm; no compose",
        );
        return ComposeWarmStartOutcome::AlreadyWarm;
    }

    // Route on the base load (ADR-105 §9). Absent/Ignored ⇒ cold path, no compose.
    let payload = match load_base(base_dir, sha) {
        BaseLoadOutcome::Loaded(payload) => payload,
        BaseLoadOutcome::Absent => {
            tracing::debug!(
                target: "anvil_intercept::graph_base_warm_start",
                workspace_root = %root.display(),
                sha = %sha,
                base_dir = %base_dir.display(),
                "warm-start: no base artefact for sha (first-run/cold path)",
            );
            return ComposeWarmStartOutcome::ColdBaseAbsent;
        }
        BaseLoadOutcome::Ignored => {
            tracing::info!(
                target: "anvil_intercept::graph_base_warm_start",
                workspace_root = %root.display(),
                sha = %sha,
                base_dir = %base_dir.display(),
                "warm-start: base artefact ignored (wrong class/epoch/integrity); serving cold, left for GC",
            );
            return ComposeWarmStartOutcome::ColdBaseIgnored;
        }
    };

    // Compute the worktree overlay (GBASE-004). The overlay is deliberately
    // fallible: an anchor-open failure would otherwise tombstone the whole base, so
    // an error routes to the cold path rather than a destructive compose.
    let fragment = match compute_overlay(&payload, root, parser, caps) {
        Ok(fragment) => fragment,
        Err(err) => {
            tracing::debug!(
                target: "anvil_intercept::graph_base_warm_start",
                workspace_root = %root.display(),
                sha = %sha,
                base_dir = %base_dir.display(),
                error = %err,
                "overlay computation failed; serving cold (no compose)",
            );
            return ComposeWarmStartOutcome::ColdOverlayError;
        }
    };

    // Compose the base with the overlay into one materialised pair (GBASE-006). An
    // inconsistent base replay ⇒ cold rebuild.
    let (sym, dep) = match compose(payload, &fragment) {
        Ok(pair) => pair,
        Err(err) => {
            // `SnapshotLoadError` is privacy-safe (no paths/bytes), so the
            // value itself is loggable — it distinguishes payload
            // corruption/invariant breaks from unexpected decode/replay errors.
            tracing::warn!(
                target: "anvil_intercept::graph_base_warm_start",
                workspace_root = %root.display(),
                sha = %sha,
                base_dir = %base_dir.display(),
                error = %err,
                "base replayed inconsistently during compose; serving cold",
            );
            return ComposeWarmStartOutcome::ColdComposeError;
        }
    };

    // Install through the SAME seam the snapshot restore uses (DSV-030): a
    // compare-and-insert that only warms a still-cold key and marks it a **restored
    // stand-in** — so the composed workspace comes up stale and cannot certify
    // until the reconcile clears the flag (ADR-105 §4 trust line).
    if cache.restore(key, sym, dep).is_some() {
        tracing::info!(
            target: "anvil_intercept::graph_base_warm_start",
            workspace_root = %root.display(),
            sha = %sha,
            base_dir = %base_dir.display(),
            "warm-start: composed resident graph from shared base + overlay (stale until reconcile)",
        );
        ComposeWarmStartOutcome::Composed
    } else {
        tracing::debug!(
            target: "anvil_intercept::graph_base_warm_start",
            workspace_root = %root.display(),
            sha = %sha,
            "warm-start: key warmed by a concurrent scan/restore before install; composed result dropped",
        );
        ComposeWarmStartOutcome::AlreadyWarm
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    use anvil_graph_cache::snapshot::SnapshotPayload;
    use anvil_graph_cache::{DependencyGraph, SymbolGraph, update_file};
    use anvil_kernel_types::{
        FileSymbols, ImportEdge, SymbolKind, SymbolNode, TrustLevel, Visibility, content_hash,
    };

    use crate::snapshot_io::base_store::publish_base;

    // ---- a path-stable, content-hashing test parser -----------------------
    //
    // Mirrors the `overlay_scan` `LineParser`: `export NAME` → a public symbol,
    // `import ./spec` → an import edge, and every file is content-hashed (the
    // GV2-032 key the base and overlay share). `.ts` only.
    #[derive(Debug, Default)]
    struct LineParser;

    fn stable_id(file: &str, name: &str) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in file.bytes().chain(std::iter::once(0)).chain(name.bytes()) {
            h ^= u64::from(byte);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    }

    impl SymbolParser for LineParser {
        fn parse(&self, path: &Path, bytes: &[u8]) -> Option<FileSymbols> {
            if path.extension().and_then(|e| e.to_str()) != Some("ts") {
                return None;
            }
            let text = std::str::from_utf8(bytes).ok()?;
            let file = path.to_string_lossy().into_owned();
            let mut symbols = Vec::new();
            let mut imports = Vec::new();
            for line in text.lines() {
                let line = line.trim();
                if let Some(name) = line.strip_prefix("export ") {
                    symbols.push(SymbolNode {
                        id: stable_id(&file, name.trim()),
                        kind: SymbolKind::Function,
                        name: name.trim().to_string(),
                        visibility: Visibility::Public,
                        file: file.clone(),
                        trust_level: TrustLevel::Unknown,
                        span: None,
                    });
                } else if let Some(spec) = line.strip_prefix("import ") {
                    imports.push(ImportEdge {
                        from_file: file.clone(),
                        to_source: spec.trim().to_string(),
                        line: 0,
                    });
                }
            }
            Some(FileSymbols {
                file,
                symbols,
                imports,
                reexports: Vec::new(),
                calls: Vec::new(),
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: Some(content_hash(bytes)),
            })
        }
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(path, body).expect("write file");
    }

    /// Build a base payload from on-disk files (parsed through the same parser the
    /// overlay uses, so hash provenance is shared) and its dependency graph.
    fn base_payload_from_files(
        root: &Path,
        files: &[(&str, &str)],
        parser: &dyn SymbolParser,
    ) -> SnapshotPayload {
        let mut sym = SymbolGraph::new();
        for (rel, body) in files {
            write(root, rel, body);
            if let Some(fs) = parser.parse(Path::new(rel), body.as_bytes()) {
                update_file(&mut sym, fs);
            }
        }
        // Derive the Imports-only dependency graph so the base carries a forward map.
        let mut dep = DependencyGraph::new();
        for node in sym.inner().node_weights() {
            for edge in sym.outgoing_edges(node.id) {
                if edge.edge_type != anvil_kernel_types::EdgeType::Imports {
                    continue;
                }
                if let (Some(f), Some(t)) = (sym.get_symbol(edge.from), sym.get_symbol(edge.to))
                    && f.file != t.file
                {
                    dep.add_dependency(f.file.clone(), t.file.clone());
                }
            }
        }
        SnapshotPayload::from_graphs(&sym, &dep).expect("base payload builds")
    }

    /// Publish a base for `sha` into `base_dir` from an on-disk fixture, returning
    /// the payload used (for parity comparisons).
    fn publish_base_from_files(
        base_dir: &Path,
        sha: &str,
        root: &Path,
        files: &[(&str, &str)],
        parser: &dyn SymbolParser,
    ) -> SnapshotPayload {
        let payload = base_payload_from_files(root, files, parser);
        publish_base(base_dir, sha, &payload.to_base_bytes()).expect("publish base");
        payload
    }

    fn key_for(root: &Path) -> WorktreeKey {
        WorktreeKey::from_canonical(root.to_path_buf())
    }

    fn caps() -> DosCaps {
        DosCaps::default()
    }

    const SHA: &str = "abc1234500000000000000000000000000000000";

    // ---- (e) load_base Absent / Ignored ⇒ cold path (no compose) ----------

    #[test]
    fn absent_base_serves_cold_without_composing() {
        let store = tempfile::tempdir().unwrap();
        let base = store.path().join("base");
        let wt = tempfile::tempdir().unwrap();
        write(wt.path(), "a.ts", "export a");
        let cache = KernelGraphCache::new();
        let key = key_for(wt.path());

        // No base was ever published for SHA ⇒ Absent ⇒ cold.
        let outcome =
            compose_worktree_from_base(&cache, &key, &base, SHA, wt.path(), &LineParser, &caps());
        assert_eq!(outcome, ComposeWarmStartOutcome::ColdBaseAbsent);
        assert!(
            !cache.contains(&key),
            "no base ⇒ nothing installed (cold path)"
        );
    }

    #[test]
    fn ignored_base_serves_cold_without_composing() {
        let store = tempfile::tempdir().unwrap();
        let base = store.path().join("base");
        let wt = tempfile::tempdir().unwrap();
        write(wt.path(), "a.ts", "export a");
        let cache = KernelGraphCache::new();
        let key = key_for(wt.path());

        // Publish a garbage artefact at the base leaf so load_base decodes it as
        // Ignored (wrong magic / corrupt), not Loaded. Write the per-worktree
        // (ANVILGC1) bytes as a base — the base loader refuses the wrong magic.
        let payload = base_payload_from_files(wt.path(), &[("a.ts", "export a")], &LineParser);
        // `to_bytes` (ANVILGC1) fed to the base leaf ⇒ BadMagic ⇒ Ignored.
        crate::snapshot_io::store::write_sealed(&base, &format!("{SHA}.base"), &payload.to_bytes())
            .expect("write wrong-class artefact");

        let outcome =
            compose_worktree_from_base(&cache, &key, &base, SHA, wt.path(), &LineParser, &caps());
        assert_eq!(outcome, ComposeWarmStartOutcome::ColdBaseIgnored);
        assert!(
            !cache.contains(&key),
            "ignored base ⇒ cold path, no install"
        );
    }

    // ---- (f) compute_overlay error ⇒ cold path ----------------------------

    #[test]
    fn overlay_error_serves_cold_without_composing() {
        let store = tempfile::tempdir().unwrap();
        let base = store.path().join("base");
        let wt = tempfile::tempdir().unwrap();
        // Publish a real base, then point the warm-start at a NON-EXISTENT root so
        // the workspace anchor cannot open ⇒ compute_overlay errors ⇒ cold.
        publish_base_from_files(&base, SHA, wt.path(), &[("a.ts", "export a")], &LineParser);
        let missing = wt.path().join("no-such-dir");
        let cache = KernelGraphCache::new();
        let key = key_for(&missing);

        let outcome =
            compose_worktree_from_base(&cache, &key, &base, SHA, &missing, &LineParser, &caps());
        assert_eq!(outcome, ComposeWarmStartOutcome::ColdOverlayError);
        assert!(
            !cache.contains(&key),
            "an overlay error must never install a (destructive) composed graph"
        );
    }

    // ---- Composed happy path: base + clean worktree installs stale --------

    #[test]
    fn clean_worktree_composes_and_installs_stale() {
        let store = tempfile::tempdir().unwrap();
        let base = store.path().join("base");
        let wt = tempfile::tempdir().unwrap();
        // Base == worktree (clean): the compose is the base, installed stale.
        publish_base_from_files(
            &base,
            SHA,
            wt.path(),
            &[("a.ts", "export a\nimport ./b"), ("b.ts", "export b")],
            &LineParser,
        );
        let cache = KernelGraphCache::new();
        let key = key_for(wt.path());

        let outcome =
            compose_worktree_from_base(&cache, &key, &base, SHA, wt.path(), &LineParser, &caps());
        assert_eq!(outcome, ComposeWarmStartOutcome::Composed);
        assert!(cache.contains(&key), "the composed graph is installed");
        assert!(
            cache.is_restored(&key),
            "the composed workspace is a restored stand-in (stale)"
        );
        // The resident graph holds the base's two files.
        let mut files = cache.warm_files(&key);
        files.sort();
        assert_eq!(files, vec!["a.ts".to_string(), "b.ts".to_string()]);
    }

    // ---- (c) sibling worktrees: one on-disk base, two independent graphs ----

    #[test]
    fn sibling_worktrees_share_base_hold_independent_graphs() {
        let store = tempfile::tempdir().unwrap();
        let base = store.path().join("base");
        let wt1 = tempfile::tempdir().unwrap();
        let wt2 = tempfile::tempdir().unwrap();
        let files = [("a.ts", "export a"), ("b.ts", "export b")];
        // ONE on-disk base artefact, published once for the shared merge-base sha.
        let payload = publish_base_from_files(&base, SHA, wt1.path(), &files, &LineParser);
        // The second worktree checks out the SAME committed files (its own copy on
        // disk) — it composes from the SAME shared base artefact.
        for (rel, body) in files {
            write(wt2.path(), rel, body);
        }

        // Exactly one base artefact exists on disk.
        let base_count = std::fs::read_dir(&base)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("base"))
            .count();
        assert_eq!(base_count, 1, "a single shared base artefact on disk");
        let _ = &payload;

        // Both worktrees compose into ONE shared cache under distinct keys.
        let cache = KernelGraphCache::with_capacity(8);
        let k1 = key_for(wt1.path());
        let k2 = key_for(wt2.path());
        assert_eq!(
            compose_worktree_from_base(&cache, &k1, &base, SHA, wt1.path(), &LineParser, &caps()),
            ComposeWarmStartOutcome::Composed
        );
        assert_eq!(
            compose_worktree_from_base(&cache, &k2, &base, SHA, wt2.path(), &LineParser, &caps()),
            ComposeWarmStartOutcome::Composed
        );

        // Both hold b.ts before the mutation.
        assert!(cache.warm_files(&k1).contains(&"b.ts".to_string()));
        assert!(cache.warm_files(&k2).contains(&"b.ts".to_string()));

        // Mutate worktree 1's resident graph: drop b.ts via a Delete apply.
        cache.apply_delta(
            &k1,
            anvil_graph_cache::certify::ChangeKind::Delete,
            FileSymbols {
                file: "b.ts".to_string(),
                symbols: Vec::new(),
                imports: Vec::new(),
                reexports: Vec::new(),
                calls: Vec::new(),
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: None,
            },
        );

        // Worktree 1 lost b.ts; worktree 2's independent resident graph is untouched.
        assert!(
            !cache.warm_files(&k1).contains(&"b.ts".to_string()),
            "worktree 1's graph dropped b.ts"
        );
        assert!(
            cache.warm_files(&k2).contains(&"b.ts".to_string()),
            "sibling worktree 2 holds an INDEPENDENT resident graph (b.ts intact)"
        );
    }

    /// Determinism smoke (council MINOR g): compose is single-threaded and
    /// deterministic, so sibling independence is a structural property, not a race
    /// — a few iterations suffice to catch a stray shared-state regression (e.g. an
    /// accidental `Arc`-shared graph). Kept small deliberately; the heavier
    /// `taskset`-pinned repetition lives at the gate, not inline.
    #[test]
    fn sibling_independence_holds_over_repetition() {
        for _ in 0..3 {
            let store = tempfile::tempdir().unwrap();
            let base = store.path().join("base");
            let wt1 = tempfile::tempdir().unwrap();
            let wt2 = tempfile::tempdir().unwrap();
            let files = [("a.ts", "export a"), ("b.ts", "export b")];
            publish_base_from_files(&base, SHA, wt1.path(), &files, &LineParser);
            for (rel, body) in files {
                write(wt2.path(), rel, body);
            }
            let cache = KernelGraphCache::with_capacity(8);
            let k1 = key_for(wt1.path());
            let k2 = key_for(wt2.path());
            let _ = compose_worktree_from_base(
                &cache,
                &k1,
                &base,
                SHA,
                wt1.path(),
                &LineParser,
                &caps(),
            );
            let _ = compose_worktree_from_base(
                &cache,
                &k2,
                &base,
                SHA,
                wt2.path(),
                &LineParser,
                &caps(),
            );
            cache.invalidate(&k1);
            assert!(
                cache.warm_files(&k2).contains(&"b.ts".to_string()),
                "invalidating one sibling must not disturb the other"
            );
        }
    }

    // ---- (d) STALE pre-reconcile: composed workspace cannot certify -------

    #[test]
    fn composed_workspace_cannot_certify_before_reconcile() {
        use anvil_checks::antipattern::types::AntipatternCheckConfig;
        use anvil_intercept_proto::protocol::{
            AssuranceState, ChangeDescriptor, ChangeKindWire, Coverage, ValidatePathsRequest,
        };

        use crate::assurance::AssuranceMachine;
        use crate::validate_paths::{ValidateEnv, validate_paths};

        let store = tempfile::tempdir().unwrap();
        let base = store.path().join("base");
        let wt = tempfile::tempdir().unwrap();
        publish_base_from_files(
            &base,
            SHA,
            wt.path(),
            &[("src/a.ts", "export foo")],
            &LineParser,
        );
        let cache = KernelGraphCache::new();
        let key = key_for(wt.path());
        assert_eq!(
            compose_worktree_from_base(&cache, &key, &base, SHA, wt.path(), &LineParser, &caps()),
            ComposeWarmStartOutcome::Composed
        );
        assert!(
            cache.is_restored(&key),
            "the composed entry is a restored (stale) stand-in"
        );

        // Drive the REAL validate_paths machinery. A body-only edit that resolves
        // self-contained would otherwise certify; the restored stand-in forces it
        // non-Certified and keeps the workspace non-Clean until reconcile.
        let clean = b"export function foo() { return 1; }".to_vec();
        let fed = |p: &str, _: &[u8]| {
            (p == "src/a.ts")
                .then(|| LineParser.parse(Path::new("src/a.ts"), b"export foo"))
                .flatten()
        };
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("pool");
        let config = AntipatternCheckConfig::default();
        let dos = caps();
        let mut assurance = AssuranceMachine::new();
        let resp = validate_paths(
            &ValidatePathsRequest {
                workspace_root: wt.path().to_string_lossy().into_owned(),
                paths: vec![ChangeDescriptor {
                    path: "src/a.ts".to_string(),
                    change: ChangeKindWire::Modified,
                    content_hash: None,
                    mtime: None,
                }],
            },
            &cache,
            &mut assurance,
            |_p| Ok(clean.clone()),
            fed,
            &ValidateEnv {
                config: &config,
                pool: &pool,
                budget: 64,
                reverse_impact_depth: 1,
                caps: &dos,
            },
        );

        assert_eq!(
            resp.coverage,
            Coverage::Partial,
            "a composed (restored/stale) workspace must never certify pre-reconcile"
        );
        assert_ne!(
            resp.workspace_assurance.state,
            AssuranceState::Clean,
            "the composed workspace stays non-Clean until the reconcile completes"
        );
    }

    // ---- already-warm key is not clobbered --------------------------------

    #[test]
    fn already_warm_key_is_not_recomposed() {
        let store = tempfile::tempdir().unwrap();
        let base = store.path().join("base");
        let wt = tempfile::tempdir().unwrap();
        publish_base_from_files(&base, SHA, wt.path(), &[("a.ts", "export a")], &LineParser);
        let cache = KernelGraphCache::new();
        let key = key_for(wt.path());

        // Pre-warm the key with a distinct graph.
        let mut sym = SymbolGraph::new();
        update_file(
            &mut sym,
            FileSymbols {
                file: "sentinel.ts".to_string(),
                symbols: vec![SymbolNode {
                    id: 0,
                    kind: SymbolKind::Function,
                    name: "sentinel".to_string(),
                    visibility: Visibility::Public,
                    file: "sentinel.ts".to_string(),
                    trust_level: TrustLevel::Unknown,
                    span: None,
                }],
                imports: Vec::new(),
                reexports: Vec::new(),
                calls: Vec::new(),
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: None,
            },
        );
        assert!(cache.restore(&key, sym, DependencyGraph::new()).is_some());

        let outcome =
            compose_worktree_from_base(&cache, &key, &base, SHA, wt.path(), &LineParser, &caps());
        assert_eq!(outcome, ComposeWarmStartOutcome::AlreadyWarm);
        assert!(
            cache.warm_files(&key).contains(&"sentinel.ts".to_string()),
            "an already-warm key keeps its authoritative graph (never re-composed)"
        );
    }

    // ========================================================================
    // GBASE-010 graduation-gate evidence — warm-start latency (N-sibling shape)
    //   + corrupt-shared-base incident behaviour
    // ========================================================================

    /// A [`SymbolParser`] that delegates to [`LineParser`] but counts every parse
    /// call, so the warm-start win can be demonstrated **deterministically** (a
    /// parse count, not a flaky wall-clock number): the shared base is parsed
    /// **once** at production, and a clean worktree's warm-start re-parses **zero**
    /// files (the base holds the unchanged majority — GBASE-004), whereas a cold
    /// scan re-parses every file, per worktree.
    #[derive(Debug, Default)]
    struct CountingParser {
        parses: std::sync::atomic::AtomicU64,
    }

    impl SymbolParser for CountingParser {
        fn parse(&self, path: &Path, bytes: &[u8]) -> Option<FileSymbols> {
            self.parses
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            LineParser.parse(path, bytes)
        }
    }

    impl CountingParser {
        fn count(&self) -> u64 {
            self.parses.load(std::sync::atomic::Ordering::Relaxed)
        }
        fn reset(&self) {
            self.parses.store(0, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// **GBASE-010 §11 criterion: warm-start latency budget (N-worktree shape).**
    ///
    /// ADR-105 §11 gates the flip on base load staying within a warm-start budget
    /// when it sits on the cold-start critical path for **N worktrees**. The honest,
    /// non-flaky expression of that budget is the **re-parse ratio**, not absolute
    /// milliseconds (agent-shell wall-clock is indicative only — see the gate doc's
    /// environment caveat, and the memory lesson that benches are flaky in agent
    /// shells):
    ///
    /// - **Cold fleet:** each of `FLEET` worktrees does a full cold scan, re-parsing
    ///   every one of `FILES` files ⇒ `FLEET × FILES` parses.
    /// - **Warm fleet (shared base):** the base is produced **once** (`FILES`
    ///   parses), then every worktree warm-starts from that **one** shared artefact,
    ///   and a clean worktree's overlay re-parses **zero** files ⇒ `FILES` parses
    ///   total, regardless of `FLEET`.
    ///
    /// So the shared base cuts fleet-wide parse work by exactly `FLEET×`. The parser
    /// here is trivial (line-splitting), so this deterministic parse-count ratio is
    /// a **lower bound** on the real win, where tree-sitter parse cost dominates a
    /// cold scan; the wall-clock is reported alongside purely as an indicator.
    #[test]
    fn warm_start_shared_base_reparses_fleet_times_fewer_than_cold() {
        const FILES: usize = 60;
        const FLEET: usize = 16;

        // A representative multi-file fixture (one cross-file import chain so the
        // dependency map is exercised, not just isolated symbols).
        let fixture: Vec<(String, String)> = (0..FILES)
            .map(|i| {
                let body = if i == 0 {
                    "export f0".to_string()
                } else {
                    format!("export f{i}\nimport ./mod{}", i - 1)
                };
                (format!("mod{i}.ts"), body)
            })
            .collect();
        let fixture_ref: Vec<(&str, &str)> = fixture
            .iter()
            .map(|(p, b)| (p.as_str(), b.as_str()))
            .collect();

        // ---- Cold fleet: FLEET full scans, each re-parsing every file. ----------
        let cold_counter = CountingParser::default();
        let cold_start = std::time::Instant::now();
        for _ in 0..FLEET {
            let wt = tempfile::tempdir().unwrap();
            // A cold scan builds the whole per-worktree graph from source.
            let _ = base_payload_from_files(wt.path(), &fixture_ref, &cold_counter);
        }
        let cold_wall = cold_start.elapsed();
        let cold_parses = cold_counter.count();

        // ---- Warm fleet: produce ONE shared base, then FLEET clean warm-starts. --
        let warm_counter = CountingParser::default();
        let store = tempfile::tempdir().unwrap();
        let base = store.path().join("base");
        let producer_wt = tempfile::tempdir().unwrap();
        // Produce the shared base once (this is the amortised one-time parse cost).
        publish_base_from_files(&base, SHA, producer_wt.path(), &fixture_ref, &warm_counter);
        let base_production_parses = warm_counter.count();
        warm_counter.reset();

        let warm_start = std::time::Instant::now();
        let cache = KernelGraphCache::with_capacity(FLEET);
        for _ in 0..FLEET {
            // Each worktree checks out the SAME committed files (a clean sibling).
            let wt = tempfile::tempdir().unwrap();
            for (rel, body) in &fixture_ref {
                write(wt.path(), rel, body);
            }
            let key = key_for(wt.path());
            assert_eq!(
                compose_worktree_from_base(
                    &cache,
                    &key,
                    &base,
                    SHA,
                    wt.path(),
                    &warm_counter,
                    &caps()
                ),
                ComposeWarmStartOutcome::Composed,
                "every clean worktree warm-starts from the one shared base",
            );
        }
        let warm_wall = warm_start.elapsed();
        let warm_compose_parses = warm_counter.count();

        // Indicative wall-clock (NOT asserted — see the environment caveat).
        eprintln!(
            "[GBASE-010 warm-start] FILES={FILES} FLEET={FLEET} \
             cold: {cold_parses} parses / {cold_wall:?} | \
             warm: {base_production_parses} base-production parses + \
             {warm_compose_parses} compose parses / {warm_wall:?}",
        );

        // (1) The shared base is parsed exactly once at production.
        assert_eq!(
            base_production_parses, FILES as u64,
            "base production parses every file exactly once",
        );
        // (2) A clean worktree's warm-start re-parses ZERO files (the base holds the
        //     unchanged majority) — so the whole fleet's warm-start parse cost is 0.
        assert_eq!(
            warm_compose_parses, 0,
            "a clean-worktree warm-start must re-parse nothing (GBASE-004 hash skip)",
        );
        // (3) Cold re-parses every file per worktree.
        assert_eq!(
            cold_parses,
            (FLEET * FILES) as u64,
            "the cold fleet re-parses every file, per worktree",
        );
        // (4) The shared-artefact win: fleet-wide warm parse work (one production +
        //     zero per worktree) is exactly FLEET× smaller than the cold fleet's.
        let warm_total = base_production_parses + warm_compose_parses;
        assert_eq!(
            cold_parses,
            warm_total * FLEET as u64,
            "shared base cuts fleet re-parse work by exactly FLEET×",
        );
    }

    /// **GBASE-010 §11 criterion: corrupt-shared-base incident behaviour.**
    ///
    /// ADR-105 §6/§9: a corrupt shared artefact must be **non-fatal and
    /// non-poisoning** — every consumer discards it and cold-serves (nothing bad is
    /// installed into any worktree's resident graph), and once the base is refreshed
    /// (the produce path heals a corrupt artefact in place, §5) every consumer
    /// recovers to a composed warm-start. No cross-worktree poison persists.
    #[test]
    fn corrupt_shared_base_all_consumers_cold_serve_then_recover() {
        const FLEET: usize = 8;
        let files = [
            ("a.ts", "export a\nimport ./b"),
            ("b.ts", "export b"),
            ("c.ts", "export c"),
        ];

        let store = tempfile::tempdir().unwrap();
        let base = store.path().join("base");
        let producer_wt = tempfile::tempdir().unwrap();
        // A valid shared base exists first.
        publish_base_from_files(&base, SHA, producer_wt.path(), &files, &LineParser);
        assert!(matches!(
            crate::snapshot_io::base_store::load_base(&base, SHA),
            crate::snapshot_io::base_store::BaseLoadOutcome::Loaded(_)
        ));

        // A fleet of clean consumer worktrees (same committed files on disk).
        let consumers: Vec<tempfile::TempDir> = (0..FLEET)
            .map(|_| {
                let wt = tempfile::tempdir().unwrap();
                for (rel, body) in files {
                    write(wt.path(), rel, body);
                }
                wt
            })
            .collect();

        // Corrupt the SHARED artefact in place (a torn write / bit-rot): overwrite
        // the sealed base leaf with garbage so `load_base` classifies it Ignored.
        let leaf = base.join(format!("{SHA}.base"));
        std::fs::write(&leaf, b"not-a-sealed-anvil-base-artefact\x00\xff").unwrap();
        assert!(matches!(
            crate::snapshot_io::base_store::load_base(&base, SHA),
            crate::snapshot_io::base_store::BaseLoadOutcome::Ignored
        ));

        // Every consumer discards the corrupt base and serves cold — NOTHING is
        // installed into any resident graph (no cross-worktree poison).
        let cache = KernelGraphCache::with_capacity(FLEET);
        let keys: Vec<WorktreeKey> = consumers.iter().map(|wt| key_for(wt.path())).collect();
        for (wt, key) in consumers.iter().zip(&keys) {
            let outcome = compose_worktree_from_base(
                &cache,
                key,
                &base,
                SHA,
                wt.path(),
                &LineParser,
                &caps(),
            );
            assert_eq!(
                outcome,
                ComposeWarmStartOutcome::ColdBaseIgnored,
                "a corrupt shared base is discarded (cold-serve), never composed",
            );
            assert!(
                !cache.contains(key),
                "a corrupt base must install nothing — no poison in any worktree",
            );
        }

        // Refresh the shared base (the produce path heals a corrupt artefact at the
        // same content-addressed sha, ADR-105 §5): a fresh publish over the corrupt
        // bytes writes (it is not `AlreadyPresent`, since the corrupt one is Ignored).
        let refreshed = base_payload_from_files(producer_wt.path(), &files, &LineParser);
        assert_eq!(
            crate::snapshot_io::base_store::publish_base(&base, SHA, &refreshed.to_base_bytes())
                .unwrap(),
            crate::snapshot_io::base_store::PublishOutcome::Written,
            "refreshing over a corrupt base writes (heals in place)",
        );

        // Every consumer now recovers to a composed warm-start.
        for (wt, key) in consumers.iter().zip(&keys) {
            let outcome = compose_worktree_from_base(
                &cache,
                key,
                &base,
                SHA,
                wt.path(),
                &LineParser,
                &caps(),
            );
            assert_eq!(
                outcome,
                ComposeWarmStartOutcome::Composed,
                "after refresh, every consumer recovers to a warm compose",
            );
            assert!(cache.contains(key), "the recovered worktree is warm");
        }
    }

    /// **GBASE-010 §11 companion: the win scales with the unchanged majority.**
    ///
    /// The clean-worktree harness proves the best case (0 re-parses). Real
    /// worktrees carry a few dirty files; this shows the win is **proportional to
    /// the unchanged majority**: a warm-start over a worktree with `DIRTY` changed
    /// files out of `FILES` re-parses **exactly `DIRTY`** files (the changed
    /// minority), not all `FILES` — the base supplies the unchanged rest. So the
    /// per-worktree re-parse ratio is `FILES / DIRTY`, degrading gracefully from
    /// the clean-worktree `FILES / 0` toward a cold scan only as a worktree
    /// approaches fully-dirty.
    #[test]
    fn warm_start_dirty_worktree_reparses_only_the_changed_minority() {
        const FILES: usize = 40;
        const DIRTY: usize = 3;

        let base_fixture: Vec<(String, String)> = (0..FILES)
            .map(|i| (format!("mod{i}.ts"), format!("export f{i}")))
            .collect();
        let base_ref: Vec<(&str, &str)> = base_fixture
            .iter()
            .map(|(p, b)| (p.as_str(), b.as_str()))
            .collect();

        let store = tempfile::tempdir().unwrap();
        let base = store.path().join("base");
        let producer_wt = tempfile::tempdir().unwrap();
        publish_base_from_files(&base, SHA, producer_wt.path(), &base_ref, &LineParser);

        // A worktree that checks out the base files, then dirties DIRTY of them
        // (new bodies ⇒ new content hashes ⇒ the overlay must re-parse exactly those).
        let wt = tempfile::tempdir().unwrap();
        for (rel, body) in &base_ref {
            write(wt.path(), rel, body);
        }
        for i in 0..DIRTY {
            write(
                wt.path(),
                &format!("mod{i}.ts"),
                &format!("export f{i}\nexport extra{i}"),
            );
        }

        let counter = CountingParser::default();
        let cache = KernelGraphCache::new();
        let key = key_for(wt.path());
        assert_eq!(
            compose_worktree_from_base(&cache, &key, &base, SHA, wt.path(), &counter, &caps()),
            ComposeWarmStartOutcome::Composed,
        );

        eprintln!(
            "[GBASE-010 warm-start dirty] FILES={FILES} DIRTY={DIRTY} \
             compose parses={} (cold would parse {FILES})",
            counter.count(),
        );
        // Exactly the changed minority is re-parsed — the unchanged majority is
        // served from the base with zero parse work.
        assert_eq!(
            counter.count(),
            DIRTY as u64,
            "warm-start re-parses only the changed minority, not the whole worktree",
        );
    }
}
