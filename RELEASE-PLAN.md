# Anvil Release Plan

**Last updated:** 2026-05-09 (added next-release window: daemon-working slate —
MLP + INTL + gates + docs)

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

## CURRENT RELEASE — slate LOCKED for `v0.6.0-beta`

A1 (Wow-Start Activation) and A2 (Daemon-Backed RMCP + Driver Reach Waves 1–3)
are both fully shipped on `dev`. The wow-start activation council outcome
(2026-05-03) is the literal protection claim, and A2 graduates the MCP path from
embedded fallback to the daemon-backed pipeline so the claim is real.

**Target tag:** **`v0.6.0-beta`** — A1 + A2 substrate together. The cut is ready
to tag; the V050F scanner-hotpath carry-over (V050F-006/-011 via #1323) and the
eager rayon pool init (V050F-007 via #1330) both merged ahead of the tag.

**Theme:** _Wow-Start activation + Daemon-Backed RTV_ —
`install → cd repo → anvil start` is the canonical first minute. Cursor and
Claude Code MCP paths activate honestly; watch mode is the save-time fallback
when MCP can't attach; daemon-backed validation backs the `tools/call` path when
owner-only IPC is available, with the embedded path as correctness-equivalent
fallback.

**v0.6.0-beta release artefacts:**

- `docs/runbooks/v0.6.0-beta-security-note.md` — operator-facing trust-boundary
  note (4 HIGH trade-offs documented: drivers.allow file mode, redaction hash
  unsalted, §4.4 redaction filter spec-only, Linux PID-reuse TOCTOU window).
- `docs/runbooks/v0.6.0-beta-release-runbook.md` — operator runbook (5 ops
  items: foreground daemon, Unix-only `intercept status`, fence persistence,
  macOS interrupt fencing, Windows CI gap).

**Current progress snapshot:**

| Area                                               | State                                                                      | What remains                                                                                                                                                 |
| -------------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Release closeout                                   | `v0.5.1-beta` shipped, latest-corrected, public/private artefacts verified | Close tracking issue #1233 when no further log entry is needed                                                                                               |
| A1 — Wow-start activation (`LAUNCH`)               | Complete, 18/18                                                            | LAUNCH-009.6 (PR #1303) + LAUNCH-011 (PRs #1300/#1301) merged 2026-05-06                                                                                     |
| Daemon-backed MCP launch path (`RMCP`)             | Complete, 8/8                                                              | Full parity moves to RMCPF; A2 graduated the daemon-vs-embedded path                                                                                         |
| A2 — Daemon-backed RMCP + driver reach (Waves 1–3) | Complete                                                                   | 8 PRs (#1304..#1311) + remediation (#1322); INTD 16/16, DRVR 5/5 active, RTAI 6/9; Wave 4 (RTAI-005/-007/-009, DRVR-003) out of cut per ADR-033              |
| v0.6.0-beta post-substrate polish                  | Complete                                                                   | macOS peer-cred parity (#1331), Windows `intercept status` + Cross-on-dev gate (#1325/#1329/#1332), MCP integration tests Unix-gated (#1335)                 |
| Carry-over hardening (`V050F`)                     | In Progress, 14/16                                                         | V050F-006/-011 closed via #1323; V050F-007 closed via #1330. Open: V050F-008 (CI-class bench baseline), V050F-015 (svix → uuid override). Both non-blocking. |
| `V060F` nominations                                | Complete, 1/1                                                              | No open nomination work                                                                                                                                      |

### Carry-over backlog (rides any tag, regardless of theme)

These are non-blocking but should not accumulate as silent debt. Triage at lock
time; pick the ones that match the cut.

| Source                                                        | State          | Open items                                                                                                                  |
| ------------------------------------------------------------- | -------------- | --------------------------------------------------------------------------------------------------------------------------- |
| [`V050F`](./plans/modules/v050-release-followups.aps.md)      | 14/16 complete | 2 open: CI-class bench baseline (V050F-008), `svix → uuid` override removal (V050F-015)                                     |
| [`V060F`](./plans/modules/v060-release-candidates.aps.md)     | 1/1 complete   | None; RCLI2-009 admin command parity is done                                                                                |
| `v0.5.0-beta` GUI dry-run gaps                                | Closed         | #1194, #1195, and #1197 are closed; validate their behaviours through LAUNCH-009 when activation wires Cursor / Claude Code |
| [`#1191`](https://github.com/eddacraft/anvil-001/issues/1191) | Closed         | Keep the ADR-031 baseline-comparison check as the daemon-backed latency regression guard                                    |

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
  [`LAUNCH` module](./plans/modules/launch-flow-readiness.aps.md) — Complete
  18/18 as of 2026-05-06.
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

| Order | PR   | Items                  | Branch                               | Status                                             |
| ----- | ---- | ---------------------- | ------------------------------------ | -------------------------------------------------- |
| 1     | PR 2 | LAUNCH-008, LAUNCH-012 | `launch/a1-protection-states`        | Complete                                           |
| 2     | PR 5 | LAUNCH-015, LAUNCH-016 | `launch/a1-language-profile-filters` | Complete                                           |
| 3     | PR 6 | LAUNCH-013             | `launch/a1-install-upgrade-guidance` | Complete                                           |
| 4     | PR 1 | LAUNCH-002, LAUNCH-006 | `launch/a1-start-entrypoint`         | Complete                                           |
| 5     | PR 3 | LAUNCH-009, LAUNCH-011 | `launch/a1-mcp-activation-fallback`  | Complete — LAUNCH-011 closeout via PRs #1300/#1301 |
| 6     | PR 4 | LAUNCH-010, LAUNCH-014 | `launch/a1-first-signal-integrity`   | Complete                                           |
| 7     | PR 7 | LAUNCH-009.6           | `launch/0096-tier-semantics`         | Complete                                           |

**Execution constraints:** Each PR references its LAUNCH item(s), includes tests
for acceptance criteria, passes council review before opening, remediates all
council findings, and follows up with reviewer comments after PR open. PR 3 must
also validate or honestly surface #1195 and #1197.

**Execution notes:** PR 1 (LAUNCH-006) promoted `anvil start` from a clap alias
for `welcome` to the dedicated activation entrypoint, so the prose references in
this plan describe state-after-PR-1. The APS LAUNCH file is authoritative for
counts (Complete, 18/18). `v0.5.1-beta` shipped on 2026-05-03, and the APS index
header has been refreshed to that release.

**Modules / work items:** all 18 LAUNCH items complete as of 2026-05-06.

**Shipped:** LAUNCH-002 (watch action/TUI coexistence), LAUNCH-006
(`anvil start` activation entrypoint), LAUNCH-008 (protection states),
LAUNCH-009 (Cursor / Claude Code MCP activation), LAUNCH-009.5 (MCP spawn probe
observability), LAUNCH-009.6 (MCP tier semantics, PR #1303), LAUNCH-010
(activation baseline), LAUNCH-011 (honest watch fallback, PRs #1300/#1301),
LAUNCH-012 (verification), LAUNCH-013 (version/upgrade guidance), LAUNCH-014
(protection-loop tutorial), LAUNCH-015 (language profile), and LAUNCH-016
(language-aware scan/watch filtering).

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
outstanding is execution against LAUNCH's 1 open item.**

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

**Execution order:** Treat A2 as a pre-wave contract gate plus four dependency
waves. Completed anchors are AIGUARD-001..-004, RMCP-005,
RTAI-002/-003/-006/-008, and the shipped INTD foundation
(INTD-002/-003/-005/-013/-014). Start new work only after confirming those
anchors still pass their contract tests.

| Wave | Items                                                                         | Purpose                                                                                                                    | Parallel delivery                                                                                                                                                                        |
| ---- | ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0    | AIGUARD envelope contract, RMCP-005 daemon fallback, RTAI-008 errors contract | Lock the shared response envelope before more consumers are added                                                          | Already complete; only re-run validation as an A2 gate                                                                                                                                   |
| 1    | DRVR-006, DRVR-007, INTD-015, INTD-008, INTD-012                              | Resolve driver/MCP scope, trust boundary, telemetry scoping, config loading, and Windows confidence                        | DRVR design decisions can run in parallel with INTD implementation; INTD-012 can land at any point but blocks final A2 completion                                                        |
| 2    | INTD-004, INTD-006, INTD-009, INTD-010, INTD-016, DRVR-001                    | Complete daemon runtime behaviours and the shared driver client                                                            | INTD watcher/interrupt/embedded/unregistered tracks can run independently once INTD-008 is available where needed; DRVR-001 can run against fake daemon fixtures while INTD items finish |
| 3    | DRVR-002, DRVR-008, RTAI-004, INTD-011                                        | Pin the editor-driver protocol, capability negotiation, mid-edit client envelope, and daemon-visible status/latency rollup | DRVR-002 and RTAI-004 can overlap after DRVR-001, but RTAI-004 must not merge before protocol/envelope compatibility is confirmed                                                        |
| 4    | RTAI-005, RTAI-007, RTAI-009, DRVR-003 if unpaused                            | Bring the editor mid-edit path online, mirror telemetry, and update architecture docs                                      | RTAI-007 can run once INTD-015 is in; RTAI-009 waits for the actual consumer state; DRVR-003/RTAI-005 require an explicit ADR-033 unpause or replacement editor-surface decision         |

**Parallelisation notes:** The safe parallel split is **daemon hardening**
(INTD-004/-006/-008/-009/-010/-015/-016), **driver contract**
(DRVR-006/-007/-001/-002/-008), and **RTAI consumer semantics**
(RTAI-004/-005/-007/-009). Do not parallelise work that changes the diagnostic
envelope or daemon error semantics after RTAI-004 starts; route those through
the Wave 0 contract first. If the release cut wants daemon-backed MCP only,
Waves 1-3 are sufficient; Wave 4's editor-surface items are a separate delivery
lane because ADR-033 currently keeps the VS Code extension archived.

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

### A7. Multi-Layer Protection Foundation — _pre-positioning for v1_

**Goal:** Land the architectural decisions and the smallest-possible foundation
pieces that make next-release multi-layer protection (MLP module) implementable
on day one. This is **NOT** the full multi-layer protection shipping — that's v1
(next-next release). This is the slice that prevents v1 implementation from
starting cold.

**Source artefacts:**

- Spec:
  [`plans/specs/2026-05-07-anvil-multilayer-protection-architecture.md`](./plans/specs/2026-05-07-anvil-multilayer-protection-architecture.md)
- Brainstorm:
  [`plans/brainstorms/2026-05-07-anvil-multilayer-protection-brainstorm.md`](./plans/brainstorms/2026-05-07-anvil-multilayer-protection-brainstorm.md)
- ADRs (Proposed):
  - [ADR-036 (rewritten)](./plans/decisions/036-daemon-scope-discovery-and-boundaries.md)
    — daemon scope, discovery, OS-boundary
  - [ADR-037](./plans/decisions/037-witness-chain-and-l4-policy.md) — witness
    chain + L4 policy
  - [ADR-038](./plans/decisions/038-hook-surface-and-noise-discipline.md) — hook
    surface + noise discipline (the Serena rule)
  - [ADR-039](./plans/decisions/039-baseline-policy-and-hard-pinned-classes.md)
    — baseline policy + hard-pinned classes
- CLI coherence spec:
  [`plans/specs/2026-05-07-cli-surface-coherence.md`](./plans/specs/2026-05-07-cli-surface-coherence.md)
- MLP module:
  [`plans/modules/multilayer-protection.aps.md`](./plans/modules/multilayer-protection.aps.md)
  (17 work items; almost all v1 / next-release)
- Future-session gaps:
  [`plans/brainstorms/2026-05-07-remaining-design-gaps.md`](./plans/brainstorms/2026-05-07-remaining-design-gaps.md)

**Slice for current release (in priority order):**

| Item                                                                                                                            | Cost                                       | Risk           | Why current-release                                                                           |
| ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ | -------------- | --------------------------------------------------------------------------------------------- |
| **A7.1** — Council review + Accepted promotion of ADRs 036 (rewrite), 037, 038, 039                                             | One council session + remediation          | None (no code) | Gates next-release implementation work; without it, v1 starts from "Proposed" docs            |
| **A7.2** — `anvil/project-id` UUID written by `anvil start` (MLP-001 minimal slice)                                             | ~50 LOC + tests in activation orchestrator | Low            | Lays foundation for v1 witness chain; lets `anvil/` start as tracked-dir convention           |
| **A7.3** — CLIC-001 exit code constants audit                                                                                   | Small refactor                             | Low            | Replaces magic numbers; pre-positions new exit codes (5/6/7/10) without breaking current uses |
| **A7.4** — CLIC-002 `--quiet` flag introduction                                                                                 | Small                                      | Low            | New flag; default off; doesn't change existing behaviour                                      |
| **A7.5** — CLIC-006 deprecation aliases (`mcp-config` → `mcp config`, `gate-config` → `gate config`, `hooks` → `hook`)          | Small                                      | Low            | Aliases keep working; deprecation message during one release                                  |
| **A7.6** — CLIC-010 help text consistency pass                                                                                  | Medium (CLI sweep)                         | Low            | Improves DX; no behaviour change                                                              |
| **A7.7** — ADR-038 noise-discipline audit of existing surfaces (`anvil status`, `anvil doctor`, `anvil intercept status`)       | Medium (CLI sweep)                         | Low            | Enforces the Serena rule retroactively                                                        |
| **A7.8** — DRVR forward-compat verification (DRVR-001/-002/-008 don't lock out future `info.json` discovery / `AgentTag` field) | Review only                                | None           | Coordination gate; ensures DRVR ships compatible with future MLP additions                    |

**Out of scope for current release** (explicitly v1, do NOT pull forward):

- Witness chain implementation (MLP-002)
- Pre-commit / pre-push hooks (MLP-003 / -004)
- `anvil baseline` command (MLP-007)
- L4 policy framework (MLP-006)
- Per-task fence isolation (MLP-014)
- CI action publishing (MLP-010)
- Server-side `validate_at_l4` fallback
- Full `anvil/policy.yml` parser

These are the v1 build sequence; pulling any forward expands current-release
scope materially.

**Recommendation: ACCEPT A7.1, A7.2 as MUST. Accept A7.3–A7.7 as SHOULD (land if
bandwidth allows). A7.8 is a review checkpoint, not new work.**

The MUSTs are tiny (council review + ~50 LOC). The SHOULDs are incremental
polish that don't expand scope. Current release stays a hardening / activation
slice while pre-positioning v1.

---

## NEXT RELEASE WINDOW (proposed) — _post-`v0.6.0-beta` daemon-working slate_

**Theme:** _Daemon working end-to-end._ `v0.6.0-beta` shipped the daemon +
driver substrate ("available when invoked"); this window flips it to "always-on,
in-tree, defensible." `anvil start` lands a real testable protection claim;
hooks fire deterministically; the witness chain records every commit; baseline
adoption works on existing repos; `anvil-run` wraps agent processes with
mechanical fence-on-fail.

**Target tag:** **`v0.7.0-beta`** when N1 + N2 + N3 land together. Narrower
cuts (e.g. MLP backbone only, no INTL) tag as `v0.6.x-beta`.

**Hard release gate — MLP-009 (protection-claim contract test suite).** Pinned
states for `unprotected | warming | pre-write-embedded | pre-write-daemon |
save-time-only | full | degraded-protection | cross-boundary-mixed |
multi-daemon-detected | path-uncertain` must all be reachable in fixtures and
rendered claims must match. **No MLP item flips Complete in `index.aps.md`
until that suite is green.**

**Source artefacts:**

- Spec:
  [`plans/specs/2026-05-07-anvil-multilayer-protection-architecture.md`](./plans/specs/2026-05-07-anvil-multilayer-protection-architecture.md)
- MLP module:
  [`plans/modules/multilayer-protection.aps.md`](./plans/modules/multilayer-protection.aps.md)
  (17 items, Proposed)
- INTL module:
  [`plans/modules/intercept-launcher.aps.md`](./plans/modules/intercept-launcher.aps.md)
  (9 items, Draft)
- ADRs (must be Accepted before any N1 code merges):
  [036 (rewritten)](./plans/decisions/036-daemon-scope-discovery-and-boundaries.md),
  [037](./plans/decisions/037-witness-chain-and-l4-policy.md),
  [038](./plans/decisions/038-hook-surface-and-noise-discipline.md),
  [039](./plans/decisions/039-baseline-policy-and-hard-pinned-classes.md)

---

### N1. Multi-Layer Protection v1 (MLP) — _the daemon-working backbone_

**Goal:** Ship the witness chain, hook surface, L4 policy, baseline,
multi-agent coordination, and rule distribution model that turn the daemon from
"binary that runs" into "background protection the user can trust." 17 items.

**New crates introduced (~6):** `anvil-witness`, `anvil-hook`, `anvil-l4`,
`anvil-config`, `anvil-baseline`, `anvil-attribution`.

**Dependency graph (top-down; siblings parallelisable):**

```text
[Wave 0 — gates, no code]
   ADR-036 / -037 / -038 / -039 promoted Proposed → Accepted (G1)
   ADR-038 noise-discipline audit of existing surfaces (G3)
   AIGUARD envelope contract test re-run against both backends (G4)
                            │
       ┌────────────────────┼────────────────────┐
       │                    │                    │
[Wave 1A — backbone]   [Wave 1B — config]   [Wave 1C — air-gap]
  MLP-001 project-id    MLP-011 multi-fmt    MLP-017 net-blocked
  MLP-002 witness chain MLP-013 hard-pinned   sandbox + tests
       │                MLP-012 rules_sha
       │                    │
       └────────┬───────────┘
                │
[Wave 2 — adoption + hooks (parallel within wave)]
   MLP-003 pre-commit hook   ←── MLP-001/-002
   MLP-007 anvil baseline    ←── MLP-001/-002
   MLP-006 L4 policy         ←── MLP-002, MLP-007 for cutoff_commit
   INTR-004 path-deny rule   ←── promoted from B1 → feeds MLP-013 metadata
                │
[Wave 3 — coordination + extensions]
   MLP-004 pre-push (L4 client)   ←── MLP-002/-003/-006
   MLP-005 post-commit / -merge / -rewrite ←── MLP-002/-003
   MLP-008 anvil hook bootstrap   ←── MLP-002/-003
   MLP-014 multi-session + per-task fence ←── MLP-002/-003 + INTL-003 schema
   MLP-015 anvil audit (L5)       ←── MLP-006
   MLP-016 L1 driver → kindling   ←── DRVR-002 + RTAI-007 (already shipped)
   MLP-010 eddacraft/anvil-action publish ←── MLP-006 + new publishing repo
                │
[Wave 4 — hard gate]
   MLP-009 protection-claim contract test suite  ←── MLP-002..-008 stable
```

**Lane / owner map (parallelisable across waves):**

| Lane                     | Items                                | Owner type            | Notes                                                                                       |
| ------------------------ | ------------------------------------ | --------------------- | ------------------------------------------------------------------------------------------- |
| **Witness backbone**     | MLP-001, MLP-002                     | Rust crate            | Gates everything downstream; flock + DAG verification + 80-parallel-hook test is highest-risk piece |
| **Config + rules_sha**   | MLP-011, MLP-012, MLP-013            | Rust crate            | Independent of backbone; can ship before                                                    |
| **Air-gap harness**      | MLP-017                              | Test infra            | Independent; sandbox at `tools/test-harness/network-blocked/`                               |
| **Hooks**                | MLP-003, MLP-005, MLP-008            | Rust + shell template | Three siblings after MLP-002 lands                                                          |
| **L4 + adoption**        | MLP-006, MLP-007                     | Rust crate            | MLP-007 (baseline) feeds `cutoff_commit` into MLP-006                                       |
| **Pre-push + audit**     | MLP-004, MLP-015                     | Rust + workflows      | After MLP-006 stable                                                                        |
| **Multi-session**        | MLP-014                              | `crates/anvil-intercept/` extension | Extends INTD-003 registry — **must coordinate `AgentTag` proto change with INTL-003** |
| **L1 + GH publish**      | MLP-016, MLP-010                     | TS + separate repo    | Parallel with everything; MLP-010 needs `eddacraft/anvil-action` publishing repo created    |
| **Hard gate (terminal)** | MLP-009                              | Rust + e2e            | Last; cannot start until -002..-008 merged                                                  |

**Anti-parallelism (hard sequencing — do NOT violate):**

1. **No `anvil-witness` consumer ships before MLP-002 contract is pinned.**
   The witness line schema cannot drift after the first hook merges.
2. **MLP-013 hard-pinned class enforcement merges with the config parser
   (MLP-011), not after.** A loophole window is unacceptable for security-class
   rules.
3. **MLP-009 last.** Pinning protection-claim states earlier locks in
   unfinished states; the suite must run after every state is reachable.
4. **`AgentTag` proto change is a single PR shared between MLP-014 and
   INTL-003.** Do not let INTL-003 ship a session shape MLP-014 has to break.

**Out-of-scope (explicit; do NOT pull forward):**

- GitHub App / GitLab / Bitbucket native integrations (vNext)
- Pre-receive hook script for self-hosted git (vNext)
- Anvil cloud sidecar / hosted services (vNext, opt-in only)
- `prepare-commit-msg` / `commit-msg` hooks (v1.5)
- macOS App Sandbox detection observability (DLIFE-010, v1.5)
- Cross-Windows ↔ WSL surface bridging (vNext, separate ADR)
- Migration tooling for `project_uuid` changes (per direction)

**Adversarial risk: First-witness lottery.** Baseline scan on a 100k-LOC repo
with thousands of pre-existing findings must not stall the CLI for >60s, and
the resulting `anvil/baseline.json` must not bloat git operations on adoption.
**Mitigation:** MLP-007's bounded scan + async continuation budget plus
content-addressed archive naming so `anvil/witness/archive/` doesn't churn
`git status`.

**Recommendation: PICK as the headline of the next window.**

---

### N2. Intercept Launcher v1 (INTL) — _wrapped-launch ingress_

**Goal:** `anvil-run`-wrapped agent launches with daemon-coordinated session
lifecycle and mechanical fence-on-fail. Sessions register before children
spawn; PGIDs / Job Objects let the daemon target interrupts; cleanup is
drop-guard guaranteed. 9 items.

**New crate:** `crates/anvil-run/` (binary).

**Dependency graph:**

```text
INTL-001 launcher binary scaffold
   │
   ├── INTL-006 shell integration (zsh / bash)            ← parallel from -001
   │
   └── INTL-002 daemon connectivity + fence check
          │
          └── INTL-003 session registration               ← carries AgentTag (coord with MLP-014)
                 │
                 ├── INTL-004 process-group / Job Object launch
                 │      │
                 │      └── INTL-005 cleanup on exit (drop guard)
                 │
                 └── INTL-007 hook side-channel registration  ← Claude Code PreToolUse path

INTL-008 cross-platform parity tests   ← wave-end gate, wraps -004/-005
INTL-009 documentation + manpage       ← parallel with everything
```

**Parallelisation:**

- INTL-001 → INTL-006 (shell wrapper) runs in parallel with INTL-002 →
  INTL-005 (binary daemon path); shell script is independent.
- INTL-007 only needs the registration **contract** from INTL-003 — prototype
  against fakes while INTL-004 lands.
- INTL-008 is a wave-end checkpoint, not a discrete work item — gate on it
  before promoting INTL.

**Cross-dependency with N1:** MLP-014 (multi-session + per-task fence) extends
the session registry that INTL-003 talks to. The `AgentTag` schema is shared.
**Land it as one proto PR** before either consumer commits its branch.

**Adversarial risk: Windows Job Object semantics ≠ POSIX PGIDs.** INTL-004
alone is platform-bisected work; the Job Object name handshake (launcher →
daemon) is the failure-prone interface. Pin a contract test that round-trips
through a named Job Object on the Windows CI matrix before INTL-005 merges.

**Recommendation: PICK alongside N1.** INTL without MLP is half-protected; MLP
without INTL leaves `anvil-run` as Tier B debt and gives the launcher's
session-registry consumers no real ingress.

---

### N3. Carry-forward gates (must close before headline ships)

These are not new work — they are pre-positioning A7 defined for the current
release. They **must be Accepted / merged before any N1 item lands.**

| ID  | Gate                                                                                  | Source                            | Why blocking                                                                            |
| --- | ------------------------------------------------------------------------------------- | --------------------------------- | --------------------------------------------------------------------------------------- |
| G1  | ADR-036 (rewritten), ADR-037, ADR-038, ADR-039 promoted Proposed → Accepted           | A7.1 carry-forward                | MLP code cannot land against Proposed ADRs                                              |
| G2  | `anvil/project-id` UUID written by `anvil start` (MLP-001 minimal slice)              | A7.2 carry-forward                | Witness chain (MLP-002) writes against this; `anvil/` becomes tracked-dir convention    |
| G3  | ADR-038 noise-discipline audit of existing surfaces                                   | A7.7 carry-forward                | New hook output cannot pass noise discipline if existing surfaces violate it            |
| G4  | AIGUARD envelope contract test re-run against both backends                           | A2 cross-cutting glue             | Witness lines carry diagnostic envelopes; rerun as N1 gate                              |
| G5  | INTR-004 (path-deny rule) promoted from B1 → Wave 2                                   | B1 absorption                     | Rule registration metadata feeds MLP-013 hard-pinned class                              |
| G6  | DRVR forward-compat verification (DRVR-001/-002/-008 don't lock out `info.json` / `AgentTag`) | A7.8 carry-forward       | Coordination gate; ensures DRVR ships compatible with MLP-014                           |

---

### N4. Documentation + comms (parallel with code)

| Doc                                            | Trigger                  | Notes                                                                                    |
| ---------------------------------------------- | ------------------------ | ---------------------------------------------------------------------------------------- |
| `docs/runbooks/anvil-baseline-adoption.md`     | After MLP-007 functional | How an existing repo adopts MLP without a wash of warnings                               |
| `docs/runbooks/anvil-air-gapped.md`            | With MLP-017             | Declares the air-gapped guarantee and how it's tested                                    |
| `docs/runbooks/anvil-witness-chain.md`         | After MLP-002 contract pinned | Operator-facing description of `anvil/witnessed.ndjson`, manifest, archive, recovery |
| `docs/runbooks/anvil-hooks-integration.md`     | After MLP-003/-008 stable | Husky / lefthook / pre-commit-framework / no-framework integration matrix                |
| Migration note for `v0.6.0-beta` → `v0.7.0-beta` users | Tag candidate    | `anvil/` directory becomes tracked; `.gitattributes` ships `merge=union -text`           |
| INTL `man anvil-run`                           | INTL-009                 | Standard manpage + `anvil-run --help` parity                                             |

---

### Wave summary (cross-lane)

| Wave | Lanes active in parallel                                                                                                                                           | Gate to next wave                       |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------- |
| 0    | G1 (ADRs Accepted), G3 (noise audit), G6 (DRVR fwd-compat), N4 doc skeletons                                                                                       | All ADRs Accepted                       |
| 1    | Witness backbone (MLP-001/-002), Config (MLP-011/-012/-013), Air-gap harness (MLP-017), INTL scaffold (INTL-001/-006)                                              | MLP-002 contract pinned                 |
| 2    | Hooks (MLP-003/-005/-008), Baseline (MLP-007), L4 policy (MLP-006), INTR-004 promotion, INTL daemon path (INTL-002/-003 with shared AgentTag)                       | Hooks deterministic on green path       |
| 3    | Pre-push (MLP-004), Multi-session (MLP-014), Audit (MLP-015), L1 → kindling (MLP-016), GH Action (MLP-010), INTL spawn + cleanup (INTL-004/-005), Hook side-channel (INTL-007), runbooks | All consumer-facing items code-complete |
| 4    | MLP-009 protection-claim contract suite, INTL-008/-009 cross-platform tests + docs, runbooks finalised, migration note                                              | Suite green; tag                        |

---

### Adversarial risk: window scope

This window is **larger than `v0.6.0-beta`** (17 + 9 = 26 work items vs
`v0.6.0-beta`'s ~21). The hard-gate pattern (MLP-009) prevents
partial-completion drift, but the **witness chain primitive (MLP-002) is the
single point of failure** — every downstream lane reads or writes against it.

**Mitigation:** Spike MLP-002 first as a standalone PR (witness chain crate +
flock + DAG verification + 80-parallel-hook test) before any hook lane starts.
If the spike reveals correctness or performance issues, the rest of the window
remains decomposable into a `v0.6.x-beta` patch shape (MLP-001 + MLP-011..-013
- INTL scaffold) without forcing a full re-plan.

---

## TIER B — Queued (next slice candidates)

### B1. Intercept Loop v0 — _absorbed into next-release window_

> **Status (2026-05-09):** B1 is no longer a queued candidate; INTL has been
> promoted to **N2** in the next-release window above, and INTR-004 has been
> promoted to **N3 G5**. Remaining INTD items (none — module is 16/16
> Complete) are not part of this absorption. This entry is preserved as a
> back-pointer for anyone arriving from older planning notes.

**Adversarial risk (preserved):** Cross-platform Windows parity (INTD-012 —
shipped). Job Object semantics ≠ PGIDs. INTL-004 alone is platform-bisected
work that does not show up in the demo.

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
7. **`AgentTag` proto change** — required by N1+N2 (next-release window).
   Single shared PR to `crates/anvil-intercept-proto/` covering the new session
   composite key; both MLP-014 and INTL-003 consume it. Land before either
   begins implementation against its own branch.
8. **`eddacraft/anvil-action` publishing repo** — required by N1 (MLP-010).
   Separate GitHub repo + release pipeline for the marketplace action. Create
   skeleton in Wave 0 so MLP-010 has somewhere to publish in Wave 3.
9. **`anvil/` directory tracking convention** — required by N1 + downstream
   git ergonomics. `.gitattributes` ships `merge=union -text` for
   `anvil/witnessed.ndjson` and `anvil/witness/manifest/chain.ndjson`. Pin
   in Wave 0 so adoption (MLP-007) doesn't have to retro-fit it.
10. **ADR-036 / -037 / -038 / -039 promotion** — required by N1 (G1). Council
    review session pre-Wave 0; remediation lives in the same PR as the
    promotion. No N1 code lands against Proposed ADRs.

---

## Suggested first-pick combinations

| Combo                                | Slices                      | Open scope                              | Posture                                                                                               |
| ------------------------------------ | --------------------------- | --------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| **Patch-shaped tag**                 | Carry-over only             | V050F 5 open                            | Tags as `v0.5.2-beta`. Smallest credible cut.                                                         |
| **Wow-start minimum**                | A1                          | LAUNCH 1 open                           | Just the activation council outcome. Tags as `v0.5.2-beta` if scoped tight, `v0.6.0-beta` if broader. |
| **Wow-start + daemon (recommended)** | A1 + A2 + carry-over        | LAUNCH 1 + A2 remaining + V050F 5       | The headline + the literal-protection substrate. Tags as `v0.6.0-beta`. Highest confidence.           |
| **Wow-start + parity**               | A1 + A2 + A3 + carry-over   | A1/A2/carry-over + RMCPF scope          | Adds full Rust MCP parity. Strong narrative, real contention with A1.                                 |
| **Wow-start + hygiene**              | A1 + A4 + A5                | LAUNCH 1 + release/language tails       | Activation + finishes the `v0.5.0-beta` tails. Skips daemon graduation — A2 follows in next window.   |
| **Founder-pitch slate**              | A1 + A2 + A6                | LAUNCH 1 + A2 remaining + dashboard MVP | Activation + daemon + dashboard. Largest persona expansion. Highest A6/A1 bandwidth contention.       |
| **Full slate**                       | A1 + A2 + A3 + A4 + A5 + A6 | All Tier A remaining                    | Most ambitious; only realistic if A1 ships clean and bandwidth holds.                                 |
| **Daemon-working (next window)**     | N1 + N2 + N3 + N4           | MLP 17 + INTL 9 + 6 gates + runbooks    | The full daemon-working slate post-`v0.6.0-beta`. Tags as `v0.7.0-beta`. Largest window since `v0.5.0-beta`; MLP-009 hard gate. |
| **Daemon-backbone (narrower next)**  | N1 Waves 0–2 only + N3      | MLP-001/-002/-003/-006/-007/-011/-012/-013/-017 + INTL-001/-006 + gates | Witness + hooks + baseline + config; defers pre-push, multi-session, audit, GH Action. Tags as `v0.6.x-beta`. Recovery shape if MLP-002 spike reveals risk. |

The councils' lesson from `v0.5.0-beta` plus the 2026-05-03 activation council:
**A1 + A2 + carry-over** is the highest-confidence cut for the next tag. A1
alone tags `v0.5.2-beta`-shaped; A1+A2 together earns `v0.6.0-beta`. A3 (RMCPF)
and A6 (Dashboard) are the high-leverage adds; both are the candidates most
likely to slip if A1 takes longer than estimated.

**Post-`v0.6.0-beta`:** the **Daemon-working** combo (N1+N2+N3+N4) is the
proposed slate; **Daemon-backbone** is the smaller fall-back if the witness
spike (MLP-002) shows correctness or performance risk. Either choice keeps
A4 (release-engineering tail) and A5 (language-credibility tail) available as
parallel hygiene picks if bandwidth holds.

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
