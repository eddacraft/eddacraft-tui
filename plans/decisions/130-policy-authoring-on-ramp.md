# ADR-130: Policy authoring on-ramp

## Status

Proposed

## Date

2026-08-23

## Context

Anvil can install, validate, evaluate, and enforce Rego packs, but a team that
did not build the product still has no supported path from "I want a rule" to
"a rule that fires" that does not require hand-writing Rego against
`PolicyInput`. POLFIT-002 exists to pick that path and sequence the candidates
that currently compete for it.

The candidates are:

- **Pack scaffolding.** `anvil policy install anvil-baseline` already writes
  `.anvil/policies/anvil-baseline/`. There is no `anvil policy init` or
  `policy scaffold` command, and the OPAE-017 action plan forbids inventing
  one. Copying that directory and editing its `.rego` members is the only path
  that ships today. It is still hand-written Rego. Thresholds live in-rego
  because `PolicyInput` configuration is unpopulated (OPAE-010 / POLFIT-005).
- **ADR-108 lint, guidance, and skill.** Accepted 2026-07-16 as the
  deterministic authoring *contract* for customer- and agent-written Rego.
  OPAE-012..017 remain Proposed. The public policy-model page already tells
  users to install `authoring-anvil-policy` (OPAE-017), which does not ship.
  Lint does not create a policy and does not avoid Rego.
- **ACTAX YAML → Rego.** The only candidate that can answer "without
  hand-writing Rego." Phase A is a data-only action-taxonomy crate
  (`releaseIntent: never`); it unblocks POLCAP-009 and AGOV-007 but does not
  author, load, or fire a policy. Phase B as currently specified is an
  intercept DSL (`on:` / `when:` / `risk:` / `decision:`) over a parallel
  `.anvil/policies/*.yml` glob, aimed at Phase C `RiskScore` and a
  still-undecided tool-call intercept. That is not the pack a team installing
  `anvil-baseline` is trying to write.

ADR-040 already promised a three-tier authoring model (out-of-the-box, YAML,
Rego as power-user) compiling to one engine, and left the YAML-tier pin to a
separate ADR (D-5). That ADR was never written. POLENG-Y, which would have
owned it, was archived; ACTAX is the living YAML module. This record picks up
that carve-out for the on-ramp only.

Two independent design proposals disagreed on the first promotion: lock the
already-shipped pack-fork as the supported answer and promote OPAE-012; or
make YAML the supported answer and promote a re-scoped ACTAX-010. This
decision takes the second destination and refuses to treat Phase A, an
unexecuted lint chain, or a docs-honesty fix as the on-ramp.

**Assumed authoring target:** packs under `.anvil/policies/`. If POLFIT-001
concludes that surface is not the supported authoring target, this decision
must be re-run.

## Decision

1. **The supported answer to "how do I write a policy?"** is: author YAML
   that Anvil compiles to a pack under `.anvil/policies/<id>/`. Humans and
   agents edit YAML. Generated Rego is an artefact listed in `pack.yaml`.
   Hand-written Rego is the power-user escape hatch, not the door. Compiled
   Rego remains the only artefact that reaches regorus (ADR-040 / ADR-098).
   YAML is never a second production runtime: do not evaluate pack YAML
   through the CPOL Rust assertion evaluator at `anvil gate` or pre-write.

2. **v1 YAML is configurative pack source**, which is the ADR-040 D-5 pin.
   The first slice is a **subset** of CPOL's closed condition enum over the
   shipped `PolicyInput` change set: `changed-path-count`,
   `changed-paths-exclude`, and `changed-paths-confined-to`. It does **not**
   copy the full CPOL enum. v1 omits `config-equals` / `config-present` until
   OPAE-010 populates `input.config`, and omits `change_kind` until
   PolicyInput carries change kinds.

   Mapping to the shipped starter: `change_scope.rego` is a count threshold
   and is in v1. `sensitive_paths.rego` uses case-insensitive substring and
   mixed prefix/suffix heuristics that these globs cannot express; it is
   **out of v1** unless a later decision adds a dedicated operator. Do not
   treat "reuse CPOL" as a round-trip of both starter rules.

   v1 does **not** introduce `on: domain.verb` intercept DSL, a second
   `*.yml` pack glob beside `pack.yaml` + `policies/*.rego`, or
   stringly-typed Rego snippets inside YAML. ACTAX retains execution
   authority; this record narrows Phase B's first slice. The
   intercept/taxonomy DSL stays a later ACTAX slice and is not the on-ramp.

3. **Until ACTAX-012 loads generated packs, the honest interim door** is
   `anvil policy install anvil-baseline` and a copy of that directory to a new
   pack id. POLFIT-003 / OPAE-021 document that door and must stop naming the
   unshipped skill. The interim door is not the supported destination. Do not
   invent `anvil policy init` or `policy scaffold`.

4. **First promotable item: `ACTAX-010`.** The first ACTAX PR applies this
   dialect to ACTAX-010..014 expected outcomes: drop the ACTAX-001
   dependency, drop `on:`/`when:`/`risk:`/`decision:` and the parallel
   `*.yml` pack glob, and load through the existing `pack.yaml` +
   `policies/*.rego` layout. That rewrite *is* promoting ACTAX-010; it is
   not a new APS id. Until it lands, ACTAX-010 is not Ready. Do not promote
   ACTAX-001 as authoring work. Do not promote ACTAX-010 as currently
   specified.

5. **Ordering**

   | Order | Item | Role |
   | ----- | ---- | ---- |
   | 0 (shipped) | Pack install / copy | Interim honesty only |
   | 1 | ACTAX-010 | v1 YAML schema (this dialect) |
   | 2 | ACTAX-011 | YAML → byte-stable Rego compiler |
   | 3 | ACTAX-012 | Load generated members through existing `discover_and_load` / gate admission |
   | 4 | ACTAX-013, ACTAX-014 | Reference YAML pack + public authoring page. The supported answer may appear in docs only from here |
   | *parallel, not on-ramp* | OPAE-012 | Manifest v2 target/input/case contract. Independently promotable as pack-contract work. ACTAX-012 must not land before OPAE-012 so *new* packs declare targets and inputs (legacy packs already fire). Generated members fire when listed in `pack.yaml` **and** gate/pre-write share admission (POLFIT-004 / OPAE-022). The ACTAX rewrite must copy both constraints into `Dependencies:` |
   | 5 | OPAE-013, OPAE-014 | Deterministic lint: safety net for hand-written Rego and a golden check that compiler output is lint-clean |
   | 6 | OPAE-015, OPAE-016, OPAE-017 | Guidance and `authoring-anvil-policy` teach the YAML path first; Rego as escape hatch. Do not ship the skill while it can only route to unshipped commands or a Rego workshop |
   | *parallel, not on-ramp* | ACTAX-001..004 | Action-taxonomy data crate for POLCAP-009 / AGOV-007. May proceed when those modules need `ActionId`. Must not be sold as authoring |

   The on-ramp is real at ACTAX-012: a team can add "flag `infra/`" or
   "count > N" without touching Rego.

6. **ADR-108 is the Rego power-user toolchain**, not the competing on-ramp.
   It is not reopened and it is not first. Implementation still begins at
   OPAE-012.

## Rationale

The adoption question is whether a team that did not build Anvil can go from
intent to a rule that fires without learning Rego. An accepted lint ADR that
never executed still ends in `.rego` files. Phase A never fires a rule.
Shipping the skill first makes a documentation lie locally true. Copying the
starter pack is honesty, not a language. YAML → Rego is the only candidate
that answers the question, but ACTAX's currently specified YAML answers a
future intercept-authoring question. Pinning v1 to a CPOL subset that can
express count thresholds — not the starter's substring heuristics — keeps
one admission layout, one runtime, and a first slice small enough to
finish.

### Alternatives Considered

| Option | Pros | Cons |
| ------ | ---- | ---- |
| YAML pack source first, configurative v1 (chosen) | Answers "without hand-writing Rego"; reuses CPOL shapes; one IR; picks up ADR-040 D-5 | Requires ACTAX-010 re-scope before Ready; compiler is new work |
| Pack-fork as the supported answer; promote OPAE-012 first | Names what ships today; ADR-108 already accepted; smallest immediate promotion | Still Rego; five-item lint/guidance/skill pile before any non-Rego author lands a rule |
| ACTAX-001 first because it is unblocked | Unblocks POLCAP-009 / AGOV-007 | Substrate, `releaseIntent: never`, no user-visible policy |
| ACTAX Phase B as currently specified | Already filed | Intercept DSL cannot express `anvil-baseline`; second pack glob; taxonomy-gated before taxonomy exists |
| Evaluate CPOL YAML in Rust at the gate | Faster; assertion evaluator already exists | Second production policy runtime (forbidden by ADR-040 / ADR-098) |
| OPAE-017 first to match public docs | Closes the lying door | Depends on 014 and 016; still Rego; POLFIT-003 owns the docs fix |
| Invent `anvil policy init` / scaffold | Familiar generator UX | Explicitly out of bounds; `install` already materialises the template |

## Consequences

- **Positive:** the supported authoring answer is a language a team can write;
  public docs have an honest interim door; ADR-108 keeps a job (power-user
  Rego); POLCAP/AGOV can take ACTAX-001 without pretending it is
  authoring-ease.
- **Negative:** ACTAX-010 cannot be promoted from its current text; YAML
  authoring is several items away; the public skill remains unnamed until
  step 6.
- **Risks:** promoting ACTAX-010 without the dialect pin ships the wrong
  language; generated `.rego` that is not listed in `pack.yaml` is invisible
  to one of the two evaluators (POLFIT-004); interpolating user strings into
  generated Rego is an injection bug; keeping the Rust assertion evaluator as
  "temporary" production eval recreates a second runtime; ACTAX-011's own
  confidence is `low` if the compiler is unbounded.
- **Mitigations:** Ready-gate ACTAX-010 against this dialect; loader writes
  generated members into the existing pack layout; glob and threshold
  payloads are emitted as JSON data documents, never interpolated into
  Rego source; every YAML path's only artefact reaching regorus is compiled
  Rego (already an ACTAX acceptance criterion) and YAML never reaches CPOL
  `evaluate()` at gate/pre-write; v1 YAML round-trips `change_scope` plus
  one new path-glob rule — `sensitive_paths` heuristics and anything else
  wait.

## References

- Related ADRs: ADR-040 D-5 (YAML-tier pin, picked up here), ADR-098 (single
  runtime), ADR-108 (Rego power-user toolchain), ADR-027 (pack architecture)
- APS modules: POLFIT-002 (this decision), ACTAX-010..014 (on-ramp),
  OPAE-012..017 (pack contract + power-user path), POLFIT-003 / OPAE-021
  (interim docs), CPACKS (starter pack), CPOL (closed assertion conditions)
- Evidence: policy-capability audit 2026-08-22 against `origin/main` @
  `7524a599b`
- Numbering: this record is ADR-130 because POLFIT-001 already holds
  ADR-129 (`129-policy-surface-inventory-and-precedence.md`) in a
  parallel worktree. If that item never lands, a later renumber-on-rename
  should backfill 129 (ADR process gap rule).
