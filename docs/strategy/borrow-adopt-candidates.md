# Borrow / Adopt Candidates

| Type  | Authority | Owner    | Status | Freshness                                        |
| ----- | --------- | -------- | ------ | ------------------------------------------------ |
| Guide | Advisory  | STRATEGY | Live   | Metadata backfilled 2026-05-27 during DOCGOV-011 |

| Upstream                  | Downstream                              |
| ------------------------- | --------------------------------------- |
| External project research | Borrow/adopt strategy and APS follow-up |

Purpose: track concrete ideas to borrow, adapt, or integrate from external
projects into Anvil.

## Workflow

- Add new candidates under the current date section (`## YYYY-MM-DD`).
- Keep entries short and decision-ready.
- Move status forward as validation happens.

Entry format:

- **source repo + link:** `owner/repo` — https://github.com/owner/repo
- **what to borrow/adopt:** specific capability/pattern
- **adopt type:** `borrow-pattern` | `integrate-dependency` | `copy-ux`
- **integration effort:** `S` | `M` | `L`
- **expected impact:** `Low` | `Med` | `High`
- **status:** `candidate` | `validating` | `adopted` | `rejected`
- **aps link (optional):** module/task ID

---

## 2026-05-07

- **source repo + link:** `entireio/cli` — https://github.com/entireio/cli
  - **what to borrow/adopt:** Branch-as-sidecar pattern for storing AI session
    data — two-tier git refs (shadow branch with full worktree snapshots for
    in-flight rewind; sharded permanent metadata branch) linked to user commits
    via a single `Entire-Checkpoint:` trailer. Full write-up at
    [`docs/architecture/references/entire-branch-sidecar.md`](../architecture/references/entire-branch-sidecar.md).
  - **adopt type:** borrow-pattern
  - **integration effort:** M
  - **expected impact:** High
  - **status:** candidate
  - **aps link (optional):** kindling-capture, council outputs, APS history
  - **overlap with existing Anvil services:**
    - kindling-capture (PostToolUse hook) currently writes to an external store
      — git-native sidecar would replace that path
    - Council review findings and APS work-item history both want a place to
      live without polluting main tree / `plans/`
  - **architecture notes / anti-frankenstein guardrails:**
    - Use `refs/anvil/*` namespace (not `refs/heads/anvil/*`) so `git branch`
      output stays clean
    - Adopt explicit version anchor in ref names (`refs/anvil/<feature>/v1`)
      from day one
    - Define GC policy for shadow refs upfront (Entire's docs do not)
    - Define `git commit --amend` / rebase behavior (re-run hook or warn)
    - Require trailer to be optional and strippable; do not block commits that
      lack it
    - Add security/privacy review for what transcripts contain before enabling
      default push

---

## 2026-03-08

- **source repo + link:** `guardrails-ai/guardrails` —
  https://github.com/guardrails-ai/guardrails
  - **what to borrow/adopt:** validator-hub model for composable input/output
    safety checks as reusable policy packs
  - **adopt type:** borrow-pattern
  - **integration effort:** M
  - **expected impact:** High
  - **status:** candidate
  - **aps link (optional):** POLRESET / OPAE / CPACKS

- **source repo + link:** `open-policy-agent/opa` —
  https://github.com/open-policy-agent/opa
  - **what to borrow/adopt:** policy-as-code decoupled decision layer for
    auditable runtime governance
  - **adopt type:** integrate-dependency
  - **integration effort:** M
  - **expected impact:** High
  - **status:** validating
  - **aps link (optional):** OPAE / OPAG

- **source repo + link:** `protectai/llm-guard` —
  https://github.com/protectai/llm-guard
  - **what to borrow/adopt:** pluggable prompt/output scanning taxonomy
    (injection, leakage, unsafe content)
  - **adopt type:** borrow-pattern
  - **integration effort:** S
  - **expected impact:** Med
  - **status:** candidate
  - **aps link (optional):** IORISK-001..003

- **source repo + link:** `confident-ai/deepeval` —
  https://github.com/confident-ai/deepeval
  - **what to borrow/adopt:** CI-native eval ergonomics and regression baselines
    for safety checks
  - **adopt type:** integrate-dependency
  - **integration effort:** M
  - **expected impact:** High
  - **status:** candidate
  - **aps link (optional):** EVAL-001..005

- **source repo + link:** `bluewave-labs/verifywise` —
  https://github.com/bluewave-labs/verifywise
  - **what to borrow/adopt:** compliance crosswalk + evidence-linked reporting
    UX
  - **adopt type:** copy-ux
  - **integration effort:** M
  - **expected impact:** High
  - **status:** candidate
  - **aps link (optional):** CEWS / COMPLY / TRUST

- **source repo + link:** `invariantlabs-ai/invariant` —
  https://github.com/invariantlabs-ai/invariant
  - **what to borrow/adopt:** rule-based assertion layer for agent actions with
    contextual checks
  - **adopt type:** borrow-pattern
  - **integration effort:** M
  - **expected impact:** High
  - **status:** candidate
  - **aps link (optional):** CPOL-001..003

- **source repo + link:** `trylonai/gateway` —
  https://github.com/trylonai/gateway
  - **what to borrow/adopt:** LLM firewall gateway architecture for centralized
    interception/enforcement/observability
  - **adopt type:** borrow-pattern
  - **integration effort:** L
  - **expected impact:** Med
  - **status:** candidate
  - **aps link (optional):** GATE-001..003

### Desloppify review — overlap & architecture notes

- **source repo + link:** `peteromallet/desloppify` —
  https://github.com/peteromallet/desloppify
  - **what to borrow/adopt:** CI architecture-contract gate pattern
    (`import-linter`) to enforce layer boundaries in automation
  - **adopt type:** borrow-pattern
  - **integration effort:** S
  - **expected impact:** High
  - **status:** candidate
  - **aps link (optional):** ARCHCFG, OPAG, CMDSAF
  - **overlap with existing Anvil services:**
    - Overlaps with architecture-safety + architecture-config-validation goals
      (boundary integrity)
    - Complements (does not replace) OPA/Rego gate evaluation by adding static
      import-level contracts
  - **architecture notes / anti-frankenstein guardrails:**
    - Keep as a dedicated preflight check in check pipeline, not a second policy
      engine
    - Define one canonical failure schema so import-contract failures render
      through existing diagnostics
    - Avoid tool-sprawl by wrapping via `ArchitectureContractPort` in runtime
  - **additional patterns to borrow/adopt:**
    - **detector registry metadata model:**
      - **what to borrow/adopt:** Detector registry metadata model
        (tier/severity/action-type/judgment-needed) as a single source of truth
      - **adopt type:** borrow-pattern
      - **integration effort:** M
      - **expected impact:** High
      - **status:** candidate
      - **aps link (optional):** OPAG, EVAL, IORISK, ATC, PATT
      - **overlap with existing Anvil services:**
        - Overlaps with warning categories and policy-linked remediation in
          OPAG/EVAL
        - Can unify divergent detector metadata currently spread across
          modules/checks
      - **architecture notes / anti-frankenstein guardrails:**
        - Create one canonical `FindingTypeRegistry` package used by
          CLI/MCP/dashboard
        - Ban ad-hoc detector enums outside registry (lint rule + contract
          tests)
        - Keep OPA policy IDs and detector IDs separate; bridge them via
          explicit mapping table

- **source repo + link:** `peteromallet/desloppify` —
  https://github.com/peteromallet/desloppify
  - **what to borrow/adopt:** Suspect-drop guard (prevent auto-resolving when
    detector counts collapse unexpectedly)
  - **adopt type:** borrow-pattern
  - **integration effort:** S
  - **expected impact:** High
  - **status:** candidate
  - **aps link (optional):** DRIFT, EVAL, POLVAL
  - **overlap with existing Anvil services:**
    - Aligns with drift-reporting intent and trust-preserving signal quality
    - Complements suppression lifecycle by preventing silent disappearance from
      scanner regressions
  - **architecture notes / anti-frankenstein guardrails:**
    - Implement in one place: result reconciliation layer (not per-detector)
    - Emit explicit `integrity_warning` event to Kindling/observability path
    - Add tunable threshold in config with conservative defaults

- **source repo + link:** `peteromallet/desloppify` —
  https://github.com/peteromallet/desloppify
  - **what to borrow/adopt:** Stale-wontfix revalidation loop (time-decayed
    exception debt)
  - **adopt type:** borrow-pattern
  - **integration effort:** M
  - **expected impact:** Med
  - **status:** candidate
  - **aps link (optional):** SUPP, POLLC, COMPLY
  - **overlap with existing Anvil services:**
    - Strong overlap with suppressions, policy lifecycle, and compliance
      evidence requirements
    - Could consolidate exception debt handling currently split across
      suppression + reporting workflows
  - **architecture notes / anti-frankenstein guardrails:**
    - Reuse existing suppression data model; do not create a second exception
      store
    - Model as lifecycle transitions (`active` -> `stale-review` -> `expired`)
      in one state machine
    - Ensure all transitions are auditable and surfaced in compliance exports

- **source repo + link:** `peteromallet/desloppify` —
  https://github.com/peteromallet/desloppify
  - **what to borrow/adopt:** Packaging smoke gate pattern (build/install/help
    command sanity in CI)
  - **adopt type:** borrow-pattern
  - **integration effort:** S
  - **expected impact:** Med
  - **status:** candidate
  - **aps link (optional):** CLIH, MCPH, RT
  - **overlap with existing Anvil services:**
    - Partial overlap with existing CI hardening work; mostly fills
      release-integrity gap
  - **architecture notes / anti-frankenstein guardrails:**
    - Keep this as release pipeline quality gate, not runtime feature
    - Centralize in one reusable CI workflow template across packages

- **source repo + link:** `peteromallet/desloppify` —
  https://github.com/peteromallet/desloppify
  - **what to borrow/adopt:** Queue-first operator loop (`scan` -> `plan` ->
    `next` -> `resolve`) for deterministic progression
  - **adopt type:** borrow-pattern
  - **integration effort:** M
  - **expected impact:** Med
  - **status:** candidate
  - **aps link (optional):** OPAG, EVAL, DASHOPS
  - **overlap with existing Anvil services:**
    - Overlaps with APS + emerging GH Projects orchestration experiments
    - Could conflict with APS flow if treated as a separate planning system
  - **architecture notes / anti-frankenstein guardrails:**
    - Keep APS as source-of-truth planning layer; use queue loop as execution UX
      only
    - Enforce APS ID linkage on queue items to avoid parallel backlogs
    - Prefer adapter into existing `next item` selection logic rather than a
      separate planner

### Cross-cutting integration constraints (to avoid Frankenstein outcomes)

1. **Single source of truth per concern**
   - Planning truth: APS/GH hybrid resolver
   - Policy truth: OPA policy packs
   - Exception truth: suppression lifecycle store
   - Evidence truth: compliance evidence workspace

2. **Adopt patterns, not product semantics**
   - Borrow mechanics (contracts, guards, queues), not desloppify scoring
     philosophy wholesale.

3. **Port-and-adapter rule for every borrowed mechanic**
   - New external checks must implement Anvil ports/contracts before entering
     runtime.

4. **No duplicate state machines**
   - Reuse existing suppression/policy lifecycle states; never add shadow status
     models.

5. **Canonical diagnostic envelope**
   - All new check families must emit the same diagnostic schema used by
     CLI/MCP/dashboard.

---

## 2026-05-13

- **source repo + link:** `preloop/preloop` — https://github.com/preloop/preloop
  - **what to borrow/adopt:** Transparent agent onboarding via config-level
    wrapping — zero SDK changes, rewrites existing agent configs so Anvil
    governance applies without touching source code. The
    `preloop agents discover` pattern is the right DX bar for Anvil adoption:
    agents shouldn't need to opt in.
  - **adopt type:** copy-ux
  - **integration effort:** M
  - **expected impact:** High
  - **status:** candidate
  - **architecture notes / anti-frankenstein guardrails:** Keep config-rewriting
    as a reversible onboarding shim — do not couple it to the policy engine
    core. Define an explicit undo path. Do not intercept configs that Anvil
    cannot fully interpret yet.

- **source repo + link:** `Justin0504/Aegis` —
  https://github.com/Justin0504/Aegis
  - **what to borrow/adopt:** Explicit kill-switch primitive for runaway agents
    (emergency halt) + one-liner instrumentation with zero agent changes
    required. Kill switch is an undervalued enterprise procurement checkbox —
    explicit emergency halt is a feature buyers ask for by name.
  - **adopt type:** borrow-pattern
  - **integration effort:** S
  - **expected impact:** High
  - **status:** candidate
  - **architecture notes / anti-frankenstein guardrails:** Kill switch must be a
    first-class, tested code path — not a last-minute flag. Route it through the
    same enforcement bus as policy violations; do not add a separate halt
    mechanism outside the main gate lifecycle.

- **source repo + link:** `luckyPipewrench/pipelock` —
  https://github.com/luckyPipewrench/pipelock
  - **what to borrow/adopt:** 17 built-in default rule packs for MCP security
    (destructive ops, credential access, reverse shells, persistence, encoded
    commands, DLP, SSRF, prompt injection) — a "secure by default" starter
    ruleset enterprise devs immediately recognise as covering their threat model
    without custom policy writing.
  - **adopt type:** borrow-pattern
  - **integration effort:** S
  - **expected impact:** Med
  - **status:** candidate
  - **architecture notes / anti-frankenstein guardrails:** Map pipelock's 17
    rules to Anvil's OWASP Agentic Top 10 coverage matrix before shipping.
    Deliver as an opt-in starter pack, not a hardcoded default, so teams can
    disable rules they don't need without forking policy config.

- **source repo + link:** `joemunene-by/ghostguard` —
  https://github.com/joemunene-by/ghostguard
  - **what to borrow/adopt:** Tiered enforcement pipeline design — fast static
    rules first, pattern scan second, anomaly/rate-limit third, optional
    LLM-judge last. Correct latency/safety tradeoff: expensive LLM evaluation
    only runs when cheap tiers pass.
  - **adopt type:** borrow-pattern
  - **integration effort:** M
  - **expected impact:** High
  - **status:** candidate
  - **architecture notes / anti-frankenstein guardrails:** Map tiers directly
    onto Anvil's existing enforcement bus as an ordered check pipeline. Do not
    add a separate pipeline executor — plug tiers into the single evaluation
    context so the audit trail captures tier outcomes uniformly. LLM-judge tier
    must be explicitly opt-in with latency budget declared in config.

- **source repo + link:** `AiAgentKarl/agent-audit-trail-mcp` —
  https://github.com/AiAgentKarl/agent-audit-trail-mcp
  - **what to borrow/adopt:** EU AI Act Article 12 compliance framing as a named
    output target for Anvil's audit trail — naming specific Act articles in
    compliance exports is a procurement checklist item. Hash-chaining (SHA-256
    prev-hash) as the tamper-evidence primitive.
  - **adopt type:** borrow-pattern
  - **integration effort:** S
  - **expected impact:** High
  - **status:** candidate
  - **architecture notes / anti-frankenstein guardrails:** Add EU AI Act article
    citation as a metadata annotation on finding exports — do not fork the audit
    trail store. Reuse existing hash-chain schema; define article citations as
    an optional structured field so it degrades gracefully for non-EU buyers.

- **source repo + link:** `HeadyZhang/agent-audit` —
  https://github.com/HeadyZhang/agent-audit
  - **what to borrow/adopt:** Static security scanner for LLM agent _code_
    (shift-left) — tool-boundary taint tracking, prompt injection detection, MCP
    config auditing, secret detection. 53 rules mapped to OWASP Agentic Top 10.
    CI gate via `--severity high`. Creates a developer-first entry point
    separate from runtime enforcement.
  - **adopt type:** borrow-pattern
  - **integration effort:** S
  - **expected impact:** Med
  - **status:** candidate
  - **architecture notes / anti-frankenstein guardrails:** Implement as a
    distinct `anvil scan` subcommand (static, pre-commit/CI) clearly separated
    from `anvil gate` (runtime enforcement). Same buyer, two touchpoints — do
    not merge. Ensure findings from `scan` and `gate` share the same diagnostic
    envelope schema.

- **source repo + link:** `Steward` (Rust forum) —
  https://users.rust-lang.org/t/steward-contract-driven-governance-engine-for-ai-systems/137079
  - **what to borrow/adopt:** Three-verdict output format (PROCEED / ESCALATE /
    BLOCKED) with evidence chain per decision + pipe-friendly CLI as a
    first-class design goal. A clean API contract that CI pipelines can consume
    without parsing prose.
  - **adopt type:** borrow-pattern
  - **integration effort:** S
  - **expected impact:** High
  - **status:** candidate
  - **architecture notes / anti-frankenstein guardrails:** Adopt three-verdict
    semantic as Anvil's canonical gate output — do not invent new verdict terms.
    ESCALATE must have a defined human approval workflow (not just a log entry).
    Pipe-friendly output requires a stable machine-readable format (JSON, not
    coloured CLI text) as the primary CI consumer path.

- **source repo + link:** `Symbiont` — https://docs.symbiont.dev
  - **what to borrow/adopt:** Verifiable agent identity layer — tying agent
    actions to a cryptographically verifiable identity (not just a session ID).
    Closes a gap in Anvil's current audit trail: "which agent identity performed
    this action" is a procurement question Anvil cannot currently answer.
  - **adopt type:** borrow-pattern
  - **integration effort:** M
  - **expected impact:** High
  - **status:** candidate
  - **architecture notes / anti-frankenstein guardrails:** Add agent identity as
    an optional field in the audit event schema first; do not block enforcement
    on identity resolution. Plan for SPIFFE/SVID compatibility to align with
    AGT's identity model and avoid vendor lock-in. Identity resolution must
    degrade gracefully to session-scoped attribution when no strong identity is
    configured.

- **source repo + link:** `provos/ironcurtain` —
  https://github.com/provos/ironcurtain
  - **what to borrow/adopt:** UX pattern of writing policy in plain English and
    compiling it to enforced deterministic rules (LLM compiles, validates
    against generated tests). Directly targets non-security-engineer buyers —
    the CTO/EM who won't write Rego.
  - **adopt type:** copy-ux
  - **integration effort:** S
  - **expected impact:** High
  - **status:** candidate
  - **architecture notes / anti-frankenstein guardrails:** "Compile to policy"
    must produce a reviewable, version-controlled rule artefact before it
    enforces — show the generated rule, don't auto-deploy it. LLM-compiled rules
    must pass the same test harness as hand-written ones. Store generated rules
    alongside hand-written policy files; do not treat them as ephemeral.

- **source repo + link:** `open-gitagent/gitagent` —
  https://github.com/open-gitagent/gitagent
  - **what to borrow/adopt:** Git-native agent definition standard with built-in
    segregation of duties (maker/checker role model, conflict enforcement,
    strict handoffs). Framework-agnostic and git-native — the closest public
    design to Anvil's policy-as-code angle from the git workflow direction.
  - **adopt type:** borrow-pattern
  - **integration effort:** S
  - **expected impact:** High
  - **status:** candidate
  - **architecture notes / anti-frankenstein guardrails:** Treat as a policy
    source format Anvil can read and enforce — not a competing governance
    system. Define a one-way import bridge: gitagent agent definitions → Anvil
    policy packs. Do not build a second git-native agent definition format;
    compose with the existing `.anvil/` convention.

- **source repo + link:** `langfuse/langfuse` —
  https://github.com/langfuse/langfuse
  - **what to borrow/adopt:** OpenTelemetry as the standard trace transport for
    AI audit events — emit OTLP-compatible spans so Anvil audit trails plug into
    existing observability stacks (Datadog, Grafana, Honeycomb) without
    requiring a custom dashboard or vendor lock-in. **Validated borrow
    (deep-dive 2026-06-06): a _refs-only governance-decision correlation span_ —
    the `traceparent` join key + pointers to durable Kindling/witness evidence,
    with low-sensitivity routing attributes only — NOT a `GovernanceTraceExport`
    that carries the evidence itself.**
  - **adopt type:** borrow-pattern (clean-room against the open OTel GenAI
    semantic conventions; **not** a Langfuse dependency)
  - **integration effort:** S (contract refinement of EXPORT, not greenfield —
    TRACE already ships the `traceparent` plumbing and redaction layer)
  - **expected impact:** High
  - **status:** validating
  - **deep-dive:**
    [`plans/brainstorms/2026-06-06-langfuse-borrow-assessment.md`](../../plans/brainstorms/2026-06-06-langfuse-borrow-assessment.md)
    — promoted 2026-06-05 from the 2026-05-02 radar; assessed 2026-06-06.
    Disposition **Track → fold into EXPORT**. Decline the Langfuse platform +
    dependency (product breadth, scope exclusion #5).
  - **aps link (optional):** EXPORT (observability-export) · TRACE
    (tracing-foundation) · ADR-035 (three-pipe rule) · ADR-059 (production
    sink); suggested CIB filings in the deep-dive §7.3.
  - **architecture notes / anti-frankenstein guardrails:** OTLP emission must be
    additive and **refs-only** — the span carries pointers to durable evidence
    (Kindling / witness chain), **never the evidence itself** (ADR-035: the
    tracing pipe is ephemeral, never source-of-truth). Define the span schema
    once, aligned to OTel GenAI semconv with `anvil.governance.*` for
    governance-specific fields, via the namespace registry; do not let modules
    invent their own attributes (TRACE R2). Keep export off by default and bound
    to the **single ratified sink** (Azure Monitor + App Insights, ADR-059);
    CLI/daemon stay local-first and never auto-export. A multi-backend "publish
    anywhere" exporter (`exporter_ref`/`destination_ref` as a pluggable matrix)
    is **out** without its own ADR. Redaction-before-export is a tested
    invariant (EXPORT-001 V2).

- **source repo + link:** `jagmarques/asqav-sdk` —
  https://github.com/jagmarques/asqav-sdk
  - **what to borrow/adopt:** Quantum-safe signing narrative (ML-DSA-65 /
    FIPS 204) for long-lived audit trails — "your audit trail holds up in court"
    is a strong enterprise hook. Also: EU AI Act article-mapping baked into
    report output (named article citations in compliance exports).
  - **adopt type:** borrow-pattern
  - **integration effort:** S
  - **expected impact:** High
  - **status:** candidate
  - **architecture notes / anti-frankenstein guardrails:** Plan for algorithm
    agility in the attestation format from day one — do not hardcode SHA-256 in
    the signing layer. Quantum-safe signing can ship as a day-two feature but
    the schema must not preclude it. EU AI Act article mapping is a metadata
    annotation layer, not a schema redesign.

---

## 2026-06-06

- **source repo + link:** `coproduct-opensource/nucleus` —
  https://github.com/coproduct-opensource/nucleus
  - **what to borrow/adopt:** Proof-scope honesty as a first-class evidence
    field. Nucleus pairs every verification output with an explicit statement of
    what the proof does **and does not** establish — e.g. "a green verification
    proves the lineage is authentic and intact; it does NOT prove the agent
    behaved well, that information-flow policy held, or that any computation was
    correct." Anvil should carry an equivalent machine-readable
    `assurance_scope` / `proof_limits` block on provenance and attestation
    exports so verdicts state their own boundaries instead of implying total
    assurance. Secondary borrow: the verifier-readable IFC verdict field model
    (source label, sink class, flow verdict, proof artifact ref) as a
    vocabulary, not an engine.
  - **adopt type:** borrow-pattern
  - **integration effort:** S
  - **expected impact:** Med
  - **status:** candidate
  - **aps link (optional):** provenance / attestation export schema (fold into
    existing audit-trail + signing work; do not stand up a separate module)
  - **overlap with existing Anvil services:**
    - Heavy overlap with already-tracked candidates — this entry deliberately
      narrows to the one non-duplicative borrow:
      - keyless OIDC→SPIFFE identity → already covered by `Symbiont`
        (2026-05-13)
      - signed provenance / tamper-evidence → already covered by `asqav-sdk` and
        `AiAgentKarl/agent-audit-trail-mcp` hash-chaining (2026-05-13)
      - taint / tool-boundary information-flow tracking → already covered by
        `HeadyZhang/agent-audit` (2026-05-13)
      - machine-readable verdict contract → already covered by `Steward`
        three-verdict format (2026-05-13)
    - The genuinely new contribution is the **proof-limit disclosure
      discipline**, which strengthens Anvil's provenance/audit honesty posture
      and is a procurement trust signal (auditors distrust over-claiming).
  - **architecture notes / anti-frankenstein guardrails:**
    - Borrow the evidence _shape and language_, clean-room. Do NOT take the
      runtime as a dependency: at 2026-06-06 the repo is ~16 stars and largely
      alpha/unwired — constitutional kernel "not yet wired into the runtime,"
      verifier service "not hosted," npm/WASM verifier "publish-gated,"
      `nucleus-policy` crate an orphan, and "vendor-agnostic" undercut by
      hardcoded vendor hostnames and a runner pinned to one assistant CLI. Only
      Tier 0 (`nucleus audit`, static config scan) is usable today.
    - Add `assurance_scope` as an additive, optional field on the existing
      attestation/audit envelope — do not fork the provenance store or invent a
      second evidence schema.
    - State limits in declarative, deterministic terms (what was checked, the
      bound, what was assumed) — aligns with Anvil's deterministic +
      warnings-over-blocks posture.
  - **licence note:** dual MIT / Apache-2.0 (Morgan's summary said MIT-only) —
    clean-room borrow of schema/wording carries no licence friction.
