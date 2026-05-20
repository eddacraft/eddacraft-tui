# What's Next

Compiled 2026-03-07, updated 2026-03-08. Two lists: (1) remediation work needed
for beta quality, (2) feature work across all modules.

---

## List 1: Remediation, Fixes & Maintenance

Everything that fixes existing code — bug fixes, code review items, security
hardening, test gaps, maintenance, and documentation drift.

### CRB — Code Review Backlog (29/29 complete)

All 29 code review backlog items are resolved. See
[code-review-backlog.aps.md](../archive/modules/code-review-backlog.aps.md) for full
details and evidence.

| ID | Summary | Priority | Status |
|----|---------|----------|--------|
| CRB-017 | Add tests for platform/core config loaders | Medium | ✅ Resolved — tests already exist (loader.test.ts: 241 lines, config.test.ts) |
| CRB-018 | Standardise works-from-repo-root workflow | Medium | ✅ Complete — PR #505 |
| CRB-019 | Consistent logging/output conventions (stderr/stdout) | Medium | ✅ Complete — PR #506 (commit 5a3882b2) |
| CRB-022 | Large command modules need decomposition (e.g. policy.ts) | Low | ✅ Complete — PR #514 |
| CRB-023 | Silent fallbacks without visibility — emit debug logs | Medium | ✅ Complete — PR #508 (commit d1bbb693, 50+ debug calls) |
| CRB-025 | Docs and scripts drifting from reality — audit accuracy | Low | ✅ Complete — PR #510 (21 issues fixed across 8 files) |

### MAINT — Codebase Maintenance (4/8 complete)

| ID | Summary | Priority | Status |
|----|---------|----------|--------|
| MAINT-002 | Error formatting consistency across CLI commands | Medium | ✅ Complete — CRB-019 migration (commit 5a3882b2) |
| MAINT-003 | Workspace root resolution — consolidate into one utility | Low | ✅ Complete — already consolidated |
| MAINT-004 | Git operation wrappers — consolidate execFile/spawn calls | Medium | ✅ Complete — PR #521 |
| MAINT-005 | JSON output formatting — standardise `--json` envelope | Low | 🔄 In Progress — PR #517 (open, not yet merged) |
| MAINT-006 | Nx generator for CLI commands | Low | 🔄 In Progress — PR #516 (open, not yet merged) |
| MAINT-007 | Nx generator for gate checks | Low | 🔄 In Progress — PR #516 (open, not yet merged) |
| MAINT-008 | Spinner/progress patterns — consolidate ora usage | Low | 🔄 In Progress — PR #517 (open, not yet merged) |

### STACK — Edda Stack Integration (17/19)

| ID | Summary | Priority | Status |
|----|---------|----------|--------|
| STACK-006 | Observation-to-Proposal type mapping | Medium | ✅ Complete — PR #515 |
| STACK-017 | Path drift cleanup in APS plan files | High | ✅ Complete — PR #515 |
| STACK-018 | Retroactive evidence capture for STACK-001–016 | High | 🔄 In Progress — PR #518 (open, not yet merged) |
| STACK-019 | Missing deliverable audit | Medium | 🔄 In Progress — PR #518 (open, not yet merged) |

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
| ISS-004 | Pulumi Preview CI check failing on main (pre-existing) | Medium | ✅ Resolved — Azure secrets provisioned, CI passing on main |
| ISS-006 | `preserve-caught-error` warnings across CLI/core/runtime (9 total) | Low | ✅ Complete — PR #513 |
| ISS-007 | `preserve-caught-error` warnings across CLI (4 remaining) | Low | ✅ Complete — subsumed by ISS-006 fix (PR #513) |

#### ISS-004: Resolved

Azure secrets (`ARM_CLIENT_ID`, `ARM_CLIENT_SECRET`, `ARM_TENANT_ID`,
`ARM_SUBSCRIPTION_ID`, `AZURE_STORAGE_ACCOUNT`, `AZURE_STORAGE_KEY`,
`PULUMI_CONFIG_PASSPHRASE`) are now provisioned in GitHub repository settings.
The `infra.yml` workflow's `check-secrets` guard works correctly, and all recent
CI runs on main show `conclusion: success`. No code changes were required.

### Summary

| Category | Total | Done | In Progress | Remaining |
|----------|-------|------|-------------|-----------|
| Code review (CRB) | 29 | 29 | 0 | 0 |
| Maintenance (MAINT) | 8 | 4 | 4 (PRs #516, #517 open) | 0 |
| Stack reconciliation (STACK) | 4 | 2 | 2 (PR #518 open) | 0 |
| Interim findings (F-series) | 3 | 3 | 0 | 0 |
| Issues (ISS) | 3 | 3 | 0 | 0 |
| **Total** | **47** | **41** | **6** | **0** |

**Merged this sweep:**
- Security & correctness: F-001, F-002, F-003, ISS-006/007, APS execSync, CLI flag removal
- Structural: CRB-022 (policy.ts decomposition, PR #514)
- Stack reconciliation: STACK-006, STACK-017 (PR #515)
- Maintenance: MAINT-002, MAINT-003
- Logging & output: CRB-019 (console.* migration, PR #506), CRB-023 (debug infrastructure, PR #508)
- Documentation: CRB-025 (21 doc issues fixed, PR #510), CRB-018 (workflow standardisation, PR #505)

**In progress (PRs open, not merged):**
- PR #516: MAINT-006 (CLI command generator), MAINT-007 (gate check generator)
- PR #517: MAINT-005 (JSON output migration), MAINT-008 (spinner consolidation)
- PR #518: STACK-018 (retroactive evidence), STACK-019 (deliverable audit)

**Previously resolved (discovered during audit):**
- F-001, F-002 (commit 0198d82a — release flow fixes, already in main)
- CRB-017 (tests already existed for all config loaders)

**Remaining:**
- None. All remediation items are complete or in progress via open PRs.

**Critical path for beta:** No remediation items remain. The backlog is fully
resolved across PRs #505–#520.

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
| Stack Integration | STACK | In Progress | 2 (STACK-018, STACK-019 — PR #518 open) |

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
