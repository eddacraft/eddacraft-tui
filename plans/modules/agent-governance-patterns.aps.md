<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Agent Governance Patterns

| ID | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| AGOV  | —     | Medium   | Draft  |

**Last reviewed:** 2026-04-26

> **Audit note (2026-04-26):** Tier C (parking lot, post-launch). Council C
> recommended dissolving this module into CPACKS + `crates/anvil-checks` —
> that was overreach. AGOV is depended on by other live planning modules:
> CPACKS cites AGOV-001/006/007 as upstream signal producers (trust scores,
> hash-chained audit, capability manifest); MDGOV cites AGOV-007 for M3
> capability-aware governance; the 2026-04-08 Language & Coverage spec
> names AGOV-007 in the M3 dependency chain. Dissolving AGOV would break
> all three.
>
> **Real overlap is narrow:** only **AGOV-002** (HIPAA/PCI-DSS compliance
> pack stubs) overlaps CPACKS, which already covers SOC 2/ISO/GDPR/OWASP/AI
> packs. AGOV-001/003/004/005/006/007 are distinct signal producers
> (trust scoring, destructive-pattern detection, change-volume threshold,
> metadata-secret scan, hash-chained audit, capability declaration) — they
> belong in AGOV, not CPACKS.
>
> **Earlier audit pass conflated archived planning modules with archived
> components.** EMBER and EDDA *planning modules* are archived because
> their work-item lists completed; the components are live code in
> `packages/edda-stack/src/ember/` and `packages/edda-stack/src/edda/`.
> AGOV-001 (trust scoring) and AGOV-006 (hash-chained audit) extend live
> code, not retired modules.
>
> **Rescope work pending** (tracked separately, see followup list):
> 1. Migrate AGOV-002 → CPACKS as additional compliance pack stubs;
>    retitle the AGOV slot or remove (CPACKS-NNN takes ownership).
> 2. Retarget all task `Scope`/`Files` paths to Rust crates:
>    - Gate checks (destructive-pattern, change-volume, metadata-secret,
>      capability-declaration) → `crates/anvil-checks/src/checks/`
>    - Trust-score and capability-manifest schemas →
>      `crates/anvil-kernel-types`
>    - CLI subcommands (`audit verify`, `capability validate`) →
>      `crates/anvil-cli/src/commands/`
>    - Compliance Rego packs (kept after AGOV-002 migration: only as
>      cross-reference) → `crates/anvil-policy/policies/compliance/`
> 3. Rewire Interfaces "Depends on" block to live components, not
>    archived planning modules:
>    - EMBER → `packages/edda-stack/src/ember/` (TS) or future Rust port
>    - EDDA → `packages/edda-stack/src/edda/` (TS) or future Rust port
>    - OPAE → `crates/anvil-policy` (live)
>    - POLVAL → still planning, retains module-level dependency
> 4. Confirm consumer refs in CPACKS, MDGOV, and the L&C spec stay
>    valid after path retargeting.
>
> Not launch-blocker. Promote to Ready only after CPACKS POLVAL prep
> work and a product decision on which signal producers actually ship.

## Purpose

Adopt governance patterns from Microsoft's Agent Governance Toolkit and the
anti-slop project into Anvil's gate-check and policy-evaluation surface. These
patterns address the growing need to govern AI-generated contributions: trust
scoring, destructive-operation detection, change-volume thresholds, credential
leakage in metadata, compliance template packs, hash-chained audit trails, and
capability declaration models. Together they strengthen Anvil's position as a
comprehensive governance layer for teams using AI coding tools.

## In Scope

- Trust scoring model for contributors and AI tools (extends Ember confidence)
- Pre-built compliance policy packs (extends POLVAL)
- Destructive operation pattern detection as a gate check
- Change volume/velocity threshold gate check
- Credential and PII scanning in PR metadata (commit messages, descriptions)
- Cryptographic hash chaining for Edda audit trail entries
- Capability declaration model for AI tools and CI pipelines

## Out of Scope

- ❌ Runtime agent sandboxing or execution rings (Anvil is static analysis)
- ❌ Agent-to-agent encrypted mesh or inter-agent trust protocols
- ❌ Kill switches or real-time process termination signals
- ❌ Framework wrapping (`kernel.wrap(agent)` style runtime interception)
- ❌ GitHub-specific anti-abuse (account age, fork rate, spam username detection)
- ❌ OPA/Rego or Cedar policy engine implementation (covered by OPAE)

## Interfaces

**Depends on:**

- EMBER — Confidence scoring model (AGOV-001 extends it)
- POLVAL — Policy pack validation infrastructure (AGOV-002 builds on it)
- EDDA — Provenance service and audit trail (AGOV-006 extends it)
- OPAE — OPA policy executor (AGOV-003, AGOV-004 ship as OPA policies)

**Exposes:**

- Gate checks: `destructive-pattern.check.ts`, `change-volume.check.ts`,
  `metadata-secret.check.ts`, `capability-declaration.check.ts`
- Policy packs: `library/compliance/hipaa/`, `library/compliance/pci-dss/`,
  `library/compliance/gdpr/`, `library/compliance/soc2/`
- CLI: `anvil policy install --pack <name>`
- Schema: `trust-score.ts`, `capability-manifest.ts`
- Edda extension: hash-chained provenance entries

## Acceptance Criteria

- [ ] Contributors and AI tools receive a computed trust score (0–1000) based on
      gate-pass history, and the score influences which gate checks apply
- [ ] At least four compliance packs (HIPAA, PCI-DSS, GDPR, SOC 2) are
      installable via `anvil policy install --pack <name>`
- [ ] Destructive patterns (SQL DROP/TRUNCATE, `rm -rf`, `chmod 777`) detected
      in diffs produce gate warnings
- [ ] PRs exceeding configurable change-volume thresholds trigger review
      escalation
- [ ] Secrets and PII detected in commit messages and PR descriptions produce
      gate findings
- [ ] Edda audit trail entries are hash-chained with SHA-256 for tamper detection
- [ ] AI tools can declare capabilities via a manifest, and gate checks validate
      that changes stay within declared scope

---

## Work Items

### Phase A: Detection & Gate Checks

#### AGOV-001: Trust scoring model

- **Intent:** Assign a continuous trust score (0–1000) to contributors and AI
  tools based on their gate-compliance history, enabling risk-proportional
  enforcement.
- **Expected Outcome:** A scoring service that computes and persists trust scores
  per contributor/tool identity, with configurable tier thresholds that map to
  gate-check strictness levels. Scores decay on violations and recover on
  compliant contributions.
- **Scope:** `packages/edda-stack/src/ember/`, `packages/anvil/runtime/src/gate/`
- **Non-scope:** Authentication or identity provider integration
- **Files:**
  - `packages/edda-stack/src/ember/trust-score.ts`
  - `packages/edda-stack/src/ember/trust-score.test.ts`
  - `packages/anvil/runtime/src/gate/trust-context.ts`
- **Dependencies:** EMBER (confidence model)
- **Validation:** `nx test edda-stack --testNamePattern="trust-score"`
- **Confidence:** medium

#### AGOV-002: Compliance template packs

- **Intent:** Ship pre-built, installable compliance policy packs so teams can
  adopt industry-standard governance with a single command.
- **Expected Outcome:** At least four compliance packs (HIPAA, PCI-DSS, GDPR,
  SOC 2) containing OPA policies, each installable via
  `anvil policy install --pack <name>`. Packs include metadata, versioning, and
  dependency declarations per POLVAL conventions.
- **Scope:** `core/src/gate/__fixtures__/library/compliance/`,
  `apps/anvil-cli/src/commands/policy.ts`
- **Non-scope:** Custom policy authoring UI; pack marketplace
- **Files:**
  - `core/src/gate/__fixtures__/library/compliance/hipaa/manifest.json`
  - `core/src/gate/__fixtures__/library/compliance/hipaa/*.rego`
  - `core/src/gate/__fixtures__/library/compliance/pci-dss/manifest.json`
  - `core/src/gate/__fixtures__/library/compliance/pci-dss/*.rego`
  - `core/src/gate/__fixtures__/library/compliance/gdpr/manifest.json`
  - `core/src/gate/__fixtures__/library/compliance/gdpr/*.rego`
  - `core/src/gate/__fixtures__/library/compliance/soc2/manifest.json`
  - `core/src/gate/__fixtures__/library/compliance/soc2/*.rego`
  - `apps/anvil-cli/src/commands/policy.ts` (extend with `install --pack`)
- **Dependencies:** POLVAL-002 (pack manifest), POLVAL-003 (pack validator)
- **Validation:** `anvil policy install --pack hipaa --dry-run`
- **Confidence:** high

#### AGOV-003: Destructive operation pattern detection

- **Intent:** Detect destructive operations in code diffs (SQL destruction,
  filesystem wiping, permission escalation) and surface them as gate findings.
- **Expected Outcome:** A gate check that scans diffs for configurable regex
  patterns matching destructive operations. Ships with a default pattern set
  covering SQL (`DROP`, `TRUNCATE`, `DELETE FROM` without `WHERE`), shell
  (`rm -rf`, `chmod 777`, `mkfs`), and infrastructure (`terraform destroy`)
  patterns. Findings are severity-rated based on pattern category.
- **Scope:** `packages/anvil/runtime/src/gate/checks/`
- **Non-scope:** Runtime blocking; patterns for languages beyond SQL/shell/IaC
- **Files:**
  - `packages/anvil/runtime/src/gate/checks/destructive-pattern.check.ts`
  - `packages/anvil/runtime/src/gate/checks/destructive-pattern.check.test.ts`
  - `core/src/gate/__fixtures__/library/security/destructive-patterns.json`
- **Dependencies:** —
- **Validation:** `nx test anvil-runtime --testNamePattern="destructive-pattern"`
- **Confidence:** high

#### AGOV-004: Change volume threshold gate check

- **Intent:** Flag PRs with unusually high change volume — a signal of
  AI-generated bulk submissions — for elevated review.
- **Expected Outcome:** A gate check that evaluates files-changed count,
  lines-added/removed, and single-commit size against configurable thresholds.
  Exceeding thresholds escalates the PR to require additional review. Default
  thresholds: 30 files, 1500 lines added, 500 lines in a single commit.
- **Scope:** `packages/anvil/runtime/src/gate/checks/`
- **Non-scope:** Blocking PRs outright; measuring code quality of bulk changes
- **Files:**
  - `packages/anvil/runtime/src/gate/checks/change-volume.check.ts`
  - `packages/anvil/runtime/src/gate/checks/change-volume.check.test.ts`
- **Dependencies:** —
- **Validation:** `nx test anvil-runtime --testNamePattern="change-volume"`
- **Confidence:** high

#### AGOV-005: Credential and PII scanning in PR metadata

- **Intent:** Extend secret detection beyond source code to catch credentials and
  PII leaked into commit messages, branch names, and PR descriptions.
- **Expected Outcome:** A gate check that scans commit messages and PR
  description text for credential patterns (API keys, tokens, passwords) and PII
  patterns (email addresses, phone numbers, national ID formats). Produces
  critical-severity findings for credential matches and major-severity for PII.
- **Scope:** `packages/anvil/runtime/src/gate/checks/`
- **Non-scope:** Scanning code diffs (existing secret checks cover that)
- **Files:**
  - `packages/anvil/runtime/src/gate/checks/metadata-secret.check.ts`
  - `packages/anvil/runtime/src/gate/checks/metadata-secret.check.test.ts`
- **Dependencies:** —
- **Validation:** `nx test anvil-runtime --testNamePattern="metadata-secret"`
- **Confidence:** high

### Phase B: Audit & Compliance Infrastructure

#### AGOV-006: Hash-chained audit trail

- **Intent:** Add cryptographic integrity to Edda's audit trail so that
  tampering with historical gate evaluations is detectable.
- **Expected Outcome:** Each audit-trail entry includes a SHA-256 hash of its
  content concatenated with the previous entry's hash, forming an immutable
  chain. A verification command (`anvil audit verify`) walks the chain and
  reports any breaks. Compatible with existing Edda provenance service.
- **Scope:** `packages/edda-stack/src/edda/`
- **Non-scope:** External blockchain anchoring; distributed consensus
- **Files:**
  - `packages/edda-stack/src/edda/hash-chain.ts`
  - `packages/edda-stack/src/edda/hash-chain.test.ts`
  - `apps/anvil-cli/src/commands/audit.ts` (extend with `verify` subcommand)
- **Dependencies:** EDDA-010 (provenance service)
- **Validation:** `nx test edda-stack --testNamePattern="hash-chain"`
- **Confidence:** medium

### Phase C: Capability Model

#### AGOV-007: Capability declaration model

- **Intent:** Allow AI tools and CI pipelines to declare their intended
  capabilities via a manifest, so that gate checks can validate contributions
  stay within declared scope.
- **Expected Outcome:** A `capability-manifest.json` schema that declares
  allowed file paths, operation types (read/write/delete), and resource scopes.
  A gate check validates that PR changes fall within the manifest's declared
  capabilities. Violations produce major-severity findings. CLI support via
  `anvil capability validate`.
- **Scope:** `packages/anvil/runtime/src/gate/checks/`,
  `apps/anvil-cli/src/commands/`
- **Non-scope:** Runtime enforcement; capability negotiation protocols
- **Files:**
  - `packages/anvil/runtime/src/gate/checks/capability-declaration.check.ts`
  - `packages/anvil/runtime/src/gate/checks/capability-declaration.check.test.ts`
  - `packages/anvil/core/src/config/capability-manifest.ts`
  - `packages/anvil/core/src/config/capability-manifest.test.ts`
  - `apps/anvil-cli/src/commands/capability.ts`
- **Dependencies:** AGOV-001 (trust score informs capability trust)
- **Validation:** `nx test anvil-runtime --testNamePattern="capability-declaration"`
- **Confidence:** low

---

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Destructive pattern regex produces false positives on legitimate test fixtures or migration scripts | Ship with a suppression mechanism (inline `// anvil-ignore destructive-pattern`) and tuneable severity |
| Change volume thresholds penalise legitimate large refactors | Default to warning severity, not blocking; allow per-PR override via `anvil.config` |
| Compliance packs become stale as regulations evolve | Version packs independently; document update cadence in pack metadata |
| Trust scoring creates perverse incentives (gaming scores) | Score computation is internal, not exposed to contributors; decay prevents score hoarding |
| Capability manifests add friction to AI tool adoption | Make capability checks opt-in; provide `anvil capability init` to generate manifests from existing PR history |

## Decisions

D-AGOV-001: Single module vs. multiple modules

- **Rationale:** These patterns share a common theme (governing AI-tool
  contributions) and have cross-dependencies (trust score feeds capability
  validation). A single module keeps the dependency graph simple.
- **Alternatives:** Separate modules per pattern (TRUST, COMPLY-PACKS, DESTRUCT,
  etc.)
- **Trade-offs:** Larger module, but coherent narrative and easier prioritisation.

D-AGOV-002: Trust score range 0–1000 (not 0–100)

- **Rationale:** Mirrors Microsoft AGT's scale, provides granularity for tier
  boundaries without floating-point complexity.
- **Alternatives:** 0–100 (simpler), 0.0–1.0 (normalised)
- **Trade-offs:** Larger range is harder to reason about intuitively but more
  flexible for future tier subdivision.

## Notes

### Provenance

Patterns sourced from:

- **Microsoft Agent Governance Toolkit** —
  trust scoring, capability model, hash-chained audit, compliance packs,
  destructive operation detection
  (https://github.com/microsoft/agent-governance-toolkit)
- **anti-slop** —
  change volume thresholds, PR metadata scanning, AI-content heuristics
  (https://github.com/peakoss/anti-slop)

### Trust Score Tiers

```text
900–1000  Verified      → Minimal gate checks, fast-path approval
700–899   Trusted       → Standard gate checks
500–699   Standard      → Full gate checks (default for new contributors)
300–499   Probationary  → Full checks + elevated review requirement
  0–299   Untrusted     → Full checks + manual approval required
```

### Capability Manifest Schema (sketch)

```json
{
  "tool": "copilot",
  "version": "1.0",
  "capabilities": {
    "paths": ["src/**", "tests/**"],
    "operations": ["read", "write"],
    "excludedPaths": ["infra/**", ".env*", "**/schema.sql"],
    "maxFilesPerPR": 25,
    "maxLinesPerPR": 1000
  }
}
```
