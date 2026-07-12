# APS Upstream Brief: Production Governance Patterns from anvil-001

- **Date:** 2026-06-15
- **Type:** Consumer feedback / patterns brief for the canonical APS project
- **Status:** Draft — for review and potential seeding into `eddacraft/anvil-plan-spec` (ROADMAP, docs, or new reference guidance)
- **Author:** Synthesised from anvil-001 operational experience (NBI, agent harness, reconciliation discipline)
- **Audience:** Maintainers and contributors to https://github.com/eddacraft/anvil-plan-spec

| Upstream (APS repo)                          | Downstream (anvil-001 heavy use)                          |
| -------------------------------------------- | --------------------------------------------------------- |
| Portable format, `aps-rules.md`, CLI (`next`/`start`/`complete`/`graph`/`lint`), templates, agent guidance, `project-context.md` scaffold | Full governance OS layered on top: NBI selector, mandatory dev-workflow routing, planning-council, atomic discipline, reconciliation/audit machinery, continuous-improvement loop |

## Purpose

This brief captures extensions, process innovations, and agent-surface patterns
that anvil-001 has exercised heavily on top of the portable APS foundation. It
is intended to:

- Help the APS repo evolve its guidance, templates, CLI surface, and reference material so that *other* adopters can more easily achieve the same "tight, living plans" outcome without reinventing the wheel.
- Identify high-value patterns that may deserve portable treatment (or at least strong recommended-practice documentation).
- Surface concrete, low-friction ways the APS project can help its users (seeds, reference modules, CLI affordances, docs updates).
- Maintain the clean separation: core format + orchestration stays portable; heavy governance and release integration can remain project-specific but benefit from better hooks and examples.

It is **not** a request to pull proprietary Anvil code. It follows the spirit
of ADR-055: narrow OSS carve-outs are allowed only for read-only consumers of
the public APS format, after provenance review and Anvil-specific behaviour is
removed. The intended contribution model is one-way seeding or clean-room
rewriting of portable concepts, not copying product internals.

## Context — anvil-001 as a Primary Production Consumer

anvil-001 is a large monorepo (Rust core + TS surfaces) that has used APS for all multi-step work since late 2025. It currently manages:

- `plans/index.aps.md` as the single source of truth (active modules + archived).
- ~50 active modules under `plans/modules/` (plus 130+ archived).
- Extensive `plans/execution/`, `plans/decisions/`, `plans/specs/`, `plans/reviews/`, `plans/audits/`.
- A standing `continuous-improvement-backlog.aps.md` (CIB).
- Full release records under `plans/releases/`.

It also ships a downstream TypeScript consumer implementation of APS semantics
(`packages/aps`: parser, loader, validator with rich adversarial fixtures,
state/locking, templates) and has its own `packages/adapters` for speckit/bmad
interop. Canonical format authority remains upstream in `anvil-plan-spec`.

The key differentiator vs lighter APS users is the **closed-loop operating system** built around the format: every non-trivial task (code, docs, planning, release, agent work) is forced through APS truth gates.

## Key Innovations Exercised at Scale

### 1. NBI — Next Best Item (Living Ranked Selector)

The single most visible process addition.

- A dedicated "Next Best Items" section at the top of the root index (see `plans/index.aps.md:89` onward).
- Ranked table: `Rank | NBI | Mode | Source | Why now | Next action`.
- Explicit dated "NBI review notes" that record every re-ranking, readiness promotion pass, release closeout, and rationale (multiple passes in June 2026 alone, including post-v0.8.0/v0.8.1 archive cascades and v0.9 scoping tied to ADR-075).
- Selection rules that prefer unblocked Ready work advancing the current release claim, adoption, trust, or recurring friction.
- Surfaced and acted on via `/plan-status` (which triggers reconciliation) and then routed through `dev-workflow`.
- Modules record the triggering NBI pass in their "Last reviewed:" lines (e.g. `graph-v2-foundation.aps.md`).

**Why it matters for other projects:** `aps next` gives a single next item. NBI gives a short, prioritised, *debated* shortlist with "why now" context that survives release windows and personnel change. It turns the index into an active decision record rather than a static backlog.

**Portable value:** An optional `## Next Best Items` pattern plus guidance on
how to maintain it (review notes, rank rules). Start with docs/examples; a
future `aps nbi` or enhanced `aps next --ranked` affordance should wait until
the table shape proves portable beyond anvil-001.

### 2. Agent / Workflow Surface (Planning as First-Class)

anvil-001 maintains a sophisticated, mandatory routing layer (much of it vendored/specialised from broader patterns developed by @joshuaboys, with anvil-specific tuning):

- `dev-workflow` (mandatory in anvil-001 only): APS Truth Gate → Ready/In Progress → Worktrunk branch from main → TDD → tiered Council (`/council quick|mini|full`) → addressing-pr-reviews → post-merge verification plan (tracked in `plans/reviews/post-merge/`) → continuous-improvement note → cleanup offer.
- `planning-workflow`: Intent → truth discovery (via `aps-planning`) → match existing item or create new → design gate (`brainstorming` or `planning-council`) → synthesis → readiness validation → clean handoff block.
- `aps-planning`: Session context loading, truth validation (drift, deps, `Blocks on:`, scope vs reality), reconciliation reports with explicit decisions (`valid | needs-plan-update | blocked`).
- `planning-council`: Multi-role judgement with dedicated playbooks (`plan-create`, `direction-validate`, `pre-execution-validate`, `plan-amend`). Uses stable roles mapped to specialist agents.
- Dedicated `anvil-plan-spec` agent for non-trivial module/task authoring, status sync, wave planning, and reconciliation.
- Commands / local harness entry points: `/plan`, `/plan-status` (NBI-aware),
  `/council`. These are local/private harness references in anvil-001 today;
  upstream examples should use sanitised, checked-in skill/playbook excerpts
  rather than requiring these exact paths or commands.
- Fable-5 tuned variants (`f5-planning-workflow`, `f5-aps-loop`, `f5-dev-workflow`).

These treat APS not just as a document format but as the **authoritative execution-authorisation substrate** for the entire development lifecycle (including the development of the agent surface itself) inside anvil-001. If upstreamed, this must remain an opt-in governance harness: APS core must still be useful without workflow skills, councils, hooks, MCP tools, or mandatory routing.

**Help for the APS repo:** Reference optional "governance harness" patterns or
an `agents/` + skills inventory for teams that want stronger enforcement. The
upstream already has planning prompts, scaffolded agents/skills, and CLI
orchestration; anvil-001's specialisation shows what a heavy user adds on top
(mandatory routing, council for planning decisions, continuous-improvement
substrate).

### 3. The "Keep Plans Alive" Discipline (the Tight Process)

This is the cultural/operational layer that other projects rarely match:

- **Single source of truth only:** `plans/index.aps.md` is canonical. Explicit rules against shadow indexes/summaries (repeated in `AGENTS.md`, `.claude/rules/aps-index.md`, `aps-planning` skill, `planning-workflow`).
- **Atomic updates:** Mark `In Progress` *before* substantive work. After completing a work item, update status + bump header count + index row in the *same* change. Archive completed modules with `git mv` to `plans/archive/modules/` + index path update in the same commit.
- **Status extensions + lifecycle narrative** (in `plans/project-context.md`):
  upstream APS canonical states remain `Draft / Ready / In Progress / Complete`
  (plus documented handling for `Blocked` where supported). anvil-001 uses a
  downstream dialect of `Proposed/Ready/In Progress/Done/Blocked` plus Anvil
  prose `Merged → Released/Shipped → Complete`. `DONE_PATTERNS` in tooling are
  case-sensitive. This is a project-context extension, not a portable APS
  contract.
- **Reconciliation machinery:** `scripts/aps/drift-check.mjs` (progress + release-record alignment, `shipped-aps-without-release-record` advisory), `active-lint.mjs`, `index-counts.mjs`. `.claude/workflows/aps-reconciliation-sweep.js` (adversarial semantic drift). Frequent dated audits under `plans/audits/`.
- **Continuous improvement substrate:** Every non-trivial agent session appends a compact entry to `plans/reviews/continuous-improvement-log.md`. Recurring items promoted to the standing CIB module. "Improvement: none" is accepted when there is no signal.
- **Documentation governance closeout:** Any change to `docs/**`, `plans/**`, AGENTS.md, READMEs etc. requires classification, cross-link/APS/index/ADR updates, validation runs, and an explicit "Docs Closeout" note.
- **Post-merge verification plans:** Extracted to tracked `plans/reviews/post-merge/<slug>.md` (gitignore exception) rather than living only in PR descriptions. Cleanup agent advances states using release records + these plans.
- **Anchor rescoring** and other cross-cutting process gates have their own documented processes with snapshot templates (`docs/guides/anchor-rescoring-process.md`).

**Portable value:** The core of `aps-rules.md` already encourages `project-context.md`. The APS repo could ship stronger recommended sections or a "governance reference" (example `project-context.md` fragments for NBI maintenance, reconciliation expectations, CI log, docs closeout checklist). The scripts/aps tooling was already the subject of an OSS carve-out discussion (ADR-055).

### 4. Other Proven Extensions

- Wave-based parallel execution (with gates) inside action plans.
- Explicit cross-cutting module conventions (`Coordinates with:`, `Blocks on:`, `Supersedes:`) with sweep/closeout rules.
- Release metadata blocks on work items (changeType, releaseIntent, releaseScope, releaseNote) — used by release orchestration.
- Full traceability from APS ID → branch naming convention → commit trailers → PR → release record → cleanup → archive.
- `plans/completed-index.aps.md` + frozen archive rows for historical releases.

## What Is Portable vs. Anvil-Specific

**Strongly portable / high value to surface in APS guidance:**
- NBI table + maintenance notes (as an advanced pattern on top of `aps next`).
- Atomic update + archive discipline (rules that "agents keep forgetting").
- Docs-governance closeout checklist (tying prose changes to plan truth).
- Cross-cutting module callout conventions.
- Recommended `project-context.md` skeleton for release integration and status dialect.

**Portable but heavyweight / best as optional governance extras:**
- Continuous-improvement log + standing CIB intake module.
- Post-merge verification plan extraction.
- Planning loops, council playbooks, workflow skills, and specialist agents.
- Read-only reconciliation helpers such as index-count and drift checks.

**Use with caution / not for every project:** These patterns add process weight.
Small projects may only need `aps next`, lint, and a short `project-context.md`.
The advanced governance harness is most appropriate when multiple agents,
release windows, or compliance-style evidence make stale plans materially risky.

**Project-specific (keep in `project-context.md`):**
- Exact lifecycle prose labels and extension mapping.
- Worktrunk/`wt` branching policy and Council tiering.
- Release-record format and cleanup agent implementation.
- The full vendored agent skill/agent surface (anvil-001's `dev-workflow` etc. are heavily tuned to its monorepo + release model).

**Already partially aligned:** The upstream now scaffolds `project-context.md`
and has planning prompts, optional agent/skill scaffolding, and an orchestration
CLI. The divergence in anvil-001 is mostly specialisation and enforcement.

## Upstream Adoption Boundary

| Pattern | Suggested upstream treatment | Notes |
| --- | --- | --- |
| Canonical status vocabulary | Core spec | Keep `Draft / Ready / In Progress / Complete` as the portable contract; document dialects separately. |
| NBI | Optional example / advanced guidance | Start as docs plus `index-with-nbi.example.md`; defer CLI until the table shape proves portable. |
| Atomic update/archive discipline | Advanced guidance | Safe to document as recommended practice without changing templates. |
| Docs closeout | Advanced guidance / checklist | Useful for docs-heavy repos; should not be required by core APS. |
| Continuous-improvement log + CIB | Optional governance add-on | High value for agent-heavy teams, process overhead for small repos. |
| Post-merge verification plans | Optional governance add-on | Ship as a copyable template under reviews/examples, not as a core requirement. |
| Reconciliation/drift scripts | Optional read-only tooling | Default local-only, path-scoped, no network, safe on untrusted plans. |
| Workflow skills, loops, councils, specialist agents | Optional agentic extras bundle | Explicit install choice, visible user consent, local-first defaults, and clear permission boundaries. |
| Worktrunk, exact Council tiers, Fable-5 variants | Project-specific | Mention only as Anvil examples, not upstream recommendations. |

## Proposed Contributions / Concrete Help the APS Repo Could Offer

1. **NBI guidance + optional example.** Add a short "Advanced Patterns" or
   "Keeping Plans Alive" page (or section in `workflow.md` /
   `ai-agent-guide.md`) describing the NBI table shape, review-note convention,
   and rank-selection heuristics. Prefer `index-with-nbi.example.md` or a
   snippet over adding NBI scaffolding to the default index template.

2. **Stronger "project-context.md" reference content.** Ship example fragments or a richer default for the update discipline, reconciliation expectations, CI log, and docs closeout. This is the natural place for "how to not let your plans rot."

3. **CLI affordances that support the discipline, after the shape stabilises.**
   Start with documented optional NBI markdown and JSON-friendly conventions.
   Defer `aps next --with-reasons` or `aps nbi` until the data contract has more
   than one adopter. Better JSON output from existing commands may be the safer
   first step.

4. **Reference "governance surface" examples.** A small optional bundle of
   reference files (minimal `continuous-improvement-log.md` shape, post-merge
   verification plan template, simple reconciliation script) that adopters can
   copy rather than invent.

5. **Optional agentic extras / governance harness.** Document a minimal optional
   surface for teams that want dev-workflow-style routing: planning loop,
   truth-gate loop, council review loop, reconciliation loop, and handoff loop.
   Ship as opt-in skills/playbooks/agents, not as the default APS path. MCP/tool
   examples must declare capabilities and require per-tool approval before file,
   shell, network, or credential access.

6. **Seed more of the mechanical tooling** (where licensing/IP allows per the
   ADR-055 carve-out model). The drift-check / index-counts / active-lint logic
   (scrubbed of Anvil dialect) would be high-value for any project that wants
   mechanical enforcement of the "keep current" rules. Treat this as optional
   read-only tooling first; only promote narrow, generic checks into core lint.

7. **Explicit status dialect section.** Clarify in the spec how projects may
   document extensions like `Proposed`, `Done`, `Merged`, `Released`, or
   `Shipped` while keeping canonical APS values as the portable contract. Current
   upstream support should be stated precisely; future alias support should be a
   proposal, not described as already present.

## Optional Agentic Extras Guardrails

If APS ships workflow skills, loops, councils, agents, MCP helpers, or
governance scripts as optional extras, they should follow these guardrails:

- **Opt-in install:** no agentic workflow should be auto-enabled by `aps init`.
- **Visible authority:** the repo must say whether APS state is advisory or an
  execution gate.
- **Fail closed on truth uncertainty:** if plan state, dependencies, or file
  scope cannot be validated, the agent asks rather than proceeds.
- **Local-first defaults:** council/reconciliation examples run locally by
  default and disclose any external model/provider boundary.
- **Sensitive-file exclusions:** examples avoid `.env`, credentials, auth files,
  and project-private context unless the user explicitly includes them.
- **Read-only by default:** reconciliation/drift helpers report before writing.
- **Capability declarations:** MCP/tool examples declare file, shell, network,
  and credential access separately.
- **Provenance checklist:** any seeded Anvil material records source, license
  status, attribution, scrub requirements, dependency audit, tests, and whether
  it is copied, rewritten from concept, or reference-only.

## Recommendations & Suggested Next Steps

- Treat this brief as input to the APS repo's own planning (it has plans/ and uses APS on itself).
- File a lightweight issue or ROADMAP item in `eddacraft/anvil-plan-spec` titled "Capture production governance patterns (NBI, update discipline, agent routing) from heavy consumers".
- Consider a small "governance patterns" reference directory or optional
  `governance` add-on bundle in the APS docs rather than bloating the core spec.
- If the APS repo wants live feedback, invite the anvil-001 team (or a representative) to a short planning-council-style review of any proposed additions.
- In anvil-001, any concrete follow-ups (e.g. "seed NBI example back", "contribute scrubbed reconciliation helper") can be tracked as CIB items or a small dedicated APS module.
- If optional agentic extras move forward, start with docs/examples first, then
  opt-in skills/agents, then read-only tooling, and only later consider core CLI
  support once the contracts have multiple adopters.

## References (in anvil-001)

- NBI section + review notes: `plans/index.aps.md:89-154` (and ongoing updates)
- Core discipline rules: `plans/aps-rules.md`, `plans/project-context.md` (lifecycle, status extensions, keeping current, docs governance), `.claude/rules/aps-index.md`
- Agent surface: local/private harness skills and commands for
  `aps-planning`, `planning-workflow`, `planning-council`, `dev-workflow`,
  Fable-5 variants, `anvil-plan-spec`, `/plan`, `/plan-status`, and `/council`.
  Upstreamable examples should use sanitised excerpts or checked-in optional
  extras rather than relying on these exact local paths.
- Mechanical support: `scripts/aps/{drift-check,active-lint,index-counts,advance-released}.mjs` + `lib/modules.mjs`; `.claude/workflows/aps-reconciliation-sweep.js`
- Continuous improvement: `plans/reviews/continuous-improvement-log.md`; standing CIB module
- Related decisions: ADR-055 (OSS carve-out for APS consumers), ADR-018 (product IP boundaries), various in `plans/decisions/`
- Product implementation: `packages/aps/` (parser/validator/loader/state/templates) + adapters
- Example of NBI-driven work in practice: `plans/archive/modules/graph-v2-foundation.aps.md` (Last reviewed notes), recent NBI review notes in the index, `plans/reviews/continuous-improvement-log.md` entries mentioning NBI

This pattern set has made APS the genuine backbone of delivery rather than a side artefact. Sharing the successful abstractions (while respecting the portable core) would materially help the APS project achieve its adoption goals.

---

**Docs closeout note (per project rules):** This is a new brainstorm/brief in `plans/brainstorms/`. It references existing APS, agent-surface, and governance docs but does not alter canonical truth files (`index.aps.md`, modules, AGENTS.md, or core guides). No index or decision-log update required at this stage; if promoted to an APS work item or contribution PR against the APS repo, the appropriate closeout steps will be followed. Validation: structure follows existing brief and triage patterns in the directory.
