# Engineering History

Technical release history for engineers, platform teams, and technical
evaluators.

This log covers architecture, infrastructure, reliability, security, and
delivery changes behind each release. For end-user feature summaries, see the
[Changelog](./CHANGELOG.md).

## [Unreleased] — Draft — Config unification, MCP live-heal, and product deep clean

Draft / unreleased. Technical work landed on `main` since `v0.9.4-beta`. The
locked `v0.9.5-beta` claim is **config unification and product deep clean**
([ADR-120](./plans/decisions/120-config-surface-consolidation.md), UCFG-001..016
on `main`) plus the **MCP live-heal** operator path (MCPLH-001..006 on `main`;
001/002/004 Merged, 003/005/006 code landed). Supervisor/proxy (MCPLH-007) stays
Draft until re-exec soak evidence. Settings (`SETCON`+) and Graph Trust Surfaces
remain beside this window, not the cut claim.

### Config surface consolidation (ADR-120, UCFG)

- **One project-config store.** `anvil init` writes canonical `.anvil.yaml` /
  `.json` / `.toml`. `.anvilrc` is read-only fallback; no command creates one.
  `anvil migrate format` and `anvil config convert --to` share a writer that
  never emits `.anvilrc`. TTY `anvil doctor` offers migrate / remove leftover
  dual-config, fold `.anvil/gate-config.json`, or record `architecture.source`
  (UCFG-015/016).
- **Canonical `snake_case` keys.** Owned writes emit `schema_version` and
  friends; camelCase still loads. Registry + migrate/set/init share one
  normalisation path (UCFG-003).
- **`gate` and `architecture` as sections.** Gate composition lives in the
  project file; `.anvil/gate-config.json` is folded, not authoritative
  (UCFG-004/005). Architecture is inline or `SectionOrSource` delegated
  (UCFG-006/007). Gate / watch / architecture commands read the resolved section
  (UCFG-008); watch-time enforcement for section configs plus a descriptor-bound
  reader so a FIFO config cannot hang the process (UCFG-013/014).
- **One discovery layer.** Policy discovery, MCP resources, config summary, and
  doctor inspect the same `anvil_config::discover` surface (UCFG-009/010). Docs
  and fixtures pinned to the canonical layout (UCFG-011/012).
- **Legacy read fallbacks stay** for at least one minor (ADR-120). Dual-truth is
  a doctor warning, not a silent merge.

### MCP live-heal (MCPLH)

- **PATH-stable install (MCPLH-001).** Managed entries write `command: anvil`
  - `mcp serve --stdio`, not a Homebrew Cellar / versioned absolute. Handshake
    rewrites owned path-drift. `--command` remains for side-by-side.
- **In-process re-exec (MCPLH-002).** Unix `mcp serve` `execve`s the preferred
  binary between JSON-RPC frames (`initialize` / `tools/list` / `tools/call`).
  Anti-loop `ANVIL_MCP_REEXECED`; kill-switch `ANVIL_MCP_NO_REEXEC`. Windows
  demotes to honest skew reporting.
- **Operator cascade (MCPLH-003).** `anvil mcp refresh` rewrites owned configs,
  recycles a skewed daemon (`--daemon auto|restart|reuse`), bumps an
  install-scoped generation so live children re-check, and reports. `--dry-run`
  / `--json`. Default `--processes report` never signals live parents.
- **Daemon recycle (MCPLH-004).** Bare `anvil`, `anvil start`, and refresh stop
  a version-mismatched intercept daemon, wait for PID exit, start the current
  binary. Harness MCP children are not signalled.
- **Split readiness (MCPLH-005).** `anvil status` / `--verify` expose MCP
  process inventory, `mcp_skew`, and separate `protecting` / `agent_ready` /
  `graph_ready`. `agent_ready` is pre-write attach + current MCP, not graph
  (spec OQ-4).
- **Opt-in orphan reap (MCPLH-006).** `--processes orphan-reap` SIGTERMs
  same-user `anvil mcp serve --stdio` children whose parent PID is gone (Unix;
  same-uid gate). `force-skewed` is rejected.

### Honesty, reliability, and adjacent engineering

- **Deep-clean residuals.** Claude project MCP writes `.mcp.json`; live
  validation is per MCP client; capsule verify binds the witness to the attested
  range; refresh-token rotation takes an exclusive lock; policy-suite timeout
  kills the process group.
- **Check precision.** PY-008 no longer treats a dotted `compile` receiver as
  the builtin and does catch f-string / attribute forms. Credit-card rules
  ignore 16-digit runs in `https` URL paths (host required). Compiled-registry
  parity is a real CI gate. Threat-model rule tests are labelled data.
- **Windows / install honesty.** `anvil update` exits non-zero on decline and
  prints only the WinGet/installer remedy that matches `--check`. Install
  symlink-swap races closed on Windows and delegated config sources.
- **Docs freshness (ADR-119 / DOCFRESH).** Report-only docs-owed probe,
  granularity split, and CI on any code change.
- **Account activity (BACT) and DEVACC.** Beta account-activity metrics and
  admin surfaces; developer-acceleration bench suite (agent-free Tier B).
- **Acknowledgements starter.** Cold-adopt across ecosystems; Apache-2.0 grant
  on the kit.

See [RELEASE-PLAN.md](./RELEASE-PLAN.md) for the `v0.9.5-beta` cut bar.
Remaining cut work is changelog promotion and the standing release bar, not more
claim implementation.

## Interim published tags (not backfilled here)

This log was last drafted for the `v0.9.0-beta` window and was **not** promoted
at that cut or at the later patches. Customer notes and operator records are
authoritative; do not treat the missing sections as “those tags did not ship”.

| Tag           | Date       | Theme                                  | Record                                                                  |
| ------------- | ---------- | -------------------------------------- | ----------------------------------------------------------------------- |
| `v0.9.4-beta` | 2026-08-10 | Install honesty + quieter false alarms | [CHANGELOG](./CHANGELOG.md) · [record](./plans/releases/v0.9.4-beta.md) |
| `v0.9.3-beta` | 2026-08-07 | Honesty + Windows path                 | [CHANGELOG](./CHANGELOG.md) · [record](./plans/releases/v0.9.3-beta.md) |
| `v0.9.2-beta` | 2026-08-03 | MCP 2.0 reconnect                      | [CHANGELOG](./CHANGELOG.md) · [record](./plans/releases/v0.9.2-beta.md) |
| `v0.9.1-beta` | 2026-08-02 | Daily path + MCP 2.0                   | [CHANGELOG](./CHANGELOG.md) · [record](./plans/releases/v0.9.1-beta.md) |

`v0.8.1-beta` (2026-06-11, headless GitHub login) and the `v0.8.2-beta` Windows
daemon-ensure hotfix (2026-06-22, not a promoted headline) also shipped without
a section in this file. See [CHANGELOG](./CHANGELOG.md) and
[v0.8.1-beta](./plans/releases/v0.8.1-beta.md).

## [0.9.0-beta] — 2026-07-12 — First-Run Wins and the Assistant Graph

Shipped 2026-07-12. Technical work landed on `main` since `v0.8.1-beta` (the
pre-cut draft said `v0.8.2-beta`; that hotfix tag is not a promoted window).
This is the `v0.9.0-beta` "Assistant-Facing Graph" window, delivered end to end:
Graph V2 Phase 1 substrate and the multi-graph registry complete (GV2 Done,
21/21), the **GCTX assistant-facing graph** ships its full tool and resource
surface behind a sealed daemon-side egress projector with the CE-1..CE-12
privacy gates, a new **call graph** (GCALL) lifts cross-symbol call edges at
save time, **Python** joins the first-tier languages (PYLAN), the daemon gains a
**full-scan executor** and opt-in **warm-start persistence**, the **USAGE/KDS**
analytics foundation lands on-device, four **governance scan surfaces** (Docker,
GitHub Actions, shell, SQL) graduate, and CIB-071 migrates user warnings to
`miette` with spans. The ADR-075 entry gates cleared 2026-06-15 (ADR-083 + the
PV-9 egress review); the scoped feature work completed over the following two
weeks. Record: [plans/releases/v0.9.0-beta.md](./plans/releases/v0.9.0-beta.md).

### Graph V2 consumer layer and multi-graph registry (GV2-013, GV2-014, GV2-020, GV2-023, GV2-026, GV2-030, GV2-031, GV2-032)

- **GraphRegistry + WorkspaceRoot bridge (GV2-020).** New `GraphRegistry`
  skeleton, join stubs, `WorkspaceRoot` bridge, and seal proof. E2E
  certification through the registry path; council review addressed on the
  execution plan (renamed to canonical `.actions.md`). GV2-020 marked Merged via
  #2622; count to 19/20 then 20/20.
- **Consumer query contract (GV2-023).** Defined the governed consumer query
  contract for weave/MCP surfaces. Citation alignment to Merged-via form;
  promoted Ready alongside GV2-020 after GCTX gates cleared.
- **Control/session and plan/provenance graph contracts (GV2-013/014).** Phase 1
  contracts defined and landed; GV2 count reconciled to 14/20 then 15/20 (Phase
  1 complete note). Provenance-join fixture made anchor-only.
- **Runtime reverse-impact hop-depth lever (GV2-026).** Exposed as a
  configurable lever for hot-path reverse closure; marked Ready/Merged with
  count updates.
- **Sealed-DTO graph snapshot + no-leak guard (GV2-030).** `anvil-graph-cache`
  snapshot projection tightened, `forward_edges` visibility reduced, sealed DTO
  with runtime guard. Promoted Ready 2026-06-13 (post ADR-069 persistence and
  PV-6..12). Snapshot DTO rustfmt + tests.
- **Re-export edges for transitive privilege (GV2-031).** Lifted re-export
  modelling so privilege is correctly transitive. NBI row scheduled; module
  narrative reconciled; marked Merged via #2627.
- **Symbol spans + per-file content hash (GV2-032).** The deferred GV2-010 span
  plumbing gained its producer — `SymbolNode.span` plus a per-file content hash
  populated through `apply_delta` — the substrate the GCTX snippet line needs to
  locate and freshness-check source. Closes GV2 at **Done, 21/21**.
- **Graph boundary and cache refinements.** `anvil-graph-cache` crate (ADR-064)
  continues to serve as the daemon graph boundary; kernel symbol feed and
  per-worktree caches remain in use. Privacy review for machine-local
  persistence (prior PV) carried forward.

### GCTX — the assistant-facing graph (GCTX-001..032)

- **ADR-083 Accepted (2026-06-15).** GCTX-002: assistant graph context delivery
  (tools/resources/slicing) targets the Rust `anvil mcp serve` (RMCPF) surface
  as primary/long-term mechanism. Additive registration through existing
  capability handlers; consumes daemon hot-read paths; no resurrection of
  archived TS MCP for production GCTX. Interim TS shim only for migration
  smoke-tests.
- **Context-egress privacy review (PV-9) filed (verdict: approve with
  conditions).** Conditions (CE-1..CE-12) folded into GCTX-001. Complements the
  earlier GV2 machine-local persistence review.
- **GCTX-001 assistant graph projection contract authored.** Captures NBI, agent
  workflows, and update discipline. Promoted Draft → Ready; GCTX-001 + GCTX-002
  marked Merged (GCTX 2/13).
- **GV2-020/023 + GCTX module Ready promotion.** Post-gate:
  `graph-context-delivery` module promoted Ready (0/13 work items); dependent
  consumer surfaces unblocked. APS index and module narrative updates
  (bullet-list brief metadata, upstream consumer brief).

- **Sealed egress projector + CE-1..CE-12 spine (GCTX-010 pilot, ADR-084).** All
  GCTX answers are produced daemon-side by a single `GctxProjector` choke point
  that emits a sealed, identity-only egress DTO with a structural no-leak test
  (CE-5). The MCP tools hold no graph — they forward over `anvil/gctx/*` RPC and
  receive the sealed DTO (ADR-084). Identity-only is the default (CE-1); opaque,
  fingerprinted pagination cursors with input caps (CE-6, ADR-091); enum outcome
  telemetry with no PII (CE-10) and the `ANVIL_GCTX_EGRESS` kill-switch re-read
  per call (CE-11).
- **Phase 1 query tools (GCTX-010..014).** `anvil_search_symbols` (identity
  search), `anvil_find_dependents` (file-keyed reverse closure with hop
  distance), `anvil_find_callers` (symbol-level caller traversal, `heuristic`/
  `partial` flags over the GCALL read API), `anvil_impact_of_change` (change-set
  blast radius: affected symbols + dependent files + heuristic known tests) and
  `anvil_affected_tests` (test attribution + coverage gaps). All depth-bounded
  at the production `MAX_REVERSE_IMPACT_DEPTH = 2`.
- **`graph://` resources (GCTX-030).** Identity-only `graph://stats` / `symbols`
  / `edges` resources with CE-6 pagination, a `bounded` edges flag, a shared
  per-session egress byte budget, and warm-on-`NotReady`.
- **Context slicing (GCTX-020..023).** A parser-free conservative token
  estimator (`gctx-simple-v1`), a daemon-side snippet extractor + budget slicer,
  and the `anvil_symbol_context` tool — neighbourhood + importers + callers with
  span-as-location and optional source snippets. Snippet egress is double-gated
  (`ANVIL_GCTX_EGRESS=1` **and** per-request `includeSource`) and runs a
  deny-by-default sensitive-path/gitignore/secret-scan pipeline before any text
  is emitted (CE-2/CE-3); a later pass hardened those gaps.
- **Benchmark + guide (GCTX-031, GCTX-032).** A deterministic `token_reduction`
  bench (in `anvil-bench`) measuring the real `ImpactOutcome` payload against
  naive file-reading over the 2-hop closure (golden-pinned), and the user-facing
  `docs/guides/ai-context-delivery.md` guide. Module closed at **14/14**.
- **C1 cold-start warm-up.** A fresh MCP session triggers an on-demand warm-up
  on `NotReady`, layered on the DSV-045 full-scan executor (below).

### GCALL — resident call graph (GCALL-002..006)

- **Cross-symbol call edges at save time.** Call-site extraction into
  `FileSymbols` for TS/JS (GCALL-002), Rust (GCALL-004), and Python (GCALL-005),
  with resident call edges and a `callers_of` read API (GCALL-003) carrying a
  best-effort `heuristic` marker (dynamic dispatch and overload fan-out are
  over-approximated, never claimed authoritative). A save-time call-lift latency
  gate (GCALL-006) keeps the extraction inside the ADR-031 budget. This is the
  substrate `anvil_find_callers` projects.

### Daemon full-scan executor and warm-start persistence (DSV-045, DSV-030)

- **Full-scan executor (DSV-045, ADR-085).** `anvil/request_full_scan`
  previously only set `Pending`; a real executor now populates the warm graph
  (cancel + rewarm on DSV-006 primitives, injected parser on the background
  scheduler per ADR-064, dirty-flag compare-and-clear fences against save-time,
  per-key CAS coalescing). This is what lets a cold MCP session warm on demand;
  it added an `AssuranceState::Bounded` variant with a `serde(other)` fallback
  to keep the wire backward-compatible.
- **Warm-start persistence (DSV-030, ADR-061 Sub-phase B / ADR-069).** Opt-in,
  per-uid, default-off snapshot persistence so a restart re-warms from disk
  rather than a cold rebuild. Sub-phase B reconciled 2026-06-24. Surfaced to
  users via the `ANVIL_PERSIST_GRAPH=1` opt-in and the one-time rebuild upgrade
  note.

### Python language anchor and tail languages (PYLAN, LANGTAIL)

- **Python first-tier support (PYLAN-001..009).** A Python grammar + extractor
  anchor, an anti-pattern catalogue and `.py` drift scan, entry-point detection,
  and layer/boundary enforcement — with external-codebase validation driving the
  Python false-positive rate to 0%. Python now participates in the resident
  graph (symbols + call edges).
- **Tail-language T1 wave (LANGTAIL).** A further wave of tail languages reached
  first-tier (`is_parseable`) parsing via `Language::from_path`.

### Governance scan surfaces (SURFDOCK, SURFGHA, SURFSH, SURFSQL)

- **Four infrastructure-hygiene catalogues.** Dockerfile build-hygiene
  (SURFDOCK), GitHub Actions supply-chain (SURFGHA), shell-script (SURFSH), and
  SQL-migration destructive/idempotency (SURFSQL) surfaces, each wired into the
  gate behind a per-surface flag and opt-out, with `drift` baselines so the gate
  warns only on new edges. Track 3 governance surfaces graduated to
  **default-on** with a `=0` opt-out contract (the gate reads the leaf flags,
  not the umbrella).

### Usage analytics foundation (USAGE-001..005) and the daemon sink (KDS-001..005)

- **Command-invocation Kindling observations (USAGE-001).** Every CLI entrypoint
  and JSON-RPC dispatcher emits a durable `command.invoked` (or equivalent)
  Kindling row carrying: command, anonymised principal, timestamp, redacted
  argument shape (names only), inline `flag_set` context (ADR-041), and
  `traceparent`. Privacy contract re-uses existing `SENSITIVE_FIELDS` deny-list;
  Copilot review addressed (posture hardening); tests avoid fixed salts in
  anonymisation. Merged via #2603.
- **Flag context inlining (USAGE-002).** `flag_set` resolved from auth/routing
  surfaces inlined onto usage rows. Merged via #2607.
- **Operator query views (USAGE-003).** New `anvil kindling usage` canned views
  for top-N commands, never-invoked commands, and flag-dependent path exercise.
  Runbook and query surface added.
- **Licence gate flag-driven (USAGE-005).** CLI licence enforcement surface made
  controllable via the single-source flag catalogue (no behaviour change for
  default-on users).
- **JSON-RPC command-invocation producer (USAGE-004).** The save-time daemon's
  JSON-RPC dispatch emits the same `command.invoked` signal, with a
  user-initiated method allowlist and an optional principal extracted from the
  envelope.
- **Daemon usage sink (KDS-001..005).** A `KindlingDaemonSink` routes
  `command.invoked` through the running daemon
  (`ANVIL_KINDLING_SINK=daemon|ndjson|off`), direct Rust emit via
  `kindling-client`, daemon-side sink selection, and (on `kindling-client` 0.3)
  reads the `anvil kindling usage` views back from the daemon — unioned with the
  local sidecar and scoped by workspace root. KDS-005 retired the legacy
  `DaemonUsageSink` and flipped the default to the daemon sink.
- **Cross-cutting module hygiene.** Follows ADR-034 convention; anti-drift hook
  maintained; privacy and redaction alignment with observability-foundation.

### Diagnostics migration (CIB-071)

- **Miette diagnostic rendering for warnings (Phase A).** User-facing
  diagnostics (warnings path) migrate from `anyhow`/`eyre` strings toward
  `miette::Diagnostic` + `SourceSpan`/`LabeledSpan`. Phase A lands the renderer
  and basic mapping while preserving original message + spacing.
- **Source-span excerpts in warning diagnostics (Phase B).** Adds accurate
  source excerpts to warnings. Full PR #2611; status reconciled in APS/CIB
  index; `resource-budgets` and cross-matrix considerations noted for future
  impact measurement. Fixes include miette test mapping preservation.
- **Standing CIB tracking.** CIB-071 status flipped In Progress → progress
  recorded; related Windows `ready_restart_required` (CIB-072) and
  demo-readiness (CIB-073..076) plus preflight ordering (CIB-077) filed as
  follow-ups.

### Secret scanning hardening

- **Lockfile and .env handling.** Lockfiles are now inspected only for
  high-signal URL credential shapes (no blanket skip, no broad keyword blast).
  Local `.env` (and lockfiles) are excluded from low-confidence keyword rules
  that historically produced noise. Comment corrections for `.env`/lockfile
  intent; new portable regression tests.
- **Refactor + fix sequence.**
  `refactor(secret-scan): scan lockfiles for URL credentials, not skip`;
  `fix(secret-scan): stop flagging lockfiles and local .env files`.

### Daemon lifecycle and intercept foundations (DLIFE)

- **Tiered daemon startup shipped (DLIFE-003/004, ADR-082).** `anvil start` now
  auto-starts the save-time daemon and `anvil watch` prompts to start it
  (falling back gracefully), on an idempotent daemon-`ensure` primitive. Plus
  `anvil intercept stop`, a macOS process start-time probe via `proc_pidinfo`
  (V060F-004), and per-worktree daemon health envelopes (CIB-098, ADR-090).
- **DLIFE-006.** Terminating `--verify` diagnostic copy corrected.
- **Intercept ref hygiene.** Stale `INTD-002` references repointed to the DLIFE
  module. UUID fallback clarification for unparseable `traceparent` in intercept
  telemetry.
- **Telemetry join (MLP2-008).** `gate_evaluated` row now receives RTAI-007
  telemetry join; MLP2 count advanced.

### Graph cache, snapshot, and hot-path refinements (supporting GV2)

- Tightened snapshot projection and `forward_edges` visibility.
- Runtime hop-depth lever wired for reverse-impact queries.
- Sealed DTO guard to prevent accidental leakage of internal graph shape across
  the cache boundary.
- Catalogue inventory test updated for `daemon.persist-graph` flag.

### Release engineering, CI, APS & hygiene

- **Window reconciliation.** Across the window the graph modules closed out: GV2
  **Done 21/21**, GCTX **14/14**, GCALL **7/7**, USAGE **5/5**, KDS **5/5**,
  with continuous APS index/Merged-via reconciliation and archive of
  prior-window modules. The all-Merged modules stay `In Progress` pending the
  `v0.9.0-beta` tag (the closeout cascade advances them to Complete + archive).
- **Release Drafter (#2691).** Automated, label-driven draft release notes wired
  in, complementing the hand-curated changelog + engineering history.
- **Canonical APS dev loop (#2955).** Installed the canonical APS loop skill set
  and retired the fable bindings.
- **MCP server surface.** `anvil://` resources ported to the Rust MCP server
  (RMCPF-020); the MCP-optional activation spine (ACTMO-001..010) decouples
  activation from the MCP path. API-stability markers + a warn-only CI check
  (TUIN-006) and output-mode probes (TUIN-003) landed alongside.
- **CI/mirror hardening.**
  `ci(mirror): harden eddacraft-tui + acknowledgements mirror workflows`
  (#2620).
- **Docs lint / format remediation.** Oxfmt applied to RELEASE-PLAN.md;
  ADR-033/ADR-083 wording and GCTX-002 state corrected per Copilot review.
- **CIB intake.** New items filed for Windows daemon restart stuck state, beta
  demo gaps + script, release preflight version-gate ordering.
- **Infra.** `.deepsec` excluded from Nx graph to unblock Vercel docs-shell.
- **Deps & acknowledgements.** Hakari + acknowledgements refresh; esbuild floor
  raised in prior window carry-over.
- **Auth / API follow-through.** GitHub OAuth app org-ownership clarified; admin
  key handling hardened; Resend probe and health work from prior window
  stabilised.
- **APS index & module state.** Continuous index refreshes, CIB count bumps,
  USAGE/MLP2/GV2/GCTX state alignment before and after the window transition.

Shipped as [v0.9.0-beta](./plans/releases/v0.9.0-beta.md). Later patch themes
are listed in the interim table above; do not read this section as the current
window.

## [0.8.0-beta] — 2026-06-11 — The Save-Time Daemon

Shipped 2026-06-11. Technical work landed on `main` since `v0.7.4-beta`. The
headline is architectural: save-time governance starts moving off per-save
cold-spawned `check` and onto a persistent intercept daemon that validates
deltas ([ADR-061](./plans/decisions/061-save-time-daemon-delta-validation.md),
Accepted 2026-06-01). Sub-phase A and the A-W Windows parity slice are now
merged; A′/B remain sequenced behind Graph V2 foundations.

### Daemon save-time validation (DSV / ADR-061, ADR-063, ADR-064, ADR-067)

- **Verdict-shaped `validate_paths` wire frozen (DSV-002).** The intercept
  protocol owns a frozen, verdict-shaped `validate_paths` request/response wire
  with a `check_families: ["antipattern"]` scoping and `coverage: certified`,
  keeping the policy engine off the hot path (ADR-061 §6).
- **Daemon ingest spine (DSV-003).** Read-safety, classification, taxonomy, and
  admission components for the daemon ingest path, with session-bound
  `scan_buffer` lineage (CLAWP-065).
- **Interactive + background work pools (DSV / ADR-061 §4).** An admission token
  separates interactive from background work; the background scan loop yields in
  bounded chunks (chunked-yield, final-chunk cancel yields `Completed` not
  `Yielded`).
- **Kernel-backed symbol feed (DSV-005, ADR-067).** A parse hook feeds the
  kernel symbol graph; a per-worktree `SymbolGraph` cache (`kernel_cache`) backs
  it, with bounded reverse-impact closure certifiability.
- **`validate_paths` verdict assembly (DSV-005 Task 8).** Boundary mappings wire
  into `anvil-checks`; verdict-assembly orchestration and assurance-transition
  notification envelopes complete the hot-path response.
- **Save-time verbs over the IPC socket (DSV-005/-007).** Save-time verbs are
  wired into IPC dispatch; `anvil watch` routes its save-time check through the
  daemon, MCP `validate_write` re-points to the daemon, and `anvil status`
  surfaces save-time assurance + confinement. Mid-edit decisions also mirror
  onto the telemetry lane (drivers→daemon path).
- **Workspace assurance state machine + confinement (DSV-005 Task 9, DSV-008).**
  A workspace assurance state machine drives transitions; an opt-in workspace
  confinement mode (ADR-061 §7) anchors admitted roots by dirfd, path-keyed.
- **DoS caps + SLO gate (DSV-006).** Per-workspace parse-size and walk-depth
  caps bound daemon work; a `validate_paths` concurrency SLO bench gates in CI
  via the (non-required) `resource-budgets` job.
- **Cross-path antipattern parity gate (DSV-009).** A parity gate plus a shared
  diagnostic sort keep the daemon's cross-path antipattern verdicts aligned with
  the cold-path `check`.
- **Windows save-time parity (DSV-010/-011, ADR-068/-070).** The named-pipe
  daemon path now serves the save-time verbs on Windows behind guarded reads,
  peer-SID auth, owner-only operator-config trust, client timeouts, and
  off-reactor peer checks. Windows `watch`/`status` clients are no longer inert
  stubs; closure evidence came from the green cross matrix after the bench-flake
  fix in PR #2365.
- **Graph boundary ADRs.**
  [ADR-063](./plans/decisions/063-gv2-hot-path-boundary.md) closed the Graph V2
  hot-/non-hot-path read boundary (reverse-impact depth is a hard-capped lever)
  and [ADR-064](./plans/decisions/064-intercept-graph-cache-crate-boundary.md)
  extracted the `anvil-graph-cache` crate for the daemon graph boundary.

### Rust language coverage (RSTLAN-001…008, ADR-065, ADR-071)

- **Rust parser and graph substrate.** `tree-sitter-rust` is wired into the
  kernel, with Rust symbol/import extraction for `use`/`mod` shapes and Rust
  entry-point detection for binaries and workspaces.
- **Rust architecture enforcement.** `anvil gate` and the architecture validator
  consume kernel-supplied Rust edges; layer/boundary checks now resolve Rust
  crate/module paths and emit violations for new cross-layer Rust edges.
- **Rust default scan coverage.** `.rs` files are in the default
  antipattern/drift set. JS/TS-specific rules stay extension-restricted, while
  Rust deferred-debt checks and the Rust AST catalogue run on Rust surfaces.
- **Dogfood acceptance.** The Rust T3 pass on anvil's own crates covered 571
  files with no panics or parse skips and met the false-positive bar. Follow-on
  Rust catalogue expansions (`.clone()` hot-loop, serde flatten/secret-field
  shapes) are deferred rather than hidden in this release.

### Git-native governance and exceptions (ADR-072, ADR-073, ADR-074)

- **Git-native governance decision gate.** ADR-072 (Git substrate), ADR-073
  (`anvil/` durable state vs `.anvil/` local state), and ADR-074 (review capsule
  v0 format) were accepted via full council review. The result is an authorised
  file-first governance path without introducing a cloud-trust dependency.
- **Capsule manifest schema (GITGOV-003).** The new `anvil-capsule` crate owns a
  versioned `anvil.capsule.v1` manifest and `anvil.capsule-verification.v1`
  shape, with SHA-256 canonical-JSON file digests and witness-range pointers.
- **Capsule create command + collectors (GITGOV-004/-005/-006).**
  `anvil capsule create --range <base>..<head> --out <dir>` now writes a capsule
  directory from a real range. A commit/range collector resolves commits, tree
  hashes, parents, and changed paths (GITGOV-005); a policy/baseline/rules
  digest collector captures the governance inputs (GITGOV-006); both reuse
  existing crates rather than re-modelling evidence.
  `anvil_rules::OPA_RUNTIME_VERSION` is the shared constant (the hook aliases
  it) and the CLI fills `ToolIdentity`/`Producer` from a single binding. Witness
  and diagnostics capsule entries are structural placeholders until
  GITGOV-007/-008 land their collectors; `verify`/`explain`/`inspect` UX remains
  future work.
- **Tracked exception store foundation (EXCEPT-001/-002/-007).**
  `ExceptionStore` now prefers tracked `anvil/exceptions/store.json`, falls back
  to legacy `.anvil/exceptions.json`, and supports non-destructive migration.
  The write path is hardened for provenance, locking, read-only worktrees, and
  symlink escapes before any CLI caller is wired. Exceptions remain unenforced
  until the later EXCEPT evaluation and CLI items land.

### Graph V2 foundation (GV2-002, ADR-063, ADR-064)

- **Stable symbol identity + export-diff primitive (GV2-002).** The
  `anvil-graph-cache` crate replaces the position-conflated
  `symbol_baseline_key` (`file::kind::name`) and the session-local
  `SymbolNode.id` counter with a stable, cross-restart symbol identity
  (overload-disambiguated by structural refs only) plus an edge-identity
  derivation, so snapshots and deltas stay comparable across daemon restarts. It
  also lands the export-diff primitive that lets the save-time fast path
  graduate from "any touched public symbol → `partial`" toward a real
  added/removed/renamed-public-symbol diff. Renames are modelled as
  delete-old-id + create-new-id with no persisted rename history; the privacy
  review ([verdict](./plans/reviews/2026-06-08-gv2-privacy-review-verdict.md),
  PV-1..PV-5) cleared the identity contract — structural-only overload
  disambiguators, named deterministic content hashes, join-time-only
  session/provenance refs — before merge.

### Flags catalogue (FLAGCAT-002…006)

- **Single-source flag catalogue.** `anvil-flags-catalogue` (TS) bootstraps the
  gating inventories and renames the `EnvironmentName` enum (`prod` →
  `production`, etc.). TS surfaces migrate onto it (FLAGCAT-003); Rust constants
  are generated from `flags/manifest.json` via `build.rs` (FLAGCAT-004); the CLI
  flips onto the generated catalogue definition (FLAGCAT-005).
- **Drift gate + adoption guide (FLAGCAT-006).** A manifest↔TS↔Rust consistency
  check is CI-gated, with an adoption guide for new flags.

### Resource budgets (RLB-002…008)

- A process-tree CPU/RSS sampler underpins a suite of resource benches: the real
  default watch path under churn (RLB-002), intercept daemon (RLB-003) and MCP
  server (RLB-004) CPU/RSS budgets, and a concurrent multi-process bench
  (RLB-005). SLO docs + CI wiring land the `resource-budgets` job (RLB-008).

### TUI dashboard renderer (TUIDASH-003…013)

- A `json-render` tree renderer + spec parser drive spec-defined dashboards:
  layout (Stack/Grid/Card/Separator), data-display, and chart component maps;
  responsive breakpoints; a generic `$data` binding pre-pass; anvil domain
  components over `.anvil/` data context; a dashboard list with live previews;
  and a catalogue parity check against `@eddacraft/render`. The engine is
  hardened against malformed/hostile specs. The live gate-summary dashboard
  (TUIDASH-013) renders gate results persisted to `.anvil/gates.json`.

### GitHub CLI auth groundwork (GHCLIAUTH-001/-003, ADR-066)

- The GitHub device-flow replacement for headless SSH/tmux auth is partially
  prepared: `mintSession`/licence minting logic is extracted for reuse, and the
  API can link GitHub identities by stable numeric `github_id` without allowing
  a re-bind. OAuth-app provisioning, live device-flow endpoints, CLI polling,
  and confirm-endpoint removal remain gated follow-up items.

### Adoption insights

- **First-week adoption signal (INSIGHTS-004).** A first-run adoption hint,
  workspace-root resolved with atomic writes and insights-precedence in plain
  `status`.

### Build, CI & delivery

- Agent-tooling dirs (`.codex`/`.claude`/`.opencode`) classify as `agent-config`
  (no unit-test matrix); the flags-catalogue package is force-built before unit
  tests so the nx cache can't restore a build marker without `dist`.
- Cross-compile tests run with `--no-fail-fast`; the Windows cross-compile suite
  is green. `release.yml` allowlists CLI `v*` tags in the plan job. Copilot
  autofix is hardened against the untrusted-checkout alert (code-scanning #826).
- `RELEASE-PLAN.md` is enforced forward-looking (a `docs:check` surface);
  `eddacraft-tui 0.2.4` and `0.3.0` land with ACKNOWLEDGEMENTS / changelog
  release hygiene; Regal is pinned/bumped across workflows.
- The six-week minor-beta cadence hold is retired — minors cut when ready and
  gates are green, not on a calendar.

### Repository hygiene

- The clawpatch pre-tag tracker is dispositioned, closed, and archived
  (CIB-039); the CLAWP #1740 test-hardening batch hardens vacuous findings;
  inline TODO/FIXME comments are removed or converted to explanatory notes.
  Continuous-improvement items CIB-016/-027/-032/-035/-046/-047 land alongside.
- Documentation-governance closeout reaches the governed live-doc metadata
  backfill finish line (DOCGOV-011): governed non-public live docs are covered,
  generated indexes are refreshed, and the remaining source-path debt is linked
  to tracking issue #2371 rather than silently normalised.

## [0.7.4-beta] — 2026-06-01 — Side-by-Side Installs

A distribution and stability patch on the `v0.7.3-beta` slate. The headline is
the `ANVIL_HOME` install-root override (DISTRIB-006); it also lands the RLB-007
watch-CPU stopgap, Windows named-pipe hardening, and a set of CLI exit-code and
output-selector correctness fixes. The save-time daemon arc
([ADR-061](./plans/decisions/061-save-time-daemon-delta-validation.md)) was
scoped in this window but lands in `v0.8.0-beta`.

### Install-root override (DISTRIB-006 / ADR-060)

- **Single install root resolved from `--anvil-home` / `ANVIL_HOME`.** A new
  resolver re-roots the daemon socket, PID file, stored credentials, and all
  durable project-state writes under one install root
  ([ADR-060](./plans/decisions/060-anvil-home-install-root-override.md),
  Accepted), so a development or candidate anvil can run beside a production
  install without sharing or clobbering state
  ([#1726](https://github.com/eddacraft/anvil-001/issues/1726)).
- **Project-state writes are gated under the install root.** The state-mutating
  commands (`config`, `gate-config`, `hooks`, and the remaining project-mutating
  commands) gate their durable writes under the resolved root; Council review
  surfaced and closed several write-guard gaps. `anvil status --json` reports
  `install_root` and `project_writes_gated` so the active root and its gating
  are observable. A blank `ANVIL_HOME`/`--anvil-home` is treated as unset
  consistently rather than resolving to an empty root.

### Watch CPU stopgap (RLB-007 / RLB-001)

- **Per-save checks scope to the changed paths.** The watch save-time check no
  longer re-runs code-quality checks across the whole project on every save —
  under multiple concurrent agents the whole-project scan saturated CPU
  ([#2156](https://github.com/eddacraft/anvil-001/issues/2156)). This is the
  stopgap ahead of the daemon delta-validation fix in `v0.8.0-beta`.
- **Multi-agent watch load-ramp harness (RLB-001).** A bench harness ramps
  concurrent watch agents to measure the per-save CPU envelope that RLB-007
  collapses.

### Windows daemon hardening

- **Named-pipe client SQOS cap + server-liveness check.** The Windows named-pipe
  client caps its security quality-of-service impersonation level and verifies
  the server process is alive before connecting, closing two Windows-only daemon
  issues found in the clawpatch sweep. A regression test covers the
  exited-process liveness path.

### CLI correctness & exit-code fidelity

- **Credential load faults return `EXIT_CONFIG_ERROR`.** A failure loading
  stored credentials now surfaces as a configuration-error exit code instead of
  a generic failure.
- **The auth gate honours per-command `--format json`.** The auth gate could
  previously ignore a command's `--format json` selection; an e2e test now
  covers the structured auth envelope.
- **Event-shape invariants made unrepresentable.** `kernel-types` validates that
  an `EngineEvent`'s `event_type` matches its payload, and `intercept-rules`
  makes the 1-based `InterruptReason` line invariant unrepresentable rather than
  checked.

### Native build freshness (checks-napi)

- The `.node` freshness guard widened to cover all native build inputs, and only
  `ENOENT` is treated as an optional missing build input (other I/O read
  failures surface rather than silently degrading to a missing-registry
  contract).

### Build & delivery

- `aarch64-linux` NAPI artifacts build with the system cross toolchain;
  `edda-stack` derives `PACKAGE_VERSION`/`PACKAGE_NAME` from `package.json`; the
  `anvil-api` runtime guard is repointed off `svix`; and the production
  dependency group took an 11-update bump.

## [0.7.3-beta] — 2026-05-31 — Surfacing the Signal

Technical work landed on `main` since `v0.7.2-beta`. SARIF output (SARIFOUT) is
recorded in the [Changelog](./CHANGELOG.md) and its engineering notes are
tracked separately with that workstream.

### Language extraction (LANGTS)

- **TypeScript symbol coverage widened (LANGTS-002).** `interface`, `type`, and
  `enum` declarations now emit `Interface` / `TypeAlias` / `Enum` symbols
  (export-wrapped → `Public`), and class methods emit `Method` symbols named
  `Owner.method` so the owning class is recoverable without a structural parent
  edge. Adds `SymbolKind::{Interface, TypeAlias, Enum, Method}` (additive).
- **Language-extractor trait + grammar-versioned cache (LANGTS-005).**
  Extraction moves behind a trait with a grammar-versioned parse cache and a
  non-panicking parse path, so a malformed source file degrades to no-symbols
  instead of aborting the walk.

### Policy engine hardening (POLENG-009)

- Panics in the Rego evaluation path are caught at the regorus facade under a
  dedicated unwind profile, so a malformed policy can no longer abort the
  process.
- Added a determinism fence and input bounds on `anvil policy eval`, plus
  hardened findings parsing.
- Tracing now spans the policy eval path.

### Native TUI dashboards (TDASH / TUIDASH)

- `anvil dashboard` ships as a read-only TUI over persisted `.anvil/` state: the
  command plus picker scaffold (TDASH-001) and three wired surfaces —
  architecture-health (TDASH-002), drift-snapshots (TDASH-003), and
  suppressions-overview (TDASH-004).
- Dashboard rendering is built on a reusable `TuiComponent` trait +
  `TuiRegistry` (TUIDASH-002) and a json-render spec parser (TUIDASH-001).

### Secret scanning & finding fidelity

- The git-history secret scan was extended to on-disk (working-tree) coverage
  (EAMIG-004), with oversize-skip surfacing and portable test hooks.
- The scan path completed its migration onto the policy walker, and scanners now
  preserve distinct findings that share a line instead of collapsing them.

### Observability & tracing

- CLI tracing is routed to `stderr` rather than `stdout`, so machine-readable
  output on `stdout` stays clean.
- Added a TypeScript trace mirror with a redacting formatter; the API middleware
  sets response headers via `c.res.headers.set`.

### Activation hardening

- Workflow installs now require explicit operator consent, and workflow file
  writes are hardened against unintended overwrites.

### Discovery performance

- The `anvil welcome` Phase 1a discovery walk is parallelised (SCAN-006), backed
  by a new `walk_discovery` benchmark (SCAN-005).

### Build & delivery

- Added a cargo workspace version-match preflight gate, routed the release gate
  through `test:js` (off the cross-language critical path), and trimmed dev/test
  debug info to line-tables-only (DEVENV-001) to cut build size.

## [0.7.2-beta] — 2026-05-25 — Save-Time Scanning & Tooling Honesty

The second Boring-Week patch on the `v0.7.0-beta` daemon-working slate. The
customer-facing fixes (watch actually runs the code-quality scanners; the
antipattern scanner stops flagging `any`/`!` in comments and strings;
PATH-shadow and 90-day-refresh honesty) are in the [Changelog](./CHANGELOG.md);
the engineering weight is the experimental policy engine, the multi-ecosystem
attribution kit, and the `eddacraft-tui` reintegration.

### Save-time scanning correctness

- **`anvil watch` runs `anvil check --all` per save (#1913).** The default watch
  action now invokes the code-quality scanners, closing the gap where a bare
  `anvil watch` watched only architecture/dependency edges while the dashboard
  read "100% pass". `--action none` restores the edges-only watch.
- **Antipattern scanner masks comments, strings, and regex (#1914).** AP-003 /
  GS-001 mask comments, string literals, and regex literals before applying
  code-construct rules, so prose or string content mentioning `any` or
  containing `!` is no longer a finding. Match positions for genuine findings
  are unchanged.

### Policy engine (POLENG)

- **`anvil policy` command group + Rego eval preview.** A new policy surface
  lands across POLENG-002..007: a `PolicyInput` v1 schema, a determinism
  contract + `Builtin` trait, a first-party builtins surface (v1), ADR-002/003
  result post-processing, and `coverage`/`trace` result fields, exposed through
  the experimental `anvil policy eval` CLI.
- **Go OPA parity gate (POLENG-008).** A CI parity gate cross-checks the regorus
  evaluation path against Go OPA and is registered in the workflows README;
  POLENG-009 + CIB-017..019 were filed from the full council as follow-up
  hardening.

### Attribution kit (ATTRIB)

- **Multi-ecosystem licence attribution drivers.** A dispatcher + per-ecosystem
  drivers cover Rust (ATTRIB-008), Node (ATTRIB-012), Go (ATTRIB-013), and
  Python (ATTRIB-014), plus a bundled-binaries driver (ATTRIB-004) and a
  deterministic, GNU-compatible comment-wrapping expander (ATTRIB-015/-016).
  Node driver tests are wired into CI.

### eddacraft-tui reintegration (TUIR / ADR-047, ADR-050)

- **Canonical source mirror.** `eddacraft-tui` is imported at v0.2.2 as the
  canonical in-tree source (TUIR-002), consumers switch to the workspace path
  crate (TUIR-003), and CI gates split between anvil and the mirror (TUIR-006)
  with a mirror + crates.io publish workflow migrated onto a GitHub App
  (TUIR-004/-005).
  [ADR-050](./plans/decisions/050-eddacraft-tui-runner-and-cli-policy.md) scopes
  the runner-helper + CLI/parser policy.

### Activation & daemon hardening

- **`anvil start/status --verify --why` (MLP2-051g)** explains a stalled
  activation diagnostic, and the MCP `protection_claim` query is capped at 500
  ms (MLP2-051i), tightening the activation IPC budget.
- **Daemon-side `session.report_process` handler (MLP2-074).** The IPC handler
  unimplemented at `v0.7.0-beta`/`v0.7.1-beta` is now implemented, closing that
  known gap.

### Tooling, docs & delivery

- **APS plan dashboard (apscan).** `anvil plan dashboard` ships as an in-tree
  APS status surface; APS active-lint scope and a canonical-alignment module
  land alongside.
- **DOCGOV documentation indexes.** Generated documentation indexes (DOCGOV),
  dead-doc archival + runbook relocation (DOCGOV-008), and live-doc metadata
  backfill harden the docs-governance gates.
- **Release-announcement backend (EMAIL).** `anvil-api` gains a
  `/admin/broadcast` endpoint over a generalised broadcasts table, an email
  template registry, broadcast audience resolvers, and a
  `sendReleaseAnnouncement` helper.
- **Supply-chain hygiene.** `qs` is pinned to a patched release;
  ACKNOWLEDGEMENTS and workspace-hack are regenerated for the new
  policy/attribution trees.

## [0.7.1-beta] — 2026-05-22 — Activation Diagnostic Honesty

The first Boring-Week patch on the `v0.7.0-beta` daemon-working slate. It closes
GH [#1831](https://github.com/eddacraft/anvil-001/issues/1831): the activation
diagnostic could cap at `ready_restart_required` forever, with no path to
`protecting` even when the intercept daemon was running and enforcing the
worktree. The customer-facing wire-up is in the [Changelog](./CHANGELOG.md); the
engineering weight is the activation/daemon hardening and the freshness-gate
threat-model work.

### Activation diagnostic wire-up (MLP2-051f)

- **Activation consumes the daemon `ProtectionClaim` snapshot.** The activation
  surface consumes `anvil_intercept::status::build_protection_claim_from_wire`
  and promotes handshake-verified MCP clients to `LiveValidation` when the
  worktree is in `PreWriteDaemon` (or `DegradedProtection` with at least one
  `Participating` surface), with concrete repair hints when promotion is
  blocked. Post-ship hardening resolved four council MAJORs.
- **`DaemonStatusV1::generated_at_unix` wire-add (MLP2-051h).** A daemon-level
  wall-clock anchor, distinct from per-session heartbeats, used as a second
  freshness consistency check. Wire-additive via `#[serde(default)]` — a
  pre-`v0.7.1-beta` daemon deserialises the field at `0`, which consumers treat
  as "no anchor; fall back to per-session freshness".
- **Windows MCP `validate_write` carries `protection_claim` (MLP2-075).** A
  named-pipe IPC client brings Windows to parity with the Unix socket path, so
  Windows + Scoop + PowerShell users get the same typed claim that was always
  `None` before.

### Reliability & threat model

- **Single wall-clock deadline on `query_daemon_status_at` (Unix).** The read
  loop now refreshes `set_read_timeout(deadline − now)` against one
  `Instant`-based deadline, so a daemon dribbling one byte per (timeout − 1 ms)
  can no longer keep the loop alive ~524 s; the 500 ms activation IPC budget is
  enforced end-to-end, at parity with the Windows path.
- **Freshness gate bounds future-timestamp tolerance.** A
  `MAX_FUTURE_CLOCK_SKEW = 90 s` upper bound rejects a daemon stamping a
  far-future time (broken RTC, snapshot replay, malicious output), bounding the
  downgrade path to 90 s of clock skew while tolerating NTP steps and VM drift.
- **L4 engine distinguishes IO outages from missing engines.** A new
  `EngineUnavailableReason::IoError` variant separates a transient filesystem
  hiccup from a permanently absent engine, so the `engine-missing` operator hint
  no longer fires for retryable IO.

### Security

- **Windows IPC trust rests on the named-pipe DACL set at pipe creation.**
  Client-side SID validation (defence-in-depth parity with the Unix
  `SO_PEERCRED` check) is deferred to MLP2-051j; same-SID processes are inside
  the v1 trust boundary the same way same-UID processes are on Unix.
- **Per-connection admitted-workspace-root budget (CIB-154).** A new DoS-family
  cap `enforcement.dos.max_admitted_roots` (default 32) bounds the number of
  distinct workspace roots one connection may admit. `Open`-mode admission pins
  one real file descriptor (`WorkspaceAnchor`) per distinct root, so an
  unbounded root set let a same-uid peer exhaust the daemon's descriptor table;
  the budget refuses the over-budget `(budget + 1)`-th admissible root with a
  structured `-32011 Workspace root budget exceeded` (distinct from
  `workspace-not-admitted`). Enforced in both `Open` and `Allowlist` modes,
  merges stricter-wins (smaller cap wins) like its sibling DoS caps, and clamps
  to a minimum of 1. The per-verb admission gate canonicalises the incoming root
  **exactly once** and makes the budget-check-then-admit decision on that single
  resolved path, closing a check/act TOCTOU a split resolution would leave. Cap
  changes need a daemon restart (`IpcLimits` is resolved once at daemon start).
  Mirrors the MLP2-024 per-worktree session-cap precedent.

### Diagnostics, docs & delivery

- **Activation tracing surfaces operator-actionable failures at `warn`.** The
  missing piece (daemon unreachable, worktree unenforced, stale snapshot,
  all-surfaces quarantined) shows at the default `ANVIL_LOG=warn` filter;
  transient states stay at `info`, the genuine pre-restart case at `debug`.
- **`docgov` validates as-built source paths.** DOCGOV closeouts naming a
  non-existent source file now fail the governance check at PR time instead of
  shipping a broken cross-reference.
- **`anvil uninstall` detects Scoop and WinGet install paths**, with a tightened
  boundary check so removal cannot stray outside the install root.
- **`anvil-run` manpage documents the SIGTERM transient-fence behaviour** — a
  launcher killed by SIGTERM may briefly fence the worktree; the next invocation
  clears it during session registration.
- **ADR-047 Accepted**
  ([eddacraft-tui canonical source mirror](./plans/decisions/047-eddacraft-tui-canonical-source-mirror.md)),
  seeding the TUIR reintegration that lands in `v0.7.2-beta`.

## [0.7.0-beta] — 2026-05-21 — Daemon Working End-to-End

### Multi-Layer Protection v2 (MLP2) — daemon-working integration

MLP2 closes the gap between every v1 primitive shipped in `v0.6.0-beta` and the
full surfaces it targets. 12 groups (A–L) cover 60+ integration items split out
from MLP-018, plus Council-flagged production hardening. Module is 60/76 at tag
time; the cut-line for the daemon-working claim is named in
[`RELEASE-PLAN.md`](./RELEASE-PLAN.md). MLP2-042..-045 + MLP2-051d (Marketplace
publishing + GH Action check render) remain blocked on the licensing / pricing
track and are explicitly carved out of the protection claim.

### Witness Chain Hardening (MLP-002..-005, MLP2-011..-015, MLP2-061..-063)

- **DAG-aware verifier** — `verify_chain_dag` walks the merge-join graph via
  `parent_commits[]` + `prev_line_hashes[]` lockstep arrays; the legacy linear
  `verify_chain` becomes a `#[deprecated]` thin wrapper. Four production call
  sites (pre-push hook, `anvil l4-validate`, `anvil audit-chain`,
  `save_with_genesis`) migrated to the DAG verifier.
- **Genesis anchor on baseline** — `anvil-baseline::save_with_genesis` emits
  `GENESIS-BASELINED` (bare) or `GENESIS-FRESH` as the chain's first witness
  line; the cutoff commit SHA lives on the line body as a separate
  `cutoff_commit: Option<String>` field rather than glued onto the anchor
  string. `GenesisAnchor::parse()` explicitly rejects the colon-suffix form.
- **`rules_sha` threading** — MLP2-014 threads `anvil_rules::rules_sha` onto
  every pre-commit witness line; empty `rule_ids` list reserved for the future
  rule-engine wiring.
- **80-writer stress test promoted** — MLP2-015 lifted the 80-way
  concurrent-writer test out of `#[ignore]` after 10/10 ~10ms flake budget on CI
  hardware.
- **Shared `witness_paths()` helper** — MLP2-061/-062 collapsed three parallel
  walkers (pre-push, `anvil l4-validate`, `anvil audit-chain`) onto a single
  source-of-truth function. Closed a trust-gap where any drift in ordering would
  let the verifier and the witnessed-set harvester cover different bytes.
- **Manifest event stream** — MLP2-012 writes one `ManifestEntry` per rollover
  to `anvil/witness/manifest/chain.ndjson` (archive path, full SHA-256, line
  count, `[start..=end]` seq range). Content-addressed archive naming makes
  re-run rollovers idempotent.

### L4 Policy Engine (MLP2-016..-022, MLP2-031, MLP2-046, MLP2-068)

- **Typed `ValidationEngine` trait** — new `validate_at_l4` pipeline in
  `crates/anvil-l4/src/validate.rs` returns
  `Allow / Block { diagnostics } / EngineUnavailable { reason }`. Pre-push hook
  swaps the inline `InternalError { TimedOut }` fall-through for trait dispatch
  with `on_warn`-aware verdict routing; default `NoOpValidationEngine` preserves
  pre-MLP2-016 byte-identical surface until a real engine binds.
- **Real antipattern engine binding** — `0aacdac8` wires the real engine into
  pre-push + `anvil l4-validate` so commit-blob walks resolve through the
  production validator.
- **Version floor + cutoff-commit ancestry** — MLP2-018 ships
  `evaluate_version_floor(policy_floor, witness_anvil_version)` with semver
  build-metadata precedence; MLP2-019 ships `RecognisedRulesRegistry` keyed on
  lowercase 64-char hex digests with `RuleSetMetadata` enforcement;
  MLP2-020/-021 thread the floor + cutoff-commit ancestry check into the
  pre-push hook (`git rev-list --first-parent --max-count=100000` per ref,
  hex-shape validation on `Policy::cutoff_commit`).
- **Time-budget cap** — MLP2-022 lands `PRE_PUSH_BUDGET = 2s` between-commit
  check; on exceed, emits a distinct `ErrorClass::TimedOut` line plus a
  structured `tracing::warn!` with `kind="gate_evaluated"`, `gate_id="prePush"`,
  `partial=true`, `commits_processed`, `commits_skipped_for_cutoff` for future
  Kindling fan-out (INTD-004).
- **`anvil l4-validate` CLI** — MLP2-046 extracts the L4-policy validator into a
  dedicated subcommand, replacing the `anvil hook pre-push` reuse for CI and
  GitHub Action consumers.
- **Atomic policy pin** — MLP2-031 ships `pin_cutoff_commit(path, cutoff)` with
  temp-then-rename writer, hex-shape pre-flight, symlink refusal on path + temp
  sibling, multi-format round-trip (yaml/yml/json/toml), and
  `PolicyPinError::BaselineNotAMap` so hand-edited scalar `baseline:` is never
  silently overwritten.
- **`git cat-file --batch` for commit blobs** — MLP2-068 (`d54a5f86`) batches
  per-commit blob fetches in `CommitAntipatternEngine`, replacing N-way
  fork-exec on history walks. Performance follow-on filed under Group O
  (MLP2-068 Merged; MLP2-069 `EngineUnavailableReason::IoError` variant remains
  Draft post-tag).

### Audit Chain L5 (MLP-015, MLP2-053..-056)

- **`anvil audit-chain` CLI** — re-walks a branch's commits and reports any that
  lack an L3 witness line. `--threshold` (default 5) toggles the
  `degraded:audit-drift` marker; `--rescan` opt-in re-evaluates today's rules
  against history; `--max-runtime` caps wall-clock walk; emits a Kindling row to
  `anvil/kindling/audit-chain.ndjson` per run.
- **Nightly L5 workflow** — `.github/workflows/anvil-audit.yml` template ships
  in-tree at `crates/anvil-cli/src/templates/anvil-audit-workflow.yml` and is
  copied by the activation orchestrator at adoption time. Active by default;
  operator disables by commenting out the `schedule:` block.
- **Group K closed 4/4** via PR `d96ab458` covering audit-chain workflow
  template, Kindling emission integration, rule rescan, and time-budget cap.

### Session Registry + Composite Identity (MLP2-001..-003, -023..-026)

- **Composite session key** — MLP2-023 extends the registry to
  `(WorktreeKey, Option<AgentTag>)` via additive `agent_tag` on
  `SessionRecord` + `IpcCommand::RegisterSession` (wire-additive via
  `serde(default, skip_serializing_if)`). Composite `by_composite` index,
  deterministic `attribute_path` tiebreak (untagged-first then
  earliest-started + lexicographic SessionId), per-tag `unregister` /
  `evict_stale`. Unblocks MLP2-003/-024/-025/-026.
- **`ProjectIdentity::verify_against_worktree`** — MLP2-003 cross-checks live
  git state (`git rev-list --max-parents=0 HEAD` first-commit +
  `git config --get remote.origin.url` canonicalised). Typed
  `AttachStatus { Clean, Fork, Mismatch, ProjectIdMissing }` and the pinned
  `degraded:identity-mismatch` wire-signal constant.
- **Per-worktree session cap** — MLP2-024 adds
  `enforcement.session.per_worktree_max` (default 16) under a new
  `SessionConfigFile` proto block, stricter-wins merge with zero-clamp.
  `RegistryError::SessionCapExceeded { worktree, cap, live }` is typed.
- **End-to-end agent-tag spoof rejection** — MLP2-025/-025b/-025c: the launcher
  and TS driver-client both forward `ANVIL_AGENT_TAG` and PID lineage; the
  daemon cross-checks them against the tag it issued at registration.
  `Cross::Match` admits, `Cross::Spoofed` blocks and fences with
  `degraded:spoofed-attribution`. `session_register_params` emits nested
  `agent_tag` and `lineage`; `RegistrationRequest` gains `launcher_pid: u32`
  from `std::process::id()`.
- **`degraded:fence-cascade` operator-recovery lane** — MLP2-026 ships persisted
  `CascadeRecord` state in `FenceFile`, `RateWindow::new(4, 60s)` on
  `FenceStore`, status surface `cascaded`/`cascade_since` fields, registry-side
  `WorktreeCascaded` refusal under documented cascade-before-registry lock
  ordering. `IpcCommand::UnblockCascade { worktree, operator }` derives
  `OperatorContext` from daemon-side IPC peer credentials.
- **Bounded LRU rule cache** — MLP2-057 caps `rule_cache` at
  `DEFAULT_RULE_SET_CACHE_CAPACITY = 1024` with `evictions` counter and
  `tracing::warn!` on capacity pressure. New
  `SessionRegistry::with_unregister_hook(Arc<dyn Fn(&Path) + Send + Sync>)`
  fires AFTER lock release on `unregister` + per-session in `evict_stale`.

### Intercept Launcher (INTL-001..-009)

- **`anvil-run` crate ships** — `crates/anvil-run/` lands via PR #1528 with
  INTL-001..-009 covered by 49 unit + 3 shell-integration tests. Wrap mode
  (`anvil-run --tool <name> -- <cmd...>`) + hook mode
  (`anvil-run hook register --tool <name>`) parse via `clap`.
- **Process-group ownership** — `setpgid` on Unix, named Job Object on Windows.
  Cleanup `Drop` guard ensures the session unregisters on every exit path
  including panic / signal.
- **Daemon preflight + heartbeat** — `preflight` queries reachability +
  worktree-fence state before spawn; `heartbeat` ticker keeps the daemon
  registry alive while the child runs. No `--no-heartbeat` CLI surface
  (skip-field, test-only) — long-running sessions cannot age out of the registry
  by operator misconfiguration.
- **Stable BSD-sysexits exit codes** — `64/EXIT_USAGE`,
  `69/EXIT_DAEMON_UNAVAILABLE`, `73/EXIT_SPAWN_FAILED`, `75/EXIT_FENCED`,
  `78/EXIT_BAD_CONFIG`. `forward_child_status` maps Unix signals to
  `128 + signo`; Windows forwards the raw code modulo 256. Tests pin the codes
  and the cross-platform behaviour.
- **Side-channel registration (INTL-007)** — `anvil-run hook register` lets
  tools that did not start through the launcher register a session with the
  daemon. Enforcement capped at fence-only per ADR-038 noise-discipline.
- **Shell integration** — `crates/anvil-run/shell/anvil-run.sh` exposes
  `claude()`, `codex()`, `aider()`, and a generic `anvil-wrap` for ad-hoc tools.
  Honours `ANVIL_RUN_DISABLE`; falls through to direct `command` exec when the
  launcher is not on `$PATH` (lose-enforcement-preferred over block-the-user).

### Baseline Hardening (MLP2-013, MLP2-032..-036)

- **`--new-identity` fork opt-out** — MLP2-033 adds the flag to `anvil start` +
  `anvil baseline` via `mint_new_identity(root, version) -> ProjectIdentity`.
  Mints fresh v7 UUID, records previous `project_uuid` as `forked_from`;
  baseline rewrite bypasses the "already exists" short-circuit so
  `metadata.project_uuid` cannot diverge from the freshly-minted identity.
- **Adversarial-refresh detection** — MLP2-035 Phase 1 ships
  `analyze_refresh(old, new, thresholds) -> RefreshSuspicion` in
  `crates/anvil-baseline/src/diff.rs`. Refuses to overwrite `baseline.json` when
  a refresh would drop ≥75% of findings AND ≥10 absolute, unless the operator
  passes `--accept-suspicious`. Knobs: `--suspicion-ratio`,
  `--suspicion-min-removed`. Constant `degraded:baseline-suspicious` exposed.
- **Partial-baseline continuation** — MLP2-036 Phase 1 lets large repos (>100k
  files) baseline in budgeted chunks. `Baseline` schema gains `partial: bool` +
  `continuation: Option<String>` (both `serde(skip_serializing_if)` so complete
  baselines serialise byte-identically). `scan_repo_for_findings_with_budget`
  returns `(Vec<Finding>, Option<String>)` with files sorted by repo-relative
  path + forward-slash normalisation. `--scan-budget` flag (default 50000); zero
  rejected at boundary.
- **Whitewash defence** — `--refresh` complete → partial refuses without
  `--accept-suspicious`; cutoff pin + suspicion detection skipped while partial.

### Hook Coexistence (ADOPT-001)

- **Framework probe + managed-block install** — `anvil hook bootstrap` probes
  the repo root for marker files in fixed order (Husky → Lefthook →
  pre-commit-framework → cargo-husky → CoreHooksPath → Plain), first match wins.
  Husky and Plain installs are byte-stable round-trip; Lefthook +
  pre-commit-framework ship an `.anvil-*.yml` snippet + marker-bounded comment
  block in the host config and require a one-time manual `extends:` / `repos:`
  merge.
- **Coexistence report** — `anvil hooks install --config` and
  `anvil hooks uninstall --config` print per-event signals (`file_mode_paths`,
  `third_party_managers`, `foreign_config_entries`, `core_hooks_path`) so
  duplicate-execution and `core.hooksPath` cases are visible.
- **Round-trip canonicalisation** — install + uninstall returns marker-bounded
  blocks cleanly; Husky files with non-canonical trailing whitespace are
  canonicalised (documented). User-added Lefthook `extends:` and
  pre-commit-framework `repos:` entries are not auto-removed — out of scope for
  the uninstall contract.

### Resource Budget Gate (ADOPT-002)

- **`anvil-bench` crate** — Linux `/proc` sampler primitive plus a reference
  repo + `watch_resource_budget` bench scenario that drives `anvil watch`
  against the fixture and emits a `BudgetVerdict`. CI workflow at
  `.github/workflows/resource-budget.yml` fails the build if steady-state CPU >
  5% or RSS > 200 MB.
- **Documented ceiling** — `docs/policies/resource-budget.md` pins the numbers +
  measurement protocol so the gate semantics survive refactors.

### Editor Coexistence (ADOPT-006)

- **Headless harness + CI gate** — covers `rust-analyzer`, `tsserver`,
  `pyright`, `ruff`, `prettier`, `eslint` against Rust/TS/Python fixtures.
  `.github/workflows/editor-coexistence.yml` blocks the candidate on regression.
  Compatibility matrix at `docs/policies/editor-coexistence.md`.
- **Implementation notes** — `7614cb88` handles rustup `rust-analyzer` shim and
  installs the component in CI; `40a86fb1` pivots the rust-analyzer runner to
  `cargo check` with failed-tail logging when the full LSP probe was unreliable
  in headless CI.

### Distribution & Self-Update (DISTRIB-001..-004)

- **Minisign-verified `anvil update`** — DISTRIB-001 wires signature
  verification into Homebrew, curl-installer sidecar, and the axoupdater library
  fallback. Signature mismatches fail loudly.
- **`anvil version --check` advisory surface** — DISTRIB-002 ships the
  network-gated update + security-advisory check; off by default. ADR-044 §9
  makes -001 and -002 load-bearing for the MCP-backend swap discovery gap.
- **Homebrew formula automation** — DISTRIB-003 (PR #1652 + `657ca39e`) extracts
  the formula auto-bump into a tested script so releases publish the matching
  formula automatically; `brew upgrade` users see new tags without a manual tap
  refresh.
- **Release cadence + EOL policy** — DISTRIB-004 ships
  `docs/policies/release-cadence.md` documenting hotfix cadence,
  patch/minor/major semantics, the "sit on a release" minimum window, and the
  `-beta` support window.

### MCP Hardening (RMCPF / RMCP / CIB)

- **Typed `protection_claim` on `validate_write` response** — MLP2-051b (PR
  #1668) emits the optional `Option<ProtectionClaim>`, gated on
  `DaemonStatus::Available`, fetched via the new `query_daemon_status_at`
  helper. Wire-additive; pre-existing drivers round-trip the response unchanged.
- **Patch-mode validator (CIB-005)** — PR #1692 makes `anvil_validate_write`
  accept unified-diff `patch` payloads via the existing `apply_patch` helpers.
  Token cost scales with the change, not the file. Closes the 2026-05-18
  beta-tester incident on a 2770-line JSON file. Council follow-on `121eeecd`
  addresses review.
- **Recoverable workspace-root preflight (CIB-007)** — same PR returns
  `expectedWorkspaceRoot` on rejection so callers can self-correct without an
  operator round-trip. Option (b) triage; option (a) (worktree-aware accept)
  deferred behind an ADR.

### Driver Client Mirror (MLP2-029..-030, MLP2-051c)

- **TS `AgentTag` mirror** — MLP2-029 lands `parseAgentTag` in
  `packages/anvil-driver-client/src/session/` with per-field type guards (no Zod
  dep), `ANVIL_AGENT_TAG_ENV` / `ANVIL_TASK_ID_ENV` constants, and a byte-exact
  JSON-parity test against the Rust `agent_tag_round_trips_through_json`
  fixture.
- **TS mid-edit Kindling observation mirror** — MLP2-030 ships
  `fromMidEditResponse` + `GateEvaluatedObservation` with 13 parity tests
  including byte-exact JSON parity against a captured Rust `to_string` fixture,
  severity → enforcement mapping, and the volume-control contract (`null` for
  empty diagnostics).
- **TS `ProtectionClaim` mirror** — MLP2-051c (`d4970b19`) ships the TS parser;
  MCP response adapter surfaces the claim when the daemon supplied one;
  responses without the field parse cleanly for backward compatibility.

### Kernel Local-Noise Ignore Canonicalisation (ADOPT-004)

- **Canonical const moves to `anvil-kernel`** — PR #1658 (`34671da7`) relocates
  `IGNORE_DIRS` from `anvil-cli/src/util.rs` to
  `anvil-kernel::watcher::filter::IGNORE_DIRS` so every walking consumer (watch,
  audit, baseline, check, drift, gate) inherits the same list. CLI helper
  becomes a `pub use` re-export.
- **Coverage expansion** — `.venv` added; `__pycache__` reconciled. A
  conformance test asserts the kernel and CLI helpers resolve to the same set.

### YAML Resource Bounds (MLP2-060)

- **Alias rejection + size + depth caps** — MLP2-060 hardens
  `anvil-config::parse` against billion-laughs and other YAML resource attacks:
  rejects aliases outright via a quote/comment-aware byte scanner, caps the file
  at 1 MiB pre-parse, and bounds post-parse depth at 32 levels. 10 new tests
  including the classic billion-laughs payload and a 40-level JSON depth
  rejection. ADR-046 documents the YAML-parser-migration deferral that motivated
  the in-place hardening.

### Insights (INSIGHTS-001)

- **`anvil insights` weekly summary** —
  `crates/anvil-cli/src/commands/insights.rs` derives a weekly rollup from the
  witness chain with no separate event store. JSON schema pinned at
  `anvil.insights.v1` / `schemas/anvil-insights.v1.json`. This release populates
  `witness_events_observed`; the other six counters (`total_saves_observed`,
  `findings_raised`, `suppressions_applied`, `suppressions_resolved`,
  `baseline_edges_added`, `daemon_uptime_percentage`) ship as schema-locked
  placeholders pending INSIGHTS-002..-004 metric wiring.

### Operations Support Framework (OPSUP-006)

- **File-presence + wall-time guards** — `f0d0490e` adds a defensive guard
  framework to short-circuit expensive commands when a required file is absent
  or a runaway loop exceeds a wall-time cap. Defence in depth for the
  daemon-working surface.

### Architecture Decisions

- **ADR-045 — update signing scheme.** Pins the minisign-based signature
  verification design that DISTRIB-001 implements.
- **ADR-046 — YAML parser migration deferral.** Documents why
  `anvil-config::parse` stays on `serde_yaml` for `v0.7.0-beta` with MLP2-060's
  in-place resource-bounds hardening, deferring the migration to a future
  release.
- **ADR-047 — eddacraft-tui canonical source mirror.** Pins the source model +
  governance for the eddacraft-tui mirror after the ATTRIB-011 scaffolding
  lands.
- **ADR-048 — feature group architectural model.** Pins three coupled decisions:
  Feature Group is a defaults carrier (class + audiences + lifecycle) with
  per-flag override; hybrid taxonomy (`primaryGroup` carries defaults, `tags`
  are taxonomy-only); kill-switch is a universal runtime channel via the
  existing `FlagOverrides.emergency` mechanism rather than a per-group default
  class. Companion spec at `plans/specs/2026-05-19-feature-gating-model.md`.

### Release Engineering

- **Main-first cutover (OPMODEL-012)** — `dev` branch retired at the 2026-05-11
  cutover; all branches and PRs now target `main`. Historical branch deletion
  scheduled per #1419.
- **Release-record schema (OPMODEL-004)** —
  `plans/specs/2026-05-10-release-record-schema.md` pins the per-release
  `releaseRecord` JSON shape that the cleanup agent advances through
  `Merged → Released/Shipped → Complete/Archived` lifecycle states.
- **Release-readiness workflow** — targeted CI gates split from best-effort
  coverage; `cargo llvm-cov nextest` runs `continue-on-error` so a
  coverage-merge regression cannot mask real test signal.
- **Air-gap harness baseline** — `tools/test-harness/network-blocked/run.sh`
  - `crates/anvil-cli/tests/air_gapped.rs` enforce no-network on every
    protection command via Linux network-namespace unprivileged user-and-
    network-namespace stripping. Coverage extension to `anvil-run`,
    `anvil audit-chain`, `anvil l4-validate`, `anvil hook pre-push` tracked
    under GH #1705 (not tag-blocking).

### Documentation & Governance

- **Six N4 user-facing runbooks** filed for `v0.7.0-beta`: air-gap
  (`docs/runbooks/anvil-air-gapped.md`), hook coexistence
  (`anvil-hook-coexistence.md`), witness-chain operator
  (`anvil-witness-chain.md`), adoption (`anvil-adoption.md`), migration
  (`v0.6.x-to-v0.7.0-beta-migration.md`), `anvil-run` manpage (`anvil-run.md`).
  Plus the Wave 0 operator-facing release runbook at
  `v0.7.0-beta-release-runbook.md`.
- **DOCGOV-005 `docs:check`** — gates metadata, tags, links (anchor + file
  existence), APS-drift, ADR-index, index-freshness, asbuilt-paths surfaces.
  Slugifier in `scripts/docs/check-links.mjs` collapses non-alphanumerics +
  spaces into single hyphens for anchor matching (matters for runbook
  cross-references).

## [0.6.0-beta]

### Daemon-Backed Mid-Edit Validation (INTD)

- **Owner-only IPC** — daemon listener accepts connections only from the owning
  UID via `SO_PEERCRED` on Linux, `getpeereid(2)` on macOS, and a per-user DACL
  with `reject_remote_clients(true)` on the Windows named-pipe listener.
  `crates/anvil-intercept-win32` ships the Windows daemon side; the synchronous
  Win32 client backs `anvil intercept status` for parity with the Unix UDS path.
- **Process-group interrupt ladder** — INTD-006 lands the
  `SIGINT → SIGTERM → SIGKILL` ladder against the worker process group on Linux.
  macOS falls through to AD-7's fence-on-uncertainty invariant in this cut
  because the `current_process_start_time` helper is Linux-only; documented in
  the v0.6.0-beta release runbook §4.
- **Fence persistence** — INTD-005 records fence state to disk in the data
  directory, re-fences on daemon startup, and survives daemon crashes, restarts,
  and reboots. The `anvil intercept stop` and `unblock` CLI subcommands are
  deferred; recovery in v1 is the runbook's daemon-stop +
  fence-directory-removal procedure.
- **Daemon configuration & embedded fallback** — INTD-008 wires the daemon
  enforcement-config loader; INTD-010 evaluates rules in embedded mode when
  daemon dispatch is unavailable, keeping correctness equivalence with the
  daemon path. INTD-011 closes the unregistered-change fence so a write that
  bypasses validation still fails closed.
- **IPC DoS budgets and telemetry scoping** — INTD-009 caps per-connection
  request and response budgets so a misbehaving client cannot exhaust the
  daemon. INTD-015 scopes telemetry subscriptions to the requesting session
  rather than broadcasting cross-session.

### Editor Driver Framework (DRVR)

- **Driver client + protocol** — `anvil-driver-client` ships the shared client
  surface; DRVR-002 lands the editor-driver protocol with capability
  negotiation, and the trust-boundary spec is documented as a release artefact.
  RTAI-004 wires the mid-edit debouncer through `validateMidEdit`.

### Activation & MCP Launch (LAUNCH)

- **`anvil start` activation** — LAUNCH-002 owns the activation entrypoint with
  `--verify` and `--watch` flags. LAUNCH-009 wires Cursor / Claude Code MCP
  install with the shared activation-state vocabulary (`protecting`,
  `ready_restart_required`, `watching`, `needs_action`, `unsupported`, `error`)
  consumed by `anvil status --verify`, `anvil doctor`, and the protection-loop
  tutorial.
- **Repo language profile** — activation profiles the repository's languages and
  surfaces an honest skip ledger; TypeScript is the supported tier in this cut,
  SQL/Markdown partial, Python/Rust unsupported. Cross-language scans (secrets)
  continue running on every file.
- **Install-method-aware version surface** — `anvil --version` detects Homebrew,
  Scoop, WinGet, the cargo-dist installer, or a dev build and prints
  `update_available`, install method, and the recommended upgrade command. JSON
  shape is pinned for agent and CI consumers.

### Scanner Hot-Path Performance (V050F)

- **Allowlist regex caching** — V050F-006 (#1323) caches the compiled allowlist
  regexes in `prepare_pattern` and replaces `AllowlistGlob.pattern: String` with
  a precomputed `is_path_glob: bool`, eliminating an N×M regex compile on every
  scanned file.
- **Custom secret pattern compile errors** — V050F-011 (#1323) introduces
  `scan_content_with_compiled_patterns` and
  `scan_content_with_pattern_errors_and_stats` so callers receive per-pattern
  compile diagnostics instead of silent drop. The legacy `scan_content_with_*`
  wrappers preserve their signatures and emit `tracing::warn!` on dropped errors
  so the silent-loss path is observable.
- **Eager rayon pool init** — V050F-007 (#1330) extracts the half-cores
  global-pool cap into the dedicated `anvil-rayon-init` micro-crate and calls it
  from the CLI binary entry point and the NAPI `scan_artifact_json` entry,
  replacing the duplicated `Once` blocks in `kernel/watch.rs` and
  `kernel/embedded.rs`.

### CI Gating & Test Reliability

- **Cross-compile gate on `dev`** — PR #1325 (`ed957ce1`) widens the
  cross-compile trigger in `.github/workflows/rust.yml` from main-only to fire
  on pushes and PRs targeting either `main` or `dev`, gated on
  `detect-rust-changes` so JS-only diffs don't spin up the Windows + macOS
  matrix. Closes the gap that let Windows-only build breakage land on `dev`
  between releases. Historical context preserved in
  `docs/runbooks/intd-012-windows-evidence.md` with a status banner.
- **MCP daemon integration tests Unix-gated** — daemon-backed integration suite
  is `#[cfg(unix)]` in this cut; Windows coverage rides the same follow-up as
  the MCP correlation envelope.
- **Coverage step non-blocking on push** — `cargo llvm-cov nextest` started
  failing the post-test profile-merge consistently on `dev` pushes
  (`error: no profile can be merged` from corrupt `.profraw`s). Strict test gate
  split from best-effort coverage in `76a17442`; coverage step marked
  `continue-on-error: true` so a coverage-merge failure doesn't mask real test
  signal. Underlying merge regression tracked separately.
- **Cancellation-test sync safety net widened** — the polling-loop bound in
  `cancellation_emits_cancelled_error_detail_not_spawn_failed` was ratcheted
  from 5 s to 30 s after sustained failures on `ubuntu-latest` under nextest's
  default parallel execution. Bound is a sync aid, not a timing assertion;
  structural follow-ups (worker-side notification, serial nextest group) noted
  inline.

## [0.5.1-beta]

### Scanner Signal Hardening

- **Secret false-positive reductions** — generic secret matching now requires a
  stronger right-hand-side shape, credit-card detection rejects UUID fragments,
  and entropy matching focuses on secret-shaped quoted values.
- **Antipattern suppression alignment** — `AP-*` checks now honour local
  `eslint-disable` directives, and `GS-001` avoids reporting guarded `Map.get`
  after `has`/`set` flows.
- **Audit input filtering** — audit scans skip broader environment-template
  files while still reporting real `.env` files regardless of directory.

### Kernel Incremental Graph Fixes

- **Synthetic import ID allocation** — watch graph updates now keep synthetic
  import IDs out of the allocator's file-ID range so incremental updates do not
  collide with real source files.
- **Import-source ID zero handling** — `update_file` now treats ID `0` as a
  valid import source, preserving edges that previously disappeared when the
  first allocated file participated in import analysis.

### TUI & Release Operations

- **TUI interaction fixes** — audit, status, and watch surfaces support zooming;
  doctor acknowledges `f` to fix; tutorial path selection has more room for
  wrapped options.
- **TypeScript scanner retirement** — the archived TypeScript scanner stack and
  parity harness now live under `anvil-archive/anvil-ts-scanner/`, with the Rust
  scanner remaining authoritative; stale scanner-era package subpath exports
  were removed from `@eddacraft/anvil-core` and `@eddacraft/anvil-runtime`.
- **PR base guard** — a release-sensitive PR base guard workflow now detects the
  branch-targeting mistake that caused the post-`v0.5.0-beta` recovery work when
  repository branch protection requires the check.

## [0.5.0-beta]

### Git Hook Compatibility (GHOOK)

- **Git 2.54 config-hook baseline** — compatibility policy added for native
  `[hook.<name>]` execution, with anvil end users kept on the existing Git 2.30+
  floor unless they opt into config mode
- **`anvil hooks --config` path** — install/uninstall can append and remove
  anvil-owned `hook.<event>.command` entries without touching foreign config or
  file hooks
- **Coexistence detection** — install, uninstall, status, doctor, onboarding,
  and tutorial surfaces detect file hooks, config hooks, third-party managers,
  `core.hooksPath`, and duplicate-execution risk
- **Contributor workflow decision** — GHOOK-005 accepted Option A: keep Husky as
  the repository bootstrap for now, while leaving `anvil hooks install --config`
  as an explicit power-user opt-in
- **Public docs sweep** — git-hook operations docs and CI/agent-harness examples
  now describe file-mode and config-mode behaviour together

### AI Guardrail & Diagnostics

- **AI guardrail profile complete** — `anvil gate --profile ai` now selects a
  curated check set, treats missing/invalid governance config as blocking, emits
  JSON by default, and documents the `anvil.gate-result.v1` contract
- **Canonical diagnostic shape** — `crates/anvil-kernel-types` now owns
  `anvil.diagnostic.v1` for gate, save-time, watch, and mid-edit diagnostics;
  the envelope coordination spec records how AIGUARD, RTAI, INTD, and DRVR share
  it
- **AI-001 reasoning rule** — `anvil-checks` now flags appeal-to-authority style
  comments, limits matching to comment regions, honours `@anvil-ignore AI-001`,
  and emits `Category::Reasoning` diagnostics at info severity
- **RTAI-001 phase-0 spike** — the mid-edit secret-detection loop measured about
  1.4 ms p95 over 1024 iterations, roughly 60x under the ADR-031 warm-path
  budget; the report chooses a single `scan_buffer` method with a mode
  discriminator for save-time versus mid-edit validation
- **Validation latency rubric** — ADR-031 pins latency budgets for save-time,
  mid-edit, and gate paths so future real-time validation work has an explicit
  performance envelope

### Scanner Coverage & Performance

- **Parallel scan rollout** — `gate`, `audit`, `check`, `drift`, policy,
  architecture validation, and watcher call-sites now share the gitignore-aware
  discovery plus rayon scan pattern; the SCAN benchmark recorded a 7.39x
  wall-time improvement on a synthetic 3k-file surface
- **ReDoS line-length guard** — `SecretCheckConfig::max_line_bytes` defaults to
  4096 bytes, skips oversized lines before regex evaluation, and reports skipped
  counts through `SecretCheckResult`
- **First-run pool cap** — first-run scans use `ANVIL_SCAN_THREADS` with a
  default cap of `min(num_cpus, 4)` to avoid starving TUI/editor work
- **`.env` secret surface** — `.env`, `.env.*`, and `.envrc` parsing routes
  values through the existing secret patterns, reports the variable name and
  source line, and supports `# @anvil-ignore SURFENV-001`
- **Scanner false-positive fixes** — AI-001 comment scanning is string-aware,
  and the TypeScript LSP fixture no longer trips the reasoning rule

### CLI, Onboarding & Editor Integration

- **`anvil mcp-config`** — Rust CLI command added for Claude Code, Cursor,
  Windsurf, and VS Code config generation; supports stdio/http transports,
  `--write`, `--verify`, workspace overrides, path-safety prompts, and atomic
  writes
- **Interactive fix handling** — start-flow surfaces share a single interactive
  fix service so doctor/status/onboarding prompts route consistently
- **Doctor missing-git behaviour** — `git-repo` now emits a structured warning
  rather than a failure when run outside a git repository
- **First-run copy** — inotify capacity guidance, instances-limit text, strict
  AI-guardrail config wording, and post-init auth-login next steps were
  tightened

### API, Infra & Release Operations

- **Database migration runner** — `apps/anvil-api` now has a first-party SQL
  migration runner, unit coverage for drift/pending cases, a manual runbook, and
  infra workflow wiring before Pulumi Up
- **Release publisher hardening** — the cargo-dist installer is SHA256-pinned;
  Scoop publisher pre-flight checks token reachability; WinGet publisher fork
  handling and `gh` argument usage were hardened after the v0.4.0-beta tag run
- **Release token runbook** — operator guidance now leads with editing the
  existing fine-grained PAT repository scope instead of rotating when Scoop or
  WinGet publishing gets a 403
- **Vercel/API runtime recovery** — Hono/Vercel entrypoint restoration, scoped
  API tsconfig, Nx framework-detection controls, and the `svix>uuid` override
  exception restored production deployment after the post-release runtime break
- **CORS and env exposure invariants** — tests now lock in lower CORS preflight
  cache lifetime and avoid treating all `NEXT_PUBLIC_` variables as sensitive

### Documentation, Plans & Attribution

- **Locked release slate** — release plan and roadmap now capture the A1 RTAI
  spike, A2 AI guardrail, A3 release engineering, and A4 language-credibility
  floor as the current release menu
- **Beta docs refresh** — tester guide and beta-user scenarios now cover the
  current onboarding, hooks, AI guardrail, MCP, and docs-auth flows
- **Portable attribution kit** — acknowledgement generation moved into a starter
  template set with `about.toml`, `about.hbs`, CI freshness snippet, and project
  example config
- **APS freshness** — GHOOK completed and archived; v0.5.0-beta
  release-follow-up, language audit, RTAI, AIGUARD, SCAN, RCLI2/RCLI3, and
  surface modules were reconciled against the current release plan

## [0.4.0-beta]

### Native Rust Scanner Becomes Authoritative

- **`.anvil` format parser and compiler** landed in `anvil-core` (`ANVFMT` Phase
  1); the registry-backed pattern catalogue replaces the legacy TS-side HTML/CSS
  catalogues entirely (`ANVFMT-014`, `ANVFMT-015`). Pattern reference docs in
  `docs/anvil` now describe the authoritative format (`ANVFMT-016`)
- **Rust scanner module (`RSCAN-001..008`)** — registry loader, artefact model +
  `scan_artifact` API, family provenance on `AntiPattern` and `Warning`,
  registry-backed pattern catalogue, rayon parallelisation, `--artifact` flag
  for non-source scanning, and a cross-engine fixture suite that runs identical
  inputs through the Rust and legacy TS scanners. Trust-boundary docs added;
  module closed via ADR-026
- **Scanner parity gaps closed (`SPG-001..006`)** — every shipped registry rule
  has a fixture, the antipattern scan has a Criterion bench, custom pattern
  compile errors surface at every secret-scan call site, and `flags:"i"` is
  honoured. Trust-boundary documentation added
- **napi-rs prebuild bridge for the legacy TS engine** (`TSRET-001`,
  `TSRET-002`) — full prebuild matrix across darwin x86/arm, linux x86/arm,
  windows x86/arm. ADR-030 supersedes the rest of TSRET (cutover from napi to
  surface drivers); pattern-registry getters added for the eventual driver
  bridge (`TSRET-003` prep)

### Workspace Hardening (RUSTNX)

- **`cargo-hakari` workspace-hack** generated and applied to every member crate
  to flatten transitive dependency feature unification (`RUSTNX-008`); internal
  crates marked `publish = false`
- **`cargo-deny` policy** added for licences, security advisories, and banned
  crates; CI gate runs the policy on every Rust PR (`RUSTNX-009`)
- **`cargo-about`** generates `ACKNOWLEDGEMENTS.md` with licence text for every
  transitive dependency; the new `anvil licenses` command surfaces it at runtime
- **`cargo-nextest`** adopted for CI test runs with per-target rust-cache keying
  (`RUSTNX-001`, `RUSTNX-002`)
- **Parallelised clippy + test jobs** in Rust CI (`RUSTNX-003`); test coverage
  cache pinned on `cargo-llvm-cov` version
- **Rust CI scope tightening** — affected-crate detection uses the PR base ref;
  vercel deployments gated by app-specific changes; nx-rust inferred targets pin
  the cargo package so vitest flags don't leak
- **Repository cleanup** — unused workspace dependencies dropped; APS module
  hygiene reconciled

### Notification Framework (NOTIFY)

- **Discovery and architecture phase** (`NOTIFY-001..005`) — inventory of
  current notification streams, taxonomy and priorities defined in
  `docs/anvil/quality/notifications`, delivery architecture and execution slices
  specified
- **Shared `Notification` envelope** (`NOTIFY-006..009`) — `check`, `gate`,
  `audit`, doctor, watch, tutorial, and onboarding/hooks all emit one envelope
  shape; subscriber filter contract documented; class and priority versioning
  surfaces in JSON outputs. `NotificationSource` trait in `anvil-tui` exposes
  current notices for future telemetry subscribers
- **Doctor JSON v2 contract** — `anvil doctor --json` now returns a root object
  with `checks`, `notifications`, and `schema_version`; every check carries a
  structured remediation object with summary, optional command, and optional
  docs URL

### Feature-Flag Plumbing (FLAGM)

- **`cli.licence-gate`** drives `requires_auth` and the `ANVIL_DEV=1` local
  override; bypass details (flag key, variant, reason) surface in verbose logs
  (`FLAGM-001..003`)
- **Admin invite path** moved from inline scope arrays to the shared flag
  resolver (`FLAGM-004`)
- **`anvil-api`** scope gates routed through `api.scope.*` flags (`FLAGM-005`);
  `/anvil` docs gate uses the same resolver
- **Dual-evaluation shims retired** (`FLAGM-006`); FLAGM module closed

### Admin / API / Operations

- **Per-operator admin keys** (`ADMINCLIH-001..004`) — peppered-hash lookup in
  `anvil-api`; per-operator key provisioning automated via Pulumi;
  send-migration uses a snapshot-token flow for atomicity; `--json` warning
  handling, AdminWriter hoist, stdout/stderr hygiene tightened
- **anvil-API route coverage** — route-level tests for `/session/refresh`
  rotation (#777), auth-device flow (#665, #777), auth-otp flow (#672),
  auth-github callback (#787), waitlist + send-migration coverage gaps
- **Admin contracts** — ISO-8601 offset enforced on `IsoTimestamp`; missing
  README dropped from files manifest; zod schemas validate API responses
- **Email correction follow-on** — admin endpoint for email-mismatch repair
- **Watcher cwd redaction** (#1017) — error chains no longer leak working
  directory paths

### CLI / TUI Polish

- **Watch reliability** — partial setup survival, per-change panic isolation,
  SIGINT forwarding, redacted error chains
- **Watch filtering** — `--patterns` and `--exclude` now feed the watch loop
  instead of being declared-only flags; `--exclude` uses glob semantics, bare
  likely-directory names warn, and the plain-mode startup banner prints the
  active include/exclude scope
- **Watch animation** — animated stats and demo overlay; animations driven by an
  event loop instead of busy-spin
- **Onboarding** — post-init landing screen, shared default checks across
  welcome and init, ASCII fallback, title fit, TOCTOU fix on `.anvilrc`
  detection, `ANVIL_NO_PROMPT` / `NONINTERACTIVE` empty-string handling, login
  prompt for gated commands lacking credentials
- **`anvil init` first-touch diagnostics** — init runs an inline sample analysis
  with top warnings, counts, and `file:line` pointers; empty repos receive a
  tutorial/watch next-step hint. Git-history sampling is bounded by a timeout,
  and low inotify headroom surfaces as a fixable hint before watch startup
- **Doctor remediation safety** — non-passing checks expose runnable commands,
  docs links, or fix prompts in plain mode and TUI detail panels; `doctor --fix`
  writes a valid YAML `.anvilrc` and refuses unsafe `git init` in directories
  without project markers
- **Tutorial** — verify-step `husky` flow, scan truncation visibility,
  watcher-failure cause in static-mode notice, language model aligned with TUI
  surfaces
- **Welcome hub navigation** — arrow keys move through panel rows, list panels
  scroll, and unfocused panels freeze their scroll state
- **`.anvilrc` gate-check vocabulary** reconciled with the runner; legacy check
  names mapped during transition (#1016, #1041)

### Distribution (DIST-011)

- **Scoop bucket** published with PR-based update flow
- **WinGet icon** shipped; `IconSha256` template placeholder quoted; PR body
  written to file with `set -e` to fail on errors
- **README install section** covers every supported package manager
- **ADR-025** records the package-manager distribution strategy

### Test Infrastructure

- **OPA policy hardening (`TCOV-009..013`, `TFIX`)** — pinned `cargo-llvm-cov`
  with version-keyed cache; real-binary OPA integration tests; hermetic OPA via
  PATH-isolated tests; `rego.v1` import for OPA 0.60 compatibility;
  cross-platform OPA binary lookup
- **Tests covering 7 GH issues** closed during the cycle (#558, #672, #665,
  #777, #787, #723, #1052)
- **Transactional smoke tests** for email render templates
- **API neon SQL** fragment composition pinned for admin list queries

### Tooling & DX

- **Agent-driven release** (`RELMGMT` Phase 3) — `/release` skill drives version
  pick, branch strategy, tag, workflow, artefact verification, comms, and
  cleanup; reads live `git`/`gh` state each turn
- **nxrust plugin** now consumed from npm as `@eddacraft/nxrust`; inferred
  targets via `cargo metadata`; per-crate `project.json` no longer needed
- **Husky pre-commit** enforces `oxfmt` on markdown and TOML so format drift
  surfaces locally rather than in CI
- **`scripts/release.sh`** preflight runs Rust + TS fmt/lint/typecheck/test as
  one bundled gate; supports `ANVIL_RELEASE_STEP_TIMEOUT` for repos where the
  parallel nx test run exceeds 600 s

### Plan Hygiene & Documentation

- **9+ completed APS modules archived** (POLISH, RCLI, MAINT, ADMINCLI,
  ADMINCLIH, anvil-scanner-parity-gaps, NXRUST, DBCON, DIST, TUTOR); counts
  reconciled across `plans/index.aps.md`
- **WEAVE module renamed from LCORE** for the standalone weave-rs strategy;
  ADR-024 amended; design spec and implementation plan published
- **ATTRIB v3 attribution pipeline module** authored (Ready); `deny.toml`
  references aligned
- **`next-steps.md`** added as session-continuity artefact for cold restart
- **ADR-030 surface drivers supersede napi cutover** — TSRET-003/-004 superseded
  by DRVR; X5 sequencing decision closed
- **Quality model and follow-on plans** documented in `docs/anvil/quality/`
- **Public docs refresh** — release pages, historical release pages, pattern
  reference, install section, runbooks (`DOCSYNC`)

### Dependencies & Security

- **TypeScript bumped to `~6.0.3`** across all workspaces
- **Production-deps group bump** (10 updates) and **development-deps group
  bump** (16 updates)
- **Pre-release security sweep** — `EAMIG-003`, `EAMIG-046` security overrides
  applied
- **CI dependencies** — `pnpm/action-setup` 6.0.0 → 6.0.3, `setup-node` 6.3.0 →
  6.4.0, `trufflehog` 3.94.3 → 3.95.2, `trivy-action` 0.35.0 → 0.36.0,
  `setup-regal` 1.0.0 → 2.0.0
- **`cargo-deny`** version pinned at 0.19.4

## [0.3.3-beta]

### Distribution & Release Engineering

- **WinGet distribution pipeline** — Windows release automation now emits and
  submits WinGet manifests for tagged releases, extending the binary
  distribution surface beyond direct install scripts and Homebrew formulae
- **Windows signing groundwork** — Authenticode signing path wired into the
  release pipeline via Azure Trusted Signing and SSL.com integration so Windows
  artefacts can move to signed distribution once identity provisioning clears
- **Release automation hardening** — `scripts/release.sh` tightened around
  preflight validation, bundled test execution, remote state checks, and
  manifest handoff to the release skill
- **Public release promotion** — release automation now flips production GitHub
  releases to `Latest` consistently instead of leaving beta-tagged artefacts
  hidden behind cargo-dist defaults

### CLI & TUI

- **Windows input handling** — Ratatui/crossterm event handling on Windows now
  filters to key-press events only, removing duplicate input in onboarding and
  discovery flows
- **Discovery surface repair** — two-panel layout restored with predictable
  scrolling behaviour and a reliable onboarding reset path
- **Tutorial completion fixes** — tutorial exit code handling, `husky` flow, and
  verify-step sentinel behaviour corrected so scripted onboarding paths complete
  deterministically
- **Installer UX polish** — post-install output now prints a branded next-steps
  block with colour support and direct pointers to `anvil auth login` and
  `anvil welcome`
- **Admin CLI surface expansion** — the admin command set moved from endpoint
  groundwork to an operational CLI with `list`, `show`, `approve`, `invite`,
  `audit`, `revoke`, and `send-migration` flows layered over the beta-user
  service APIs
- **CLI hardening wave** — admin command paths now validate flags earlier,
  detect TTY state from stdin and stderr together, sanitise control characters
  in rendered tables, align audit types to the server contract, and make error
  handling more testable and explicit

### API & Operations

- **Admin list endpoints** — read-only waitlist and audit-list surfaces added in
  support of the in-progress admin CLI module (`ADMINCLI-001`–`ADMINCLI-004`)
- **Licence key boot probe** — `anvil-api` now validates the ES256 signing key
  during startup and reports status through `/health`, surfacing secret/config
  failures before auth traffic hits runtime paths
- **Admin approval collision handling** — approval flow now retries `user_code`
  uniqueness collisions and accepts longer codes to reduce back-to-back approval
  failures
- **Structured auth logging** — waitlist and auth routes emit more consistent,
  structured operational logs for support and production debugging
- **DBCON groundwork** — database consolidation module introduced for the Neon
  project merge, including operator-only waitlist pause controls and bridge
  migration work
- **Email correction path** — auth UX now handles email mismatch more clearly,
  and the admin API exposes an email-update endpoint so operators can repair
  beta-user addresses without direct database edits
- **Migration operations** — admin migration sending now has an operator
  runbook, correct dry-run semantics, and non-zero failure exits for automation
  safety
- **DBCON follow-on work** — option-B reset flow started,
  `ANVIL_API_DATABASE_URL` rename introduced, and verification/snapshot steps
  hardened for the next Neon cutover stage

### CI, Benchmarking & Security

- **Nightly stress benchmarks** — benchmark runner added to CI to catch native
  engine performance regressions outside the tagged release path
- **Dependency remediation** — `follow-redirects` pinned to a non-vulnerable
  range to close a known supply-chain issue
- **ADR coverage** — ADR-024 published for the literate-core agent harness; KERN
  and BENCH modules archived after completion
- **Toolchain refresh** — pnpm, Cargo crates, and selected GitHub Actions moved
  forward during the release window to keep the admin CLI and release pipeline
  on current dependency baselines

## [0.3.2-beta]

### CLI Surface

- **Self-update command** — `anvil update` added as an in-place binary updater
  with version detection, asset download, and verification flow (`RCLI`)
- **Admin invite command** — `anvil admin invite` shipped with dual-mode invite
  flow (email plus approval path), extending beta-user operations from the CLI
- **Welcome/onboarding completion** — all WELCOME tasks closed, finishing the
  first-run path with discovery mode, executable tutorial steps, live watch
  demo, fix flow, and hook installation guidance

### Release & Platform

- **Interactive release script** — `scripts/release.sh` now orchestrates
  preflight, branching, tagging, and workflow kickoff, and writes
  `.release/manifest.json` as a handoff contract for the release skill
- **Feature flag operations docs** — feature-flag inventory and governance
  guides published to make ad-hoc flags auditable across runtime surfaces
- **Windows target expansion** — `aarch64-pc-windows-msvc` added to cargo-dist
  configuration, with updater support explicitly deferred pending upstream
  binary availability

### Reliability & Codebase Maintenance

- **OTP query determinism** — `ORDER BY` restored in `findActiveOtpCodes` to
  prevent non-deterministic code selection under concurrent auth traffic
- **SQL centralisation** — inline API-route SQL moved into `db/queries.ts` to
  make data access easier to audit and less error-prone
- **Tutorial/TUI fixes** — tutorial commands brought back in sync with the Rust
  CLI and long audit result lists fixed to scroll correctly
- **Install flow repair** — installer next-step output now prints reliably; the
  Homebrew tap publish path is triggered automatically during release
- **CI stability** — Semgrep version pinned to avoid upstream breakage and OSSF
  Scorecard restricted to the default branch to reduce noisy failures

### Planning & Governance

- **Versioning decision recorded** — ADR-020 published for release/versioning
  policy
- **Decision log introduced** — `DECISION-LOG.md` added as the single-entry
  index for ADR discovery
- **APS maintenance** — completed modules archived and APS workflow rules
  tightened to keep release and planning state aligned
- **Coverage uplift** — 59 unit tests added for previously under-covered
  `anvil-cli` modules (`TCOV`)

## [0.3.1-beta]

### Infrastructure

- **Feature flags module** — shared feature flagging system across TypeScript
  and Rust surfaces (`FLAGS-001`–`FLAGS-009`)
  - Contract schema with JSON Schema validation
  - Runtime resolver with environment-aware flag evaluation
  - Snapshot system for point-in-time flag state capture
  - Telemetry hooks for flag evaluation tracking
  - Exemplar test fixtures
  - Kernel-side feature flag types, resolver, and snapshot mirroring TS surface
  - Feature flag governance, inventory, and reference guides
  - ADR-019: flags–observability alignment decision
- **CI composite actions** — `setup-workspace` action extracted to deduplicate
  Node/pnpm/Nx setup across workflows; `detect-changes` action for path-based
  job filtering
- **CI workflow fixes** — 8 issues resolved: checkout ordering, clippy/rustfmt
  failures, formatting in setup-workspace action
- **Docs-shell app** — Next.js shell application for docs domain proxy with auth
  callback, login, logout routes, JWT/cookie/state libraries, and unit tests
- **Docs upstream scaffolding** — Docusaurus apps for private and public docs
  with middleware, sidebar configs, and Vercel deployment configuration
- **Vercel build skip** — `vercel-ignore-build.sh` script for skipping preview
  deploys on non-release branches

### Documentation & Delivery

- **PR template** — durable link requirement for manual testing; rationale moved
  to section comment
- **README** — lowercase brand usage, Windows aarch64 scope clarification,
  'Anvil Check' action name restored
- **Release doc checklist** — public distribution repo section added; broken
  inline code span fixed; oxfmt formatting applied
- **AGENTS.md** — updated with current conventions

### Dependencies

- pnpm dependency upgrades (vitest, vite, @types/node, globals, @nx/eslint,
  @nx/vite, @vitest/coverage-v8, @github/copilot)
- Cargo dependency upgrades via Cargo.lock refresh
- lint-staged configuration updated

### Tooling

- `aps-cleanup.service` — systemd service for APS status lifecycle automation
- `nx.json` configuration updates

## [0.3.0-beta]

### Platform Foundations

- **Language and runtime baseline** — TypeScript moved to 6.0 across workspace
  packages and the Node engine floor was raised to `>=22`, reducing divergence
  between local and CI environments (`MAINT-011`)
- **Rust toolchain uplift** — toolchain advanced to 1.94.0 with Windows and
  macOS cross-compilation support aligned to the release matrix
- **Linting and formatting refresh** — oxlint adopted as the first-pass linter
  and oxfmt replaced Prettier for the primary formatting path
- **Documentation platform refresh** — Docusaurus upgraded to 3.10 to keep the
  docs stack current with the Rust CLI release

### Performance & Verification

- **Kernel benchmarking in CI** — Criterion benchmarks for critical kernel paths
  and the stress-test harness were wired into CI, with execution scoped to main
  pushes and manual dispatch where appropriate (`BENCH`)
- **Test coverage uplift** — 59 unit tests added for under-covered `anvil-cli`
  modules alongside an integration suite for the checks crate
- **CI modernisation** — GitHub Actions refreshed to current major versions,
  unused jobs removed, and CodeQL added with path scoping to improve signal and
  maintainability

### Architecture & Dependency Governance

- **Dependency refresh** — key build and runtime dependencies updated, including
  Criterion 0.8, Reqwest 0.13, Dirs 6, and Vite 8
- **Architecture decisions published** — ADR-015 (shared packages restructure)
  and ADR-016 (unified config format) recorded the main design decisions behind
  the release

## [0.2.1-beta]

### Platform & Integration

- **Edda/Ember/Stack integration** — contracts and service-layer work matured
  the project-memory foundation introduced in the 0.2.x beta line

### Security & Hardening

- **Parser and adapter hardening** — validation tightened across parsers,
  adapters, and the APS plan loader to reduce malformed-input and edge-case risk
- **Subprocess execution hardening** — command execution paths further locked
  down to reduce shell-safety regressions
- **Dependency remediation** — vulnerable dependencies patched, including
  `minimatch`, `axios`, `svgo`, and `tar`

## [0.1.3]

### CLI & Delivery

- **CLI stream policy** — stdout/stderr behaviour standardised so automation and
  human-readable output are easier to consume consistently
- **Hook script consolidation** — Git hook scripts moved to a single source of
  truth to reduce drift across local and CI execution
- **Default API endpoint update** — default backend URL moved to
  `eddacraft-api.vercel.app`

### Architecture

- **Rust engine decision recorded** — ADR-011 published the architecture
  decision for the Rust core engine direction
