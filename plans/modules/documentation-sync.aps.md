<!--
APS Module: Documentation Sync
====================================
Keeps docs in sync with feature work. Replaces archived documentation-polish.
See: plans/aps-rules.md
-->

# Documentation Sync

| ID      | Owner | Status      | Progress |
| ------- | ----- | ----------- | -------- |
| DOCSYNC | —     | In Progress | 16/23    |

## Purpose

Keep the public docs-site (Docusaurus) in sync with feature development. The
**Anvil** section is sourced from `docs/public/anvil/`; **Kindling**, **APS**,
and other sibling product sections mirrored under `docs/public/<product>/` are
also in scope when refreshed to match upstream releases (see DOCSYNC-023,
DOCSYNC-024). Rust CLI migration, web dashboard rollout, policy governance, and
new language support are the primary Anvil drivers. API reference generation
(from the OpenAPI spec) is also in scope. Internal-doc governance — ADR
process, architecture diagrams, runbook/as-built freshness — is owned by
DOCGOV.

**Problem:** Documentation was polished for 0.1.0 but has no forward plan.
The Rust CLI replaces the Node.js package entirely, the dashboard adds a new
surface, and policy governance changes the governance model — all need
documentation updates that aren't tracked.

## In Scope

DOCSYNC scope is **public-facing Docusaurus content** under `docs/public/` —
primarily the Anvil section (`docs/public/anvil/`), plus mirrored sibling
sections (`docs/public/kindling/`, `docs/public/aps/`, and others) when a
release refresh is scheduled here. Host wiring for those sections is tracked by
[DSITE](public-docs-site-host.aps.md). Internal docs under `docs/guides/`,
`plans/**`, and architecture / runbook freshness now live under DOCGOV.

- **Rust migration docs:** Install, CI, troubleshooting updated for native binary
- **Docs-site sync:** Keep public docs in sync with feature releases
- **API reference:** Auto-generate from OpenAPI spec (feeds into API governance)
- **Tutorial updates:** Keep tutorials current as surfaces change (Ink → Ratatui), and keep shell examples usable from macOS, Linux, and Windows PowerShell.
- **Multi-version docs:** Support docs for current + previous release

## Out of Scope

- Marketing content (covered by website module)
- Blog posts (separate concern)
- APS specification authoring in the external repo (derived public mirror at
  `docs/public/aps/` is synced via DOCSYNC-024)
- ADR template/process and architecture diagrams (now owned by DOCGOV)
- Internal runbook and as-built freshness (now owned by DOCGOV-006)

## Interfaces

**Depends on:**

- `docs/public/anvil/` — Docusaurus content source
- `docs/public/aps/` — APS section derived from anvil-plan-spec
- `apps/docs-site` — Docusaurus instance (reads from `docs/public/anvil/`)
- Feature modules — source of documentation truth
- API governance — OpenAPI spec for API reference
- DOCGOV-002 — metadata convention every new public doc must carry
- DOCGOV-005 — `pnpm docs:check` validates DOCSYNC output

**Exposes:**

- Public docs-site refresh cadence aligned with feature releases
- Tutorial / quickstart / install-guide surface for the Rust CLI

## Estimated Scope

- **Effort:** 2 weeks

## Work Items

### Rust CLI Migration (0.3.0-beta)

- DOCSYNC-001: Documentation sync checklist per release
- DOCSYNC-002: ADR process documentation and template
- DOCSYNC-003: Architecture diagram update process
- DOCSYNC-004: Tutorial update for Ratatui migration
- DOCSYNC-005: API reference generation pipeline
- DOCSYNC-006: Rust engine architecture documentation
- DOCSYNC-007: Update install, CI, and troubleshooting for native binary
- DOCSYNC-008: Rust migration guide (releases/rust-rewrite.md)
- DOCSYNC-009: Remove Node.js/npm references from all public docs
- DOCSYNC-010: Update beta-testing-guide for 0.3.0-beta

### Future

- DOCSYNC-011: Dashboard feature documentation
- DOCSYNC-012: Policy governance documentation updates — rewrite the public
  policy tutorial for the POLRESET/regorus pack model (see status table note)
- DOCSYNC-013: Multi-language support documentation
- DOCSYNC-021: Refresh docs for 0.3.2-beta/0.3.3-beta and current repo topology
- DOCSYNC-022: Refresh current public docs for final release scope and 0.4.0-beta watch filtering
- DOCSYNC-023: Full Kindling public docs refresh for upstream 0.2.0 (sibling `eddacraft/kindling`)
- DOCSYNC-024: Full refresh of `docs/public/aps/` against `anvil-plan-spec` v0.4.0
- DOCSYNC-025: Refresh Anvil public docs for current daemon and MCP surfaces
- DOCSYNC-026: Cross-platform tutorial command examples
- DOCSYNC-027: Audit all Anvil public docs against the current Rust CLI and
  remove obsolete runtime and command guidance
- DOCSYNC-028: Rebuild the anvil public docs around a first-time-user journey,
  public-only language, source-derived reference material, and enforceable
  trust boundaries
- DOCSYNC-029: Rebuild the APS public docs around a first-time-user journey,
  public-only language, and the current anvil-plan-spec command and scaffold
  contracts
- DOCSYNC-030: Repair the beta onboarding brief for update checks, GitHub or
  OTP authentication, executable test steps, and private feedback

### Scanner / Two-Engine State (from RSCAN-008 council review, 2026-04-21)

- DOCSYNC-016: VSCode vs CI warning-divergence troubleshooting entry in `docs/public/anvil/operations/troubleshooting.md`

### Reassigned

- DOCSYNC-014 (Docs contribution guide) → superseded by DOCGOV-001
  (`docs/guides/documentation-governance.md` already covers this)
- DOCSYNC-015, -017, -018, -019, -020 → absorbed and closed in DOCGOV-006
  (targets internal runbook/architecture docs, not the public docs-site)

## Stats

| Phase                           | Total | Done | In Progress | Draft |
| ------------------------------- | ----- | ---- | ----------- | ----- |
| Rust CLI Migration              |    10 |    9 |           0 |     1 |
| Future                          |    12 |    7 |           3 |     2 |
| Scanner / Two-Engine State      |     1 |    0 |           0 |     1 |
| **Total**                       |    23 |   16 |           3 |     4 |

### Item Detail

| ID          | Status | Notes                                         |
| ----------- | ------ | --------------------------------------------- |
| DOCSYNC-001 | Done   | `docs/guides/release-doc-checklist.md` (authority now under DOCGOV per scope sharpening 2026-05-22) |
| DOCSYNC-002 | Done   | `plans/decisions/adr-template.md` + `docs/guides/adr-process.md` (authority now under DOCGOV-004) |
| DOCSYNC-003 | Done   | `docs/guides/architecture-diagrams.md` (authority now under DOCGOV) |
| DOCSYNC-004 | Done   | TUI references updated in beta guide, quickstart |
| DOCSYNC-005 | Draft  |                                               |
| DOCSYNC-006 | Done   | Crate READMEs + rust-rewrite.md               |
| DOCSYNC-007 | Done   | Install, CI, troubleshooting updated across all public docs |
| DOCSYNC-008 | Done   | `docs/public/anvil/releases/rust-rewrite.md`  |
| DOCSYNC-009 | Done   | All `pnpm anvil`/`npx anvil` refs replaced in public docs |
| DOCSYNC-010 | Done   | Beta guide updated for 0.3.0-beta, Node.js dep removed |
| DOCSYNC-011 | Draft  |                                               |
| DOCSYNC-012 | In Progress | CLICT-002 reconciliation PR rewrites `docs/public/anvil/tutorials/policies.md` around `install` → `validate` → `gate` pack workflow per POLRESET; close when PR lands. |
| DOCSYNC-013 | Draft  |                                               |
| DOCSYNC-016 | Draft  | Origin: operations-reviewer OPS-002 (RSCAN-008 council) |
| DOCSYNC-021 | Done   | 0.3.2/0.3.3 public release docs, auth quickstarts, README and repo-topology docs refreshed |
| DOCSYNC-022 | Done   | Final release-scope pass: current install/upgrade docs + 0.4.0-beta watch-filter docs refreshed |
| DOCSYNC-023 | Done   | Full `docs/public/kindling/` refresh against upstream `eddacraft/kindling` v0.2.0: `demo`/`browse`, thin-client adapters, integrations matrix, VS Code adapter, 0.2 crate versions, retrieval score range, removed stale `list` flags |
| DOCSYNC-024 | Done   | `docs/public/aps/**` aligned to `anvil-plan-spec` v0.4.0: terminology, CLI, file layout, examples. Follow-up accuracy pass clarified native-vs-bash CLI surface, `--plans` support, and terminal status semantics. |
| DOCSYNC-025 | Done   | Anvil public docs refreshed for current daemon lifecycle, MCP targets, watch NDJSON lifecycle wording, and safer daemon reset guidance |
| DOCSYNC-026 | Done   | Public tutorials and terminal tutorial policy-directory step now include macOS/Linux and Windows PowerShell/native-shell variants |
| DOCSYNC-027 | Done   | README + `docs/public/anvil/**` + beta quickstart audited against `v0.9.0-beta`; obsolete runtime and roadmap guidance removed; 216 fenced command examples parse against the shipped Rust CLI; docs and site validation pass |
| DOCSYNC-028 | Merged 2026-07-20 via PR #3366 | Structured new-user rebuild delivered with one canonical journey, generated references, public-only validation, and complete navigation. |
| DOCSYNC-029 | In Progress | APS equivalent of DOCSYNC-028: code-truth audit completed against anvil-plan-spec `origin/main` at `53e6278`; implementation plan: [`plans/execution/DOCSYNC-029.actions.md`](../execution/DOCSYNC-029.actions.md). |
| DOCSYNC-030 | In Progress | Beta testing brief: add a current-version/update check, GitHub-device and OTP sign-in evidence, explicit CI and cleanup checks, and a private feedback route. |

## Approved New-User Rebuild

DOCSYNC-027 established Rust command truth. A clean-room review on 2026-07-18
then found duplicated onboarding, internal implementation references, hidden
pages, undefined terminology, and manually maintained product claims that had
drifted from their sources.

DOCSYNC-028 owns the corrective rebuild. DOCSYNC remains the content owner,
DSITE owns shared Docusaurus wiring, and the existing DOCGOV `docs:check`
surface is extended instead of duplicated.

### DOCSYNC-028 delivery contract

- **Status:** Merged 2026-07-20 via PR #3366
- **Intent:** Give a first-time user one complete path to verified value without
  repository access or prior anvil knowledge.
- **Expected Outcome:** Public docs use lowercase `anvil`, `eddacraft`, and
  `kindling`; contain no internal plans, paths, symbols, or work-item language;
  derive volatile reference facts from product sources; expose every public
  page through intent-based navigation or an explicit unlisted contract; and
  pass clean-room first-use acceptance.
- **Scope:** `docs/public/anvil/**`, `docs/public/beta/quickstart.md`, anvil and
  beta docs-site navigation and entrypoints, and the existing docs validation
  pipeline.
- **Non-scope:** Product behaviour, internal architecture or runbook content,
  marketing-site redesign, and sibling product documentation beyond required
  lowercase naming.
- **Dependencies:** DOCSYNC-027 is Done. Coordinate host changes with DSITE-001
  and validation changes with DOCGOV's existing `docs:check` authority.
- **Validation:** See `plans/execution/DOCSYNC-028.actions.md`.

## Approved APS New-User Rebuild

DOCSYNC-024 synchronised the public APS section to v0.4.0. A clean-room audit
against anvil-plan-spec v0.6.0 on 2026-07-20 found that the public journey now
misstates the native command surface, installer flow, Windows requirements,
migration command, default scaffold, monorepo support, and generated project
shape. It also introduces source-repository concepts before a new user reaches
a first validated plan.

DOCSYNC-029 applies the DOCSYNC-028 documentation model to APS. The public
section will lead with one verified first-success path, separate tutorials and
how-to guidance from concepts and reference, translate implementation truth
into standalone user language, and validate examples against a versioned
snapshot of the upstream CLI contract.

### DOCSYNC-029 delivery contract

- **Status:** In Progress
- **Intent:** Give a first-time APS user one complete path from installation to
  a lint-clean plan and an executable work item without repository access or
  prior planning vocabulary.
- **Expected Outcome:** Public APS docs use lowercase `anvil`, `eddacraft`, and
  `kindling`; contain no internal plans, paths, decision IDs, or work-item
  language; describe the v0.6.0 native CLI, installer, scaffold, orchestration,
  and monorepo behaviour accurately; expose every public page through
  intent-based navigation; and reject invalid fenced `aps` commands in the docs
  validation pipeline.
- **Scope:** `docs/public/aps/**`, the APS docs-site sidebar, the existing public
  docs validation surface, and a source-pinned APS CLI contract snapshot.
- **Non-scope:** anvil-plan-spec product behaviour, upstream repository docs,
  APS parser or CLI implementation, Docusaurus visual redesign, and sibling
  product documentation beyond lowercase naming.
- **Dependencies:** DOCSYNC-024 is Done and DOCSYNC-028 is Merged. Coordinate
  host changes with DSITE and validation changes with the existing DOCGOV
  `docs:check` authority.
- **Validation:** See `plans/execution/DOCSYNC-029.actions.md`.
- **Results:** Rebuilt all 15 public APS pages and the sidebar around one
  install-to-first-plan journey; corrected the native CLI, platform, migration,
  scaffold, agent, and monorepo contracts against anvil-plan-spec v0.6.0 at
  `53e6278`; added a source-pinned command manifest plus public-only,
  navigation, and fenced-command validation; 68/68 documented `aps` commands
  match the contract; the docs regression suite, docs checks, APS lint,
  formatting, and Docusaurus production build pass. Council session
  `council-21580e5f` converged with no findings.

### Reassigned items (out of DOCSYNC totals)

| ID          | Disposition                                                                          |
| ----------- | ------------------------------------------------------------------------------------ |
| DOCSYNC-014 | Superseded by DOCGOV-001 (`docs/guides/documentation-governance.md` already covers it) |
| DOCSYNC-015 | Closed by DOCGOV-006 (gate-runner runbook freshness)                                 |
| DOCSYNC-017 | Closed by DOCGOV-006 (`docs/runbooks/release-runbook.md` freshness)                  |
| DOCSYNC-018 | Closed by DOCGOV-006 (`rust-architecture-endstate.md` as-built freshness)            |
| DOCSYNC-019 | Closed by DOCGOV-006 (`docs/guides/release-doc-checklist.md` freshness)              |
| DOCSYNC-020 | Closed by DOCGOV-006 (`docs/guides/anvil-rule-authoring.md` ReDoS framing)           |
