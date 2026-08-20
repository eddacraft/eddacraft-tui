# DOCRB-005 Component Truth Migration Evidence — 2026-08-20

| Type   | Authority | Owner | Status | Freshness |
| ------ | --------- | ----- | ------ | --------- |
| Review | Advisory  | DOCRB | Open   | Implementation evidence assembled 2026-08-20 from exact base `f0f834b39` and migration content commit `b9c3e5898`; exact-head independent verification and Council pending |

| Upstream | Downstream |
| -------- | ---------- |
| ADR-123, `plans/execution/DOCRB-005.actions.md`, `plans/specs/2026-08-17-docrb-corpus-disposition.md`, current component source and tests | DOCRB-005 verification, Council, and pull request |

## Scope and immutable revisions

DOCRB-005 started from exact `origin/main`
`f0f834b39bbdbc3ff9c8c198ec6098f3afc33389` in the isolated Worktrunk
`docs/docrb-005-component-truth-migration`. The immutable migration-content
commit is `b9c3e5898f6fc89426b9d036200c0195ed71e4f0`. A later evidence-only
commit records docs-owed repairs, this report, and exact-head review results
without changing product behaviour.

The final bounded feature scope is 46 exact paths:

- 14 central move/merge architecture paths;
- 18 component-local authority or README discovery paths;
- 3 APS planning paths, including the index's DOCRB-005 lifecycle row;
- 9 fixed authority, discovery, generated-index, and evidence closeout paths;
- 2 binding file-level docs-owed repairs.

The two docs-owed additions are
`docs/architecture/rust-mcp-server-spec.md` and
`docs/reviews/2026-08-16-gctx-dogfood-failure-points.md`. Both previously
depended on `mcp-shim-as-built.md`, which is now a deprecated compatibility
record. Manual review confirmed their handshake claims against
`mcp_client.rs::all_clients`, `probe_all`, and `agent_registry.rs`; the
edits only repoint current runtime authority and refresh review metadata. Four
directory-granularity advisories remain untouched.

Independent verification found that the initial path matrix omitted
`plans/index.aps.md`: its NBI and module-catalogue prose still called
DOCRB-005 Draft/Schedule after the authorised module transition to In Progress.
The bounded correction changes only DOCRB-005 lifecycle truth. It leaves the
aggregate at 7/11 and does not promote DOCRB-008 or another item.

No public DOCSYNC content, docs-site/start-here retirement, product code,
configuration, checker subsystem, mandatory CI rule, sibling APS status, or
DOCRB-008/-009 implementation is in this change.

## Inventory reconciliation

At the exact base, all 14 move/merge central candidates still existed and
totalled 11,787 lines. The retire rows were already resolved: the monitor
document is archived and DOCRB-006 removed both retired central Draw.io
sources. `auth-as-built.md` already named docs-shell and the current
private/public renderers and contained no docs-site-as-live wording, so its
DOCRB-005 repair was a verified no-op.

| Central path | Current disposition | Successor or retained authority |
| ------------ | ------------------- | ------------------------------- |
| `kernel-as-built.md` | Deprecated compatibility/history record | `crates/anvil-kernel/ARCHITECTURE.md` |
| `checks-as-built.md` | Slim live cross-system registry-to-consumer map; historical component snapshot retained | `crates/anvil-checks/ARCHITECTURE.md` plus `quality-model.md` |
| `intercept-as-built.md` | Deprecated compatibility/history record | `crates/anvil-intercept/ARCHITECTURE.md` plus central trust/save views |
| `mcp-shim-as-built.md` | Deprecated compatibility/history record | `crates/anvil-cli/ARCHITECTURE.md` and the active Rust MCP spec |
| `activation-as-built.md` | Deprecated compatibility/history record | `crates/anvil-cli/ARCHITECTURE.md` and activation runbooks/ADRs |
| `cli-tui-runner-as-built.md` | Deprecated compatibility/history record | CLI architecture, linked to TUI authority |
| `tui-as-built.md` | Deprecated compatibility/history record | `crates/anvil-tui/ARCHITECTURE.md` |
| `widgets-as-built.md` | Deprecated compatibility/history record | anvil composites in TUI architecture; shared contracts in `eddacraft-tui/README.md` |
| `api-as-built.md` | Deprecated compatibility/history record | `apps/anvil-api/ARCHITECTURE.md`; auth remains central |
| `driver-framework-as-built.md` | Slim live cross-system protocol/client/daemon/rules/Windows map; historical component snapshot retained | driver-client architecture and intercept-proto README |
| `observability-as-built.md` | Deprecated compatibility/history record | `crates/anvil-observability/README.md` |
| `capsule-as-built.md` | Deprecated compatibility/history record | `crates/anvil-capsule/README.md` |
| `adapter-packages-as-built.md` | Deprecated compatibility/history record | adapters, APS tooling, and kindling-integration READMEs |
| `tutorial-as-built.md` | Deprecated compatibility/history record | `crates/anvil-tui/ARCHITECTURE.md` |

Checks, driver, and auth are the three retained live central concerns because
their trust or composition boundaries span multiple owners. Component
implementation detail has one local live authority. Deprecated central paths
retain successor links, governing decisions, retained cross-system links, and
an explicit Git-history route; they are not content-free tombstones.

The corpus contained a placement conflict for observability, capsules, and
adapters: component rows classified them README-only while later central rows
named or offered an `ARCHITECTURE.md`. DOCRB-005 preserves the more specific
component classification. Current leaf invariants moved to READMEs, stale
counts and resolved-gap narratives remain historical, and no unnecessary
architecture unit was created.

## Per-section and material-source disposition

Legend: **L** is a local successor, **X** is a retained cross-system authority,
**H-body** is a full historical snapshot retained in the central file, and
**H-git** is omitted compatibility-record detail recoverable with
`git log --follow`.

| Input | Complete base section and source-link coverage | Disposition and proof |
| ----- | ---------------------------------------------- | --------------------- |
| `kernel-as-built.md` | **L/X:** §1 overview; §3 diagram; §4 layout; §5 watcher (§5.1–5.4); §6 parser (§6.1–6.5); §7 graph (§7.1–7.7); §8 policy (§8.1–8.4); §9 protocol (§9.1–9.3); §10 embedded API (§10.1–10.4); §11 loop (§11.1–11.4); §12 dispatcher; §13 cross-cutting (§13.1–13.5); §14 performance; §16 source groups for kernel, benches, tests, and kernel-types; §17 related authorities. **H-body:** §2 spec reconciliation and §15 G-01..G-06. | Current kernel orchestration/invariants are in `crates/anvil-kernel/ARCHITECTURE.md`; graph ownership and cross-system context link to graph-cache and `rust-architecture-overview.md`. Dated budgets, spec reconciliation, gaps, and the exhaustive source catalogue remain in the central historical body. |
| `checks-as-built.md` | **L/X:** §1 overview; §2 diagram; §3 model; §4 registry (§4.1–4.8); §5 finding; §6 suppressions; §7 language profile; §8 baseline/drift; §9 surfaces (§9.1–9.5); §10 performance; §11 cross-cutting (§11.1–11.5); §13 source groups for checks, AST/NAPI, kernel types, intercept rules, and CLI seams; §14 related authorities. **H-body:** §12 G-01..G-09. | Registry, family, finding, suppression, and performance truth is in `crates/anvil-checks/ARCHITECTURE.md`. Registry-to-CLI/daemon/MCP/baseline composition remains the slim live central concern; `quality-model.md` retains conceptual authority. Old counts/gaps and the exhaustive catalogue remain historical. |
| `intercept-as-built.md` | **L/X:** §1 overview; §2 architecture; §3 process; §4 IPC (§4.1–4.5); §4a save-time (§4a.1–4a.6); §5 trust; §6 fence; §7 persistence; §8 recovery; §9 interrupt; §10 registry; §11 telemetry/DoS; §12 embedded; §13 redaction; §14 CLI; §15 Win32; §17.1–17.4 intercept/proto/rules/Win32 source groups; §18 related authorities. **H-body:** §16 known gaps. | Daemon lanes, admission, guarded reads, fencing, and failure invariants are in `crates/anvil-intercept/ARCHITECTURE.md`. Protocol/client/trust relationships remain in driver, save-to-validation, and trust-boundary authorities. Gaps and the full old source map remain historical. |
| `mcp-shim-as-built.md` | **L/X:** §1 overview; §2 diagram; §3 process (§3.1 dual-era); §4 tools (§4.1–4.4); §5 routing; §6 fallback; §7 correlation; §8 enforcement; §9 redaction; §10 install; §11 config; §12 call path; §13 cross-cutting; §15.1–15.3 MCP/command/cross-crate source groups; §16 related authorities. **H-git:** §14 G-01..G-06 and the exhaustive catalogue. | Current process, registry, containment, enforcement, and fallback are in `crates/anvil-cli/ARCHITECTURE.md#mcp-shim`; design intent remains in `rust-mcp-server-spec.md`, with daemon/capability context linked. The compatibility record names those successors and the exact Git-history command. |
| `activation-as-built.md` | **L/X:** overview; diagram; protection vocabulary; mutating and read-only lifecycles; watch fallback; save-time routing; language profile; MCP install; diagnostics; cross-cutting honesty/idempotency/state/exclusions; tutorial integration; version; activation/command source groups and related authorities. **H-git:** G-01..G-04. | Orchestration, `ProtectionState`, client registry/install, and failure honesty are in `crates/anvil-cli/ARCHITECTURE.md#activation-orchestration`. System/auth/spec/runbook concerns remain linked; dated rollout/gaps are routed to Git history. |
| `cli-tui-runner-as-built.md` | **L/X:** overview; diagram; `SurfaceExit`; `surface_loop`; animation; watch/tutorial loops; shared terminal; errors; panic/thread/determinism/global-state invariants; `tui.rs` symbols, command call sites, and related authorities. **H-git:** G-01..G-06. | Terminal lifecycle, event loops, dirty redraw, panic containment, and channel failure are in `crates/anvil-cli/ARCHITECTURE.md#cli-tui-runner`; surface/render authority links to TUI. Old counts/gaps are routed to Git history. |
| `tui-as-built.md` | **L/X:** overview; diagram; layout; dispatcher; every named surface; watch model/adapter/render/footer/notifications; tutorial; welcome/wizard/onboarding; doctor/status/audit/gate/browser/init; widget vocabulary; snapshots; determinism/compatibility/migration/zoom; TUI source modules and CLI consumers. **H-git:** G-01..G-07. | Anvil surface/state/render/snapshot truth is in `crates/anvil-tui/ARCHITECTURE.md`. Terminal polling remains CLI-owned, shared primitives remain eddacraft-owned, and dashboard context remains central. Counts/gaps are routed to Git history. |
| `widgets-as-built.md` | **L/X:** overview; crate resolution; diagram; theme; keyboard; every upstream widget; anvil composites; snapshot pinning; determinism/theme/zoom/mouse/Unicode invariants; upstream, downstream, and consumer source groups. **H-git:** G-01..G-07. | Shared contracts remain in `crates/eddacraft-tui/README.md`; anvil composites/presentation are in `crates/anvil-tui/ARCHITECTURE.md`. The compatibility record links both owners and routes the exhaustive old catalogue/counts/gaps to Git. |
| `api-as-built.md` | **L/X:** §1 overview; §2 diagram; §3 cold start; §4 routes (§4.1); §5 middleware (§5.1–5.4); §6 DB (§6.1–6.3); §7 migrations (§7.1–7.4); §8 migration history; §9 health/observability (§9.1–9.4); §10 deploy; §12 cross-cutting (§12.1–12.4); §14 live API and archived CLI source groups; §15 related authorities. **H-body:** §11 archived Node CLI (§11.1–11.4) and §13 G-01..G-07. | HTTP, persistence, migration, health, and failure invariants are in `apps/anvil-api/ARCHITECTURE.md`; authentication remains in the central auth map. Archived CLI, rollout details, gaps, and the full old catalogue remain in the historical body. |
| `driver-framework-as-built.md` | **X/L:** §1 overview; §2 diagram; §3 wire (§3.1–3.5); §4 enforcement; §5 trust (§5.1–5.3); §6 TS client (§6.1–6.5); §7 Rust clients; §8 rules (§8.1–8.4); §9 Win32; §10 capability; §11 cross-cutting (§11.1–11.4); §12 spec/code reconciliation; §14 proto/rules/client/Win32 source groups; §15 related authorities. **H-body:** §13 gaps. | Client internals are in `packages/anvil-driver-client/ARCHITECTURE.md`; protocol orientation is in `crates/anvil-intercept-proto/README.md`. The multi-owner wire/version/capability/trust map remains live centrally; gaps and the exhaustive old map remain historical. |
| `observability-as-built.md` | **L:** §1 overview; §2 diagram; §3 layout; §4 subscriber and all six subconcerns; §5 namespace; §6 redaction; §7 traceparent; §8 cross-cutting; §10 manifest/source/consumer groups; §11 related authorities. **H-body:** §9 G-01..G-05. | Entry points, redaction/traceparent invariants, and failures are in `crates/anvil-observability/README.md`. Dated diagram/counts/gaps and the exhaustive source map remain in the central historical body. |
| `capsule-as-built.md` | **L/X:** overview; diagram; create/verify/prune lifecycle; surfaces; canonical digest, empty discipline, schemas, witness chain, Git integrity, and determinism; capsule source/manifest/CLI/test groups and related authorities. **H-body:** G-01..G-06. | Current lifecycle, contract, and failure truth is in `crates/anvil-capsule/README.md`; public explanation and ADRs remain separate. The old diagram, gaps, rollout specifics, and source catalogue remain in the historical body. |
| `adapter-packages-as-built.md` | **L/X split:** §1 overview; §2 diagram; Part 1 and §§3–7 adapters/SpecKit/BMAD/other formats/contracts; Part 2 and §§8–13 APS validation/templates/examples/scripts/schema; Part 3 and §§14–18 kindling contracts/capture/scripts/benchmarks; §19 cross-cutting; §21.1–21.3 three package source groups; §22 related authorities. **H-body:** §20 G-01..G-08 plus dated counts/readiness/status. | Authority is split among `packages/adapters/README.md`, `packages/aps/README.md`, and `packages/kindling-integration/README.md`/`CONTRACTS.md`; no duplicate architecture unit was created. Sibling-module status, stale counts, resolved gaps, combined diagram, and exhaustive catalogue remain historical. |
| `tutorial-as-built.md` | **L/X:** overview; diagram; path inventory; state machine/transitions/per-step/reset; discovery domain/filter/state/render; ProtectionLoop; executor; fix; verify; watch demo; showcase; render affordances; copy invariants; snapshots; determinism/honesty/offline/notifications/reset; tutorial module, CLI, public, and ADR source groups. **H-git:** G-01..G-06. | State/render/copy/snapshot truth is in `crates/anvil-tui/ARCHITECTURE.md#tutorial-engine`; terminal/effect delivery remains CLI-owned and public tutorials remain separate. Dated counts/gaps and the exhaustive catalogue are routed to Git history. |

No material base H2/H3 section or material source-link group was unaccounted
for: every entry above resolves to one current local owner, one retained
cross-system owner, a visibly historical retained body, or an explicit
`git log --follow` route.

## Reproducible replacement commands

All commands below were run from the isolated Worktrunk with
`BASE=f0f834b39bbdbc3ff9c8c198ec6098f3afc33389`; every RED and GREEN command
returned exit `0`.

| Slice | RED command | GREEN command |
| ----- | ----------- | ------------- |
| Pilots | `for n in kernel intercept api; do git show "${BASE}:docs/architecture/${n}-as-built.md" \| rg -q '^\\| As-built \\| Derived .*\\| Live'; done; git show "${BASE}:crates/anvil-kernel/ARCHITECTURE.md" \| rg -q 'remains the implementation-map authority until DOCRB-005'; git show "${BASE}:crates/anvil-intercept/ARCHITECTURE.md" \| rg -q 'DOCRB-005 owns its migration or deliberate retention'; git show "${BASE}:apps/anvil-api/ARCHITECTURE.md" \| rg -q 'remains subordinate to the'; git show "${BASE}:apps/anvil-api/ARCHITECTURE.md" \| rg -q '^> DOCRB-005\.'` -> exit `0` | `for n in kernel intercept api; do rg -q '^\\| As-built \\| Historical .*\\| Deprecated' "docs/architecture/${n}-as-built.md"; done; for p in crates/anvil-kernel/ARCHITECTURE.md crates/anvil-intercept/ARCHITECTURE.md apps/anvil-api/ARCHITECTURE.md; do rg -q 'This document is the live .*component authority' "$p"; done; rg -q '^\[auth as-built\].*remains authoritative' apps/anvil-api/ARCHITECTURE.md` -> exit `0` |
| Checks | `git show "${BASE}:docs/architecture/checks-as-built.md" \| rg -q '^\\| As-built \\| Derived .*\\| Live'; if git cat-file -e "${BASE}:crates/anvil-checks/ARCHITECTURE.md" 2>/dev/null; then exit 1; fi` -> exit `0` | `rg -q '^## Current cross-system authority' docs/architecture/checks-as-built.md; rg -q '^## Historical pre-migration component snapshot' docs/architecture/checks-as-built.md; rg -q 'This document is the live component authority' crates/anvil-checks/ARCHITECTURE.md; rg -q '^\[quality model\].*remains authoritative' crates/anvil-checks/ARCHITECTURE.md` -> exit `0` |
| CLI | `for n in activation mcp-shim cli-tui-runner; do git show "${BASE}:docs/architecture/${n}-as-built.md" \| rg -q '^\\| As-built \\| Derived .*\\| Live'; done; if git cat-file -e "${BASE}:crates/anvil-cli/ARCHITECTURE.md" 2>/dev/null; then exit 1; fi` -> exit `0` | `for n in activation mcp-shim cli-tui-runner; do rg -q '^\\| As-built \\| Derived .*\\| Deprecated' "docs/architecture/${n}-as-built.md"; rg -q 'git log --follow -- docs/architecture/' "docs/architecture/${n}-as-built.md"; done; rg -q '^## Activation orchestration' crates/anvil-cli/ARCHITECTURE.md; rg -q '^## MCP shim' crates/anvil-cli/ARCHITECTURE.md; rg -q '^## CLI TUI runner' crates/anvil-cli/ARCHITECTURE.md; test -d crates/anvil-cli/src/activation; test -d crates/anvil-cli/src/mcp; test -f crates/anvil-cli/src/tui.rs` -> exit `0` |
| TUI | `for n in tui widgets tutorial; do git show "${BASE}:docs/architecture/${n}-as-built.md" \| rg -q '^\\| As-built \\| Derived .*\\| Live'; done; if git cat-file -e "${BASE}:crates/anvil-tui/ARCHITECTURE.md" 2>/dev/null; then exit 1; fi` -> exit `0` | `for n in tui widgets tutorial; do rg -q '^\\| As-built \\| Derived .*\\| Deprecated' "docs/architecture/${n}-as-built.md"; rg -q 'git log --follow -- docs/architecture/' "docs/architecture/${n}-as-built.md"; done; rg -q '^## Surface contract and dispatch' crates/anvil-tui/ARCHITECTURE.md; rg -q '^## anvil-specific widgets and shared widgets' crates/anvil-tui/ARCHITECTURE.md; rg -q '^## Tutorial engine' crates/anvil-tui/ARCHITECTURE.md; test -d crates/anvil-tui/src/surfaces; test -d crates/anvil-tui/src/widgets; test -d crates/eddacraft-tui/src/widgets` -> exit `0` |
| Driver/protocol | `git show "${BASE}:docs/architecture/driver-framework-as-built.md" \| rg -q '^\\| As-built \\| Derived .*\\| Live'; if git cat-file -e "${BASE}:packages/anvil-driver-client/ARCHITECTURE.md" 2>/dev/null; then exit 1; fi; if git cat-file -e "${BASE}:crates/anvil-intercept-proto/README.md" 2>/dev/null; then exit 1; fi` -> exit `0` | `rg -q '^## Current cross-system authority' docs/architecture/driver-framework-as-built.md; rg -q '^## Historical pre-migration component snapshot' docs/architecture/driver-framework-as-built.md; rg -q '^## Request and notification flow' packages/anvil-driver-client/ARCHITECTURE.md; rg -q '^## Contract invariants' crates/anvil-intercept-proto/README.md; test -f crates/anvil-intercept-rules/src/registry.rs; test -f crates/anvil-intercept-win32/src/lib.rs` -> exit `0` |
| README-only leaves | `for n in observability capsule adapter-packages; do git show "${BASE}:docs/architecture/${n}-as-built.md" \| rg -q '^\\| As-built \\| Derived .*\\| Live'; done; if git cat-file -e "${BASE}:crates/anvil-observability/README.md" 2>/dev/null; then exit 1; fi; if git cat-file -e "${BASE}:crates/anvil-capsule/README.md" 2>/dev/null; then exit 1; fi; if git show "${BASE}:packages/adapters/README.md" \| rg -q '^## Current component authority'; then exit 1; fi` -> exit `0` | `for n in observability capsule adapter-packages; do rg -q '^\\| As-built \\| Historical .*\\| Deprecated' "docs/architecture/${n}-as-built.md"; done; rg -q '^## Entry points and flow' crates/anvil-observability/README.md; rg -q '^## Lifecycle' crates/anvil-capsule/README.md; rg -q '^## Current component authority' packages/adapters/README.md; test -f packages/aps/README.md; test -f packages/kindling-integration/README.md` -> exit `0` |

## Replacement RED and GREEN

### Pilots and checks

RED proved four central paths asserted live component authority, the three
pilots deferred to central authority, and the checks local architecture was
absent. GREEN proved local kernel, intercept, API, and checks authority is
discoverable; central compatibility/history routes are explicit; checks alone
retains the required slim cross-system consumer map; owned metadata, source
paths, links, Markdown lint, formatting, and diff checks pass.

Fresh source review covered kernel watcher/parser/graph/policy/embedded/watch
flows; intercept IPC, admission, guarded reads, buffer/save lanes, assurance and
fencing; API middleware/routes/health/persistence/migrations; and the checks
registry, suppression, redaction, performance boundaries, and current families.
Dated rollout, benchmark, known-gap, and resolved-gap prose was not promoted as
live truth.

### CLI and TUI

RED proved both local architecture files were absent, six central paths claimed
live authority, and local README discovery was absent. GREEN proved local
activation/MCP/terminal and TUI surface/widget/tutorial authority, current
source paths, shared-widget ownership, successor/history routes, and the
absence of duplicate central live authority.

Source review covered activation state/orchestration/client registry, MCP
transport/registry/enforcement/fallback, terminal lifecycle and panic
containment, anvil surface composition, eddacraft shared widget contracts, and
the tutorial state/executor/file-change/snapshot paths. Focused tests passed:

- `cargo test -p eddacraft-anvil registry_lists_registered_tools`: 1 passed;
- `cargo test -p eddacraft-anvil-tui protection_loop_copy --lib`: 3 passed.

### Driver, protocol, and README-only leaves

RED proved the driver client architecture and proto/observability/capsule
orientations were absent and the three central leaf paths still asserted live
authority. GREEN proved every new local path is discoverable, all cited source
roots and repository-local Markdown targets exist, the driver central remainder
is genuinely cross-system, and the leaf central paths are deprecated
compatibility/history records.

Focused claim tests passed:

- `cargo test -p eddacraft-anvil-intercept-proto`: 89 passed;
- `cargo test -p eddacraft-anvil-observability`: 30 passed;
- `cargo test -p eddacraft-anvil-capsule`: 142 passed;
- `pnpm --filter @eddacraft/anvil-driver-client test`: 219 passed, 2 skipped;
- `pnpm --filter @eddacraft/anvil-driver-client typecheck`: passed;
- `pnpm --filter @eddacraft/anvil-adapters test`: 382 passed;
- `pnpm --filter @eddacraft/anvil-adapters typecheck`: passed.

## Discovery, metadata, and history

`docs/architecture/README.md`, documentation governance, the architecture
diagram guide, and the source-pinned corpus disposition now distinguish live
cross-system authority, component-local authority, and deprecated compatibility
records. The canonical generator changed only
`docs/indexes/by-type.md`, `by-authority.md`, `by-owner.md`, and
`by-status.md`; the index README and tag index were already current.

Old central filenames remain so existing inbound links resolve. Material
current invariants and source links were moved or retained; obsolete rollout
and resolved-gap prose is either visibly historical in the compatibility
record or reachable through `git log --follow`. Public guidance, runbooks,
ADRs, active specs, and sibling component authority remain linked rather than
copied.

## Validation evidence

Before the evidence-only closeout, fresh repository gates reported:

- `pnpm format:check`: pass across 1,707 files;
- `pnpm docs:index:check`: pass after canonical generation;
- `pnpm docs:check`: zero errors; only inherited baselined warnings;
- `pnpm aps:active-lint`: 142 files clean;
- `pnpm aps:index:check`: pass;
- `pnpm aps:drift --json`: `findingCount: 0`;
- `git diff --check`: pass.

After the migration commit, `pnpm docs:owed --since f0f834b39` reported two
gating file-level edges and four directory-granularity advisories. The two
gating edges are the bounded link repairs described above; the four advisories
do not justify unrelated freshness churn. The final exact-head gate run and
docs-owed result are appended before review publication.

## Independent review

Independent verify-loop and exact-head Council results are pending. Only
binding in-scope findings will alter this change. Requests for public content,
new checker automation, unrelated component rollout, or DOCRB-008/-009 work are
scope expansion and will not be applied.

## Docs Closeout

- **Doc type:** component architecture/orientation, central cross-system maps,
  deprecated compatibility records, governance, and evidence.
- **Source truth checked:** current component source, tests, schemas, retained
  cross-system views, active specs, ADR-123, and the source-pinned corpus.
- **Links and indexes:** manual local-target trace passed; canonical index
  generation and the full repository link surface are green.
- **Public diagram impact:** unaffected; no public content or diagram asset
  changed.
- **Remaining risk:** exact-head independent verification, Council, and hosted
  checks are pending.
