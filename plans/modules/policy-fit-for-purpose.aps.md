<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Policy Fit for Purpose

| ID     | Type      | Owner | Priority | Status | Progress |
| ------ | --------- | ----- | -------- | ------ | -------- |
| POLFIT | Conductor | —     | high     | Ready  | 3/9      |

**Current state (2026-08-23):** POLFIT-001 and POLFIT-002 **Merged** via
#4103 and #4104 — ADR-129 (intra-repo surface precedence) and ADR-130
(authoring on-ramp). Both design gates are closed, so POLFIT-003..-006 and
-008 are no longer blocked by them. POLFIT-009 is **Merged** (#4108).
POLFIT-007 is **Merged** via #4107 — CPACKS-007 via #4113, CPACKS-006 via
eval wrappers. POLFIT-008 is **Merged** via #4110 (ADR-131).

The dated entries below are history, not current status. Per-item `Status:`
lines are authoritative.

**Promoted:** 2026-08-23 — POLFIT-007 and POLFIT-009 advanced Draft -> Ready on
operator instruction. Neither depends on the two design gates: -007 drives the
already-filed CPACKS residue, and -009 is a posture-and-stamp pass the
conductor owns outright. CPACKS-007 was promoted Proposed -> Ready in the same
change so -007's delegated work is fully authorised.

2026-08-23 — POLFIT-008 advanced Draft -> Ready on operator instruction
("complete POLFIT-008") and is In Progress. POLFIT-001 (Merged, ADR-129)
is the closed dependency.

2026-08-22 — POLFIT-001 and POLFIT-002 advanced Draft -> Ready on
operator instruction; module status follows its items to **Ready**. Both are
decision-record items: Ready authorises producing the ADR, not implementing
what the ADR decides. POLFIT-003..-006 stay Draft behind them.

**Last reviewed:** 2026-08-22 (created from a policy-capability audit against
`origin/main` @ `7524a599b`, binary version 0.9.7-beta. The audit read the
shipped code — not the plans — and found seven user-modifiable policy
surfaces, only three of which have a public reference page, plus three
untracked defects. Every item below is either a delegation to an existing
module or a gap that had no home.)

> **Conductor module.** POLFIT coordinates existing policy modules; it does not
> take their execution authority. Implementation lands in the module named by
> each item's `Coordinates with` field. POLFIT owns an item outright only where
> the audit found a gap no vertical module claims — POLFIT-001, -002, and -008.

> **Relationship to POLRESET.** POLRESET (Complete 10/10, archived 2026-07-13)
> asked *does policy produce value at all?* and answered yes: packs load,
> validate, evaluate through regorus, and route to `warn`/`fence`/`interrupt`.
> POLFIT asks the next question — *is that capability fit for a team to
> actually adopt?* — and the audit's answer is not yet, for reasons that are
> individually small and collectively decisive.

## Purpose

Make Anvil's shipped policy capability adoptable by a team that did not build
it. Today a user who wants to change policy behaviour must discover seven
different modification surfaces with no stated precedence between them, four of
which have no public reference page — `anvil/policy.*` L4 branch policy appears
only in a changelog line, `enforcement.mode` and `enforcement.intercept-rules`
appear nowhere, and the anti-pattern registry override is stated only inside
ADR-026. The same user must follow a public instruction to install an authoring
skill that does not exist, and gets different admission behaviour from the gate
than from pre-write validation for the same pack.

The seventh surface — the registry override — is **policy-adjacent rather than
policy-engine**: it changes what anvil flags without going through regorus at
all. POLFIT-001 counts it because a user cannot be told "here is every place
policy lives" while it is omitted; POLFIT-008 exists to classify it.

A large part of "fit for purpose" is **ease of authoring**. The audit found no
supported path from "I want a rule" to "a rule that fires" that does not
require hand-writing Rego against an input contract whose configuration field
is permanently dead. POLFIT-002 owns that gap explicitly.

## Product Outcome

Policy is fit for purpose when:

- every place a user can change policy behaviour is enumerated in one public
  page, with a stated precedence order between them;
- a team can author a working policy without writing Rego from scratch, and the
  documented authoring door resolves to a surface that ships;
- a pack admitted by `anvil policy validate` behaves the same at `anvil gate`
  as it does at MCP pre-write;
- pack thresholds are configurable by the adopting project rather than frozen
  by the pack author;
- the enterprise policy story states honestly what it does and does not do, so
  no adopter plans around capability that is four Draft modules away.

## Scope

### In Scope

- Enumerating and pinning precedence across the shipped policy modification
  surfaces (config rule modes, Rego packs, architecture YAML, L4 branch policy,
  intercept rules, enforcement posture, and the anti-pattern registry override).
- Closing the documentation gaps where a shipped surface has no public entry.
- Reconciling admission behaviour between the gate and pre-write evaluators.
- Sequencing the authoring on-ramp — deciding between and ordering the ACTAX
  YAML tier, the OPAE lint/guidance/skill chain, and pack scaffolding.
- Closing the small residue left by POLRESET's first slice (CPACKS-006/-007).
- Forcing an honest posture call on the dormant enterprise policy modules.

### Out of Scope

- Organisational hierarchy, lifecycle, federation, and compliance reporting
  execution. POLFIT-009 only makes their posture honest; promoting them stays
  demand-gated per ORGHIER's existing activation gate.
- A second production policy runtime. ADR-040/ADR-098 stand: Rego authored,
  regorus evaluated, Go OPA as reference/parity tooling only.
- Broad compliance packs (OWASP/SOC 2/ISO/GDPR/AI). Those stay behind
  CPACKS-008 and COMPLY's evidence-semantics gate.
- New tool-call interception surfaces. ADR-098 AD-4 still requires its own ADR.

## Design Gates

Both gates are **Ready as of 2026-08-22** — they are being worked, not parked.
They still gate the items below them.

1. **Policy surface precedence (POLFIT-001).** No item that changes behaviour
   across more than one surface may start until precedence is decided. ADR-120
   consolidated the config surface but explicitly placed "policy merge
   semantics" out of scope, so this picks up an unowned carve-out.
2. **Authoring on-ramp sequencing (POLFIT-002).** ACTAX Phase A and the
   OPAE-013..017 chain both claim the authoring-ease outcome. One product
   decision must sequence them before *either of those* is promoted to Ready;
   the decision itself runs in parallel with POLFIT-001.

## Coordinated Modules

| Module                                                                | Role in POLFIT                                       | Posture                                              |
| --------------------------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------- |
| [opa-enhancements](./opa-enhancements.aps.md)                         | policy authoring and runtime UX contracts            | Live delegate — owns OPAE-010, -021, -022            |
| [compliance-policy-packs](./compliance-policy-packs.aps.md)           | starter pack and its documentation residue           | Live delegate — owns CPACKS-006, -007                |
| [docs-definition-layer](./docs-definition-layer.aps.md)               | source-cited public config field catalogue           | Reopened delegate — owns DOCDEF-007                  |
| [policy-action-taxonomy](./policy-action-taxonomy.aps.md)             | YAML authoring tier compiling to Rego                | Sequencing candidate under POLFIT-002                |
| [git-native-exceptions](./git-native-exceptions.aps.md)               | scoped, expiring, auditable exceptions               | Unchanged; already the escape hatch for a fired rule  |
| [insecure-construction-catalogue](./insecure-construction-catalogue.aps.md) | `patterns/` registry and scanner families      | Consulted on POLFIT-008; does not own the loader      |
| [architecture-config-validation](./architecture-config-validation.aps.md) | architecture YAML boundary surface              | Consulted on POLFIT-001 precedence                    |
| [org-policy-hierarchy](./org-policy-hierarchy.aps.md)                 | organisational hierarchy (root of the enterprise chain) | Posture-only under POLFIT-009                     |
| [policy-lifecycle](./policy-lifecycle.aps.md)                         | versioning, canary, grace, retirement                | Posture-only under POLFIT-009                        |
| [policy-federation](./policy-federation.aps.md)                       | signed pack distribution across repositories         | Posture-only under POLFIT-009                        |
| [compliance-reporting](./compliance-reporting.aps.md)                 | compliance evidence and reporting                    | Posture-only under POLFIT-009                        |

## Work Items

### POLFIT-001: Policy surface inventory and precedence decision

- **Status:** Merged 2026-08-23 via PR #4103. Ancestor of `origin/main`
  (`b9bde3976`). ADR-129 is the decision record. POLFIT-003, -004, -005, and
  -006 remain Draft until separately promoted. POLFIT-008 is classified by
  ADR-131.
- **Intent:** Decide, in one record, every surface from which a user can change
  policy behaviour, which are supported product surfaces, and what wins when
  two disagree.
- **Expected Outcome:** A decision record enumerates the seven shipped
  surfaces —
  `.anvil.yaml` `enforcement.rules` rule modes, `.anvil/policies/**.rego`
  packs, the `architecture` section or `.anvil/architecture.yaml`,
  `anvil/policy.*` L4 branch policy, `.anvil.yaml`
  `enforcement.intercept-rules`, `.anvil.yaml` `enforcement.mode`, and the
  ADR-026 anti-pattern registry resolution order — classifies each as
  supported/internal/deprecated, and states precedence and merge semantics
  where two surfaces address the same behaviour. Explicitly picks up the
  "policy merge semantics" carve-out ADR-120 left unowned.
- **Files:** `plans/decisions/129-policy-surface-inventory-and-precedence.md`,
  `plans/decisions/DECISION-LOG.md`,
  `docs/public/anvil/concepts/policy-model.md`
- **Validation:** `pnpm adr:check && pnpm aps:active-lint`
- **Dependencies:** ADR-026, ADR-037, ADR-040, ADR-098, ADR-120
- **Coordinates with:** OPAE, ARCHCFG, DOCDEF, INSEC
- **Confidence:** medium

### POLFIT-002: Policy authoring on-ramp decision

- **Status:** Merged 2026-08-23 via PR #4104 — ADR-130 Proposed on
  `origin/main` @ `b977d7f8e`. Deliverable is the decision record, not
  implementing what it decides.
- **Evidence:** `pnpm adr:check` exit 0; `pnpm aps:active-lint` exit 0;
  `pnpm docs:check` exit 0. Ancestor of `origin/main`.
- **Intent:** Decide how a team creates a working policy without hand-writing
  Rego, and sequence the competing candidates so only one is promoted first.
- **Expected Outcome:** A decision record picks an ordering across the ACTAX
  YAML-to-Rego tier (Phase A, currently blocked by nothing and itself blocking
  POLCAP-009 and AGOV-007), the OPAE-013/-014 deterministic lint, the
  OPAE-015/-016 generated guidance, the OPAE-017 `authoring-anvil-policy`
  skill, and pack scaffolding from an installed starter. States which single
  path is the supported answer to "how do I write a policy?" and what the
  others are for. Names the first promotable item.
- **Files:** `plans/decisions/130-policy-authoring-on-ramp.md`,
  `plans/decisions/DECISION-LOG.md`,
  `plans/decisions/040-rust-policy-engine-regorus.md`,
  `plans/decisions/108-policy-authoring-lint-and-agent-guidance.md`,
  `docs/guides/opa-policy-testing.md`
- **Validation:** `pnpm adr:check && pnpm aps:active-lint`
- **Dependencies:** ADR-108 (Accepted 2026-07-16)
- **Coordinates with:** POLFIT-001 (non-blocking — see Notes), ACTAX-010..014,
  OPAE-012..017, CPACKS, CPOL
- **Notes:** POLFIT-001 was listed as a dependency when this item was filed and
  was moved here on 2026-08-22 when both were promoted to Ready, so
  `Dependencies:` carries only what must complete first. The relationship is
  real but not blocking: every on-ramp candidate — the ACTAX YAML tier, the
  OPAE lint and guidance chain, and pack scaffolding — targets the same
  `.anvil/policies/` pack surface, so the ordering question is answerable
  regardless of how precedence between *different* surfaces resolves.
  **Falsifier:** if POLFIT-001 concludes the pack surface is not the supported
  authoring target, this item must be re-run.
- **Confidence:** medium

### POLFIT-003: The public authoring door resolves to a shipped surface

- **Status:** Draft
- **Intent:** Stop the public policy model page instructing users to install an
  authoring skill that does not exist.
- **Expected Outcome:** `docs/public/anvil/concepts/policy-model.md` either
  describes an authoring path that ships today, or the `authoring-anvil-policy`
  instruction is removed until OPAE-017 lands. No public page directs a user at
  an unshipped surface.
- **Validation:** `pnpm docs:check && pnpm docs:public:check`
- **Dependencies:** POLFIT-002
- **Coordinates with:** OPAE-021 (owns the fix), OPAE-009, OPAE-017
- **Confidence:** high

### POLFIT-004: One pack-admission contract across gate and pre-write

- **Status:** Draft
- **Intent:** Make a pack behave the same way at `anvil gate` as it does at MCP
  pre-write validation.
- **Expected Outcome:** The divergence is resolved by decision, not left
  latent: the gate's flat recursive `*.rego` walk and pre-write's
  manifest-driven `discover_and_load` agree on what is admitted, or the
  difference is documented as intended with the user-visible consequence
  stated. A loose `.rego` and a pack member no longer fire on different
  surfaces without explanation.
- **Files:** `crates/anvil-cli/src/commands/gate.rs`,
  `crates/anvil-cli/src/mcp/policy_prewrite.rs`
- **Validation:** `cargo test -p eddacraft-anvil -- policy_prewrite_routing` and
  `cargo test -p eddacraft-anvil -- run_check_policy`
- **Dependencies:** POLFIT-001
- **Coordinates with:** OPAE-022 (owns the fix), OPAE-011
- **Confidence:** medium

### POLFIT-005: Policy packs are configurable by the adopting project

- **Status:** Draft
- **Intent:** Let an adopting project tune a pack's thresholds instead of
  inheriting the pack author's constants.
- **Expected Outcome:** The `PolicyInput` configuration gap is closed so a pack
  rule reading project configuration is no longer dead code in production, and
  the shipped `anvil-baseline` thresholds become tunable.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- policy_input_config`
- **Dependencies:** POLFIT-001
- **Coordinates with:** OPAE-010 (owns the work, already filed)
- **Confidence:** medium

### POLFIT-006: Shipped enforcement config keys are documented

- **Status:** Draft
- **Intent:** Close the gap between the public config field catalogue's claimed
  completeness and the keys the binary actually reads.
- **Expected Outcome:** `enforcement.mode` and `enforcement.intercept-rules`
  appear in the catalogue with the same source-cited treatment as
  `enforcement.rules`, or are explicitly listed as out of catalogue with a
  reason. The page stops asserting coverage it does not have.
- **Validation:** `pnpm docs:check && pnpm docs:public:check`
- **Dependencies:** POLFIT-001
- **Coordinates with:** DOCDEF-007 (owns the fix)
- **Confidence:** high

### POLFIT-007: Starter-pack residue closed

- **Status:** Merged 2026-08-24 via PR #4107. Ancestor of `origin/main`
  (`c916da49f`). CPACKS-007 via #4113; CPACKS-006 via eval wrappers that emit
  v1 Finding objects without changing the pack's `warning` family.
- **Files:** `ci/eval/suites.json`, `ci/eval/baseline/history.jsonl`,
  `ci/eval/README.md`, `policies/eval/anvil_baseline_change_scope.rego`,
  `policies/eval/anvil_baseline_sensitive_paths.rego`,
  `crates/anvil-policy/src/eval/port.rs`,
  `crates/anvil-cli/src/commands/policy/starter_proof.rs`
- **Intent:** Finish the two small items POLRESET's first slice left open so
  CPACKS reaches a terminal state.
- **Expected Outcome:** The `anvil-baseline` fixtures are wired into the CI eval
  suite, and the known-gaps and non-compliance-posture documentation audit is
  complete. CPACKS carries no live item outside the CPACKS-008 expansion gate.
- **Validation:** `pnpm aps:active-lint && pnpm docs:check`
- **Dependencies:** none
- **Coordinates with:** CPACKS-006, CPACKS-007 (both already filed)
- **Confidence:** high

### POLFIT-008: The anti-pattern registry override is a stated surface or a closed one

- **Status:** Merged 2026-08-24 via PR #4110. Ancestor of `origin/main`
  (`c3e38252e`).
- **Intent:** Decide whether a repository-local compiled registry — the seventh
  surface in POLFIT-001's enumeration, and the only one that changes what anvil
  flags without going through regorus — is a supported way to change policy
  behaviour, given that nothing outside ADR-026 says so.
- **Expected Outcome:** The ADR-026 four-tier resolution order — explicit path,
  then `ANVIL_REGISTRY_PATH`, then a cwd upward walk, then an executable-directory
  upward walk, then the embedded fallback — is either documented as a supported
  override with its trust boundary stated, or bounded so a cloned repository
  cannot silently replace the anti-pattern catalogue in an adopting project.
  POLFIT owns this item because the loader is cross-family infrastructure that
  no vertical module claims; INSEC owns the families, not the resolution order.
  **Decision (ADR-131):** bound the implicit walks; keep explicit
  `registry_path` / `ANVIL_REGISTRY_PATH` as the supported unsigned override.
  Default load is the compile-time embedded catalogue. A cloned
  `patterns/compiled/registry.json` does not replace it.
- **Files:** `crates/anvil-checks/src/antipattern/registry_loader.rs`,
  `plans/decisions/131-registry-override-explicit-only.md`,
  `plans/decisions/026-rust-scanner-authoritative.md`,
  `plans/decisions/129-policy-surface-inventory-and-precedence.md`,
  `plans/decisions/DECISION-LOG.md`,
  `docs/public/anvil/concepts/policy-model.md`,
  `docs/guides/anvil-rule-authoring.md`,
  `crates/anvil-checks/ARCHITECTURE.md`
- **Validation:** `cargo test -p eddacraft-anvil-checks -- registry_loader` and
  `pnpm adr:check && pnpm aps:active-lint && pnpm docs:check`
- **Dependencies:** POLFIT-001, ADR-026
- **Coordinates with:** INSEC, DOCDEF
- **Evidence:** `cargo test -p eddacraft-anvil-checks --no-fail-fast -- registry_loader` 20 passed; `pnpm adr:check` exit 0 (132 files, next 132); `pnpm aps:active-lint` exit 0; `pnpm docs:check` 13/13 pass. Independent verify pass-with-advisories (leftover walk phrasing repaired).
- **Confidence:** high

### POLFIT-009: Enterprise policy modules carry an honest posture

- **Status:** Merged 2026-08-23 via PR #4108. Ancestor of `origin/main`
  (`abe6be8b6`); all six posture blocks verified present on the merged tree.
  Conductor owns this outright:
  the deliverable is a posture and review stamp on each named module, not a
  delegated work item. Scope confirmed on execution: all six coordinated
  modules were stale (four at 2026-07-11, two at 2026-07-17) and five of the
  six carry **zero work items with a `Status:` field**, so no progress counter
  could ever have reflected reality. TRUST was the only one whose recorded
  status already matched. **ADR-129 §D-5 (merged after this item was filed)
  routes the org/federated overlay question here**; the honest answer is
  that it stays open by design — there is no merge function to specify for
  modules that do not ship — and each of ORGHIER/POLLC/POLFED now records
  that. Promoted Ready 2026-08-23 by operator instruction.
- **Intent:** Stop the enterprise policy modules reading as an active roadmap
  when no organisational policy capability ships and none is scheduled.
- **Scope:** All six coordinated modules — ORGHIER, POLLC, POLFED, COMPLY,
  CEWS, and TRUST. Widened from the original four on execution (2026-08-23):
  CEWS and TRUST sit on the same dependency chain and were already named under
  `Coordinates with`, so leaving them unstamped would have left two of the six
  still reading as scheduled.
- **Expected Outcome:** Each of the six carries a current review stamp and an
  explicit posture — dormant, blocked, demand-gated, or promoted — so an
  adopter or a planning session can tell at a glance that organisational policy
  today means hand-copying a pack directory into each repository. No index row
  implies scheduled work that does not exist.
- **Validation:** `pnpm aps:active-lint && pnpm aps:index:check`
- **Dependencies:** none
- **Coordinates with:** ORGHIER, POLLC, POLFED, COMPLY, CEWS, TRUST
- **Confidence:** high

## Audit Evidence

The findings below were read from `origin/main` @ `7524a599b` (0.9.7-beta) on
2026-08-22. Each names the source that establishes it.

| Finding                                                | Source                                                                                       | Item       |
| ------------------------------------------------------ | -------------------------------------------------------------------------------------------- | ---------- |
| Seven user-modifiable policy surfaces, no stated precedence | `anvil-config/src/rule_modes.rs`, `anvil-l4/src/policy.rs`, `anvil-intercept-rules/src/config.rs`, `anvil-kernel-types/src/enforcement.rs`, `anvil-checks/src/antipattern/registry_loader.rs` | POLFIT-001 |
| `enforcement.mode` read but absent from the catalogue  | `crates/anvil-cli/src/mcp/enforcement.rs:67-81` vs `docs/public/anvil/reference/config.md`    | POLFIT-006 |
| `enforcement.intercept-rules` read but undocumented    | `crates/anvil-intercept-rules/src/config.rs:93-110`                                           | POLFIT-006 |
| Public docs direct users at an unshipped skill         | `docs/public/anvil/concepts/policy-model.md` vs OPAE-017 (Proposed)                            | POLFIT-003 |
| Gate is pack-blind; pre-write is pack-aware            | `gate.rs:3708-3760` (flat `*.rego` walk) vs `mcp/policy_prewrite.rs:140` (`discover_and_load`) | POLFIT-004 |
| `PolicyInput` carries no populated config field        | OPAE-010, confirmed against the frozen v1 contract                                            | POLFIT-005 |
| Registry override precedence undocumented for users    | `antipattern/registry_loader.rs:156-190`, decided in ADR-026 §1                                | POLFIT-008 |
| No authoring path that avoids hand-written Rego        | ACTAX Proposed; OPAE-013..017 all Proposed                                                     | POLFIT-002 |
| Enterprise chain is four Draft modules, 0 items done   | ORGHIER, POLLC, POLFED, COMPLY headers                                                         | POLFIT-009 |

## Release Posture

POLFIT is an adoption-readiness programme. It is not part of a release claim
set and does not gate a release cut. Individual delegated items may land in a
release window on their owning module's terms.
