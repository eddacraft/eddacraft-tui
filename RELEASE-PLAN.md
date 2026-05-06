# Anvil Release Plan

**Last updated:** 2026-05-05 (post `v0.5.1-beta` ship — next-release slate
unlocked)

> Companion: [ROADMAP.md](./ROADMAP.md) — themes, big bets, horizons. Source of
> truth for module status: [`plans/index.aps.md`](./plans/index.aps.md).

---

## Recently shipped

| Tag           | Date       | Theme                               | Headline                                                                                                                                  |
| ------------- | ---------- | ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `v0.5.0-beta` | 2026-05-01 | AI Guardrails & Mid-Edit Validation | Locked slate **A1+A2+A3+A4** — RTAI Spike + Rust MCP launch shim + AIGUARD envelope + hygiene + language floor (44 items)                 |
| `v0.5.1-beta` | 2026-05-03 | Scanner Signal & TUI Hotfixes       | Patch — secret/antipattern FP fixes, TUI zoom controls, audit env-template filtering, kernel import bug fixes, TS scanner archive cascade |

**`v0.5.1-beta` closeout status:** shipped and verified. The private and public
release records both point `/releases/latest` at `v0.5.1-beta`; both are normal
GitHub releases because GitHub does not allow prereleases to be marked Latest.
Install site returned HTTP `200`. WinGet follow-up PR
[microsoft/winget-pkgs#367974](https://github.com/microsoft/winget-pkgs/pull/367974)
merged after the generated-manifest `Icons:` correction. Tracking issue
[#1233](https://github.com/eddacraft/anvil-001/issues/1233) remains open only as
the durable release log.

**`v0.5.0-beta` validation backend:** embedded-fallback-backed, **not**
daemon-backed. RMCP-005's `DaemonValidationClient` defaulted to `Unavailable`,
so MCP `tools/call` ran through the embedded `anvil-checks` pipeline. Daemon
wiring is the headline carry-over (see A1 below).

**`v0.5.0-beta` GUI dry-run gaps** (tracked outside the contract, now closed):

- **#1194** — closed: `anvil mcp install` now supports non-default command
  validation for release-candidate / side-by-side binary dry-runs.
- **#1195** — closed: Claude Code install/verify now targets the path Claude
  Code reads instead of the obsolete `~/.claude/mcp.json` location.
- **#1197** — closed: the Rust MCP shim now gives clients explicit pre-write
  validation instructions for `anvil_validate_write`.

---

## CURRENT RELEASE — slate UNLOCKED

The window after `v0.5.1-beta` is open. Pick from Tier A; everything else
queues. Lock the slate before tagging.

**Target tag:** TBD — `v0.5.2-beta` if the slate stays patch-shaped (carry-over
plus small driver reach), `v0.6.0-beta` if the slate accepts a coherent feature
slice (daemon-backed RMCP, RMCPF, dashboard MVP).

**Theme proposal:** _Wow-Start activation + Daemon-Backed RTV_ — make
`install → cd repo → anvil start` the canonical first minute (LAUNCH module's
activation slice from the 2026-05-03 council), and graduate the MCP path from
embedded fallback to the daemon-backed pipeline so the activation claim is
literal. (See ROADMAP Horizon 1 + brainstorms in
`plans/brainstorms/2026-05-02-wow-start-*.md`.) Adopt or replace at lock time.

**Current progress snapshot:**

| Area                            | State                                                                      | What remains                                                       |
| ------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| Release closeout                | `v0.5.1-beta` shipped, latest-corrected, public/private artefacts verified | Close tracking issue #1233 when no further log entry is needed     |
| Wow-start activation (`LAUNCH`) | In Progress, 16/18 complete                                                | LAUNCH-009.6, LAUNCH-011                                           |
| Daemon-backed MCP (`RMCP`)      | Complete, 8/8                                                              | Full parity moves to RMCPF; driver/RTAI follow-ups remain separate |
| Carry-over hardening (`V050F`)  | In Progress, 11/16 complete                                                | V050F-006, -007, -008, -011, -015                                  |
| `V060F` nominations             | Complete, 1/1                                                              | No open nomination work                                            |

### Carry-over backlog (rides any tag, regardless of theme)

These are non-blocking but should not accumulate as silent debt. Triage at lock
time; pick the ones that match the cut.

| Source                                                        | State          | Open items                                                                                                                                       |
| ------------------------------------------------------------- | -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| [`V050F`](./plans/modules/v050-release-followups.aps.md)      | 11/16 complete | 5 open: allowlist regex cache, eager rayon init, CI-class bench baseline, `scan_content` compile-error surfacing, `svix → uuid` override removal |
| [`V060F`](./plans/modules/v060-release-candidates.aps.md)     | 1/1 complete   | None; RCLI2-009 admin command parity is done                                                                                                     |
| `v0.5.0-beta` GUI dry-run gaps                                | Closed         | #1194, #1195, and #1197 are closed; validate their behaviours through LAUNCH-009 when activation wires Cursor / Claude Code                      |
| [`#1191`](https://github.com/eddacraft/anvil-001/issues/1191) | Closed         | Keep the ADR-031 baseline-comparison check as the daemon-backed latency regression guard                                                         |

---

## TIER A — Current-release candidates

### A1. Wow-Start Activation — _the headline next-tag investment_

**Goal:** Make `install → cd repo → anvil start` the canonical first minute. The
user gets a literal protection claim in seconds, with Cursor and Claude Code MCP
paths activated honestly and watch mode as a save-time fallback. Five
independent agent brainstorms (Claude / Codex / Copilot / Gemini / Opencode)
converged on the same gap; the planning council ratified `anvil start` as the
activation entrypoint on 2026-05-03.

**Source artefacts:**

- Pitch (5 brainstorm docs):
  [`plans/brainstorms/2026-05-02-wow-start-{claude,codex,copilot,gemini,opencode}.md`](./plans/brainstorms/)
- Executable plan:
  [`LAUNCH` module](./plans/modules/launch-flow-readiness.aps.md) — currently In
  Progress 16/18.
- Adjacent re-architecture brainstorm:
  [`plans/brainstorms/2026-05-01-hearth-rearchitecture.md`](./plans/brainstorms/2026-05-01-hearth-rearchitecture.md)

**Execution sequence (2026-05-04):** Deliver the LAUNCH activation slice as six
original reviewable PRs against `dev`, plus the LAUNCH-009.6 follow-up that was
split out during MCP spawn-probe review. APS ownership and progress remain in
[`plans/modules/launch-flow-readiness.aps.md`](./plans/modules/launch-flow-readiness.aps.md).

```text
PR 2 (LAUNCH-008/-012)  ── state vocabulary ──┐
                                              ├──► PR 1 (LAUNCH-002/-006)
PR 5 (LAUNCH-015/-016)  ── repo-language profile ──┤
                                                   ├──► PR 4 (LAUNCH-010/-014)
PR 6 (LAUNCH-013)       ── install detector ──── (independent)
                                                   │
PR 3 (LAUNCH-009/-011)  ── MCP + watch fallback ──┘  (depends on PR 2)
PR 7 (LAUNCH-009.6)     ── MCP tier semantics ────┘  (follow-up to PR 3)
```

| Order | PR   | Items                  | Branch                               | Risk                                  |
| ----- | ---- | ---------------------- | ------------------------------------ | ------------------------------------- |
| 1     | PR 2 | LAUNCH-008, LAUNCH-012 | `launch/a1-protection-states`        | Complete                              |
| 2     | PR 5 | LAUNCH-015, LAUNCH-016 | `launch/a1-language-profile-filters` | Complete                              |
| 3     | PR 6 | LAUNCH-013             | `launch/a1-install-upgrade-guidance` | Complete                              |
| 4     | PR 1 | LAUNCH-002, LAUNCH-006 | `launch/a1-start-entrypoint`         | Complete                              |
| 5     | PR 3 | LAUNCH-009, LAUNCH-011 | `launch/a1-mcp-activation-fallback`  | Partial — LAUNCH-011 remains open     |
| 6     | PR 4 | LAUNCH-010, LAUNCH-014 | `launch/a1-first-signal-integrity`   | Complete                              |
| 7     | PR 7 | LAUNCH-009.6           | TBD                                  | medium — tier-semantic reconciliation |

**Execution constraints:** Each PR references its LAUNCH item(s), includes tests
for acceptance criteria, passes council review before opening, remediates all
council findings, and follows up with reviewer comments after PR open. PR 3 must
also validate or honestly surface #1195 and #1197.

**Execution notes:** PR 1 (LAUNCH-006) promoted `anvil start` from a clap alias
for `welcome` to the dedicated activation entrypoint, so the prose references in
this plan describe state-after-PR-1. The APS LAUNCH file is authoritative for
counts (currently In Progress, 16/18). `v0.5.1-beta` shipped on 2026-05-03, and
the APS index header has been refreshed to that release.

**Modules / work items (2 outstanding):**

- **LAUNCH-009.6** — reconcile MCP tier semantics so the spawn probe can promote
  the diagnostic without losing `RestartRequired` meaning.
- **LAUNCH-011** — honest watch-mode fallback when MCP cannot attach.

**Recently completed:** LAUNCH-002 (watch action/TUI coexistence), LAUNCH-006
(`anvil start` activation entrypoint), LAUNCH-008 (protection states),
LAUNCH-009 (Cursor / Claude Code MCP activation), LAUNCH-009.5 (MCP spawn probe
observability), LAUNCH-010 (activation baseline), LAUNCH-012 (verification),
LAUNCH-013 (version/upgrade guidance), LAUNCH-014 (protection-loop tutorial),
LAUNCH-015 (language profile), and LAUNCH-016 (language-aware scan/watch
filtering).

**Prerequisites:**

- A1 ships against the **embedded-fallback** RMCP backend that landed in
  `v0.5.0-beta`. Daemon graduation (A2) is _not_ a prerequisite — wow-start
  inherits the daemon path automatically when A2 lands.
- Editor reach is **scoped to Cursor and Claude Code in v1.** Windsurf, VS Code,
  Copilot CLI, Codex CLI, and process auto-attach are explicitly out-of-scope
  until RMCP / DRVR verifies them.
- Confirm the closed `v0.5.0-beta` GUI gap fixes **#1195** (Claude Code path
  mismatch) and **#1197** (clients ignored `anvil_validate_write`) in the
  LAUNCH-009 validation path — without those behaviours, the activation claim is
  a lie.

**Out-of-scope (council-locked):**

- No-args TUI theatre (`anvil` with no subcommand auto-attaching to a running AI
  session). Council rejected as "rigged demo" risk; `anvil start` is the honest
  surface.
- Rule-file injection (`.cursorrules`, `.clauderules`, global AI rules) as
  enforcement. MCP pre-write validation is the only v1 enforcement claim.
- Demo fixtures, challenge files, or guaranteed-catch prompt catalogues before
  local protection is working.
- Cloud login, team policy pull, CI setup.
- Git hook installation as a default activation step.
- Surface-driver migration (DRVR) — assumes in-process Rust surfaces stay
  authoritative for the activation cut.

**Adversarial risk:** **First-repo lottery.** If the activation lands on a clean
/ small / unfamiliar-language repo, the protection claim is literal but empty —
the user sees "activated, no findings yet" and bounces. **Mitigation:**
LAUNCH-010 (baseline old findings) explicitly seeds context so the first genuine
save produces a signal; LAUNCH-015 (repo language profile) names the gap
honestly when the repo's languages are out of scope, instead of pretending
coverage; LAUNCH-014 (faster tutorial) gives a guaranteed-value path when the
live repo doesn't trip anything. Secondary risk: claiming "attached" when
activation is partial, or surfacing false positives on out-of-scope languages
(e.g. JS-shaped antipattern findings on `.py` files). LAUNCH-008 + LAUNCH-012
(protection states + verification) and LAUNCH-016 (language-aware filters) are
the literal fixes — be honest about what's wired and don't scan what we don't
support.

**Recommendation: PICK. This is the next-tag headline. The five-brainstorm
convergence + council ratification means the framing is locked; what's
outstanding is execution against LAUNCH's 2 open items.**

---

### A2. Daemon-Backed RMCP + Driver Reach — _the launch-blocker substrate_

**Goal:** Graduate the MCP launch path from embedded fallback to the
daemon-backed pipeline, and bring the driver framework + at least one editor
surface online. Same demo, real backend.

**Modules / work items (~19 items):**

- **RMCP-005 graduation** — complete. The live JSON-RPC client is committed;
  `tools/call` uses the daemon-backed path when owner-only IPC is available and
  keeps the embedded path as the correctness-equivalent fallback.
- **RTAI subset** (4 items): RTAI-004 (driver-side debouncer), RTAI-005 (editor
  mid-edit path), RTAI-007 (telemetry mirror), RTAI-009 (architecture doc +
  supersession links).
- **DRVR-001 / DRVR-002** — shared driver client + editor-driver protocol
  (consumes the AIGUARD envelope).
- **Remaining INTD items** (-004, -006, -008..-012, -015, -016) — watcher
  integration, process-group interrupt, configuration loading, embedded mode,
  unregistered-change handling, status / diagnostics, Windows CI matrix,
  telemetry subscription scoping, DoS protection budgets.
- **ADR-031 latency CI gating** (#1191) — closed; retain the baseline-comparison
  check as the regression guard for the daemon-backed slice.

**Prerequisites:**

- Confirm `scan_buffer` RPC + INTD-002 listener in place (already shipped).
- Lock the daemon-vs-embedded fallback decision: _embedded remains the
  correctness-equivalent fallback when the daemon is not running_.
- Pin the AIGUARD envelope contract test before RTAI-004 commits, so driver +
  daemon paths cannot diverge.

**Out-of-scope (protect the slice):**

- RMCPF full parity port — separate Tier A candidate (A3).
- Dashboard MVP and `anvil export` — A6.
- INTL (intercept launcher / wrapped-launch v2) — Tier B.

**Adversarial risk:** **Two backends, one envelope.** With embedded fallback +
daemon-backed both shipping in the same release, contract drift between them is
the most likely regression source. Wire the AIGUARD-envelope contract test
against both paths in CI before either implementation lands.

**Recommendation: PICK as the substrate beneath A1. Without the daemon path,
A1's "literal protection claim" inherits A1's embedded-fallback backend —
correct, but the wow-start council outcome is stronger when both ship
together.**

---

### A3. RMCPF — Rust MCP Full Port

**Goal:** Graduate the launch shim to feature parity with the archived TS MCP
server. Reuses the AIGUARD `anvil.diagnostic.v1` envelope.

**Modules / work items:** RMCPF (scope to be detailed in module — currently
parked in Horizon 6 of the roadmap, promote to Tier A here when ready).

**Prerequisites:**

- A2's daemon-backed `tools/call` path stable, or A3 ships against embedded
  fallback only and inherits A2's daemon graduation when it lands.
- Archive cascade complete (TSRET-005 — done).

**Out-of-scope:**

- New MCP tools beyond TS-MCP parity surface.
- IDE-specific behaviour (separate driver tracks).

**Adversarial risk:** Without a parity test harness, "full port" silently
drifts. The archived TS server in `archive/anvil-ts-scanner/` is the parity
oracle until the harness retires.

**Recommendation: CONSIDER. Strong narrative pair with A2; pure engineering
work, no product expansion. Defer if A1+A2 takes all bandwidth.**

---

### A4. Release Engineering Tail — _carry-over from `v0.5.0-beta` A3_

**Goal:** Finish the attribution pipeline v3 and complete parallel-scan rollout.
Pure hygiene, no product surface.

**Modules / work items (~10 items):**

- **ATTRIB-004..-011** — full attribution pipeline v3 (downstream port, public
  mirror, kit polish, etc.)
- **SCAN-004** — provenance recording on parallel-scan results
- **SCAN-005** — WalkParallel rollout to remaining call-sites

**Prerequisites:**

- None outside the slice. ATTRIB-001/-002/-003 and SCAN-001/-002/-003 already
  shipped in `v0.5.0-beta`.

**Out-of-scope:**

- TINT entirely; EAMIG/EATEST entirely (Tier B).
- New attribution sources beyond the v3 design.

**Adversarial risk:** ATTRIB tail wants to expand into a "downstream port"
milestone. Cap scope at the items already in the module; resist new pulls.

**Recommendation: CONSIDER. Independent of A1/A2/A3; rides the next tag
cleanly.**

---

### A5. Language Credibility Tail — _carry-over from `v0.5.0-beta` A4_

**Goal:** Finish the language credibility floor — drift schema versioning, FP
reporting, SURFSQL Phase 1.

**Modules / work items (~12 items):**

- **LANGTS-002 / LANGTS-004 / LANGTS-005** — TS substrate (warn-only) remaining
  items.
- **OPSUP-002..-007** — drift schema versioning, per-track flags, file-presence
  guards, FP reporting, anchor re-scoring process owner (still unassigned —
  decide before tagging).
- **SURFSQL Phase 1** — `.sql` structural governance surface.

**Prerequisites:**

- ADR-027 / ADR-028 / ADR-029 accepted (done).
- Anchor re-scoring process owner named before OPSUP-006 commits.

**Out-of-scope:**

- Rust anchor (RSTLAN), Python anchor (PYLAN) — Phase 2.
- All Track 4 packs (PACKPUL, PACKLLM, PACKDRZ, PACKNXT, PACKHON, PACKTOK).
- MDGOV.
- SURFGHA / SURFDOCK / SURFSH (Phase 3).

**Adversarial risk:** **PACKLLM PII heuristics are an FP minefield** — even
warn-only, a noisy first run on a prospect's repo would damage credibility more
than not shipping. Order: SURFSQL → LANGTS tail → OPSUP. Keep PACKLLM out.

**Recommendation: CONSIDER. Independent of A1/A2; pairs naturally with A4 if the
cut wants a "credibility tail" theme.**

---

### A6. Dashboard MVP — _"Team-Lead Glance"_

**Goal:** A team-lead opening `localhost:3000/dashboard` and seeing **last gate
run, current warnings ranked by severity, recent activity** without learning CLI
commands. Serves the buyer persona that funds the tool.

**Modules / work items (~12 items):**

- **DASH-001..006, DASH-008** (skip DASH-007 command palette; scope DASH-004
  charts to sparkline only).
- **DASHCORE-001** (overview metric cards), **DASHCORE-003** (activity feed),
  **DASHCORE-006** (warning list), **DASHCORE-007** (warning detail panel).
- **NEW glue item:** `anvil export` — writes canonical
  `.anvil/{warnings,gates,provenance,config}.json` from latest run state.
  Critical missing bridge between CLI and dashboard.
- _Optional add:_ DASHOPS-005 (config viewer), DASHOPS-006 (diagnostics).

**Prerequisites:**

- Pin DASH-005 to **today's CLI `--json` shapes**, not a future SCHEMA contract.
  Ship-now over governance.
- Add the `anvil export` CLI work item (1 task, owned by anvil-cli).
- Decide deployment model: local `nx dev website` only for v1 — no auth, no
  multi-user (matches D-DASH-001).

**Out-of-scope:**

- Architecture graph (DASHARCH-003, low confidence).
- Drift comparison views (DASHARCH-005/006).
- AI builder (DASHAI all).
- Suppression trends (DASHARCH-008).
- Plan approval workflows.
- Audit user/AI-tool breakdowns (DASHOPS-002/003).
- Real-time SSE.

**Adversarial risk:** "Why use this instead of `anvil check` in CI logs?" Honest
answer: only if the warning list with file/line + severity grouping is genuinely
faster to triage than scrolling CI output. **Smallest credible demo is therefore
DASHCORE-006 + DASHCORE-007 alone.** Build that first, demo to one
platform-engineer external user before committing to the full Tier A.

**Recommendation: CONSIDER. Largest slice on the list. Ships the team-lead
persona narrative. Defer entirely if A1+A2 takes all bandwidth — the wow-start
activation is the higher-priority persona expansion this window.**

---

## TIER B — Queued (next slice candidates)

### B1. Intercept Loop v0 — _the wrapped-launch v2 narrative_

**Goal:** `anvil-run`-wrapped agent launches with mechanical fence-on-fail
enforcement.

**Modules:** INTD remainder not in A1, INTL (all 9 items), INTR-004 (path-deny).
~20 items.

**Why queued:** Coherent product story but **not the wow-start path**. Promote
after A1 (wow-start activation) ships and A2 (daemon-backed RTV) is stable.

**Adversarial risk:** Cross-platform Windows parity (INTD-012). Job Object
semantics ≠ PGIDs. INTL-004 alone is two weeks of platform work that does not
show up in the demo.

---

### B2. Enterprise Readiness Foundation

**Goal:** Multi-repo / fleet / enterprise deployment — the constellation that
answers "how does this deploy in front of N repos for an org-tier customer?"

**Modules (foundation cut):**

- **GATE** (3 items) — gateway topology + enforcement contract + observability.
- **POLFED** (8 items) — multi-repo federation workflow over OPAE bundles.
- **ORGHIER** (7 items) — multi-level policy hierarchy.
- **POLLC** (7 items) — lifecycle / canary / grace periods.

**Promotion gate:** first enterprise prospect or design-partner request, OR
internal decision to ship Anvil's own deployment topology as reference.

**Sequence:** GATE + POLFED first; ORGHIER + POLLC as the multi-tenant layer.
COMPLY/CEWS/TRUST follows in B3.

---

### B3. Compliance & Trust Surface — _enterprise auditor cut_

**Goal:** SOC 2 / ISO 27001 / NIST framework support, evidence workspace, public
trust artefacts.

**Modules:**

- **COMPLY** (8 items) — framework registry, policy-to-control mapper, posture
  scoring, report generator, historical posture.
- **CEWS** (4 items) — control-evidence model, ingestion, workspace views,
  export packs.
- **TRUST** (3 items) — trust artifact model, publishing pipeline, freshness/
  ownership rules.

**Why queued:** Sequenced after B2 foundation. CEWS depends on COMPLY's evidence
collector; do not start until COMPLY-001..004 are on the slice.

---

### B4. Beta Migration Hardening

**Goal:** Regression-prevention for v0.3.x findings — early-access migration
tests + integration surface.

**Modules:**

- **EAMIG** (50 items, Ready, Rust-aligned).
- **EATEST** (38 items, Ready, Rust-aligned).
- **TINT** (15 items, flip Draft → Ready — TFIX/RCLI/KERN deps now
  archived-Complete).

**Smallest viable cut:** All EAMIG **High** priority + all EATEST **High**
priority + TINT subprocess contract tests. Walk down by Priority, not by Phase.

**Why queued:** Too large to combine with A1/A2 in the same window.

---

### B5. Phase 1 Language Spec-Faithful — _Council D's Candidate 2_

**Goal:** The MVP named in `2026-04-08-language-and-coverage-design.md` §9 —
"Anvil governs four file shapes" pitch deck slide.

**Modules:** LANGTS + SURFSQL T2 + PACKPUL + PACKLLM (TS substrate, warn-only)

- OPSUP slices 1 & 2.

**Prerequisites:** A5 tail must land first (LANGTS tail, OPSUP-002..-007).

**Adversarial risk:** **High.** Five modules pulled live in parallel with A1

- A2 work is an attention crisis. PACKLLM's PII heuristics are an FP minefield —
  even warn-only, a noisy first run on a prospect's repo would damage
  credibility more than not shipping. Recommend ordering: PACKPUL → SURFSQL →
  PACKLLM (TS) only after A1+A2 is past its own validation.

---

### B6. Skills Discovery & Observability (SKOBS)

**Goal:** Skill catalogue + observability for agent skill packs (registered
2026-05-04). Module exists; scope and items being captured.

**Modules:** [`SKOBS`](./plans/modules/skobs.aps.md).

**Why queued:** Newly registered. Promote to Tier A when scope locks and
prerequisites are clear.

---

### B7. Schema Contracts (SCHEMA) and Test Quality follow-on

**Goal:** TS↔Rust parity governance + integration/external-services testing.

- **SCHEMA** (6 items) — TS↔Rust contract parity; activate when the parity
  surface starts churning.
- **TEXT** (test-external-services, 14 items) — external service contract tests;
  not on launch critical path.
- **TINT** — covered in B4.

---

### B8. Rust CLI Tier 2 — _re-audit before commit_

**Goal:** Extend RCLI parity surface — interactive-mode polish, OPAE-blocked
items.

**Caveat:** Half the listed items already exist in
`crates/anvil-cli/src/ commands/`. **Re-audit RCLI2 against current crate state
before promoting to Ready.**

**Genuinely outstanding:** pr-comment, exception, policy-debug, policy-watch.
The OPAE-blocked subset (RCLI2-005..-008) is **Tier C** until OPAE moves.
RCLI2-009 (admin command parity) is already nominated to V060F as a carry-over
candidate.

---

## TIER C — Parked (waiting for demand pull)

These do not compete for current-release attention. Keep in `plans/modules/` for
cataloguing; promote on signal.

| Module                                                                                   | Why parked                                                                                                                                            |
| ---------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| **DASHAI** (dashboard-ai-builder)                                                        | Wave 4 of dashboard. Coordinate with TUIDASH json-render schema post-launch.                                                                          |
| **DASHARCH** (dashboard-architecture-views)                                              | Demote Ready → Draft pending real schema source from `crates/anvil-architecture` + drift snapshot format.                                             |
| **DASHOPS** remaining                                                                    | Plan/role/AI-tool views are spec-orphan today.                                                                                                        |
| **OBS** (observability-foundation)                                                       | Park, rescope post-launch against `apps/anvil-api`.                                                                                                   |
| **OPAE** (opa-enhancements)                                                              | 36 tasks is a programme. Only policy-library + bundle inheritance pieces are launch-relevant; defer until a "policy library beats gate" slice.        |
| **CPACKS** (compliance-policy-packs)                                                     | Shippable as ecosystem content after OPAE library + POLVAL.                                                                                           |
| **AGOV** (agent-governance-patterns)                                                     | Signal-producer module for CPACKS/MDGOV. Promote when CPACKS POLVAL prep lands.                                                                       |
| **OPAG** (opa-agent-orchestration)                                                       | Orchestration on a policy stack that does not exist yet.                                                                                              |
| **EVAL** (eval-harness-integration)                                                      | Adapter contract is small, useful for RTAI regression once A2 ships; revisit post.                                                                    |
| **CPOL** (contextual-policy-assertions)                                                  | Isolated, complements OPAE; small scope (3 tasks) — Tier B/C boundary.                                                                                |
| **IORISK** (io-risk-controls)                                                            | Closest to RTAI's input/output validation theme. Could enrich A2 as a 1–2 task addition, but **default: do not include** (dilutes "wow" with config). |
| **ATC** (adversarial-testing-catalog)                                                    | Pair with PATT as v0.7 safety pack.                                                                                                                   |
| **PATT** (prompt-attack-regression-packs)                                                | Pair with ATC.                                                                                                                                        |
| **POLVAL** (policy-pack-validation)                                                      | Necessary precondition for any pack work; promote when packs activate.                                                                                |
| **ARCHCFG** (architecture-config-validation)                                             | Could absorb into `crates/anvil-architecture` as a tier-2 item.                                                                                       |
| **TUIDASH** (tui-dashboard-render)                                                       | Demote Ready → Draft pending DASHAI catalogue resolution and schema source pin.                                                                       |
| **RCLI3** (rust-cli-tier3)                                                               | Genuinely useful for parity; pure historical-contract work. Frame as "post-launch parity."                                                            |
| **RSTLAN, PYLAN**                                                                        | Heavy anchors. Self-dogfood compelling, not launch-blocking.                                                                                          |
| **LANGTAIL, PACKTOK**                                                                    | Tier D in Council D's classification — defer until breadth becomes a sales blocker.                                                                   |
| **MDGOV** (markdown-governance)                                                          | M1 wellformedness as internal compounding value — promote slice 1 when bandwidth allows.                                                              |
| **WEAVE**                                                                                | Greenfield import + harness build. Schedule after intercept-loop thesis is proven.                                                                    |
| **PFGW, ILGOV, LAC, OPENSPEC, GV2, GCTX, UCFG, BMAD4, CGBDG, FLAGCAT, APGOV, SEC, TEST** | Various long-bet / future / signpost / cross-cutting items. See [ROADMAP.md](./ROADMAP.md) Horizon 6 + audit followup tasks #17–22.                   |

---

## Cross-cutting glue status

These are prerequisites surfaced by councils. They aren't slices on their own —
they're glue some Tier A picks need.

1. **GUI gaps #1195 and #1197 closed** — validated through A1 / LAUNCH-009; keep
   the Claude Code path fix and MCP `instructions` directive in the final
   release smoke so the activation claim stays literal.
2. **`anvil export` CLI work item** — required by A6 (Dashboard MVP). One task
   in `crates/anvil-cli`. Without it, dashboard has no canonical `.anvil/*.json`
   to read.
3. **AIGUARD-envelope contract test against both backends** — required by A2.
   Pin before RTAI-004 commits, so embedded-fallback and daemon-backed paths
   cannot diverge silently.
4. **Anchor re-scoring process owner** — required by A5 tail (OPSUP-006).
   Currently no permanent owner named.
5. **ADR-031 latency CI gating (#1191)** — closed; required by A2 as an ongoing
   regression guard for daemon-backed latency changes.
6. **TS MCP parity oracle** — required by A3. The archived TS server in
   `archive/anvil-ts-scanner/` is the parity oracle until a Rust-side parity
   harness retires it.

---

## Suggested first-pick combinations

| Combo                                | Slices                      | Open scope                              | Posture                                                                                               |
| ------------------------------------ | --------------------------- | --------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| **Patch-shaped tag**                 | Carry-over only             | V050F 5 open                            | Tags as `v0.5.2-beta`. Smallest credible cut.                                                         |
| **Wow-start minimum**                | A1                          | LAUNCH 2 open                           | Just the activation council outcome. Tags as `v0.5.2-beta` if scoped tight, `v0.6.0-beta` if broader. |
| **Wow-start + daemon (recommended)** | A1 + A2 + carry-over        | LAUNCH 2 + A2 remaining + V050F 5       | The headline + the literal-protection substrate. Tags as `v0.6.0-beta`. Highest confidence.           |
| **Wow-start + parity**               | A1 + A2 + A3 + carry-over   | A1/A2/carry-over + RMCPF scope          | Adds full Rust MCP parity. Strong narrative, real contention with A1.                                 |
| **Wow-start + hygiene**              | A1 + A4 + A5                | LAUNCH 2 + release/language tails       | Activation + finishes the `v0.5.0-beta` tails. Skips daemon graduation — A2 follows in next window.   |
| **Founder-pitch slate**              | A1 + A2 + A6                | LAUNCH 2 + A2 remaining + dashboard MVP | Activation + daemon + dashboard. Largest persona expansion. Highest A6/A1 bandwidth contention.       |
| **Full slate**                       | A1 + A2 + A3 + A4 + A5 + A6 | All Tier A remaining                    | Most ambitious; only realistic if A1 ships clean and bandwidth holds.                                 |

The councils' lesson from `v0.5.0-beta` plus the 2026-05-03 activation council:
**A1 + A2 + carry-over** is the highest-confidence cut for the next tag. A1
alone tags `v0.5.2-beta`-shaped; A1+A2 together earns `v0.6.0-beta`. A3 (RMCPF)
and A6 (Dashboard) are the high-leverage adds; both are the candidates most
likely to slip if A1 takes longer than estimated.

---

## Followup work tracked separately

Audit + council outputs identified rescope work that is **not part of any
release slice** but should not be lost. Tracked as tasks #17–23:

- **#17** Rescope ILGOV — Rust target + graph-effect-vs-intent.
- **#18** Rescope CFGINT — pick crate home + define graph artefact.
- **#19** Rescope AGOV — retarget paths + AGOV-002 migration to CPACKS.
- **#20** AIGUARD prep — clean Interfaces, coordinate envelope with RTAI/INTD.
- **#21** POLFED rescope + ADR — codify OPAE/POLFED boundary.
- **#22** GATE prep — coordinate contracts with INTD/DRVR/RTAI envelopes.
- **#23** Promote Enterprise Readiness constellation to Tier B.

These execute when bandwidth allows; none block any Tier A pick.

---

## What this document is NOT

- **Not an archive list.** See [ROADMAP.md](./ROADMAP.md) "Cuts and parks" for
  what changed.
- **Not the source of truth for module status.** That lives in
  [`plans/index.aps.md`](./plans/index.aps.md).
- **Not a schedule.** Tier classification reflects readiness and value, not
  date.
