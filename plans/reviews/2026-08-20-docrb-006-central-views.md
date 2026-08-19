# DOCRB-006 central views evidence — 2026-08-20

| Type | Authority | Owner | Status |
| ---- | --------- | ----- | ------ |
| Review | Advisory | DOCRB | Open |

## Scope and source revision

This report records the implementation evidence for DOCRB-006. The exact
comparison base is
`d9b30b23daef0da05f74a7d44dfa3accd0e03fe7`. Initial source reconciliation
used that revision. Council repair reconciliation used exact head
`97899b00a92a9a29040a854fdfb42e6f2bf03ecf`; targeted
`git diff d9b30b23d..97899b00a -- crates apps infra tools docs/public`
remained empty. There was therefore no product, deployment, tooling, or
public-content source drift between the pinned base, the Council-reviewed head,
and the live-source reads used for the repaired views.

The branch changes documentation, planning, four generated documentation
indexes, and the retirement of exactly two live Draw.io sources. It changes no
product source, package manifest, lockfile, repository script, Mermaid
automation, CI enforcement, public asset, deployment, or release claim.

Anvil's developer-function gate could not authorise the sibling worktree root
directly because the MCP server trusts the primary checkout root. Every
repository write was nevertheless checked through the trusted primary
`/home/aneki/Projects/src/anvil-001` content/write gate before the equivalent
patch was applied in this isolated worktree. No gate returned `block`.

## Result and authority boundaries

The five required DOCRB-006 views are:

1. system context in `docs/architecture/overview.md`;
2. container/component relationships in the same overview;
3. macro trust and deployment boundaries;
4. the cross-owner save-to-validation sequence; and
5. macro documentation delivery.

The overview no longer carries quality-model internals, a generic check
pipeline, or EDDA internals. Its `Check pipeline` and `Gate layer`
compatibility headings link to `quality-model.md` and carry no pipeline or
gate detail. The five required views sit alongside retained central authorities:
KERN owns `quality-model.md`, BAUTH owns `auth-as-built.md`, and EDDA owns
`edda-stack.md`. Component internals remain in component-root
`ARCHITECTURE.md` files and are linked rather than copied.

Independent verification and Council review remain pending. This report is not
a completion, release, or deployment claim.

## Per-view source-edge trace

| View and material edge | Adjacent authority and source trace | Result |
| ---------------------- | ----------------------------------- | ------ |
| System context: developer/editor/CI → local anvil; local auth → hosted API; reader → docs shell → hosted API | `crates/anvil-cli/README.md`, `crates/anvil-intercept/ARCHITECTURE.md`, `apps/anvil-api/ARCHITECTURE.md`, `apps/docs-shell/ARCHITECTURE.md`, `infra/src/vercel.ts` | Pass; actors and cross-system calls only |
| Container/component: CLI/MCP/daemon/kernel/check/TUI/dashboard relationships and hosted API/data/docs containers | `Cargo.toml`, local component architecture documents under `crates/anvil-kernel`, `crates/anvil-intercept`, `crates/anvil-dashboard-server`, `apps/dashboard`, `apps/anvil-api`, and `apps/docs-shell`, plus `infra/src/vercel.ts` | Pass; internal request flows stay local |
| Trust/deployment: native Rust Unix path + connected daemon UID; TypeScript Unix path-only validation and rebound TOCTOU; native Windows server SID; TypeScript SID-derived name and #2484; cross-platform registration lineage separated from Linux-only ancestry/tag/spoof fencing; public and protected API ingress | Native registration/GCTX clients, `crates/anvil-intercept/src/ipc.rs` and `lib.rs`, `crates/anvil-intercept-win32/src/lib.rs`, `packages/anvil-driver-client/src/transport/{unix,windows}.ts`, driver protocol types, API index/routes, and APGOV/BAUTH/docs-shell authorities | Pass; no universal session binding or blanket credentials-before-persistence claim; explicitly records no server-side Unix caller-UID comparison |
| Save-to-validation: MidEdit and PreWrite caller-buffer modes; MidEdit-only best-effort observation; routing eligibility; separate post-save `validate_paths`; scoped failures, unavailable assurance, warn/reconnect; empty-cycle `--all`; live versus unwired fences | `crates/anvil-cli/src/mcp/validation.rs`, `watch.rs`, `watch_save_time.rs`, `save_time_driver.rs`, `crates/anvil-intercept/src/midedit.rs`, `ipc.rs`, `save_time.rs`, `validate_paths.rs`, `interrupt.rs`, `unregistered.rs`, and `fence.rs` | Pass; caller-buffer and post-save lanes stay separate; spoof fence is live; unsafe-interrupt and unregistered/watcher fence paths are labelled defined-but-unwired |
| Documentation delivery: governed sources → private/public builds → deployed renderers; docs shell routing → protected matched upstreams; rollback-only legacy project | Docusaurus configurations, `apps/docs-shell/proxy.ts`, `apps/docs-shell/lib/jwt.ts`, both renderer middleware matchers, `infra/src/vercel.ts`, `infra/src/components/vercel-app.ts`, app `vercel.json` files, and `tools/scripts/vercel-ignore-build.sh` | Pass; `/anvil` entitlement, public routing, renderer `/favicon.ico` exclusion, protected upstream secret, rollback-only `apps/docs-site`, and the DOCRB/DSITE gap remain visible |

The supporting Mermaid blocks in the quality, BAUTH, and EDDA documents were
also checked against their adjacent detailed prose and cited sources. They
remain supporting central authorities outside the five required DOCRB-006 views.

## Mermaid render evidence

After final formatting, all eight exact Mermaid blocks were extracted directly
from their Markdown owners and rendered manually with
`@mermaid-js/mermaid-cli@11.16.0`. All inputs and outputs remained under
`/tmp/docrb006-render.nSx6et`. The successful run used that directory's
temporary `puppeteer.json`, containing only `--no-sandbox` and
`--disable-setuid-sandbox`. No renderer dependency, configuration, output,
script, or generated asset entered the repository.

| Mermaid owner/block | Final SVG bytes |
| ------------------- | --------------: |
| `docs/architecture/overview.md` — system context | 20,159 |
| `docs/architecture/overview.md` — container/component | 32,103 |
| `docs/architecture/trust-and-deployment-boundaries.md` | 38,542 |
| `docs/architecture/save-to-validation.md` | 44,520 |
| `docs/architecture/docs-delivery.md` | 29,092 |
| `docs/architecture/quality-model.md` | 17,620 |
| `docs/architecture/auth-as-built.md` | 17,189 |
| `docs/architecture/edda-stack.md` | 12,548 |

Every final output was non-empty. The repair render exposed sequence-label
`--all` and semicolon tokenisation; the label was made render-safe while the
adjacent prose retained the exact `--all` contract, and the repaired exact
block rendered successfully. All eight recorded byte sizes come from the final
post-format blocks, including both overview outputs.

## Links, duplication, and retirement evidence

The documentation checker traversed repository-local links in the changed
documents. No changed-document outbound link failed. Removing the old overview
pipeline exposed two live inbound references:
`docs/guides/command-safety.md#check-pipeline` and `#gate-layer`;
the overview now provides navigation-only compatibility headings that send
readers to the quality authority without restoring duplicate content.
`CONTEXT.md` and the shipped-codebase review checklist now route Rust
layout/layering directly to `rust-architecture-overview.md` and check/gate
concepts directly to `quality-model.md`.

Focused duplicate-authority assertions proved:

- the overview has exactly two Mermaid blocks;
- trust, save, and docs delivery each have exactly one Mermaid block;
- quality, BAUTH, and EDDA each retain exactly one supporting Mermaid block;
- the overview contains no quality-model, generic-pipeline, or EDDA diagram;
- the five required DOCRB-006 views and retained supporting authorities are
  discoverable from `CONTEXT.md` and the architecture README; and
- no package, lockfile, script, Mermaid automation, or CI path changed.

`docs/architecture/anvil-system-components.drawio` and
`docs/architecture/pptx-workflow.drawio` are the only retired live sources.
The architecture README and corpus disposition label them retired rather than
linking to them. Non-archive occurrences are intentional work-item,
disposition, retirement, or evidence records. The historical repository-tree
reference under
`docs/archive/reviews/eddacraft-prep-2026-03/repo-tree.txt` remains unchanged
as provenance.

## Replacement RED/GREEN evidence

Each vertical slice used a replacement assertion before and after the write:

- system-context RED proved the audience, actors, hosted edge, and authority
  links absent; GREEN proved them present and the overview duplicates absent;
- container/component RED proved the bounded relationship view absent; GREEN
  proved the macro containers and local-authority links present;
- trust RED proved the exact Unix, Linux, Windows, API, and docs trust facts
  absent; GREEN proved all facts and subordinate-authority links present;
- save RED proved the distinct MidEdit, PreWrite, post-save, observation, and
  fence lanes absent; GREEN proved they remained separate; and
- docs-delivery RED proved the source/build/deploy/shell/renderer topology and
  ownership gap absent; GREEN proved the live routing, protected upstreams,
  rollback-only project, and gap present.

Council repair RED/GREEN assertions additionally proved the native/TypeScript
trust split, registration/Linux split, public API ingress, save routing and
fallback state, fence wiring status, renderer matcher exemption,
`TOKEN_PEPPER` production posture, corrected five-view terminology, and
navigation/governance freshness changes were absent before repair and present
after it.

Supporting-authority and retirement assertions then proved the old overview
duplicates and two live Draw.io files were gone while their owning documents
and historical evidence remained.

## Repository gates

The final report-inclusive candidate rerun produced:

| Gate | Exit/result |
| ---- | ----------- |
| `pnpm format:check` | 0; all 1,684 matched files formatted |
| `pnpm docs:index` | 0; six generated indexes refreshed with no owned-field drift |
| `pnpm docs:index:check` | 0; 0 errors, 0 warnings, 6 files checked |
| `pnpm docs:check` | 0; 11/11 surfaces passed, 0 failed |
| `pnpm docs:owed --since d9b30b23d` | 0; exact range reported 0 owed, 0 gating, 0 advisory, 0 review, 0 baselined |
| `pnpm aps:active-lint` | 0; 139 files checked, all clean |
| `pnpm aps:index:check` | 0; inherited DOCDEF stored `0/6` versus computed `1/6` advisory only |
| `pnpm aps:drift --json` | 0; advisory `findingCount: 1`, the same inherited DOCDEF `aps-progress-mismatch` |
| `git diff --check` | 0; no whitespace errors |
| Focused scope/link/duplicate/retirement assertions | 0; all Council repair claims and owned-path constraints passed |

`pnpm docs:check` also reports the repository's baselined link/tag warnings
and corpus-wide docs-owed advisories; its surfaces still pass. Those warnings
are distinct from the explicit exact-range docs-owed result above. The sibling
DOCDEF count/drift advisory is inherited, belongs to
`plans/modules/docs-definition-layer.aps.md`, and is not absorbed into
DOCRB-006 owned files.
