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

The final bounded feature scope is 45 exact paths:

- 14 central move/merge architecture paths;
- 18 component-local authority or README discovery paths;
- 2 APS planning paths;
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
