# DOCRB-006 central views evidence — 2026-08-20

| Type | Authority | Owner | Status |
| ---- | --------- | ----- | ------ |
| Review | Advisory | DOCRB | Open |

## Scope and source revision

This report records the implementation evidence for DOCRB-006. The exact
comparison base and source-audit revision are both
`d9b30b23daef0da05f74a7d44dfa3accd0e03fe7`. At the source-audit boundary,
`HEAD` was that exact revision, and a targeted
`git diff <base>..HEAD -- crates apps infra tools docs/public` was empty.
There was therefore no product, deployment, tooling, or public-content drift
between the pinned base and the live-source reads used for these views.

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

The implementation leaves exactly five authoritative central concerns:

1. system context in `docs/architecture/overview.md`;
2. container/component relationships in the same overview;
3. macro trust and deployment boundaries;
4. the cross-owner save-to-validation sequence; and
5. macro documentation delivery.

The overview no longer carries quality-model internals, a generic check
pipeline, or EDDA internals. Its `Check pipeline` compatibility heading is a
navigation-only link to `quality-model.md` and carries no pipeline detail.
Quality, BAUTH, and EDDA authority remains in `quality-model.md`,
`auth-as-built.md`, and `edda-stack.md`. Component internals remain in
component-root `ARCHITECTURE.md` files and are linked rather than copied.

Independent verification and Council review remain pending. This report is not
a completion, release, or deployment claim.

## Per-view source-edge trace

| View and material edge | Adjacent authority and source trace | Result |
| ---------------------- | ----------------------------------- | ------ |
| System context: developer/editor/CI → local anvil; local auth → hosted API; reader → docs shell → hosted API | `crates/anvil-cli/README.md`, `crates/anvil-intercept/ARCHITECTURE.md`, `apps/anvil-api/ARCHITECTURE.md`, `apps/docs-shell/ARCHITECTURE.md`, `infra/src/vercel.ts` | Pass; actors and cross-system calls only |
| Container/component: CLI/MCP/daemon/kernel/check/TUI/dashboard relationships and hosted API/data/docs containers | `Cargo.toml`, local component architecture documents under `crates/anvil-kernel`, `crates/anvil-intercept`, `crates/anvil-dashboard-server`, `apps/dashboard`, `apps/anvil-api`, and `apps/docs-shell`, plus `infra/src/vercel.ts` | Pass; internal request flows stay local |
| Trust/deployment: Unix owner-only rendezvous, client daemon-UID check, Windows DACL/SID checks, Linux-only optional attribution, hosted API/data/docs boundaries | `crates/anvil-intercept/src/ipc.rs`, `crates/anvil-intercept/src/lib.rs`, `crates/anvil-intercept-win32/src/lib.rs`, `crates/anvil-cli/src/mcp/gctx_client.rs`, APGOV/BAUTH/docs-shell authorities, and `infra/src/vercel.ts` | Pass; explicitly records no server-side Unix caller-UID comparison |
| Save-to-validation: MidEdit and PreWrite caller-buffer modes; MidEdit-only best-effort observation; separate post-save `validate_paths`; independent fence transitions | `crates/anvil-cli/src/mcp/validation.rs`, `crates/anvil-cli/src/commands/watch_save_time.rs`, `crates/anvil-intercept/src/midedit.rs`, `ipc.rs`, `save_time.rs`, `validate_paths.rs`, `interrupt.rs`, `unregistered.rs`, and `fence.rs` | Pass; the three lanes/fences are not collapsed |
| Documentation delivery: governed sources → private/public builds → deployed renderers; docs shell routing → protected upstreams; rollback-only legacy project | Docusaurus configurations, `apps/docs-shell/proxy.ts`, `apps/docs-shell/lib/jwt.ts`, both renderer middleware files, `infra/src/vercel.ts`, `infra/src/components/vercel-app.ts`, app `vercel.json` files, and `tools/scripts/vercel-ignore-build.sh` | Pass; `/anvil` entitlement, public routing, protected upstream secret, rollback-only `apps/docs-site`, and the DOCRB/DSITE gap remain visible |

The supporting Mermaid blocks in the quality, BAUTH, and EDDA documents were
also checked against their adjacent detailed prose and cited sources. They
remain supporting authorities, not extra central concerns.

## Mermaid render evidence

Eight changed Mermaid blocks were extracted directly from their Markdown
owners and rendered manually with
`@mermaid-js/mermaid-cli@11.16.0`. All inputs and outputs remained under
`/tmp/docrb006-mermaid`. Chromium rejected its nested sandbox in this
container, so the successful run used a temporary
`/tmp/docrb006-puppeteer.json` configuration containing only
`--no-sandbox` and `--disable-setuid-sandbox`. No renderer dependency,
configuration, output, script, or generated asset entered the repository.

| Mermaid owner/block | Final SVG bytes |
| ------------------- | --------------: |
| `docs/architecture/overview.md` — system context | 20,159 |
| `docs/architecture/overview.md` — container/component | 32,099 |
| `docs/architecture/trust-and-deployment-boundaries.md` | 28,392 |
| `docs/architecture/save-to-validation.md` | 39,094 |
| `docs/architecture/docs-delivery.md` | 29,092 |
| `docs/architecture/quality-model.md` | 17,620 |
| `docs/architecture/auth-as-built.md` | 17,189 |
| `docs/architecture/edda-stack.md` | 12,548 |

Every output was non-empty. The first render pass exposed a reserved Mermaid
identifier in the retained quality view and punctuation that the sequence
parser rejected in the save view; both source blocks were repaired and all
eight final blocks were rendered again successfully.

## Links, duplication, and retirement evidence

The documentation checker traversed repository-local links in the changed
documents. No changed-document outbound link failed. Removing the old overview
pipeline exposed one live inbound
`docs/guides/command-safety.md#check-pipeline` reference; the overview now
provides a navigation-only compatibility heading that sends readers to the
quality authority without restoring duplicate content.

Focused duplicate-authority assertions proved:

- the overview has exactly two Mermaid blocks;
- trust, save, and docs delivery each have exactly one Mermaid block;
- quality, BAUTH, and EDDA each retain exactly one supporting Mermaid block;
- the overview contains no quality-model, generic-pipeline, or EDDA diagram;
- the five required concerns are discoverable from `CONTEXT.md` and the
  architecture README; and
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

Supporting-authority and retirement assertions then proved the old overview
duplicates and two live Draw.io files were gone while their owning documents
and historical evidence remained.

## Repository gates

A preliminary report-boundary run established:

- `pnpm format:check`, `pnpm docs:index:check`,
  `pnpm aps:active-lint`, and `git diff --check` passed;
- `pnpm aps:index:check` exited zero with the inherited DOCDEF stored
  `0/6` versus computed `1/6` advisory;
- `pnpm aps:drift --json` exited zero with `findingCount: 1`, the same
  inherited `aps-progress-mismatch` for DOCDEF;
- `pnpm docs:check` exposed the inbound compatibility anchor described above,
  which was repaired; and
- its retired-claims surface could not read the two intentionally deleted but
  then-unstaged files. The final commit-candidate run stages those deletions so
  they are absent from the tracked corpus instead of appearing as unreadable
  tracked paths.

The final fresh gate set is run only after this report, its generated-index
effect, and the two staged retirements are part of the exact commit candidate:

```text
pnpm format:check
pnpm docs:index
pnpm docs:index:check
pnpm docs:check
pnpm aps:active-lint
pnpm aps:index:check
pnpm aps:drift --json
git diff --check
```

The expected inherited documentation warnings and the sibling DOCDEF count
advisory do not belong to DOCRB-006 and are not absorbed into its owned files.
