<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Agent Governance Patterns

| ID | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| AGOV  | —     | Medium   | Draft  |

**Last reviewed:** 2026-07-11 (post-POLRESET downstream coherence review —
`plans/reviews/2026-07-11-polreset-downstream-coherence.md`: the module's own
rescope list, pending since 2026-04-26, is now **executed** — see the struck
items in the followup list below).

> **Reset posture (POLRESET-010 / ADR-098, 2026-07-04; gate restated
> 2026-07-11):** post-first-slice expansion — not a prerequisite for first
> policy value. The old scheduling condition ("after the starter pack and
> report-only policy CI exist") is **met** — both shipped (POLRESET-007
> #3167, POLRESET-008 #3170). The live gate is the one at the end of this
> note block: a **product decision on which signal producers actually ship**,
> plus CPACKS coordination. Coordinated by
> [`POLRESET`](../archive/modules/policy-value-enforcement-reset.aps.md).

> **Policy-solution validation (2026-06-24):** AGOV remains Draft. Its signal
> producers should feed `crates/anvil-policy` / `crates/anvil-checks` and Rego
> packs evaluated by the POLENG regorus facade. AGOV-002's pack-stub overlap
> with CPACKS is still the only policy-pack overlap; do not add OPA runtime work
> here.
>
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
> **Rescope work list (executed 2026-07-11):**
> 1. ~~Migrate AGOV-002~~ — **done**: AGOV-002 removed as superseded. The
>    install mechanism shipped in Rust as OPAE-004
>    (`anvil policy install <PACK-ID>`); broad compliance-pack content is
>    owned by CPACKS behind its CPACKS-008 expansion gate.
> 2. ~~Retarget Scope/Files paths~~ — **done**: gate checks →
>    `crates/anvil-checks` + the `patterns/` registry (anti-pattern family
>    flow); trust-score and capability-manifest schemas →
>    `crates/anvil-kernel-types`; CLI subcommands →
>    `crates/anvil-cli/src/commands/`. (The old paths
>    `packages/anvil/runtime/src/gate/`, `apps/anvil-cli/`, and `core/` do
>    not exist in the repo.)
> 3. ~~Rewire Depends-on to live components~~ — **done** (see Interfaces);
>    note the JS/TS workspace is in deliberate retirement, so the Ember/Edda
>    TS components are reference semantics for a Rust successor, not
>    integration targets.
> 4. Confirm consumer refs in CPACKS, MDGOV, and the L&C spec stay valid
>    after path retargeting — **open** (consumers cite AGOV item IDs, which
>    are unchanged except AGOV-002; CPACKS's pack ownership already matches).
>
> Not launch-blocker. Promote to Ready only after CPACKS coordination and a
> product decision on which signal producers actually ship — that decision is
> the module's live gate.

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
- ❌ Policy engine implementation — regorus is the single shipped engine
  (ADR-040; the OPA subprocess path was deleted by ADR-098 PR-C)

## Interfaces

**Depends on:**

<!-- Rewired 2026-07-11 to live components (rescope item 3). -->
- EMBER confidence model — live TS code (`packages/edda-stack/src/ember/`),
  but the JS/TS workspace is in deliberate retirement: AGOV-001 takes its
  semantics as reference and lands the schema in `crates/anvil-kernel-types`
- POLVAL — pack validation infrastructure (Done —
  `crates/anvil-policy-engine/src/pack/`)
- EDDA provenance / audit trail — the shipped Rust equivalent is the
  **anvil-witness chain** (ADR-037, `WitnessLine`/`verify_chain_dag`);
  AGOV-006 must be re-evaluated against it (see its delta note)
- `crates/anvil-policy-engine` (regorus facade) — AGOV-003/004 detection
  ships as gate checks in `crates/anvil-checks`, with any Rego expression
  evaluated through the facade (the old "OPAE OPA policy executor" wording
  predated the reset)

**Exposes:**

- Gate checks: destructive-pattern, change-volume, metadata-secret, and
  capability-declaration checks in `crates/anvil-checks` (anti-pattern
  family flow via the `patterns/` registry)
- Compliance pack content: **moved to CPACKS** (AGOV-002 removed; install UX
  already shipped as OPAE-004 `anvil policy install <PACK-ID>`)
- Schemas: trust-score and capability-manifest types in
  `crates/anvil-kernel-types`
- Audit: hash-chained provenance via the anvil-witness chain (extension, not
  a parallel trail)

## Acceptance Criteria

- [ ] Contributors and AI tools receive a computed trust score (0–1000) based on
      gate-pass history, and the score influences which gate checks apply
- [ ] ~~At least four compliance packs installable~~ — moved to CPACKS
      (CPACKS-008 expansion gate) with AGOV-002's removal; the install UX
      itself shipped as OPAE-004
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
- **Scope:** `crates/anvil-kernel-types/src/` (trust-score schema),
  `crates/anvil-checks/src/` (gate integration)
- **Non-scope:** Authentication or identity provider integration
- **Dependencies:** EMBER confidence semantics as reference (live TS code;
  the JS/TS workspace is retiring, so the score lands Rust-native)
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- trust_score`
  and `cargo test -p eddacraft-anvil-checks -- trust_context`
- **Confidence:** medium

#### AGOV-002: Compliance template packs — REMOVED 2026-07-11 (superseded)

- **Status:** Removed — doubly superseded per the executed rescope list:
  the install mechanism it planned shipped in Rust as **OPAE-004**
  (`anvil policy install <PACK-ID>` + `install --list`), and broad
  compliance-pack **content** (HIPAA/PCI-DSS/GDPR/SOC 2) is owned by
  **CPACKS** behind its CPACKS-008 expansion gate (itself gated on COMPLY's
  evidence-semantics design). Slot retained for ID stability; do not
  resurrect here.

#### AGOV-003: Destructive operation pattern detection

- **Intent:** Detect destructive operations in code diffs (SQL destruction,
  filesystem wiping, permission escalation) and surface them as gate findings.
- **Expected Outcome:** A gate check that scans diffs for configurable regex
  patterns matching destructive operations. Ships with a default pattern set
  covering SQL (`DROP`, `TRUNCATE`, `DELETE FROM` without `WHERE`), shell
  (`rm -rf`, `chmod 777`, `mkfs`), and infrastructure (`terraform destroy`)
  patterns. Findings are severity-rated based on pattern category.
- **Scope:** `crates/anvil-checks/src/` + `patterns/` registry (anti-pattern
  family flow — see `plans/` anti-pattern family authoring conventions)
- **Non-scope:** Runtime blocking; patterns for languages beyond SQL/shell/IaC
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil-checks -- destructive_pattern`
- **Confidence:** high

#### AGOV-004: Change volume threshold gate check

- **Intent:** Flag PRs with unusually high change volume — a signal of
  AI-generated bulk submissions — for elevated review.
- **Expected Outcome:** A gate check that evaluates files-changed count,
  lines-added/removed, and single-commit size against configurable thresholds.
  Exceeding thresholds escalates the PR to require additional review. Default
  thresholds: 30 files, 1500 lines added, 500 lines in a single commit.
- **Scope:** `crates/anvil-checks/src/`
- **Non-scope:** Blocking PRs outright; measuring code quality of bulk changes
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil-checks -- change_volume`
- **Confidence:** high

#### AGOV-005: Credential and PII scanning in PR metadata

- **Intent:** Extend secret detection beyond source code to catch credentials and
  PII leaked into commit messages, branch names, and PR descriptions.
- **Expected Outcome:** A gate check that scans commit messages and PR
  description text for credential patterns (API keys, tokens, passwords) and PII
  patterns (email addresses, phone numbers, national ID formats). Produces
  critical-severity findings for credential matches and major-severity for PII.
- **Scope:** `crates/anvil-checks/src/`
- **Non-scope:** Scanning code diffs (existing secret checks — SEC-008
  named-pattern detection — cover that)
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil-checks -- metadata_secret`
- **Confidence:** high

### Phase B: Audit & Compliance Infrastructure

#### AGOV-006: Hash-chained audit trail

- **Intent:** Add cryptographic integrity to Edda's audit trail so that
  tampering with historical gate evaluations is detectable.
- **Expected Outcome:** Each audit-trail entry includes a SHA-256 hash of its
  content concatenated with the previous entry's hash, forming an immutable
  chain, with a verification command that walks the chain and reports breaks.
  **Delta note (2026-07-11):** the Rust workspace already ships exactly this
  shape — the anvil-witness chain (`WitnessLine`/`verify_chain_dag`, ADR-037,
  GITGOV). Before any work, re-evaluate this item against it: the residual
  is at most extending witness coverage to the governance events AGOV cares
  about, not building a hash chain.
- **Scope:** `crates/anvil-witness/src/` (extension),
  `crates/anvil-cli/src/commands/` (verify surface)
- **Non-scope:** External blockchain anchoring; distributed consensus
- **Dependencies:** anvil-witness chain (shipped)
- **Validation:** `cargo test -p eddacraft-anvil-witness -- governance_events`
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
- **Scope:** `crates/anvil-kernel-types/src/` (capability-manifest schema),
  `crates/anvil-checks/src/` (gate validation),
  `crates/anvil-cli/src/commands/` (`anvil capability validate`)
- **Non-scope:** Runtime enforcement; capability negotiation protocols
- **Dependencies:** AGOV-001 (trust score informs capability trust); consumed
  downstream by POLCAP (which builds the agent-facing signed capability view
  on this manifest) and MDGOV M3
- **Validation:** `cargo test -p eddacraft-anvil-checks -- capability_declaration`
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
