# APS Completion Audit — 2026-04-06

**Branch:** `chore/aps-completion-audit` (from `release/ts-packages`)
**Method:** Automated code-level validation of every work item's claimed status
against actual file existence, implementation substance, and validation criteria.

## Executive Summary

Audited **11 APS modules** covering **191 work items** (65 sampled for PBLU).
Found **3 categories of discrepancy**:

1. **Overstated completions** — items marked Complete that fail their own
   validation criteria (CRB-014, CRB-029, MAINT-005, MAINT-008)
2. **Understated progress** — items marked Draft with full implementations in
   the codebase (16 KERN items, 3 RATS items)
3. **Stale metadata** — `aps-project.md` header counts and file map comments
   don't match reality

### Headline Numbers

| | Claimed | Verified |
|---|---|---|
| Total items marked Complete | ~148 | ~131 genuinely complete |
| Items overstated (marked Complete, not fully done) | 0 | 8 found (2 FAIL, 6 PARTIAL) |
| Items understated (marked Draft, actually done) | 0 | ~19 found |
| Accurate modules | — | EMBER, STACK, EDDA, RENG, PBLU, SECB |

---

## Per-Module Results

### Complete Modules — Verified Accurate

| Module | Claimed | Verified | Notes |
|--------|---------|----------|-------|
| EMBER | 14/14 | **14/14 PASS** | All implementations substantive with tests |
| STACK | 19/19 | **19/19 PASS** | Full traceability including execution steps |
| EDDA | 19/19 | **19/19 PASS** | All services, CLI commands, and docs present |
| PBLU | 57/57 | **20/20 sampled PASS** | Representative sample across all waves |
| SECB | 8/8 | **8/8 PASS** | All security fixes verified in code |
| RENG | 4/6 | **4/6 accurate** | 4 Done verified; 2 Draft show partial progress |

### Complete Modules — Discrepancies Found

#### EERB (Edda-Ember Review) — 16/16 claimed → 14 PASS, 2 PARTIAL

| Item | Issue |
|------|-------|
| EERB-009 | Search still runs against 100-char truncated index entry; matches past char 100 produce false negatives |
| EERB-010 | Convenience methods don't accept `limit` parameter as specified; default raised to 1000 but not configurable per-call |

Both are low-severity — "good enough" fixes that don't fully satisfy their
stated validation criteria.

#### CRB (Code Review Backlog) — 29/29 claimed → 22 PASS, 4 PARTIAL, 2 FAIL

| Item | Severity | Issue |
|------|----------|-------|
| **CRB-014** | **FAIL** | `git-status.test.ts` and `git-agent.test.ts` (52 claimed tests) do not exist |
| **CRB-029** | **FAIL** | All 12 claimed command test files are entirely absent (login, logout, whoami, authorship, drift, explain, new, plan, release, validate, welcome, architecture) |
| CRB-016 | PARTIAL | Windows separator tests only in `suppress.tool.test.ts`; missing from `fix.tool.test.ts` and `resources.test.ts` |
| CRB-017 | PARTIAL | `packages/anvil/core/src/config/loader.test.ts` missing |
| CRB-021 | PARTIAL | Filenames inconsistent — `analyzer.ts` (American) vs `Analyser` class names (British) |
| CRB-022 | PARTIAL | `policy.ts` decomposed but 8 other command files exceed the 300-line threshold |

CRB-014 and CRB-029 likely represent test files lost during branch migration or
rebase. The `release/ts-packages` branch may not contain all commits from the
original implementation branch.

### In-Progress Modules — Status Corrections Needed

#### MAINT (Codebase Maintenance) — Header says 8/10, module body says 6/10

| Item | Claimed | Actual | Issue |
|------|---------|--------|-------|
| MAINT-005 | Complete | **PARTIAL** | `json()` helper used in 27 files but `JSON.stringify` still direct in ~14 command files |
| MAINT-008 | Complete | **FAIL** | `createSpinner` used in 21 files but direct `ora()` imports remain in 7 command files |
| MAINT-006 | In Progress | PARTIAL | Generator exists on branch but not merged to main |
| MAINT-007 | In Progress | PARTIAL | Same as MAINT-006 |

**Corrected count:** 4/10 Complete (not 6/10 or 8/10)

#### KERN (Rust Kernel) — 4/25 claimed → ~20/25 actually done

**This is the largest discrepancy in the audit.** All of Phases 1–4 are fully
implemented with tests and benchmarks, yet remain marked Draft:

| Phase | Items | Claimed | Actual |
|-------|-------|---------|--------|
| Phase 0 — Spike | KERN-001–004 | 4 Done | 4 Done ✓ |
| Phase 1 — Watcher + Parser | KERN-005, 010–013 | 5 Draft | **5 Done** |
| Phase 2 — Semantic Graph | KERN-020–023 | 4 Draft | **4 Done** |
| Phase 3 — Policy + Events | KERN-030–033 | 4 Draft | **4 Done** |
| Phase 4 — Integration | KERN-040–043 | 4 Draft | **3 Done, 1 Partial** (KERN-042: TS engine side placeholder) |
| Phase 5 — Daemon | KERN-044, 050–052 | 4 Draft | 1 Done (KERN-043 benchmarks), 3 Draft ✓ |

#### RATS (Ratatui TUI) — 1/7 claimed → ~4–5/7 actually done

| Item | Claimed | Actual | Evidence |
|------|---------|--------|----------|
| RATS-001 | Done | Done ✓ | Full shared crate with 12+ widgets, theme system, snapshot tests |
| RATS-002 | Draft | **Done** | Watch dashboard with 2×2 grid layout, event adapter, snapshot tests |
| RATS-003 | Draft | **Done** | Gate result viewer with 2-panel layout, navigation, snapshot tests |
| RATS-004 | Draft | **Done** | 4-step onboarding wizard with snapshot tests |
| RATS-005 | Draft | Partial | Migration infrastructure exists, `--tui` flag in 15 files |
| RATS-006 | Draft | Draft ✓ | No cross-terminal testing evidence |
| RATS-007 | Draft | Partial | Watch surface exists but full CLI wiring unclear |

---

## Stale Metadata in `aps-project.md`

1. **MAINT count:** Header says `8/10`, module body says `6/10`, verified is
   `4/10`
2. **KERN count:** Header says `4/25`, verified is `~20/25`
3. **RATS count:** Header says `1/7`, verified is `~4–5/7`
4. **File map comment:** States "crates/ paths don't exist in this monorepo —
   work done in external workspace" but all crates are now present
5. **SECB-006 file path:** Points to `apps/website/app/api/waitlist/route.ts`
   (moved to `apps/anvil-api/src/routes/waitlist.ts`)

---

## Recommended Actions

### Immediate (status corrections) — DONE

- [x] Update KERN work items KERN-005, 010–013, 020–023, 030–033, 040–041,
  043 from Draft → Done
- [x] Update RATS work items RATS-002, 003, 004 from Draft → Done
- [x] Revert MAINT-005, MAINT-008 from Complete → In Progress
- [x] Update `aps-project.md` header counts to match reality
- [x] Remove stale "crates/ don't exist" comment from file map

### Investigate (possible lost work) — RESOLVED

- [x] CRB-014: `git-status.test.ts` exists on `dev` HEAD; `git-agent.test.ts`
  found in commit `04c996f0`. This audit preserves that historical evidence
  from the legacy TypeScript CLI lineage; it does not imply both files exist
  in the current repository layout.
- [x] CRB-029: 12 command test files were identified in commit `7a549c55` on
  `dev` and appear to have been removed during merge conflict resolution
  `491d4cf3`. Recorded as historical evidence for the legacy/archived CLI;
  the current Rust CLI repository does not contain these `apps/anvil-cli/...`
  test files.

### Low Priority (partial completions)

- [ ] EERB-009: Search against full statements instead of truncated index
- [ ] EERB-010: Add optional `limit` parameter to convenience methods
- [ ] CRB-016: Add Windows separator tests to `fix.tool.test.ts` and
  `resources.test.ts`
- [ ] CRB-017: Create `packages/anvil/core/src/config/loader.test.ts`
- [ ] CRB-021: Reconcile American/British spelling in filenames
- [ ] CRB-022: Decompose remaining 8 large command files (or adjust target)
- [ ] MAINT-005: Migrate remaining ~14 direct `JSON.stringify` usages
- [ ] MAINT-008: Migrate remaining 7 direct `ora()` imports
