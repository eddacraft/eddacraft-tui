# APS Reconciliation Sweep — 2026-05-29

Advisory, read-only sweep of the ~95 active APS modules. The deterministic
checks (`index-counts.mjs`, `drift-check.mjs`) were clean for counts and drift;
`active-lint.mjs` surfaced one structural error. The findings below are
semantic drift — claims a module makes about itself or its dependencies that
contradict the on-disk reality — which the deterministic gates cannot catch.
This sweep changes no `plans/modules/` file; each item lists a suggested fix
for a follow-up reconciliation PR.

## Summary

- **Deterministic floor:** 0 count drifts across 41 counted modules, 0
  `drift-check.mjs` findings, **1 `active-lint.mjs` error** —
  `plans/modules/weave.aps.md` is missing its `## Work Items` section (E002).
- **Actionable semantic findings:** 29 (3 high, 12 medium, 14 low).
- **Real-but-intentional (already-explained) findings:** 0.

There is real work to do here, but it is bookkeeping, not feature risk: most
items are stale cross-references to archived modules, contradictory
`Released/Shipped` provenance lines tied to `v0.7.0-beta`, and header/body or
narrative/count mismatches. Two findings carry systemic value — one off-by-one
count that the count gate is structurally blind to (`####` task headings), and
the lone `active-lint` structural error on `weave`.

## Actionable drift (prioritized)

### High

| Module | Kind | Claim | Suggested fix |
| --- | --- | --- | --- |
| `weave.aps.md` | structural (active-lint E002) | Module has no `## Work Items` section | Add a `## Work Items` section so the module passes `active-lint.mjs` (currently the only failing file of 101 checked). |
| `early-access-tests.aps.md` | pr-claim-unverified | EATEST-019..023 all cite `Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)` | Repoint all five Status lines (229/240/250/260/270) to the correct anchor `05206228e · 2026-05-13` (`test(architecture): add EATEST-019..023`); `d7873161` is an unrelated Win32-pipe fix touching no `crates/anvil-architecture/` files. |
| `graph-v2-foundation.aps.md` | archived-ref | Depends on `anvil-intercept`/INTD "when implementation lands"; GV2-013/-023 link `plans/modules/surface-drivers.aps.md` | INTD (Complete 16/16) and DRVR (Complete 5/5) are archived; drop "when implementation lands", repoint GV2-013/-023 links to `plans/archive/modules/surface-drivers.aps.md`, note both as archived-complete substrate deps. |
| `realtime-ai-validation.aps.md` | archived-ref | DRVR / RMCP / LAUNCH linked via live `./*.aps.md` paths | All three (`surface-drivers`, `rust-mcp-launch-shim`, `launch-flow-readiness`) are archived; repoint the `./` links (lines 104, 215-223, 229) to `../archive/modules/`, matching the already-fixed INTD links. |
| `rust-cli-tier2.aps.md` | count-narrative-mismatch | Stats table labels Phase 1 & 2 `Proposed` while header reads `In Progress 5/9` | RCLI2-001..004 are all `Done` (commits + 2026-04-26 audit note); flip the Phase 1 & 2 Stats cells from `Proposed` to `Done`. |
| `rust-mcp-full-port.aps.md` | pr-claim-unverified | Stats table says RMCPF-011/-012 `Merged via PR #1558`; task bodies say `Released/Shipped via v0.7.0-beta` | Both provenances verified and consistent — the Stats caption is just stale; update the Phase 1 row to `Released/Shipped via v0.7.0-beta, d7873161 · 2026-05-21, PR #1558`. |

### Medium

| Module | Kind | Claim | Suggested fix |
| --- | --- | --- | --- |
| `multilayer-protection-v2.aps.md` | count-narrative-mismatch | Header `70/87`; narrative says total 86 / done 65 | True off-by-one: ground-truth is 71 done / 87 total. Set header (line 5) and the `index.aps.md` MLP2 row to `71/87`, drop the stale "done 65 / 3 released-shipped counted separately" narrative. **Root cause:** `extractModule` only matches `###` headings, so 8 `####`-using modules (MLP2 + compliance-policy-packs, opa-enhancements, policy-action-taxonomy, test-coverage-uplift, test-external-services, agent-governance-patterns, skill-discovery-observability) skip the count gate; widen the regex in `scripts/aps/lib/modules.mjs` or normalise to `###`. |
| `compliance-evidence-workspace.aps.md` | archived-ref | NOTE(post-rust) says `policy-lifecycle` is archived | `policy-lifecycle` is Draft and live (not archived) and is still a live Depends-on (line 42); remove the false "archived" assertion in the NOTE block (line 17). |
| `compliance-reporting.aps.md` | archived-ref | NOTE says opa-architecture-integration, drift-reporting, suppressions, `policy-lifecycle` are "all archived" | First three are archived; `policy-lifecycle` is still active (line 57). Edit the NOTE (lines 20-22) to exclude `policy-lifecycle` from the archived list. |
| `config-intelligence.aps.md` | archived-ref | Lists SURFENV (`surface-env-files`) under "Consumed by (live planning modules)" | SURFENV is archived; repoint references (lines 28, 38, 100) to `plans/archive/modules/surface-env-files.aps.md` and move it out of the live-modules list with an "(archived)" note. |
| `council-gate-bridge.aps.md` | status-body-mismatch | "Blocked — deferred until MLP-002 is `Merged` … do not start discovery" | MLP-002 shipped (Done 2026-05-13, archived via v0.7.0-beta); refresh the Status block + "Last reviewed" date to record the merge precondition is met, but keep Blocked pending the still-unmet schema-stability condition (MLP-002 landed as a v1 spike with MLP-002b / DAG-merge follow-ups open). |
| `distribution-and-update.aps.md` | prod-wireup-unverified | DISTRIB-001/-002/-003 all `Released/Shipped via v0.7.0-beta` while bodies say release preconditions unmet | Self-contradicting. Verify the embedded minisign key, advisory-in-release-body, and macOS smoke matrix against `v0.7.0-beta`; either revert to `Merged via PR #1562/#1569/#1652` or delete the "will advance … once" clauses if genuinely met. |
| `lang-ts-audit.aps.md` | count-narrative-mismatch | Header `Done 3/6`; prose says "five anticipated tasks", "two … completed" | LANGTS-006 was appended (Merged 2026-05-21, PR #1820); update the Tasks intro prose (132-135) to "six anticipated tasks", three Done, matching `3/6`. |
| `markdown-governance.aps.md` | status-body-mismatch | Header `Draft`; all six MDGOV-001..006 items `Ready` | Owner is `—` (the stated sole blocker), so reconcile downward: set the six item Statuses back to `Proposed` and re-canonicalise the header `Draft` → `Proposed` (module line 8 + `index.aps.md` line 669). Flip to Ready only when an owner is named. |
| `opa-agent-orchestration.aps.md` | status-body-mismatch | Header `Ready` while OPAG-001/-004 depend on unstarted Draft OPAE tasks | A Ready module gated on a Draft module's unstarted items can't execute; demote OPAG to `Blocked` (module line 5 + `index.aps.md` line 485) with a NOTE naming the OPAE-001/-012/-024/-025/-026 gate. |
| `rust-cli-tier3.aps.md` | pr-claim-unverified / status-body-mismatch | RCLI3-001 & RCLI3-017b `Released/Shipped` but bodies say advance is "pending" | Tag verifies (v0.7.0-beta = d7873161, both commits are ancestors); delete the stale "Cleanup agent will advance … once a tagged release records the commit" sentence in both bodies (~154-155, ~547-548) and drop RCLI3-001's stray unbalanced `)`. |
| `test-coverage-uplift.aps.md` | pr-claim-unverified | TCOV-012 marked Complete via "landed in this branch" with no SHA | It is on main (impl `b06e49a03`) but the path is stale (git-mv'd to archive under ADR-033); cite `b06e49a03` and the archive path `archive/anvil-ts-scanner/runtime-gate/policy.integration.test.ts`. Apply the same SHA-cite fix to TCOV-013. |
| `tracing-foundation.aps.md` | archived-ref | Anti-drift hook lists `launch-flow-readiness.aps.md` as a live co-edit sibling; prose "As LAUNCH archives" (future tense) | LAUNCH is Complete 18/18 and archived; drop it from the same-PR co-edit list (line 45), rewrite line 39 to past tense, bump Last reviewed. |

### Low

| Module | Kind | Claim | Suggested fix |
| --- | --- | --- | --- |
| `dashboard-foundation.aps.md` | archived-ref | DASH-009 (Done) deliverable edits retired `.claude/rules/aps-project.md` | Repoint the Files entry / Expected Outcome away from the retired rules file (use `plans/index.aps.md` / no per-repo rules file) or add an Audit note that the file-map step was obviated by the retirement. |
| `early-access-migration.aps.md` | archived-ref | EAMIG-039/-040 Files point at `archive/eddacraft-tui-local/src/widgets/log_panel.rs` with a "re-target to `crates/anvil-tui/src/widgets/`" note | The re-target note is itself wrong (no `log_panel.rs` there); repoint to `crates/eddacraft-tui/src/widgets/log_panel.rs` and correct the description — eddacraft-tui is a local workspace path dep, not a published 0.1.0 crate. |
| `lineage-authorship-confidence.aps.md` | status-body-mismatch | Header `Ready`; own Audit note (2026-04-26) says "not executable" without rework | Flip LAC header `Ready` → `Draft` (line 5) to match the note; validation lines still target the retired TS Nx project. |
| `org-policy-hierarchy.aps.md` | archived-ref | `index.aps.md` ORGHIER row lists archived `opa-architecture-integration` | Module body is reconciled; repoint the `index.aps.md` ORGHIER cell (line 478) to drop/annotate the archived dep. Same stale ref also on POLLC (479) and POLFED (481) if sweeping broader. |
| `test-coverage-uplift.aps.md` | status-body-mismatch | TCOV-015..021 carry no Status field | `Status` is a Required Field per `aps-rules.md`; add `- **Status:** Proposed` to each (counts already consistent, no header change). |
| `test-coverage-uplift.aps.md` | archived-ref | TCOV-019/020/021 Files target `archive/anvil-mcp-server/` | Target is archived; add `Blocked/Retired — archived target` Status (mirroring Phase 4's scope-drift callout) and a one-line note on the Phase 3 entry and line-79 Depends-on. |
| `test-external-services.aps.md` | other | Last-reviewed note: "still current on `dev`" | `dev` retired by OPMODEL-012 (2026-05-11); change to "still current on `main`" (lines 8-9). |
| `test-external-services.aps.md` | archived-ref | Depends-on line 66 cites bare `TFIX` | TFIX is archived; repoint to `plans/archive/modules/test-infrastructure-fix.aps.md` with an "(archived Complete)" marker. |
| `tracing-foundation.aps.md` | archived-ref | INTD-013/-014/-015 listed as live "Committed" coordination points (lines 107-125) | INTD is Complete/archived; repoint to `plans/archive/modules/intercept-daemon.aps.md` and change the "Committed" labels to "Complete (archived)". |
| `usage-insights.aps.md` | status-body-mismatch | Cross-References cites MLP-003 as "suppression log for suppression view" | MLP-003 is the pre-commit hook, not a suppression log; INSIGHTS-002's merged reconciliation says no suppression log exists. Remove the MLP-003 cross-ref (lines 232-233) or repoint to the `anvil-checks` antipattern scan. |
| `unified-config-format.aps.md` | archived-ref | UCFG-014 (Proposed) targets `archive/anvil-mcp-server/` | Target is archived/superseded by the Rust MCP shim; add an Audit note (and to sibling UCFG-015 targeting the archived VS Code extension) and either drop the task or flip Status to `Won't Do`/`Deferred`. |

## Real but intentional (no action)

None. Every semantic finding in this sweep was a genuine, unexplained
discrepancy — there were no cases where an in-body audit/rescope note already
justified the apparent drift.

## Method & limits

- **Read-only / advisory.** No `plans/modules/` file was modified; the
  suggested fixes above are for a follow-up reconciliation PR.
- **Deterministic floor** came from `index-counts.mjs --check --json`,
  `drift-check.mjs --json`, and `active-lint.mjs --json`, all run from the repo
  root. The five `index-counts` informational notes (EATEST, GATE, ILGOV,
  POLFED, UCFG having no managed N/M count cell) are by-design, not drift.
- **Prod-wireup claims are flagged, not proven.** This sweep did not trace Rust
  call sites; the `distribution-and-update` finding flags a status/precondition
  contradiction but does not verify the embedded minisign key, advisory body,
  or macOS smoke matrix against the binary.
- **PR/tag claims used `gh`/`git` best-effort.** Where a finding states a tag or
  commit "verifies", it was checked against ancestry of `v0.7.0-beta`
  (`d7873161`); treat as best-effort, not a release audit.
- **Known gate blind spot:** `drift-check.mjs`/`index-counts.mjs` skip the count
  gate for modules using `####` task headings (8 modules, see the MLP2 finding),
  so some count drift can pass the deterministic floor undetected.
