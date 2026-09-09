# Project scaffold reconciliation

| Field | Value |
| --- | --- |
| Status | Final, operator-approved 2026-09-09 |
| Date | 2026-09-09 |
| Decision | [ADR-143](../decisions/143-project-scaffold-reconciliation.md) |
| APS | [PSCAF](../modules/project-scaffolding.aps.md) |

## 1. Summary

`anvil init` is the idempotent project-scaffold reconciler. It creates or
completes the files that let an operator configure anvil manually, without
activating runtime integrations. `anvil start` consumes that same reconciler as
the first section of its activation plan, then applies the opinionated runtime
overlay. There is one scaffold catalogue and one reconciliation engine, so the
two commands cannot drift.

The scaffold has a mandatory foundation and optional protection components.
The foundation comprises the main project configuration, project identity,
ignore rules, and scaffold-version metadata. In an interactive terminal the
operator then answers:

> Which protections would you like to configure?

The choices include acceptance policy, detected architecture, check and
enforcement posture, and planning support. A selected component creates only
the artefacts needed to make that advertised protection functional. No empty
placeholder or state file is created merely to make the tree look complete.

## 2. Product boundary

### `anvil init`

Direct `init` owns project configuration only. It may create or add missing
scaffold-owned sections after showing a semantic proposal and obtaining any
required consent. It validates what it wrote and reports a structured outcome
for every considered component.

It does not start a daemon, install hooks, configure MCP clients, create CI
workflows, run the first repository scan, create a baseline, or activate witness
capture. It ends with a stable link to the public configuration-options page and
an exact rerun command for anything skipped or requiring input.

### `anvil start`

`start` puts scaffold selection at the beginning of its existing activation
plan; it does not launch a nested `init` wizard. It requests the recommended
scaffold profile, resolves component dependencies, then overlays runtime
activation such as daemon, hooks, MCP, workflows, first scan, baseline, and
witness setup according to the existing consent contracts.

The first scan belongs only to `start` and flows derived from it. Moving it out
of `init` keeps configuration authoring deterministic and prevents direct init
from silently becoming activation.

## 3. Catalogue and components

One typed scaffold catalogue is the source for:

- interactive labels, descriptions, recommendations, and dependency messages;
- non-interactive profile membership;
- component-to-file and component-to-config-section ownership;
- detection requirements and confidence handling;
- structured outcomes and exact rerun commands; and
- the generated public **Project configuration options** page.

Every catalogue entry states its stable identifier, purpose, files or sections
owned, whether it is recommended, how suitability is detected, prerequisites,
the consequence of leaving it inactive, and how to configure it later.

The scaffold catalogue owns composition metadata only. It references canonical
setting identifiers and writer identities from ADR-132's settings catalogue;
it does not duplicate their type, scope, merge, sensitivity, mutability, or
source metadata. Missing bootstrap settings are added to that catalogue, and
all main-config mutations route through a bounded settings-service bootstrap
operation. A parity check fails on an unknown setting id, mismatched target, or
non-settings writer. Non-config artefacts retain their existing domain writers
(identity, acceptance policy, and architecture definition).

The initial component set is:

| Component | Purpose | Functional artefacts | Recommended profile |
| --- | --- | --- | --- |
| `foundation` | Give the project a stable anvil identity and reconciliation version | Main config, project id, ignore rules, scaffold-version metadata | Mandatory |
| `acceptance-policy` | Make L4 acceptance evaluation reachable | A discoverable acceptance policy with the ADR-037 default branch posture | Yes |
| `architecture` | Declare repository boundaries suitable for architecture validation | Tracked `anvil/architecture.yaml` plus `architecture.source` in the main config | Yes when detection is high confidence; otherwise `needs_input` |
| `checks` | Declare the desired check and enforcement posture | Only the main-config sections required by the selected posture | Yes |
| `planning` | Configure supported planning integration | Only the selected planning configuration and functional APS starter | Yes |

Shared-file ownership is disjoint and pointer-specific:

| Component | Canonical settings / paths owned for bootstrap |
| --- | --- |
| `foundation` | Main-config `/schema_version` and `/format`; `anvil/project-id` fields `project_uuid`, `created_at`, `created_by_version`, and `scaffold_version`; exact `.gitignore` lines `.anvil/`, `anvil/exceptions/.lock`, and `anvil/witness/.chain-initialised` |
| `acceptance-policy` | Whole newly created `anvil/policy.yml` only; any existing `anvil/policy.{yaml,yml,json,toml}` is operator-owned and immutable to scaffold reconciliation |
| `architecture` | Main-config `/architecture/source`; whole newly created `anvil/architecture.yaml` |
| `checks` | Main-config `/checks` and `/enforcement/mode`, referenced as settings keys `protection.checks` and `protection.enforcement.mode` |
| `planning` | Main-config `/planning_dir`, referenced as new settings key `project.planning.dir`; a new `plans/index.aps.md` only when the selected directory is absent |

The main config file itself has no component owner. No scaffold force operation
may replace config settings or identity metadata. Existing `project_uuid`,
`forked_from`, `first_commit`, and `origin_canonical` fields are immutable here;
identity rotation remains exclusively `--new-identity`, including its lineage
and baseline reconciliation. `scaffold_version` is integer schema `1`, tracked
inside `anvil/project-id`, and owned by foundation; `created_by_version` remains
the anvil binary version. The planning starter is a minimal valid APS index, not
an empty directory or state marker. A pre-existing planning directory or index
is preserved and its health reported.

Runtime integrations are not scaffold components. The catalogue may describe a
dependency from a runtime protection to a scaffold component; for example, an
L4 pre-push hook requires `acceptance-policy`.

`foundation` is mandatory rather than a normal selectable component. Every
other component and runtime integration depends on a validated foundation.
`--skip foundation` is invalid input: it fails before planning or writing
anything.

## 4. Reconciliation and ownership

Reconciliation is additive by default:

1. Missing scaffold files and keys may be proposed.
2. Any existing path or key is operator-owned and is never silently changed,
   removed, merged, normalised, or reformatted.
3. A missing scaffold-owned section may be added to an existing config only
   after a semantic diff and consent where the active interface permits it.
4. Existing values remain authoritative even when they differ from catalogue
   defaults.
5. New-file writers use race-safe exclusive creation or an equivalent
   descriptor-bound operation; a parent-path swap cannot redirect a validated
   write.
6. Existing-file augmentation requires the repository's shared mutation lock on
   every supported platform. If the lock is unavailable, the component reports
   `needs_input` and emits a patch rather than writing. Under that lock, planning
   records the no-follow file identity and digest, reopens and verifies the same
   snapshot immediately before atomic publication, and refuses with a fresh plan
   if it changed.

The shared lock is an OS advisory exclusive lock on
`<git-common-dir>/anvil/config-mutation.lock`, so linked worktrees coordinate on
one object and a crashed process releases the held lock. PSCAF-001 moves every
anvil main-config writer onto this primitive. A non-Git directory has no common
metadata authority: it may create absent files exclusively, but augmentation of
an existing file emits a patch and reports `needs_input`.

For YAML/YML, JSON, and TOML, augmentation uses syntax-span insertion that
preserves comments, key order, whitespace, and unrelated bytes. If a safe
insertion point cannot be proven, the component reports `needs_input` with a
patch preview; it does not fall back to whole-document serialisation.

Multi-file components pre-render and validate every proposed artefact before
the first write, then commit under the mutation lock in dependency-safe order.
Unreferenced targets are created before config pointers; shared-file CAS updates
are last. If a later write fails, newly created artefacts are removed only when
their no-follow identity and digest still match this transaction. A crash may
leave an unreferenced generated target but never a dangling pointer. The next
run recognises an exact generated residue and resumes; any differing residue is
operator-owned and reports `needs_input`. Foundation similarly writes config
and identity before ignore hints; no protection is valid until the complete
foundation validates. Failure injection at every boundary proves rollback,
safe residue, and repeat-run recovery without a durable transaction state file.

The no-lost-update guarantee covers every cooperating anvil writer, all of which
must use the shared lock. An arbitrary external editor can ignore that protocol;
the CLI warns before existing-file augmentation and can detect changes only up
to its final identity/digest check. Operators needing exclusion against external
editors use the emitted patch path instead of automatic augmentation.

`--force` is scoped, never ambient. It is accepted only with one or more
explicit `--include <component>` selectors, displays the component-scoped diff,
and may replace only artefacts that the catalogue explicitly marks replaceable.
In the initial catalogue, the only replaceable artefact is a previously
generated `anvil/architecture.yaml` whose provenance comment names its scaffold
version and template id and whose bytes match the catalogue's digest for that
recorded version. No separate state file stores this digest. Modified
architecture, main-config keys, identity,
ignore rules, policy, and planning content are not force-replaceable. A named
component with no replaceable artefact and bare `--force` both refuse with an
actionable error.

Acceptance policies are never force-replaceable through scaffold reconciliation,
even when `acceptance-policy` is named. Existing policy variants remain
byte-preserved. Replacing policy is a separate policy-authoring workflow with an
old/new semantic diff and explicit confirmation for any weakening.

On repeat runs, the default view shows the currently configured components and
the missing or previously skipped choices. An explicit **review all** action
shows the complete catalogue.

## 5. Interfaces

Interactive use presents the component checklist. Non-interactive use selects
the recommended profile unless the caller chooses another profile or component
override:

```text
anvil init [--profile core|recommended|all]
           [--include <component>]...
           [--skip <component>]...
           [--force]
```

`--include` and `--skip` are repeatable. Explicit component selectors override
profile membership; a skip wins over an include and is reported. Dependency
resolution happens before writes. Skipping a prerequisite deselects dependent
protections and explains the consequence instead of creating a knowingly inert
configuration.

Unknown profile or component identifiers are invalid input and fail before any
write. Repeated selectors are de-duplicated. Including and skipping the same
optional component resolves to one reported `skipped` outcome. Explicitly
including foundation is a harmless de-duplicated no-op; skipping it is invalid.

| Profile | Membership |
| --- | --- |
| `core` | Mandatory foundation only |
| `recommended` | Foundation plus every catalogue component marked recommended and suitable for the detected repository |
| `all` | Foundation plus every known optional component; an unsafe or uncertain component reports `needs_input` rather than fabricating configuration |

Interactive optional choices remain unticked by default under ADR-103; the
recommended marker is explanation, not implicit consent.

Fresh checks use the settings catalogue defaults exactly:
`protection.checks = [secret-detection, import-boundaries, antipattern-scan]`
and `protection.enforcement.mode = warn`. Interactive posture labels are
**Off**, **Warn (recommended)**, and **Enforce**, carrying canonical setting
values `off`, `warn`, and `enforce`; the bootstrap storage adapter maps
`enforce` to the shipped main-config spelling `block`. Interactive check
selection may replace the default set only with stable identifiers from the
authoritative check catalogue. Fresh planning uses
`project.planning.dir = plans`.

Machine-readable output reports one of these states for every considered
component:

- `created` — new functional artefacts were written;
- `configured` — missing declarations were added safely;
- `preserved` — existing operator-owned configuration already satisfies the
  component or was intentionally left untouched;
- `skipped` — the profile or operator excluded it; or
- `needs_input` — a safe choice could not be made without operator input.

Mutation outcome is not health. Every considered component also reports
`valid`, `invalid`, `conflict`, or `unknown` health with diagnostics and
evidence. `preserved` means only that bytes or values were not changed; it does
not imply the component works. A dependant may be selected or activated only
when every prerequisite is `valid`. Invalid project ids, malformed policies,
ambiguous policy variants, and dangling architecture sources therefore block
dependent protections while remaining operator-owned.

The exit contract distinguishes invalid input or failed writes from a successful
run containing `skipped` or `needs_input` components. Exact exit-code mapping is
owned by the implementation item and must reuse the global CLI registry.

### `start` machine and read-only modes

ADR-103 remains binding. `anvil start --verify` and `anvil start --json` are
read-only: they evaluate scaffold health but neither plan nor apply mutations,
and their existing stdout fixtures remain byte-identical. No new output field is
added without a separately versioned machine-contract amendment. Piped,
non-interactive mutable start resolves the recommended scaffold profile and may
apply its safe additive plan under the existing non-interactive auto policy, but
its compact stdout fixture remains byte-identical; diagnostic detail uses the
existing stderr channel. Gated project-write environments perform no scaffold
write and report the existing gated state. The interactive TUI/plain plan is the
surface that visibly renders scaffold selection as its first section.

## 6. Architecture inference

Architecture configuration is a selectable scaffold component. Detection
examines bounded repository evidence and offers a suitable tracked definition;
the operator may skip it.

The initial detector is deterministic:

| Candidate | Required evidence | Scored evidence tokens (one point each) |
| --- | --- | --- |
| `layered` | At least three distinct allowlisted layer groups | `presentation`/`ui`/`controllers`; `application`/`services`/`use_cases`; `domain`; `infrastructure`/`data`/`repositories` |
| `hexagonal` | `domain` plus both a ports and adapters directory | `domain`; `ports`; `adapters`; `inbound`; `outbound` |
| `workspace-boundaries` | A supported root manifest declares at least two resolvable local members | One point per member with its own supported manifest, capped at four points |

Each semicolon-separated conceptual synonym group contributes at most one
point, even when several synonyms are present. Evidence is directory names and
membership declarations only from
`Cargo.toml`, `package.json`, `pnpm-workspace.yaml`, `go.work`, and
`pyproject.toml`. The scan does not read source files, follow symlinks, or enter
the fixed built-in directory set `.git`, `.anvil`, `node_modules`, `target`,
`vendor`, `dist`, `build`, `generated`, `.generated`, or `gen`. Git global
excludes, `.git/info/exclude`, and `.gitignore` do not affect inference; this
fixed rule makes classification checkout-independent. It stops at depth four,
4,096 directory entries,
64 manifests, or 1 MiB of combined manifest bytes. Hitting a bound makes health
`unknown` and the result `needs_input`. No wall-clock threshold participates in
the result, so the same tree cannot change classification with machine speed.

A candidate is high confidence when its required evidence holds, its score is
at least four, and it leads the next candidate by at least two points. Two
qualifying candidates separated by fewer than two points are ambiguous. No
qualifying candidate is low confidence. Ties are never broken by iteration
order. `workspace-boundaries` participates in the same scoring and ambiguity
rules; it has no precedence over layered or hexagonal evidence. When it wins,
the generated definition is rooted at workspace members; member-local models
are reported as evidence but are not merged. Close workspace-plus-layered or
workspace-plus-hexagonal scores are therefore ambiguous. Heterogeneous member
models need input in the initial release.

Every declared workspace member or glob prefix is normalised before expansion
and must remain a repository-relative descendant. Absolute, parent-relative,
UNC, device-prefixed, symlink, and reparse-crossing paths are rejected before
any read. Expanded members are containment-checked again. One rejected member
produces a diagnostic and `needs_input`; architecture output never references a
path outside the authorised repository. Fixtures cover Unix, drive-letter, UNC,
device, traversal, glob, symlink, and reparse shapes.

- High-confidence interactive detection presents the inferred model and diff
  before consent.
- Ambiguous interactive detection offers suitable alternatives and skip.
- Low-confidence non-interactive detection writes nothing, reports
  `needs_input`, and prints the exact rerun command.
- Detection never invents a generic architecture merely to satisfy the profile
  and never fails the rest of scaffold reconciliation.

The generated definition is `anvil/architecture.yaml`, referenced through
`architecture.source` in the canonical main config. This is a deliberate move
away from generating the ignored legacy `.anvil/architecture.yaml`; existing
legacy delegation remains readable under ADR-120.

## 7. L4 activation honesty

An L4 protection is active only when all required evidence exists: a valid
project identity, an active pre-push hook, and a discoverable, parseable
acceptance policy. `anvil status` must not infer L4 from hook presence alone.

`anvil doctor` parses and diagnoses the same policy source as status and the L4
entry points. Public guidance lists every intentional silent-pass condition:
missing policy, missing project identity, an allowed or clean pushed range, and
the hook's `command -v anvil` guard when Git's PATH cannot resolve the binary.
It also explains that `l4_or_l3` intentionally accepts a valid L3 witness and
gives a deterministic `l4_only` exercise using the accepted hex-SHA range.

This contract does not change `l4_or_l3`, Serena's internal-error posture,
`on_warn: allow`, or existing acceptance-policy content.

## 8. Documentation contract

The public **Project configuration options** page is generated from the same
catalogue used by the CLI. A docs check fails when generated output is stale.
Narrative guides may explain workflows but must link to that page rather than
maintaining a second component inventory.

## 9. Acceptance journeys

1. A fresh interactive `anvil init` creates the foundation, lets the operator
   select protections, validates the resulting scaffold, and activates nothing.
2. A fresh non-interactive init applies the recommended profile; uncertain
   architecture is `needs_input`, not fabricated.
3. Re-running init preserves existing bytes and values, showing only missing or
   skipped components unless **review all** is chosen.
4. `anvil start` displays scaffold choices in its activation plan, satisfies
   prerequisites through the shared engine, then performs runtime activation
   and the first scan.
5. Skipping acceptance policy prevents selection of the dependent L4 hook and
   explains how to enable it later.
6. An existing config receives only consented missing sections; a conflicting
   existing value is preserved.
7. Scoped force cannot affect an unnamed component and bare force refuses.
8. L4 status and doctor agree for missing identity, missing/invalid policy, and
   active hook-plus-policy cases.
9. The CLI catalogue and public options page are proven to derive from the same
   source.
10. `--skip foundation` fails before writes; preserved-but-invalid prerequisites
    cannot activate dependants; concurrent edits cause re-plan rather than lost
    updates; and existing policies, identity, planning, and strict enforcement
    remain byte-identical even under scoped force.
11. Read-only and piped start fixtures remain byte-identical; multi-file failure
    injection never leaves a dangling config pointer; profile/selector golden
    tests and detector fixtures cover every normative row and bound.

## 10. Explicit non-goals

- Changing policy evaluation semantics or policy precedence.
- Replacing the settings truth service or creating a second general settings
  writer; scaffold reconciliation is the bounded bootstrap path recognised by
  the settings specification.
- Creating runtime state during direct init.
- Treating generated architecture as authoritative when detection confidence is
  low.
- Silently rewriting existing project configuration.
