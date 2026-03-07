# What's Next

Compiled 2026-03-07. Two lists: (1) remediation work needed for beta quality,
(2) feature work across all modules.

---

## List 1: Remediation, Fixes & Maintenance

Everything that fixes existing code — bug fixes, code review items, security
hardening, test gaps, maintenance, and documentation drift.

### CRB — Code Review Backlog (23/29 complete)

| ID | Summary | Priority | Status |
|----|---------|----------|--------|
| CRB-017 | Add tests for platform/core config loaders | Medium | Draft |
| CRB-018 | Standardise works-from-repo-root workflow | Medium | Draft |
| CRB-019 | Consistent logging/output conventions (stderr/stdout) | Medium | Draft |
| CRB-022 | Large command modules need decomposition (e.g. policy.ts) | Low | Draft |
| CRB-023 | Silent fallbacks without visibility — emit debug logs | Medium | Draft |
| CRB-025 | Docs and scripts drifting from reality — audit accuracy | Low | Draft |

### MAINT — Codebase Maintenance (1/8 complete)

| ID | Summary | Priority | Status |
|----|---------|----------|--------|
| MAINT-002 | Error formatting consistency across CLI commands | Medium | Draft |
| MAINT-003 | Workspace root resolution — consolidate into one utility | Low | Draft |
| MAINT-004 | Git operation wrappers — consolidate execFile/spawn calls | Medium | Draft |
| MAINT-005 | JSON output formatting — standardise `--json` envelope | Low | Draft |
| MAINT-006 | Nx generator for CLI commands | Low | Draft |
| MAINT-007 | Nx generator for gate checks | Low | Draft |
| MAINT-008 | Spinner/progress patterns — consolidate ora usage | Low | Draft |

### STACK — Edda Stack Integration (16/19 complete)

| ID | Summary | Priority | Status |
|----|---------|----------|--------|
| STACK-006 | Observation-to-Proposal type mapping | Medium | Draft |
| STACK-017 | Path drift cleanup in APS plan files | High | In Progress |
| STACK-018 | Retroactive evidence capture for STACK-001–016 | High | Draft |
| STACK-019 | Missing deliverable audit | Medium | Draft |

### F-series — Interim Review Findings (untracked)

From `plans/reviews/interim-finds-2026-03-04.md`. These are confirmed findings
not yet tracked as APS work items.

| ID | Summary | Priority |
|----|---------|----------|
| F-001 | Release smoke check uses Unix `ls` — breaks on Windows | P1 |
| F-002 | Workflow monitor marks wrong run as exact match | P1 |
| F-003 | Template rendering builds regex from unescaped variable names | P2 |

### ISS — Standalone Issues

From `plans/issues.md`. Only active (non-resolved) items.

| ID | Summary | Severity |
|----|---------|----------|
| ISS-004 | Pulumi Preview CI check failing on main (pre-existing) | Medium |
| ISS-006 | `preserve-caught-error` warnings across CLI/core/runtime (9 total) | Low |
| ISS-007 | `preserve-caught-error` warnings across CLI (4 remaining) | Low |

### Summary

| Category | Count |
|----------|-------|
| Code review (CRB) | 6 |
| Maintenance (MAINT) | 7 |
| Stack reconciliation (STACK) | 4 |
| Interim findings (F-series) | 3 |
| Issues (ISS) | 3 |
| **Total** | **23** |

**Critical path for beta:** F-001, F-002 (release flow), STACK-017 (plan
accuracy), CRB-025 (documentation drift).

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
| Stack Integration | STACK | In Progress | 1 feature (STACK-006) + 3 reconciliation |

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
| 0.4.0 (memory system) | 4 | ~4 remaining |
| Future — Rust | 4 | ~48 |
| Future — Other | 6 | ~54+ |
| **Total** | **29** | **~234** |

**Next actionable wave:** Dashboard Foundation (DASH, 9 tasks, Ready status) is
the first feature module that can start immediately.
