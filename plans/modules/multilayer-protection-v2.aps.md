# Multi-Layer Protection v2 (Integration + Follow-ups)

| ID   | Owner  | Status      | Progress   |
| ---- | ------ | ----------- | ---------- |
| MLP2 | @aneki | In Progress | 63/83 |

**Last reviewed:** 2026-05-22 (MLP2-051f advanced In Progress →
Merged via PR
[#1840](https://github.com/eddacraft/anvil-001/pull/1840) at
`e1cc066a` — activation diagnostic consumes the daemon
`ProtectionClaim` snapshot via
`promote_to_live_validation_when_daemon_attests` so
`anvil start --verify` and `anvil status --verify` reach
`protecting` when the intercept daemon attests the canonical
worktree. Closes GH
[#1831](https://github.com/eddacraft/anvil-001/issues/1831).
Done-count advances 62 → 63; total stays 83.)

Earlier 2026-05-22 (MLP2-051f filed under Group J —
activation diagnostic consumes the daemon `ProtectionClaim` snapshot
so `anvil start --verify` can reach `protecting` when the intercept
daemon attests the current worktree. Implementation slice for the
activation-daemon-evidence wire-up spec
(`plans/specs/2026-05-21-activation-daemon-evidence-wireup.md`).
Hard-gate precursors all merged: MLP2-075 (Windows IPC parity, PR
#1836) and MLP2-051h (`DaemonStatusV1::generated_at_unix` wire-add,
on main at `4ec9c5a4`). Module total advances 82 → 83; done-count
unchanged at 62. Status: In Progress.)

Earlier 2026-05-22 (MLP2-051h filed under Group J —
`DaemonStatusV1::generated_at_unix` wire-add precursor to the
MLP2-051f activation diagnostic. Filed ahead of MLP2-051f per the
activation-daemon-evidence wire-up spec §"APS placement" so the
field exists on the wire before the first consumer arrives; does
not block the `v0.7.0-beta` tag (activation diagnostic does not
exist yet). Module total advances 81 → 82; done-count unchanged at
62.)

Earlier 2026-05-21 (Group R added — MLP2-074 daemon-side
`session.report_process` IPC handler. Filed against the
[v0.7.0-beta pre-tag release council](../reviews/release-council/2026-05-21-v0.7.0-beta-pre-tag.md)
action A2 and tracked at GH
[#1827](https://github.com/eddacraft/anvil-001/issues/1827); does not block
the `v0.7.0-beta` tag (launcher absorbs the gap, ships as Known Gap).
Module total advances 80 → 81; done-count unchanged at 62.)

Earlier 2026-05-21: Group Q added — MLP2-072 MCP auth-gate
shape + MLP2-073 pre-write summary dedupe. Both filed against the
[2026-05-21 new-user journey audit](../audits/2026-05-21-new-user-journey-audit.md)
and tracked at GH [#1796](https://github.com/eddacraft/anvil-001/issues/1796)
and [#1799](https://github.com/eddacraft/anvil-001/issues/1799); neither
blocks the `v0.7.0-beta` tag. Module total advances 78 → 80; done-count
unchanged at 60.

Earlier 2026-05-21: MLP2-071 advanced `Blocked` → `Ready`
after the cross-session-attribution design pass landed at
[`plans/specs/2026-05-21-intd-015-cross-session-attribution-design-pass.md`](../specs/2026-05-21-intd-015-cross-session-attribution-design-pass.md).
The spec is the named unblock the prior `Blocked on:` line carried;
implementation slice contract + validation matrix are now part of the
MLP2-071 entry. Module total stays at 78; done-count stays at 60 — `Ready`
is a planning status, not a done-count advance.

Earlier 2026-05-20: Group P added — MLP2-070 lineage anchor
daemon-derivation hardening + MLP2-071 INTD-015 cross-session policy
follow-up. Both filed against the release council pass 1 verdicts for
[#1674](https://github.com/eddacraft/anvil-001/issues/1674) and
[#1722](https://github.com/eddacraft/anvil-001/issues/1722); neither blocks
the `v0.7.0-beta` tag. Module total advances 76 → 78; done-count unchanged
at 60. INTD-015 follow-up filed here rather than in
`intercept-daemon.aps.md` because that module is archived at 16/16
Complete.

Earlier 2026-05-19: MLP2-068 advanced `In Progress` →
`Merged` after implementation commit `d54a5f86`; Group O advances 0/2 →
1/2 and module progress advances 59/76 → 60/76. MLP2-069 remains Draft and
does not gate `v0.7.0-beta`.

Earlier 2026-05-18: MLP2-025 umbrella closed — Phase 1
primitives shipped via PR #1597 on 2026-05-15; Phase 2 (-025b PR
#1603) and Phase 3 (-025c PR #1608) had already shipped on
2026-05-16, leaving the umbrella's status stale at `In Progress
(Phase 1 only…)`. Advanced `In Progress` → `Merged`; done-count
54 → 59 of 76 (the +5 closes the prior counter drift between
the module header and the index narrative, which had already
counted -025b/-025c and the -051a..-051e wave as done). Earlier
2026-05-17 entry preserved below — MLP2-025c closed via PR #1608 at
`1ea23349` — launcher migration that activates the MLP2-025/-025b
spoof cross-check in production: `session_register_params`
emits nested `agent_tag` + `lineage` (daemon parser has been
waiting for these since MLP2-023), `RegistrationRequest` gains
`launcher_pid`, TS driver-client `AnvilScanBufferParams` gains
optional `env_agent_tag` and `validateMidEdit` forwards
`process.env.ANVIL_AGENT_TAG`. Cross-check is now live —
`Cross::Match` admits, `Cross::Spoofed` blocks + fences with
`degraded:spoofed-attribution`. MLP2-025c advances `In Progress`
→ `Merged`; done-count 53 → 54 of 76. Also reconciles the
Group D stats footer that drifted across the MLP2-026
2026-05-17 closure: Group D 3/6 → 5/6 (was previously stale
after MLP2-026's bump to Merged left the footer untouched).
Earlier 2026-05-17: MLP2-026 closed via PR #1624 at
`5e3798da` — `degraded:fence-cascade` mode ships persisted
`CascadeRecord` state in `FenceFile`, `RateWindow::new(4, 60s)`
on `FenceStore`, `WorktreeStatus`/`WorktreeStatusV1` `cascaded`
+ `cascade_since` fields, registry-side `WorktreeCascaded`
refusal under documented cascade-before-registry lock ordering,
`IpcCommand::UnblockCascade { worktree, operator }` with
daemon-derived `OperatorContext`, and the
`anvil intercept unblock --acknowledge-cascade <worktree>` CLI
affordance — implementation follows
`plans/specs/2026-05-16-mlp2-026-fence-cascade-control-lane.md`
§3–§9 verbatim. MLP2-026 advances `In Progress` → `Merged`;
done-count 52 → 53 of 76 (rebased on top of MLP2-051b closure
at PR #1668). Earlier 2026-05-17: MLP2-051 re-spec
on branch `chore/aps-mlp2-051-respec` — split into MLP2-051
umbrella + 051a..051e sub-tasks after a 2026-05-17 audit showed
only `anvil status` renders a `ProtectionClaim` today; the other
four target surfaces — `anvil doctor`, MCP shim, TS
driver-client, GH Action — emit no claim and need additive
rendering rather than string-to-typed migration. Total count
went 71 → 76 with no done-count change. MLP2-048 closed 2026-05-16 via PR #1625 at
`f13e1014` — `anvil status --json` now consumes the daemon
snapshot via the new `build_protection_claim_from_wire` adapter,
with a documented empty-`surfaces` fallback when the daemon is
unreachable. 2026-05-16: audit closure for MLP2-016 — PR #1627
merged at `0aacdac8` binds `CommitAntipatternEngine` in production
`run_pre_push` + `l4_validate::run`, satisfying the 2026-05-15
audit's "real engine + e2e test without fixture injection" gate;
MLP2-016 advanced `In Progress` → `Merged`. Earlier 2026-05-15: full MLP/MLP2 Council audit
reopened MLP2-016 and MLP2-048 from Done to In Progress: production
pre-push still binds the no-op L4 engine, and `anvil status --json`
still emits a local-only claim with empty `surfaces`. Added Group M
(MLP2-061..066) for review-discovered corrective work. Earlier history: wave 1G shipped 2026-05-15 via PR #1576
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

> **Release cut-line (2026-05-15 Council audit).** MLP2 is not required to
> reach 66/66 for `v0.7.0-beta`; the cut is the subset needed for the public
> sustained-use claim. Required before Boring Week: MLP2-011 (including merge-
> parent binding), MLP2-013, MLP2-014, reopened MLP2-016, reopened MLP2-048,
> MLP2-061, and MLP2-062. MLP2-050/051 are required only if the release claim
> includes non-CLI protection-claim parity. Marketplace publishing (MLP2-042..
> 045) is deferred pending the licensing / pricing model lock for
> distributing `eddacraft/anvil-action` through the GitHub Marketplace —
> the gate is the commercial / redistribution decision, not the Boring
> Week validation timing. Observation fan-out (MLP2-006..008/010/054) and
> cross-platform attribution (MLP2-027/028) remain deferred unless Boring
> Week feedback exercises those surfaces.

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

- **Status:** Done
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
- **Files:** `crates/anvil-intercept/src/kindling_observation.rs`
  (new `KindlingObservationSink` trait + `MidEditObservationEmitter`
  + `RecordingKindlingObservationSink` test sink +
  `NoopKindlingObservationSink` default + `MidEditEmissionRequest`
  per-call inputs + `EmissionOutcome` reporter +
  `DEFAULT_MIDEDIT_EMIT_CAPACITY` / `_WINDOW` defaults),
  `crates/anvil-intercept/src/midedit.rs` (`ScanBufferService` gains
  `with_observation_emitter` builder + `observation_emitter`
  accessor; no scan-path changes — the emitter sits behind the
  existing service surface), `crates/anvil-intercept/src/ipc.rs`
  (`scan_buffer_from_jsonrpc` threads `traceparent` through,
  captures `file_path` + `started` before the await, derives
  `gate_eval_id` from the W3C parent-id with UUID v4 fallback via
  new `derive_gate_eval_id`, and fires the emitter on success —
  pre-write mode skips the emit per ADR-031),
  `packages/kindling-integration/src/adapter.ts` (typed
  `emitGateEvaluated(observation, capsuleId?)` consumer-side entry
  point delegating to existing `emit()` so daemon-emitted rows have
  a type-checked TS-side ingestion contract).
- **Validation:** Adversarial — Kindling DB locked → response
  still returns; rate-limit primitive (MLP2-009) prevents flood.
  **Evidence (Done 2026-05-15):** `cargo test -p
  eddacraft-anvil-intercept` — 366 tests green (+13 new vs baseline
  353): 8 new unit tests in `kindling_observation::tests` cover
  no-finding short-circuit, sink delivery, rate-window throttle
  accumulation, allow-after-throttle pending-drops report,
  sink-failure swallow + recovery, NoopSink contract, daemon
  session-id introspection, and the recording-sink fail-next hook;
  5 new IPC integration tests in `ipc::tests` cover the end-to-end
  daemon path
  (`handle_jsonrpc_value_emits_gate_evaluated_for_finding_bearing_scan`
  proves traceparent → `b7ad6b7169203331` parent-id derivation +
  daemon session id stamping + file_path/file_count round-trip;
  `_stays_silent_when_scan_has_no_findings` proves the volume-
  control contract propagates through the IPC layer;
  `_does_not_emit_for_pre_write_mode` pins the ADR-031 mid-edit-only
  budget class; `_skips_emission_when_no_emitter_wired` proves
  byte-compat with legacy daemons that never wire a sink;
  `derive_gate_eval_id_prefers_traceparent_parent_id` pins the
  MLP2-008 join-key derivation + UUID v4 fallback;
  `ipc_emission_throttling_does_not_perturb_scan_responses` sends a
  5-frame burst against cap=2 and asserts the recorder cap holds
  while every scan response succeeds). The sink-error swallow path
  uses `RecordingKindlingObservationSink::fail_next_with` to inject
  a `KindlingSinkError::Unavailable`; the test confirms the next
  call still flows + scan response remains uncoupled. Rate
  primitive: shared `RateWindow` from MLP2-009 with a
  32-events-per-5-seconds default cap (configurable per emitter).
  All emission is fire-and-forget; the IPC handler discards the
  `EmissionOutcome` so scan latency stays unaffected by sink
  health. `pnpm format:check` (1349 files) clean, `pnpm lint:check`
  (clippy + fmt-check across 26 projects) clean, `pnpm typecheck`
  (26 projects) clean,
  `pnpm --filter @eddacraft/anvil-kindling-integration test`
  60 / 60 green. **Deferred (intentionally out of scope; tracked
  alongside MLP2-007 / MLP2-008):** the concrete sink wiring from
  the daemon to the TS-owned SQLite handle. The trait + emitter
  contract here is the stable seam — host startup picks a concrete
  sink (IPC bridge to the kindling-integration package, in-process
  Rust sink if a Rust SQLite client lands, etc.) without disturbing
  the scan_buffer hot path. Default `None` keeps the daemon
  behaviour identical to v1 baseline until a host opts in. The
  session-id field is currently the daemon-process UUID v4
  (placeholder for per-edit session ids landing with MLP2-023's
  composite session keys); the wire shape already matches the
  schema's `string().uuid()` contract so the migration is
  field-only.
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

- **Status:** Done
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
  (extend `KindlingObservationSink` trait with defaulted
  `try_emit_action_executed`; new `ActionExecutedObservation` /
  `ActionExecutedDetails` / `ActionDiffSummary` types matching the
  TS Zod schema; new `ActionOutcome` enum; new `PostHookAction`
  closed-set vocabulary; new `from_post_hook` builder; new
  `PostHookEmitter` + `PostHookEmissionRequest` +
  `ActionEmissionOutcome` for hook-side emission;
  `RecordingKindlingObservationSink` extended with
  `recorded_actions` / `actions_len` / `fail_next_action_with`),
  `crates/anvil-cli/src/commands/hook.rs` (`append_witness` returns
  `Result<LineHash, AppendError>` so the caller has the SHA-256 of
  the just-appended line; `run_post_commit` / `run_post_merge` /
  `run_post_rewrite` accept a `&PostHookEmitter`, time the work,
  resolve the relevant SHA — `git rev-parse HEAD` for post-commit,
  the merge ref for post-merge, the new SHA from each rewrite pair
  for post-rewrite — and call new `emit_post_hook_action` helper
  after each successful witness append; `run` mints a per-process
  UUID v4 for `session_id` and binds a `PostHookEmitter::noop` —
  concrete sink wiring is the deferred follow-up shared with
  MLP2-006 / MLP2-007).
- **Validation:** Three integration tests (one per hook).
  **Evidence (Done 2026-05-16):** `cargo test -p
  eddacraft-anvil-intercept` 335/335 green (+21 from baseline);
  `cargo test -p eddacraft-anvil` 1479/1479 green (+4 from
  baseline). New unit tests in `kindling_observation::tests`
  (10 — `from_post_hook` kind + action_type stamping, `action_id`
  shape, command field carries commit SHA + witness line hash,
  `details.working_directory` population, optional fields omitted
  on serialise via `skip_serializing_if`, emitter sink delivery,
  three-action no-short-circuit contract, sink-failure swallow +
  recovery, `NoopKindlingObservationSink` auto-satisfies the new
  trait method via the default body, `PostHookAction::as_str`
  joins back to witness-line `validation_at` tokens). New
  integration tests in `commands::hook::tests` (4 —
  `post_commit_emits_one_action_executed_row_per_invocation`
  drives `run_post_commit` with a recording emitter and asserts
  the canonical row shape end-to-end;
  `post_merge_emits_action_executed_with_merge_sha_in_command`
  drives `run_post_merge` with a synthetic merge ref and pins the
  command field; `post_rewrite_emits_one_action_executed_row_per_pair`
  drives the rewrite pipeline through two pairs and asserts the
  per-pair row count + action_id shape;
  `post_commit_with_no_project_id_is_silent_kindling_emit_too`
  pins the no-witness → no-row contract). The hook process is
  short-lived, so each invocation mints its own UUID v4
  `session_id`; the daemon-fan-out IPC bridge that would let the
  hook talk to the long-running daemon's session is the deferred
  follow-up. Wire shape: `kind = "action_executed"`,
  `action_type = "command"`, `outcome = "success"`,
  `action_id = "{post-commit|post-merge|post-rewrite}:{commit_sha}"`,
  `details.command = "anvil hook {action} (commit={sha},
  witness_line_hash={hash})"`,
  `details.working_directory = repo root`.
  `cargo clippy --workspace --all-targets -- -D warnings` clean;
  `cargo fmt --all --check` clean. **Closes Phase 3 entry for
  MLP2-010** (gated by MLP2-006 which shipped 2026-05-15).
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP-005, MLP2-006
- **Source:** MLP-005 deferred outcome.

### B. Witness chain extensions

#### MLP2-011: DAG-aware merge verification

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
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
  output; tamper tests at each parent. **Council audit addendum
  (2026-05-15):** verification must bind each `prev_line_hashes[i]`
  to the witness line whose `commit_sha` equals `parent_commits[i]`,
  and pre-push must not treat merge `parent_commits[]` as witnessed
  unless that binding is proven. Add an adversarial fixture where an
  unwitnessed side-branch parent is listed on a merge witness with no
  matching prior line; the push must still require L4 or block per
  policy.
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

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
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

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
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

- **Status:** Merged
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

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
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
  **Council audit addendum (2026-05-15):** current production
  `run_pre_push` still binds `NoOpValidationEngine`, so the typed
  pipeline is not sufficient evidence. Completion requires the
  production default path to use a real rule engine and an end-to-end
  test that does not inject a fixture engine.
  **Audit closure (2026-05-16, PR #1627 at `0aacdac8`):** new
  `crates/anvil-cli/src/l4_engine.rs::CommitAntipatternEngine`
  materialises the commit's tree via
  `git diff-tree --diff-filter=ACMR` + `git show <sha>:<path>`
  into a tempdir, runs `anvil_checks::antipattern::run_antipattern_check`,
  and maps findings to `ValidationDiagnostic`. Bound as the
  production default in `commands::hook::run_pre_push` and
  `commands::l4_validate::run` via `default_engine()`. E2e test
  `production_default_engine_blocks_known_antipattern` drives
  the default constructor (no fixture injection) against a real
  git repo carrying a `/* eslint-disable */` `.ts` file and pins
  `AP-001` in the resulting `Block { diagnostics }` with
  `exit_code == EXIT_BLOCK`. Council quick (adversarial +
  kernel + ops) ran pre-PR: 3 CRITICAL + 5 MAJOR fixed in-PR
  (empty-catalogue → `EngineUnavailable { BinaryMissing }`,
  zero-SHA guard, `--diff-filter=ACMR` delete-only intent,
  `TempDir` failure no longer `Timeout`, `tracing::warn!` on
  engine-unavailable carrying reason, captured git stderr to
  `tracing::debug!`, collapsed two-pass alloc,
  `warn_only_antipattern_admits_under_on_warn_allow` pin).
  Follow-ups (registry bundling for installed binaries,
  `git cat-file --batch` perf, `EngineUnavailableReason::IoError`)
  tracked in `plans/reviews/post-merge/feat-mlp2-016-real-engine.md`.
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

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Intent:** Env-supplied `AgentTag` must match the tag the
  daemon issued for this PID lineage at INTL-003
  registration. Mismatches treated as missing, not honoured.
- **Phasing (added 2026-05-15 mid-implementation):** the original
  six-subtask plan compressed a clean primitives layer (A1–A4) and an
  integration phase (A5: daemon control-lane wire-up) into a single
  PR. The integration phase turned out to need its own design pass
  because (i) the writer's env-tag ingress path is undesigned in the
  current daemon (`env_agent_tag()` reads the daemon's own env, not
  the writer's), (ii) the Council's "control-lane has both registries"
  premise misread the layering — `ScanBufferService` holds
  `EnforcementPipeline` but no `SessionRegistry` reference, and
  (iii) the ~20 production `register()` callers all need switching to
  `register_with_lineage` for the lineage index to be populated in
  production. Splitting honours the "each subtask reviewable in
  isolation" goal.
  - **Phase 1 (this item, In Progress → Merged this wave):** the
    primitives — proto field, `tag_env` reader, `(pid, pid_starttime)`
    lineage index, `Cross` classifier, `cross_check_env_tag` wrapper.
    All shipped behind their public API; no production call site
    consumes them yet.
  - **Phase 2 (new sub-item MLP2-025b, Draft):** daemon control-lane
    wire-up + production-caller migration + block+fence combinator +
    telemetry. Needs its own integration-phase contract spec before
    TDD — data shapes, IPC additions, function signatures per layer,
    lock/lifecycle rules. Tracked alongside the Phase 2 entry below.
- **Planning Council 2026-05-15 revisions (Phase 1 applied):** module
  placement (env-reader moved off `auth.rs`), explicit `pid_starttime`
  validation at every ancestor hop, intra-lineage trust-boundary
  note. The "cross-check at the daemon control-lane" verdict is
  carried forward to Phase 2 unchanged — Phase 1 only ships the
  primitives it needs.
- **Expected Outcome (Phase 1 — primitives, this PR):**
  - `SessionRecord` (proto) gains a wire-additive
    `daemon_issued_tag: Option<AgentTag>` mirror that
    captures the tag the daemon actually issued at
    `register()` time, distinct from the client-supplied
    `agent_tag` field.
  - `crates/anvil-intercept/src/tag_env.rs` exposes
    `env_agent_tag()` plus a pure `agent_tag_from_env`
    helper. Module is greenfield; first reader of
    `ANVIL_AGENT_TAG_ENV`.
  - `SessionRegistry` gains a `(pid, pid_starttime)` lineage
    index plus three public methods:
    - `register_with_lineage(...)` — additive register variant
      that captures the daemon-issued tag and seeds the index.
    - `lookup_tag_by_pid_starttime(pid, starttime)` — pure
      lookup used by tests.
    - `lookup_tag_for_lineage(pid)` — production wrapper that
      walks the writer's PID lineage via
      `anvil_attribution::walk_ancestors`, validates
      `(pid, pid_starttime)` at every ancestor hop, and
      returns the daemon-issued tag of any registered
      ancestor.
  - `Cross::{Untagged, Match, Spoofed}` + `Cross::classify`
    pure classifier + `SessionRegistry::cross_check_env_tag`
    production wrapper.
  - `unregister` and `evict_stale` drop matching entries
    from the lineage index so stale anchors do not linger.
  - **No production caller is wired to the new methods.** The
    primitives are dead-code from `main`'s perspective until
    Phase 2 (MLP2-025b) connects them to the daemon
    control-lane.
- **Out of scope for Phase 1 (deferred to MLP2-025b):**
  - Daemon-side env-tag ingress (how the writer's
    `ANVIL_AGENT_TAG` reaches the daemon at write time).
  - Switching production `register()` callers to
    `register_with_lineage` so the lineage index is
    populated in production.
  - Block-current-write + record-fence combinator on
    `Cross::Spoofed`.
  - `degraded:spoofed-attribution` reason string
    emission (notification envelope + `tracing::warn!`).
  - `pub const` for the reason string.
  - Trust-boundary doc on the live API.
- **Files (Phase 1):**
  - `crates/anvil-intercept-proto/src/lib.rs` — wire-additive
    `daemon_issued_tag` on `SessionRecord` + three new tests.
  - `crates/anvil-intercept/Cargo.toml` — add
    `anvil-attribution` path dep.
  - `crates/anvil-intercept/src/lib.rs` — declare `tag_env`
    module.
  - `crates/anvil-intercept/src/tag_env.rs` — greenfield
    module + four tests.
  - `crates/anvil-intercept/src/registry.rs` — `Cross` enum +
    `Cross::classify`, `by_pid_lineage` index field,
    `register_with_lineage`, `lookup_tag_by_pid_starttime`,
    `lookup_tag_for_lineage`, `cross_check_env_tag`; lineage
    drop in `unregister` and `evict_stale`; eight new tests.
  - `Cargo.lock` — internal path dep added; ACKNOWLEDGEMENTS
    regenerated (no diff — internal crate).
  - Test-helper sites populating `SessionRecord` updated to
    include `daemon_issued_tag: None`:
    `crates/anvil-intercept/src/{auth,interrupt,registry,status}.rs`
    + `crates/anvil-cli/src/commands/intercept.rs`.
- **Validation (Phase 1):**
  - `cargo test -p eddacraft-anvil-intercept-proto --lib`
    (38 tests, up from 35)
  - `cargo test -p eddacraft-anvil-intercept --lib`
    (312 tests, up from 305)
  - `cargo test -p eddacraft-anvil --bins commands::intercept`
    (7 tests, unchanged)
- **Subtasks (Phase 1, all complete):**
  1. Proto wire-additive `daemon_issued_tag` — DONE.
  2. `tag_env` greenfield module — DONE.
  3. Registry lineage lookup with `pid_starttime` pinning — DONE.
  4. Registry `cross_check_env_tag` three-state API — DONE.
- **Why split (added 2026-05-15):** the original
  six-subtask plan compressed primitives and integration
  into one PR. Mid-implementation discovery surfaced three
  open questions for the integration phase: writer-side
  env-tag ingress is undesigned, the Council layering
  verdict was based on a misread of which struct holds the
  registry reference, and ~20 production `register()`
  callers need migrating. None of these block the
  primitives — splitting keeps the review surface clean.
- **Confidence:** high (Phase 1 surface fully covered by
  unit tests; the deferred wire-up is what the integration
  phase will own).
- **Priority:** Critical
- **Dependencies:** MLP-014, MLP2-023
- **Source:** MLP-014 footnote 6.

#### MLP2-025b: Daemon control-lane wire-up for spoof cross-check

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Contract spec:** `plans/specs/2026-05-16-mlp2-025-spoof-cross-check-control-lane.md` (Accepted via PR #1599). All four blockers and six open questions resolved in the spec; implementation follows §3–§9 verbatim.
- **Intent:** Wire the MLP2-025 Phase 1 primitives into the
  daemon's live write-time decision path so the spoof
  cross-check actually fires on real writes. Without this
  item, the Phase 1 primitives are dead code from `main`'s
  perspective.
- **Open design questions (must be resolved before TDD):**
  1. **Writer-side env-tag ingress.** `env_agent_tag()` reads
     the daemon's own env, not the writer's. Options:
     (a) the writer (anvil-run launcher or driver) packages
     its `ANVIL_AGENT_TAG` into the IPC scan-buffer request
     as a new wire-additive field; (b) the daemon reads
     `/proc/<writer_pid>/environ` directly on Linux; (c)
     hybrid — request field with daemon fallback to /proc.
     Pick one. Each option has different security and
     portability properties.
  2. **Write-time decision call site.** The Council
     2026-05-15 verdict said "do the cross-check at the
     control-lane caller that has both registries". On
     re-read of the code,
     `crates/anvil-intercept/src/lib.rs` `RegistryDispatcher`
     holds `Arc<SessionRegistry>` but
     `crates/anvil-intercept/src/midedit.rs`
     `ScanBufferService` (the write-time entry point) holds
     `EnforcementPipeline` and *not* the registry. Decide:
     thread a registry reference into `ScanBufferService`,
     or hoist the cross-check into the IPC handler that
     calls `ScanBufferService`?
  3. **Block + fence combinator.** Today
     `RegistryError::WorktreeFenced` is a register-time
     error. There is no equivalent write-time "block this
     write AND record a fence" outcome. Either extend
     `EnforcementDecision` with a `Spoofed` variant, or have
     the control-lane caller short-circuit the pipeline
     entirely on `Cross::Spoofed`.
  4. **Production caller migration.** ~20 sites call
     `register()` today. Decide which need switching to
     `register_with_lineage` (only the daemon's true
     registration call site, or also tests / embedded
     pathways?). A migration that's too aggressive risks
     breaking embedded mode; too conservative leaves the
     lineage index empty in production.
- **Required artefact before TDD:** a contract-style design
  spec under `plans/specs/` defining (i) data shapes
  (structs touched, new IPC fields), (ii) message flow
  diagram (writer → daemon → registry → pipeline → response),
  (iii) function signatures at each layer, (iv)
  lock/lifecycle/error-channel rules. Web-API-style data
  contract, not free-form prose.
- **Council:** `mini` review on the spec before
  implementation; `quick` per impl subtask thereafter.
- **Expected Outcome (sketch, refined by the contract
  spec):** daemon's write-time IPC handler invokes
  `SessionRegistry::cross_check_env_tag(env_tag, writer_pid)`
  before invoking `EnforcementPipeline`; on `Cross::Spoofed`
  the handler blocks the current write and records a
  worktree-level fence with reason
  `degraded:spoofed-attribution`; emission via both
  notification envelope and `tracing::warn!`. Reason
  strings as `pub const`.
- **Confidence:** low (open questions above)
- **Priority:** Critical (MLP2-025 is a security surface;
  Phase 1 alone is incomplete)
- **Dependencies:** MLP2-025 (Phase 1 primitives), MLP-014,
  MLP2-023
- **Source:** MLP2-025 mid-implementation discovery,
  2026-05-15 — the integration phase needed its own design
  pass.
- **Post-merge addendum (2026-05-19, PR #1717):** the
  2026-05-18 umbrella closure was premature on the production
  wire-up axis. `IpcListener::with_cross_check_context` had
  zero call sites: `run_foreground` built the listener with
  `cross_check: None`, and the scan-buffer handler silently
  skipped the cross-check on every request. A second upstream
  gap — `validate_scan_buffer_request_shape` and
  `validate_oversized_scan_buffer_params` did not allow
  `env_agent_tag` in their params allowlists — would have
  rejected tagged requests at the schema gate before the
  cross-check could read them. DeepSec finding #1671 (filed
  2026-05-17) surfaced the wire-up gap during the 2026-05-19
  release-readiness triage. PR #1717 closes both gaps and
  pins them with an integration test (`tests/spoof_cross_check_wired.rs`)
  that goes through the real Unix socket via `run_foreground`.
  The wire-up is **Linux-only** by `cfg(target_os = "linux")`
  — `pid_starttime` / `parent_pid` are Linux-only today
  (deferred to MLP2-027 for macOS) and Windows accept passes
  `peer_pid: None` (deferred to MLP2-028). Wiring on non-Linux
  would classify every env-tagged write as `Cross::Spoofed`
  and block legitimate sessions; the cfg gate widens
  automatically when MLP2-027 / MLP2-028 land.

#### MLP2-025c: Launcher-side population of `lineage` + `env_agent_tag`

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Closure (2026-05-17, PR #1608 at `1ea23349`):** the launcher
  migration shipped per the locked spec. `session_register_params`
  (`crates/anvil-run/src/ipc.rs`) now emits nested `agent_tag`
  (the shape the daemon's MLP2-023+ parser has been waiting for —
  flat fields were silently dropped on every production session
  since MLP2-023 shipped) and nested `lineage` (the MLP2-025b
  anchor). `RegistrationRequest` gained `launcher_pid: u32`;
  `run.rs` populates from `std::process::id()`. TS driver-client
  `AnvilScanBufferParams` gained optional `env_agent_tag?: string`
  + `AnvilScanBufferResult.spoof_block?`; `validateMidEdit` reads
  `process.env.ANVIL_AGENT_TAG` and forwards it (empty string and
  undefined both fold to "omit" — daemon classifies absence as
  `Cross::Untagged`). Three new wire-shape pins in `ipc::tests`
  (incl. spec §7 trust-model invariant pin `session_register_
  params_lineage_pid_starttime_matches_agent_tag`); three new
  driver-client tests pin env_agent_tag set/unset/empty-string.
  Behaviour change after merge: every `anvil-run` launcher emits
  a proper `agent_tag`+`lineage` on register, every TS-driver
  mid-edit scan-buffer carries the writer's `ANVIL_AGENT_TAG`,
  and the daemon's spoof cross-check becomes active —
  `Cross::Match` admits, `Cross::Spoofed` blocks + fences with
  `degraded:spoofed-attribution`, `Cross::Untagged` preserves the
  pre-MLP2-025 path. Flat `driver_id`/`claimed_agent_id`/
  `pid_starttime`/`cwd`/`tmux_pane` stayed on the wire alongside
  the new nested objects (deferred cleanup; the daemon doesn't
  read them, they harm nothing). Out of scope and tracked
  separately: Windows peer-PID (MLP2-028), end-to-end spoof
  confirmation test against a real daemon+launcher+spoofer triple.
- **Intent:** Populate the wire fields MLP2-025b added so the
  daemon's lineage index actually gets seeded in production and
  the write-time spoof cross-check has something to match. Until
  this landed, every production register call went through the
  legacy path with `lineage = None`; the cross-check was wired
  daemon-side but inert.
- **Mid-implementation survey** (2026-05-16, no contract spec):
  the migration is a clean small surface — one Rust function
  (`session_register_params`) + one struct (`RegistrationRequest`)
  + one TS interface (`AnvilScanBufferParams`) + one TS call site
  (`validate-mid-edit.ts`). Bonus: the current Rust launcher
  sends flat `driver_id` / `claimed_agent_id` / `pid_starttime`
  fields the daemon **ignores** (since MLP2-023's daemon parser
  expects a nested `agent_tag`). Activating MLP2-023's composite
  identity in production is part of this PR's scope — without it
  the daemon never sees a tag on any registered session.
- **Expected Outcome:**
  - `crates/anvil-run/src/ipc.rs` `session_register_params` emits
    nested `agent_tag` and `lineage` objects matching the daemon's
    parser (`crates/anvil-intercept/src/ipc.rs:2309`). Flat
    `driver_id`/`claimed_agent_id`/`pid_starttime`/`cwd`/`tmux_pane`
    fields are dropped (the daemon never read them).
  - `RegistrationRequest` gains `launcher_pid: u32` field;
    callers populate via `std::process::id()`. The existing
    `pid_starttime` field is reused for the lineage anchor's
    `pid_starttime` value.
  - `packages/anvil-driver-client/src/protocol/types.ts`
    `AnvilScanBufferParams` gains `env_agent_tag?: string`.
  - `packages/anvil-driver-client/src/midedit/validate-mid-edit.ts`
    request-builder populates `env_agent_tag` from
    `process.env.ANVIL_AGENT_TAG`.
  - All four MLP2-025b primitives become active in production:
    on every legitimate write, the daemon's
    `cross_check_env_tag(env_tag, writer_pid)` runs against the
    populated lineage index and returns `Cross::Match`. Spoofed
    writes (env tag from out-of-lineage process) return
    `Cross::Spoofed` and trigger the fence + block.
- **Files:**
  - `crates/anvil-run/src/ipc.rs` — `session_register_params`
    signature change + wire shape rewrite + updated tests.
  - `crates/anvil-run/src/session.rs` — `RegistrationRequest`
    gains `launcher_pid`; `register()` threads it into the params
    call.
  - `crates/anvil-run/src/spawn.rs` (or wherever the request is
    built) — populate `launcher_pid: std::process::id()`.
  - `packages/anvil-driver-client/src/protocol/types.ts` — add
    `env_agent_tag?: string` to `AnvilScanBufferParams`.
  - `packages/anvil-driver-client/src/midedit/validate-mid-edit.ts`
    — read `process.env.ANVIL_AGENT_TAG` and include in scan-buffer
    request params.
- **Validation:**
  - `cargo test -p eddacraft-anvil-run --lib ipc::tests` —
    existing fixture tests assert the new wire shape (nested
    `agent_tag` + `lineage`).
  - `cargo test --workspace` clean.
  - New TS unit test in `packages/anvil-driver-client/` pinning
    that `env_agent_tag` is included when `ANVIL_AGENT_TAG` is set
    and omitted when it isn't.
- **Trust model:** the launcher's register-time claim about its
  own `(pid, pid_starttime)` is **trusted** (§7 of the
  MLP2-025b spec). The launcher is in the daemon's trust zone;
  if it lies about itself, the operator has bigger problems.
- **Out of scope:** Windows launcher (peer-PID greenfield for
  Windows is MLP2-028); behavioural confirmation tests that
  exercise the full daemon + launcher path with a real spoof
  (that's a separate integration test sub-item).
- **Confidence:** high (no design unknowns; spec contract is
  locked).
- **Priority:** Critical (MLP2-025/-025b are dead code in
  production without this).
- **Dependencies:** MLP2-025, MLP2-025b
- **releaseNote:**
  - audience: user
  - type: security
  - text: "Anvil now performs end-to-end agent-tag spoof
    rejection. The launcher and TypeScript driver-client
    forward each writer's `ANVIL_AGENT_TAG` and PID lineage
    to the daemon, which cross-checks them against the tag
    it issued at registration. Spoofed tags block the
    offending write and fence the worktree with
    `degraded:spoofed-attribution`."
- **Source:** MLP2-025b spec Q6 verdict (2026-05-16); mid-impl
  discovery 2026-05-16 that the current launcher's flat
  register-session wire shape is daemon-ignored.

#### MLP2-026: `degraded:fence-cascade` mode at 5 fences in 60s

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Closure (2026-05-17, PR #1624 at `5e3798da`):** seven impl
  commits (F1–F7) on `feat/mlp2-026-fence-cascade` shipped the
  spec verbatim: `CascadeRecord` + wire-additive
  `FenceFile.cascades` (F1), `fence_worktree()` engage path +
  `DEGRADED_FENCE_CASCADE` / `DEGRADED_FENCE_CASCADE_CLEAR`
  consts + engage notification + `tracing::warn!` (F2/F7),
  `WorktreeStatus`/`WorktreeStatusV1` `cascaded` +
  `cascade_since` fields with serde-default wire compat (F3),
  `RegistryError::WorktreeCascaded` + register-time refusal
  under cascade-before-registry lock ordering (F4),
  `IpcCommand::UnblockCascade { worktree, operator }` with
  daemon-derived `OperatorContext { uid, pid, hostname }` (F5),
  and `anvil intercept unblock --acknowledge-cascade <worktree>`
  CLI subcommand (F6). All five spec invariants pinned:
  `is_cascaded` reflects on-disk `CascadeRecord` (inv-1),
  cascade-before-registry ordering (inv-2), `clear_cascade`
  idempotency (inv-3), engage-flag survives daemon restart
  (inv-4). Test suite: `cargo test -p eddacraft-anvil-intercept
  --lib` green with new `fence::tests::*` (engage threshold,
  round-trip, persists-until-acknowledged), `registry::tests::
  register_on_cascaded_worktree_is_refused`, `ipc::tests::
  unblock_cascade_round_trips_with_operator_context`, and
  `commands::intercept::tests::
  unblock_acknowledge_cascade_dispatches_ipc` on the CLI side.
- **Contract spec:** `plans/specs/2026-05-16-mlp2-026-fence-cascade-control-lane.md` (Accepted via PR #1617). Five resolved open questions; no `BLOCKING` remains; implementation follows §3–§9 verbatim.
- **Intent:** When five fences fire within 60s, the daemon
  enters `degraded:fence-cascade` mode requiring operator-
  clear. Uses the shared rate-window primitive from MLP2-009.
- **Planning Council 2026-05-15 revisions:** `RateWindow`
  capacity corrected to 4 (off-by-one), cascade engaged-state
  persisted in `FenceFile` (survives daemon restart), status
  surface gains `cascaded` / `cascade_since`, `tracing::warn!`
  alongside notification, `operator` audit field on the IPC
  verb, lock ordering documented, telemetry subtask folded
  into engage/clear sites. See the
  Planning Council 2026-05-15 synthesis in the PR description for the
  planning artefacts (the review record itself lives outside the tracked
  tree per the `plans/reviews/*` gitignore rule).
- **Expected Outcome:**
  - `FenceStore` holds an in-memory per-worktree
    `RateWindow::new(4, Duration::from_secs(60))`. The 5th
    `record()` call within 60 s returns `RateDecision::Throttle`
    — that is the engage trigger. (Capacity 4, not 5: the
    rate-window admits up to capacity, so capacity must be
    one less than the desired threshold count.)
  - `fence_worktree()` records each firing through the rate
    window. On `Throttle`, the engaged-state record is
    written and persisted.
  - **Cascade engaged-state is persisted in `FenceFile`** as
    a new wire-additive field `cascades:
    Vec<CascadeRecord>` where each record is
    `{ worktree: PathBuf, since_unix: u64, reason: String }`.
    Wire-additive via `#[serde(default,
    skip_serializing_if = "Vec::is_empty")]`, `version` stays
    at 1. Daemon restart restores engaged cascades from disk;
    the in-memory `RateWindow` rebuilds empty on restart but
    the engaged flag survives (correct behaviour: cascade is
    a security boundary, must not silently clear on
    process restart).
  - `FenceStore::is_cascaded(&path) -> bool` and
    `clear_cascade(&path)` accessors expose the engaged
    state.
  - `SessionRegistry::register()` consults
    `FenceStore::is_cascaded(&path)` and refuses new sessions
    on a cascaded worktree with
    `RegistryError::WorktreeCascaded { worktree }`,
    mirroring the `SessionCapExceeded` precedent from
    MLP2-024.
  - **Lock ordering, documented:** `FenceStore` lock is
    acquired BEFORE `SessionRegistry::Inner` lock at every
    call site. The cascade check in `register()` snapshots
    the cascade flag, releases the fence lock, then takes
    the registry lock. Comments at both lock sites cite this
    rule.
  - `IpcCommand::UnblockCascade { worktree, operator:
    Option<OperatorContext> }` is the wire-additive verb.
    `OperatorContext { uid: Option<u32>, pid: u32, hostname:
    Option<String> }` is populated daemon-side from IPC peer
    credentials. The daemon handler calls `clear_cascade()`
    and resets the rate window for that worktree.
  - CLI: greenfield `anvil intercept unblock
    --acknowledge-cascade <worktree>` subcommand
    (`crates/anvil-cli/src/commands/intercept.rs`). Path is
    canonicalised before the IPC dispatch (matching
    `unblock_worktree`'s `lookup_path` guard in
    `fence.rs:192–198`). `--acknowledge-cascade` remains as
    UX clarity; the audit-of-record is the `operator` field
    on the wire, not the flag.
  - **Status surface update:** `WorktreeStatus` and
    `WorktreeStatusV1` gain `cascaded: bool` and
    `cascade_since: Option<u64>`. `render_status` adds a
    `cascade: engaged since <ts>` line when applicable.
    Operators can discover cascade state without invoking
    a doomed `register`.
  - **Telemetry in two channels:**
    - Notification envelope (existing `telemetry.rs`
      convention around lines 339–354) — `degraded:fence-cascade`
      on engage and a paired clear notification.
    - `tracing::warn!(target: "anvil_intercept::fence",
      reason = "degraded:fence-cascade", %worktree,
      since_unix, ...)` at engage; `tracing::info!(target:
      "anvil_intercept::fence", reason =
      "degraded:fence-cascade-clear", %worktree, ?operator,
      ...)` at clear. Mirrors the priority asymmetry from
      `FenceTransition::ActiveToFenced` (warn) vs
      `FencedToActive` (info).
    - Both `degraded:fence-cascade` and
      `degraded:fence-cascade-clear` are `pub const` string
      literals.
  - Per-task fence isolation from MLP2-023 is preserved:
    cascade is keyed on the worktree path, not on
    `(WorktreeKey, AgentTag)`. Per-task escalation is
    deferred (would require a `fence_worktree` signature
    change).
- **Files:**
  - `crates/anvil-intercept/src/fence.rs` — add
    `CascadeRecord` (serde-additive on `FenceFile`); in-memory
    per-worktree `RateWindow::new(4, 60s)`; fire-count
    inside `fence_worktree()`; `is_cascaded()` / `clear_cascade()`
    accessors; engage-on-throttle wired through; tests
    extend the existing `tests` module (precedent
    `explicit_unblock_removes_persisted_fence` at lines
    527–626+).
  - `crates/anvil-intercept/src/registry.rs` — consult
    cascade on `register()`; add the `WorktreeCascaded` error
    variant alongside `SessionCapExceeded`; document the
    lock-ordering rule at the lock site.
  - `crates/anvil-intercept/src/status.rs` — add `cascaded:
    bool` and `cascade_since: Option<u64>` to
    `WorktreeStatus` and `WorktreeStatusV1`; render the new
    `cascade:` line in `render_status`.
  - `crates/anvil-intercept/src/ipc.rs` — daemon-side
    `IpcCommand::UnblockCascade` handler; populate
    `OperatorContext` from peer credentials.
  - `crates/anvil-intercept-proto/src/lib.rs` (or wherever
    `IpcCommand` is declared) — wire-additive
    `UnblockCascade { worktree, operator:
    Option<OperatorContext> }` variant +
    `OperatorContext` type.
  - `crates/anvil-cli/src/commands/intercept.rs` — greenfield
    `Unblock { worktree: PathBuf, acknowledge_cascade: bool }`
    subcommand; canonicalise path before dispatch.
  - `crates/anvil-intercept/src/telemetry.rs` — emit
    notification + `tracing::warn!` on engage; notification +
    `tracing::info!` on clear.
- **Validation:**
  - `cargo test -p eddacraft-anvil-intercept --lib
    fence::tests::five_fences_in_sixty_seconds_engage_cascade`
  - `cargo test -p eddacraft-anvil-intercept --lib
    fence::tests::four_fences_in_sixty_seconds_do_not_engage_cascade`
  - `cargo test -p eddacraft-anvil-intercept --lib
    fence::tests::cascade_state_persists_until_acknowledged`
  - `cargo test -p eddacraft-anvil-intercept --lib
    fence::tests::cascade_state_round_trips_through_store_reload`
  - `cargo test -p eddacraft-anvil-intercept --lib
    fence::tests::acknowledge_cascade_resets_rate_window`
  - `cargo test -p eddacraft-anvil-intercept --lib
    registry::tests::register_on_cascaded_worktree_is_refused`
  - `cargo test -p eddacraft-anvil-intercept --lib
    status::tests::cascaded_worktree_surfaces_in_status_json`
  - `cargo test -p eddacraft-anvil-intercept --lib
    ipc::tests::unblock_cascade_round_trips_with_operator_context`
  - `cargo test -p eddacraft-anvil-cli --lib
    commands::intercept::tests::unblock_acknowledge_cascade_dispatches_ipc`
- **Subtasks:**
  1. **`CascadeRecord` persistence + `RateWindow::new(4, 60s)`
     on `FenceStore`.** Wire-additive `CascadeRecord` on
     `FenceFile`; in-memory per-worktree rate window with
     **capacity 4** so the 5th fire returns `Throttle`;
     engage on `Throttle`; persist `cascades` to disk; emit
     `degraded:fence-cascade` via notification AND
     `tracing::warn!` at the engage site. Tests:
     `four_fences_in_sixty_seconds_do_not_engage_cascade`,
     `five_fences_in_sixty_seconds_engage_cascade`,
     `cascade_state_round_trips_through_store_reload`.
  2. **Fire-count integration in `fence_worktree()`.** Each
     fire records through the window; engaged state
     persists across subsequent fires; test
     `cascade_state_persists_until_acknowledged`.
  3. **Status surface update.** Add `cascaded` and
     `cascade_since` to `WorktreeStatus` /
     `WorktreeStatusV1`; render the `cascade:` line; test
     `status::tests::cascaded_worktree_surfaces_in_status_json`.
  4. **Registry refusal on cascaded worktrees.** Add
     `RegistryError::WorktreeCascaded`; cascade check in
     `register()`; lock-ordering comment at both call
     sites; test
     `register_on_cascaded_worktree_is_refused`.
  5. **IPC `UnblockCascade` verb with `OperatorContext`.**
     Wire-additive variant + `OperatorContext` type;
     daemon-side handler populates the context from peer
     credentials, calls `clear_cascade()`, resets the rate
     window, emits the clear notification + `tracing::info!`.
     Tests: `unblock_cascade_round_trips_with_operator_context`,
     `acknowledge_cascade_resets_rate_window`.
  6. **CLI `intercept unblock --acknowledge-cascade`.**
     Greenfield subcommand; canonicalise path before
     dispatch; test
     `commands::intercept::tests::unblock_acknowledge_cascade_dispatches_ipc`.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP2-009, MLP2-023
- **releaseNote:**
  - audience: operator
  - type: added
  - text: "Anvil now engages a `degraded:fence-cascade` mode
    when five fences fire on the same worktree within sixty
    seconds, refusing new sessions until an operator
    acknowledges. Run
    `anvil intercept unblock --acknowledge-cascade <worktree>`
    to clear; `anvil status` surfaces `cascaded` /
    `cascade_since`, and the engaged state survives daemon
    restart."
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
- **releaseNote:**
  - audience: user
  - type: changed
  - text: "`anvil baseline` now mints `anvil/project-id` on
    first run (preserved on re-run) and pins `cutoff_commit`
    into the canonical policy file in the same flow, so
    adopting Anvil into an existing repo no longer fails on
    a missing project identity."
- **Source:** MLP-001 footnote 3, MLP-007 footnote 5.

#### MLP2-033: `--new-identity` fork opt-out CLI flag

- **Status:** Merged
- **Intent:** `anvil start --new-identity` mints a fresh
  `project_uuid` instead of inheriting from the parent repo
  (which is the current fork behaviour). Lives on `anvil
  start` and `anvil baseline`.
- **Expected Outcome:**
  - `--new-identity` clears any existing
    `forked_from`-inherited UUID and writes a fresh v7 UUID.
  - Default behaviour (without the flag) preserves the
    current "fork inherits" semantics.
- **Files:** `crates/anvil-cli/src/activation/identity.rs`
  (new `mint_new_identity` primitive),
  `crates/anvil-cli/src/commands/baseline.rs`,
  `crates/anvil-cli/src/commands/start.rs`.
- **Evidence (Merged 2026-05-15 via PR #1580 at `9c0537ea`):**
  New `mint_new_identity(root, version) -> ProjectIdentity` in
  `activation/identity.rs` mints a fresh v7 UUID and records the
  previous `project_uuid` (if any) as `forked_from`. Always
  writes — explicit operator intent. Mirrors `ensure_project_id`'s
  TOCTOU + symlink-refusal pattern, plus an extra
  `refuse_if_symlink(&path)` since the overwrite would otherwise
  follow a symlink-to-file out of the repo. Atomic temp-then-rename
  uses `std::fs::rename`'s replace-existing semantics on POSIX
  (default) and Windows (since Rust 1.66). Re-reads after rename
  for the council-C-2 convergence pattern. `--new-identity` flag
  on `BaselineArgs` and `StartArgs` dispatches to the new
  primitive; `baseline.rs` bypasses the "baseline already
  exists" short-circuit when the flag is set so `baseline.json`'s
  `metadata.project_uuid` cannot diverge from the freshly minted
  identity (mirrors the divergence trap MLP2-032 closed for
  cutoff). `start.rs` pre-mints before the orchestrator runs;
  mutually exclusive with `--verify`/`--json` (read-only). Mint
  failure in `start.rs` is non-fatal — surfaces a `tracing::warn!`
  + one-line eprintln and lets the orchestrator's idempotent
  `ensure_project_id` step pick up whatever state was left on
  disk (matches the orchestrator's existing identity-failure
  posture). +9 unit pins:
  `mint_new_identity_on_empty_repo_acts_like_fresh`,
  `mint_new_identity_records_existing_uuid_as_forked_from`,
  `mint_new_identity_is_not_idempotent_each_call_remints`,
  `mint_new_identity_treats_malformed_existing_as_no_parent`,
  `mint_new_identity_refuses_when_anvil_is_a_symlink` (unix-only),
  `new_identity_remints_uuid_and_records_forked_from`,
  `new_identity_bypasses_already_exists_short_circuit`,
  `new_identity_on_empty_repo_mints_with_no_parent`,
  `new_identity_preserves_existing_cutoff_commit` (Council
  quick #C-4 regression guard). `cargo test --workspace` clean
  (4116 tests); `cargo clippy --workspace --all-targets -- -D
  warnings` clean. Council quick on PR #1580 found 4 MINOR + 2
  NIT — no MAJOR/CRITICAL. Folded in: race-window doc comment
  on `parent_uuid` capture, `tracing::warn!` on temp-file
  cleanup failure (`#C-2`), symlink-asymmetry doc, cutoff-carry
  regression test (`#C-4`). Skipped: `start.rs` `bail!` test
  (codebase pattern — `--watch + --verify` similarly untested).
- **Validation:** Fork tree fixture: parent uuid A → child A
  (no flag) → grandchild B (with flag).
- **Confidence:** high
- **Priority:** Low
- **Dependencies:** MLP-001, MLP-007, MLP2-032
- **releaseNote:**
  - audience: user
  - type: added
  - text: "`anvil start --new-identity` and
    `anvil baseline --new-identity` mint a fresh
    `project_uuid` and record the previous one as
    `forked_from`, giving forks an explicit opt-out from
    inheriting their parent repo's identity."
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

- **Status:** Merged (Phase 1 only; Phase 2 deferred)
- **Intent:** Detect baseline refreshes that look like
  adversarial whitewashing (huge violation drop without a
  corresponding code-size reduction) and surface as
  `degraded:baseline-suspicious`.
- **Expected Outcome:**
  - **Phase 1 (this PR):** library heuristic
    `analyze_refresh(old, new, thresholds) -> RefreshSuspicion`
    in `anvil-baseline::diff` flags refreshes that remove
    ≥`removed_ratio_threshold` × `old_total` findings AND
    ≥`minimum_removed` absolute findings. CLI surface: `anvil
    baseline --refresh` calls it before saving and refuses to
    overwrite `baseline.json` until the operator re-runs with
    `--accept-suspicious` (explicit acknowledgement). Two
    threshold knobs configurable via `--suspicion-ratio` and
    `--suspicion-min-removed`.
  - **Phase 2 (follow-up):**
    (a) `crates/anvil-intercept/src/fence.rs` degraded-mode
    wiring so the daemon picks up `degraded:baseline-suspicious`
    on attach when the on-disk baseline carries a recently
    suspicious provenance marker;
    (b) git-driven code-churn correlation (the spec's "without
    a corresponding code-size reduction" axis) — needs either a
    commit SHA or scanned LoC field added to baseline metadata;
    (c) `baseline.suspicion.{ratio,minimum_removed}` policy-file
    section so per-repo defaults don't require re-typing CLI
    flags every refresh.
- **Files (Phase 1):** `crates/anvil-baseline/src/diff.rs`
  (extend), `crates/anvil-baseline/src/lib.rs` (re-export),
  `crates/anvil-cli/src/commands/baseline.rs` (CLI flag wiring +
  pre-save analysis gate).
- **Evidence (Phase 1 Merged 2026-05-15 via PR #1582 at `c51e824e`):**
  New `analyze_refresh(old: &[BaselineFinding], new:
  &[BaselineFinding], thresholds: &SuspicionThresholds) ->
  RefreshSuspicion` is a pure decision over two finding sets —
  no I/O. Set membership keyed on the same
  `(rule_id, file_path, fingerprint)` triple
  `BaselineDiff::diff` already uses to partition. Defaults:
  ratio = 0.75, minimum_removed = 10 (rejects firing on tiny
  baselines where 100% drop is statistically meaningless).
  `DEGRADED_REASON = "degraded:baseline-suspicious"` constant
  exposed via `REFRESH_DEGRADED_REASON` re-export to avoid
  collision with `identity::AttachStatus::DEGRADED_REASON`.
  CLI flow: `analyze_refresh` runs BEFORE `save_baseline` so a
  suspicious refresh refuses to overwrite — operator must
  explicitly `--accept-suspicious` (not a hard error;
  warnings-over-blocks via `Ok(())` return + informative
  message). +13 unit pins (9 in diff.rs, 4 in baseline.rs):
  `degraded_reason_constant_is_pinned`,
  `default_thresholds_match_documented_values`,
  `analyze_refresh_clean_when_old_is_empty`,
  `analyze_refresh_clean_when_no_removals`,
  `analyze_refresh_clean_when_drop_below_minimum_removed`,
  `analyze_refresh_clean_when_drop_below_ratio_threshold`,
  `analyze_refresh_suspicious_when_both_gates_crossed`,
  `analyze_refresh_honours_overridden_thresholds`,
  `analyze_refresh_set_membership_uses_full_triple`,
  `refresh_refuses_to_save_when_suspicious_without_ack`,
  `refresh_proceeds_when_suspicious_with_ack_flag`,
  `refresh_threshold_override_above_one_disables_detection`,
  `refresh_at_exactly_1_0_threshold_still_fires` (Council #C-1
  + #C-5 boundary pin),
  `refresh_under_minimum_removed_is_clean`. `cargo test
  --workspace` clean (4131 tests); `cargo clippy --workspace
  --all-targets -- -D warnings` clean. Council quick on PR
  #1582 found 2 MAJOR + 3 MINOR + 1 NIT — both MAJORs
  (doc/code mismatch on the ratio off-switch + write-then-warn
  ordering defeating the heuristic) folded into the same
  branch with a regression test for each. Threshold knobs are
  CLI-only for v1 — policy-file `baseline.suspicion.*` is
  Phase 2.
- **Validation (Phase 1):** Seed an existing baseline with N
  synthesised findings, refresh against an empty worktree
  (100% drop), assert refusal-without-ack + acceptance-with-ack
  + boundary at exactly 1.0 threshold + tiny-baseline veto.
- **Validation (Phase 2 — pending):** End-to-end fence-state
  read showing `degraded:baseline-suspicious` survives daemon
  attach; code-churn signal joined against the drop ratio.
- **Confidence:** low (needs threshold tuning — see Phase 2)
- **Priority:** Low
- **Dependencies:** MLP-007, MLP2-034
- **Source:** MLP-007 footnote 6.

#### MLP2-036: Async continuation for >100k file baselines

- **Status:** Merged (Phase 1 only; Phase 2 deferred)
- **Intent:** `anvil baseline` currently scans synchronously.
  Add async continuation + a "partial baseline" marker so
  huge monorepos don't time out during adoption.
- **Expected Outcome:**
  - **Phase 1 (this PR):** schema additions
    `partial: bool` + `continuation: Option<String>` on
    `Baseline`; `Baseline::merge_partial_findings` helper
    (dedupe-aware union); CLI flag `--scan-budget <N>`
    (default 50_000) on `anvil baseline`; partial state on
    disk auto-resumes on plain `anvil baseline`. Suspicion
    detection + cutoff pin both skipped while baseline is
    partial.
  - **Phase 2 (follow-up):** time-based budget option
    (`--scan-budget-secs`); 100k synthetic-file fixture for
    documented performance budget (spec said "TBD; profile
    first"); `anvil status --json` rendering of `partial=true`
    as a degraded surface; caller-friendly
    `Baseline::commit_partial(...)` /
    `Baseline::commit_complete(...)` builder so the
    asymmetric-responsibility split on
    `merge_partial_findings` can't be misused.
- **Files (Phase 1):** `crates/anvil-baseline/src/store.rs`
  (schema + validate + `merge_partial_findings`),
  `crates/anvil-cli/src/commands/baseline.rs`
  (`--scan-budget` flag, budget-aware scanner, resume
  orchestration).
- **Evidence (Phase 1 Merged 2026-05-15 via PR #1584 at `0220b302`):**
  Schema fields are `serde(default,
  skip_serializing_if = ...)` so a complete baseline
  serialises byte-identically to pre-MLP2-036 (older anvil
  reads unaffected). `validate()` refuses
  `(partial=true, continuation=None)` and
  `(partial=false, continuation=Some)` so half-edited
  baselines fail at the load boundary.
  `scan_repo_for_findings_with_budget(repo_root, budget,
  resume_cursor)` returns `(Vec<Finding>, Option<String>)`.
  Files sorted by repo-relative path with forward-slash
  normalisation so cursors are portable across OSes; non-UTF8
  paths dropped (Council #C-4) so `to_string_lossy`'s U+FFFD
  substitution can't corrupt cursor compares. Asserts
  `budget > 0` (Council #C-2); `parse_scan_budget` clap value
  parser rejects `--scan-budget 0` at the CLI boundary.
  Orchestrator: partial-on-disk auto-resumes; resume
  accumulator carries prior findings + merges. `--new-identity`
  forces a fresh accumulator (Council #C-1) so prior-identity
  findings don't leak into the new identity's baseline.
  `--refresh` of a complete baseline that would produce a
  partial result refuses without `--accept-suspicious` (Council
  #C-3) — the complete → partial transition is an explicit
  whitewash vector. Cutoff pin skipped while partial. +12
  unit pins (6 in store.rs, 6 in baseline.rs) plus 5 Council
  regression pins. `cargo test --workspace` clean (4148 tests;
  +17 net); `cargo clippy --workspace --all-targets -- -D
  warnings` clean. Council quick on PR #1584 found 3 MAJOR + 3
  MINOR + 1 NIT — all 3 MAJORs folded with regression tests;
  #C-4 folded; remaining minors deferred to Phase 2.
- **Validation (Phase 1):**
  `one_shot_scan_matches_resumed_scan_byte_for_byte` — same
  fixture scanned in one shot vs three chunks of 3 produces
  byte-identical findings (the spec's "full + resumed flow
  produces same final baseline" pin).
  `resume_continues_from_cursor_and_can_complete` — three-round
  budget=4/3/5 sequence over 10 files, ending with
  partial=false.
- **Validation (Phase 2 — pending):** 100k synthetic-file
  performance fixture documenting per-budget wall-clock
  budget; `anvil status --json` round-trip of
  `partial=true`.
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
- **releaseNote:**
  - audience: user
  - type: added
  - text: "`anvil start --format json|toml` lets you choose
    `.anvil.json` or `.anvil.toml` at adoption time. The
    default remains yaml, and all three formats round-trip
    through the same canonical representation."
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
- **releaseNote:**
  - audience: user
  - type: changed
  - text: "Anvil now discovers `.anvil.yaml`, `.anvil.yml`,
    `.anvil.json`, and `.anvil.toml` first, falling back to
    legacy `.anvilrc` only when none are present. Run
    `anvil migrate` to convert an existing `.anvilrc` to the
    new filename."
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
- **releaseNote:**
  - audience: user
  - type: added
  - text: "`anvil l4-validate` is now a dedicated CLI
    subcommand for running L4 verification over a commit
    range, replacing the previous `anvil hook pre-push`
    reuse for CI and GitHub Action consumers."
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

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
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
  spec. **Council audit addendum (2026-05-15):** current
  `status.rs::print_json` derives only a local activation claim and
  emits `ProtectionClaim::new(..., Vec::new())`; completion requires
  daemon snapshot / `anvil-intercept::status` integration or an
  explicitly documented fallback state that does not claim per-surface
  coverage. **Closure (2026-05-16, branch
  `feat/mlp2-048-status-daemon-snapshot`):** new
  `anvil_intercept::status::build_protection_claim_from_wire` adapter
  consumes `DaemonStatusV1` directly (parity-pinned against the
  in-memory `build_protection_claim` across 6 wire-shape unit tests
  incl. mixed-fence overlay). `anvil status --json` calls
  `pub(crate) query_daemon_status()` best-effort, canonicalises cwd,
  and routes through new `resolve_protection_claim` helper which
  uses the wire adapter when a snapshot is available and falls back
  to the locally-derivable worktree state with an explicitly empty
  `surfaces` array otherwise (documented fallback that does not
  over-claim coverage). Both failure arms (IPC unavailable + cwd
  canonicalise failed) emit `tracing` events at `debug`/`warn`
  respectively so an operator chasing "why is `surfaces` empty?" can
  distinguish the cause. +5 CLI-side `resolve_protection_claim`
  pins (incl. draining → Warming/Detached path). Council quick
  review: 2 MAJOR (silent daemon-down fallback + canonicalise
  silent miss) + 2 MINOR (missing mixed-fence parity test +
  missing CLI draining test) + 1 NIT all addressed pre-push.
- **Confidence:** medium
- **Priority:** Critical (HARD-GATE close)
- **Dependencies:** MLP-009
- **releaseNote:**
  - audience: user
  - type: changed
  - text: "`anvil status --json` now emits a typed
    `ProtectionClaim` built from the live daemon snapshot,
    including per-surface entries with closed-set state
    values. When the daemon is unreachable, output falls
    back to a locally-derivable worktree state with an empty
    `surfaces` array rather than over-claiming coverage."
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

#### MLP2-051: Driver / CLI / MCP-shim protection-claim conformance pass (umbrella)

- **Status:** Merged 2026-05-17 — all four umbrella-required sub-tasks
  (MLP2-051a #1655, -051b #1668, -051c #1675, -051e #1679) rebase-merged
  to `main`; the HARD-GATE close for §14 closed-set rendering is now
  pinned on every shipping surface. MLP2-051d remains `Blocked` on the
  Marketplace track licensing/pricing model lock and is carved out of the
  umbrella's closure condition per the spec ("051d is required only if
  MLP2-042..045 ship"); it re-attaches if the Marketplace track ships
  later. Awaiting `v0.7.0-beta` release evidence to advance to
  Released/Shipped → Complete.
- **Source note:** Originally re-specced 2026-05-17 — split into
  MLP2-051a..-051e below. The original task assumed every surface
  already rendered
  a protection claim using stringly-typed values that needed
  migration to the closed-set types in
  `crates/anvil-kernel-types/src/protection_claim.rs`. An audit
  on 2026-05-17 (after MLP2-048 closed the `anvil status` lane)
  showed only **one** surface — `anvil status` plain + JSON — has
  ever rendered the claim. The other four target surfaces
  (`anvil doctor`, MCP shim, TS driver-client, GH Action) emit no
  claim today, so the work is *additive* (new rendering on each
  surface) rather than *migrative* (rip out string matching).
  That changes scope enough to warrant a split. Keeping MLP2-051
  as a coordinating umbrella so the original `Dependencies:
  MLP2-051` references in MLP2-049/050 still resolve; the
  HARD-GATE close lives on the sub-tasks.
- **Intent (umbrella):** Each render surface that the spec §14
  protection-claim vocabulary touches consumes the same closed-set
  types from `crates/anvil-kernel-types/src/protection_claim.rs`
  and produces parity-checked output. The umbrella is closed when
  every sub-task is `Merged`.
- **Audit findings (2026-05-17, branch
  `chore/aps-mlp2-051-respec`):**
  - `anvil status` (plain + JSON) — **typed today**. Plain
    renderer consumes `LegibleSnapshot { protection:
    WorktreeClaimState, ... }`; JSON path consumes
    `ProtectionClaim` via the daemon-snapshot adapter shipped
    with MLP2-048. No further work here — kept as the reference
    implementation.
  - `anvil doctor` (`crates/anvil-cli/src/commands/doctor.rs`) —
    renders enforcement layer state but emits no
    `ProtectionClaim`. Adds new section. → **MLP2-051a**.
  - MCP shim `crates/anvil-cli/src/mcp/validation.rs` — emits
    `Diagnostic` / `Mode` / `ValidationBackendFailure`, no claim.
    Cross-boundary: the MCP wire shape is consumed by editor
    drivers, so the addition must be wire-additive (new optional
    field) under `serde(skip_serializing_if = "Option::is_none")`.
    → **MLP2-051b**.
  - TS driver-client `packages/anvil-driver-client/` — no
    `protection_claim/` module, no `ProtectionClaim` mirror.
    Needs a Zod mirror + adapter to parse the optional field from
    MLP2-051b's MCP response. → **MLP2-051c**.
  - GH Action check status — `apps/action/*` tree does not exist
    today. Gated on the Marketplace publishing track
    (MLP2-042..045), which the release cut-line defers unless
    Boring Week exercises that surface. → **MLP2-051d**, blocked
    by MLP2-042+043.
- **Cross-surface parity test:** consumes the sub-tasks. Same
  `DaemonStatusV1` input drives every surface; the rendered
  claim's `worktree_state` + sorted-by-identifier `surfaces` must
  match byte-for-byte across all five surfaces. → **MLP2-051e**.
- **Files:** see sub-tasks.
- **Validation:** umbrella is `Merged` when 051a + 051b + 051c +
  051e are `Merged`. 051d is required only if MLP2-042..045 ship
  (Boring Week gate).
- **Confidence:** high (post-audit). Sub-tasks each have a
  smaller, well-scoped surface.
- **Priority:** Critical (HARD-GATE close — distributed across
  sub-tasks).
- **Dependencies:** MLP-009, MLP2-048 (both shipped).
- **Source:** MLP-009 footnote 4; 2026-05-17 audit on
  `chore/aps-mlp2-051-respec`.

#### MLP2-051a: `anvil doctor` typed protection claim section

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
  rebase-merged at `5c762f3d`). Awaiting `v0.7.0-beta` release evidence to
  advance to Released/Shipped → Complete.
- **Intent:** `crates/anvil-cli/src/commands/doctor.rs` gains a
  "protection claim" section that consumes the daemon snapshot
  (or the local-only fallback) and prints both the worktree
  state and the per-surface entries. Reuses
  `anvil_intercept::status::build_protection_claim_from_wire`
  + the local fallback path from `status.rs::resolve_protection_claim`
  — extract that helper into a shared module if both binaries
  need it, otherwise call through `commands::status`.
- **Expected Outcome:**
  - Doctor output prints `protection: <worktree_state>` and
    one line per surface with its `identifier` + `state`.
  - Daemon-down path emits the same documented fallback shape
    used by `anvil status --json` (worktree state from local
    activation diagnostic, empty `surfaces`).
  - `--json` mode emits the same `ProtectionClaim` shape used
    by `anvil status --json`.
- **Files:** `crates/anvil-cli/src/commands/doctor.rs`,
  potentially a shared helper module if extraction is needed.
- **Validation:** Snapshot test for plain + JSON output across
  at least Unprotected / PreWriteDaemon / DegradedProtection
  states. Parity with `anvil status --json` for the same daemon
  input.
- **Confidence:** high
- **Priority:** Critical (HARD-GATE close)
- **Dependencies:** MLP2-048
- **releaseNote:**
  - audience: user
  - type: added
  - text: "`anvil doctor` now prints a typed protection-claim
    section showing the worktree state and per-surface
    entries, with `--json` emitting the same
    `ProtectionClaim` shape as `anvil status --json`."
- **Source:** MLP2-051 re-spec, 2026-05-17.

#### MLP2-051b: MCP shim emits typed protection claim in `validate_write` response

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
  rebase-merged at `7ff0e123`). Awaiting `v0.7.0-beta` release evidence to
  advance to Released/Shipped → Complete.
- **Intent:** `crates/anvil-cli/src/mcp/validation.rs` extends the
  `validate_write` response with an optional `protection_claim`
  field carrying the closed-set
  `anvil_kernel_types::protection_claim::ProtectionClaim` shape.
  Wire-additive via `serde(default, skip_serializing_if = "Option::is_none")`
  so a pre-MLP2-051b driver round-trips unchanged.
- **Expected Outcome:**
  - MCP response carries `protection_claim` when the daemon is
    reachable; field is omitted otherwise (no over-claim).
  - Existing `Diagnostic` / `Mode` / enforcement decision fields
    unchanged.
  - New rust unit test pins the wire-additive contract:
    response without `protection_claim` deserialises into a
    driver pinned to the new shape (driver-side compat).
- **Files:** `crates/anvil-cli/src/mcp/validation.rs`,
  contract test in `crates/anvil-cli/tests/`.
- **Validation:** Wire round-trip test +
  `serde(deny_unknown_fields)` audit (must NOT be set on the
  response struct so future additive fields stay safe).
- **Confidence:** medium (cross-boundary; needs driver-client
  pairing in MLP2-051c to actually be consumed).
- **Priority:** Critical (HARD-GATE close)
- **Dependencies:** MLP2-048, MLP2-051a (reuse the shared
  claim-building helper if extracted).
- **releaseNote:**
  - audience: developer
  - type: added
  - text: "The MCP shim's `validate_write` response now
    carries an optional typed `protection_claim` field using
    the closed-set vocabulary. The field is omitted when the
    daemon is unreachable, and pre-existing drivers
    round-trip the response unchanged."
- **Source:** MLP2-051 re-spec, 2026-05-17.

#### MLP2-051c: TS driver-client `ProtectionClaim` mirror + MCP response adapter

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
  rebase-merged at `d4970b19`). Awaiting `v0.7.0-beta` release evidence to
  advance to Released/Shipped → Complete.
- **Intent:** Add `packages/anvil-driver-client/src/protection_claim/`
  with a hand-rolled parser mirroring
  `crates/anvil-kernel-types/src/protection_claim.rs` (closed-set
  string unions for `WorktreeClaimState` + `SurfaceClaimState`,
  `ProtectionClaim` + `SurfaceClaim` structs). Wire it through
  the MCP response parser so consumers receive a typed claim
  when the daemon supplied one. Hand-rolled, no Zod dep — matches
  the MLP2-029 AgentTag + MLP2-030 mid-edit mirror pattern.
- **Expected Outcome:**
  - Hand-rolled `parseProtectionClaim` (no Zod dep unless
    elsewhere in this package — match the MLP2-029 pattern).
  - Byte-exact JSON parity test against a captured Rust
    `ProtectionClaim` fixture (mirrors MLP2-029/-030 pattern).
  - Missing-field tolerance: pre-MLP2-051b MCP response (no
    `protection_claim`) parses cleanly with the field absent.
- **Files:** new
  `packages/anvil-driver-client/src/protection_claim/index.ts`
  + tests, MCP response adapter updates in `client/` or wherever
  `validate_write` deserialisation lives.
- **Validation:** Cross-language parity test against a captured
  Rust fixture (matches the MLP2-029 pattern). All package
  tests green.
- **Confidence:** high (the MLP2-029 + MLP2-030 mirrors set
  the pattern; this is the third such mirror).
- **Priority:** Critical (HARD-GATE close)
- **Dependencies:** MLP2-048, MLP2-051b (consumes the wire
  field).
- **releaseNote:**
  - audience: developer
  - type: added
  - text: "`@anvil/driver-client` ships a hand-rolled
    `ProtectionClaim` parser mirroring the Rust closed-set
    types, and the MCP response adapter surfaces the typed
    claim when the daemon supplied one. Responses without
    the field parse cleanly for backward compatibility."
- **Source:** MLP2-051 re-spec, 2026-05-17.

#### MLP2-051d: GH Action check renders typed protection claim

- **Status:** Blocked
- **Intent:** When the GitHub Action publishing track lands
  (MLP2-042..045), its check-status output consumes the typed
  `ProtectionClaim` from `anvil status --json` rather than
  re-deriving state from CLI exit codes or stdout strings.
- **Expected Outcome:** GH Action emits a single claim line
  using the closed-set vocabulary; parity test against the CLI
  surface.
- **Files:** TBD — `apps/action/*` tree does not exist yet.
- **Validation:** End-to-end test in the action repo (or
  vendored harness if the action lives in-tree).
- **Confidence:** low — file paths TBD until MLP2-042..045
  decide the action's home.
- **Priority:** Critical (HARD-GATE close, but release cut-line
  defers the Marketplace track pending the licensing / pricing
  model lock for `eddacraft/anvil-action`; the gate is
  commercial, not code).
- **Dependencies:** MLP2-042 + MLP2-043 (Action exists),
  MLP2-051b + MLP2-051c (typed claim is on the wire), MLP2-048.
- **Source:** MLP2-051 re-spec, 2026-05-17.

#### MLP2-051e: Cross-surface protection-claim parity test

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
  rebase-merged at `d6df62f2`; deps MLP2-051a/b/c all Merged via PRs
  #1655/#1668/#1675). Awaiting `v0.7.0-beta` release evidence to advance to
  Released/Shipped → Complete.
- **Intent:** Single end-to-end harness drives a fixed
  `DaemonStatusV1` input through every surface (CLI status,
  CLI doctor, MCP shim, TS driver-client) and asserts the
  rendered `ProtectionClaim` is byte-identical across all of
  them.
- **Expected Outcome:**
  - Reuses the per-state fixtures from MLP2-049
    (`crates/anvil-cli/tests/fixtures/status_v1/`).
  - One Rust integration test for the three Rust surfaces;
    one TS test for the driver-client surface that re-reads
    the same fixture.
- **Files:** new `crates/anvil-cli/tests/protection_claim_cross_surface.rs`,
  new TS spec in `packages/anvil-driver-client/__tests__/`.
- **Validation:** Both tests green; same input → same JSON
  string across every surface (or byte-equivalent claim
  object after deserialise + canonical sort).
- **Confidence:** high (fixtures already exist; the harness
  is straightforward composition).
- **Priority:** Critical (HARD-GATE close)
- **Dependencies:** MLP2-051a + MLP2-051b + MLP2-051c (all
  three render surfaces produce a claim).
- **Source:** MLP2-051 re-spec, 2026-05-17.

#### MLP2-051f: Activation diagnostic consumes daemon `ProtectionClaim`

- **Status:** Merged 2026-05-22 via PR
  [#1840](https://github.com/eddacraft/anvil-001/pull/1840) at
  `e1cc066a`. Closes GH
  [#1831](https://github.com/eddacraft/anvil-001/issues/1831).
  Awaiting `v0.7.0-beta` release evidence to advance Merged →
  Released/Shipped → Complete.
- **Intent:** `crates/anvil-cli/src/activation/diagnostic.rs` consumes
  `anvil_intercept::status::query_daemon_snapshot` +
  `build_protection_claim_from_wire` and, when the daemon attests
  live enforcement for the current worktree, promotes
  handshake-verified MCP clients to `McpTier::LiveValidation`. With
  the promotion, `protection_state()` reaches `Protecting` for the
  `anvil start --verify` + `anvil status --verify` paths, closing
  GH [#1831](https://github.com/eddacraft/anvil-001/issues/1831). The
  spec lives at
  `plans/specs/2026-05-21-activation-daemon-evidence-wireup.md`;
  council verdicts (`plan-f4668683`, 4 COUNTER / 1 CONSENSUS) attached
  the hard gates enumerated below.
- **Expected Outcome:**
  - `anvil start --verify` returns `protecting` when the daemon is
    running, the worktree is registered + canonicalised-matched on
    the daemon side, and at least one MCP client has reached
    `RestartHandshakeVerified`.
  - Honest fallbacks: daemon unreachable, IPC timeout, stale
    snapshot heartbeat (>45s), `WorktreeClaimState::Warming`, or
    `DegradedProtection`-all-`Quarantined` leave the diagnostic at
    `ready_restart_required` and surface a state-appropriate repair
    hint via `activation/render.rs`.
  - Diagnostic transparency via `tracing::info!` on every promotion
    and `tracing::debug!` on every skip path (mirrors
    `promote_restart_required_after_handshake` in
    `diagnostic.rs:507-520`).
  - Stale comment at `crates/anvil-cli/src/activation/mcp_client.rs:307`
    is replaced with a grep-able pointer to the new
    `promote_to_live_validation_when_daemon_attests` function
    (full module path), eliminating the MLP2-025b
    zero-callers shape this fix exists to fix.
- **Files:**
  - `crates/anvil-cli/src/activation/diagnostic.rs` — new
    `promote_to_live_validation_when_daemon_attests` + call site in
    `verify()` after `promote_restart_required_after_handshake`.
  - `crates/anvil-cli/src/activation/mcp_client.rs` — comment update
    at line 307 with the new function reference.
  - `crates/anvil-cli/src/activation/render.rs` — state-dependent
    repair-hint branching for `ReadyRestartRequired`.
  - New constant `ACTIVATION_DAEMON_QUERY_TIMEOUT = 500ms` colocated
    with the diagnostic-side query function; dedicated wrapper
    around `query_daemon_snapshot` that enforces the bound
    independently of the daemon-side `REQUEST_TIMEOUT = 2s`.
  - `crates/anvil-cli/tests/protection_claim_cross_surface.rs` or
    new `crates/anvil-cli/tests/activation_daemon_evidence.rs` —
    end-to-end integration test against a real daemon socket (no
    mocks for the IPC boundary).
- **Validation (hard gates from council, all required):**
  1. **Worktree canonicalisation contract.** Activation MUST
     canonicalise its `worktree` argument before the IPC call,
     using the same `std::fs::canonicalize` + warn-on-failure
     pattern as
     `crates/anvil-cli/src/commands/protection_claim_section.rs::fetch_protection_claim_for_cwd`
     (line 72). Daemon-side path is canonicalised at register-time
     via `DriverManifest::validate_workspace_roots`; the activation
     side must produce a path comparing byte-equal to the
     registered form. Regression test: register at canonical path,
     query via symlink, assert `Unprotected` (NOT promoted).
  2. **Heartbeat freshness window.** 45 seconds, computed as
     `max(SessionRecord.last_heartbeat_unix)` across the worktree's
     registered sessions compared against `SystemTime::now()`.
     Calibrated against the producer cadence (`HEARTBEAT_INTERVAL=10s`
     + `DEFAULT_HEARTBEAT_TTL=30s` + ~5s skew slack). Not
     operator-configurable upward (security veto on downgrade
     surface). Tighter via config permitted, never looser.
     Second consistency anchor: `DaemonStatusV1.generated_at_unix`
     vs `SystemTime::now()` — same 45s bound. Sentinel `0` means
     "no anchor; fall back to per-session freshness" (pinned by
     MLP2-051h tests).
  3. **`WorktreeClaimState` promotion predicate, enumerated.**
     - `PreWriteDaemon` → promote.
     - `DegradedProtection` with ≥1 `SurfaceClaim::Participating`
       → promote.
     - `DegradedProtection` all `Quarantined` → do NOT promote;
       render-hint points at `anvil intercept recover`.
     - `Warming` → do NOT promote (transient).
     - `Unprotected` → do NOT promote.
  4. **`ACTIVATION_DAEMON_QUERY_TIMEOUT = 500ms`** named constant
     with a hung-daemon stub test asserting verify latency does
     not extend beyond `timeout + 100ms`. Inheriting the 2s
     `REQUEST_TIMEOUT` is rejected — verify is interactive.
  5. **End-to-end integration test against a real daemon socket**
     (or the INTD integration test stub) — no mocks for the IPC
     boundary. The test must spawn a daemon, call `verify()`
     end-to-end, and assert `protection_state() == Protecting`.
     If the wire-up is absent, this test fails. Eliminates the
     MLP2-025b shape where a synth-mocked unit suite passes
     against a missing production call site.
  6. **Structured tracing on every promotion / skip path.**
     `tracing::info!` on success carrying `worktree`,
     `worktree_claim_state`, `clients_promoted`. `tracing::debug!`
     on skip carrying `reason` ∈ `{daemon_unreachable,
     worktree_unenforced, stale_heartbeat, platform_gap,
     warming, all_surfaces_quarantined}`.
  7. **Render-hint regression tests** for each
     `protection_state` × daemon-state combination in the §"Failure
     modes & their states" matrix from the spec (including
     Windows-row, daemon-mid-restart, daemon-no-sessions-yet,
     `DegradedProtection`-all-`Quarantined`).
  8. **Client attribution predicate (council split, recommended
     resolution).** Promotion requires ≥1
     `SurfaceClaim::Participating` for the worktree (any client) —
     cardinality-based, not identity-based, because daemon
     `agent_tag` ↔ `McpClientId` alignment is not yet resolved
     (ARCH-001 follow-up). Strictly tighter than mass-promotion
     of every handshake-verified client.
  9. Existing CI gates green: `cargo test --workspace`,
     `cargo fmt --all --check`, `cargo clippy --workspace
     --all-targets -- -D warnings`,
     `pnpm format:check && pnpm lint:check && pnpm typecheck && pnpm test`.
- **Confidence:** medium (medium-touch surface; the hard gates
  exist precisely because the easy version of this fix would
  reproduce MLP2-025b's zero-callers shape).
- **Priority:** Critical — closes GH #1831
  (`ready_restart_required` stuck after MCP install on Windows +
  Scoop + PowerShell). Two Windows users surfaced the same defect
  on `v0.7.0-beta`; the bug is platform-agnostic but Windows-skewed
  by selection bias.
- **Dependencies:**
  - MLP2-075 (Windows IPC parity, PR #1836, merged) — hard gate.
  - MLP2-051h (`DaemonStatusV1::generated_at_unix` wire-add,
    merged on `main` at `4ec9c5a4`) — hard gate.
- **releaseNote:**
  - audience: user
  - type: fixed
  - text: "`anvil start --verify` and `anvil status --verify` now
    report `protecting` when the intercept daemon is running and
    attests live enforcement for the current worktree, instead of
    staying stuck at `ready_restart_required` after the MCP
    handshake. Closes GH #1831."
- **Source:** Activation-daemon-evidence wire-up spec
  (`plans/specs/2026-05-21-activation-daemon-evidence-wireup.md`)
  §"APS placement"; council session `plan-f4668683` (5 personas,
  4 COUNTER / 1 CONSENSUS).

#### MLP2-051h: `DaemonStatusV1::generated_at_unix` wire-add

- **Status:** In Progress
- **Intent:** Precursor to the MLP2-051f activation wire-up.
  `DaemonStatusV1` carries no daemon-level wall-clock anchor today —
  `HealthStateV1.uptime_seconds` is monotonic-since-start and the only
  Unix-clock signals are per-session `last_heartbeat_unix` /
  `started_at_unix`. The MLP2-051f freshness check needs a second
  consistency anchor at the snapshot level so the activation-side
  consumer can sanity-check the snapshot itself, not just the latest
  session inside it (defence in depth against a daemon that stops
  refreshing its own clock but keeps sessions registered).
- **Expected Outcome:**
  - Additive `generated_at_unix: u64` field on
    `anvil_intercept_proto::status::DaemonStatusV1`, stamped at
    snapshot-build time with `SystemTime::now()` Unix seconds.
  - Wire-additive via `#[serde(default)]`: a pre-MLP2-051h daemon
    talking to a post-MLP2-051h consumer deserialises to
    `generated_at_unix: 0`, which the consumer treats as "no snapshot
    anchor available" (falls back to per-session heartbeat freshness
    only — same posture as today).
  - Daemon-side construction path (`anvil_intercept::status::build_status`)
    takes the Unix-seconds value as an explicit argument so the call
    is deterministic and testable; the IPC provider
    (`DaemonStatusProvider::query_status`) captures
    `SystemTime::now()` at the same boundary it already captures
    `Instant::now()` for the latency snapshot.
- **Files:**
  - `crates/anvil-intercept-proto/src/status.rs` — field + serde
    attribute + module doc note.
  - `crates/anvil-intercept/src/status.rs` — `DaemonStatus` field,
    `build_status` arg, `to_wire` mapping,
    `DaemonStatusProvider::query_status` capture site.
  - `crates/anvil-cli/src/commands/intercept.rs` +
    `crates/anvil-cli/src/commands/status.rs` +
    `crates/anvil-cli/tests/protection_claim_cross_surface.rs` +
    `crates/anvil-run/src/preflight.rs` — update test fixtures /
    struct-literal construction sites.
- **Validation:**
  - **Parity tests (mandatory):** pre-MLP2-051h wire shape (JSON
    without the field) deserialises into the new type with
    `generated_at_unix == 0`; new shape round-trips with the value
    intact; new shape serialised back always includes the key (even
    when `0`) so a downstream consumer can byte-equivalence two
    snapshots from the same producer. Pinned in
    `crates/anvil-intercept-proto/src/status.rs` tests
    (`pre_mlp2_051h_payload_round_trips_with_generated_at_unix_default_zero`,
    `generated_at_unix_round_trips_when_present`,
    `generated_at_unix_serialises_always_when_zero`).
  - **Live-stamp test (mandatory):** the production
    `DaemonStatusProvider::query_status` path actually stamps a
    non-zero `generated_at_unix` at the IPC boundary. Pinned in
    `crates/anvil-intercept/src/status.rs::tests::provider_stamps_non_zero_generated_at_unix`
    so a future caller of `build_status` (or a refactor of
    `DaemonStatusProvider::query_status`) silently passing `0` fails
    the test, not the downstream consumer.
  - **Sentinel-equality test (mandatory):** `generated_at_unix == 0`
    is pinned as the documented "no anchor available — fall back to
    per-session heartbeat freshness" sentinel. Pinned in
    `crates/anvil-intercept/src/status.rs::tests::generated_at_unix_zero_is_the_no_anchor_sentinel`
    so a future MLP2-051f consumer cannot drift the contract to a
    `> threshold` check (which would treat a `NoopStatusProvider`
    snapshot as "anchor present, just very old" and pass the
    freshness gate — the failure mode the MLP2-051h precursor exists
    to prevent).
  - `cargo test --workspace` green; `cargo clippy --workspace
    --all-targets -- -D warnings` green; `cargo fmt --all --check`
    green.
  - No consumer change required — every existing reader either uses
    `#[serde(default)]` semantics (older `Deserialize` derive) or
    will simply ignore the new field until MLP2-051f wires the
    activation-side freshness check against it.
  - `NoopStatusProvider::query_status` emits a `tracing::debug!`
    event on every invocation so a live trace in a production binary
    (where `run_foreground` always swaps the noop default for a real
    `DaemonStatusProvider` via `with_status_provider` on both the
    Unix and Windows listener-bind branches) is a diagnosable wiring
    regression rather than a silent fallback to the no-anchor posture.
- **Confidence:** high — one field, one parity test, all consumers
  are additive-tolerant by the project's `additive-optional-fields`
  rule (MLP2-052) and the existing `DaemonStatusV1` precedent
  (`cache_entries`, `cache_invalidations_total`,
  `in_flight_evaluations`, `cache_invalidations_rate_limited` all
  follow the same shape).
- **Priority:** High (gates MLP2-051f activation wire-up; non-blocking
  for the `v0.7.0-beta` tag because activation diagnostic does not
  exist yet, but lands ahead of it to keep the wire-add and the
  consumer in separate PRs).
- **Dependencies:** none (additive precursor; deliberately filed
  ahead of MLP2-051f so the field exists on the wire before the
  first consumer arrives).
- **Source:** Activation-daemon-evidence wire-up spec
  (`plans/specs/2026-05-21-activation-daemon-evidence-wireup.md`)
  §"New objections deferred to follow-up issues" — adversarial /
  security objection that `DaemonStatusV1` has no daemon-level
  wall-clock anchor; spec §"APS placement" calls out filing as
  MLP2-051h ahead of MLP2-051f.

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

- **Status:** Merged
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

- **Status:** Merged
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

- **Status:** Merged
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

- **Status:** Merged
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

- **Status:** Merged
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

### M. Full-codebase Council corrective follow-ons

Items in this group were filed from the 2026-05-15 full MLP/MLP2 review.
They are not new product capabilities; they close correctness, security, and
planning-truth gaps found while auditing the shipped primitives against the
remaining v2 integration surface.

#### MLP2-061: Post-rollover append-head recovery

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Intent:** Hook-side witness appends derive the next `(seq,
  prev_line_hash)` from the full archive + active chain, not only
  `active.ndjson`, so a rollover cannot cause a fresh genesis to be
  seeded on top of archived history.
- **Expected Outcome:**
  - `append_witness` verifies `witness_paths(repo_root)` and chains
    new lines from the verified DAG tip after rollover.
  - A tight-rollover regression appends after active rollover and
    verifies archive + active as one continuous chain.
  - `hook bootstrap --witness-recent` checks active + archives before
    retroactively witnessing a commit.
- **Files:** `crates/anvil-cli/src/commands/hook.rs`,
  `crates/anvil-cli/tests/`.
- **Validation:** Tight rollover fixture: append to rollover, append
  again, then `verify_chain_dag(witness_paths)` succeeds with no
  duplicate genesis.
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** MLP2-011, MLP2-012
- **Source:** Council full audit 2026-05-15 (general + adversarial:
  `chain_head` and `commit_is_witnessed` active-only reads undermine
  rollover correctness).

#### MLP2-062: `anvil l4-validate` verifies witness-chain integrity before trusting witnessed SHAs

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Intent:** The dedicated L4 CLI surface must reject or block on a
  broken witness chain before treating any `commit_sha` as L3 evidence.
- **Expected Outcome:**
  - `l4_validate.rs` reuses the same archive + active path ordering
    and DAG verifier as pre-push before collecting witnessed commits.
  - Broken or tampered witness chains produce a blocking CI/action
    result, not an empty trusted set or silent allow.
  - Archive files are streamed in deterministic order.
- **Files:** `crates/anvil-cli/src/commands/l4_validate.rs`,
  `crates/anvil-cli/tests/`.
- **Validation:** Tampered witness fixture under `anvil l4-validate`
  exits blocking/error and never reports the forged commit as
  witnessed.
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** MLP2-046, MLP2-011
- **releaseNote:**
  - audience: user
  - type: security
  - text: "`anvil l4-validate` now verifies witness-chain
    integrity before trusting any witnessed commit SHA as L3
    evidence. Broken or tampered chains produce a blocking
    result instead of a silent allow or empty trusted set."
- **Source:** Council full audit 2026-05-15 (security: CI/Marketplace
  L4 surface trusted witness files without first verifying the chain).

#### MLP2-063: Bounded policy-file load path for hook and L4 validation

- **Status:** Merged
- **Intent:** Policy loading for pre-push and `anvil l4-validate`
  must honour the same file-size and parse-resource bounds as
  `.anvil.*` config parsing.
- **Expected Outcome:**
  - A shared bounded policy loader rejects files larger than
    `anvil_config::MAX_CONFIG_FILE_BYTES` before `read_to_string`.
  - Hook and L4 validation use the shared loader for
    `anvil/policy.{yml,yaml,json,toml}`.
  - Oversized policy files fail with one noise-disciplined internal
    error in hooks and a clear blocking/error result in CI mode.
- **Files:** `crates/anvil-cli/src/commands/{hook,l4_validate}.rs`,
  `crates/anvil-l4/src/policy.rs` if the helper belongs with policy
  parsing.
- **Validation:** 1 MiB+ policy fixture does not allocate the whole
  file and returns the expected hook / CLI outcome.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP2-060
- **Source:** Council full audit 2026-05-15 (security: policy loaders
  bypassed the bounded `anvil-config::parse_file` path).

#### MLP2-064: Rule-cache generation guard for invalidate-during-resolve

- **Status:** Merged
- **Intent:** Cache misses that resolve outside the mutex must not
  insert stale rules after a watcher invalidation has already observed
  a stricter config write.
- **Expected Outcome:**
  - `RuleSetCache` tracks a per-worktree generation / invalidation
    token.
  - `get_or_resolve` records the generation before resolving and
    re-checks it under the lock before insertion.
  - If invalidation raced the resolve, the stale entry is discarded or
    recomputed; later evaluations do not keep using the old rule set.
- **Files:** `crates/anvil-intercept/src/rule_cache.rs`,
  `crates/anvil-intercept/src/watcher.rs`.
- **Validation:** Deterministic barrier test: miss starts resolving,
  config invalidates while cache is empty, resolver returns old
  payload, cache does not serve that stale payload to the next caller.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP2-001, MLP2-002, MLP2-058
- **Source:** Council full audit 2026-05-15 (security: stale reinsert
  window after pre-attribution invalidation).

#### MLP2-065: Partial-baseline resume detects tree drift before the cursor

- **Status:** Merged
- **Intent:** Budgeted baseline scans must not mark a baseline
  complete when files were added or renamed lexicographically before
  the saved continuation cursor between runs.
- **Expected Outcome:**
  - Partial baseline state records enough scan-generation context to
    detect pre-cursor tree drift, or resume forces a full restart when
    such drift is possible.
  - A new violating file inserted before the cursor between resume
    runs is scanned before the baseline is marked complete.
  - Operator messaging explains restart/rescan rather than silently
    skipping files.
- **Files:** `crates/anvil-baseline/src/store.rs`,
  `crates/anvil-cli/src/commands/baseline.rs`.
- **Validation:** Start budgeted baseline, add `000-*.ts` with a
  violation before the saved cursor, resume; final baseline includes
  the new finding or restarts safely.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP2-036
- **Source:** Council full audit 2026-05-15 (adversarial: fixed-
  fixture resume tests did not cover mutation between chunks).

#### MLP2-066: Maintained YAML parser migration and ADR closeout

- **Status:** Merged
- **Intent:** Complete the MLP2-060 follow-up by deciding and tracking
  migration from deprecated `serde_yaml` to a maintained YAML parser
  or recording why alias-reject byte scanning remains the accepted
  long-term posture.
- **Expected Outcome:**
  - ADR-level note records the parser decision, false-positive trade-
    off, and migration / non-migration rationale.
  - If migration is selected, `anvil-config` moves to the maintained
    crate with the same size, alias, and depth fixtures green.
  - If migration is deferred, the deferral has an owner and review
    date.
- **Files:** `crates/anvil-config/src/parse.rs`,
  `plans/decisions/037-witness-chain-and-l4-policy.md` or a new ADR
  if the decision is broader than MLP, and
  `plans/decisions/DECISION-LOG.md` when an ADR changes.
- **Validation:** `cargo test -p eddacraft-anvil-config --lib`; ADR
  check when ADR files change.
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MLP2-060
- **Source:** Council full audit 2026-05-15 (planning/security:
  MLP2-060 said maintained-parser migration was tracked separately,
  but no APS item existed).

### N. Daemon evaluator host (GV2 groundwork)

Standalone bridge between the current in-memory kernel graph (a
library primitive that lives and dies inside each CLI process) and
the future GV2 multi-graph substrate. Reuses the daemon's existing
`RuleSetCache` pattern (bounded LRU + watcher invalidation +
generation guard) to hold a per-worktree `SymbolGraph` warm across
CLI invocations. Exposes only a narrow evaluator RPC — verdicts,
not graph nodes — so the kernel's internal graph schema stays free
to redesign once GV2-001..-023 land.

#### MLP2-067: Daemon-hosted graph cache with narrow evaluator RPC

- **Status:** Draft
- **Intent:** `anvil check` and `anvil watch` currently rebuild the
  `anvil_kernel::graph::SymbolGraph` fresh per CLI invocation. The
  daemon already holds a per-worktree `RuleSetCache` keyed on
  `WorktreeKey` with file-watcher invalidation (MLP2-001 / MLP2-057
  / MLP2-064 shipped this pattern). Extend the same pattern to hold
  the symbol graph in the daemon, apply file-watcher deltas via the
  existing `anvil_kernel::graph::incremental` API, and serve a
  narrow evaluator RPC so callers pay the cold-build cost once per
  daemon lifetime instead of once per CLI invocation. Intentionally
  scoped to **verdicts over the wire, never graph nodes** — that
  keeps the IPC contract evaluator-shaped instead of graph-shaped,
  so GV2's eventual graph-model redesign does not break daemon
  consumers.
- **Expected Outcome:**
  - New `crates/anvil-intercept/src/kernel_cache.rs` module holding
    `HashMap<WorktreeKey, SymbolGraph>` with the same bounded LRU +
    generation-guard + unregister-hook pattern as `RuleSetCache`.
    Default capacity matches `DEFAULT_RULE_SET_CACHE_CAPACITY`.
  - Daemon watcher applies `GraphDelta`s via the existing
    `incremental::update_file` / `remove_file` /
    `re_resolve_imports` API on relevant file saves, instead of
    evicting the cached graph wholesale.
  - New narrow IPC verb (e.g. `kernel.evaluate`) accepts a
    `WorktreeKey` + file change descriptor and returns a
    `Vec<Diagnostic>` from the existing in-process evaluator. The
    `SymbolGraph` itself is never serialised over the wire.
  - `anvil check` and `anvil watch` try the daemon socket first; on
    `EConnRefused` or socket timeout they fall through to the
    existing in-process kernel evaluator with byte-identical output.
    Same daemon-RPC-plus-embedded-fallback shape as MLP2-005.
  - Benchmark fixture demonstrates the cold-build cost is paid once
    per daemon lifetime and not once per CLI invocation across a
    synthesised reference fixture (size pinned in the benchmark).
- **Files:** `crates/anvil-intercept/src/kernel_cache.rs` (new),
  `crates/anvil-intercept/src/watcher.rs` (wire graph delta
  application alongside rule-cache invalidation),
  `crates/anvil-intercept-proto/src/` (new IPC verb wire shape +
  forward-compat additive fields),
  `crates/anvil-cli/src/commands/check.rs`,
  `crates/anvil-cli/src/commands/watch.rs` (daemon-first wiring +
  embedded fallback), `crates/anvil-kernel/benches/` (cold-vs-warm
  bench fixture).
- **Validation:** Cold-vs-warm before/after benchmark on a
  synthesised fixture: cold build via embedded path vs warm read
  via daemon path across N consecutive CLI invocations against a
  long-lived daemon process. Daemon-down regression test confirms
  byte-identical output through the embedded fallback. Council
  follow-up: pin the IPC verb shape with an additive-fields
  forward-compat test in the MLP2-052 style so GV2 cannot break it
  silently.
- **Confidence:** medium — the rule-cache pattern is a strong
  template, but the watcher-driven delta-application path on the
  cached graph is new code rather than reused.
- **Priority:** Medium — does not gate `v0.7.0-beta`. Wave 5
  candidate: promote to High if Boring Week feedback shows cold-
  start friction. Otherwise land before GV2 design starts so the
  consumer-pattern groundwork is in place when GV2-023 (consumer
  query contract) needs it.
- **Dependencies:** MLP2-001 (rule_cache pattern + watcher
  invalidation infrastructure), MLP2-057 (bounded LRU + unregister
  hook), MLP2-064 (generation guard for cache-vs-resolve race),
  MLP2-005 (daemon-RPC-plus-embedded-fallback template once it
  ships).
- **Coordinates with:** GV2-001 (architecture spec), GV2-022 (hot-
  path read API and latency guardrails), GV2-023 (consumer query
  contract). The narrow-verdict RPC scope of MLP2-067 intentionally
  avoids locking in graph-shape decisions that GV2-010..-014 will
  own.
- **Source:** Brainstorm 2026-05-16 — middle-ground design between
  the current in-process kernel graph (rebuilds per CLI invocation)
  and the full GV2 multi-graph substrate. The conclusion was to lay
  the consumer-pattern groundwork now via a narrow evaluator RPC,
  reusing the existing daemon cache + watcher pattern, while
  deferring GV2's stable identity, persistence, and multi-graph
  joins.

### O. MLP2-016 audit follow-ons (2026-05-17)

#### MLP2-068: `git cat-file --batch` for `CommitAntipatternEngine` blob fetch

- **Status:** Merged
- **Merged:** 2026-05-19 reconciliation after implementation commit
  `d54a5f86` (`feat(l4-engine): MLP2-068 batch git cat-file for commit
  blobs`). Cleanup agent advances to Released/Shipped when `v0.7.0-beta`
  release evidence lands.
- **Intent:** `CommitAntipatternEngine::validate_commit` (shipped
  MLP2-016, PR #1627) reads each scannable file in a commit via a
  separate `git show <sha>:<path>` `Command::spawn`. At ~5–15 ms
  spawn cost per file, a commit touching 200 files burns 1–3 s on
  process startup alone — most of MLP2-022's 2 s pre-push wall-
  clock budget — before any actual rule scan runs. Council
  kernel-maintainer flagged this in the PR #1627 review and the
  follow-up was captured in the post-merge plan rather than fixed
  in-PR. Switch to `git cat-file --batch` with stdin-piped
  `<sha>:<path>` lines so the engine pays one `git` process per
  commit instead of N + 1.
- **Expected Outcome:**
  - `read_commit_blob` replaced by a `BatchCatFile` helper that
    spawns `git cat-file --batch` once per `validate_commit` call
    and pipes the per-file revspecs over stdin in one batch.
  - Per-commit cost on a 200-file fixture drops from O(N) process
    spawns to O(1).
  - Existing `list_commit_files_*` and `read_commit_blob_*` tests
    continue to pass (the contract — `Option<Vec<u8>>` per path
    in input order — is preserved).
  - New `validate_commit_handles_200_file_commit_under_budget`
    test pins the perf shift with a synthesised 200-file fixture
    and a wall-clock assertion well under `PRE_PUSH_BUDGET`.
- **Files:** `crates/anvil-cli/src/l4_engine.rs` (replace
  `read_commit_blob` with batch helper; preserve the colon-in-path
  and zero-SHA guards).
- **Validation:** Cargo test continues green; 200-file synthesised
  fixture demonstrates the per-commit cost drop; `tracing::debug!`
  on git stderr still surfaces individual batch-entry failures so
  the observability surface from MLP2-016 stays intact.
- **Confidence:** medium — the `git cat-file --batch` protocol is
  stable but the streaming-stdout parser is new code in this
  crate.
- **Priority:** Medium — does not block `v0.7.0-beta`; ships
  whenever push latency on fat commits becomes the bottleneck.
- **Dependencies:** MLP2-016 (the engine surface this optimises).
- **Source:** PR #1627 Council quick review (kernel-maintainer +
  operations-reviewer); post-merge plan
  `plans/reviews/post-merge/feat-mlp2-016-real-engine.md`.

#### MLP2-069: `EngineUnavailableReason::IoError` variant

- **Status:** Draft
- **Intent:** `anvil_l4::EngineUnavailableReason` closes over
  `{ NotImplemented, BinaryMissing, Timeout }`. MLP2-016 maps
  several distinct I/O outages (`tempfile::TempDir::new()`
  failure, mid-validate disk-full, `git show` permission error)
  onto `BinaryMissing` because no better variant exists. Council
  flagged this as MAJOR: the existing reasons carry semantic
  baggage that misleads observability tooling — `BinaryMissing`
  implies "git is not on PATH" and `Timeout` implies "the
  operation started but stalled". Add a dedicated `IoError`
  variant so production incident investigation can distinguish
  infrastructure-resource failures from binary-resolution
  failures.
- **Expected Outcome:**
  - New `EngineUnavailableReason::IoError` variant in
    `crates/anvil-l4/src/validate.rs`.
  - `CommitAntipatternEngine` re-maps the `TempDir`, blob-write,
    and other I/O failure sites from `BinaryMissing` →
    `IoError`. `BinaryMissing` reserved for "binary truly not on
    PATH / not executable".
  - Hook + `l4-validate` `tracing::warn!` on `EngineUnavailable`
    already prints `reason = ?reason` (Council #C-016I), so the
    new variant surfaces in production logs without further
    wiring.
  - `engine_unavailable_reasons_are_distinct` test in `anvil-l4`
    extended to include the new variant; closed-set match arms
    across the workspace (`hook.rs`, `l4_validate.rs`,
    `l4_engine.rs`) re-exhaustive-checked.
- **Files:** `crates/anvil-l4/src/validate.rs` (new variant),
  `crates/anvil-cli/src/l4_engine.rs` (re-mapping at I/O sites),
  any sibling consumers found exhaustive-matching the enum.
- **Validation:** Cargo test continues green; new
  `tempdir_failure_reports_io_error` test pins the re-mapping.
- **Confidence:** high — additive enum variant with a small re-
  mapping surface.
- **Priority:** Low — observability hygiene, not correctness.
  Land when MLP2 has next breathing room.
- **Dependencies:** MLP2-016.
- **Source:** PR #1627 Council quick review (kernel-maintainer +
  adversarial-reviewer); post-merge plan
  `plans/reviews/post-merge/feat-mlp2-016-real-engine.md`.

### P. v0.7.0-beta release-council follow-ups (2026-05-20)

#### MLP2-070: Re-derive lineage anchor inside the daemon IPC handler

- **Status:** In Progress
- **Intent:** `SessionRegistry::register_with_lineage`
  (`crates/anvil-intercept/src/registry.rs:526-553`) admits
  `daemon_issued_tag`, `pid`, and `pid_starttime` from its caller.
  The MLP2-025 spoof cross-check assumes those values are
  daemon-derived; if a same-UID IPC caller reaches the register
  path with attacker-supplied lineage, the cross-check passes
  against the attacker's self-declared anchor. Move lineage
  derivation off the wire: read `pid` from the connected peer's
  `SO_PEERCRED` / `GetNamedPipeClientProcessId` and
  `pid_starttime` from `/proc/PID/stat` field 22 (Linux) /
  `proc_pidinfo` (macOS) / `GetProcessTimes` (Windows) inside
  the daemon's IPC handler, and reject any request whose body
  carries lineage fields rather than allowing them to override
  the peer-derived values.
- **Expected Outcome:**
  - The wire shape for `session_register_params` drops the
    `lineage` and `pid_starttime` body fields (or keeps them
    only as advisory and refuses on mismatch with the peer-
    derived values, behind an additive-fields forward-compat
    pin in the MLP2-052 style).
  - `register_with_lineage` is reached only from the daemon's
    own IPC handler with peer-credential-derived `pid` and
    `pid_starttime`. The legacy `register` path remains for
    callers that do not want the lineage index.
  - Regression test: a hand-crafted IPC frame carrying a
    forged `pid` / `pid_starttime` / `daemon_issued_tag` is
    rejected (or the body values are ignored in favour of the
    peer-derived ones, with a `tracing::warn!` on mismatch).
  - Cross-check parity preserved: the MLP2-025 spoof reject
    path still fires on PID-reuse-after-launcher-exit, with no
    change to the public `register` surface or the
    `degraded:spoofed-attribution` fence reason.
- **Files:** `crates/anvil-intercept/src/registry.rs`
  (signature change on `register_with_lineage` or new
  daemon-internal variant), `crates/anvil-intercept/src/ipc.rs`
  (peer-credential plumb-through to the register handler),
  `crates/anvil-intercept-proto/src/session.rs` (wire-shape
  pin), `crates/anvil-intercept/tests/midedit_contract.rs` (or
  new dedicated lineage-forgery test).
- **Validation:** `cargo test -p eddacraft-anvil-intercept`
  (registry + ipc) + new lineage-forgery regression test.
  Existing MLP2-025 spoof cross-check tests must continue to
  pass byte-identically.
- **Confidence:** medium — the surface is well-understood (the
  peer-credential read pattern is already used by INTD-015's
  `originating_driver_id` mint in `fanout.rs`), but the IPC
  handler refactor crosses the proto crate's wire-shape.
- **Priority:** Medium — does not block `v0.7.0-beta` (the
  manifest allowlist gates IPC reach in the same-UID trust
  zone), but should land before any non-Anvil same-UID driver
  ships against the daemon.
- **Dependencies:** MLP-014 (parent multi-session work,
  Complete per `plans/archive/modules/multilayer-protection.aps.md`),
  MLP2-023 (composite session key), MLP2-025 (spoof cross-check
  primitives, Merged), INTD-002 (peer-credential plumbing).
- **Source:** Release council pass 1, 2026-05-20
  ([`plans/reviews/release-council/2026-05-20-v0.7.0-beta-pre-tag.md`](../reviews/release-council/2026-05-20-v0.7.0-beta-pre-tag.md))
  ship-with-doc verdict on DeepSec
  [#1674](https://github.com/eddacraft/anvil-001/issues/1674)
  ("IPC clients can mint trusted lineage tags"). Operator
  framing in [`docs/runbooks/v0.7.0-beta-security-note.md`](../../docs/runbooks/v0.7.0-beta-security-note.md)
  §M1.

#### MLP2-071: INTD-015 cross-session policy follow-up

- **Status:** In Progress (Phase 1 landed; Phase 2 follow-up pending)
- **Phase 1 (this PR):** Shipped the daemon-side reachability of
  the fan-out, the keyed redaction primitive that folds in §H2,
  and the registry binding flow:
  - `IpcCommand::SubscribeTelemetry` + `UnsubscribeTelemetry`
    proto variants with round-trip tests
    (`crates/anvil-intercept-proto/src/lib.rs`).
  - `TelemetryRedactionKey` per-startup HMAC salt
    (`crates/anvil-intercept/src/fanout.rs`); replaces unsalted
    SHA-256 on production callers under the
    `intd015-path-v1\0` domain separator, closing
    `v0.6.0-beta-security-note.md` §H2 on the redaction-primitive
    half.
  - `Fanout` constructed in `run_foreground` via
    `DaemonState::new` with
    `Resolved::cross_session_policy()` and a fresh
    `TelemetryRedactionKey::new_random()` salt.
  - `RegistryOwnershipResolver` consults the live
    `SessionRegistry` via the new `bind_subscriber` /
    `lookup_subscriber_binding` methods on the registry.
  - Regression pins added:
    `daemon_state_constructs_fanout_with_configured_cross_session_policy`
    (proves the literal #1722 reachability closure) +
    `registry_ownership_resolver_consults_subscriber_binding`
    (proves Phase D's binding flow).
- **Phase 2 (follow-up):** Subscriber surface + production
  broadcaster. The IPC accept-loop multiplex that routes the
  `SubscribeTelemetry` frame through to `Fanout::register` and
  the producer site that calls `Fanout::route` are deferred
  until the production `NotificationEnvelope` broadcaster
  feature lands (no in-tree producer broadcasts notification
  envelopes to network subscribers today; see
  `crates/anvil-intercept/src/fanout.rs:73-99` for the wave-1
  doc on the missing producer). Phase 2 unblocks alongside the
  notification telemetry stream feature itself; tracking
  continues at #1722.
- **Design pass:** Complete — see
  [`plans/specs/2026-05-21-intd-015-cross-session-attribution-design-pass.md`](../specs/2026-05-21-intd-015-cross-session-attribution-design-pass.md).
  The spec decides the `IpcCommand::SubscribeTelemetry` frame
  shape, `SubscriberId` peer-credential minting, the
  `RegistryOwnershipResolver` production impl, the per-startup
  HMAC salt that folds in §H2 of
  `docs/runbooks/v0.6.0-beta-security-note.md`, the spoofed-
  origin denial rule (D6), and the implementation slice
  contract with its validation matrix. MLP2-070 (lineage-
  anchor daemon-derivation hardening) is documented as a
  prerequisite for operators enabling `allow_cross_session:
  true` in production, not for the wire-up itself; the slice
  ships safely with the default-false posture.
- **Intent:** Resume INTD-015 once the design pass produces a
  written contract. The release-council pass 1 verdict
  (2026-05-20, operations-reviewer) treated the existing
  inert-but-parsed shape as ship-correct for `v0.7.0-beta`
  because the default (`false`) keeps the redaction filter on
  the cold path; the follow-up exists so an operator who
  *does* enable the flag eventually sees the redacted-delivery
  contract the spec describes, rather than a no-op.
- **Expected Outcome (post-unblock):**
  - Design pass artefact filed under `plans/specs/` defining
    the cross-session-attribution contract: which
    `(rule_id, hash_of_path)` pairs reach which subscribers,
    how the spoof cross-check interacts with cross-session
    deliveries, and how the per-startup HMAC salt (tracked in
    `v0.6.0-beta-security-note.md` §H2 follow-up) feeds the
    redaction primitive.
  - Implementation slice that wires the design through
    `Fanout::route` and the subscribe IPC frames added by
    INTD-011 / DRVR-001, with regression coverage for the
    three INTD-015 cases (own-session honoured, cross-session
    rejected when flag false, redacted delivery when flag true).
  - Operator-facing release-note line removed from CHANGELOG
    "Known gaps" once the flag does what its documentation
    says.
- **Files:** `plans/specs/` (new design pass artefact, TBD
  name), `crates/anvil-intercept/src/fanout.rs`,
  `crates/anvil-intercept/src/config.rs`,
  `crates/anvil-intercept/src/ipc.rs` (subscribe frame
  surface).
- **Validation:** Existing `fanout` unit tests stay green
  throughout the design pass. Implementation slice adds the
  three INTD-015 cases above + a default-deny pin on missing
  originator.
- **Confidence:** low (until design pass) — the wiring is
  understood, but the cross-session-attribution shape
  interacts with MLP2-025's spoof cross-check, MLP2-014's
  per-task fence isolation, and the v0.6.0-beta §H2
  per-startup HMAC follow-up, so the contract has to land
  first.
- **Priority:** Medium — operator-visible (the documented
  flag does not currently produce the documented behaviour),
  but the safe default keeps it cold-path.
- **Dependencies:** MLP-014 (multi-session + per-task fence
  isolation, Complete), INTD-015 (Complete in
  `plans/archive/modules/intercept-daemon.aps.md`), MLP2-070
  (lineage anchor daemon-derivation — the spoof-cross-check
  hardening this design pass interacts with), per-startup
  HMAC salt tracked in `v0.6.0-beta-security-note.md` §H2
  follow-up.
- **Source:** Release council pass 1, 2026-05-20
  ([`plans/reviews/release-council/2026-05-20-v0.7.0-beta-pre-tag.md`](../reviews/release-council/2026-05-20-v0.7.0-beta-pre-tag.md))
  defer-with-issue verdict on
  [#1722](https://github.com/eddacraft/anvil-001/issues/1722)
  ("INTD-015 — Fanout cross-session policy unreachable").
  Filed in MLP2 rather than `intercept-daemon.aps.md` because
  the canonical INTD module is archived at 16/16 Complete
  (`plans/archive/modules/intercept-daemon.aps.md`); MLP2 is
  the active home for daemon integration debt.

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| A. Daemon enforcement + observation | 10 (MLP2-001..-010) | 4/10 |
| B. Witness chain extensions | 5 (MLP2-011..-015) | 5/5 (Complete) |
| C. L4 policy execution | 7 (MLP2-016..-022) | 6/7 |
| D. Multi-session + fence isolation | 6 (MLP2-023..-026 + MLP2-025b + MLP2-025c) | 5/6 |
| E. Cross-platform attribution | 2 (MLP2-027..-028) | 0/2 |
| F. TypeScript driver-client mirrors | 2 (MLP2-029..-030) | 2/2 |
| G. Baseline + identity wiring | 6 (MLP2-031..-036) | 5/6 |
| H. Hook + config surface completion | 5 (MLP2-037..-041) | 5/5 (Complete) |
| I. GitHub Action publishing | 6 (MLP2-042..-047) | 1/6 |
| J. Protection-claim render conformance | 10 (MLP2-048..-052 + MLP2-051a..-051e) | 8/10 |
| K. Kindling activation orchestrator | 4 (MLP2-053..-056) | 4/4 (Complete) |
| L. Production hardening (Council follow-ons) | 4 (MLP2-057..-060) | 4/4 (Complete) |
| M. Full-codebase Council corrective follow-ons | 6 (MLP2-061..-066) | 6/6 (Complete) |
| N. Daemon evaluator host (GV2 groundwork) | 1 (MLP2-067) | 0/1 |
| O. MLP2-016 audit follow-ons | 2 (MLP2-068..-069) | 1/2 |
| P. v0.7.0-beta release-council follow-ups | 2 (MLP2-070..-071) | 0/2 |
| Q. New-user journey audit follow-ups | 2 (MLP2-072..-073) | 2/2 (Merged — PRs [#1819](https://github.com/eddacraft/anvil-001/pull/1819), [#1821](https://github.com/eddacraft/anvil-001/pull/1821)) |
| R. v0.7.0-beta release-council follow-ups | 1 (MLP2-074) | 0/1 |
| **Total** | **81** | **62/81** |

## Recommended landing order

The 66 items have natural sequencing through their `Dependencies:`
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

### Q. New-user journey audit follow-ups (2026-05-21)

Two MCP-surface findings raised by the
[2026-05-21 new-user journey audit](../audits/2026-05-21-new-user-journey-audit.md).
Neither blocks the `v0.7.0-beta` tag — they document discrepancies between
the marketed pre-write catch path and what a brand-new install actually
experiences.

#### MLP2-072: MCP `anvil_validate_write` blocks every write when unauthenticated

- **Status:** Merged via PR [#1819](https://github.com/eddacraft/anvil-001/pull/1819) (merged 2026-05-21 at `18c899bb`)
- **Tracking:** GH issue [#1796](https://github.com/eddacraft/anvil-001/issues/1796)
- **Intent:** After `anvil start` installs the MCP entries in
  `~/.cursor/mcp.json` / `~/.claude.json`, the MCP server currently
  returns `decision: block, code: authentication-required` for every
  `anvil_validate_write` call until the user runs `anvil auth login`.
  Agents that honor the server's own published instructions ("Honour
  `block` decisions; do not bypass them via alternate write tools")
  then refuse to write any file at all. The marketed save-time
  governance promise becomes a hard wall on first install.
- **Expected Outcome:** The pre-write gate distinguishes
  *gate-unavailable* from *content-veto*. Two reasonable shapes for the
  fix, either acceptable:
  1. When auth is missing, return `decision: allow` with a `degraded` /
     `gateUnavailable` flag that agents can surface as a warning
     without refusing the write.
  2. Introduce a `gateUnavailable` decision distinct from `block`, and
     update the server's `initialize` `instructions` so agents only
     honor `block` when accompanied by diagnostics.
- **Evidence pointer:** `crates/anvil-cli/src/commands/mcp.rs:380-415`
  (the `mcp_tool_auth_ok` gate + the auth-required response payload at
  `:493`).
- **Dependencies:** None internal to MLP2. Coordinates with FLAGCAT-008
  if `welcome` / `status` / `check` come off the licence gate (then the
  MCP decision shape may want to match the CLI's planless posture).
- **Validation:** Drive `anvil mcp serve --stdio` from a JSON-RPC harness
  without credentials and assert the new decision/flag shape; existing
  tests at `crates/anvil-cli/src/commands/mcp.rs:660-680` should be
  updated alongside.

#### MLP2-073: Pre-write `summary.total` double-counts identical diagnostics

- **Status:** Merged via PR [#1821](https://github.com/eddacraft/anvil-001/pull/1821) (merged 2026-05-21 at `15a397bd`)
- **Tracking:** GH issue [#1799](https://github.com/eddacraft/anvil-001/issues/1799)
- **Intent:** A single hardcoded secret on one line currently returns
  `summary.total = 2` with two diagnostics that share the same `id`,
  same `location`, and same `summary`. The dispatch path is emitting
  the same finding twice before summarising.
- **Expected Outcome:** Diagnostics are deduped by `id` (and
  defensively by `(rule_id, location)`) before the summary is
  computed; a single planted `sk-…` literal returns `summary.total = 1`.
- **Repro:** See the audit harness in
  [`plans/audits/2026-05-21-new-user-journey-audit.md`](../audits/2026-05-21-new-user-journey-audit.md)
  — call `anvil_validate_write` with `operation: "update"` against
  `src/smelly.ts` containing one hardcoded API key.
- **Validation:** Regression pin in
  `crates/anvil-cli/src/commands/mcp.rs` tests that fixtures a single
  secret-detection finding and asserts the JSON-RPC response has
  `summary.total == 1` and unique-by-`id` diagnostics.

#### MLP2-074: Daemon-side `session.report_process` IPC handler

- **Status:** Ready
- **Tracking:** GH issue [#1827](https://github.com/eddacraft/anvil-001/issues/1827)
- **Source:** v0.7.0-beta pre-tag release council `council-a1e2648f`
  (2026-05-21) action A2; council verdict at
  [`plans/reviews/release-council/2026-05-21-v0.7.0-beta-pre-tag.md`](../reviews/release-council/2026-05-21-v0.7.0-beta-pre-tag.md).
- **Intent:** `anvil-run` invokes the daemon JSON-RPC method
  `session.report_process` to report the child process's PID and start
  time after launch. The daemon dispatch table at
  `crates/anvil-intercept/src/ipc.rs:2431` has no handler; the daemon
  returns `-32601 Method not found`. `anvil-run` absorbs the error and
  proceeds (`crates/anvil-run/src/spawn.rs:102-128`), but the child's
  `pid_starttime` never reaches MLP-014's PID-reuse defence. The
  daemon's lineage anchor remains the launcher's `pid_starttime`,
  narrowing the cross-check from agent process to wrapping launcher.
- **Expected Outcome:** Daemon accepts `{ session_id, pid,
  pid_starttime }`, verifies peer credentials match the launcher's
  session, updates the registry's lineage anchor to the child's
  `(pid, pid_starttime)`, and returns success; on validation failure
  returns a typed error the launcher can log instead of the generic
  `Method not found`.
- **Validation:** Regression test pinning `pid_starttime` propagation
  through the wire format; `anvil-run --tool claude-code -- true` no
  longer prints the `Method not found` warning to stderr.
- **Dependencies:** None — the launcher already sends the right wire
  shape; only the daemon dispatch is missing.

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
