# Node9 — Borrow Assessment

**Date:** 2026-06-06
**Status:** Brainstorm — assessment of Node9 as a borrow candidate (nominated by
Morgan). **Outcome: decline code/runtime adoption and decline dependency (Node9
is a runtime stdio MCP gateway — the Proxilion/PIC + Drako-rung-6 lane Anvil has
already declined three times). Validate Morgan's borrow after un-bundling it:
the genuinely high-value primitive is the *first-run exposure report framed as
"what would have been blocked" evidence* — but it earns its place only as a
read-only VIEW over Anvil's existing collectors + findings (the CIB-015 `anvil
bom` discipline), not as a new scanner fleet. Tool-definition pinning is also
worth taking, but split it: pin-by-hash + drift detection as a deterministic
save-time check is in scope; intercepting `tools/list`/`tools/call` at runtime
is out. Morgan's open question (score vs ranked findings) has a sharper answer
than he frames: Anvil already owns an internal `RiskScore` (ACTAX-020) that
routes enforcement tiers — keep that; do NOT add a user-facing aggregate
"exposure score" headline (false precision). Cite Node9 as parallel evolution;
no APS module filed in this pass; suggested CIB filings (next-available from
CIB-048) in §7.3.**
**Source:** https://github.com/node9-ai/node9-proxy (Apache-2.0, TypeScript
97.8%, v1.28.0 released 2026-05-27, 448 commits, **200★**, vendor: node9-ai)

---

## 0. What this document is

A borrow assessment of an external repo, in the format of
[`2026-06-03-meho-borrow-assessment.md`](./2026-06-03-meho-borrow-assessment.md),
[`2026-05-24-drako-borrow-assessment.md`](./2026-05-24-drako-borrow-assessment.md),
and
[`2026-05-22-proxilion-pic-borrow-assessment.md`](./2026-05-22-proxilion-pic-borrow-assessment.md).
The goal is **not** to adopt Node9 but to mine it for reusable ideas,
scope-guard each one, map it onto exact APS modules, and name the gaps. Facts
were read from the public repo landing on 2026-06-06 and cross-checked against
`plans/modules/*`, `docs/vision/anvil-scope-guard.md`, the
[agent-security-package brainstorm](./agent-security-package.md), and the prior
assessments above.

**Maturity note up front:** Node9 is the most-adopted candidate assessed to date
(200★ / v1.28.0 / 448 commits, vs Drako 5★, MEHO 3★, Proxilion 3★). That makes
the "just depend on it" pull stronger and the "parallel evolution / competitive
validation" signal louder — but it does not move the scope-guard line. Adoption
maturity ≠ same layer.

---

## 1. Nomination summary

Morgan nominated Node9 on two framings:

1. **First-run exposure reporting (the useful borrow).** Before enforcement,
   scan local agent histories, reachable credential files, risky actions, DLP
   findings, MCP capabilities, and tool drift so a team can see the current
   *blast radius*. Makes the case for Anvil through **evidence rather than
   fear**.
2. **MCP gateway ideas.** Intercept `tools/list` and `tools/call`, pin tool
   definitions by hash, quarantine changed tools, fail closed on corrupt pin
   state.

He maps both onto "anvil exposure" or an expanded `anvil scan` report with
**"what would have been blocked"** evidence, and asks the open question:
**should Anvil expose a score, or stick to ranked findings to avoid false
precision?**

| Project | What it is | Stack | Maturity |
| ------- | ---------- | ----- | -------- |
| Node9 | An "execution security layer for AI agents": a transparent **stdio MCP gateway** that sits between an agent and its tools, intercepts `tools/list`/`tools/call`, SHA-256-pins tool definitions and **quarantines** the session if a tool changes between connections (rug-pull defense), runs **pre-execution hooks** across Claude Code / Codex / Gemini CLI, does **DLP/credential scanning** (AWS/GitHub/Stripe/PEM) on tool args + responses, emits a **"blast radius" exposure report** over `~/.ssh` / `~/.aws` / `.env`, and writes atomic audit rows to `~/.node9/audit.log`. Ships curated domain "shields." | TypeScript 97.8% | Apache-2.0 · v1.28.0 (2026-05-27) · 448 commits · **200★** · single org (node9-ai) |

**Operational footprint:** a runtime proxy process per agent + an audit log.
This is a deployed interception layer, not a save-time CLI.

---

## 2. Scope-guard test

Per [`docs/vision/anvil-scope-guard.md`](../../docs/vision/anvil-scope-guard.md),
Anvil operates at the **moment of change creation**, enforces deterministic
policy against artefacts, and captures provenance for **policy decisions**. The
four-question borderline framework: (1) increases prevention, (2) operates
before/at execution time *(read as Anvil's "moment of change creation")*, (3)
strengthens deterministic control, (4) enforces rather than only informs.

Morgan's nomination bundles two layers the scope guard separates cleanly. The
central move — as with MEHO and Drako — is to **un-bundle them**:

| Node9 capability | Question that decides it | Scope-guard read |
| ---------------- | ------------------------ | ---------------- |
| Runtime stdio MCP gateway intercepting `tools/list` / `tools/call` | #2 — before/at change creation? | **Out.** Runtime mediation of executed tool calls. Same layer + topology declined for Proxilion/PIC (2026-05-22), Drako rung 6 (2026-05-24), and MEHO's authorization seam (2026-06-03). A proxy-per-agent is not Anvil's CLI+daemon footprint. |
| Pre-execution hooks across Claude Code / Codex / Gemini that intercept tool calls | #2 | **Out as a runtime interceptor.** Note: Anvil already has a *creation-time* hook lane (`anvil hook pre-push`, intercept daemon INTD/RTAI). Node9's hooks gate the agent's *tool call* at runtime; Anvil's gate the *artefact* at save/commit/push. Adjacent, not the same. |
| Tool definition **pinning by SHA-256** + **drift detection** (tool changed since last seen) | #1, #3 — deterministic, preventive? | **Split.** Computing a content hash of a declared MCP tool definition and warning when it drifts from a pinned baseline is a **deterministic, artefact-shaped check** — in scope as a *save-time scan* (`new-edges-only`, baseline the current pin, warn on change). **Enforcing** the pin by quarantining a *live session mid-call* is the runtime half — out. |
| Quarantine changed tools / fail-closed on corrupt pin state | #4 | **Out as runtime behaviour; In as a finding.** Anvil reports "this tool's definition drifted from its pin" as a deterministic finding; it does not hold the live session open to quarantine it. (Anvil's own daemon already has a fail-closed quarantine model — INTD/surface-drivers — but that fences *Anvil's* surfaces, not the agent's MCP calls.) |
| DLP / credential scanning (AWS/GitHub/Stripe/PEM) on tool args + response text | #1, #3 | **Mostly already owned, In where artefact-shaped.** Anvil already scans for secrets at save/commit (IORISK, `anvil-checks` secret detection). Scanning the *contents of live tool-call traffic* is the runtime half — out. Scanning *reachable credential files* in the repo/workspace as a one-shot is in scope (it's a static artefact scan). |
| First-run **exposure report / "blast radius"** over `~/.ssh` / `~/.aws` / `.env`, agent histories, MCP capabilities, tool drift | #4 — enforce or inform? | **In as a VIEW that feeds enforcement, with caveats.** A read-only report is *only informational* on its own (decision-rule #4 fails a pure dashboard). It earns scope **iff** it is a view over collectors Anvil already runs *for enforcement* and frames the findings as **"what Anvil's gate would flag/block"** — i.e. it sells the enforcement, it is not a standalone observability product. This is the CIB-015 `anvil bom` discipline applied verbatim. |
| Atomic audit log (`~/.node9/audit.log`) | Observability platform? | **Out — already owned.** Anvil's audit lane is the local-first witness chain (`anvil/witness/manifest/chain.ndjson`, MLP2). A second runtime audit store changes the footprint. |
| Curated domain "shields" (db / cloud / shell) | #1, #3 | **In as policy-pack inspiration only.** Maps to CPACKS / pipelock's "17 default rule packs" candidate (already tracked, 2026-05-13 candidates list). Parallel evolution, not new scope. |

**Verdict:** the runtime gateway, the live `tools/call` interception, the
session-quarantine, and the runtime audit store fail decision-rule #2 on the
same precedent that declined Proxilion/PIC, Drako rung 6, and MEHO's seam. The
**exposure report (as a view-that-sells-enforcement)** and the **tool-pin /
drift detection (as a save-time check, not a runtime quarantine)** are the
in-scope borrows. Morgan's instinct on the exposure report is right; his framing
needs the un-bundling above so the MCP gateway does not smuggle Anvil into being
a runtime proxy.

---

## 3. Overlap with existing Anvil work

Anvil has already adjudicated Node9's runtime lane three times, and already owns
almost every collector the exposure report would view over.

| Node9 capability | Anvil equivalent (status) |
| ---------------- | ------------------------- |
| Runtime stdio MCP gateway / agent↔tool interception | **Declined** — Proxilion/PIC (2026-05-22), Drako rung 6 (2026-05-24), MEHO seam (2026-06-03). Same operational-topology objection (proxy + audit store). |
| Deployable central enforcement point | **GATE** (gateway-control-plane-patterns) · **Draft 0/3** — demoted Ready→Draft pending an enterprise consumer. Node9 is a working reference of GATE's "deployable control-plane patterns"; it does **not** change GATE's promotion gate (needs a prospect). |
| MCP `tools/list` / `tools/call` handling | Anvil already *implements* an MCP server (`tools/call`, JSON-RPC, PR #1277; seven tools incl. `anvil_validate_write`, `anvil_gate` — see post-merge RMCPF-011/012). Anvil is an MCP **server/tool provider**, not a gateway that proxies *other* servers. Different role. |
| Tool capability view before a call | **POLCAP** (policy-capability-discovery) · **Proposed** — `anvil policy capabilities` gives a governed agent a deterministic, signed, machine-readable view of action families it may attempt, with **fail-closed semantics for unknown/stale cap-IDs**. This is Anvil's *advisory* answer to "what can this agent do"; `gate` stays the enforcement authority. Strong conceptual overlap with Node9's `tools/list` shaping — but Anvil's is creation-time + advisory, not a runtime intercept. |
| Tool-definition pinning by hash + drift | **Partial gap.** Anvil hashes policy bundles (`rules_sha`, MLP2-014) and pins update artefacts (ADR-045 signing), but has **no MCP-tool-definition pin/baseline**. `anvil mcp-config` touches MCP server config; it does not pin tool *definitions* and diff them. This is the one genuinely net-new deterministic check Node9 suggests (mirrors the agent-security-package brainstorm idea #5 "Tool Definition Integrity & Registry"). |
| DLP / credential scanning | **IORISK** (io-risk-controls) · **Ready** — provider-agnostic input/output risk controls (prompt injection, sensitive-data leakage); `anvil-checks` secret detection already ships. Scanning *reachable credential files* as a one-shot exposure pass is a thin extension; scanning *live tool traffic* is out. |
| First-run exposure / "blast radius" report | **CIB-015 already decided the shape.** The `anvil bom` triage (Merged 2026-05-26, [brainstorm](./2026-05-26-anvil-bom-surface.md)) concluded: a read-only **view + `--diff` drift gate** over existing collectors earns its place; new collectors (MCP-server inventory, credential-reference registry) were **rejected**. Node9's exposure report is the same surface with a sharper *sales* framing ("what would have been blocked"). |
| "What would have been blocked" evidence | **Partial gap / strong fit.** Anvil already separates *current posture* from *new regressions* mechanically (`cutoff_commit`, baseline; CIB-016 names the phrasing). It has report-only transport paths (daemon-save-time `roundtrip_validate_paths`). It does **not** yet have a single first-run command that runs the gate in **report-only mode** and renders "N findings Anvil's gate would flag here." That framing is the borrow. |
| Risk score fused into routing | **ACTAX** (policy-action-taxonomy) · `RiskScore` output (ACTAX-020) routed to `warn/fence/interrupt` tiers, with **AGOV trust score as session amplifier** (ACTAX-021). Anvil already *has* an internal scalar — for routing, not for display. This directly informs Morgan's open question (§6, §8). |
| Quarantine / fail-closed | **AGOV-001/006**, INTD/surface-drivers reliability-budget quarantine, `015-intercept-loop-enforcement.md` ("fail closed for wrapped launches"). Anvil's quarantine fences *its own* surfaces; it is not a session-level MCP quarantine. |
| Atomic runtime audit log | **Witness chain** (MLP2) + `anvil audit-chain` — already owned, local-first, hash-chained. |
| Curated domain "shields" | **CPACKS** (compliance-policy-packs) · Draft + the pipelock "17 default rule packs" candidate (2026-05-13). Parallel evolution. |
| Pre-execution hooks across Claude/Codex/Gemini | `detect_agents.rs` inventory (Claude Code, Cursor, Aider, Windsurf, Codex) + `anvil hook` + INTL wrapped-launch. Anvil hooks the *artefact*; Node9 hooks the *tool call*. |

Anvil's lane: **deterministic, evidence-producing governance of change at
save/commit/push time, language-agnostic, local-first CLI + daemon, and an MCP
*server* exposing its own validation tools.** Node9's lane: **a runtime stdio
gateway that proxies other MCP servers and mediates live tool calls.** Adjacent
(both gate agent activity with policy + audit), not overlapping in the layer
each uniquely owns — the same finding as Proxilion/PIC, Drako, and MEHO.

---

## 4. The borrows worth taking

Verdicts: **Use directly** (facts/shapes; clean-room reimplement) · **Adapt** ·
**Inspiration only**.

### Borrow A — first-run exposure report as "what would have been blocked" evidence (the primitive · Adapt → report-only mode over CIB-015 view)

**This is the most valuable primitive in Node9, and the one Morgan is right
about.** It is an *adoption wedge*, not a new capability: it makes Anvil's
existing deterministic findings legible as a one-shot "here is your blast radius
/ here is what Anvil's gate would have flagged" report, before the team turns
enforcement on. Evidence over fear.

The shape that survives the scope guard (composing CIB-015 + CIB-016 + the
report-only transport that already exists):

- A first-run command (working title `anvil exposure`, or `anvil scan
  --report-only` / `anvil gate --report-only`) that runs the **existing** gate
  pipeline in report-only mode and renders findings as **"N findings Anvil's
  gate would flag here"** — explicitly *would-block* framing, not *did-block*.
- It is a **view over collectors Anvil already runs for enforcement** (the
  CIB-015 rule: no new detectors): `anvil-checks` secret/antipattern findings,
  IORISK DLP findings, the detected-agents cache, MCP-server config (`anvil
  mcp-config`), policy refs (`anvil policy list`). The three slices CIB-015
  blessed (agents / policy refs / witness summary) are the spine; the
  credential-*file* reach is the thin in-scope extension (static artefact scan,
  not live-traffic scan).
- It uses the **CIB-016 vocabulary** verbatim: "current posture — N findings,
  baselined as-is" on an established repo; "new regressions — M since baseline"
  thereafter. The exposure report is the natural home for that phrasing.
- Output: human-readable + `--json`, and a `--diff` against the last run (the
  CIB-015 drift gate). Findings are **ranked**, severity- and confidence-tagged
  — *not* aggregated into a single number (§6).

Scope-guard caveat (load-bearing): this earns its place **only** because it
sells the enforcement. A standalone "blast radius dashboard" that never feeds a
gate decision is a pure observability product (scope-guard #5, decision-rule
#4) — out. Anchor every line of the report to "this is what `anvil gate` does
about it."

### Borrow B — MCP tool-definition pin + drift detection as a save-time check (concrete · Adapt → new check, NOT a runtime quarantine)

Node9's SHA-256 tool-definition pinning is the same idea as the
agent-security-package brainstorm's #5 ("Tool Definition Integrity & Registry"),
and it has a clean **deterministic, artefact-shaped** core:

- Compute a content hash of each declared MCP tool definition (name +
  description + parameter schema — "the description is what attackers modify").
- Baseline the current set of pins (`new-edges-only`, ADR-003 — pre-existing
  tools are baselined as-is, not flagged on first run).
- On subsequent scans, **warn** when a tool's definition drifts from its pin
  (the "rug-pull" signal) as a deterministic finding.

What stays out: **quarantining a live session mid-call** and **failing closed on
the runtime hot path** — that is the gateway behaviour (§2, §5). Anvil reports
the drift; it does not hold the agent's session open. This lands as a check in
`crates/anvil-checks` / `crates/anvil-intercept-rules` (alongside INTR), or as a
slice of the exposure report (Borrow A), surfacing through POLCAP's vocabulary
where it overlaps.

### Borrow C — "evidence over fear" as the standing adoption framing (Inspiration / docs)

Node9's positioning — *show the blast radius before you ask anyone to enforce* —
is the right DX framing for Anvil's wow-start and POC motion, and it composes
with the existing borrows already on file: preloop's transparent onboarding
(2026-05-13), Drako's "current posture vs new regression" (CIB-016), and the
ghostguard tiered pipeline. Zero-code; write it into the wow-start narrative and
the exposure-report spec so the next reviewer treats the report as a *sales
artefact for enforcement*, not as a new product line.

### Borrow D — answer to the open question: keep the routing `RiskScore`, refuse the headline exposure score (framing · the answer Morgan asks for)

Morgan's open question — "expose a score, or rank findings?" — has a sharper
answer than a flat yes/no, because Anvil already has **both** halves decided:

1. **Internal `RiskScore` exists and stays (ACTAX-020/-022).** Anvil already
   fuses a thin risk score that *routes* enforcement tiers (`warn / fence /
   interrupt`), amplified by AGOV trust score. That is a deterministic,
   load-bearing scalar — keep it. It is not a vanity metric; it drives a
   decision.
2. **No user-facing aggregate "exposure score" headline.** Collapsing a
   heterogeneous exposure report (credential reach + DLP + tool drift + risky
   actions) into one 0–100 number is exactly the "determinism score" Drako
   raised and Anvil declined ("no equivalent scalar... should not invent one
   without an ADR", Drako §8). It manufactures false precision and invites
   gaming. **Rank findings; tag each with severity + confidence; show counts
   per class.** If a single summary is ever wanted, it is a *count by severity*
   ("3 critical, 7 high"), never a fused score.

So: **expose ranked findings + per-finding severity/confidence + the existing
internal `RiskScore` where it routes enforcement; do not add an aggregate
exposure score.** This is a zero-code framing borrow — write it into the
exposure-report spec and, if a fused public score is ever proposed, require an
ADR (Drako precedent).

---

## 5. What NOT to borrow

| Item | Reason |
| ---- | ------ |
| Node9's TypeScript codebase | Wrong stack for the engine (Anvil core = Rust workspace; TS is packages/tooling). Apache-2.0 is vendor-friendly, but clean-room reimplement is the standing rule (every adopted dep adds to attribution-pipeline-v3: `about.toml`, `deny.toml`, `ACKNOWLEDGEMENTS.md`). No port/vendor. |
| The runtime stdio MCP gateway / `tools/list`/`tools/call` interception | Out of scope (decision-rule #2). Same precedent as Proxilion/PIC, Drako rung 6, MEHO seam. Changes Anvil's operational topology to a proxy-per-agent. |
| Session quarantine + fail-closed on the runtime hot path | The enforcement half of tool-pinning. Anvil reports drift (Borrow B); it does not mediate the live session. Anvil's own fail-closed quarantine fences *its* surfaces, not the agent's MCP calls. |
| DLP on live tool-call traffic | Runtime traffic inspection. Anvil scans *artefacts* (files, configs, commits). The static credential-*file* reach is the only in-scope slice. |
| `~/.node9/audit.log` runtime audit store | Anvil's audit lane is the witness chain (local-first, hash-chained). A second runtime store changes the footprint — same objection as MEHO's Postgres/Valkey. |
| Curated "shields" as a code import | Inspiration for CPACKS / the pipelock default-pack candidate only. Don't fork Node9's rule content; Anvil's rules are language-agnostic structural rules. |
| An aggregate user-facing "exposure score" | False precision (Borrow D, Drako §8). Requires an ADR if ever revisited. |
| Pre-execution tool-call hooks across Claude/Codex/Gemini | Runtime interception layer. Anvil's hooks are creation-time on the artefact. |

---

## 6. Risks of the proposed framing

- **Gateway smuggling (highest).** Node9 is a *gateway first*; the exposure
  report is a feature of it. The risk is adopting the report and quietly
  inheriting the proxy to "make it real-time." Hard-state in any spec: the
  exposure report runs the **existing** pipeline in **report-only** mode over
  **existing** collectors. No proxy, no live interception. Cross-reference
  Proxilion/PIC + Drako rung 6 inline so the next reviewer cannot re-open it.
- **Report drifts into a dashboard.** "Blast radius report" reads like an
  observability product (scope-guard #5). It survives *only* as a
  view-that-sells-enforcement (decision-rule #4). Every finding must map to a
  gate action; if a slice has no enforcement behind it, cut the slice — do not
  invent a collector for it (the CIB-015 rejection of MCP-server-inventory and
  credential-registry collectors stands).
- **Score temptation.** Node9 ships "blast radius" as a vibe; the easy next step
  is a single number. That is false precision (Borrow D). Keep the internal
  `RiskScore` (routing) and ranked findings (display) cleanly separated; gate
  any fused public score behind an ADR.
- **Collector creep on the credential slice.** "Reachable credential files
  (`~/.ssh`, `~/.aws`, `.env`)" is a *static artefact scan* in scope — but
  scanning a user's `$HOME` is a privacy/footprint expansion beyond the repo.
  Bound it to the workspace + explicitly declared paths; never auto-walk `$HOME`
  without opt-in. (CIB-015 already rejected a credential-*reference registry*
  for exactly this sensitivity.)
- **Maturity asymmetry cuts both ways.** At 200★ Node9 is real software, which
  makes "depend on it" tempting — but it is single-org, at a layer Anvil does
  not operate at, so a dependency would still be a de-facto fork of a runtime
  proxy. Its maturity is best used as *competitive validation* that the exposure
  wedge has market pull, not as a dependency.
- **POLCAP collision.** Node9's `tools/list` shaping and Anvil's POLCAP
  capability view solve adjacent problems with overlapping vocabulary
  (capabilities, fail-closed, signed view). Keep them distinct: POLCAP is
  creation-time + advisory; Node9's is runtime + enforcing. Don't let the
  exposure report's tool-drift slice grow into a runtime capability broker.

---

## 7. Recommendation

**Decline Node9 code/runtime adoption and dependency. Validate Morgan's borrow
after un-bundling it: take the first-run exposure report as a report-only VIEW
that sells enforcement (Borrow A, built on the CIB-015/-016 decisions already
made), take MCP tool-definition pin + drift as a deterministic save-time check
(Borrow B, not a runtime quarantine), adopt "evidence over fear" as the standing
adoption framing (Borrow C), and adopt keep-the-routing-RiskScore /
refuse-the-aggregate-exposure-score as the answer to the open question (Borrow
D). Cite Node9 as parallel evolution and as competitive validation of the
exposure wedge; no APS module filed in this pass.**

### Most valuable primitive
The **first-run exposure report framed as "what would have been blocked"
evidence** (Borrow A). It is the highest-leverage piece because it is an
*adoption wedge built almost entirely from parts Anvil already ships* —
deterministic findings, baseline/posture-vs-regression, report-only transport,
the CIB-015 view — repackaged to make the enforcement case before anyone has to
trust the enforcement.

### Customer impact
**High on adoption, low on net-new engineering.** The exposure report directly
attacks the #1 friction in a security-tool POC ("why should I turn this on?")
by answering it with the buyer's own repo. It is the procurement/POC artefact
("show me the blast radius in *my* code"). Because it reuses existing
collectors, the cost is mostly UX + a report-only mode, not a new scanner fleet.
Borrow B (tool-drift) adds a concrete, named threat (MCP rug-pull) that buyers
recognise. Neither requires Anvil to become a runtime proxy.

### Acquisition strategy
**Borrow-pattern, clean-room, no dependency.** Reimplement the report-only mode
and tool-pin check in the Rust workspace over existing collectors. No TS import,
no proxy adoption, no second audit store. Cite Node9 as parallel evolution
(courtesy, not load-bearing prior art) and as market validation of the exposure
wedge.

### Decision-ladder placement
Per the assessment brief (ignore / track / document / specify / plan / prototype
/ depend):

| Slice | Placement | Why |
| ----- | --------- | --- |
| Node9 runtime MCP gateway (the product) | **Ignore / decline** | Out of scope; Proxilion/PIC + Drako + MEHO + GATE precedent. |
| First-run exposure report (Borrow A) | **Document now → Specify** (builds on CIB-015/-016, already decided) | In scope as a report-only view; highest-value, lowest-net-new. The one slice worth moving toward a spec. |
| MCP tool-pin + drift check (Borrow B) | **Track → Specify** when the exposure report or POLCAP advances | Net-new deterministic check; real gap; pairs with the report. |
| "Evidence over fear" framing (Borrow C) | **Document now** (wow-start + exposure spec) | Zero-code; composes with preloop/Drako/ghostguard borrows. |
| Score vs ranked findings (Borrow D) | **Document now** (closes the open question; ADR gate on any fused score) | Zero-code; ACTAX `RiskScore` already settles half of it. |
| Dependency on Node9 | **No** | Single-org, wrong layer/topology, runtime proxy. |

### 7.1 APS modules to update (exact list)

| Module | ID · Status | Update |
| ------ | ----------- | ------ |
| continuous-improvement-backlog | **CIB** · In Progress (29/47) | File the Borrow-A/B/C/D items (see §7.3). The exposure report extends the **already-merged CIB-015** decision (`anvil bom` view) — note the lineage so the report is built as the blessed view, not a new collector. |
| io-risk-controls | **IORISK** · Ready | The DLP/credential-*file* exposure slice is an IORISK consumer (reachable-credential-file scan as a finding source for the report). Workspace-bounded; never auto-walk `$HOME`. |
| policy-capability-discovery | **POLCAP** · Proposed | Cross-reference: Node9's `tools/list` shaping is the *runtime* analogue of POLCAP's creation-time advisory capability view. Keep distinct (advisory vs enforcing). Tool-pin/drift (Borrow B) can share POLCAP's cap-vocabulary where they overlap. |
| policy-action-taxonomy | **ACTAX** · (RiskScore in flight) | Note that the routing `RiskScore` (ACTAX-020/-022) is the answer to "should Anvil score?" — keep internal/routing; do not expose an aggregate exposure score (Borrow D). |
| agent-governance-patterns | **AGOV** · Draft (parking lot) | The exposure report's agent/witness slices are the CIB-015 `AGOV-NNN` view; tool-definition integrity echoes the agent-security-package #5 idea. No promotion (AGOV still in the post-launch parking lot per 2026-04-26 audit). |
| gateway-control-plane-patterns | **GATE** · Draft 0/3 | Add Node9 to GATE-001 reference topologies as a *parallel-evolution* example of a deployable runtime gateway. **Do not change status** — promotion still gated on an enterprise prospect. |
| compliance-policy-packs | **CPACKS** · Draft | Node9's domain "shields" reinforce the pipelock default-pack candidate (2026-05-13). Inspiration only; no fork. |
| intercept-rules | **INTR** · In Progress 5/8 | Candidate home for the Borrow-B tool-definition-drift rule if it lands as a deterministic check rather than a report slice. |

### 7.2 Gaps identified

1. **No report-only "what-would-have-been-blocked" mode** as a first-class
   first-run command. The pieces exist (report-only transport, baseline,
   CIB-015 view, CIB-016 phrasing) but no single command composes them into the
   exposure wedge. This is the work.
2. **No MCP tool-definition pin/baseline + drift check.** `anvil mcp-config`
   touches server config; nothing hashes tool *definitions* and diffs them.
   Net-new (the agent-security-package #5 idea, never built).
3. **Credential-*file* reach is unscoped.** IORISK scans content for secrets;
   "which reachable credential files exist in the workspace" as an exposure
   finding needs a workspace-bounded, opt-in-beyond-repo contract before it
   lands (privacy/footprint risk, §6).
4. **"Exposure score" has no ADR.** If a fused public score is ever wanted, it
   needs an ADR (Drako §8 precedent). Until then: ranked findings + severity
   counts only.

### 7.3 Suggested CIB filings (next-available IDs — allocate at filing time)

Next-available is **CIB-048** (CIB header reads 29/47; max id CIB-047). Not
hard-coded here, to avoid a numbering race — allocate when filed under
[`continuous-improvement-backlog`](../modules/continuous-improvement-backlog.aps.md):

- **CIB-(next):** Spec the first-run **exposure report** (`anvil exposure` /
  `--report-only`) as a view over existing collectors + report-only gate mode,
  with "what-would-have-been-blocked" framing and CIB-016 posture-vs-regression
  vocabulary. Acceptance note: **no new collectors** (CIB-015 rule), **no
  proxy**, **ranked findings not a fused score** (Borrow D), credential-file
  slice workspace-bounded. Builds on CIB-015.
- **CIB-(next+1):** Triage an **MCP tool-definition pin + drift check** (Borrow
  B) — hash tool name/description/param-schema, baseline (`new-edges-only`),
  warn on drift; explicitly *no* runtime session quarantine. Decide home
  (`anvil-checks` / INTR / exposure-report slice) at triage.
- **CIB-(next+2, docs):** Write the **"evidence over fear" adoption framing**
  (Borrow C) + the **score-vs-ranked-findings answer** (Borrow D) into the
  exposure-report spec, wow-start narrative, and a note that any fused public
  score requires an ADR.

---

## 8. Open questions (defer to follow-up specs)

- Can the exposure report be **derived entirely from the witness chain + the
  CIB-015 view**, so it adds zero collectors? CIB-015 found the witness chain
  carries no reliable agent attribution today (`ProtectionClaim` has no agent
  tag, `agent_tag` not persisted) — so the agent slice may need the
  detected-agents cache, not the chain. Pin which collector backs each slice in
  the spec.
- Is "report-only mode" just `anvil gate` with enforcement suppressed and a
  `would_block: true` annotation on each finding, or a distinct command? Likely
  a flag on the existing pipeline (reuse, don't fork) — confirm against the
  daemon-save-time `roundtrip_validate_paths` report-only path already shipped.
- For Borrow B: does pinning the *param schema* + *description* (not the full
  server config) catch the rug-pull threat without over-flagging benign
  version bumps? Probably yes — but decide the canonical hash inputs in the
  triage so the pin is stable.
- Should the credential-file exposure slice default to **repo + declared paths
  only**, with `$HOME`-adjacent paths (`~/.ssh`, `~/.aws`) behind an explicit
  opt-in flag? (§6 privacy risk; CIB-015 rejected a credential registry for the
  same sensitivity.) Lean yes.
- Does an enterprise-prospect trigger ever promote GATE *and* a runtime exposure
  capability together (the "Enterprise Readiness constellation")? If so, Node9
  becomes a reference topology in that wave — but only then, and the runtime
  half still gets its own scope-guard pass.

---

## 9. One-line summary

> Decline the Node9 codebase, its runtime stdio MCP gateway, the live
> `tools/call` interception, the session-quarantine, and the second audit store
> (scope guard: Out; Proxilion/PIC + Drako + MEHO + GATE precedent). Validate
> Morgan's borrow *after un-bundling it*: take the **first-run exposure report**
> as a report-only view that sells enforcement (built on the already-decided
> CIB-015/-016 work), take **MCP tool-pin + drift** as a deterministic
> save-time check (not a runtime quarantine), adopt **"evidence over fear"** as
> the adoption framing, and answer the open question with **keep the routing
> `RiskScore`, refuse the aggregate exposure score**. No dependency; clean-room;
> cite Node9 as parallel evolution and as competitive validation of the exposure
> wedge.
