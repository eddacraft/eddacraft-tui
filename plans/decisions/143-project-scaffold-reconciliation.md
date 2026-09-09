# ADR-143: Project scaffold reconciliation

## Status

Accepted

## Date

2026-09-09

## Context

`anvil init` has become an incomplete middle state. The first-user path moved to
`anvil start`, while configuration artefacts formerly created during onboarding
were retired from that flow. Operators who use only `start` therefore never see
some useful project configuration, including architecture, while operators who
choose `init` cannot tell whether it is a manual bootstrap, a partial activation,
or a legacy predecessor to `start`.

The ambiguity is also an enforcement defect. `start` can install a pre-push hook
and status can label L4 `On`, yet both L4 entry points intentionally return
success without evaluation when no discoverable acceptance policy exists. A
beta tester reported that L4 never fires. Source tracing confirmed that the
engine works; the activation prerequisites and status claim do not agree.

The repository already has two relevant decisions. ADR-120 makes the main
`.anvil.<ext>` config canonical and permits delegated architecture through
`architecture.source`. ADR-102 proposed a separate, non-interactive
`anvil architecture init` that generates ignored legacy
`.anvil/architecture.yaml`. Implementing that command now would create a second
bootstrap surface and preserve the drift this decision must remove.

## Decision

### 1. One reconciler, two product surfaces

`anvil init` is the idempotent base-scaffold reconciler. `anvil start` invokes
the same typed catalogue and reconciliation engine as the first section of its
activation plan, then applies the existing opinionated activation overlay.
There is no duplicated scaffold implementation and no nested wizard.

Direct init is configuration-only. Daemon, hooks, MCP, workflows, witnesses,
baselines, and the first scan remain activation responsibilities. The first scan
moves wholly to `start` and flows derived from it.

### 2. Mandatory foundation, selectable protections

Every scaffold has a foundation: canonical main config, project identity,
ignore rules, and scaffold-version metadata. The operator then chooses which
protections to configure: acceptance policy, detected architecture, checks and
enforcement posture, and planning support.

Interactive use begins with “Which protections would you like to configure?”.
Non-interactive use selects a named profile (`core`, `recommended`, or `all`)
with repeatable `--include` and `--skip` overrides. Structured results are
`created`, `configured`, `preserved`, `skipped`, or `needs_input`.

Components are dependency-aware. A runtime protection cannot be planned when a
required scaffold component is skipped; the plan deselects it and explains the
consequence.

Foundation is unskippable. Every optional component and runtime integration
depends on a validated foundation; `--skip foundation` is rejected before any
write. Mutation outcome and component health are separate: the five outcome
states do not imply validity, and dependants require explicit `valid` health.

### 3. Additive ownership and scoped replacement

Scaffold reconciliation is additive. Existing paths and keys are
operator-owned. Missing owned sections may be proposed after semantic
comparison, but existing content is never silently changed, removed,
normalised, or reformatted.

`--force` is valid only alongside explicit component selectors, shows a diff,
and is confined to catalogue-marked replaceable artefacts. Initially that is
only an unmodified, provenance-matching generated architecture definition;
settings, identity, policy, ignore, and planning content are never replaced by
scaffold force. Bare force refuses. Writes must use race-safe exclusive creation
or an equivalent descriptor-bound operation; validating a path and later
opening it by name is insufficient.

Acceptance policy is excluded from force replacement. Existing policy bytes are
preserved under every scaffold invocation; policy replacement belongs to a
separate policy-authoring workflow with semantic weakening confirmation.
Updates to an existing file use a no-follow identity-and-digest compare-and-swap
under the mandatory shared mutation lock, revalidating immediately before atomic
publication and refusing/re-planning on cooperating-writer change. If the lock
is unavailable, bootstrap emits a patch and returns `needs_input`. Arbitrary
external editors are outside that lock guarantee and are disclosed as such.
The lock is rooted in the Git common directory so all linked worktrees share it;
PSCAF-001 migrates every anvil main-config writer to the primitive. Non-Git
existing-file augmentation is patch-only.

### 4. Architecture joins the project scaffold

ADR-102's separate `anvil architecture init` decision is superseded. The
architecture component performs bounded detection, offers a suitable scaffold
or skip, and writes tracked `anvil/architecture.yaml` referenced by
`architecture.source`. Low-confidence non-interactive runs write nothing and
report `needs_input`; they never invent a generic model or fail unrelated
components. Existing legacy `.anvil/architecture.yaml` remains readable under
ADR-120.

### 5. One catalogue feeds CLI and public documentation

A typed scaffold catalogue owns component identity, purpose, artefacts,
recommendation, detection, prerequisites, inactive consequence, and later
command. Both CLI selection and a generated public **Project configuration
options** page consume it. Staleness is a docs-check failure. This is the one
component inventory; narrative guides link rather than copy it.

For config targets, it references ADR-132 settings-catalogue identifiers and
routes writes through a bounded settings-service bootstrap operation. The
settings catalogue remains canonical for type, scope, merge, sensitivity,
mutability, source, and writer identity. PSCAF owns only component composition,
recommendation, dependencies, and non-config artefact association. A parity
gate prevents the two layers from drifting.

This amends ADR-132's compatibility-path sequencing only: PSCAF brings forward
the project-scoped bootstrap mutation adapter needed by init/start, but does not
open general settings mutation or the SETPREF/SETGOV surfaces. Existing config
augmentation is syntax-preserving; when insertion cannot preserve unrelated
bytes, bootstrap returns `needs_input` rather than reserialising the document.

### 6. L4 claims require all prerequisites

The acceptance-policy component creates the ADR-037 default only when no policy
exists. L4 is `On` only with a valid project identity, active pre-push hook, and
discoverable parseable policy. Status, doctor, hook, and `l4-validate` share
policy discovery and parsing. Doctor names invalid-policy remediation; public
guidance names all intentional silent-pass paths and the `l4_or_l3` witness
short-circuit.

No L4 evaluation semantics, existing policy content, Serena error posture, or
`on_warn` default changes.

ADR-103's machine boundary is unchanged. `start --verify` and `start --json`
remain read-only and byte-stable. Piped mutable start may apply the recommended
additive scaffold through the existing non-interactive policy, but its compact
stdout fixture stays byte-stable. Scaffold choices are visibly prepended only to
the interactive TUI/plain activation plan.

The complete executable contract is
[`plans/specs/2026-09-09-project-scaffold-reconciliation.md`](../specs/2026-09-09-project-scaffold-reconciliation.md).

## Rationale

The project needs two entry points because their intent differs: `init` supports
manual and subsequent-project setup, while `start` is the opinionated way to get
running. They should differ by composition, not by maintaining two independent
sets of bootstrap writes.

Making the catalogue executable prevents the CLI and documentation from
becoming rival inventories. Additive ownership keeps repeat runs safe in real
repositories. Explicit low-confidence handling prevents an architecture-shaped
placeholder from being mistaken for a verified model. Treating L4 prerequisites
as a dependency closes the specific gap behind the beta report without changing
the policy engine.

### Alternatives considered

| Option | Benefit | Why rejected |
| --- | --- | --- |
| Keep `init` minimal and let `start` own all useful setup | Smallest immediate change | Missing configuration stays invisible; two bootstrap paths continue to drift |
| Have `start` shell out to the public `init` command | Reuses behaviour | Produces a nested interaction and couples activation to CLI presentation rather than a shared engine |
| Keep separate `anvil architecture init` | Narrow command ownership | Creates a second scaffold catalogue and generates the ignored legacy path |
| Generate every known config file | Visibly complete tree | Empty placeholders imply protection and create state without function |
| Rewrite existing config to canonical form | Simpler writer | Violates operator ownership and loses comments/formatting; unsafe on repeat runs |
| Treat a hook as sufficient proof of L4 | Preserves current status logic | Continues claiming enforcement when the hook is guaranteed to no-op |

## Consequences

- The shared scaffold engine becomes an internal boundary consumed by both
  commands; command-specific code owns presentation and activation only.
- `init` gains a stable non-interactive contract and machine-readable component
  outcomes.
- `start` must render scaffold dependencies as part of its existing plan.
- Architecture scaffolding moves from ARCHCFG-007 into PSCAF and changes the
  generated path to tracked `anvil/architecture.yaml`.
- The first scan and baseline logic must be removed from direct init paths.
- Catalogue changes require regenerated public documentation.
- More up-front contract and tests are required, but repeat runs and protection
  claims become auditable.

## Supersedes and coordinates

- Supersedes the `init` verdict in [ADR-102](102-architecture-cli-surface.md);
  its other command verdicts are unchanged.
- Implements ADR-120's delegated architecture direction without changing
  legacy read compatibility.
- Coordinates with ADR-037 (default acceptance policy), ADR-103 (activation TUI
  consent), ADR-114 (`start` versus daily ensure), ADR-132 (settings truth),
  CIB-267 (Git argv/PATH silent-pass leftovers), and CIB-415 (superseded by
  PSCAF ownership).

## References

- [Project scaffold reconciliation specification](../specs/2026-09-09-project-scaffold-reconciliation.md)
- [PSCAF module](../modules/project-scaffolding.aps.md)
- [ADR-120](120-config-surface-consolidation.md)
- [ADR-102](102-architecture-cli-surface.md)
