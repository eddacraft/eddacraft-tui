# Anvil Release Plan

**Last updated:** 2026-04-28 (Graph v2 + Rust MCP launch-shim planning update)

> Companion: [ROADMAP.md](./ROADMAP.md) — themes, big bets, horizons.

---

## 🔒 CURRENT RELEASE — Locked 2026-04-26

The slate for the current release is **A1 + A2 + A3 + A4** — the realistic
ceiling per council consensus, with the A1 MCP path narrowed to a Rust launch
shim rather than a full MCP server port. Total ≈ 37 work items across 4 coherent
slices.

| Slice                             | Goal                                                                            | Items |
| --------------------------------- | ------------------------------------------------------------------------------- | ----- |
| **A1** RTAI Spike Slice           | Real-time AI validation that fires before save through the Rust MCP launch path | ~23   |
| **A2** AIGUARD                    | `anvil gate --profile ai` + stable JSON diagnostic envelope                     | 4     |
| **A3** Release Engineering        | GHOOK + ATTRIB + SCAN smallest viable cut                                       | 7     |
| **A4** Language Credibility Floor | LANGTS audit + OPSUP slice 1 (check-ID registry) + SURFENV                      | 3     |

**Out of this release:** A5 (Dashboard MVP), RMCPF (full Rust MCP parity port),
GV2/GCTX (Graph v2 foundation and graph context delivery), and full DRVR editor
/ MCP driver cutover. The current release ships the Rust MCP launch shim only.
All other Tier B and Tier C items remain queued/parked.

### Required prerequisites (cross-cutting glue)

These are **not slices** — they're glue items that the locked candidates need.
Some now have work-item homes after the 2026-04-28 MCP-path update; any
remaining unclaimed glue must be claimed before the slice it supports starts.

- [ ] **Reasoning-pattern rule** in `crates/anvil-checks` — minimum-viable
      AI-pattern check (e.g. AI-001 appeal-to-authority). Required by **A1**;
      tracked as INTR-008. Without it, the demo headline is "secret detection
      mid-edit."
- [ ] **Single latency rubric ADR** — INTD-014 / RTAI / RMCP must cite one
      rubric rather than inventing per-surface numbers. Required by **A1**.
- [ ] **Demo runbook** — `anvil init` → `anvil mcp install` → open Cursor /
      Claude Code → paste known-bad pattern → Anvil flags before save. LAUNCH
      polishes save-time, RTAI defines validation semantics, RMCP owns the Rust
      MCP launch path; nobody yet owns the user journey. Required by **A1**.
- [ ] **Rust MCP launch shim for Cursor / Claude Code** — RCLI3-016 already
      writes config pointing at `anvil mcp serve --stdio`; **RMCP** makes that
      command real in Rust for the A1 path. Do not port the whole TS MCP server
      in this release.
- [ ] **Diagnostic envelope coordination** — AIGUARD-002 ↔ RMCP-006 ↔ RTAI-006 /
      RTAI-008 ↔ INTD-013 must agree for the launch path. DRVR-002 consumes the
      same shape later. Decide envelope shape _before_ AIGUARD-001 starts.
      Required by **A2**.
- [ ] **ADR-027 / ADR-028 / ADR-029 acceptance** — currently Proposed; Council
      D: "their continued Proposed status is procedural debt, not technical
      debt. Resolve in a single review session." Required by **A4**.
- [ ] **Anchor re-scoring process owner** — currently no permanent owner named
      in the L&C spec. Required by **A4**.

### Adversarial risks for this release

1. **RTAI latency budget is unverified.** RTAI-001 spike measures truth first;
   if real number is 250ms, demo "works" but does not _wow_. Mitigation must be
   planned before the rest of A1 commits.
2. **Envelope coordination drift.** If A2 ships its diagnostic envelope before
   A1 locks down RTAI/INTD/RMCP shapes, consumers branch.
3. **Cross-cutting expansion.** A3 wants to grow (downstream-port, WalkParallel
   spike). Be willing to say no.
4. **A4 looks too small.** Counter-frame as "Phase 0 floor," not "theme
   release."

### What ships next (after this release)

Most likely successors, in order of council confidence:

- **RMCPF** Rust MCP Full Port — full parity with the existing TS MCP server,
  after RMCP proves the launch path
- **A5** Dashboard MVP — once RTAI ships clean
- **GV2** Graph v2 Foundation — shared graph substrate for future enforcement,
  provenance, and graph-context delivery
- **B1** Intercept Loop v0 — wrapped-launch v2 narrative
- **B4** Beta Migration Hardening — EAMIG/EATEST/TINT regression-prevention

The Enterprise Readiness constellation (B2+B3) promotes when the first prospect
surfaces.

---

## How to use this document

This is a **menu, not a schedule.** Each entry is a coherent release-slice
candidate with a defined scope, prerequisites, and adversarial risk. Pick
candidates for the **current release**; everything else queues.

The release-strategy memory is **ship-now-with-pre-flight, no time estimates,
sequence via cherry-pick.** This document is built for that posture: candidates
are independently shippable; you choose which N to commit to, and the rest stay
parked.

### Tiers

- **Tier A** — current-release candidate. Ready, deps real, value clear. Pick
  from this list.
- **Tier B** — queued. Coherent slice, but waits for a prerequisite or signal.
  Promote to Tier A when ready.
- **Tier C** — parked. Waiting for a demand pull or a prior horizon.

### Reading a candidate

Each candidate gives you:

- **Goal** — what the slice ships, in one sentence
- **Modules / work items** — the exact subset to commit
- **Prerequisites** — what must land before this can start
- **Out-of-scope** — what protects the slice from creep
- **Adversarial risk** — the most likely reason it slips
- **Recommendation** — pick / consider / defer

---

## TIER A — Current-release candidates

### A1. RTAI Spike Slice — _the launch-blocker_

**Goal:** Real-time AI-output validation that fires before save. The 60-second
demo: run `anvil mcp install`, open Cursor / Claude Code with
`anvil mcp serve --stdio` attached, ask for a confident rewrite, watch Anvil
warn or block before the write hits disk.

**Modules / work items (~23 items):**

- **INTD subset** (6 of 16): INTD-001, INTD-002, INTD-003, INTD-005, INTD-013,
  INTD-014
- **INTR subset** (4 of 8): INTR-001 (rule trait), INTR-002 (secret detection
  wrapper), INTR-006 (config), INTR-008 (launch reasoning-pattern wrapper)
- **RMCP subset** (8 of 8): RMCP-001..RMCP-008 — Rust MCP launch shim only:
  stdio server, minimal pre-write validation tool, daemon-preferred validation
  adapter, canonical diagnostics, Cursor / Claude Code install verification,
  smoke tests
- **DRVR coordination**: DRVR remains the full driver-framework track, but A1 no
  longer depends on building a TS `DriverClient` solely to bridge MCP back into
  Rust. DRVR-004-lite is replaced by RMCP for the launch demo.
- **RTAI subset** (5 of 9): RTAI-001 (Phase-0 spike), RTAI-002, RTAI-003,
  RTAI-006, RTAI-008 — MCP/RMCP path first (deterministic demo target). RTAI-004
  is deferred with the TS `DriverClient`/editor-driver path.

**Prerequisites:**

- RTAI-001 spike completed and latency budget validated _before_ any other RTAI
  item commits
- INTD remains the primary A1 path for this release; treat RMCP's
  embedded/shared Rust validation path as contingency-only, activated only by
  explicit release-owner decision if INTD threatens the release
- RMCP-001 scope lock recorded before implementation starts
- ADR-031 latency rubric referenced by INTD-014 / RTAI / RMCP
- Generated Cursor / Claude Code config verified to launch
  `anvil mcp serve --stdio`
- Before tagging, record whether RMCP shipped daemon-backed or via the embedded
  validation fallback

**Out-of-scope (protect the slice):**

- INTD-007/-008/-010/-011/-016 (fence persistence, full config, unregistered
  handling, diagnostics, DoS budgets)
- DRVR-003 (full VSCode cutover) — RTAI-005 mid-edit can demo without it
- DRVR-004 full MCP cutover and TS `DriverClient` bridge — replaced for A1 by
  RMCP; full parity moves to RMCPF in the next release
- DRVR-007/-008 (driver trust + non-VSCode LSP) — defer to v1.1
- RTAI-004, RTAI-005, RTAI-007, RTAI-009 — TS `DriverClient`, editor-specific
  path, telemetry mirror, and docs
- Intercept Launcher (INTL) entirely — wrapped-launch is v2 narrative
- TUIDASH, RCLI2, and RCLI3 except the already-pulled-forward MCP config/install
  items RCLI3-016/RCLI3-016b

**Adversarial risk:** **Latency budget is fiction and INTD may slip.** Sub-50ms
daemon-side / sub-100ms total numbers extrapolate from in-process kernel
benchmarks. Rust MCP stdio framing may still break "feels real-time." RTAI-001
measures truth; if real number is 250ms, demo "works" but does not _wow_. If
INTD becomes the release bottleneck, RMCP can fall back to the embedded/shared
Rust validation path, but only after an explicit release-owner decision.

**Recommendation: PICK. This is the launch.**

---

### A2. AI Guardrail Profile (AIGUARD)

**Goal:** Ship `anvil gate --profile ai` with a stable JSON diagnostic envelope.
The shape AI tools consume when they invoke Anvil — launch-aligned with RTAI's
"trust-in-AI-generated-code" thesis.

**Modules / work items (4 items):**

- AIGUARD-001 — profile definition in `crates/anvil-cli/src/commands/gate.rs`
- AIGUARD-002 — stable JSON diagnostic schema in
  `crates/anvil-kernel-types/src/diagnostics.rs` _(strategic piece)_
- AIGUARD-003 — `anvil gate --profile ai` CLI flag wiring
- AIGUARD-004 — docs guide for AI workflows

**Prerequisites:**

- Coordinate AIGUARD-002 envelope shape with **RMCP-006** (Rust MCP launch
  response), **RTAI-006/RTAI-008** (pre-write semantics and error contract), and
  **INTD-013** (telemetry control envelope). DRVR-002 consumes the same shape
  later. Whichever lands first publishes canonical shape; others reference it.

**Out-of-scope:**

- IDE integration (separate)
- Auto-fix
- Live IDE feedback

**Adversarial risk:** Envelope coordination drift — if AIGUARD-002 ships before
RMCP-006 / RTAI-006 / RTAI-008 / INTD-013 lock down their shapes, the envelope
diverges and consumers branch. Coordinate the schema decision _before_
AIGUARD-001 starts.

**Recommendation: PICK or CONSIDER. Small (4 items), high RTAI/RMCP-alignment,
low contention. Strong "consider" if RMCP-006 is on the slice anyway.**

---

### A3. Release Engineering — _Council E's "smallest viable cut"_

**Goal:** Launch hygiene — git hooks, attribution pipeline, scan performance —
without expanding product surface. None depends on RTAI/INTD/DRVR.

**Modules / work items (7 items):**

- **GHOOK-001** — Git 2.54 hook policy
- **ATTRIB-001/002/003** — attribution pipeline v3 core (cargo-about, deny.toml,
  kit at `tools/starters/acknowledgements/`)
- **SCAN-001/002/003** — scan performance: parallel rollout, ReDoS guard, bound
  rayon thread count

**Prerequisites:**

- None outside the slice
- SCAN-003 (bound rayon thread count) must coordinate env-var name + default
  with whoever owns RTAI's first-run UX

**Out-of-scope:**

- ATTRIB-009/010/011 (downstream port + public mirror)
- SCAN-004/005 (provenance + WalkParallel spike)
- TINT entirely; EAMIG/EATEST entirely

**Adversarial risk:** Cross-cutting work always wants to expand. Be willing to
say "no" to ATTRIB's downstream-port milestone, to SCAN's WalkParallel spike,
etc. The hype phase is funding for the launch-blocker, not engineering polish.

**Recommendation: PICK. Cleanest cross-cutting slice in the audit. Independent
of RTAI bandwidth. Ships releasable polish.**

---

### A4. Language Credibility Floor — _Council D's Candidate 1_

**Goal:** Three small independent deliverables that produce tangible artefacts
even if the rest of the Language & Coverage theme stalls. Pure governance /
operational floor.

**Modules / work items (3 items):**

- **LANGTS** — full TS audit + checklist artefact (re-usable governance asset)
- **OPSUP slice 1** — check-ID registry replacing the hardcoded
  `AVAILABLE_CHECKS` array in `crates/anvil-cli/src/commands/gate.rs`
- **SURFENV** — `.env` file secret-scan promotion to T1 (cheapest "Anvil saved
  my key" demo)

**Prerequisites:**

- ADR-027 / ADR-028 / ADR-029 accepted at minimum bar (Council D recommends
  accepting all three now — they are blocking nothing real)

**Out-of-scope:**

- Rust anchor (RSTLAN), Python anchor (PYLAN)
- All Track 4 packs (PACKPUL, PACKLLM, etc.)
- MDGOV
- All surfaces beyond `.env` (SURFSQL, SURFGHA, SURFDOCK, SURFSH)

**Adversarial risk:** Looks too small for a "theme" release. Counter-narrative:
"Phase 0" or "foundation slice" framing makes it the credible-by-design floor —
we shipped the audit and the operational floor; the rest queues in priority
order against demand and RTAI bandwidth.

**Recommendation: PICK. Three small deliverables. Zero RTAI contention. Each
independent.**

---

### A5. Dashboard MVP — "Team-Lead Glance"

**Goal:** A team-lead opening `localhost:3000/dashboard` and seeing **last gate
run, current warnings ranked by severity, recent activity** without learning CLI
commands. Serves the buyer persona that funds the tool.

**Modules / work items (~12 items):**

- **DASH-001..006, DASH-008** (skip DASH-007 command palette, scope DASH-004
  charts to sparkline only)
- **DASHCORE-001** (overview metric cards), **DASHCORE-003** (activity feed),
  **DASHCORE-006** (warning list), **DASHCORE-007** (warning detail panel)
- **NEW work item:** `anvil export` — writes canonical
  `.anvil/{warnings,gates,provenance,config}.json` from latest run state.
  **Critical missing bridge** between CLI and dashboard.
- _Optional add:_ DASHOPS-005 (config viewer), DASHOPS-006 (diagnostics)

**Prerequisites:**

- Pin DASH-005 to **today's CLI `--json` shapes**, not a future SCHEMA contract.
  Ship-now over governance.
- Add the `anvil export` CLI work item (1 task, owned by anvil-cli)
- Decide deployment model: local `nx dev website` only for v1 — no auth, no
  multi-user (matches D-DASH-001)

**Out-of-scope:**

- Architecture graph (DASHARCH-003, low confidence)
- Drift comparison views (DASHARCH-005/006)
- AI builder (DASHAI all)
- Suppression trends (DASHARCH-008)
- Plan approval workflows
- Audit user/AI-tool breakdowns (DASHOPS-002/003)
- Real-time SSE

**Adversarial risk:** "Why would a team lead use this instead of just looking at
a Slack notification or `anvil check` in CI logs?" Honest answer: only if the
warning list with file/line + severity grouping is genuinely faster to triage
than scrolling CI output. **Smallest credible demo is therefore DASHCORE-006 +
DASHCORE-007 alone (warning list + detail panel, nothing else).** If those don't
feel better than CI logs, the rest of the dashboard won't save it. Build that
first, demo to one platform-engineer external user before committing to the full
Tier A.

**Recommendation: CONSIDER. Largest slice on the list (~12 items + 1 CLI item).
Ships the team-lead persona narrative. Defer entirely if RTAI takes all
bandwidth — Candidate A1 wins contention.**

---

## TIER B — Queued (next slice candidates)

### B1. Intercept Loop v0 — _the wrapped-launch v2 narrative_

**Goal:** `anvil-run`-wrapped agent launches with mechanical fence-on-fail
enforcement.

**Modules:** INTD (full minus -009/-012/-016), INTL (all 9 items), INTR-004
(path-deny). ~24 items.

**Why queued:** Coherent product story but **not the launch-blocker**. Promote
after RTAI Spike has demonstrated traction.

**Adversarial risk:** Cross-platform Windows parity (INTD-012). Job Object
semantics ≠ PGIDs; less-exercised test surface. INTL-004 alone is two weeks of
platform work that does not show up in the demo.

---

### B2. Enterprise Readiness Foundation

**Goal:** Multi-repo / fleet / enterprise deployment — the constellation that
answers "how does this deploy in front of N repos for an org-tier customer?"

**Modules (foundation cut):**

- **GATE** (3 items) — gateway topology + enforcement contract + observability
- **POLFED** (8 items) — multi-repo federation workflow over OPAE bundles
- **ORGHIER** (7 items) — multi-level policy hierarchy
- **POLLC** (7 items) — lifecycle / canary / grace periods

**Why queued (now Tier B):** Enterprise readiness becoming important soon.
Promotion gate: first enterprise prospect or design-partner request, OR internal
decision to ship Anvil's own deployment topology as reference.

**Sequence:** GATE + POLFED foundation first; ORGHIER + POLLC as the
multi-tenant layer. COMPLY/CEWS/TRUST (auditor surface) follows in B3.

---

### B3. Compliance & Trust Surface — _enterprise auditor cut_

**Goal:** SOC 2 / ISO 27001 / NIST framework support, evidence workspace, public
trust artefacts.

**Modules:**

- **COMPLY** (8 items) — framework registry, policy-to-control mapper, posture
  scoring, report generator, historical posture
- **CEWS** (4 items) — control-evidence model, ingestion, workspace views,
  export packs (after COMPLY prerequisites land)
- **TRUST** (3 items) — trust artifact model, publishing pipeline, freshness/
  ownership rules

**Why queued:** Sequenced after B2 foundation. CEWS depends on COMPLY's evidence
collector; do not start until COMPLY-001..004 are on the slice.

---

### B4. Beta Migration Hardening

**Goal:** Regression-prevention for v0.3.x findings — early-access migration

- tests + integration surface.

**Modules:**

- **EAMIG** (50 items, Ready, Rust-aligned)
- **EATEST** (38 items, Ready, Rust-aligned)
- **TINT** (15 items, flip Draft → Ready — TFIX/RCLI/KERN deps now
  archived-Complete)

**Smallest viable cut:** All EAMIG **High** priority items + all EATEST **High**
priority items + TINT subprocess contract tests. Walk down by Priority, not by
Phase. Defer Low-confidence and Low-priority to a third slice.

**Why queued:** Pre-launch attention should not split across 100+ items of
post-rehearsal cleanup. Promote after RTAI ships.

---

### B5. Phase 1 Language Spec-Faithful — _Council D's Candidate 2_

**Goal:** The MVP named in `2026-04-08-language-and-coverage-design.md` §9 —
"Anvil governs four file shapes" pitch deck slide.

**Modules:** LANGTS + SURFSQL T2 + PACKPUL + PACKLLM (TS substrate, warn-only)

- OPSUP slices 1 & 2 (check-ID registry + drift schema versioning +
  `anvil drift migrate`).

**Prerequisites:**

- ADR-027 / ADR-028 / ADR-029 accepted
- OPSUP slices 1 & 2 land first
- LANGTS audit publishes the T3 checklist
- Anvil's own repo passes each at warn level (dogfood gate)

**Adversarial risk:** **High.** Five modules pulled live in parallel with RTAI
launch work is a real attention crisis. PACKLLM's PII heuristics are an FP
minefield — even warn-only, a noisy first run on a prospect's repo would damage
credibility more than not shipping it. Recommend ordering: PACKPUL → SURFSQL →
PACKLLM (TS) only after RTAI is past its own launch validation.

---

### B6. Rust CLI Tier 2 — _re-audit before commit_

**Goal:** Extend RCLI parity surface — interactive-mode polish, OPAE-blocked
items.

**Caveat:** Half the listed items already exist in
`crates/anvil-cli/src/ commands/` (check, validate, drift, gate_config,
policy.rs). **Re-audit RCLI2 against current crate state before promoting to
Ready** — several work items may already be substantially done.

**Genuinely outstanding:** pr-comment, exception, policy-debug, policy-watch.
The OPAE-blocked subset (RCLI2-005..-008) is **Tier C** until OPAE moves.

---

### B7. Schema Contracts (SCHEMA) and Test Quality follow-on

**Goal:** TS↔Rust parity governance + integration/external-services testing.

- **SCHEMA** (6 items) — TS↔Rust contract parity; activate when the parity
  surface starts churning.
- **TEXT** (test-external-services, 14 items) — external service contract tests;
  not on launch critical path.
- **TINT** — covered in B4.

---

## TIER C — Parked (waiting for demand pull)

These do not compete for current-release attention. Keep in `plans/modules/` for
cataloguing; promote on signal.

| Module                                                                                          | Why parked                                                                                                                                                                             |
| ----------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **DASHAI** (dashboard-ai-builder)                                                               | Wave 4 of dashboard. Coordinate with TUIDASH json-render schema post-launch.                                                                                                           |
| **DASHARCH** (dashboard-architecture-views)                                                     | Demote Ready → Draft pending real schema source from `crates/anvil-architecture` + drift snapshot format.                                                                              |
| **DASHOPS** remaining                                                                           | Plan/role/AI-tool views are spec-orphan today.                                                                                                                                         |
| **OBS** (observability-foundation)                                                              | Park, rescope post-launch against `apps/anvil-api` (hosted-product surface, not local CLI).                                                                                            |
| **OPAE** (opa-enhancements)                                                                     | 36 tasks is a programme. Only policy-library + bundle inheritance pieces are launch-relevant; defer until a "policy library beats gate" slice (post-RTAI v0.5+).                       |
| **CPACKS** (compliance-policy-packs)                                                            | Shippable as ecosystem content after OPAE library + POLVAL. Compliance-pack effort scales by framework count.                                                                          |
| **AGOV** (agent-governance-patterns)                                                            | Signal-producer module for CPACKS/MDGOV. Promote when CPACKS POLVAL prep lands. AGOV-002 → CPACKS migration pending.                                                                   |
| **AIGUARD**                                                                                     | If not picked for Tier A, parks here as "small but coherent."                                                                                                                          |
| **OPAG** (opa-agent-orchestration)                                                              | Orchestration on a policy stack that does not exist yet. Park until OPAE library lands.                                                                                                |
| **EVAL** (eval-harness-integration)                                                             | Adapter contract is small, useful for RTAI regression once RTAI ships; revisit post-launch.                                                                                            |
| **CPOL** (contextual-policy-assertions)                                                         | Isolated, complements OPAE; small scope (3 tasks) — Tier B/C boundary.                                                                                                                 |
| **IORISK** (io-risk-controls)                                                                   | Closest to RTAI's input/output validation theme. Could enrich RTAI as a 1–2 task addition, but **default recommendation: do not include** in launch slice (dilutes "wow" with config). |
| **ATC** (adversarial-testing-catalog)                                                           | Useful when there is a model under test; pair with PATT as v0.6 safety pack.                                                                                                           |
| **PATT** (prompt-attack-regression-packs)                                                       | Pair with ATC.                                                                                                                                                                         |
| **POLVAL** (policy-pack-validation)                                                             | Necessary precondition for any pack work; small scope. Promote when packs activate.                                                                                                    |
| **ARCHCFG** (architecture-config-validation)                                                    | Could absorb into `crates/anvil-architecture` as a tier-2 item.                                                                                                                        |
| **TUIDASH** (tui-dashboard-render)                                                              | Demote Ready → Draft pending DASHAI catalogue resolution and schema source pin.                                                                                                        |
| **RCLI3** (rust-cli-tier3)                                                                      | Genuinely useful for parity; pure historical-contract work. Frame as "post-launch parity."                                                                                             |
| **RSTLAN, PYLAN**                                                                               | Heavy anchors. Self-dogfood compelling, not launch-blocking.                                                                                                                           |
| **LANGTAIL, PACKTOK**                                                                           | Tier D in Council D's classification — defer until breadth becomes a sales blocker (LANGTAIL) or RSTLAN moves (PACKTOK).                                                               |
| **MDGOV** (markdown-governance)                                                                 | M1 wellformedness as internal compounding value — promote slice 1 when bandwidth allows.                                                                                               |
| **WEAVE**                                                                                       | Greenfield import + harness build. Schedule after intercept-loop thesis is proven.                                                                                                     |
| **PFGW, ILGOV, LAC, OPENSPEC, GV2, GCTX, RMCPF, UCFG, BMAD4, CGBDG, FLAGCAT, APGOV, SEC, TEST** | Various long-bet / future / signpost / cross-cutting items. See [`ROADMAP.md`](./ROADMAP.md) Horizon 6 + audit followup tasks #17–22.                                                  |

---

## Cross-cutting gaps that any slice must address

These are missing prerequisites surfaced by councils. They aren't slices on
their own — they're glue that some Tier A picks need:

1. **`anvil export` CLI work item** — required by A5 (Dashboard MVP). One task
   in `crates/anvil-cli`. Without it, dashboard has no canonical `.anvil/*.json`
   to read.
2. **One reasoning-pattern rule in `crates/anvil-checks`** — required by A1
   (RTAI Spike). Without it, the demo headline is "secret detection mid-edit"
   rather than "AI-pattern detection."
3. **Single latency rubric** — required by A1. INTD-014, RTAI, and RMCP must
   cite one ADR-031 rubric rather than inventing per-surface numbers.
4. **Demo runbook** — required by A1. `anvil init` → `anvil mcp install` → open
   Cursor / Claude Code → paste known-bad pattern → Anvil flags before save.
   LAUNCH polishes save-time, RTAI defines validation semantics, RMCP owns the
   Rust MCP launch path — nobody owns the user journey.
5. **Rust MCP launch shim for Cursor / Claude Code** — RCLI3-016 writes config;
   RMCP makes `anvil mcp serve --stdio` real for A1.
6. **Diagnostic envelope coordination** — required by A2 (AIGUARD). AIGUARD-002
   ↔ RMCP-006 ↔ RTAI-006/RTAI-008 ↔ INTD-013 must agree; DRVR-002 consumes the
   result later.
7. **3 ADRs to accept now** — ADR-027 (pack architecture), ADR-028 (markdown
   crate), ADR-029 (suppression parser authority). Council D: "their continued
   Proposed status is procedural debt, not technical debt. Resolve in a single
   review session." Required by A4.
8. **Anchor re-scoring process owner** — required by A4. Council D recommends
   owning it — currently no permanent owner.

---

## Suggested first-pick combinations

You will pick — but for context, here are coherent multi-candidate combinations
the councils support:

| Combo                           | Slices                 | Net items | Posture                                                                                                             |
| ------------------------------- | ---------------------- | --------- | ------------------------------------------------------------------------------------------------------------------- |
| **Hype-phase minimum**          | A1 + A3                | ~30       | RTAI/RMCP demo + launch hygiene. Smallest credible launch.                                                          |
| **Hype-phase plus integration** | A1 + A2 + A3           | ~34       | Adds AIGUARD diagnostic envelope. Locks the AI-tooling integration story.                                           |
| **Hype-phase plus credibility** | A1 + A3 + A4           | ~33       | Adds language audit + check-ID registry + .env scan. Three governance artefacts riding alongside.                   |
| **Full launch slate**           | A1 + A2 + A3 + A4      | ~37       | All except dashboard, full MCP parity, and Graph v2. Realistic if RTAI/RMCP ships clean.                            |
| **Founder-pitch slate**         | A1 + A2 + A3 + A4 + A5 | ~49       | Launch + team-lead surface. Most ambitious Tier A. Adversarial risk: RTAI/RMCP bandwidth contention with dashboard. |

The councils consistently recommend **A1 + A3** as the floor and **A1 + A2 +
A3 + A4** as the realistic ceiling for a single release. A5 (Dashboard) is
high-leverage but is the candidate most likely to slip if RTAI takes longer than
RTAI-001 estimates.

---

## Followup work tracked separately

Audit + council outputs identified rescope work that is **not part of any
release slice** but should not be lost. Tracked as tasks #17–23:

- **#17** Rescope ILGOV — Rust target + graph-effect-vs-intent
- **#18** Rescope CFGINT — pick crate home + define graph artefact
- **#19** Rescope AGOV — retarget paths + AGOV-002 migration to CPACKS
- **#20** AIGUARD prep — clean Interfaces, coordinate envelope with RTAI/INTD
- **#21** POLFED rescope + ADR — codify OPAE/POLFED boundary
- **#22** GATE prep — coordinate contracts with INTD/DRVR/RTAI envelopes
- **#23** Promote Enterprise Readiness constellation to Tier B

These execute when bandwidth allows; none block any Tier A pick.

---

## What this document is NOT

- **Not an archive list.** Audit + councils recommended 0 archives after
  correcting for component-vs-module conflation. See [ROADMAP.md](./ROADMAP.md)
  "Cuts and parks" for what changed.
- **Not the source of truth for module status.** That lives in
  [`plans/index.aps.md`](./plans/index.aps.md).
- **Not a schedule.** Tier classification reflects readiness and value, not
  date.
