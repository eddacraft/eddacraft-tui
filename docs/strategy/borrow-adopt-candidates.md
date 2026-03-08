# Borrow / Adopt Candidates

Purpose: track concrete ideas to borrow, adapt, or integrate from external projects into Anvil.

## Workflow

- Add new candidates under the current date section (`## YYYY-MM-DD`).
- Keep entries short and decision-ready.
- Move status forward as validation happens.

Entry format:

- **source repo + link:** `owner/repo` — `https://github.com/owner/repo`
- **what to borrow/adopt:** specific capability/pattern
- **adopt type:** `borrow-pattern` | `integrate-dependency` | `copy-ux`
- **integration effort:** `S` | `M` | `L`
- **expected impact:** `Low` | `Med` | `High`
- **status:** `candidate` | `validating` | `adopted` | `rejected`
- **aps link (optional):** module/task ID

---

## 2026-03-08

- **source repo + link:** `guardrails-ai/guardrails` — https://github.com/guardrails-ai/guardrails
  - **what to borrow/adopt:** validator-hub model for composable input/output safety checks as reusable policy packs
  - **adopt type:** borrow-pattern
  - **integration effort:** M
  - **expected impact:** High
  - **status:** candidate
  - **aps link (optional):** OPAE-006..010

- **source repo + link:** `open-policy-agent/opa` — https://github.com/open-policy-agent/opa
  - **what to borrow/adopt:** policy-as-code decoupled decision layer for auditable runtime governance
  - **adopt type:** integrate-dependency
  - **integration effort:** M
  - **expected impact:** High
  - **status:** validating
  - **aps link (optional):** OPAE / OPAG

- **source repo + link:** `protectai/llm-guard` — https://github.com/protectai/llm-guard
  - **what to borrow/adopt:** pluggable prompt/output scanning taxonomy (injection, leakage, unsafe content)
  - **adopt type:** borrow-pattern
  - **integration effort:** S
  - **expected impact:** Med
  - **status:** candidate
  - **aps link (optional):** IORISK-001..003

- **source repo + link:** `confident-ai/deepeval` — https://github.com/confident-ai/deepeval
  - **what to borrow/adopt:** CI-native eval ergonomics and regression baselines for safety checks
  - **adopt type:** integrate-dependency
  - **integration effort:** M
  - **expected impact:** High
  - **status:** candidate
  - **aps link (optional):** EVAL-001..005

- **source repo + link:** `bluewave-labs/verifywise` — https://github.com/bluewave-labs/verifywise
  - **what to borrow/adopt:** compliance crosswalk + evidence-linked reporting UX
  - **adopt type:** copy-ux
  - **integration effort:** M
  - **expected impact:** High
  - **status:** candidate
  - **aps link (optional):** CEWS / COMPLY / TRUST

- **source repo + link:** `invariantlabs-ai/invariant` — https://github.com/invariantlabs-ai/invariant
  - **what to borrow/adopt:** rule-based assertion layer for agent actions with contextual checks
  - **adopt type:** borrow-pattern
  - **integration effort:** M
  - **expected impact:** High
  - **status:** candidate
  - **aps link (optional):** CPOL-001..003

- **source repo + link:** `trylonai/gateway` — https://github.com/trylonai/gateway
  - **what to borrow/adopt:** LLM firewall gateway architecture for centralized interception/enforcement/observability
  - **adopt type:** borrow-pattern
  - **integration effort:** L
  - **expected impact:** Med
  - **status:** candidate
  - **aps link (optional):** GATE-001..003
