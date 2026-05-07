# Borrow / Adopt Candidates

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
    in-flight rewind; sharded permanent metadata branch) linked to user
    commits via a single `Entire-Checkpoint:` trailer. Full write-up at
    [`docs/architecture/references/entire-branch-sidecar.md`](../architecture/references/entire-branch-sidecar.md).
  - **adopt type:** borrow-pattern
  - **integration effort:** M
  - **expected impact:** High
  - **status:** candidate
  - **aps link (optional):** kindling-capture, council outputs, APS history
  - **overlap with existing Anvil services:**
    - kindling-capture (PostToolUse hook) currently writes to an external
      store — git-native sidecar would replace that path
    - Council review findings and APS work-item history both want a place to
      live without polluting main tree / `plans/`
  - **architecture notes / anti-frankenstein guardrails:**
    - Use `refs/anvil/*` namespace (not `refs/heads/anvil/*`) so `git branch`
      output stays clean
    - Adopt explicit version anchor in ref names (`refs/anvil/<feature>/v1`)
      from day one
    - Define GC policy for shadow refs upfront (Entire's docs do not)
    - Define `git commit --amend` / rebase behavior (re-run hook or warn)
    - Require trailer to be optional and strippable; do not block commits
      that lack it
    - Add security/privacy review for what transcripts contain before
      enabling default push

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
  - **aps link (optional):** OPAE-006..010

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
