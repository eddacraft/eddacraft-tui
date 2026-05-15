# Multi-Layer Protection v2 (Integration + Follow-ups)

| ID   | Owner  | Status      | Progress   |
| ---- | ------ | ----------- | ---------- |
| MLP2 | @aneki | In Progress | 30/60 done |

**Last reviewed:** 2026-05-15 (wave 1G shipped 2026-05-15 via PR #1576
at `33659b6c` — Group H closed 5/5: MLP2-037 (`anvil hook bootstrap
--witness-recent` with `--reverse` git rev-list +
`validation_at: "bootstrap-recovery"`), MLP2-038 (end-to-end
union-merge proof: real `git init` + `git merge` integration test),
MLP2-039 (`anvil start --format yaml|yml|json|toml` pre-writes
`.anvil.<ext>` with embedded `format` field matching extension),
MLP2-040 (gate.rs prefers `.anvil.<ext>` via MLP-011's `discover`,
new `anvil migrate` command), MLP2-041 (typed `GateConfigView` /
`InitConfigView` / `PolicyConfigView` foundation). Council session
`council-e8633cef` quick-converged with 2 MAJOR + 2 MINOR fixed
pre-push. Wave 1H shipped earlier the same day via PR #1575 at
`a40525ad` — MLP2-032 wires `ensure_project_id` +
`anvil_l4::pin_cutoff_commit` into the `anvil baseline`
orchestrator with canonical `anvil_config::discover` policy
precedence and first-create cutoff fallback; MLP2-034 Phase 1
wires `anvil-checks::antipattern::run_antipattern_check` into
the same orchestrator to populate `BaselineFinding {rule_id,
file_path, fingerprint}` with move-resistant snippet
fingerprints. +6 unit pins in `commands/baseline.rs` (2 are
Council #C-1/#C-2 regression guards folded into the same branch
before merge). Group G now 3/6 (MLP2-031 + -032 + -034 Phase 1
shipped). Earlier history: wave 1C shipped 2026-05-14 — MLP2-052,
MLP2-057, MLP2-048, MLP2-016 closed together on branch
`feat/mlp2-wave-016-048-057-052` with one Council remediation
pass on top — Council #C-016A `on_warn` consultation fix folded
into MLP2-016. Module created from MLP-018 split-out; each of
the 56 deferred sub-items in `[multilayer-protection]`'s v1-scope
footnotes promoted to its own MLP2-NNN task.)

> **Scope.** MLP2 ships the integration work that closes every v1
> primitive landed by the MLP module into a full surface. MLP
> delivered the libraries (witness chain, hook, L4 policy, baseline,
> attribution, kindling-observation builder, protection-claim
> vocabulary, etc.); MLP2 wires those libraries into the daemon's
> enforcement pipeline, the editor / MCP / CI surfaces that render
> their output, and the cross-platform extensions that v1
> deliberately scoped out.
>
> Every MLP2 task carries an explicit `Source:` line naming the
> originating MLP task / footnote / PR. The intent is one-to-one
> traceability between a shipped primitive and its remaining
> integration debt — no deferral disappears into "tracked as
> follow-up" with no concrete acceptance criterion.

## Purpose

MLP shipped its v1 surface area with **17/18** items Done (only
the catalogue task MLP-018 itself remained, now Done with the
split into this module). Many of those items shipped a focused
primitive ahead of full surface integration to keep PR scopes
bounded, with deferred follow-ups recorded as `Scope-narrowing
footnotes` on the individual MLP entries.

This module collects those footnotes into 56 first-class APS work
items so each one is plannable, prioritisable, and tractable. The
groupings (A–K) match the original MLP-018 catalogue and reflect
shared ownership: tasks within a group can land in the same PR or
share a primitive (e.g., the rate-window primitive in A9 and D3).

Group L (MLP2-057..-060) extends the module by four
production-hardening items filed from the Council review of the
MLP2-001 + MLP2-002 PR (#1522, session `council-e2fdfc0c`,
2026-05-14). These tasks do not close MLP-018 catalogue items;
they harden MLP2's own surface before the cache + in-flight
primitives are wired into the production daemon path. Each Group L
task's `Source:` line cites the Council finding IDs.

## In Scope

- Daemon-side enforcement integration of the v1 libraries
  (`anvil-attribution`, `anvil-witness`, `anvil-rules`,
  `anvil-baseline`, `anvil-hook`, `anvil-l4`,
  `anvil-kernel-types::protection_claim`,
  `anvil-intercept::kindling_observation`).
- Cross-platform extensions where v1 shipped Linux-only.
- Surface conformance — every renderer of a protection claim
  consumes the closed-set vocabulary from
  `anvil-kernel-types::protection_claim`.
- TypeScript mirrors where the daemon-side Rust shipped first.
- External publishing pipeline for the GitHub Action.
- Production hardening on MLP2's own surface where Council review
  flagged a deployment-readiness gap (Group L, MLP2-057..-060).

## Out of Scope

- Inventing new v2 capabilities — every task here closes a v1
  deferral or a Council-flagged production-hardening item on
  MLP2's own surface (Group L). New capabilities outside that
  envelope go through their own planning module.
- GitLab / Bitbucket integrations (vNext universal v2).
- Anvil cloud sidecar / hosted services (vNext, opt-in only).
- Rule-pack distribution channel beyond git-tracked (vNext).
- Cross-Windows ↔ WSL surface bridging (vNext, separate ADR).

## Interfaces

- **Depends on:** All of MLP — every MLP2 task targets the
  surface of a Done MLP primitive.
- **Coordinates with:** INTD (daemon enforcement pipeline), DRVR
  (driver framework), RMCP / RMCPF (MCP shim), RTAI (mid-edit
  validation backbone), LAUNCH (`anvil start` activation
  orchestrator), kindling-integration (observation consumer).

## ADRs cited

- **ADR-036** — Daemon scope, discovery, OS boundary.
- **ADR-037** — Witness chain + L4 policy framework.
- **ADR-038** — Hook surface + noise discipline.
- **ADR-039** — Baseline policy + hard-pinned rule classes.

## Tasks

### A. Daemon enforcement + observation integration

#### MLP2-001: Daemon-side `worktree_key → (rules_sha, ResolvedRuleSet)` cache with `.anvil.*` watcher invalidation

- **Status:** Done
- **Intent:** Daemon caches resolved rule sets keyed by
  `worktree_key`; each cached entry carries the `rules_sha` that
  identifies it (so witness-chain consumers can confirm a cached
  resolution still matches their expected version). The cache
  invalidates on `.anvil.*` file changes so config edits propagate
  without restart. (Council 2026-05-14 #C-030 / #C-038: the
  original title implied a compound `(worktree_key, rules_sha)`
  key; the implementation uses worktree-only keying because all
  agents in a worktree share the same rule set.)
- **Expected Outcome:**
  - In-memory cache in `anvil-intercept`'s session registry tier.
  - File watcher hooks invalidate cache entries on `.anvil.yaml`
    / `.anvil.yml` / `.anvil.json` / `.anvil.toml` writes.
  - Cache miss falls back to `anvil-config::parse_file` +
    `anvil-rules::rules_sha` recompute.
- **Files:** `crates/anvil-intercept/src/registry.rs`,
  `crates/anvil-intercept/src/watcher.rs`,
  `crates/anvil-intercept/src/config.rs`.
- **Validation:** Cache hit/miss telemetry; watcher event delivers
  invalidation within 250ms; concurrent writers don't race the
  cache. **Evidence (Done 2026-05-14, Council-reviewed 2026-05-14):**
  `cargo test -p eddacraft-anvil-intercept` — 242 lib tests green,
  including 18 `rule_cache::` unit tests (lookup hit/miss,
  resolver failure does not poison, invalidate idempotency,
  format-agnostic `rules_sha`, mixed-case ext rejected per
  `anvil_config::discover` lock-step rule, canonicalise-fail
  conservatively flushes all entries, subdirectory `.anvil.yaml`
  ignored, multi-worktree isolation, concurrent invalidate +
  store) and 2 `watcher::` integration tests (config write
  invalidates cache; unrelated write does not). Coalesce window is
  50ms so the watcher delivers invalidation well inside the 250ms
  budget; the cache uses `Mutex<HashMap>` for race-free concurrent
  mutation. **Production-wiring caveat:** the cache type is shipped
  as a library primitive — `run_foreground` does not yet
  instantiate `RuleSetCache` and the watcher's `recv_blocking`
  remains a stub, so the cache currently runs unit-test-only.
  Production wiring lands with MLP2-014 / INTD-004 (Council
  2026-05-14 #C-010 / #C-011 / #C-042). New file:
  `crates/anvil-intercept/src/rule_cache.rs`; modified:
  `crates/anvil-intercept/src/watcher.rs`,
  `crates/anvil-intercept/src/lib.rs`,
  `crates/anvil-intercept/Cargo.toml`,
  `crates/anvil-intercept/src/kindling_observation.rs`.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-012, MLP-011
- **Coordinates with:** MLP2-023 (session-key change) — the cache
  is keyed on the worktree path (`worktree_key`), which is
  unaffected by the session registry's `(WorktreeKey, AgentTag)`
  extension because resolved rule sets are worktree-scoped, not
  per-agent. MLP2-023 may rename `WorktreeKey` shape; the cache
  takes a forward-compatible newtype so the rename is mechanical.
- **Source:** MLP-012 footnote 1. Downgraded MLP2-023 from
  `Dependencies` to `Coordinates with` 2026-05-14 after the
  contradiction with this task's own description was flagged in
  dependency audit.

#### MLP2-002: In-flight evaluation pinning during config-update bursts

- **Status:** Done
- **Intent:** A config write during an in-flight evaluation MUST
  NOT swap the rule set mid-evaluation. The scheduler pins the
  resolved set for the duration of the call.
- **Expected Outcome:**
  - Evaluation start records the `rules_sha` it resolved.
  - Config-write watcher signals invalidation but does not abort
    in-flight evaluations; new evaluations pick up the new set.
  - Burst-handling: multiple config writes within a window
    coalesce; in-flight evaluation count is observable.
- **Files:** `crates/anvil-intercept/src/midedit.rs` (scan_buffer
  service, `ScanBufferResponse.rules_sha`, in-flight counter, RAII
  `InFlightGuard`); `crates/anvil-intercept/src/kindling_observation.rs`
  (test fixture call sites updated for the new field).
- **Validation:** Adversarial test — write config mid-evaluation,
  assert in-flight call returns with the original `rules_sha`.
  **Evidence (Done 2026-05-14, Council-reviewed 2026-05-14):**
  `cargo test -p eddacraft-anvil-intercept midedit::` — 15 tests
  green, including 5 MLP2-002 tests:
  `scan_buffer_with_pin_returns_pinned_rules_sha`,
  `scan_buffer_without_pin_omits_rules_sha`,
  `scan_buffer_in_flight_counter_tracks_active_evaluations` (gated
  rule + barrier observes the 0 → 1 → 0 transition without
  sleeping), `scan_buffer_in_flight_clears_after_timed_out_exit`
  (asserts the RAII guard releases on the `TimedOut` path), and
  `config_invalidation_while_worker_running_does_not_swap_pinned_rules_sha`
  — the adversarial test now uses a multi-thread runtime + a
  `GateRule` barrier to park the worker, the test body invalidates
  the cache while the worker is provably blocked, and the barrier
  is released only after the cache is empty (Council #C-001 /
  #C-029 / #C-036; the earlier captured-into-local-String version
  was tautological). `InFlightGuard` uses `AcqRel` for
  fetch_add/fetch_sub and `Acquire` for `in_flight()` so the
  `in_flight==0 after exit` guarantee is portable to weakly-ordered
  architectures (Council #C-040). Burst-coalescing is delivered by
  the watcher coalescer (50 ms window); in-flight count exposed via
  `ScanBufferService::in_flight()` — note this is a library
  primitive today, with the daemon's status surface wiring to
  follow in a separate item (Council #C-009). The wire shape stays
  backward-compatible: `rules_sha` is `#[serde(default,
  skip_serializing_if = "Option::is_none")]`, so the existing MCP
  deserialiser in `anvil-cli/src/mcp/validation.rs` keeps parsing
  v1 responses without change.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP2-001
- **Source:** MLP-012 footnote 2.

#### MLP2-003: Composite identity check at daemon attach

- **Status:** Done
- **Intent:** Daemon attach cross-checks `(project_uuid,
  first_commit, origin_canonical)` from the session's worktree
  against the value persisted in `anvil/project-id`; mismatches
  surface as `degraded:identity-mismatch`.
- **Expected Outcome:**
  - Attach reads `anvil/project-id` (existing MLP-001 surface) +
    `git rev-list --max-parents=0 HEAD` for `first_commit` +
    `git config --get remote.origin.url` canonicalised.
  - All three must match the daemon's registry record;
    `degraded:identity-mismatch` on disagreement.
  - Fork detection: when `forked_from` is set and matches the
    parent identity, attach succeeds without degradation.
- **Files:** `crates/anvil-cli/src/activation/identity.rs`
  (`ProjectIdentity` extended with `first_commit` +
  `origin_canonical` fields; new `verify_against_worktree` +
  `attach_check` API; `canonicalise_origin` /
  `read_first_commit` / `read_origin_canonical` helpers;
  `IdentityCheck` / `IdentityMismatch` / `AttachStatus` typed
  enums with a pinned `degraded:identity-mismatch` wire-signal
  constant). Registry-side wiring (the `register-session` IPC
  consumer of `AttachStatus`) lands with MLP2-025 — the
  primitive is in place but the daemon attach path picks it up
  alongside the spoof-rejection cross-check (`#[allow(dead_code)]`
  annotations document the call-sites that wire in MLP2-025).
- **Validation:** Fork acceptance; renamed origin rejection;
  rebased history rejection. **Evidence (Done 2026-05-14):**
  `cargo test -p eddacraft-anvil --bins activation::identity::`
  — 42 tests green (21 baseline + 21 MLP2-003-specific). New
  tests cover: parse/render round-trip of `first_commit` +
  `origin_canonical` fields; rejection of malformed `first_commit`
  (non-40-hex, uppercase) and `origin_canonical` (empty, with
  control chars); `canonicalise_origin` lock-step between SSH
  alias / HTTPS / no-`.git` / trailing-slash / mixed-case host
  spellings, plus path-case preservation so a forge rename
  surfaces as a mismatch; `read_first_commit` against a real
  tempdir-backed `git init` (with empty-repo → `None`);
  `verify_against_worktree` against matching identity →
  `Match`; renamed origin → `Mismatch::OriginCanonical`;
  rebased history → `Mismatch::FirstCommit`; `forked_from` set
  → `ForkedFromParent`; pre-MLP2-003 file (no fields) → `Match`;
  empty-repo vs recorded `first_commit` → typed mismatch;
  missing origin vs recorded → typed mismatch. The high-level
  `attach_check` API covers the four `AttachStatus` variants
  (`Clean` / `Fork` / `Mismatch` / `ProjectIdMissing`) and pins
  the wire-level `degraded:identity-mismatch` signal via
  `AttachStatus::DEGRADED_REASON`. Workspace `cargo test` clean;
  `pnpm format:check` + `pnpm lint:check` (nx clippy + fmt-check
  across 26 projects) clean.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-001, MLP2-023
- **Source:** MLP-001 footnote 1.

#### MLP2-004: Daemon chain-head cache update on post-commit

- **Status:** Draft
- **Intent:** `anvil hook post-commit` triggers a daemon-side
  cache update for the worktree's witness-chain head so
  subsequent verifications skip re-reading the active ndjson.
- **Expected Outcome:**
  - Post-commit hook notifies the daemon (via existing IPC) of
    the new chain head.
  - Daemon session record gains a `chain_head_sha` field.
  - Verifiers prefer the cached head; on miss, fall back to
    `anvil-witness::verify_chain`.
- **Files:** `crates/anvil-hook/src/post.rs` (extend),
  `crates/anvil-intercept/src/registry.rs`,
  `crates/anvil-intercept-proto/src/lib.rs` (extend
  `SessionRecord`).
- **Validation:** Concurrent commits across multiple worktrees
  update independent cache entries; restart re-populates from
  `anvil-witness::tail`.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-002, MLP-005
- **Source:** MLP-005 deferred outcome.

#### MLP2-005: Witness append — daemon RPC + embedded fallback

- **Status:** Draft
- **Intent:** `anvil hook pre-commit` currently invokes the
  witness library directly. Route through the daemon's IPC when
  reachable (so multiple worktrees share rate limits + chain
  state) and fall back to embedded library calls when the daemon
  is unreachable.
- **Expected Outcome:**
  - Hook attempts daemon RPC first; on timeout / unreachable
    falls back to embedded `anvil-witness::WitnessWriter` call.
  - Fallback path emits `degraded:embedded-witness` to Kindling
    via the surface-claim vocabulary
    (`SurfaceClaimState::EmbeddedFallback`).
  - Daemon-side IPC writes through the same `WitnessWriter`; no
    divergence between fallback and daemon-routed appends.
- **Files:** `crates/anvil-cli/src/commands/hook.rs`,
  `crates/anvil-intercept/src/ipc.rs` (new witness-append RPC).
- **Validation:** Two-process race test — daemon-routed and
  fallback appends interleave correctly under flock; integration
  test with daemon killed mid-append.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-002, MLP-003
- **Source:** MLP-003 deferred outcome ("Daemon RPC + embedded
  fallback").

#### MLP2-006: Daemon notification layer emits `GateEvaluatedObservation` to Kindling

- **Status:** Draft
- **Intent:** Wire the daemon's notification fan-out to call
  `anvil-intercept::kindling_observation::from_midedit_response`
  on every scan_buffer completion and write the resulting row
  via the kindling-integration SQLite handle.
- **Expected Outcome:**
  - Daemon holds a Kindling client handle (created at startup,
    per-machine DB path).
  - Notification layer constructs `ObservationContext` from
    session id + traceparent + scan timing.
  - `from_midedit_response` short-circuits on pass-no-finding;
    only finding-bearing scans produce a row.
  - Failure to write to Kindling is logged at the daemon level
    but does NOT block the scan response.
- **Files:** `crates/anvil-intercept/src/fanout.rs`,
  `crates/anvil-intercept/src/midedit.rs` (call site),
  `packages/kindling-integration/src/adapter.ts` (consumer end).
- **Validation:** Adversarial — Kindling DB locked → response
  still returns; rate-limit primitive (MLP2-009) prevents flood.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-016, MLP2-009
- **Source:** MLP-016 footnote 1.

#### MLP2-007: MCP shim mirror of mid-edit Kindling observations

- **Status:** Draft
- **Intent:** The MCP shim (`crates/anvil-cli/src/mcp/validation.rs`)
  must produce bit-identical `gate_evaluated` rows for its
  mid-edit calls so MCP and direct-driver observations are
  indistinguishable downstream.
- **Expected Outcome:**
  - MCP shim's validation path constructs `ObservationContext`
    using its own session-id / traceparent.
  - Calls the same `from_midedit_response` builder from
    `anvil-intercept::kindling_observation`.
  - Wire-shape parity test: same diagnostic input → same JSON
    output regardless of MCP-vs-direct origin.
- **Files:** `crates/anvil-cli/src/mcp/validation.rs`.
- **Validation:** Parity test in
  `crates/anvil-cli/tests/mcp_kindling_parity.rs`.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-016
- **Source:** MLP-016 footnote 2.

#### MLP2-008: RTAI-007 telemetry-contract join

- **Status:** Draft
- **Intent:** Explicit field map between RTAI-007's mid-edit
  envelope and the `gate_evaluated` Kindling row, so a row can
  be joined back to its originating telemetry envelope by
  traceparent + gate_eval_id.
- **Expected Outcome:**
  - RTAI-007's envelope schema documents which fields populate
    which Kindling row fields.
  - Joining test: emit envelope → emit Kindling row → join by
    traceparent → fields agree.
  - Both surfaces share the same `gate_eval_id` source (the
    traceparent's `span_id`, or a derived hash).
- **Files:** `crates/anvil-intercept/src/telemetry.rs` (RTAI-007
  surface), `crates/anvil-intercept/src/kindling_observation.rs`
  (consumer of the join key).
- **Validation:** Join-back integration test.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-016, RTAI-007
- **Source:** MLP-016 footnote 4.

#### MLP2-009: Volume-bounded burst rate-shaping for observations

- **Status:** Done
- **Intent:** Shared rate-window primitive caps observation emit
  rate so a keystroke burst can't flood Kindling. Same primitive
  used by MLP2-026's `degraded:fence-cascade` detector.
- **Expected Outcome:**
  - New `anvil-intercept::rate_window` module with a
    sliding-window counter (configurable per-emitter rate +
    burst tolerance).
  - MLP2-006 consumes it for `gate_evaluated` emissions.
  - When the rate is exceeded, additional emissions drop and a
    single `degraded:observation-throttled` row records the
    drop count.
- **Files:** `crates/anvil-intercept/src/rate_window.rs` (new).
  Consumer wiring (originally listed against `fanout.rs`) lands
  with MLP2-006 (`gate_evaluated` Kindling emit) — the primitive
  ships here as a standalone sliding-window counter so MLP2-006
  / MLP2-026 / MLP2-059 can each adopt it without coupling.
- **Validation:** Burst test — 1000 emissions in 100 ms → bounded
  output count + single throttle marker. **Evidence (Done
  2026-05-14):** `cargo test -p eddacraft-anvil-intercept --lib
  rate_window::` — 10 tests green covering: within-capacity
  admit; over-cap throttle; consecutive throttles accumulate
  `drops`; first `Allow` after a throttle burst carries
  `pending_drops`; `Allow` resets the pending counter to zero
  for the next burst; sliding window evicts expired timestamps;
  zero capacity is clamped to 1 (defensive); `admitted_at`
  diagnostic surface; the headline 1000-event burst at cap=50
  admits exactly 50 and throttles 950; concurrent records
  across 8 threads x 200 calls share the same cap (total admits
  stay at capacity). The single-throttle-marker contract is
  delivered via the `RateDecision::Allow { pending_drops }`
  variant carrying the cumulative drop count back to the
  consumer, so MLP2-006 can emit exactly one
  `degraded:observation-throttled` row per sustained burst.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-016
- **Source:** MLP-016 footnote 5.

#### MLP2-010: Kindling `action_executed` emission for post-hooks

- **Status:** Draft
- **Intent:** `anvil hook post-commit` / `post-merge` /
  `post-rewrite` each emit a Kindling `action_executed`
  observation. Pairs with MLP2-004's chain-head cache update.
- **Expected Outcome:**
  - Post-hook surfaces produce `ActionExecutedObservation` (from
    `packages/kindling-integration/src/observation-contract.ts`)
    via the daemon's notification fan-out.
  - Wire shape: action name (post-commit / post-merge /
    post-rewrite), commit SHA, witness line hash.
  - Pass-no-finding silence rule does NOT apply — every post-hook
    invocation produces exactly one row.
- **Files:** `crates/anvil-intercept/src/kindling_observation.rs`
  (extend with `action_executed` builder),
  `crates/anvil-cli/src/commands/hook.rs`.
- **Validation:** Three integration tests (one per hook).
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-005, MLP2-006
- **Source:** MLP-005 deferred outcome.

### B. Witness chain extensions

#### MLP2-011: DAG-aware merge verification

- **Status:** Draft
- **Intent:** Merge commits carry `parent_commits[]` +
  `prev_line_hashes[]`; the verifier currently walks the chain
  linearly. Replace with a graph walk that joins on merge
  parents.
- **Expected Outcome:**
  - `anvil-witness::verify_chain_dag` walks the line graph,
    joining at merge nodes against all listed parents.
  - Detects tamper / dropped / stray-genesis / orphan-merge.
  - Existing `verify_chain` deprecated to a thin wrapper that
    calls `verify_chain_dag` and asserts the result is linear.
- **Files:** `crates/anvil-witness/src/verify.rs`.
- **Validation:** Merge fixture from MLP-005's `merge_witness_plan`
  output; tamper tests at each parent.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-002, MLP-005
- **Source:** MLP-002 footnote 1, MLP-006 deferred outcome.

#### MLP2-012: Manifest event stream (`anvil/witness/manifest/chain.ndjson`)

- **Status:** Merged
- **Intent:** Rollover events from `WitnessWriter::append`
  become append-only entries in
  `anvil/witness/manifest/chain.ndjson` so consumers can stream
  archive transitions without polling the directory.
- **Expected Outcome:**
  - On rollover, manifest line emitted with archive path +
    merkle hash + line count.
  - Manifest is in-tree (part of the witness chain primitive)
    and travels via git like the rest.
  - Tail follow primitive in `anvil-witness::manifest_tail` for
    consumers.
- **Files:** `crates/anvil-witness/src/writer.rs`,
  `crates/anvil-witness/src/manifest.rs` (new).
- **Validation:** Rollover test under a tight `RolloverPolicy`
  produces ordered manifest entries matching the archives.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-002
- **Source:** MLP-002 footnote 2 (MLP-002b).

#### MLP2-013: Witness genesis-line emission (`GENESIS-BASELINED`)

- **Status:** Draft
- **Intent:** `anvil baseline` emits the first witness line with
  `GENESIS-BASELINED` plus the `cutoff_commit` value, so
  later L4 verifications can distinguish baselined vs
  greenfield repos at the chain level.
- **Expected Outcome:**
  - `anvil-baseline`'s save path calls
    `anvil-witness::WitnessWriter::write_genesis` with the
    `GENESIS-BASELINED` anchor + cutoff_commit on the line body.
  - `GENESIS-FRESH` for `anvil start` adoption (no cutoff).
  - Verifier accepts both anchor types.
- **Files:** `crates/anvil-baseline/src/io.rs` (extend),
  `crates/anvil-witness/src/genesis.rs` (already supports the
  anchors; this wires the call site).
- **Validation:** Round-trip test: baseline → read genesis →
  cutoff_commit matches.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-002, MLP-007
- **Source:** MLP-007 footnote 4.

#### MLP2-014: Witness writer call-site wiring at the hook

- **Status:** Draft
- **Intent:** `WitnessLine.rules_sha` exists from MLP-002 but no
  call site populates it. The hook resolves the active rule
  set + computes `rules_sha`; this task threads that into the
  `WitnessLine` at write time.
- **Expected Outcome:**
  - `anvil hook pre-commit` resolves `(worktree_key, config)` →
    `ResolvedRuleSet` (via MLP2-001 cache).
  - Computes `rules_sha` via `anvil-rules::rules_sha`.
  - Passes to `WitnessWriter::append` so every line carries the
    rule-set digest.
- **Files:** `crates/anvil-cli/src/commands/hook.rs`,
  `crates/anvil-hook/src/lib.rs`.
- **Validation:** Lines from two commits with different config
  files carry distinct `rules_sha` values.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-002, MLP-012, MLP2-001
- **Source:** MLP-012 footnote 5.

#### MLP2-015: Promote 80-writer stress test to CI

- **Status:** Draft
- **Intent:** `eighty_writers_no_interleaving` in `anvil-witness`
  is gated behind `#[ignore]`. Promote to a CI-runnable test
  once the runner has the parallel budget.
- **Expected Outcome:**
  - Remove `#[ignore]` OR add a dedicated `--features stress`
    flag the CI matrix runs separately.
  - CI runtime budget review: confirm the test fits in the
    cargo-test job under 60s.
- **Files:** `crates/anvil-witness/tests/concurrency.rs`,
  `.github/workflows/release-readiness.yml` (if a dedicated
  matrix lane is added).
- **Validation:** Test runs green on CI for 10 consecutive runs
  before un-ignoring; flake budget review.
- **Confidence:** high
- **Priority:** Low
- **Dependencies:** MLP-002
- **Source:** MLP-002 footnote 4.

### C. L4 policy execution

#### MLP2-016: `validate_at_l4` server-side rule-engine execution

- **Status:** Done
- **Intent:** Pre-push's `NeedsL4Validation` decisions currently
  emit `InternalError { TimedOut }` because the L4 engine isn't
  wired. This task swaps in the real rule-engine call.
- **Expected Outcome:**
  - New `anvil l4-validate <commit-range>` CLI subcommand (or
    daemon RPC) running the full `anvil-l4::Policy` pipeline
    against each unwitnessed commit.
  - Pre-push hook calls this instead of returning
    `InternalError`.
  - Returns `Allow` / `Block` per-commit with diagnostic
    payload.
- **Files:** `crates/anvil-l4/src/validate.rs` (new),
  `crates/anvil-cli/src/commands/hook.rs` (swap the timeout
  branch).
- **Validation:** End-to-end test: push with unwitnessed commit
  → server runs L4 → allow/block surfaces to the operator.
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** MLP-004, MLP-006
- **Source:** MLP-006 deferred outcome, MLP-004 footnote 1.

#### MLP2-017: `refs/notes/anvil-l4` writes for L4 witnesses

- **Status:** Draft
- **Intent:** L4 produces a witness too, but ADR-037 §D-7
  forbids in-tree ledger mutation at L4 — so the witness goes
  to `refs/notes/anvil-l4` out-of-band.
- **Expected Outcome:**
  - `anvil l4-validate` writes a `WitnessLine` (kind `l4`) to
    `refs/notes/anvil-l4` indexed by commit SHA.
  - `git fetch origin refs/notes/anvil-l4:refs/notes/anvil-l4`
    surfaces the notes for verifiers.
  - GitHub Action wrapper (MLP-010 / MLP2-042) sets up the
    notes refspec.
- **Files:** `crates/anvil-l4/src/notes.rs` (new),
  `crates/anvil-cli/src/templates/anvil-workflow.yml` (add
  fetch refspec).
- **Validation:** Round-trip: validate-at-l4 → notes fetch →
  verifier reads the note.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP2-016
- **Source:** MLP-006 deferred outcome, MLP-004 footnote 2.

#### MLP2-018: `required_anvil_version` policy evaluation

- **Status:** Merged
- **Intent:** `BranchRule.required_anvil_version` is parsed but
  not enforced. Adds the evaluation pass: refuse pushes from
  anvil versions below the floor.
- **Expected Outcome:**
  - L4 validate checks every commit's witness `anvil_version`
    against the policy floor.
  - Below-floor commits route to L4 (per policy) or block.
  - Clear diagnostic with the required version and the
    observed version.
- **Files:** `crates/anvil-l4/src/decide.rs` (extend
  `CommitDecision`).
- **Validation:** Boundary tests — equal-to-floor allows;
  below-floor rejects; above-floor allows.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-006, MLP-012
- **Source:** MLP-006 deferred outcome.
- **Evidence (Merged 2026-05-14 via PR #1567 at `96ad5d2d`):**
  Server-side mirror of MLP2-020's hook-side `check_version_floor`.
  New `evaluate_version_floor(policy_floor, witness_anvil_version)`
  in `crates/anvil-l4/src/decide.rs` returning typed
  `VersionFloorOutcome`: `Satisfied` / `WitnessVersionAbsent` /
  `BelowFloor { required, observed }` / `InvalidFloor { raw }` /
  `InvalidWitnessVersion { raw }`. Uses `semver::Version` directly
  so prerelease + build-metadata precedence matches
  `anvil_rules::RequiredAnvilVersion::parse` byte-for-byte. +9
  boundary pins including equal/above/below-floor, prerelease,
  build metadata, invalid floor, invalid witness, precedence
  ordering. Marked `#[allow(dead_code)]` until the L4 validate
  engine wires it through (Council quick reviewed, no MAJOR
  findings against the floor evaluator).

#### MLP2-019: L4 verification of witness `rules_sha` against recognised version

- **Status:** Merged
- **Intent:** L4 confirms the witness's `rules_sha` value
  resolves to a rule set the L4 server recognises (allows
  matching its policy floor).
- **Expected Outcome:**
  - L4 server holds a registry of recognised `rules_sha`
    values (from past releases).
  - Unrecognised `rules_sha` → route to full re-evaluation OR
    block per policy.
  - Coordination point with rule-pack distribution (vNext).
- **Files:** `crates/anvil-l4/src/recognised_rules.rs` (new).
- **Validation:** Unrecognised digest produces explicit
  diagnostic, not silent allow.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-012, MLP2-016
- **Source:** MLP-012 footnote 4.
- **Evidence (Merged 2026-05-14 via PR #1567 at `96ad5d2d`):**
  New `crates/anvil-l4/src/recognised_rules.rs` with
  `RecognisedRulesRegistry` (HashMap-backed O(1) lookup keyed by
  64-char lowercase-hex digest), `RuleSetMetadata { rules_sha,
  anvil_version, opa_runtime_version, rule_ids, config_sha,
  recognised_at }`, and `evaluate_rules_sha(registry,
  witness_rules_sha, on_no_witness)` returning typed
  `RulesShaOutcome::{ Absent, Recognised, AdmitUnrecognised,
  NeedsRevalidation, Block }`. Registry refuses empty /
  short / long / uppercase / non-hex digests at insert
  (`RegistryError::EmptyDigest` / `InvalidDigestShape`); refuses
  conflicting metadata under the same digest
  (`RegistryError::Conflict`); idempotent re-insert of identical
  records. Routing reuses `OnNoWitness` vocabulary as the v1
  unrecognised-rules_sha policy axis (documented; future schema
  bump may introduce a dedicated `on_unrecognised_rules_sha`
  field). +15 unit pins. Marked `#[allow(dead_code)]` until the
  daemon-side L4 validate engine wires it through.

#### MLP2-020: Hook-side `required_anvil_version` floor check at fire time

- **Status:** Merged
- **Intent:** `anvil hook pre-commit` reads
  `anvil/policy.yml`'s `required_anvil_version` and refuses to
  run (with a clear "upgrade anvil" message) if the running
  binary is below the floor.
- **Expected Outcome:**
  - Hook calls
    `RequiredAnvilVersion::parse(policy).satisfied_by(env!(CARGO_PKG_VERSION))`.
  - On failure: noise-disciplined one-line message + exit-0
    (don't block commits on an internal precondition).
  - Daemon-side check at registration mirrors this.
- **Files:** `crates/anvil-cli/src/commands/hook.rs`.
- **Validation:** Above-floor / equal / below-floor cases.
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** MLP-003, MLP-012
- **Source:** MLP-012 footnote 3.
- **Evidence (Merged 2026-05-14 via PR #1566 at `9ec726dd`):**
  Hook-side floor check wired after policy load. `check_version_floor`
  returns three outcomes: `Satisfied` (no floor pinned or current
  version ≥ floor), `BelowFloor` (running binary too old), and
  `InvalidFloor` (malformed semver). `BelowFloor` routes through new
  `ErrorClass::VersionFloor` (distinct line: "required_anvil_version
  not met — upgrade anvil (push admitted)"); `InvalidFloor` routes
  through `ErrorClass::EmbeddedFailed` ("validation errored") because
  the remediation is fixing the policy file, not upgrading the
  binary. Both admit the push per ADR-038 §D-6 (Serena rule). 6 new
  unit tests pin the outcomes including malformed-floor + malformed-
  current-version routings.

#### MLP2-021: `cutoff_commit` baseline-ancestry acceptance in pre-push

- **Status:** Merged
- **Intent:** Pre-push currently walks the literal pushed
  range only. Extend to accept the cutoff via a
  `git rev-list --first-parent` ancestry walk per pushed ref,
  so legacy commits behind the baseline are not re-validated.
- **Expected Outcome:**
  - `Policy::commit_is_before_cutoff(commit)` driven by ancestry
    walk against `cutoff_commit` (from
    `anvil-baseline::Baseline.cutoff_commit`).
  - Pre-push skips re-validation for commits before cutoff,
    treating them as baselined.
- **Files:** `crates/anvil-cli/src/commands/hook.rs` (extend
  `run_pre_push`), `crates/anvil-l4/src/policy.rs`.
- **Validation:** Adoption fixture — old commits before cutoff
  silently pass; new commits after cutoff get full validation.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-004, MLP-006, MLP-007
- **Source:** MLP-004 footnote 3.
- **Evidence (Merged 2026-05-14 via PR #1566 at `9ec726dd`):**
  New `first_parent_ancestry(repo_root, tip_sha)` helper shells to
  `git rev-list --first-parent --max-count=100000 <tip> --` per
  pushed ref (cap bounds pathologically deep histories so the git
  invocation itself cannot consume the 2 s budget; `is_hex_sha`
  guard + `--` terminator defend against revspec injection). The
  hook lazily builds a `(cutoff_index, HashMap<sha, index>)` lookup
  per ref so the per-commit cutoff check is O(1) instead of
  `Policy::commit_is_before_cutoff`'s O(N) double scan (Council
  kernel-maintainer follow-up). `Policy::validate()` now refuses
  non-hex `cutoff_commit` values (4–64 hex chars) with new
  `PolicyParseError::InvalidCutoffCommit` so symbolic refs like
  `HEAD`/branch names don't silently no-op at fire-time (Council
  adversarial follow-up). 7 new unit tests across `anvil-l4`
  (`policy::tests` + `resolve::tests`) and `anvil-cli`
  (`commands::hook::tests`) pin the ancestry shape, the
  hex-validation rejection cases, and the cutoff filter behaviour.

#### MLP2-022: Pre-push time-budget cap with `partial: true`

- **Status:** Merged
- **Intent:** ADR-038 names a 2s p95 budget for pre-push. v1
  walks unboundedly. Add the cap-trigger surface so very large
  pushes return `partial: true` rather than blocking developers
  for tens of seconds.
- **Expected Outcome:**
  - Pre-push tracks wall-clock budget.
  - On cap: stop walking, return `Allow` with a `partial: true`
    marker + a Kindling row recording the partial state.
  - Operator can opt-in to stricter "block on partial" via
    config.
- **Files:** `crates/anvil-cli/src/commands/hook.rs`.
- **Validation:** Long-range push fixture; cap triggers at 2s;
  Kindling row produced.
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** MLP-004
- **Source:** MLP-004 footnote 4.
- **Evidence (Merged 2026-05-14 via PR #1566 at `9ec726dd`):**
  `PRE_PUSH_BUDGET = Duration::from_secs(2)` (ADR-038 p95 target),
  `Instant::now()` at the start of the per-push-ref walk, between-
  commit budget check via `is_budget_exceeded(start, budget)`. On
  cap-exceeded the hook breaks out of the `'walk` label, emits one
  `ErrorClass::TimedOut` line via SuppressionLog with distinct
  rendering ("pre-push budget exceeded; partial validation, push
  admitted") and a structured `tracing::warn!` event with
  `kind = "gate_evaluated"`, `gate_id = "prePush"`, `partial = true`,
  `budget_ms`, `elapsed_ms`, `commits_processed`, and
  `commits_skipped_for_cutoff` so the future Kindling fan-out
  (deferred to INTD-004's IPC plumbing) can consume the partial-
  state observation directly. `ValidationPending` is suppressed
  when the budget fires so the operator sees exactly one
  informative line (Council follow-up — gates `engine_unavailable
  && !budget_exceeded`). Operator opt-in to "block on partial" is
  deferred to a follow-up policy field. 5 new unit tests pin the
  budget constant, ancestry-walk cap, `is_budget_exceeded`
  boundary, the suppression interaction, and the
  `ErrorClass::TimedOut` render.

### D. Multi-session + per-task fence isolation

#### MLP2-023: Registry session key change to `(WorktreeKey, AgentTag)`

- **Status:** Done
- **Intent:** Extend
  `crates/anvil-intercept/src/registry.rs` session key from
  `WorktreeKey` to `(WorktreeKey, Option<AgentTag>)` so multiple
  sub-agents per worktree are first-class.
- **Expected Outcome:**
  - `SessionRegistry::register` accepts the composite key.
  - `attribute_path` returns the right
    `(WorktreeKey, AgentTag)` for a writer (deterministic
    tiebreak: untagged first, then earliest-started + lexicographic
    `SessionId`).
  - Per-task fence: fence is keyed on the composite, so a bad
    sub-agent doesn't cascade-fence the whole worktree (MLP2-026
    consumes this surface).
  - Worktree-level fence remains for unattributable writers.
- **Files:** `crates/anvil-intercept-proto/src/lib.rs`
  (`SessionRecord.agent_tag: Option<AgentTag>` + matching
  `IpcCommand::RegisterSession.agent_tag`, both wire-additive via
  `serde(default, skip_serializing_if)`),
  `crates/anvil-intercept/src/registry.rs` (composite
  `by_composite: HashMap<(PathBuf, Option<AgentTag>), SessionId>`
  index + new `sessions_for_worktree` accessor + deterministic
  `attribute_path` tiebreak + per-tag `unregister`/`evict_stale`),
  plus mechanical call-site updates in
  `crates/anvil-intercept/src/{lib,ipc,fence,interrupt,status,auth}.rs`,
  `crates/anvil-cli/src/commands/intercept.rs` test fixtures, and
  `crates/anvil-intercept/tests/jsonrpc_conformance.rs`.
- **Validation:** Two sessions same worktree distinguished by
  AgentTag; per-task fence does not affect siblings; worktree
  fence on unattributable still applies to all.
  **Evidence (Done 2026-05-14):** `cargo test -p
  eddacraft-anvil-intercept` — 252 lib tests green (was 242
  baseline; +10 MLP2-023 tests in `registry::tests::*`:
  `two_distinct_tags_on_same_worktree_coexist`,
  `same_tag_on_same_worktree_returns_already_owned`,
  `untagged_and_tagged_on_same_worktree_coexist`,
  `second_untagged_session_on_same_worktree_returns_already_owned`,
  `attribute_path_prefers_untagged_session_then_earliest_tag`,
  `attribute_path_deterministic_tiebreak_across_tagged_only`,
  `unregister_one_tagged_session_leaves_sibling_alive`,
  `evict_stale_removes_only_the_expired_tagged_session`,
  `agent_tag_round_trips_through_active_sessions`,
  `session_dispatcher_trait_propagates_agent_tag`). Proto crate
  +4 wire-compat tests: legacy `SessionRecord` /
  `RegisterSession` shapes deserialise with `agent_tag: None`;
  new shapes round-trip with `Some`. **Backward-compat:** every
  existing caller passes `agent_tag: None` and observes the
  pre-MLP2-023 single-session-per-worktree semantics exactly;
  the new composite key only widens the invariant when a tag is
  supplied. **Wire-additive:** `agent_tag` is
  `#[serde(default, skip_serializing_if = "Option::is_none")]`
  on both `SessionRecord` and `IpcCommand::RegisterSession`, so
  older daemons / launchers parse new payloads cleanly when they
  don't declare the field (none of the in-tree consumers use
  `deny_unknown_fields`).
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** MLP-002, MLP-003, MLP-014
- **Source:** MLP-014 footnote 1.

#### MLP2-024: Per-worktree session cap configuration

- **Status:** Done
- **Intent:** `enforcement.session.per_worktree_max` (default
  16) caps how many sub-agents one worktree can host. Above
  the cap, new registrations are refused.
- **Expected Outcome:**
  - Field added to `anvil-config` enforcement-config schema.
  - `anvil-intercept` checks the cap at registration time;
    above-cap refuses with `RegistryError::SessionCapExceeded`.
  - Telemetry: cap-hit count + which worktree.
- **Files:** `crates/anvil-intercept-proto/src/enforcement_config.rs`
  (new `SessionConfigFile { per_worktree_max: Option<usize> }`
  block under `EnforcementConfigFile.session`),
  `crates/anvil-intercept/src/config.rs`
  (`Resolved.session_per_worktree_max: usize` with stricter-wins
  merge + zero-clamp; manual `Default` impl now that the field
  carries a non-zero baseline),
  `crates/anvil-intercept/src/registry.rs`
  (`SessionRegistry::with_per_worktree_cap` builder, cap field on
  `SessionRegistry`, cap-counting walk over `by_composite` at
  register time, new `RegistryError::SessionCapExceeded` variant),
  and the 7 existing `Resolved { ... }` literal sites in
  `embedded.rs` updated to include the new field. (The originally-
  listed `anvil-config` crate isn't where the enforcement-config
  schema lives — INTD-008 put it under
  `anvil-intercept-proto::enforcement_config`; this PR follows
  the existing home.)
- **Validation:** Cap=2 fixture; third registration refused;
  one session ends → next registration accepted. **Evidence
  (Done 2026-05-14):** `cargo test -p eddacraft-anvil-intercept
  --lib config:: registry::` — 9 new tests green (5 registry,
  4 config). Coverage:
  `third_session_on_capped_worktree_is_refused` (cap=2 + 3
  distinct tags → 3rd refused with `SessionCapExceeded { cap,
  live }`); `cap_freed_by_unregister_admits_next_registration`
  (unregister opens a slot); `cap_is_scoped_per_worktree`
  (cap=1 on wt-a does not block wt-b);
  `zero_cap_is_clamped_to_one` (operator-typo defence);
  `cap_counts_tagged_and_untagged_together` (composite-key
  semantics compose correctly). Config-resolution tests:
  default 16 when unset; project value honoured; stricter-wins
  picks the smaller value; zero clamped to 1. 261 intercept-lib
  tests pass (was 252 baseline); telemetry on `cap-hit count +
  which worktree` is delivered via the `SessionCapExceeded`
  variant's `{ worktree, cap, live }` payload — IPC-side
  emission lands when MLP2-058's tracing surface picks it up.
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** MLP2-023
- **Source:** MLP-014 footnote 2.

#### MLP2-025: Registry-side spoof rejection cross-check

- **Status:** Draft
- **Intent:** Env-supplied `AgentTag` must match the tag the
  daemon issued for this PID lineage at INTL-003
  registration. Mismatches treated as missing, not honoured.
- **Expected Outcome:**
  - At each enforcement decision, daemon walks the writer's
    PID lineage and looks up registered ancestors.
  - If env tag exists but doesn't match any registered
    ancestor for this lineage → strip the tag, downgrade to
    worktree-level fence.
  - Logged as `degraded:spoofed-attribution`.
- **Files:** `crates/anvil-intercept/src/registry.rs`,
  `crates/anvil-intercept/src/auth.rs`.
- **Validation:** Spoof test — process sets `ANVIL_AGENT_TAG`
  to a fake value → registry treats it as unattributable.
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** MLP-014, MLP2-023
- **Source:** MLP-014 footnote 6.

#### MLP2-026: `degraded:fence-cascade` mode at 5 fences in 60s

- **Status:** Draft
- **Intent:** When five fences fire within 60s, the daemon
  enters `degraded:fence-cascade` mode requiring operator-
  clear. Uses the shared rate-window primitive from MLP2-009.
- **Expected Outcome:**
  - `anvil-intercept::fence` consumes
    `anvil-intercept::rate_window::SlidingCount(5, 60s)`.
  - Cascade mode emits an explicit operator-touch surface
    (`anvil intercept unblock --acknowledge-cascade`).
  - Until cleared, new sessions for the worktree are refused.
- **Files:** `crates/anvil-intercept/src/fence.rs`.
- **Validation:** Burst test — fire 5 fences in 60s →
  cascade engaged; sixth registration refused; clear surface
  works.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP2-009, MLP2-023
- **Source:** MLP-014 footnote 3.

### E. Cross-platform attribution

#### MLP2-027: macOS `pid_starttime` + `parent_pid`

- **Status:** Draft
- **Intent:** Extend `anvil-attribution` to macOS via `sysctl
  kern.proc.pid.<pid>` (wrapped in `nix`); workspace policy
  forbids the raw `unsafe { libc::sysctl(...) }` call.
- **Expected Outcome:**
  - `pid_starttime_macos` returns the Unix-seconds start time
    from `kp_proc.p_starttime`.
  - `parent_pid_macos` returns `kp_eproc.e_ppid`.
  - `cfg(target_os = "macos")` branches in
    `crates/anvil-attribution/src/process.rs`.
- **Files:** `crates/anvil-attribution/src/process.rs`.
- **Validation:** macOS CI runner runs the existing test suite
  with macOS-specific cases.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-014
- **Source:** MLP-014 footnote 4.

#### MLP2-028: Windows `pid_starttime` + `parent_pid`

- **Status:** Draft
- **Intent:** Extend `anvil-attribution` to Windows via
  `GetProcessTimes` + `Process32First`/`Process32Next` (wrapped
  in `windows-rs`); workspace policy forbids raw unsafe FFI.
- **Expected Outcome:**
  - `pid_starttime_windows` via `OpenProcess` +
    `GetProcessTimes` (creation time → Unix seconds).
  - `parent_pid_windows` via the `tlhelp32` snapshot.
  - `cfg(target_os = "windows")` branches.
- **Files:** `crates/anvil-attribution/src/process.rs`,
  `crates/anvil-attribution/Cargo.toml` (Windows target
  dependency).
- **Validation:** Windows CI runner runs the test suite.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-014
- **Source:** MLP-014 footnote 4.

### F. TypeScript driver-client mirrors

#### MLP2-029: AgentTag wire-shape mirror in driver-client

- **Status:** Done
- **Intent:** TypeScript mirror at
  `packages/anvil-driver-client/src/session/` consuming the
  proto JSON shape pinned in
  `crates/anvil-intercept-proto/src/session.rs`.
- **Expected Outcome:**
  - `AgentTag` interface with the three fields (`driver_id`,
    `claimed_agent_id`, `pid_starttime`) plus a hand-rolled
    `parseAgentTag` (the driver-client has no Zod dep; per-field
    type guards keep the runtime footprint zero and the error
    messages typed).
  - `ANVIL_AGENT_TAG_ENV` / `ANVIL_TASK_ID_ENV` constants.
  - Round-trip parity test with the Rust serialisation.
- **Files:** `packages/anvil-driver-client/src/session/types.ts`
  (new), `packages/anvil-driver-client/src/session/types.test.ts`
  (new), `packages/anvil-driver-client/src/session/index.ts`
  (barrel),
  `packages/anvil-driver-client/src/index.ts` (re-export). Subdir
  layout matches `diagnostics/types.ts` rather than the spec's
  original `session.ts` so the parity test sits next to the type
  definitions, mirroring the in-repo TS convention.
- **Validation:** Cross-language parity test (encode in Rust,
  decode in TS, deep-equal the original). **Evidence (Done
  2026-05-14):** `pnpm test` in `packages/anvil-driver-client`
  — 153 tests green (was 143 baseline; +10 MLP2-029 tests).
  Coverage:
  - Env-var constants match `ANVIL_AGENT_TAG_ENV` /
    `ANVIL_TASK_ID_ENV` byte-for-byte against the Rust
    `pub const &str` values.
  - `parseAgentTag(JSON.parse(RUST_EMITTED_JSON))` deep-equals
    the Rust-equivalent object (the fixture string is the exact
    output of the Rust `agent_tag_round_trips_through_json`
    test).
  - Forward-compat: unknown future keys on the wire are silently
    dropped, mirroring the Rust struct's lack of
    `#[serde(deny_unknown_fields)]`.
  - Typed-error rejects for null / non-object input, missing
    required fields (each named in the message), non-integer
    `pid_starttime` (fractional, negative, Infinity, NaN).
  - `JSON.stringify(makeAgentTag(...))` produces the byte-exact
    Rust-emitted JSON (insertion order matches serde field
    order), so the TS → Rust direction round-trips too.
  - `parse(make(...))` lossless; distinct `pid_starttime` makes
    distinct tags (PID-reuse defence, mirroring the Rust
    `distinct_pid_starttimes_produce_distinct_tags` test).
  `pnpm typecheck`, `pnpm format:check` (1319 files), and
  `pnpm lint:check` (nx clippy + fmt-check across 26 projects)
  all clean.
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** MLP-014
- **Source:** MLP-014 footnote 5.

#### MLP2-030: Mid-edit Kindling observation builder mirror

- **Status:** Done
- **Intent:** TypeScript mirror of
  `anvil-intercept::kindling_observation::from_midedit_response`
  for the daemon-unreachable embedded-fallback path.
- **Expected Outcome:**
  - `packages/anvil-driver-client/src/kindling/` ships
    `fromMidEditResponse(ctx, response)` returning
    `GateEvaluatedObservation | null` with identical volume-
    control rules and severity → enforcement mapping.
  - Wire-shape parity test against the Rust builder.
- **Files:** `packages/anvil-driver-client/src/kindling/types.ts`
  (type definitions + builder + severity / enforcement helpers),
  `packages/anvil-driver-client/src/kindling/types.test.ts`
  (parity tests, 13), `packages/anvil-driver-client/src/kindling/index.ts`
  (barrel), `packages/anvil-driver-client/src/index.ts` (root
  re-export). Subdir layout matches the MLP2-029 / `session/` and
  `diagnostics/` patterns rather than the spec's flat
  `kindling.ts` so the parity test sits next to the types.
- **Validation:** Parity test in
  `packages/anvil-driver-client/src/kindling/types.test.ts`.
  **Evidence (Done 2026-05-14):** `pnpm test kindling` in
  `packages/anvil-driver-client` — 13 tests green (166 total
  package tests; was 153 baseline after MLP2-029). Coverage:
  - Constants pinned (`KIND_GATE_EVALUATED` / `MIDEDIT_GATE_ID`
    match Rust).
  - Headline byte-exact parity test against a `serde_json::to_string`
    fixture captured from a one-shot Rust test (mixed-severity
    error + warning batch → blocking enforcement + violation/
    warning counts + `rules_violated` populated).
  - JSON round-trip via `JSON.parse(JSON.stringify(obs))`
    preserves field equality.
  - Volume-control contract: empty diagnostics → `null`,
    matching Rust `from_midedit_response` returning `None`.
  - Severity → enforcement mapping (error-only → `blocking`,
    warning-only → `warning`, info-only → `informational`,
    mixed batch picks the worst).
  - Optional `rules_violated` field is **omitted from the wire**
    (key absent) when no diagnostics qualify, mirroring Rust's
    `#[serde(skip_serializing_if = "Option::is_none")]`.
  - Caller-supplied `ObservationContext` plumbing
    (`session_id` / `timestamp` / `gate_eval_id` /
    `duration_ms` / `file_path`) round-trips into the row.
  - Pinned constants (`kind = "gate_evaluated"`, `gate_id =
    "midEdit"`) are unconditional — the builder ignores any
    caller attempt to override.
  `pnpm typecheck` clean, `pnpm format:check` (1319 files)
  clean, `pnpm lint:check` (nx clippy + fmt-check across 26
  projects) clean. **Closes Group F (2/2).**
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-016
- **Source:** MLP-016 footnote 3.

### G. Baseline policy + identity wiring

#### MLP2-031: `cutoff_commit` pinning into `anvil/policy.yml`

- **Status:** Merged
- **Intent:** `anvil baseline` writes the cutoff commit back
  into `anvil/policy.yml` so the L4 policy lane (anvil-l4)
  reads it from the policy file rather than from
  `baseline.json`.
- **Expected Outcome:**
  - Baseline save path writes (or updates) policy with
    `cutoff_commit: <sha>`.
  - Policy parser surfaces this on `BranchRule.cutoff_commit`.
  - Round-trip: baseline → policy parsed → cutoff matches.
- **Files:** `crates/anvil-baseline/src/io.rs`,
  `crates/anvil-l4/src/policy.rs`,
  `crates/anvil-policy/` (if a higher-level writer exists).
- **Evidence (Merged 2026-05-14 via PR #1567 at `96ad5d2d`):**
  New `pin_cutoff_commit(path, cutoff)` in
  `crates/anvil-l4/src/policy.rs` with typed `PolicyPinError::{Io,
  Parse, NotAnObject, BaselineNotAMap, InvalidCutoffCommit,
  Serialise, SymlinkRefusal}`. Atomic temp-then-rename writer with
  hex-shape pre-flight (so a malformed cutoff never reaches disk),
  symlink refusal on both the policy path and the temp sibling
  (mirrors `anvil_baseline::io::save`'s TOCTOU pattern), and
  multi-format round-trip (yaml / yml / json / toml). Preserves
  additive top-level fields (forward-compat); does not preserve
  comments (documented v1 limitation). Refuses on non-object root
  and on non-map `baseline:` field so a hand-edited scalar under
  `baseline:` is never silently overwritten with a fresh map.
  +9 unit pins covering round-trip across all four formats,
  invalid-cutoff refusals, missing-file refusal, non-object root,
  non-map baseline, atomic write, unknown-field preservation,
  symlink refusal on path and temp sibling. The
  `anvil-baseline/src/lib.rs` "out of scope" note updated to point
  callers at `anvil_l4::pin_cutoff_commit`; the higher-level
  `anvil baseline` orchestrator wires it in via MLP2-032 (separate
  PR). Marked `#[allow(dead_code)]` until that orchestrator
  picks it up. Council quick review flagged 3 MAJOR — all fixed
  (`NotAnObject` ambiguity → split into `BaselineNotAMap`;
  `atomic_replace` Windows comment rewritten; redundant
  `#[allow(dead_code)]` cleaned up). semver/tempfile workspace-dep
  hoisting punted (codebase convention is bare per-crate strings
  across 13 crates).
- **Validation:** Round-trip + a multi-format check (yaml /
  json / toml policy all accept the field).
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-006, MLP-007
- **Source:** MLP-007 footnote 3.

#### MLP2-032: `anvil baseline` writes project identity

- **Status:** Merged
- **Intent:** Baseline CLI command (`anvil baseline`) calls
  `identity::ensure_project_id` alongside its bootstrap work,
  so adopting Anvil into an existing repo writes
  `anvil/project-id` and the baseline file in the same flow.
- **Expected Outcome:**
  - `commands/baseline.rs` invokes
    `crates/anvil-cli/src/activation/identity.rs::ensure_project_id`
    on the target path.
  - Idempotent on re-run (existing identity preserved).
  - Co-pins `cutoff_commit` into `anvil/policy.{yml,…}` via
    `anvil_l4::pin_cutoff_commit` (MLP2-031) when the baseline
    record carries one and a policy file exists.
- **Files:** `crates/anvil-cli/src/commands/baseline.rs`,
  `crates/anvil-l4/src/lib.rs` (re-export `pin_cutoff_commit`),
  `crates/anvil-l4/src/policy.rs` (drop `#[allow(dead_code)]`
  now the orchestrator wires the symbol).
- **Evidence (Merged 2026-05-15 via PR #1575 at `a40525ad`):**
  `run_create_or_refresh` now calls `ensure_project_id` instead
  of erroring on absent `anvil/project-id` — first-run mints a
  fresh v7 UUID, re-run preserves the existing identity (council
  C-2 re-read pattern from `ensure_project_id` itself). After
  `save()`, the orchestrator calls `try_pin_cutoff` which uses
  `anvil_config::discover` (canonical yaml > yml > json > toml
  precedence) to locate the policy file, then dispatches to
  `anvil_l4::pin_cutoff_commit`. Cutoff resolution falls back to
  the policy file's existing `baseline.cutoff_commit` on
  first-create so the two files cannot silently diverge. The pin
  step is best-effort: a missing or unreadable policy file emits
  a one-line hint and does not fail `anvil baseline`
  (warnings-over-blocks). +6 unit pins:
  `create_mints_identity_when_absent`,
  `create_is_idempotent_on_identity_when_present`,
  `refresh_pins_cutoff_into_policy_when_present`,
  `refresh_does_not_fail_when_no_policy_file_to_pin`,
  `pin_targets_yaml_over_yml_when_both_present` (Council #C-1
  regression guard against hand-rolled candidate-list drift),
  `create_picks_up_cutoff_from_policy_when_baseline_absent`
  (Council #C-2 first-create convergence guard).
  `cargo test --workspace` clean; `cargo clippy --workspace
  --all-targets -- -D warnings` clean. Council quick review on
  PR #1575 found 2 MAJOR + 3 MINOR + 1 NIT — both MAJORs
  (precedence drift, first-create divergence) folded into the
  same branch with regression tests; MINOR/NIT addressed
  inline (TOCTOU caveat documented on
  `scan_repo_for_findings`; `find_policy_file` rewritten on
  top of `anvil_config::discover` so the local candidate list
  is gone).
- **Validation:** First-run vs re-run; symlink refusal (covered
  by `ensure_project_id`'s own pins); cutoff round-trip into
  policy.yml.
- **Confidence:** high
- **Priority:** High
- **Dependencies:** MLP-001, MLP-007, MLP2-031
- **Source:** MLP-001 footnote 3, MLP-007 footnote 5.

#### MLP2-033: `--new-identity` fork opt-out CLI flag

- **Status:** Draft
- **Intent:** `anvil start --new-identity` mints a fresh
  `project_uuid` instead of inheriting from the parent repo
  (which is the current fork behaviour). Lives on `anvil
  start` and `anvil baseline`.
- **Expected Outcome:**
  - `--new-identity` clears any existing
    `forked_from`-inherited UUID and writes a fresh v7 UUID.
  - Default behaviour (without the flag) preserves the
    current "fork inherits" semantics.
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/commands/baseline.rs`.
- **Validation:** Fork tree fixture: parent uuid A → child A
  (no flag) → grandchild B (with flag).
- **Confidence:** high
- **Priority:** Low
- **Dependencies:** MLP-001, MLP-007, MLP2-032
- **Source:** MLP-001 footnote 2.

#### MLP2-034: Scanner integration — populate `BaselineFinding` from anvil-checks

- **Status:** Merged (Phase 1 only; Phase 2 deferred)
- **Intent:** Wire `anvil-checks`'s diagnostic pipeline output
  through to `anvil-baseline::BaselineFinding` so
  `anvil baseline --refresh` actually records what the rules
  found.
- **Expected Outcome:**
  - **Phase 1 (this PR):** scan path
    `anvil-checks::antipattern::run_antipattern_check(...)` →
    warnings → `BaselineFinding { rule_id, file_path,
    fingerprint }` where `fingerprint` comes from
    `anvil-baseline::compute_fingerprint(rule.id, source_line)`.
    Both `anvil baseline` (initial) and
    `anvil baseline --refresh` produce a populated record.
  - **Phase 2 (follow-up):** diff partition
    (`anvil-baseline::Baseline::diff`) drives the "new edges
    only" gate at the hook lane. Tracked by MLP2-035 / -036's
    consumers, not duplicated here.
- **Files:** `crates/anvil-cli/src/commands/baseline.rs`
  (scanner orchestration, file walk, snippet re-read for
  fingerprinting). No surface change to `anvil-checks` —
  consumed via its existing `run_antipattern_check` entry point.
- **Evidence (Phase 1 Merged 2026-05-15 via PR #1575 at `a40525ad`):**
  `scan_repo_for_findings` walks the worktree with
  `ignore::WalkBuilder` (matching `anvil check --all`'s SCAN-001
  shape but rooted at the explicit baseline target), calls
  `run_antipattern_check` against the default extension set, and
  builds one `BaselineFinding` per non-suppressed warning. The
  source line at `warning.location.line` is re-read from the
  same file the scanner just consumed, then fed to
  `compute_fingerprint(warning.id, snippet)` — same
  move-resistance contract as MLP-007's library tests
  (whitespace-noisy snippet → normalised → 16-hex digest). Per-
  finding errors (read failure, empty snippet, fingerprint
  rejection) are silently skipped so adoption is never blocked
  by a transient I/O race or exotic encoding (warnings-over-
  blocks). Suppressed warnings are dropped — the author's
  explicit acknowledgement disqualifies them from the
  baseline. +2 unit pins:
  `create_populates_findings_from_scanner` (AP-003 surfaces on
  `src/app.ts` with the expected file_path + 16-hex
  fingerprint), `refresh_repopulates_findings_after_new_violation`
  (`--refresh` rewrites the record after a new violation
  appears).
- **Validation (Phase 1):** Adoption fixture: scan repo →
  baseline populated. Refresh fixture: empty scan → introduce
  AP-003 → `--refresh` → AP-003 surfaces.
- **Validation (Phase 2 — pending):** Diff partition with one
  new violation across an existing baseline; hook-lane gate
  consumes the partition.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-007, anvil-checks
- **Source:** MLP-007 footnote 2.

#### MLP2-035: Adversarial-refresh detection (`degraded:baseline-suspicious`)

- **Status:** Draft
- **Intent:** Detect baseline refreshes that look like
  adversarial whitewashing (huge violation drop without a
  corresponding code-size reduction) and surface as
  `degraded:baseline-suspicious`.
- **Expected Outcome:**
  - Heuristic: refresh that removes >N findings without code
    churn touches a threshold.
  - Operator-clear surface with explicit acknowledgement.
  - Configurable threshold for projects with legitimate large
    refactors.
- **Files:** `crates/anvil-baseline/src/diff.rs` (extend),
  `crates/anvil-intercept/src/fence.rs` (degraded mode).
- **Validation:** Adversarial fixture: remove 90% of findings
  without code change → degraded mode fires.
- **Confidence:** low (needs threshold tuning)
- **Priority:** Low
- **Dependencies:** MLP-007, MLP2-034
- **Source:** MLP-007 footnote 6.

#### MLP2-036: Async continuation for >100k file baselines

- **Status:** Draft
- **Intent:** `anvil baseline` currently scans synchronously.
  Add async continuation + a "partial baseline" marker so
  huge monorepos don't time out during adoption.
- **Expected Outcome:**
  - Scan emits a partial baseline file with a `continuation:
    <cursor>` marker.
  - Resume reads the cursor and continues; merges into the
    full baseline at the end.
  - Performance: 100k files complete within a documented
    budget (TBD; profile first).
- **Files:** `crates/anvil-baseline/src/io.rs`,
  `crates/anvil-cli/src/commands/baseline.rs`.
- **Validation:** Fixture of 100k synthetic files; full +
  resumed flow produces same final baseline.
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** MLP-007, MLP2-034
- **Source:** MLP-007 footnote 7.

### H. Hook + config surface completion

#### MLP2-037: `anvil hook bootstrap --witness-recent` mode

- **Status:** Merged
- **Intent:** Walk `<remote>..HEAD`, run validation against
  each unwitnessed commit, write retroactive witnesses tagged
  `validation_at: bootstrap-recovery`. Recovers from
  worktree-bootstrap failure (hooks didn't fire).
- **Expected Outcome:**
  - `--witness-recent` flag triggers the walk.
  - Each missing-witness commit gets a retroactive line.
  - One-line success: `anvil: bootstrapped (N commits
    witnessed retroactively)`.
- **Files:** `crates/anvil-cli/src/commands/hook.rs` (extend
  bootstrap dispatch), `crates/anvil-hook/src/bootstrap.rs`.
- **Validation:** Fixture: 3 commits with missing witnesses
  → bootstrap-recent → all three carry the recovery tag.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-002, MLP-003, MLP-008
- **Source:** MLP-008 deferred outcome.

#### MLP2-038: `merge=union -text` orchestrator step for `.gitattributes`

- **Status:** Merged
- **Intent:** The activation orchestrator pre-positions
  `.gitattributes` (MLP-001 step 1a-b); the explicit step
  that writes `anvil/witness/active.ndjson merge=union -text`
  lands here.
- **Expected Outcome:**
  - Orchestrator writes the `.gitattributes` entry on first
    activation; idempotent on re-run.
  - Parallel-branch commits to the witness file produce a
    clean union merge (validated by the canonical line
    encoding from MLP-002).
- **Files:** `crates/anvil-cli/src/activation/orchestrator/mod.rs`.
- **Validation:** Merge-conflict fixture: two branches each
  append a different line → merge produces both lines, no
  conflict markers.
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** MLP-001, MLP-002
- **Source:** MLP-002 footnote 3.

#### MLP2-039: `anvil start --format json|toml` CLI flag

- **Status:** Merged
- **Intent:** Operators can choose `.anvil.json` or
  `.anvil.toml` at adoption time instead of the default yaml.
- **Expected Outcome:**
  - `--format` flag on `anvil start`; default `yaml`.
  - Orchestrator writes the config in the chosen format using
    `anvil-config` writers.
  - Round-trip: all three formats produce byte-identical
    canonical-JSON output (MLP-011's invariant).
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/activation/orchestrator/mod.rs`.
- **Validation:** Format matrix: each chosen format produces
  the right file extension + parses back through `discover`.
- **Confidence:** high
- **Priority:** Low
- **Dependencies:** MLP-011
- **Source:** MLP-011 footnote 1.

#### MLP2-040: `.anvilrc` → `.anvil.<ext>` filename migration

- **Status:** Merged
- **Intent:** Migrate the existing `.anvilrc` reader (in
  `commands/gate.rs`) to the multi-format `.anvil.<ext>`
  surface from MLP-011, while keeping `.anvilrc` working as
  a deprecation tail.
- **Expected Outcome:**
  - `.anvil.yaml` / `.yml` / `.json` / `.toml` discovered
    first via MLP-011's `discover`; falls back to `.anvilrc`
    if none.
  - One-time `anvil migrate` (or analogous) command writes
    the new file from the old.
- **Files:** `crates/anvil-cli/src/commands/gate.rs`,
  `crates/anvil-cli/src/commands/migrate.rs` (new).
- **Validation:** Existing `.anvilrc` projects still work;
  new-format projects skip the fallback.
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** MLP-011
- **Source:** MLP-011 footnote 2.

#### MLP2-041: Typed `AnvilConfig` schema

- **Status:** Merged
- **Intent:** Each consumer surface (init, gate, policy)
  evolves its own typed view over the same
  `serde_json::Value` intermediate from MLP-011, instead of
  passing untyped values around.
- **Expected Outcome:**
  - Per-consumer `*ConfigView` structs in each consumer crate.
  - `from_value(&serde_json::Value)` for each, with validation
    at the type boundary.
  - Migration is incremental — consumers move at their own
    pace.
- **Files:** Spans `crates/anvil-cli/src/commands/{init,gate}.rs`,
  `crates/anvil-policy/`.
- **Validation:** Each consumer has its own typed-view test.
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** MLP-011
- **Source:** MLP-011 footnote 3.

### I. GitHub Action publishing

#### MLP2-042: External `eddacraft/anvil-action` Marketplace repo

- **Status:** Draft
- **Intent:** Stand up the
  `github.com/eddacraft/anvil-action` publishing repo —
  `action.yml`, bundled binary install, semver-tagged
  releases, Marketplace listing.
- **Expected Outcome:**
  - Separate repo with `action.yml` declaring `policy`,
    `fail-on-warning`, `anvil-version` inputs.
  - Bundled binary install matching the Marketplace
    fingerprint (SHA-256 verified).
  - Marketplace listing live; major-version tag `v1` moves
    on minor / patch releases.
- **Files:** New repo `github.com/eddacraft/anvil-action`.
- **Validation:** PR against a sandbox repo using `uses:
  eddacraft/anvil-action@v1` produces a check status.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-010, MLP2-016 (anvil l4-validate)
- **Source:** MLP-010 footnote 1.

#### MLP2-043: Activation orchestrator writes `.github/workflows/anvil.yml`

- **Status:** Draft
- **Intent:** `anvil start` / `anvil baseline` write the
  template at `crates/anvil-cli/src/templates/anvil-workflow.yml`
  into a target repo's `.github/workflows/anvil.yml` at
  adoption time.
- **Expected Outcome:**
  - Orchestrator detects whether the workflow file already
    exists; writes if absent, leaves alone if present.
  - Same pattern for `anvil-audit.yml` (MLP2-053).
  - Output indicates which file was written.
- **Files:** `crates/anvil-cli/src/activation/orchestrator/mod.rs`.
- **Validation:** Greenfield + re-run + existing-file cases.
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** MLP-010
- **Source:** MLP-010 footnote 2.

#### MLP2-044: Branch-protection integration end-to-end test

- **Status:** Draft
- **Intent:** Live integration test that verifies "require
  check before merge" against the published action. Confirms
  the Marketplace listing surfaces the right check status.
- **Expected Outcome:**
  - Test repo with branch protection requiring the Anvil
    check.
  - PR producing a `block` decision fails the check;
    merge button disabled.
  - PR producing `pass` allows merge.
- **Files:** External test-rig repo (not in anvil-001).
- **Validation:** Manual E2E run before each Marketplace
  release.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP2-042
- **Source:** MLP-010 footnote 3.

#### MLP2-045: Major-version tag automation

- **Status:** Draft
- **Intent:** `v1` major-version tag auto-tracks the latest
  minor / patch release. Lives in the external publishing
  repo's release workflow.
- **Expected Outcome:**
  - Release workflow on `eddacraft/anvil-action` moves the
    `v1` tag on every successful minor / patch release.
  - Tag-move is signed and traceable to the originating
    semver tag.
- **Files:** External `eddacraft/anvil-action/.github/workflows/release.yml`.
- **Validation:** Two successive patch releases; `v1` moves
  both times.
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** MLP2-042
- **Source:** MLP-010 footnote 4.

#### MLP2-046: `anvil l4-validate` binary surface

- **Status:** Merged
- **Intent:** Dedicated `anvil l4-validate` CLI subcommand
  (rather than the current `anvil hook pre-push` reuse). The
  template + Marketplace action both swap to this binary
  when it ships.
- **Expected Outcome:**
  - New `crates/anvil-cli/src/commands/l4_validate.rs` calls
    `anvil-l4`'s engine over a commit range.
  - Template's `anvil hook pre-push` invocation swaps to
    `anvil l4-validate` in a follow-up patch.
- **Files:** `crates/anvil-cli/src/commands/l4_validate.rs`,
  `crates/anvil-cli/src/main.rs` (register),
  `crates/anvil-cli/src/templates/anvil-workflow.yml`
  (swap step).
- **Validation:** Same test coverage as the existing
  pre-push lane; parity test ensures behaviour matches.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-006, MLP2-016
- **Source:** MLP-006 deferred lane, MLP-010 footnote 5.

#### MLP2-047: Pre-push end-to-end subprocess integration tests

- **Status:** Draft
- **Intent:** Helper coverage is 40+ tests across anvil-hook
  / anvil-l4 / anvil-cli::commands::hook. The run-the-binary
  smoke pass lands here, exercising the actual subprocess
  flow.
- **Expected Outcome:**
  - Test fixture spawns `anvil hook pre-push` as a
    subprocess with synthesised stdin.
  - Asserts exit code, stderr lines, and the witness chain
    state.
  - Lands alongside MLP2-052 (driver / CLI / MCP-shim
    conformance pass) since both touch the binary surface.
- **Files:** `crates/anvil-cli/tests/pre_push_subprocess.rs`.
- **Validation:** Tests run on Linux CI; macOS / Windows
  smoke variants follow MLP2-027 / MLP2-028.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-004, MLP2-046, MLP2-052
- **Source:** MLP-004 footnote 5.

### J. Protection-claim render conformance

#### MLP2-048: `anvil status --json` render path

- **Status:** Done
- **Intent:** `crates/anvil-cli/src/commands/status.rs`
  emits `ProtectionClaim` from a daemon-snapshot input.
  Closes the HARD-GATE rendering surface.
- **Expected Outcome:**
  - Status command queries the daemon for the worktree
    snapshot.
  - Builds `ProtectionClaim` from the snapshot — closed-set
    state mapping + per-surface entries.
  - `--json` emits the validated wire shape; default emits
    a terse one-line claim.
- **Files:** `crates/anvil-cli/src/commands/status.rs`,
  `crates/anvil-intercept/src/status.rs` (snapshot source).
- **Validation:** Per-state fixture (see MLP2-049) round-trips
  through the render path; rendered string matches the
  spec.
- **Confidence:** medium
- **Priority:** Critical (HARD-GATE close)
- **Dependencies:** MLP-009
- **Source:** MLP-009 footnote 1.

#### MLP2-049: Per-state golden fixture files

- **Status:** Merged
- **Intent:** Per-state JSON snapshots at
  `crates/anvil-cli/tests/fixtures/status_v1/` — one per
  worktree state (10) + one per surface state (8).
- **Expected Outcome:**
  - 18 fixture files; each pinned by an `assert_snapshot!`
    test in `crates/anvil-cli/tests/status_render.rs`.
  - Fixture generation: synthesise a daemon-snapshot input
    that produces each state; capture the JSON.
  - Re-running the render against the fixture produces
    byte-identical output.
- **Files:** `crates/anvil-cli/tests/fixtures/status_v1/`,
  `crates/anvil-cli/tests/status_render.rs` (new).
- **Validation:** All 18 snapshots green; intentional changes
  require explicit snapshot-update.
- **Confidence:** high
- **Priority:** High
- **Dependencies:** MLP-009, MLP2-048
- **Source:** MLP-009 footnote 2.

#### MLP2-050: TypeScript e2e mirror of protection-claim states

- **Status:** Draft
- **Intent:** End-to-end conformance test at
  `apps/e2e/src/protection_claim_states.spec.ts`
  exercising the rendered surface against the closed-set
  vocabulary.
- **Expected Outcome:**
  - Spec drives the system into each of the 10 worktree
    states + 8 surface states.
  - Reads `anvil status --json` output and validates against
    the TS Zod schema (mirrors `protection_claim.rs`).
  - Pass-no-finding states do not produce phantom rows.
- **Files:** `apps/e2e/src/protection_claim_states.spec.ts`,
  `apps/e2e/src/lib/protection_claim_schema.ts` (Zod mirror).
- **Validation:** All 18 states reachable in the test
  harness; render matches.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-009, MLP2-048, MLP2-049
- **Source:** MLP-009 footnote 3.

#### MLP2-051: Driver / CLI / MCP-shim protection-claim conformance pass

- **Status:** Draft
- **Intent:** Every surface that renders a protection claim
  consumes
  `crates/anvil-kernel-types/src/protection_claim.rs`
  types rather than pattern-matching strings. Audit each
  surface and rip out string-based renders.
- **Expected Outcome:**
  - Audit list: `anvil status`, `anvil doctor`, MCP shim
    `validation.rs`, editor driver client, GitHub Action
    check status.
  - Each migrated to consume the closed-set types.
  - Parity test: same input → same rendered claim across
    surfaces.
- **Files:** Spans `crates/anvil-cli/src/commands/{status,doctor}.rs`,
  `crates/anvil-cli/src/mcp/validation.rs`,
  `packages/anvil-driver-client/src/protection_claim.ts`.
- **Validation:** Cross-surface parity test.
- **Confidence:** medium
- **Priority:** Critical (HARD-GATE close)
- **Dependencies:** MLP-009, MLP2-048
- **Source:** MLP-009 footnote 4.

#### MLP2-052: Additive-optional-fields forward-compat test

- **Status:** Done
- **Intent:** Pin that adding an optional `degraded_reasons`
  / `cross_boundary_token` field doesn't bump
  `schema_version` and consumers ignore unknown optional
  fields.
- **Expected Outcome:**
  - Test fixture: input JSON includes an unknown optional
    field; deserialise succeeds, the unknown field is
    ignored on serialise.
  - Schema-version stays `anvil.protection-claim.v1`.
  - Documents the additivity rule in
    `protection_claim.rs`'s module docstring.
- **Files:** `crates/anvil-kernel-types/src/protection_claim.rs`
  (extend test module),
  `crates/anvil-cli/tests/protection_claim_states.rs`.
- **Validation:** Additive-fields tests cover the round-trip
  + unknown-field-ignore behaviour.
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** MLP-009
- **Source:** MLP-009 footnote 5.

### K. Kindling activation orchestrator follow-ups

#### MLP2-053: Activation orchestrator writes `.github/workflows/anvil-audit.yml`

- **Status:** Draft
- **Intent:** Mirrors MLP2-043 for the audit-chain workflow.
  `anvil start` / `anvil baseline` write the template at
  `crates/anvil-cli/src/templates/anvil-audit-workflow.yml`
  into a target repo's `.github/workflows/anvil-audit.yml`.
- **Expected Outcome:**
  - Same write-if-absent semantics as MLP2-043.
  - Idempotent on re-run.
- **Files:** `crates/anvil-cli/src/activation/orchestrator/mod.rs`.
- **Validation:** Greenfield + re-run + existing-file cases.
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** MLP-015
- **Source:** MLP-015 footnote 2.

#### MLP2-054: Kindling `gate_evaluated` emission for `anvil audit-chain`

- **Status:** Draft
- **Intent:** The audit-chain command currently produces a
  JSON `AuditReport`. Add a Kindling row per audit run with
  `mode: audit` so historical drift is queryable through the
  observation timeline.
- **Expected Outcome:**
  - Audit-chain consumer (CLI + workflow) calls a builder
    that produces a `GateEvaluatedObservation` with
    `gate_id: "audit-chain"` and `inputs.baseline_hash`
    populated.
  - Pass / fail / degraded mapping matches the report's
    drift state.
- **Files:** `crates/anvil-cli/src/commands/audit_chain.rs`
  (extend), `crates/anvil-intercept/src/kindling_observation.rs`
  (audit-chain builder).
- **Validation:** Audit-chain run produces a row; query the
  row back through kindling-integration.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-015, MLP2-006
- **Source:** MLP-015 footnote 1.

#### MLP2-055: `anvil audit-chain` rule re-scoring via anvil-checks

- **Status:** Draft
- **Intent:** v1 audit-chain is a witness-presence check.
  Extend to re-run the rule engine across history (sharing
  the pipeline with the L4 validate lane from MLP2-016).
- **Expected Outcome:**
  - Audit-chain optionally re-evaluates each commit's
    contents against the current rule set; reports rule
    drift (commits that would block today but were allowed
    historically).
  - Off by default; `--rescan` opt-in to limit nightly
    runtime.
- **Files:** `crates/anvil-cli/src/commands/audit_chain.rs`.
- **Validation:** History fixture where a rule was added
  after some commits → re-scan flags those commits.
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** MLP-015, MLP2-016
- **Source:** MLP-015 footnote 3.

#### MLP2-056: `anvil audit-chain` time-budget cap

- **Status:** Draft
- **Intent:** Bound runtime for very large histories so the
  nightly cron doesn't run away. Profile first; cap second.
- **Expected Outcome:**
  - `--max-runtime <seconds>` flag (default unbounded for
    backwards compat).
  - On cap: stop walking, report `partial: true` in the
    audit report.
  - Kindling row records the partial state (re-uses MLP2-054).
- **Files:** `crates/anvil-cli/src/commands/audit_chain.rs`.
- **Validation:** Long-history fixture; cap triggers; report
  marked partial.
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** MLP-015, MLP2-054
- **Source:** MLP-015 footnote 4.

### L. Production hardening (Council follow-ons)

Items in this group were filed from the
[Council session `council-e2fdfc0c`](../reviews/) 2026-05-14 review of
the MLP2-001 + MLP2-002 PR (#1522). Each task closes a deferred
finding the reviewers flagged as out-of-scope-for-the-original-PR but
needed before the daemon-side cache is wired into the production
enforcement path. The originating finding IDs are in each task's
`Source:` line so the audit trail stays explicit.

These items extend MLP2's scope by one rule: production-hardening
follow-ons on MLP2's *own* surface are tracked here rather than in a
new module, because the cost of a separate module (new ID prefix, new
status row, new release-window line) outweighs the benefit of strict
"every MLP2 task closes an MLP-018 catalogue item" provenance. The
Source line distinguishes Group L tasks from Group A–K tasks.

#### MLP2-057: Bounded rule-set cache with LRU eviction + unregister hook

- **Status:** Done
- **Intent:** `RuleSetCache` today is unbounded
  (`Mutex<HashMap<WorktreeKey, RuleSetEntry>>` with no cap) and no
  callback fires when a session is unregistered, so a long-running
  daemon attributing many short-lived worktrees accumulates stale
  entries indefinitely. Cap the cache, add LRU eviction on insert
  when at capacity, and hook `SessionRegistry::unregister` +
  `evict_stale` to call `invalidate(&worktree_key)` so cache
  lifetime tracks session lifetime.
- **Expected Outcome:**
  - `RuleSetCache::with_capacity(max_entries)` constructor; default
    `max_entries = 1024` (sized against INTD-016 session cap).
  - LRU policy on insert when at capacity — evict the
    least-recently-used `WorktreeKey` and emit a cache-pressure
    telemetry event.
  - `SessionRegistry::unregister(...)` and the TTL eviction path
    (`evict_stale`) invoke `RuleSetCache::invalidate` on the
    departing worktree's key, so a register/unregister cycle leaves
    no cache residue.
  - Saturation counter (`cache.entries_count`, `cache.evictions`)
    exposed for MLP2-058 to surface.
- **Files:** `crates/anvil-intercept/src/rule_cache.rs`,
  `crates/anvil-intercept/src/registry.rs` (unregister + evict_stale
  hooks).
- **Validation:** Fill cache to capacity + 1 → oldest entry
  evicted; register-then-unregister a worktree → cache returns
  `Miss` after unregister; concurrent insert + evict do not
  deadlock.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP2-001
- **Source:** Council 2026-05-14 #C-007 / #C-018 / #C-024 (security
  + operations: unbounded HashMap is a slow-burn memory DoS on a
  privileged long-running daemon).

#### MLP2-058: Tracing + status-surface instrumentation for rule_cache + in_flight

- **Status:** Merged
- **Intent:** The MLP2-001 cache and MLP2-002 in-flight counter
  ship as library primitives with zero `tracing::` calls and no
  exposure via the daemon's `query_status` IPC handler. An operator
  consulting daemon logs during a config-edit propagation incident
  sees nothing. Wire structured logging into the cache + in-flight
  paths and add the corresponding fields to `DaemonStatus`.
- **Expected Outcome:**
  - `tracing::debug!` on `RuleSetCache::get_or_resolve` miss path
    and `invalidate_on_change` per-key drops.
  - `tracing::info!` when `invalidate_on_change` returns a
    non-empty vec (config-edit propagation observable).
  - `tracing::warn!` once when `WatcherIntegration::new` is built
    with `rule_cache = None` (the production-wiring gap from
    MLP2-014 is visible at startup).
  - `tracing::warn!` on `RuleSetCache::lock` poisoned-recovery
    (Council #C-025).
  - `DaemonStatusV1` (proto) gains
    `cache_entries: Option<u32>` +
    `cache_invalidations_total: Option<u64>` +
    `in_flight_evaluations: Option<u8>` fields, **each annotated
    with `#[serde(default, skip_serializing_if = "Option::is_none")]`
    matching the precedent set by
    `ScanBufferResponse.rules_sha` in MLP2-002**. The optional
    shape preserves forward-compat: a newer consumer parsing a
    v1-only payload (older daemon that has not been upgraded yet)
    sees `None`; an older consumer parsing the new payload ignores
    the unknown keys cleanly. The `query_status` IPC handler reads
    the underlying counters and emits `Some(...)`; the CLI
    renderer surfaces them when present and omits the row when
    absent (Council #C-016 / PR #1526 review).
- **Files:**
  `crates/anvil-intercept/src/{rule_cache,midedit,watcher,status}.rs`,
  `crates/anvil-intercept-proto/src/status.rs`,
  `crates/anvil-cli/src/commands/status.rs` (renderer).
- **Validation:** Integration test capturing a tracing subscriber
  asserts the expected event shape; `anvil intercept status --json`
  surfaces the new fields; the renderer formats them. **Wire-compat
  test:** a v1-shaped payload (no new fields) round-trips through
  the new `DaemonStatusV1` deserialiser without error; a payload
  carrying the new fields round-trips through a hypothetical v1-only
  deserialiser that does not declare them (using
  `#[serde(deny_unknown_fields)]` *must remain off* on every
  consumer; assert in the test).
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP2-001, MLP2-002
- **Source:** Council 2026-05-14 #C-008 / #C-009 / #C-012 / #C-013 /
  #C-014 / #C-015 / #C-025 (operations: "if it happens and isn't
  logged it didn't happen" + no operator-visible cache surface).

#### MLP2-059: Per-worktree invalidation rate limit

- **Status:** Draft
- **Intent:** Pre-attribution cache invalidation in
  `WatcherIntegration::ingest_at` is the correct semantic choice
  (cache cleared even for unattributable writers), but it lets an
  attacker with write access to a worktree's parent drive thousands
  of invalidations per second by repeatedly touching `.anvil.*`.
  Each invalidation forces a YAML reparse on the next access. Cap
  the per-worktree invalidation rate with a sliding-window or
  token-bucket primitive and coalesce over-cap events.
- **Expected Outcome:**
  - Per-worktree token bucket (default ~10 invalidations/second,
    burst size 16) in `RuleSetCache::invalidate_on_change`.
  - Over-cap invalidations are coalesced (single eviction +
    counter increment) rather than dropped.
  - `cache.invalidate.rate_limited` counter exposed via MLP2-058's
    status surface.
- **Files:** `crates/anvil-intercept/src/rule_cache.rs`
  (rate-window primitive), `crates/anvil-intercept/src/watcher.rs`.
- **Validation:** Fire 100 `.anvil.yaml` writes in 1s → cache
  evicts once, counter records the 99 coalesced events.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP2-001, MLP2-058
- **Source:** Council 2026-05-14 #C-023 (security: pre-attribution
  invalidation is an attacker-driven DoS amplifier).

#### MLP2-060: YAML resource-bounds hardening in anvil-config parser

- **Status:** Done
- **Intent:** `anvil-config::parse_file` dispatches `.anvil.yaml` /
  `.yml` through `serde_yaml 0.9.34+deprecated` straight into a
  `serde_json::Value`. The crate has no recursion-depth limit and
  expands YAML aliases (`&anchor` / `*anchor`) during deserialisation,
  so a post-parse depth walk runs only after the alias graph has
  already been materialised in memory — which is exactly the
  billion-laughs damage. The MLP2-001 cache resolver invokes the
  parser on every cache miss, materially increasing the rate at
  which untrusted YAML is parsed. Defend against billion-laughs /
  deeply-nested-mapping attacks **at parse time**, not after
  (Council #C-023b clarified PR #1526 review).
- **Expected Outcome:**
  - Pre-parse size cap on `.anvil.*` files (1 MiB) via
    `std::fs::metadata` check before `read_to_string`.
  - **Parse-time alias-expansion defence**, picked from one of
    (decision recorded in the ADR-level note below):
    1. **Reject aliases outright** — the simplest fix. Run a
       streaming YAML lexer pass (e.g. `yaml-rust` event-stream
       or `saphyr-parser`) before handing bytes to `serde_yaml`;
       if any `*alias` token appears, fail with
       `ParseError::AliasNotPermitted`. `.anvil.*` configs are
       hand-edited and small; no operator needs anchors.
    2. **Cap alias-expansion cost** — pre-parse the document into
       a YAML event stream, count anchor refs, reject if the
       expansion-product would exceed (say) 64 KiB worth of
       nodes. Lets benign anchors through; more code.
  - A *secondary* post-parse depth walk on the resulting
    `serde_json::Value` rejects documents whose nested depth
    exceeds 32. This is defence-in-depth, not the primary control
    — it catches deeply-nested maps that arrived without aliases.
  - Caps enforced in `anvil-config::parse_file` (the primary
    boundary) and as a fast-path size pre-check in
    `crate::rule_cache::resolve_for_worktree`.
  - ADR-level note: decide between option (1) and option (2);
    track migration to `serde_yaml_ng` / `serde-yaml-bw`
    (maintained successors) which may make alias control easier.
- **Files:** `crates/anvil-config/src/parse.rs`,
  `crates/anvil-intercept/src/rule_cache.rs` (resolve fast-path).
- **Validation:** Fuzz fixtures.
  **Evidence (Done 2026-05-14, option (1) — reject aliases
  outright):** `cargo test -p eddacraft-anvil-config --lib` —
  70 tests green (was 60 baseline; +10 MLP2-060 tests).
  Coverage:
  - **Classic billion-laughs payload** (5-level nested
    `&a0 [lol]` + `*a0` references; ~200 bytes on disk, would
    expand to gigabytes under unbounded alias resolution) →
    `ParseError::AliasNotPermitted` at the byte-scanner gate,
    BEFORE `serde_yaml` materialises the alias graph. This is
    the primary regression test for the alias defence.
  - Single anchor (`&a foo`) → rejected.
  - Single alias (`*a`) → rejected.
  - `&` / `*` inside double-quoted scalars (`"https://example.com/a&b=*"`)
    → accepted (scanner correctly treats them as data).
  - `&` / `*` inside single-quoted scalars (`'a&b *foo'`) →
    accepted.
  - `&` / `*` inside `#` comments → accepted.
  - Operator-realistic `.anvil.yaml` (`enforcement.mode: warn`,
    `session.per_worktree_max: 8`, `telemetry.allow_cross_session: false`)
    parses cleanly.
  - 1 MiB + 16-byte JSON payload → `ParseError::FileTooLarge`
    at the `fs::metadata` check, BEFORE `read_to_string`.
  - 40-deep JSON object → `ParseError::DepthExceeded` at the
    post-parse walk (cap 32).
  - 30-deep JSON object (under cap) → parses normally.
  **Implementation choice (option 1 — reject aliases outright,
  per the spec's two options):** picked option (1) because
  `.anvil.*` configs are hand-edited and operator-realistic
  configs never use anchors. The byte scanner is conservative
  (a literal `&` or `*` in an unquoted scalar would
  false-positive) but the false-positive mitigation is simple:
  quote the value, or use JSON / TOML. ADR note deferred —
  the inline doc on `scan_for_yaml_aliases` records the
  decision rationale. Migration to `serde_yaml_ng` / `serde-
  yaml-bw` (maintained successors) tracked separately. Caps
  enforced in `anvil-config::parse_file`; the
  `rule_cache::resolve_for_worktree` already routes through
  `parse_file`, so the size + alias + depth checks apply
  transparently on every cache miss (no separate fast-path
  needed). `MAX_CONFIG_FILE_BYTES = 1 MiB` and `MAX_PARSED_DEPTH = 32`
  exposed as `pub const`s for downstream consumers.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-011
- **Source:** Council 2026-05-14 #C-023b (security minor:
  untrusted YAML parsing surface amplified by the cache
  resolver). Acceptance plan tightened in PR #1526 review after
  reviewer flagged that post-parse depth walks run too late to
  defend against alias-expansion attacks.

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| A. Daemon enforcement + observation | 10 (MLP2-001..-010) | 4/10 |
| B. Witness chain extensions | 5 (MLP2-011..-015) | 0/5 |
| C. L4 policy execution | 7 (MLP2-016..-022) | 2/7 |
| D. Multi-session + fence isolation | 4 (MLP2-023..-026) | 2/4 |
| E. Cross-platform attribution | 2 (MLP2-027..-028) | 0/2 |
| F. TypeScript driver-client mirrors | 2 (MLP2-029..-030) | 2/2 |
| G. Baseline + identity wiring | 6 (MLP2-031..-036) | 3/6 |
| H. Hook + config surface completion | 5 (MLP2-037..-041) | 5/5 (Complete) |
| I. GitHub Action publishing | 6 (MLP2-042..-047) | 0/6 |
| J. Protection-claim render conformance | 5 (MLP2-048..-052) | 0/5 |
| K. Kindling activation orchestrator | 4 (MLP2-053..-056) | 0/4 |
| L. Production hardening (Council follow-ons) | 4 (MLP2-057..-060) | 1/4 |
| **Total** | **60** | **12/60** |

## Recommended landing order

The 60 items have natural sequencing through their `Dependencies:`
declarations. High-priority lanes that unblock the most downstream
work:

1. **MLP2-023** (registry key change) — unblocks D group and most
   of A.
2. **MLP2-016** (`validate_at_l4`) — unblocks I group's
   Marketplace action.
3. **MLP2-001** (rules-sha cache) — unblocks A group.
4. **MLP2-048** + **MLP2-051** (status render + conformance) —
   closes the HARD-GATE protection-claim surface.

Group A, D, and C-1 (MLP2-016) form the daemon-enforcement
critical path. Group I depends on MLP2-016. Group J closes the
protection-claim contract.

## Priority and phasing plan

Derived from each task's declared `Dependencies:` line. Every MLP
dependency is already Done (the MLP module is Complete 18/18), so
phasing is driven entirely by **MLP2-internal** dependencies plus
crate-file contention.

### Load-bearing tasks (prioritise these first)

These five tasks gate the largest fanout downstream — every other
group has at least one item waiting on one of them. Land them
ahead of the rest of their respective groups in Phase 1.

| Task | Group | Why load-bearing |
| --- | --- | --- |
| **MLP2-023** | D | Registry session-key change unblocks A1 / A3 / D2 / D3 / D4 + the spoof-rejection chain |
| **MLP2-016** | C | `validate_at_l4` engine unblocks C2 / C4 / I1 / I5 / K3 — the L4 + Marketplace + audit-rescore lanes |
| **MLP2-009** | A | Shared rate-window primitive unblocks A6 (Kindling emit) + D4 (`fence-cascade`) + K2 (audit-chain row) |
| **MLP2-048** | J | `anvil status --json` render path unblocks the entire J group's HARD-GATE closure |
| **MLP2-001** | A | Daemon-side rules-sha cache unblocks A2 + B4 (writer wiring), plus is the natural integration point for A6/A7 |

### Phase 1 — Day-0 starters (33 tasks, fully parallel up to file conflicts)

Every task here depends only on Done MLP items. Can start
immediately; the only coordination is around shared files (see
"Parallelisation notes" below).

- **A:** MLP2-001¹, MLP2-004, MLP2-005, MLP2-007, MLP2-008,
  MLP2-009¹
- **B:** MLP2-011, MLP2-012, MLP2-013, MLP2-015
- **C:** MLP2-016¹, MLP2-018, MLP2-020, MLP2-021, MLP2-022
- **D:** MLP2-023¹
- **E:** MLP2-027, MLP2-028
- **F:** MLP2-029, MLP2-030
- **G:** MLP2-031, MLP2-032, MLP2-034
- **H:** MLP2-037, MLP2-038, MLP2-039, MLP2-040, MLP2-041
- **I:** MLP2-043
- **J:** MLP2-048¹, MLP2-052
- **K:** MLP2-053
- **L:** MLP2-060

¹ = load-bearing for Phase 2; land first within the group. **Note:**
MLP2-001 moved from Phase 2 → Phase 1 on 2026-05-14 after audit
resolved its `MLP2-023` listing to a `Coordinates with:` callout
(see MLP2-001 body); the cache is worktree-scoped and is
forward-compatible with MLP2-023's session-key extension.

### Phase 2 — Phase-1-gated (16 tasks pending; MLP2-002 + MLP2-003 + MLP2-024 pre-shipped)

Each entry shows its gating Phase-1 dependency. **MLP2-002 was
co-shipped with MLP2-001 in wave 1A (2026-05-14) — it appears here
under its original Phase-2 placement for traceability but is Done;
see the task body for evidence.** Council 2026-05-14 #C-031 /
#C-039.

| Task | Gated by |
| --- | --- |
| MLP2-014, MLP2-057 | MLP2-001 |
| MLP2-058 | MLP2-001 + MLP2-002 |
| MLP2-025 | MLP2-023 |
| ~~MLP2-024~~ (Done 2026-05-14) | MLP2-023 |
| ~~MLP2-003~~ (Done 2026-05-14) | MLP2-023 |
| MLP2-026 | MLP2-023 + MLP2-009 |
| MLP2-006 | MLP2-009 |
| MLP2-017, MLP2-019, MLP2-042, MLP2-046, MLP2-055 | MLP2-016 |
| MLP2-049, MLP2-051 | MLP2-048 |
| MLP2-033 | MLP2-032 |
| MLP2-035, MLP2-036 | MLP2-034 |
| ~~MLP2-002~~ (Done 2026-05-14 with MLP2-001) | MLP2-001 |

### Phase 3 — Phase-2-gated (7 tasks)

| Task | Gated by |
| --- | --- |
| MLP2-010, MLP2-054 | MLP2-006 |
| MLP2-044, MLP2-045 | MLP2-042 |
| MLP2-050 | MLP2-049 |
| MLP2-047 | MLP2-046 + MLP2-052 |
| MLP2-059 | MLP2-058 |

### Phase 4 — Tail (1 task)

| Task | Gated by |
| --- | --- |
| MLP2-056 | MLP2-054 |

### Parallelisation notes

The phases above are dependency-correct; the practical question is
which Phase-1 tasks can run *concurrently* across engineers /
agents vs need to serialise because they edit the same files.

**Fully parallel across groups.** Groups E (cross-platform
attribution), F (TS driver-client mirrors), I (when not blocked
by MLP2-016), and the external Marketplace repo for MLP2-042 all
sit in disjoint crates / repos and never collide. These are the
easiest concurrent picks.

**Contention clusters to serialise:**

- **`crates/anvil-intercept/src/registry.rs`** — MLP2-023 (key
  change), MLP2-001 (cache), MLP2-003 (composite identity at
  attach), MLP2-024 (session cap), MLP2-025 (spoof check). The
  file is ~1000 lines and these tasks all extend the
  `SessionRegistry` surface. **Land MLP2-023 first**, then the
  rest of D + A1–A3 sequentially or with very careful
  rebases.
- **`crates/anvil-l4/src/decide.rs`** — MLP2-016 (engine),
  MLP2-018 (`required_anvil_version` eval), MLP2-019
  (`rules_sha` verify), MLP2-021 (cutoff ancestry). All
  extend `CommitDecision`. Land MLP2-016 first; the others
  can branch off it.
- **`crates/anvil-intercept/src/fence.rs`** — MLP2-026
  (`fence-cascade`) is the only Phase-1 / 2 fence-touching task,
  but it depends on MLP2-009's rate-window primitive (new file).
  Land MLP2-009 first, then MLP2-026 picks it up.
- **`crates/anvil-cli/src/commands/status.rs`** — MLP2-048 and
  MLP2-051 both rewrite the render path; MLP2-048 builds it,
  MLP2-051 audits all consumers. Land MLP2-048 first (in Phase
  1), MLP2-051 follows in Phase 2.
- **`crates/anvil-cli/src/activation/orchestrator/mod.rs`** —
  MLP2-038 (`.gitattributes` step), MLP2-039 (`--format` flag),
  MLP2-043 (`anvil.yml` writer), MLP2-053 (`anvil-audit.yml`
  writer). All extend the same orchestrator dispatch table.
  These four can be batched into a single PR or landed
  sequentially with trivial rebases.
- **`crates/anvil-attribution/src/process.rs`** — MLP2-027
  (macOS) + MLP2-028 (Windows). The Linux path is shipped; the
  two new platforms add `#[cfg]` branches and don't collide with
  each other. Fully parallel.

**Cross-team coordination:**

- **MLP2-008 (RTAI-007 telemetry join)** has an external
  dependency on the RTAI module. Park until RTAI-007 surfaces;
  the wire shape from `anvil-intercept::kindling_observation`
  is already pinned so the join lands as a Rust-side field map
  when RTAI-007 is ready.
- **MLP2-042 (external `eddacraft/anvil-action` repo)** lives
  outside this repo. Sequence relative to anvil-001 releases
  rather than relative to other MLP2 tasks; the in-repo
  template (MLP-010) already references the future
  `eddacraft/anvil-action@v1` so the swap is a one-line change
  for adopters.

**Suggested first wave (highest parallelism × highest unblock
value):**

1. Day 0: **MLP2-023** (registry refactor, single-track) +
   **MLP2-016** (L4 engine, single-track on `anvil-l4`) +
   **MLP2-009** (rate-window primitive, new file) +
   **MLP2-048** (status render, single-track on `status.rs`).
2. Day 0 parallel filler: groups E, F, H (orchestrator items
   batched), MLP2-043, MLP2-053, MLP2-052.
3. Day 0+: G group (031, 032, 034) on baseline + identity.
4. Once any of MLP2-023 / -016 / -009 / -048 / -034 lands, kick
   off their Phase-2 dependents.

## Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Registry refactor (MLP2-023) ripples across many call sites | High | High | Land behind a feature flag; staged migration with the existing key as a fallback |
| L4 engine `validate_at_l4` (MLP2-016) performance regression vs hook-side | Medium | Medium | Benchmark first; the existing 40+ helper tests cover the pure-logic path |
| Cross-platform PIDs (MLP2-027 / -028) introduce flaky tests | Medium | Medium | Linux remains the canonical surface; macOS / Windows are opt-in CI matrix lanes |
| External `eddacraft/anvil-action` repo (MLP2-042) supply-chain compromise | Low | Critical | SHA-256 pin for the bundled binary; major-tag move signed; documented rotation procedure |
| Kindling burst rate-shaping (MLP2-009) drops too many observations | Medium | Medium | Configurable thresholds; `degraded:observation-throttled` makes drops observable |

## Decisions

1. **MLP2 is a follow-up module, not a v2 of the spec.** It
   contains zero new capabilities — every task closes a v1
   deferral. New capabilities go through their own planning
   module.
2. **One-to-one traceability from primitive to integration
   debt.** Every MLP2 task names its originating MLP task /
   footnote / PR in the `Source:` line.
3. **Group A–K mirrors the MLP-018 catalogue.** Reorganising
   would lose the traceability guarantee from decision 2.
4. **Each task is plannable in isolation.** Acceptance criteria
   are explicit; no task says "tracked as follow-up" without
   the follow-up living in its own entry.

## Coordinates with

- **INTD** — daemon enforcement pipeline; the bulk of group A
  + D lands inside `anvil-intercept`.
- **DRVR** — driver framework; group F (TS mirrors) plus the
  driver side of group J's conformance pass.
- **RMCP / RMCPF** — MCP shim consumes the closed-set
  vocabulary and the Kindling builder (MLP2-007 / MLP2-051).
- **RTAI** — RTAI-007 telemetry join (MLP2-008).
- **LAUNCH** — `anvil start` activation orchestrator (group
  G, H, I, K).
- **kindling-integration** — Kindling SQLite consumer for
  group A's observation emissions.
