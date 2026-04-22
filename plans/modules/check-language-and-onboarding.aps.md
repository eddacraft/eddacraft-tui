# Check Language and Onboarding

| ID    | Owner  | Status      | Progress |
|-------|--------|-------------|----------|
| CLAR  | @aneki | In Progress | 6/9      |

## Purpose

Rebaseline how Anvil talks about checks, scans, gates, graphs, policies,
warnings, tutorials, and onboarding so the product teaches a coherent mental
model instead of exposing implementation drift. The first job is discovery:
inventory every relevant surface in code, docs, config, CLI, and TUI, then
derive a common language and user-learning path from what actually exists.

This module exists because the same concept is currently presented through
multiple overlapping vocabularies. Users meet different names in onboarding,
configuration, command output, docs, and internal code, which makes it harder
to understand what Anvil does, which parts are distinct, and which words are
aliases for the same thing.

## In Scope

- Inventory every check, scan, gate, graph, policy, warning, and closely
  related term exposed in the codebase
- Inventory forward-looking plan/spec language so emerging commands, screens,
  routes, and surfaces are captured before they ship with conflicting names
- Map each term to the surface where it appears: CLI, TUI, welcome flow,
  tutorial, docs, config, tests, plans, architecture docs
- Distinguish user-facing concepts from implementation backends and historical
  aliases
- Define a canonical language set and term taxonomy for Anvil's quality model
- Reorganise onboarding and tutorial flows around that canonical language
- Update docs and product copy so core surfaces teach the same mental model
- Identify dead, misleading, duplicated, or implementation-leaking terminology

## Out of Scope

- Re-architecting the gate runner or scanner internals in the first pass
- Adding entirely new analysis capabilities
- Renaming low-level crate/package internals purely for aesthetic consistency
- Dashboard implementation beyond language and information architecture inputs
- Marketing-site messaging outside Anvil product understanding surfaces

## Interfaces

**Depends on:**

- `crates/anvil-cli/` — command vocabulary, onboarding defaults, welcome flow
- `crates/anvil-tui/` — gate, watch, tutorial, welcome, and status surfaces
- `crates/anvil-kernel/` — graph/scanner terminology and internal nouns
- `crates/anvil-policy/` and `policies/` — policy vocabulary and evaluation
- `docs/public/anvil/` and `docs/guides/` — user-facing documentation
- `plans/index.aps.md`, ADRs, and architecture docs — intended product language

**Exposes:**

- Canonical terminology inventory for product and engineering use
- A term taxonomy that maps concepts, aliases, and implementation backends
- A recommended user-learning model for onboarding, tutorial, and docs
- Follow-on execution tasks for registry unification, copy changes, and UX fixes

## Acceptance Criteria

- [ ] A repo-wide inventory exists for every user-visible check/scan/gate/graph/
      policy/warning term and its source location
- [ ] The inventory also captures planned and emerging commands, surfaces, and
      nouns described in APS modules and specs
- [ ] Canonical product language is defined, with aliases and deprecated terms
      explicitly listed
- [ ] A short mental-model document explains how Anvil's concepts fit together
      for a new user
- [ ] Onboarding, tutorial, and docs gaps are documented against that model
- [ ] Follow-on tasks are identified for code, copy, and UX changes needed to
      align the product

## Constraints

- Discovery first: do not normalise wording in code before the inventory and
  taxonomy exist
- Canonical language must reflect shipped behaviour, not aspirational features
- User-facing clarity takes precedence over preserving incidental internal names
- Historical terms may be retained as aliases during migration, but only when
  explicitly documented

## Tasks

### CLAR-001: Inventory quality-language surfaces across the repo

- **Intent:** Build a complete inventory of where checks, scans, gates, graphs,
  policies, warnings, and adjacent terms appear, including forward-looking
  plans and specs
- **Expected Outcome:** A discovery document enumerates each term, where it
  appears, whether it is user-facing or internal, and what behaviour it refers
  to
- **Files:** `crates/anvil-cli/`, `crates/anvil-tui/`, `crates/anvil-kernel/`,
  `crates/anvil-policy/`, `docs/public/anvil/`, `docs/guides/`, `plans/`
- **Validation:** `plans/specs/YYYY-MM-DD-check-language-inventory.md` exists
- **Confidence:** high
- **Status:** Complete

### CLAR-002: Classify concepts, aliases, and implementation backends

- **Intent:** Turn the raw inventory into a taxonomy that separates product
  concepts from execution details and historical drift
- **Expected Outcome:** Every inventoried term is classified as canonical
  concept, alias, backend implementation, internal-only term, or deprecated
  term; ambiguous overlaps are called out
- **Files:** `plans/specs/YYYY-MM-DD-check-language-inventory.md`
- **Dependencies:** CLAR-001
- **Validation:** Inventory doc includes a taxonomy table covering all recorded
  terms
- **Confidence:** high
- **Status:** Complete

### CLAR-003: Define Anvil's user mental model and canonical language

- **Intent:** Write the minimal concept model a new user needs in order to
  understand what Anvil checks, what a gate run means, and how related surfaces
  connect
- **Expected Outcome:** A short design/spec document defines the canonical
  terms, preferred definitions, forbidden or deprecated wording, and the
  learning sequence for new users
- **Files:** `plans/specs/`, `docs/architecture/`, `docs/public/anvil/`
- **Dependencies:** CLAR-002
- **Validation:** `plans/specs/YYYY-MM-DD-anvil-quality-language-design.md`
  exists with canonical term definitions
- **Confidence:** medium
- **Status:** Complete

### CLAR-004: Audit onboarding, welcome, and tutorial flows against the model

- **Intent:** Identify where current onboarding and tutorial surfaces confuse,
  overload, or fail to teach the canonical mental model
- **Expected Outcome:** A gap report maps each onboarding/tutorial step to the
  intended concepts, highlighting missing explanations, wrong labels, and poor
  sequencing
- **Files:** `crates/anvil-cli/src/commands/welcome.rs`,
  `crates/anvil-cli/src/commands/defaults.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/`,
  `docs/public/anvil/`
- **Dependencies:** CLAR-003
- **Validation:** Inventory or design spec includes an onboarding/tutorial gap
  matrix
- **Confidence:** medium
- **Status:** Complete

### CLAR-005: Define follow-on execution slices for language and UX alignment

- **Intent:** Convert the discovery and design outputs into bounded
  implementation work rather than one large rewrite
- **Expected Outcome:** Follow-on APS tasks or modules are proposed for registry
  unification, config/CLI naming alignment, documentation rewrites, and
  tutorial/onboarding revisions
- **Files:** `plans/modules/`, `plans/specs/`, `plans/index.aps.md`
- **Dependencies:** CLAR-004
- **Validation:** Follow-on work items are listed with scope and validation
- **Confidence:** medium
- **Status:** Complete

### CLAR-006: Runtime naming alignment for checks and gates

- **Intent:** Align onboarding, config, gate execution, and gate-config around
  one user-facing naming layer for checks and gates
- **Expected Outcome:** Users see one coherent check/gate vocabulary across
  guided init, `.anvilrc`, gate output, and `.anvil/gate-config.json`, with
  explicit alias handling where migration is needed
- **Files:** `crates/anvil-cli/src/commands/defaults.rs`,
  `crates/anvil-cli/src/commands/gate.rs`,
  `crates/anvil-cli/src/commands/gate_config.rs`,
  `crates/anvil-cli/src/commands/init.rs`,
  `docs/public/anvil/operations/config.md`
- **Dependencies:** CLAR-005
- **Validation:** One canonical check-name table matches onboarding, gate, and
  gate-config surfaces, or the mapping is explicitly documented and tested
- **Confidence:** medium
- **Status:** Ready

### CLAR-007: Welcome and tutorial model rewrite

- **Intent:** Rewrite first-run teaching surfaces so they explain the canonical
  model before introducing subsystem-specific commands and modes
- **Expected Outcome:** Welcome and tutorial explicitly teach
  scan -> checks -> findings -> gate, and quick actions/path descriptions are
  framed around user goals rather than raw command names
- **Files:** `crates/anvil-cli/src/commands/welcome.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/`
- **Dependencies:** CLAR-006
- **Validation:** First-run flow contains an explicit model explanation and
  tutorial path text aligns with the canonical language design
- **Confidence:** medium
- **Status:** Ready

### CLAR-008: Public docs terminology cleanup

- **Intent:** Bring config and tutorial docs into alignment with the canonical
  quality language
- **Expected Outcome:** Public docs teach one quality model, use `finding` as
  the generic result noun where appropriate, and no longer imply incompatible
  check systems
- **Files:** `docs/public/anvil/operations/config.md`,
  `docs/public/anvil/tutorials/architecture.md`,
  `docs/public/anvil/`
- **Dependencies:** CLAR-006, CLAR-007
- **Validation:** Targeted docs reviewed against
  `plans/specs/2026-04-21-anvil-quality-language-design.md`
- **Confidence:** medium
- **Status:** Ready

### CLAR-009: APS wording reconciliation for active modules

- **Intent:** Update active and proposed APS modules whose wording would
  otherwise introduce new terminology drift
- **Expected Outcome:** `RCLI2`, `RCLI3`, and other targeted modules use the
  canonical terms or explicitly justify exceptions
- **Files:** `plans/modules/rust-cli-tier2.aps.md`,
  `plans/modules/rust-cli-tier3.aps.md`, `plans/modules/`, `plans/specs/`
- **Dependencies:** CLAR-005
- **Validation:** Targeted APS modules are updated with canonical-language
  notes or reconciled wording
- **Confidence:** high
- **Status:** Complete
