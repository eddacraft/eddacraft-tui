# Daemon + Graph Architecture Review — Council Verdict

**Date:** 2026-06-01
**Reviewers:** Review council — architect, kernel-maintainer, adversarial-reviewer, operations-reviewer, security-analyst, pragmatic-lead; synthesised by council-judge
**Scope:** `origin/main` at `96c963833`; ADR-061, ADR-063, the daemon save-time validation contract, the Sub-phase A implementation plan, Graph v2 foundation APS, and current kernel/intercept code.
**Inputs:** the prior independent Codex review (`2026-06-01-daemon-graph-architecture-review.md`) was supplied as one input and adjudicated finding-by-finding.

## Verdict (council-judge synthesis)

## 1. Overall verdict

**NO — do not start Sub-phase A as currently specified; resolve the two compile-time/soundness blockers first, then start.** All six personas independently converged on the same root defect: the certify path is built on a `DependencyGraph.reverse` index that **no production code in `crates/anvil-intercept/` ever constructs** (grep returns zero), while `certify()` is typed against `&SymbolGraph` only — so Task 6 as written cannot compile and the load-bearing soundness property (revalidate unchanged importers) cannot be implemented from the state Task 7 caches. A second critical, equally unanimous, is that `coverage: certified` is a **false attestation**: the only diagnostic engine wired (`run_antipattern_check`) is a stateless regex scanner that never runs the four graph-policy invariants the gate exists to enforce. These are not documentation gaps. The ADRs and wire *shape* are directionally sound; the *implementation grounding* is not. The pragmatic-lead is correct that the fixes are additive (days, not a GV2 dependency) and that start can begin the moment Task 6/7 are amended — but until they are, the plan is unbuildable.

---

## 2. Confirmed blockers

### B1 — `DependencyGraph.reverse` is never built in any production path; `certify()` cannot reach `dependents_of` — CRITICAL
**Raised by:** Codex (Critical #2) + **all 6 personas** (architect, kernel-maintainer, adversarial, operations, security, pragmatic-lead). **AGREED — strongest consensus in the council.**
**Evidence:** `certify(graph: &SymbolGraph, …)` at `plans/execution/2026-06-01-daemon-save-time-subphase-a.md:162`; `dependents_of` lives only on `DependencyGraph` (`crates/anvil-kernel/src/graph/dependency.rs:40-45`); Task 7 caches only `HashMap<WorktreeKey, SymbolGraph>` (subphase-a.md:174); `grep DependencyGraph crates/anvil-intercept/src/` → **zero results** (adversarial, pragmatic-lead). The architect/security sharpen this further: `add_dependency`/`dependents_of` have **zero non-test callers anywhere** (`dependency.rs:145-219` are test bodies; `embedded.rs:111` and kernel `watch.rs:81/262/461` build only `SymbolGraph`) — so ADR-061 §6 / contract §3:157-160 calling it "existing" and "O(1)" is **factually wrong**, not an ownership gap.
**Required fix:** Task 7 caches a `(SymbolGraph, DependencyGraph)` pair per `WorktreeKey`; cold-build derives `DependencyGraph` from the resolved import edges and `apply_delta` maintains the reverse index incrementally; Task 6 signature becomes `certify(sym_graph, dep_graph, change, delta, budget)`. Correct ADR-061 §6 / contract §3 prose to stop calling the index "existing"/"O(1)". I **reject** Codex's alternative of "pull GV2-011/GV2-022 forward" — the `DependencyGraph` type already exists; wiring it is additive (pragmatic-lead, kernel-maintainer).

### B2 — `coverage: certified` is a false attestation: graph-policy invariants never run — CRITICAL
**Raised by:** Codex (Critical #1) + **all 6 personas**. **AGREED.**
**Evidence:** Task 8 orchestration calls only `run_antipattern_check(changed_paths, config, workspace_root)` (subphase-a.md:187), a stateless regex/fs scanner (`crates/anvil-checks/src/antipattern/check.rs:94-118`) that depends only on `anvil-kernel-types`, **not** `anvil-kernel` (architect) — so it structurally cannot read the graph. The four trust-boundary invariants (`CrossLayerViolation`, `NewDependencyIntroduction`, `PublicApiExpansion`, `PrivilegeExpansion`) register in `embedded.rs:119-133` and never run on this path. Security flags this as the more serious half: a "certified clean" verdict that silently skipped `PrivilegeExpansion`/`PublicApiExpansion` is a *false security attestation*.
**Judge's ruling on the fix (this is where personas split — see §5):** The architect/adversarial reveal that `run_embedded`/`PolicyEngine` has **no production caller** today (tests/benches only) and that production structural checking is a *third* engine, `anvil_architecture::validate_with_files_and_edges` (`gate.rs:991-1024`), reached only via `anvil gate`. Therefore forcing the full policy pipeline onto the save-time hot path **re-introduces the exact CPU problem ADR-061 was written to solve** (pragmatic-lead). **Required fix: narrow the claim, not the hot path.** Rename/split the wire field so `certified` means `certified: [antipattern]` (operations-reviewer's `check_families: ["antipattern"]` is the cleanest, forward-compatible mechanism), explicitly scoping the §8.2 parity gate and `coverage:certified` to the antipattern family across **all** current surfaces. Do **not** adopt Codex's "run graph policy before returning certified" as written.

### B3 — `ScanDiagnostics` is a phantom type; the "frozen" wire references a type the proto crate does not own — MAJOR
**Raised by:** Codex (Major #5) + **all 6 personas**. **AGREED.**
**Evidence:** Task 1 declares `ValidatePathsResponse { diagnostics: ScanDiagnostics }` "reuse the scan_buffer envelope type verbatim" (subphase-a.md:93); `ScanDiagnostics` does not exist anywhere (grep zero). Real type is `ScanBufferResponse` (`crates/anvil-intercept/src/midedit.rs:68`), daemon-local, not in `anvil-intercept-proto`. Task 1 will not compile as written.
**Required fix:** Define the shared diagnostic envelope in `anvil-intercept-proto` first, then type `ValidatePathsResponse.diagnostics` against it, with serialise-parity tests. **Pragmatic-lead's lighter alternative** (type it directly as `Vec<anvil_kernel_types::Diagnostic>` and defer the full move to A'/cleanup) is **acceptable for Sub-phase A** provided the type is *defined in the proto crate*, not re-declared daemon-local — re-declaration creates exactly the drift Codex warns about. Security adds value: one shared struct lets a single redaction guard apply uniformly across `scan_buffer` + `validate_paths`.

### B4 — Export-surface diff has no primitive; `previously_public` keys lack stable identity — MAJOR
**Raised by:** Codex (Major #3) + **all 6 personas**. **AGREED on the defect; split on severity of fix.**
**Evidence:** `update_file` removes all file symbols then re-adds (`incremental.rs:74-87`), so every save shows full churn; `symbol_baseline_key = file::kind::name` (`incremental.rs:27-29`) conflates identity with position; no `export_surface_changed()` helper exists; re-export detection has no edge-type/flag in the schema; GV2-002 (stable identity) is Draft.
**Required fix:** Default any modify touching public/privileged symbols to **partial/stale** until a real export-diff helper lands. Add fixtures: body-only change (→ false), rename (→ true), delete (→ true), internal→public (→ true), re-export add/remove. **Judge's ruling:** kernel-maintainer/security want a defined `export_surface_changed(delta) -> bool` helper before Task 6; pragmatic-lead argues the `previously_public` set-diff is *already conservatively safe* (rename = delete+add = surface-changed) and only fixtures are needed. **Both are satisfied by the same shipping rule**: it is acceptable to ship on the `previously_public` set-diff *if and only if* the conservative default-to-partial holds and the edge-case fixtures exist. The blocker is the missing fixtures + the explicit conservative default, not necessarily a new helper function.

### B5 — Sub-phase A inverts the deliberate `intercept → kernel` crate boundary — MAJOR
**Raised by:** architect (independent) + adversarial (independent, as a compile blocker). **AGREED (2 reviewers); I elevate to a named blocker.**
**Evidence:** `anvil-intercept/Cargo.toml` depends on `anvil-kernel-types` only; `anvil-kernel` arrives only transitively via `anvil-checks` which is **dev-dependencies** (adversarial). `watcher.rs:28` documents the deliberate refusal ("Pulling eddacraft-anvil-kernel into anvil-intercept would drag…"). Tasks 6/7/8 require `graph::incremental::{update_file,remove_file,re_resolve_imports}` from full `anvil-kernel`. The plan never acknowledges the boundary change, its build-weight cost, or the cycle risk.
**Required fix:** Make an explicit decision — either (a) add `anvil-kernel` to intercept `[dependencies]` with a cycle audit, or (b) extract the incremental-graph cache into a shared lower crate (e.g. `anvil-graph-cache`) both can depend on. This is a real architecture decision (ADR note), not incidental wiring. **This is a hard predecessor to B1's fix** — you cannot cache `DependencyGraph` in the daemon until the daemon can depend on the crate that defines it.

### B6 — Initial workspace assurance state is undefined; new workspaces can never reach `clean` in Sub-phase A — CRITICAL (single-reviewer)
**Raised by:** operations-reviewer (independent, missed by Codex). **Single-reviewer; I retain at critical for ops-correctness.**
**Evidence:** Contract §6 defines only `clean→stale` and `stale→pending→running→clean`; no initial-state entry in contract/ADR-061/plan. Sub-phase A has only the client-triggered `request_full_scan` (Task 9); the background scheduler is Sub-phase B. So a freshly connected workspace has no defined state and no automatic path to `clean` — contract line 42 (`certified iff state == clean`) means `validate_paths` returns `partial` on **every** call until a client manually scans. Never documented as user-visible behaviour.
**Required fix:** Define initial state = `stale(reason: cross-file-resolution-needed)` on first connection; specify that `watch` issues an automatic `request_full_scan` on connect/reconnect; add test `initial_workspace_state_is_stale_not_clean`; update the ADR-061 §9 state diagram. **Judge's note:** this is a genuine product-correctness gap, not a compile blocker — it does not stop *coding* Task 1–8 but must be resolved before the contract is called complete. I keep it critical because shipping a gate that is silently `partial` 100% of the time defeats its purpose.

---

## 3. Codex review scorecard

| # | Codex finding | Council disposition | Notes |
|---|---|---|---|
| C1 | `coverage: certified` over-claims (antipattern vs graph policy) | **CONFIRMED + BROADENED** | Architect/adversarial show `PolicyEngine`/`run_embedded` has **no prod caller**; real structural engine is a third system (`anvil_architecture`, `gate.rs:991`) behind `anvil gate`, and watch's `anvil check` is *also* antipattern-only (`check.rs:248`). Codex's framing ("vs the structural path watch uses") is itself inaccurate. **Fix REFINED:** narrow the claim (split coverage by family), do **not** force the policy engine onto the hot path. |
| C2 | Reverse-impact index not cached | **CONFIRMED + STRENGTHENED to compile-blocker** | Codex: "plan caches SymbolGraph not the index." Council: the index is **never populated anywhere in production**; `add_dependency` has zero non-test callers. "Existing"/"O(1)" in ADR-061/contract is factually false. Codex's "pull GV2 forward" alternative **OVERTURNED** as unnecessary. |
| C3 | Export-surface detection underspecified | **CONFIRMED** | Mechanism pinned (remove-all/re-add + key-only baselines). Pragmatic-lead **partially downgrades**: existing `previously_public` set-diff is conservatively safe; needs fixtures + documented rename=surface-change, not necessarily a blocking new helper. Net: confirmed, fix scope softened. |
| C4 | Don't blur interim delta application with ADR-063 hot reads | **CONFIRMED but DOWNGRADED to minor / non-blocking** | 4 of 6 (kernel, adversarial, security, pragmatic) judge this a **documentation/naming risk, not a plan defect** — the plan already states "Out of scope: sub-phase A'" (subphase-a.md:9). Security: the interim parse/resolve runs inside the daemon under the openat2 guard, so it is a *latency* concern, not a trust-boundary one. **Do not gate start on this.** Fix = module-level comments in `kernel_cache.rs`/`validate_paths.rs` marking the interim API as replaced by GV2-022 in A'. |
| C5 | Frozen wire references unowned `ScanDiagnostics` type | **CONFIRMED** (all 6) | See B3. Pragmatic-lead's lighter fix accepted with the constraint that the type live in proto. |
| C6 | Canonical GV2 architecture spec file absent | **CONFIRMED, but correctly scoped** (all 6) | `docs/architecture/graph-v2-foundation-spec.md` absent; GV2-001 Draft. **Blocks the GV2 Ready checklist and A', NOT Sub-phase A** (A uses only the interim SymbolGraph cache). Do not tick "taxonomy accepted by architecture review" until it lands. |

**Net:** Codex got all 6 directionally right. Two of its Criticals were *understated* (C1, C2 are worse than written). One Major (C4) the council **downgraded** to non-blocking. One alternative remedy (C2's "pull GV2 forward") and one required-fix (C1's "run graph policy on hot path") were **overturned** as counter to ADR-061's CPU goal.

---

## 4. New findings beyond Codex

**CRITICAL**
- **Initial assurance state undefined** (operations) — B6 above.

**MAJOR**
- **Crate-boundary inversion `intercept → kernel`** (architect, adversarial) — B5 above. Compile-blocking and an unacknowledged architecture decision.
- **`run_antipattern_check` uses the global rayon pool, cannot be steered into Task 10's interactive pool** (adversarial) — `check.rs:5,113` `.par_iter()` with no `pool.install`. The two-pool isolation goal is unachievable without an API change to `anvil-checks`.
- **`run_antipattern_check` does unguarded `fs::read_to_string` — bypasses Task 3's openat2/dirfd read-safety** (adversarial) — `check.rs:118`. Re-reads files outside `RESOLVE_NO_SYMLINKS|RESOLVE_BENEATH`, reopening the TOCTOU/symlink-escape window Task 3 closes. Fix: pass pre-read guarded bytes, not paths.
- **Auth "reuse" is unwired dead code; `validate_paths` is the FIRST verb to read arbitrary on-disk paths** (security) — `auth.rs:26-33` doc says no consumer wired (DRVR-001 Wave 2); `validate_workspace_roots`/`is_driver_allowed` have zero call sites in `ipc.rs`. `scan_buffer` never reads disk (`midedit.rs:393` scans `request.text`). Makes Task 3 the read-safety guard **load-bearing** and a hard predecessor to Task 8, not a "reuse."
- **`confinement.rs` placed in `anvil-intercept` but the ANVIL_HOME resolver it must reuse lives in `anvil-cli`** (security) — wrong-direction dep (`anvil-cli/Cargo.toml:27` → intercept, not reverse). The confinement allowlist ("agent cannot grant itself access") can't be loaded from where the plan puts the code.
- **Structured log fields for assurance transitions unspecified; contradicts ADR-035** (operations) — contract:225 says "structured INFO log on every transition" but names no fields; ADR-035 routes user-visible state changes to the Notification envelope. Unmonitorable from day one.
- **No daemon mid-session disconnect/reconnect spec for watch** (operations) — Task 12 covers daemon-present/absent but not death mid-session; `first_fallback_warns_once` guard already fired; `SHUTDOWN_DRAIN_DEADLINE` 250ms (`ipc.rs:99`) truncates in-flight responses with no defined handling.

**MINOR**
- `GraphDelta.removed_edges` always empty from `remove_file`/`update_file` (`incremental.rs:150,291-298`) — any certify logic reading `delta.removed_edges` finds nothing; must use `dependents_of` exclusively (kernel-maintainer, missed by Codex).
- `ALL_ANVIL_METHODS` pin test is one-directional — new Task 1 constants can exist unpinned (`protocol.rs:184-201`) (adversarial).
- "never a source read-oracle" over-claims — `Diagnostic.summary`/`remediation_hint` are free-text and can echo a matched secret literal (`diagnostics.rs:161-172`); non-critical within the same-uid SO_PEERCRED boundary but the framing is wrong (security).
- SLO gate (Tasks 10/11/16) labelled a "sibling-plan candidate" but ADR-061 §9 makes it a Phase 2 merge dependency; contract §8 lists only 3 gates — an author could merge Phase 2 without the SLO bench (pragmatic-lead).

---

## 5. Contradictions and judge's rulings

No `DEBATE_REQUIRED` rises to a blocking contradiction — all disagreements are about *fix scope*, not defect existence. Rulings:

1. **C1 fix — run graph policy on hot path (implied by Codex/security) vs narrow the claim (pragmatic-lead).** **RULING: narrow the claim.** The architect/adversarial evidence that the policy engine is dead in production and the real engine is whole-repo `anvil gate` is decisive: forcing it onto save-time reintroduces the CPU regression ADR-061 exists to fix. Ship `coverage: certified` scoped to the antipattern family (operations' `check_families` field), parity gate scoped to match. Security's concern is satisfied because a *correctly-labelled* "antipattern-only" verdict is not a false attestation; an unlabelled one is.

2. **C3/B4 — blocking export-diff helper (kernel, security) vs fixtures-only on existing set-diff (pragmatic-lead).** **RULING: split the difference.** The `previously_public` set-diff is acceptable to ship **iff** (a) the conservative default-to-partial for any touched public/privileged symbol is explicit, and (b) the rename/delete/re-export/internal→public fixtures exist. No separate helper is mandatory; the conservative default is.

3. **C4 — plan defect (Codex/architect) vs naming risk (kernel, adversarial, security, pragmatic 4–2).** **RULING: non-blocking, majority wins.** Add module-level comments marking the interim API; do not gate start. Security's point stands: the interim lane runs inside the daemon's dirfd guard, so it is latency, not trust-boundary.

4. **B3/C5 fix weight — full proto move + parity tests (Codex/kernel/security) vs lighter `Vec<Diagnostic>` in proto (pragmatic-lead).** **RULING: lighter fix accepted for Sub-phase A, with the hard constraint that the type is defined in `anvil-intercept-proto` (not re-declared daemon-local).** Full envelope unification can defer to A'.

---

## 6. Recommended pre-implementation sequence

Ordered; each item is a predecessor to the next where noted.

1. **Resolve the crate boundary (B5).** Decide: add `anvil-kernel` to `anvil-intercept/[dependencies]` (with cycle audit) **or** extract `anvil-graph-cache`. Write the ADR note. *Predecessor to everything in Tasks 6/7/8 — they cannot compile otherwise.*
2. **Amend Task 7 + Task 6 for `DependencyGraph` (B1).** Cache `(SymbolGraph, DependencyGraph)` per `WorktreeKey`; build it cold from import edges; maintain it in `apply_delta`; change `certify` signature to take both. Correct ADR-061 §6 / contract §3 prose ("existing"/"O(1)" → net-new). Add `certify_uses_dependency_graph_reverse_not_symbol_graph_scan` + reverse-index-consistency-after-delta tests.
3. **Decide the coverage claim (B2).** Add `check_families` to `ValidatePathsResponse` (or rename `certified` → `certified: [antipattern]`); scope §8.2 parity gate + the contract's certified definition to the antipattern family across all surfaces. Note in the plan that `run_embedded`/`PolicyEngine` is currently dead in prod.
4. **Define the proto diagnostic envelope (B3).** Define the shared type in `anvil-intercept-proto`; type `ValidatePathsResponse.diagnostics` against it; add scan_buffer↔validate_paths serialise-parity tests. *Predecessor to calling Task 1's wire "frozen."*
5. **Define initial assurance state + auto-scan (B6).** Initial = `stale(cross-file-resolution-needed)`; `watch` auto-issues `request_full_scan` on connect/reconnect; update ADR-061 §9 diagram; add `initial_workspace_state_is_stale_not_clean`.
6. **Specify export-surface conservative default + fixtures (B4).** Default touched public/privileged → partial; add rename/delete/internal→public/re-export fixtures. Document that `delta.removed_edges` is always empty and importer discovery uses `dependents_of` exclusively.
7. **Close the read-safety + rayon-pool gaps (new majors).** Make Task 3 (openat2 read-safety) a hard predecessor of Task 8; route `run_antipattern_check` to pre-read guarded bytes (closes the `fs::read_to_string` TOCTOU at `check.rs:118`); extend `run_antipattern_check` to accept a `&rayon::ThreadPool` so Task 10's interactive pool governs it. Reword Task 2 to state the workspace-root handshake is net-new, not "reuse."
8. **Resolve crate placement of `confinement.rs` + structured-log/ADR-035 field contract + mid-session reconnect tests** (ops/security majors) before the contract is called complete.
9. **Add the non-blocking documentation items:** C4 interim-lane comments; SLO-gate-is-a-Phase-2-blocker note in contract §8; `ALL_ANVIL_METHODS` pin-test update; narrow the "not a read-oracle" wording.

**GV2 / A'-only (do NOT block Sub-phase A start):** Land `docs/architecture/graph-v2-foundation-spec.md` (C6) before ticking "taxonomy accepted" or promoting GV2 to Ready; GV2-002 stable identity before the export fast-path graduates from conservative-partial.

---

**Findings I judge wrong / over-stated:** Codex C4 (overstated as a plan defect — it is a naming risk the plan already guards at subphase-a.md:9); Codex C1's required-fix (running graph policy on the hot path is the wrong remedy — it reopens the CPU regression); Codex C2's "pull GV2 forward" alternative (unnecessary — the type exists and wiring is additive). The security finding "auth being reused" is correctly framed as net-new; I concur it is not a critical *defect* but a critical *plan mislabel* — its real teeth are that it makes Task 3 load-bearing, which I have folded into step 7.

Evidence file read: `/home/aneki/Projects/src/anvil-001.council/CODEX-REVIEW-INPUT.md`.
