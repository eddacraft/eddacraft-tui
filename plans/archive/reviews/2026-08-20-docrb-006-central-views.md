# DOCRB-006 central views evidence — 2026-08-20

| Type | Authority | Owner | Status |
| ---- | --------- | ----- | ------ |
| Review | Advisory | DOCRB | Open |

## Scope and source revision

This report records the implementation evidence for DOCRB-006. The original
implementation base was
`d9b30b23daef0da05f74a7d44dfa3accd0e03fe7`; the exact publication comparison
base is `8bf8622e755324452304bd9226830bdf507fcac3`. The rebased
implementation head before this evidence-only reconciliation is
`ce83aa1fcdfbe64ebcb1fb04783757110e46a49b`.

The six documentation patches rebased without conflicts and `git range-diff`
reported every patch unchanged:

| Original commit | Rebased commit |
| --------------- | -------------- |
| `97899b00a92a9a29040a854fdfb42e6f2bf03ecf` | `624f98eea5bb58d98d14152e18c9044dea92cd25` |
| `b217b4c6f2b3a77cb937db3a7ea850a14c55decf` | `cf6e8731c489c1a489bfb1d3278bf9474d6a76a0` |
| `6ce2c7abca5194fe61d4cd9e9d9fb4fbcdf9aa21` | `bd11220c8ca2e6f7559ae0515ed7672e7f9add52` |
| `086a537ef4cdc5ae746afb722ffd62b07d0869f2` | `b5237e3733249448c3124800b432e05e240af3ac` |
| `16e778cd928ed4c46059503b073edd0fdddab861` | `d0dee691e08907f249e8d86b2603f7ce671dda48` |
| `dc245ddc442b1a3a5a2f2f83853fa3d4f51639ca` | `ce83aa1fcdfbe64ebcb1fb04783757110e46a49b` |

The new base adds one source-cited config-catalogue commit across ten upstream
paths. Those ten paths are blob-identical between `8bf8622e7` and the rebased
implementation head, and none overlaps the 24 DOCRB-006 changed paths. The
catalogue, its public navigation, docs-check extension, fixtures, formatting
configuration, and DOCDEF bookkeeping are therefore preserved. They do not
alter the trust, save-validation, deployment, or docs-delivery source facts
traced below; the final gates run with the upstream tooling in place.

The branch changes exactly 24 documentation and planning paths: four generated
documentation indexes, a freshness-only `docs/README.md` downstream
reconciliation, the retirement of exactly two live Draw.io sources, and the
owned source documents. It changes no product source, package manifest,
lockfile, repository script, Mermaid automation, CI enforcement, public asset,
deployment, or release claim.

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

Independent verify-loop and architecture/security Council review both returned
PASS at exact head `91f6c7feb17c634c1803c72af2033f60a3c9bad5`. When this
report was first published, hosted PR checks and merge remained pending. This
report does not itself establish completion, release, deployment, or merge.

## Per-view source-edge trace

| View and material edge | Adjacent authority and source trace | Result |
| ---------------------- | ----------------------------------- | ------ |
| System context: developer/editor/CI → local anvil; local auth → hosted API; reader → docs shell → hosted API | `crates/anvil-cli/README.md`, `crates/anvil-intercept/ARCHITECTURE.md`, `apps/anvil-api/ARCHITECTURE.md`, `apps/docs-shell/ARCHITECTURE.md`, `infra/src/vercel.ts` | Pass; actors and cross-system calls only |
| Container/component: CLI/MCP/daemon/kernel/check/TUI/dashboard relationships and hosted API/data/docs containers | `Cargo.toml`, local component architecture documents under `crates/anvil-kernel`, `crates/anvil-intercept`, `crates/anvil-dashboard-server`, `apps/dashboard`, `apps/anvil-api`, and `apps/docs-shell`, plus `infra/src/vercel.ts` | Pass; internal request flows stay local |
| Trust/deployment: native Rust Unix path + connected daemon UID; TypeScript Unix path-only validation and rebound TOCTOU; native Windows server SID; TypeScript SID-derived name and #2484; cross-platform registration lineage separated from Linux-only ancestry/tag/spoof fencing; public and protected API ingress | Native registration/GCTX clients, `crates/anvil-intercept/src/ipc.rs` and `lib.rs`, `crates/anvil-intercept-win32/src/lib.rs`, `packages/anvil-driver-client/src/transport/{unix,windows}.ts`, driver protocol types, API index/routes, and APGOV/BAUTH/docs-shell authorities | Pass; no universal session binding or blanket credentials-before-persistence claim; explicitly records no server-side Unix caller-UID comparison |
| Save-to-validation: MidEdit and PreWrite caller-buffer modes; MidEdit-only MidEdit observation; independent best-effort post-save `gate_evaluated` observation; routing eligibility; separate post-save `validate_paths`; selected subprocess action with scoped/all `check` versus self-scoped `gate`; scoped failures, unavailable assurance, warn/reconnect; live versus unwired fences | `crates/anvil-cli/src/mcp/validation.rs`, `commands/intercept.rs`, `watch.rs`, `watch_save_time.rs`, `save_time_driver.rs`, `crates/anvil-intercept/src/midedit.rs`, `ipc.rs`, `save_time.rs`, `lib.rs`, `validate_paths.rs`, `interrupt.rs`, `unregistered.rs`, and `fence.rs` | Pass; the first snapshot is skipped, the daemon branch requires a post-initial `check`, non-empty changed paths, and eligible or forced routing, and deletion-driven or otherwise empty post-initial cycles go directly to selected subprocess `check --all`; MidEdit alone enters its observation path, PreWrite does not, and wired `validate_paths` independently observes the save-time verdict after validation and before response; spoof fence is live; unwired fences remain labelled |
| Documentation delivery: governed sources → private/public builds → deployed renderers; docs shell routing → protected matched upstreams; rollback-only legacy project | Docusaurus configurations, `apps/docs-shell/proxy.ts`, `apps/docs-shell/lib/jwt.ts`, both renderer middleware matchers, `infra/src/vercel.ts`, `infra/src/components/vercel-app.ts`, app `vercel.json` files, and `tools/scripts/vercel-ignore-build.sh` | Pass; `/anvil` entitlement, public routing, renderer `/favicon.ico` exclusion, protected upstream secret, rollback-only `apps/docs-site`, and the DOCRB/DSITE gap remain visible |

The supporting Mermaid blocks in the quality, BAUTH, and EDDA documents were
also checked against their adjacent detailed prose and cited sources. They
remain supporting central authorities outside the five required DOCRB-006 views.

## Mermaid render evidence

After the publication rebase and final formatting, all eight exact Mermaid
blocks were extracted directly from their Markdown owners and rendered manually
with `@mermaid-js/mermaid-cli@11.16.0`. All inputs and outputs remained under
`/tmp/docrb006-rebase-render.uHquM4`. The successful run used that directory's
temporary `puppeteer.json`, containing only `--no-sandbox` and
`--disable-setuid-sandbox`. No renderer dependency, configuration, output,
script, or generated asset entered the repository.

| Mermaid owner/block | Final SVG bytes |
| ------------------- | --------------: |
| `docs/architecture/overview.md` — system context | 20,159 |
| `docs/architecture/overview.md` — container/component | 32,103 |
| `docs/architecture/trust-and-deployment-boundaries.md` | 38,542 |
| `docs/architecture/save-to-validation.md` | 46,364 |
| `docs/architecture/docs-delivery.md` | 29,092 |
| `docs/architecture/quality-model.md` | 17,620 |
| `docs/architecture/auth-as-built.md` | 17,189 |
| `docs/architecture/edda-stack.md` | 12,548 |

Every final output was non-empty. The save block rendered successfully with the
literal `check --all` edge and the post-initial condition. All eight recorded
byte sizes are from the exact post-rebase blocks, including both overview
outputs.

## Links, duplication, and retirement evidence

The documentation checker traversed repository-local links in the changed
documents. No changed-document outbound link failed. Removing the old overview
pipeline exposed two live inbound references:
`docs/guides/command-safety.md#check-pipeline` and `#gate-layer`;
the overview now provides navigation-only compatibility headings that send
readers to the quality authority without restoring duplicate content.
`CONTEXT.md` and the shipped-codebase review checklist now route Rust
layout/layering directly to `rust-architecture-overview.md` and check/gate
concepts directly to `quality-model.md`. Manual review found `docs/README.md`
compatible with the changed governance disposition; only its freshness metadata
changed to reconcile the directly owed downstream.

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

The final repair RED/GREEN assertions proved the corrected BAUTH owner, the
independent wired `validate_paths` observation between validation and response,
and the selected subprocess action distinction between scoped/all `check` and
self-scoped `gate` were absent before the repair and present afterwards.

The narrow routing repair assertion proved that the diagram did not previously
make non-empty changed paths part of the daemon-branch condition; GREEN proved
that condition and a direct empty-cycle `check --all` subprocess path. The
final Council minor RED then exposed the false initial-cycle claim; GREEN
records that the first snapshot is skipped and limits `check --all` to
deletion-driven or otherwise empty post-initial cycles.

Supporting-authority and retirement assertions then proved the old overview
duplicates and two live Draw.io files were gone while their owning documents
and historical evidence remained.

## Repository gates

The first real post-commit exact-range audit at `b217b4c6f` corrected the
pre-commit assumption: `pnpm docs:owed --since d9b30b23d` exited zero because
the surface is still report-only, but its summary contained 1 baselined/gating
owed document. The newly committed governance-guide review had made its direct
`docs/README.md` downstream stale. A manually reviewed, freshness-only
downstream follow-up then cleared that finding.

The final publication-rebase, report-inclusive rerun produced:

| Gate | Exit/result |
| ---- | ----------- |
| `pnpm format:check` | 0; all 1,685 matched files formatted |
| `pnpm docs:index` | 0; six generated indexes refreshed with no owned-field drift |
| `pnpm docs:index:check` | 0; 0 errors, 0 warnings, 6 files checked |
| `pnpm docs:check` | 0; 11/11 surfaces passed, 0 failed |
| `pnpm docs:owed --since 8bf8622e7` | 0; publication range reported 0 owed, 0 gating, 0 advisory, 0 review, 0 baselined |
| `pnpm aps:active-lint` | 0; 139 files checked, all clean |
| `pnpm aps:index:check` | 0; inherited DOCDEF stored `0/6` versus computed `2/6` advisory only |
| `pnpm aps:drift --json` | 0; advisory `findingCount: 1`, the same inherited DOCDEF `0/6` versus `2/6` `aps-progress-mismatch` |
| `git diff --check` | 0; no whitespace errors |
| Focused scope/link/duplicate/retirement/provenance assertions | 0; all Council and final repair claims, 24-path scope, six-commit mapping, and ten-path upstream preservation checks passed |

Before the successful final set, the restricted linked-worktree
`pnpm docs:index` attempt exited 1 with `EROFS`; after the six generated
outputs passed anvil's pre-write gates, the authorised refresh exited 0 and
produced no generated-file diff. The first focused assertion wrapper exited 127
because zsh has no Bash `mapfile`; the portable replacement assertion exited
0. These were tooling-environment and harness issues, not documentation gate
failures.

`pnpm docs:check` also reports the repository's baselined link/tag warnings
and corpus-wide docs-owed advisories; its surfaces still pass. Those warnings
are distinct from the explicit exact-range docs-owed result above. The sibling
DOCDEF count/drift advisory is inherited, belongs to
`plans/modules/docs-definition-layer.aps.md`, and is not absorbed into
DOCRB-006 owned files.
