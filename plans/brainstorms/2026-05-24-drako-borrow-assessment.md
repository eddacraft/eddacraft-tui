# Drako — Borrow Assessment

**Date:** 2026-05-24
**Status:** Brainstorm — assessment of Drako as a borrow candidate
proposed by Morgan (agent). **Outcome: decline code adoption; borrow
two concrete deliverables (SARIF export + agent-BOM framing) and one
narrative framing (baseline as "current posture vs new regression").
Reject step 6 of the proposed ladder (runtime enforcement) on the
same scope-guard grounds that declined Proxilion / PIC on 2026-05-22.**
**Source:** https://github.com/DrakoLabs/drako

---

## 1. Nomination summary

Morgan nominated Drako on the framing that the useful wedge is not its
rule set, but the **adoption ladder**:

1. Offline deterministic scan
2. Agent BOM inventory
3. Baseline-aware CI gate
4. SARIF / code-scanning export
5. Desktop-agent config scan
6. Runtime enforcement after buyer trust is established

| Project | What it is                                                                                                                                       | Stack                  | Maturity                       |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------- | ------------------------------ |
| Drako   | AI-agent governance platform: offline static AST scan of agent code, BOM, baseline gate, SARIF, desktop MCP-server discovery, optional runtime proxy. | Python 98% (+ TS scan) | 5★ / 129 commits / v3.0.0 / single-org (DrakoLabs) |

Target frameworks: CrewAI, LangGraph, AutoGen, LangChain, Semantic
Kernel, PydanticAI, LlamaIndex. License: **BUSL-1.1** (converts to
Apache 2.0 four years post-release; commercial use OK, no resale as
competing hosted service).

---

## 2. Scope-guard test

Per `docs/vision/anvil-scope-guard.md`, Anvil operates at the
**moment of change creation** (file-save / pre-commit / pre-push),
enforces deterministic policy against artefacts, and captures
provenance for **policy decisions**. Out of scope: agent
orchestration, runtime SaaS-call mediation, observability that does
not feed enforcement.

Applied to the six rungs of the proposed ladder:

| Rung                              | Scope-guard read                                          | Anvil's existing surface |
| --------------------------------- | --------------------------------------------------------- | ------------------------ |
| 1. Offline deterministic scan     | In scope (prevention, deterministic, pre-execution).      | `anvil check` (planless anti-pattern + secret scan), `anvil gate` (config-heavy checks), `anvil audit`. |
| 2. Agent BOM inventory            | In scope **if** the BOM feeds policy decisions or witness-chain enrichment. Out if it's only informational. | `detect_agents.rs` already builds an `AgentInventory` (Claude Code, Cursor, Aider, Windsurf, Codex), cached at `.anvil/cache/detected-agents.json`, used by `anvil start` / `anvil status`. Not yet exposed as a first-class "bom" surface. MCP servers covered via `anvil mcp-config`. |
| 3. Baseline-aware CI gate         | In scope and already an Anvil principle ("warnings over blocks; new edges only"). | `anvil baseline` writes `anvil/baseline.json`; `cutoff_commit` semantics shipped in MLP2-021/-031; `GENESIS-BASELINED` / `GENESIS-FRESH` semantics live (MLP2-013). |
| 4. SARIF export                   | In scope (deterministic, machine-readable, feeds existing dev workflows). | **Gap.** `anvil export` exists for constraints/config but no SARIF emitter for `check` / `gate` / `audit` findings. |
| 5. Desktop-agent config scan      | In scope **if** treated as a scan target with deterministic findings, not just adoption telemetry. | Partial. `activation/detect_agents.rs` covers the five-tool inventory; MCP-server config surface is touched by `anvil mcp-config`. Neither is framed as "scan the developer's agent surface and report findings against policy." |
| 6. Runtime enforcement (proxy)    | **Out of scope.** This is the same SaaS-call mediation layer Anvil declined for Proxilion / PIC on 2026-05-22 (see `2026-05-22-proxilion-pic-borrow-assessment.md`). | Not present; explicitly out per scope guard #5 and the prior assessment. |

**Verdict:** rungs 1–5 are aligned; rung 6 fails decision-framework
rule #2 (operates after change creation) and rule #4 (advisory /
mitigative rather than preventive at the artefact). Adopt = no, for
the Python codebase; borrow = yes, for two concrete deliverables and
one framing, scoped to rungs 1–5.

---

## 3. Overlap with existing Anvil work

Anvil already implements most of the ladder. Mapping rung-by-rung:

| Drako capability                          | Anvil equivalent (status)                                                                                                            |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `drako scan .` (offline AST)              | `anvil check` (anti-pattern + secret) + `anvil gate` (architecture / policy / boundaries / command-safety / lint / test / coverage). |
| `drako scan --baseline`                   | `anvil baseline` + `cutoff_commit` baseline-ancestry (MLP2-021/-031); first-scan posture distinguished from new regressions.         |
| `drako scan --format sarif`               | **Missing.** No SARIF emitter today.                                                                                                 |
| `drako bom .` (agent + tool + MCP BOM)    | `detect_agents.rs` covers AI tools; `anvil mcp-config` covers MCP servers. No single `anvil bom` surface that joins them.            |
| `drako init` (auto-generate .drako.yaml)  | `anvil init` + `anvil wizard` + `anvil start` writes `.anvilrc` / `.anvil.<ext>`.                                                    |
| `drako desktop scan`                      | `activation/detect_agents.rs` (inventory only); no policy-scan of desktop agent configs.                                             |
| `drako proxy start` (runtime enforcement) | Deliberately not built — see Proxilion/PIC decline (2026-05-22).                                                                     |
| `drako history` / `drako diff v2 v3`      | Witness chain (`anvil/witness/manifest/chain.ndjson`, MLP2-012) + `anvil audit-chain`.                                               |
| Pre-commit hooks                          | `anvil hooks` + `anvil hook` runtime subcommands.                                                                                    |
| CI gate (`--fail-on critical`)            | `anvil gate` with thresholds + exit codes 2/5/7/10 per CLI surface coherence spec.                                                   |

Anvil's lane is **structural governance of source artefacts at save /
commit / push time, language-agnostic**. Drako's lane is **AI-agent
code in a fixed set of Python frameworks, with optional runtime
proxy**. They are adjacent, not overlapping, in the part of the
problem they uniquely own.

---

## 4. The borrows worth taking

### Borrow A — SARIF export (concrete deliverable)

**Shape:** add `--format sarif` to `anvil check`, `anvil gate`, and
`anvil audit` so findings drop into GitHub Code Scanning, Sonar,
DefectDojo, and the standard SARIF consumers without bespoke
adapters.

- Deterministic by construction (we already produce deterministic
  findings).
- Pure additive output mode — no behaviour change for existing
  consumers.
- Likely lives next to `anvil export` or as an `--output sarif`
  flag on the existing finding-emitting commands.
- Suggested home: file as a new module under
  `plans/index.aps.md → Adoption and Sustained Use`, or as an
  item under the existing **Engineering Platform** row, named
  something like `SARIFOUT`. Decide at planning time.

This is the single highest-leverage borrow. Findings already exist;
making them consumable in the developer's existing dashboards is
where adoption friction drops.

### Borrow B — `anvil bom` as a first-class surface (framing + small build)

**Shape:** promote `detect_agents.rs` + the MCP-server inventory +
declared policies + credential **references** (never values) +
controlled-action surfaces into one command:

```
anvil bom            # human-readable summary
anvil bom --json     # machine-readable BOM for downstream tools
anvil bom --diff     # change since last BOM (drift)
```

Why this matters even though the pieces exist:

- It names a deliverable buyers can ask for ("show me the agent
  surface in this repo") without needing to know which Anvil
  subcommand emits which slice.
- It lets the witness chain enrich `ProtectionClaim` with "what
  agent + which MCP servers were present at the moment of change",
  which strengthens cross-session attribution (MLP2-071).
- It is a thin aggregator — no new detectors required for v1; it
  reframes what's already there.

Scope-guard caveat: this only earns its place in Anvil if the BOM
**feeds enforcement** (e.g., policy assertions against the BOM,
witness enrichment, drift signals). If it's only a pretty
inventory, it belongs in a separate tool.

### Borrow C — "current posture vs new regression" framing (narrative)

**Shape:** make this distinction explicit in `anvil baseline` /
`anvil check` / `anvil gate` output and in the docs. Anvil already
does this mechanically via `cutoff_commit`; the borrow is the
**phrasing**.

- First scan on an established repo: "current posture — N findings,
  baselined as-is."
- Subsequent scans: "new regressions — M findings since baseline."
- Maps onto Anvil's stated principle "warnings over blocks; new
  edges only" and gives that principle a user-facing vocabulary.

Zero-code borrow. Pure UX / docs work. Slot into the CLI surface
coherence spec or the wow-start docs.

---

## 5. What NOT to borrow

| Item                                      | Reason                                                                                                                                   |
| ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Drako's Python codebase                   | Wrong stack (Anvil is Rust workspace + TS packages); BUSL-1.1 not compatible with Anvil's licensing model (see `2026-04-07-anvil-licensing-decision.md` before any code import); upstream is 5★ / single-org. |
| The 97 Drako rule definitions             | They target Python AI frameworks (CrewAI etc.); Anvil's rule surface (`crates/anvil-rules`) is language-agnostic structural rules. Different threat model. |
| `drako proxy start` / runtime enforcement | Already declined via Proxilion / PIC (2026-05-22). Fails scope-guard rule #2 (operates after creation) and changes Anvil's operational topology (proxy + audit store). |
| EU AI Act articles 9/11/12/14 mapping     | Compliance-mapping is a marketing surface, not enforcement. If Anvil ever needs this, it belongs in a separate docs layer, not the engine. |
| Drako's dashboard at getdrako.com         | Out-of-scope per scope guard #5 (observability platforms) and Anvil's "Web Dashboard" row in `plans/index.aps.md` is governed separately. |
| "Determinism scoring" of model configs    | Targets agent runtime config (temperature, seed, iteration caps). Anvil does not govern model-runtime config and should not start.       |

---

## 6. Risks of the proposed framing

- **Ladder framing implies sequencing.** Morgan's six-rung framing
  reads like a roadmap. Anvil already has rungs 1–3 shipped or
  in-flight; framing them as "borrowed from Drako" misattributes
  prior art. Cite Drako as **parallel evolution**, not as the source
  of the adoption pattern. (See `plans/decisions/DECISION-LOG.md`
  conventions for attribution.)
- **BOM scope creep.** "Inventory agents, tools, MCP servers,
  policies, credentials references, and controlled action surfaces"
  is broad. Without a scope-guard pass on each slice, `anvil bom`
  could drift into being a generic asset-management tool. Anchor
  each slice to a witness-chain or policy-decision use case.
- **Runtime rung pressure.** Rung 6 ("runtime enforcement after
  buyer trust") is the same pull that nominated Proxilion. The
  decline reasoning from 2026-05-22 should be cited inline in any
  spec that touches `anvil bom` / `anvil mcp` so the next reviewer
  does not re-open it.
- **SARIF schema surface.** SARIF 2.1.0 is large; Anvil only needs
  the result + rule + location subset. Pin the supported subset in
  the spec to avoid open-ended schema maintenance.

---

## 7. Recommendation

1. **Decline code adoption** of Drako. Wrong stack, BUSL-1.1
   licensing, framework-specific rules, and the runtime rung
   fails scope-guard on prior precedent.

2. **Adopt three borrows**, scoped tightly:

   - **Borrow A (SARIF export):** file a new APS work item
     (working title `SARIFOUT-001`) for `--format sarif` on
     `anvil check` / `anvil gate` / `anvil audit`. Pin SARIF
     subset in the spec. Slot into the next planning wave after
     `v0.7.0-beta` cuts.
   - **Borrow B (`anvil bom`):** file a brainstorm follow-up under
     `plans/brainstorms/` named
     `2026-MM-DD-anvil-bom-surface.md` that scope-guards each
     slice (agents / MCP servers / policy refs / credential refs /
     controlled actions) **individually** against whether it
     feeds enforcement or witness enrichment. Do **not** file an
     APS module yet — the design needs the scope-guard pass first.
   - **Borrow C (baseline framing):** non-APS docs change. Land
     in the wow-start copy + `anvil baseline` / `anvil check`
     output text. Owner: whoever next touches the wow-start
     surface. Trivial.

3. **No dependency on Drako.** No Python import, no rule import,
   no SARIF schema fork (use the upstream SARIF 2.1.0 schema
   directly). The borrow is conceptual + one schema reference.

4. **Cite Drako as parallel evolution** in the SARIFOUT spec and
   the `anvil bom` brainstorm. Note: 5★ / 129 commits / BUSL-1.1
   means Drako is not load-bearing prior art — citation is
   courtesy, not dependency.

5. **Rung 6 (runtime enforcement) stays declined.** Cross-reference
   `2026-05-22-proxilion-pic-borrow-assessment.md` in any future
   spec that touches agent-runtime mediation.

---

## 8. Open questions (defer to follow-up specs)

- Does the witness chain's `ProtectionClaim` already carry enough
  agent-attribution data that `anvil bom --json` can be derived
  from the chain instead of being a separate scanner? If yes, the
  borrow shrinks further — `anvil bom` becomes a view over the
  chain, not a new collector.
- Should `anvil check` / `anvil gate` SARIF output include
  baseline-suppressed findings as `suppressions[]` (per SARIF
  2.1.0 §3.35) so reviewers can see what was deliberately
  accepted at baseline time? Likely yes; pin it in the SARIFOUT
  spec.
- Drako's `--threshold 70` is a "determinism score". Anvil has no
  equivalent scalar today and should not invent one without an
  ADR. Note and move on.

---

## 9. One-line summary

> Decline the Drako codebase. Take SARIF output as a concrete work
> item, `anvil bom` as a framing borrow needing its own scope-guard
> pass, and "current posture vs new regression" as a docs-only
> phrasing borrow. Runtime enforcement stays out for the same
> reasons Proxilion stayed out.
