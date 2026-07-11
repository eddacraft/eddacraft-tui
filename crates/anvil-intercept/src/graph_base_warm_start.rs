//! GBASE-006 (ADR-105 §1/§3): the **per-worktree warm-start** that materialises a
//! resident graph from the shared base plus the worktree's live overlay.
//!
//! This is the intercept-side installer that stitches the three pieces together:
//!
//! ```text
//!   base_store::load_base ─▶ overlay_scan::compute_overlay ─▶ graph_cache::compose ─▶ cache.restore
//!        (GBASE-002)               (GBASE-004)                     (GBASE-006)          (DSV-030 seam)
//! ```
//!
//! It **mirrors the cold per-worktree restore path** (`save_time::restore_snapshot_into_cache`)
//! exactly, swapping the on-disk source: instead of loading a per-worktree
//! snapshot it loads the **shared base** (shared on disk across every worktree of
//! the merge-base), computes the worktree's overlay, composes them, and installs
//! the result through the **same** [`KernelGraphCache::restore`] seam. `restore`
//! marks the entry a **restored stand-in**, so the composed workspace comes up
//! **stale** and cannot certify until the content-hash reconcile completes
//! (ADR-105 §4, the inherited ADR-069 trust line) — the trust line is enforced by
//! reusing the exact same installer the snapshot path uses, not by a parallel
//! mechanism.
//!
//! # Failure posture — every non-`Composed` outcome serves cold (non-fatal)
//!
//! Routing on [`BaseLoadOutcome`] and the fallible overlay is
//! discard-and-serve-cold throughout (ADR-105 §6): an absent/ignored base, an
//! environmental overlay failure, or an inconsistent base replay all leave the key
//! cold for the ordinary cold-scan path to warm. Nothing here is a hard error.
//!
//! # Scope note (GBASE-006 vs GBASE-009)
//!
//! This module owns the **composition + cache-installation seam** and the minimal
//! background-pool wire ([`crate::save_time::SaveTimeState::spawn_compose_restore`]).
//! It does **not** own the *lifecycle decision* — resolving a worktree's
//! merge-base sha (git, which the resident daemon deliberately never runs) and
//! routing a worktree to the base path vs. the permanent per-worktree path. That
//! re-entrant `persistence_route` topology is **GBASE-009**; it supplies the `sha`
//! and calls this seam. Until then, the seam is exercised directly (and by the
//! save-time wire), the way GBASE-003 shipped its trigger executor ahead of the
//! full event loop.
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
    // Fast pre-check (the authoritative compare-and-insert is `restore` below): a
    // key already warm is left to its authoritative graph, never clobbered.
    if cache.contains(key) {
        return ComposeWarmStartOutcome::AlreadyWarm;
    }

    // Route on the base load (ADR-105 §9). Absent/Ignored ⇒ cold path, no compose.
    let payload = match load_base(base_dir, sha) {
        BaseLoadOutcome::Loaded(payload) => payload,
        BaseLoadOutcome::Absent => return ComposeWarmStartOutcome::ColdBaseAbsent,
        BaseLoadOutcome::Ignored => return ComposeWarmStartOutcome::ColdBaseIgnored,
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
                error = %err,
                "overlay computation failed; serving cold (no compose)",
            );
            return ComposeWarmStartOutcome::ColdOverlayError;
        }
    };

    // Compose the base with the overlay into one materialised pair (GBASE-006). An
    // inconsistent base replay ⇒ cold rebuild.
    let Ok((sym, dep)) = compose(payload, &fragment) else {
        tracing::warn!(
            target: "anvil_intercept::graph_base_warm_start",
            workspace_root = %root.display(),
            "base replayed inconsistently during compose; serving cold",
        );
        return ComposeWarmStartOutcome::ColdComposeError;
    };

    // Install through the SAME seam the snapshot restore uses (DSV-030): a
    // compare-and-insert that only warms a still-cold key and marks it a **restored
    // stand-in** — so the composed workspace comes up stale and cannot certify
    // until the reconcile clears the flag (ADR-105 §4 trust line).
    if cache.restore(key, sym, dep) {
        tracing::info!(
            target: "anvil_intercept::graph_base_warm_start",
            workspace_root = %root.display(),
            "warm-start: composed resident graph from shared base + overlay (stale until reconcile)",
        );
        ComposeWarmStartOutcome::Composed
    } else {
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

    /// Concurrency-sensitive: run the sibling-independence composition 20× so a
    /// stray shared-state regression surfaces under repetition (paired with the
    /// suite's `taskset -c 0,1` pinning in CI/local gate runs).
    #[test]
    fn sibling_independence_holds_over_repetition() {
        for _ in 0..20 {
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
        assert!(cache.restore(&key, sym, DependencyGraph::new()));

        let outcome =
            compose_worktree_from_base(&cache, &key, &base, SHA, wt.path(), &LineParser, &caps());
        assert_eq!(outcome, ComposeWarmStartOutcome::AlreadyWarm);
        assert!(
            cache.warm_files(&key).contains(&"sentinel.ts".to_string()),
            "an already-warm key keeps its authoritative graph (never re-composed)"
        );
    }
}
