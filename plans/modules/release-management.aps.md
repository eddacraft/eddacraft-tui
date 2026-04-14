<!--
APS Module: Release Management
====================================
Release cadence, changelog, publish strategy, and interactive release tooling.
See: plans/aps-rules.md
-->

# Release Management

| ID      | Owner | Status    |
| ------- | ----- | --------- |
| RELMGMT | —     | Complete |

## Purpose

Establish release management practices for the growing set of packages: npm
packages (TypeScript), Rust crates, and the CLI. Covers release cadence,
changelog governance, semver policy, version coordination across monorepo
packages, and interactive tooling that enforces the release process.

**Problem:** The project has npm packages, Rust crates, a CLI, a website,
and a docs site — but no release management module. Releases happen ad-hoc
via the runbook. There's no changelog governance, semver policy, or
coordination between TypeScript and Rust release cycles. The runbook is
comprehensive but entirely manual — steps get skipped, verification is
inconsistent, and there's no durable record of what was checked for each
release.

## In Scope

- **Release cadence:** How often to release, what triggers a release
- **Changelog governance:** Format (Keep a Changelog), automation, review
- **Semver policy:** What constitutes major/minor/patch across packages
- **Version coordination:** npm packages vs Rust crates — coupled or independent?
- **Publish pipeline:** cargo-dist binary release via GitHub Releases
- **Pre-release strategy:** Two channels — beta (current) and production
- **Release notes:** Auto-generated vs manual, communication strategy
- **Breaking change process:** Migration guides, deprecation period
- **Interactive release script:** Shell script that enforces the runbook with
  hard/soft gates, creates a GitHub Issue for tracking, and writes an ephemeral
  manifest as handoff to the Claude skill
- **Claude release skill:** `/release` skill for post-script judgment steps
  (changelog review, doc triage, comms, cleanup)
- **Release tracking:** GitHub Issues as the single pane of glass for each
  release lifecycle

## Out of Scope

- CI/CD pipeline implementation (covered by CI modules)
- Feature flags (separate concern)
- Distribution pipeline (binaries, install scripts, package managers — see DIST)

## Interfaces

**Depends on:**

- CI pipeline — automated release checks
- DIST — binary distribution (the script triggers the workflow, DIST defines it)
- All packages — version data

**Exposes:**

- Release policy document
- Changelog format specification
- Semver decision matrix
- `scripts/release.sh` — interactive release script
- `/release` Claude skill — post-release verification and cleanup
- `.github/ISSUE_TEMPLATE/release.md` — release tracking issue template

## Estimated Scope

- **Effort:** 2 weeks

---

## Phase 1 — Policy (Ratified)

### RELMGMT-001: Release cadence policy and triggers

- **Status:** Complete (ratified 2026-04-14)
- **Intent:** Define what triggers a release (feature-complete, schedule, hotfix)
- **Outcome:** Releases are triggered when there's a meaningful change to ship.
  Remaining smaller items are bundled up alongside. No fixed cadence — solo
  operator judgment call during beta. The runbook and branching strategy
  document the two promotion paths (direct vs. stabilisation). A formal
  trigger matrix is deferred until post-beta when team size or release
  cadence warrants it.

---

### RELMGMT-002: Changelog format specification and automation

- **Status:** Complete (ratified 2026-04-14)
- **Intent:** Standardise changelog format (Keep a Changelog)
- **Outcome:** Keep a Changelog format is established and consistently used in
  `CHANGELOG.md`. Entries are hand-written from commit review — this produces
  better reader-facing changelogs than automation. The release doc checklist
  enforces updating the changelog. Automation (conventional commits tooling)
  dropped from scope — not needed for current scale.

---

### RELMGMT-003: Semver policy across npm + Rust packages

- **Status:** Complete (ratified 2026-04-14)
- **Intent:** Define versioning strategy across crates and packages
- **Outcome:** Lockstep versioning for Anvil core (`crates/anvil-*`,
  `packages/anvil/*`) — all share the release tag version. Independent
  versioning for everything else (edda-stack, aps, eddacraft-tui, kindling,
  mcp-server, vscode-extension, website, docs-site). The rule: if it ships
  in the binary or directly supports it at runtime, it's core and lockstep.
  See ADR-020 for the full decision record.

---

### RELMGMT-004: Publish pipeline documentation

- **Status:** Complete (ratified 2026-04-14)
- **Intent:** Document the publish pipeline for released artefacts
- **Outcome:** The binary publish pipeline (cargo-dist via tag push, dual
  GitHub Release to private + public repos) is documented in the release
  runbook (sections 3–5). npm packages are not published — they are consumed
  internally within the monorepo. MCP server distribution is a known gap
  tracked separately (likely a DIST task once the approach is decided).
  VS Code extension has never been published to the Marketplace.

---

### RELMGMT-005: Pre-release channel strategy

- **Status:** Complete (ratified 2026-04-14)
- **Intent:** Define release channels and when to promote between them
- **Outcome:** Two channels — beta (invited testers) and production (public).
  No alpha or rc. Promotion from beta to production is a judgment call.
  cargo-dist handles the prerelease flag automatically via tag suffix.
  Feature flag channels align: `development | beta | production`.

---

### RELMGMT-006: Breaking change process and migration guide template

- **Status:** Complete (ratified 2026-04-14)
- **Intent:** Standardise how breaking changes are communicated and migrated
- **Outcome:** Breaking changes are documented as sections in
  `docs/public/anvil/releases/upgrade-notes.md` with what changed and how
  to migrate. Major migrations get a dedicated doc (e.g. `rust-rewrite.md`
  for the Node→Rust move). The release doc checklist enforces updating
  upgrade notes for any breaking or behavioural changes. A formal template
  is unnecessary at current scale — the existing ad-hoc docs are
  high quality.

---

## Phase 2 — Interactive Release Tooling (Draft)

### RELMGMT-007: Release tracking issue template

- **Status:** Complete (2026-04-14)
- **Intent:** GitHub Issue template that tracks a release from preflight
  through post-release verification. Created by the release script at the
  start of each release. Label: `release`. Title: `release/vX.Y.Z`.
- **Expected Outcome:** `.github/ISSUE_TEMPLATE/release.md` with sections
  for preflight, branch strategy, tagging, workflow monitoring, post-release
  verification, doc review, comms, and cleanup — each with checkbox items
  matching the runbook steps
- **Validation:** `gh issue create --template release.md` produces a
  well-formed issue with all sections
- **Files:** `.github/ISSUE_TEMPLATE/release.md`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

### RELMGMT-008: Gitignore ephemeral release manifest

- **Status:** Complete (2026-04-14)
- **Intent:** Add `.release/` to `.gitignore` so the ephemeral manifest
  written by the release script is never committed
- **Expected Outcome:** `.release/manifest.json` is ignored by git
- **Validation:** `echo test > .release/foo && git status` does not show
  `.release/foo` as untracked
- **Files:** `.gitignore`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

### RELMGMT-009: Interactive release shell script

- **Status:** Complete (2026-04-14)
- **Intent:** Shell script that walks the operator through the mechanical
  release steps (preflight, branching, tagging, workflow kickoff) with
  interactive gates. Hard gates abort on failure (tests, version mismatch).
  Soft gates allow retry/skip/abort (clippy, TS build). Creates a GitHub
  Issue at the start, updates it throughout, and writes an ephemeral
  `.release/manifest.json` as the handoff contract for the `/release` skill.
- **Expected Outcome:** `./scripts/release.sh` interactively guides a
  release from preflight through tag push, with all results tracked in a
  GitHub Issue and a manifest written for the skill
- **Validation:**
  - Script creates a GitHub Issue with label `release`
  - Preflight failures on hard gates abort the script
  - Preflight failures on soft gates offer retry/skip/abort
  - `.release/manifest.json` contains version, tag, SHAs, workflow run ID,
    issue number, preflight results, and diff summary
  - Manifest includes both crate and package change information
- **Files:** `scripts/release.sh`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RELMGMT-007, RELMGMT-008

**Manifest format:**

```jsonc
{
  "version": "0.4.0-beta",
  "tag": "v0.4.0-beta",
  "releaseType": "beta",           // "beta" | "production"
  "branchStrategy": "direct",      // "direct" | "stabilisation"
  "releaseBranch": null,           // "release/x.y.z" or null for direct
  "timestamp": "2026-04-14T10:30:00Z",
  "shas": {
    "dev": "abc1234",
    "main": "def5678",
    "tag": "def5678"
  },
  "workflowRunId": "12345678",
  "issueNumber": 42,
  "issueUrl": "https://github.com/EddaCraft/anvil-001/issues/42",
  "preflight": {
    "cargoTest": "pass",
    "cargoClippy": "pass",
    "cargoBuild": "pass",
    "binaryVersion": "0.4.0-beta",
    "pnpmBuild": "pass",
    "pnpmTest": "pass"
  },
  "diffSummary": {
    "changedPaths": ["crates/anvil-cli/", "packages/edda-stack/src/"],
    "changedPackages": ["edda-stack", "anvil-runtime"],
    "changedCrates": ["anvil-cli", "anvil-kernel"]
  }
}
```

**Gate types:**

| Type | Behaviour |
| ---- | --------- |
| Hard gate | Pass → continue. Fail → abort (no override). |
| Soft gate | Pass → continue. Fail → [r]etry / [s]kip / [a]bort. Skip logs reason to issue. |

**Script phases:**

1. Initialisation — prompt for version, derive tag/type, choose branch
   strategy, create GitHub Issue
2. Preflight — run cargo test/clippy/build, pnpm build/test, verify versions
3. Branch & tag — execute branch strategy, merge, version bump, tag, push
4. Workflow kickoff — capture run ID, print monitoring commands
5. Manifest — generate diff summary, write `.release/manifest.json`, print
   handoff message

---

### RELMGMT-010: Claude `/release` skill

- **Status:** Complete (2026-04-14)
- **Intent:** Claude Code skill for post-script judgment steps. Reads the
  ephemeral manifest as a gate contract — refuses to start without a valid
  one. Handles workflow monitoring, artefact verification, changelog review,
  doc checklist triage, comms drafting, and post-release cleanup. Updates
  the GitHub Issue throughout and closes it when done.
- **Expected Outcome:** `/release` in Claude Code picks up where the script
  left off, verifies the release, handles documentation and comms, performs
  cleanup, and closes the tracking issue
- **Validation:**
  - Skill refuses to start without `.release/manifest.json`
  - Skill refuses to start if manifest timestamp is older than 24h
  - Skill validates manifest fields against live state (tag exists, issue
    exists, workflow run ID resolves)
  - All 8 expected artefacts verified on public repo
  - GitHub Issue updated with verification results and closed on completion
- **Files:** `.claude/skills/release.md`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RELMGMT-009

**Skill steps:**

1. Gate — read manifest, refuse if missing or stale
2. Validate — confirm tag exists, issue exists, workflow run resolves
3. Monitor workflow — poll until complete or prompt to wait
4. Verify artefacts — check all 8 expected assets on `EddaCraft/anvil`
5. Changelog review — assess completeness against diff summary
6. Doc checklist triage — cross-reference changed paths against
   `release-doc-checklist.md`, present applicable items
7. Comms — draft release message from runbook template
8. Post-release cleanup:
   - Back-merge to `dev` (verify or create PR)
   - Release branch deletion (if stabilisation strategy)
   - Public repo release state (prerelease flag matches release type)
   - `install.eddacraft.ai` health check
9. Update and close GitHub Issue

---

### RELMGMT-011: Update release runbook

- **Status:** Complete (2026-04-14)
- **Intent:** Update the existing runbook to reference the interactive
  release script and `/release` skill. Keep incident playbook and known
  gotchas as-is — they're still the reference for when things go wrong.
- **Expected Outcome:** Runbook has a "Quick start" section at the top
  pointing to `scripts/release.sh` and `/release`, with existing manual
  steps marked as reference
- **Validation:** `grep -q "scripts/release.sh" docs/guides/release-runbook.md`
- **Files:** `docs/guides/release-runbook.md`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** RELMGMT-009, RELMGMT-010

---

## Execution Order

Phase 1 tasks (001–006) are policy/documentation and independent of Phase 2.

Phase 2 dependency graph:

```
[RELMGMT-007: Issue template] ──┐
                                 ├──→ [RELMGMT-009: Script] ──→ [RELMGMT-010: Skill] ──→ [RELMGMT-011: Runbook]
[RELMGMT-008: Gitignore]  ──────┘
```

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 1 — Policy | 6 | 6/6 ratified |
| 2 — Interactive Tooling | 5 | 5/5 complete |
| **Total** | **11** | **11/11 done** |
