# What's Next

Compiled 2026-03-07, updated 2026-03-08. Two lists: (1) remediation work needed
for beta quality, (2) feature work across all modules.

---

## List 1: Remediation, Fixes & Maintenance

Everything that fixes existing code — bug fixes, code review items, security
hardening, test gaps, maintenance, and documentation drift.

### CRB — Code Review Backlog (29/29 complete)

All 29 code review backlog items are resolved. See
[code-review-backlog.aps.md](../modules/code-review-backlog.aps.md) for full
details and evidence.

| ID | Summary | Priority | Status |
|----|---------|----------|--------|
| CRB-017 | Add tests for platform/core config loaders | Medium | ✅ Resolved — tests already exist (loader.test.ts: 241 lines, config.test.ts) |
| CRB-018 | Standardise works-from-repo-root workflow | Medium | ✅ Complete — PR #505 |
| CRB-019 | Consistent logging/output conventions (stderr/stdout) | Medium | ✅ Complete — PR #506 (commit 5a3882b2) |
| CRB-022 | Large command modules need decomposition (e.g. policy.ts) | Low | ✅ Complete — PR #514 |
| CRB-023 | Silent fallbacks without visibility — emit debug logs | Medium | ✅ Complete — PR #508 (commit d1bbb693, 50+ debug calls) |
| CRB-025 | Docs and scripts drifting from reality — audit accuracy | Low | ✅ Complete — PR #510 (21 issues fixed across 8 files) |

### MAINT — Codebase Maintenance (7/8 complete)

| ID | Summary | Priority | Status |
|----|---------|----------|--------|
| MAINT-002 | Error formatting consistency across CLI commands | Medium | ✅ Complete — CRB-019 migration (commit 5a3882b2) |
| MAINT-003 | Workspace root resolution — consolidate into one utility | Low | ✅ Complete — already consolidated |
| MAINT-004 | Git operation wrappers — consolidate execFile/spawn calls | Medium | Open — scope revised (see below) |
| MAINT-005 | JSON output formatting — standardise `--json` envelope | Low | ✅ Complete — PR #517 |
| MAINT-006 | Nx generator for CLI commands | Low | ✅ Complete — PR #516 |
| MAINT-007 | Nx generator for gate checks | Low | ✅ Complete — PR #516 |
| MAINT-008 | Spinner/progress patterns — consolidate ora usage | Low | ✅ Complete — PR #517 |

#### MAINT-004: Revised Scope

Investigation found the original estimate ("100+ call sites across 19 files")
was vastly overstated. Actual scope:

- **16 total** `execFile`/`execFileSync`/`spawn`/`spawnSync` calls with `'git'`
  across **10 files** (7 production, 3 test)
- `packages/anvil/runtime/src/concurrency/git-agent.ts` already serves as a
  partial git wrapper (4 calls)
- Remaining production calls: `SystemCheck.ts` (3), `release-changelog.ts` (2),
  `plan.ts` (1), `init.ts` (1), `release-git.ts` (1), `policy.check.ts` (1)
- No `runCommand('git', ...)` pattern exists — each call uses Node.js
  `child_process` directly with explicit timeouts (added in CRB-024)

**Recommendation:** Extend `git-agent.ts` into a shared git operations module
and migrate the ~12 remaining production calls. Single PR, estimated 2–4 hours.

### STACK — Edda Stack Integration (19/19 complete)

All stack reconciliation items are resolved.

| ID | Summary | Priority | Status |
|----|---------|----------|--------|
| STACK-006 | Observation-to-Proposal type mapping | Medium | ✅ Complete — PR #515 |
| STACK-017 | Path drift cleanup in APS plan files | High | ✅ Complete — PR #515 |
| STACK-018 | Retroactive evidence capture for STACK-001–016 | High | ✅ Complete — PR #518 |
| STACK-019 | Missing deliverable audit | Medium | ✅ Complete — PR #518 |

### F-series — Interim Review Findings (3/3 complete)

All findings from `plans/reviews/interim-finds-2026-03-04.md` are resolved.

| ID | Summary | Priority | Status |
|----|---------|----------|--------|
| F-001 | Release smoke check uses Unix `ls` — breaks on Windows | P1 | ✅ Resolved — commit 0198d82a (readdirSync replaces ls) |
| F-002 | Workflow monitor marks wrong run as exact match | P1 | ✅ Resolved — commit 0198d82a (name + headBranch check) |
| F-003 | Template rendering builds regex from unescaped variable names | P2 | ✅ Complete — PR #513 |

### ISS — Standalone Issues

| ID | Summary | Severity | Status |
|----|---------|----------|--------|
| ISS-004 | Pulumi Preview CI check failing on main (pre-existing) | Medium | Archived — infrastructure ops issue, not code (see below) |
| ISS-006 | `preserve-caught-error` warnings across CLI/core/runtime (9 total) | Low | ✅ Complete — PR #513 |
| ISS-007 | `preserve-caught-error` warnings across CLI (4 remaining) | Low | ✅ Complete — subsumed by ISS-006 fix (PR #513) |

#### ISS-004: Reclassified as Infrastructure

Investigation found this is not a code bug. The `infra.yml` workflow already has
a `check-secrets` guard that gracefully skips Pulumi when Azure credentials
aren't configured. The CI check fails because the required secrets
(`AZURE_CREDENTIALS`, `PULUMI_ACCESS_TOKEN`, etc.) haven't been set up in GitHub
repository settings. No code change can fix this — it requires Azure credential
provisioning in the deployment environment.

### Summary

| Category | Total | Done | Remaining |
|----------|-------|------|-----------|
| Code review (CRB) | 29 | 29 | 0 |
| Maintenance (MAINT) | 8 | 7 | 1 (MAINT-004, revised scope) |
| Stack reconciliation (STACK) | 4 | 4 | 0 |
| Interim findings (F-series) | 3 | 3 | 0 |
| Issues (ISS) | 3 | 2 | 1 (ISS-004, reclassified as infra) |
| **Total** | **47** | **45** | **2** |

**Completed this sweep (PRs #505–#518):**
- Security & correctness: F-001, F-002, F-003, ISS-006/007, APS execSync, CLI flag removal
- Structural: CRB-022 (policy.ts decomposition)
- Stack reconciliation: STACK-006, STACK-017, STACK-018, STACK-019
- Maintenance: MAINT-002, MAINT-003, MAINT-005, MAINT-006, MAINT-007, MAINT-008
- Nx generators: CLI command generator, gate check generator
- Logging & output: CRB-019 (console.* migration), CRB-023 (debug infrastructure)
- Documentation: CRB-025 (21 doc issues fixed), CRB-018 (workflow standardisation)

**Previously resolved (discovered during audit):**
- F-001, F-002 (commit 0198d82a — release flow fixes, already in main)
- CRB-017 (tests already existed for all config loaders)
- CRB-019 (PR #506), CRB-023 (PR #508), CRB-025 (PR #510), CRB-018 (PR #505)

**Remaining:**
- MAINT-004 (git wrappers) — scope revised from "100+ sites" to 12 calls in 6
  files. Single PR when prioritised.
- ISS-004 (Pulumi CI) — reclassified as infrastructure ops. Requires Azure
  credential setup, not code changes.

**Critical path for beta:** No remediation items remain on the critical path.
MAINT-004 is a quality-of-life improvement. ISS-004 is blocked on infrastructure
provisioning.

---

## List 2: Feature Work

New capabilities, commands, UI, integrations — grouped by release milestone.

### 0.1.x — Current Milestone

| Module | Scope | Status | Remaining |
|--------|-------|--------|-----------|
| .anvil File Format | ANVFMT | In Progress | Phase 2–4 (compiler, new patterns, cleanup) |
| BMAD v4 Backward Compat | BMAD4 | Proposed | 8 tasks |

### 0.2.0 — Web Dashboard (39 tasks)

| Module | Scope | Status | Tasks |
|--------|-------|--------|-------|
| Dashboard Foundation | DASH | Ready | 9 |
| Dashboard Core Views | DASHCORE | Ready | 9 |
| Dashboard Architecture Views | DASHARCH | Ready | 8 |
| Dashboard Operations Views | DASHOPS | Ready | 7 |
| Dashboard AI Builder | DASHAI | Draft | 6 |

### 0.3.0 — Organisational Policy Governance (~79 tasks)

| Module | Scope | Status | Tasks |
|--------|-------|--------|-------|
| OPA Enhancements | OPAE | Draft | 36 |
| Policy Pack Validation | POLVAL | Draft | 5 |
| Architecture Config Validation | ARCHCFG | Draft | 5 |
| AI Guardrail Profile | AIGUARD | Draft | 4 |
| Org Policy Hierarchy | ORGHIER | Draft | 7 |
| Policy Lifecycle | POLLC | Draft | 7 |
| Compliance Reporting | COMPLY | Draft | 8 |
| Policy Federation | POLFED | Draft | 8 |

### 0.4.0 — Edda Stack (Memory System)

Already mostly complete. Remaining feature work:

| Module | Scope | Status | Remaining |
|--------|-------|--------|-----------|
| Ember | EMBER | Complete | 0 |
| Edda | EDDA | Complete | 0 |
| Edda-Ember Review | EERB | Complete | 0 |
| Stack Integration | STACK | Complete | 0 |

### Future — Rust (post-1.0.0)

| Module | Scope | Status | Tasks |
|--------|-------|--------|-------|
| Rust Kernel | KERN | In Progress | ~21 remaining (4/25 complete) |
| Rust Engine Ports | RENG | Proposed | 6 |
| Ratatui TUI | RATS | Proposed | 6 (1/7 complete) |
| Ink-to-Ratatui Port | PORT | Proposed | 15 |

### Future — Other

| Module | Scope | Status | Tasks |
|--------|-------|--------|-------|
| Open-Spec Adapter | OPENSPEC | Draft | 6 |
| Real-Time Validation (Simple) | RTVS | Draft | ~16 |
| Real-Time Validation (Full) | RTVF | Draft | ~32 |
| Python Support | PYLAN | Placeholder | — |
| Rust Support | RSTLAN | Placeholder | — |
| .NET Support | DNLAN | Placeholder | — |

### Summary

| Milestone | Modules | Tasks |
|-----------|---------|-------|
| 0.1.x (current) | 2 | ~10 |
| 0.2.0 (dashboard) | 5 | 39 |
| 0.3.0 (policy governance) | 8 | ~79 |
| 0.4.0 (memory system) | 4 | 0 remaining |
| Future — Rust | 4 | ~48 |
| Future — Other | 6 | ~54+ |
| **Total** | **29** | **~230** |

**Next actionable wave:** Dashboard Foundation (DASH, 9 tasks, Ready status) is
the first feature module that can start immediately.
