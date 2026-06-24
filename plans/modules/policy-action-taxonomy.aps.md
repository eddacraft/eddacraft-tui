<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Policy Action Taxonomy & YAML Authoring Tier

| ID | Owner | Priority | Status   |
| ----- | ----- | -------- | -------- |
| ACTAX | —     | medium   | Proposed |

**Last reviewed:** 2026-05-22

> Inspired by [permit0](https://github.com/permit0-ai/permit0) (Apache 2.0).
> Anvil already owns the engine (regorus / ADR-040), intercept loop (ADR-015),
> pack architecture (ADR-027), and witness chain (ADR-037). This module adds
> the **authoring substrate** that's missing: a canonical action vocabulary,
> a YAML-first policy DSL that compiles to Rego, and risk-score fusion into
> the existing intercept routing. Tool-call interception is held out as
> Phase D pending a separate decision.
>
> **Policy-solution validation (2026-06-24):** ACTAX is aligned with the
> current direction because YAML compiles to Rego and regorus remains the single
> evaluation engine. Package validation commands were normalised to the
> workspace's `eddacraft-*` Cargo package names; any new crates introduced by
> this module should follow the same naming convention.

## Purpose

Give non-Rego authors a config-file path to express policy intent — `on:
email.send when: outbound … decision: tier=high` — while keeping regorus
as the single evaluation engine. Adds a stable, append-only **action
taxonomy** (noun.verb) that lets one rule cover every tool that performs
the same logical action, and fuses a thin risk score into the intercept
daemon's existing `warn / fence / interrupt` routing so authors can
declare risk dimensions rather than wiring every threshold by hand.

## In Scope

- Canonical action taxonomy: `<domain>.<verb>` vocabulary, append-only,
  versioned, shipped as a data crate (no engine code)
- YAML policy pack schema (`.anvil/policies/*.yml`) covering: `on:` (verb
  match), `when:` (predicate), `risk:` (dimension scores), `amplifiers:`
  (conditional bumps), `decision:` (tier output)
- YAML → Rego compiler executed at pack load; Rego remains the IR
- `RiskScore` output type on `anvil-policy-engine` alongside `EvalResult`,
  ~5 dimensions aligned with IORISK (destruction, outbound, sensitivity,
  irreversibility, scope)
- Intercept routing consumes `RiskScore`: thresholds map to existing
  `warn / fence / interrupt` modes (ADR-015); binary `allow | interrupt`
  contract preserved at the daemon boundary
- AGOV trust score wired as the **session amplifier** stage (one of the
  amplifier inputs, not the only one)
- Authoring docs + one reference YAML pack to prove the round-trip
- Acknowledgements entry citing permit0 (attribution per Apache 2.0)

## Out of Scope

- ❌ **Tool-call interception** — held as Phase D below pending a separate
  decision (different threat model from save-time intercept; needs its own
  ADR and crate, see Risks)
- ❌ MCP server adapters, agent-side enforcement shims
- ❌ Replacing Rego — regorus stays the single evaluation engine (ADR-040)
- ❌ Replacing the existing pack architecture (ADR-027) — YAML packs are an
  authoring tier *above* compiled-in Rust packs, not a replacement
- ❌ Cryptographic signing of the witness chain (separate work; ADR-037
  acknowledges "provenance, not authentication")
- ❌ Lifting permit0's full 22 × 159 taxonomy verbatim — curated subset only
  (see D-ACTAX-001)
- ❌ Compliance-pack content (owned by CPACKS); ACTAX provides authoring
  primitives, not framework coverage
- ❌ Trust scoring algorithm itself — that's AGOV-001's scope; ACTAX only
  consumes the score as an amplifier input

## Interfaces

**Depends on:**

- ADR-040 — Rust policy engine (regorus) — substrate this module composes on
- ADR-027 — Pack architecture — coexistence contract between Rust packs and
  YAML packs
- ADR-015 — Intercept loop enforcement — routing surface for `RiskScore`
- `crates/anvil-policy-engine` — facade that gains a `RiskScore` output
- `crates/anvil-policy` — pack loader gains YAML pack support
- `crates/anvil-intercept-rules` — rule registry gains risk-score consumer
- IORISK-001..006 — risk taxonomy dimensions (must reach Ready before
  ACTAX-C tasks; coordinate, do not duplicate)
- AGOV-001 — trust scoring (amplifier input; ACTAX-C can stub if AGOV
  remains Draft)
- CPOL — contextual policy assertions (sibling authoring surface; resolve
  schema overlap before ACTAX-B starts)

**Exposes:**

- `crates/anvil-action-taxonomy/` — data crate: `taxonomy.yml`, serde
  types, version constant
- YAML pack schema published under `schemas/policy-pack.schema.json`
- `crates/anvil-policy::yaml` — YAML loader + Rego compiler
- `RiskScore` output type and threshold-routing contract documented for
  intercept consumers
- One reference pack: `library/policy/reference/safe-defaults.yml`
- Public docs: `docs/public/anvil/policy-yaml-authoring.md`

## Acceptance Criteria

- [ ] Action taxonomy crate compiles; versioned; append-only invariant
      enforced by a test (`cargo test -p eddacraft-anvil-action-taxonomy`)
- [ ] YAML pack schema validates a known-good and a known-bad fixture
- [ ] YAML pack compiles to Rego with byte-stable output across runs
      (determinism, per planless-first principle)
- [ ] Reference pack evaluates against fixture inputs and returns a
      `RiskScore` plus a tier decision
- [ ] Intercept daemon routes `RiskScore` to `warn / fence / interrupt`
      without any change to its IPC contract
- [ ] Authoring guide is sufficient for an engineer with no Rego experience
      to write and ship a working YAML pack
- [ ] permit0 attribution present in `ACKNOWLEDGEMENTS.md` and any vendored
      taxonomy file carries its source comment + Apache 2.0 NOTICE
- [ ] No regression in `cargo test -p eddacraft-anvil-policy-engine` or
      `cargo test -p eddacraft-anvil-intercept-rules`
- [ ] No new dependency on a Rust daemon, axum, biscuit-auth, or MCP
      server (we already have equivalents)

## Risks & Mitigations

| Risk | Mitigation |
| ---- | ---------- |
| Two parallel authoring surfaces (YAML + Rego) confuse users | Document YAML as easy-mode, Rego as power-user; YAML compiles to Rego so there is one IR |
| Taxonomy drift if we don't track upstream permit0 | Pin the version copied; document divergence policy; append-only rule means we don't *need* to track |
| Risk-score thresholds become tribal knowledge | Ship calibrated defaults with the reference pack; tests assert each tier boundary |
| Phase D (tool-call intercept) gets rushed into Phase A | Out-of-scope is enforced — Phase D requires its own ADR before any task is promoted to Ready |
| Trademark exposure from "permit0" name | Don't use the name in product surfaces; attribution-only in ACKNOWLEDGEMENTS |
| Schema overlaps with CPOL assertion schema | Coordinate with CPOL owner before ACTAX-B; resolve via shared types or explicit non-overlap |
| YAML compiler becomes a parallel engine over time | Test that compiled Rego is the only artefact reaching regorus; CI fails if YAML bypasses compilation |
| Tool-call threat model debate stalls Phase A–C | Phase A–C deliver value at save-time without Phase D; sequence is intentional |

---

## Work Items

> All tasks are **Proposed**. None executable until module status advances
> to Ready and IORISK / CPOL coordination is resolved.

### Phase A: Action Taxonomy (data substrate)

#### ACTAX-001: Establish action taxonomy crate

- **Intent:** Create `crates/anvil-action-taxonomy/` as a pure data crate
  with serde types and a curated taxonomy file.
- **Expected Outcome:** Crate compiles, exposes `Taxonomy`, `Domain`,
  `Verb` types, and a `TAXONOMY: &Taxonomy` constant loaded from
  `taxonomy.yml`. No engine code, no runtime dependencies beyond serde.
- **Scope:** `crates/anvil-action-taxonomy/`
- **Non-scope:** Loading, evaluation, or pack integration
- **Files:**
  - `crates/anvil-action-taxonomy/Cargo.toml`
  - `crates/anvil-action-taxonomy/src/lib.rs`
  - `crates/anvil-action-taxonomy/taxonomy.yml`
- **Validation:** `cargo test -p eddacraft-anvil-action-taxonomy`
- **Confidence:** high
- **changeType:** internal
- **releaseIntent:** never
- **releaseScope:** none

#### ACTAX-002: Curate initial taxonomy from permit0 reference

- **Intent:** Pull permit0's taxonomy as a starting reference and curate
  down to the domains Anvil's save-time and (future) tool-call surfaces
  actually use.
- **Expected Outcome:** `taxonomy.yml` covers code / fs / net / cmd /
  secret / agent domains at minimum; explicitly drops domains Anvil will
  not enforce (payment, physical, etc.). Each retained entry has a source
  comment and the file carries the Apache 2.0 NOTICE.
- **Non-scope:** Verb additions outside the curated domains
- **Dependencies:** ACTAX-001
- **Validation:** `cargo test -p eddacraft-anvil-action-taxonomy -- curation`
- **Confidence:** medium
- **changeType:** internal
- **releaseIntent:** never
- **releaseScope:** none

#### ACTAX-003: Enforce append-only invariant

- **Intent:** Lock the taxonomy file so verbs cannot be removed or
  renamed once published.
- **Expected Outcome:** A test compares the working copy against a pinned
  manifest and fails on removal / rename. Additions pass. Bump procedure
  documented in the crate README.
- **Dependencies:** ACTAX-002
- **Validation:** `cargo test -p eddacraft-anvil-action-taxonomy -- append_only`
- **Confidence:** high
- **changeType:** internal
- **releaseIntent:** never
- **releaseScope:** none

#### ACTAX-004: Attribution + acknowledgements entry

- **Intent:** Record the permit0 source and Apache 2.0 attribution in
  `ACKNOWLEDGEMENTS.md` and the taxonomy file header.
- **Expected Outcome:** Attribution pipeline (ATTRIB) recognises the new
  source; license-field lint passes.
- **Dependencies:** ACTAX-002
- **Coordinates with:** ATTRIB (attribution-pipeline-v3)
- **Validation:** `pnpm acknowledgements:check` (or current ATTRIB
  validation command)
- **Confidence:** high
- **changeType:** docs
- **releaseIntent:** candidate
- **releaseScope:** none

### Phase B: YAML Policy Pack Authoring

#### ACTAX-010: Define YAML policy pack schema

- **Intent:** Specify the YAML schema for `.anvil/policies/*.yml` packs.
- **Expected Outcome:** JSON Schema published under `schemas/` covering
  `on:`, `when:`, `risk:`, `amplifiers:`, `decision:` sections. A good
  fixture and a bad fixture both round-trip through the validator with
  the expected result.
- **Scope:** `schemas/policy-pack.schema.json`,
  `crates/anvil-policy/src/yaml/`
- **Non-scope:** Compilation to Rego (ACTAX-011)
- **Dependencies:** ACTAX-001, coordination resolution with CPOL
- **Coordinates with:** CPOL (contextual-policy-assertions)
- **Validation:** `cargo test -p eddacraft-anvil-policy -- yaml_schema`
- **Confidence:** medium
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

#### ACTAX-011: YAML → Rego compiler

- **Intent:** Compile a validated YAML pack into a Rego module
  consumable by `anvil-policy-engine`.
- **Expected Outcome:** Compiler emits byte-stable Rego for a given YAML
  input (determinism). Compiled Rego evaluates against fixture inputs and
  matches the YAML author's stated intent.
- **Non-scope:** Risk-score wiring (Phase C)
- **Dependencies:** ACTAX-010, POLENG facade reaching usable state
- **Validation:** `cargo test -p eddacraft-anvil-policy -- yaml_compile`
- **Confidence:** low
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

#### ACTAX-012: Pack loader integration

- **Intent:** Teach `anvil-policy` to discover and load YAML packs
  alongside compiled-in Rust packs.
- **Expected Outcome:** A directory of YAML packs loads at startup, each
  contributing compiled Rego modules. Errors surface with file:line
  pointing to the YAML source, not the generated Rego.
- **Dependencies:** ACTAX-011
- **Validation:** `cargo test -p eddacraft-anvil-policy -- yaml_loader`
- **Confidence:** medium
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

#### ACTAX-013: Reference YAML pack

- **Intent:** Ship one realistic YAML pack as documentation-by-example
  and as a regression fixture.
- **Expected Outcome:** `library/policy/reference/safe-defaults.yml`
  covers a small set of taxonomy verbs, evaluates end-to-end, and is
  cited from the authoring guide.
- **Dependencies:** ACTAX-012
- **Validation:** `cargo test -p eddacraft-anvil-policy -- reference_pack`
- **Confidence:** high
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

#### ACTAX-014: Authoring guide

- **Intent:** Document how an engineer with no Rego experience writes
  and ships a YAML pack.
- **Expected Outcome:** Public docs page covering schema, examples,
  validation commands, and the YAML → Rego mental model.
- **Dependencies:** ACTAX-013
- **Coordinates with:** DOCSYNC (public docs sync)
- **Validation:** Manual review + `pnpm docs:check`
- **Confidence:** high
- **changeType:** docs
- **releaseIntent:** candidate
- **releaseScope:** none

### Phase C: Risk-Score Fusion with Intercept Routing

#### ACTAX-020: Add `RiskScore` output to policy engine

- **Intent:** Extend `anvil-policy-engine` so evaluation can return a
  structured `RiskScore` alongside `EvalResult`.
- **Expected Outcome:** Engine facade exposes `eval_with_risk()` (or
  equivalent); existing `eval()` callers unaffected. Score dimensions
  align with IORISK taxonomy.
- **Scope:** `crates/anvil-policy-engine/`
- **Non-scope:** Intercept routing changes (ACTAX-022)
- **Dependencies:** ACTAX-011, IORISK-001..006 at Ready
- **Coordinates with:** IORISK (io-risk-controls), POLENG (policy-engine)
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- risk_score`
- **Confidence:** medium
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

#### ACTAX-021: Wire AGOV trust score as session amplifier

- **Intent:** Plumb AGOV-001's trust score into the YAML pack
  `amplifiers:` section as a recognised input.
- **Expected Outcome:** A YAML pack can express "if trust < threshold
  then scope+2" and the score reaches evaluation as deterministic
  context. AGOV unavailable → amplifier evaluates to neutral, never an
  error.
- **Dependencies:** ACTAX-020, AGOV-001 (stubbed if AGOV remains Draft)
- **Coordinates with:** AGOV (agent-governance-patterns)
- **Validation:** `cargo test -p eddacraft-anvil-policy -- trust_amplifier`
- **Confidence:** low
- **changeType:** feature
- **releaseIntent:** hold
- **holdCondition:** AGOV-001 reaches at least Proposed with a stable
  score schema
- **releaseScope:** minor

#### ACTAX-022: Route `RiskScore` to intercept tiers

- **Intent:** Map `RiskScore` thresholds to the existing
  `warn / fence / interrupt` modes without changing the daemon's IPC
  contract.
- **Expected Outcome:** Intercept rule registry consumes `RiskScore`;
  thresholds are configurable; daemon still emits `allow | interrupt`
  externally. Tests cover each tier boundary.
- **Scope:** `crates/anvil-intercept-rules/`
- **Non-scope:** New IPC surface; new daemon flags
- **Dependencies:** ACTAX-020
- **Coordinates with:** INTR (intercept-rules)
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules -- risk_routing`
- **Confidence:** medium
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

#### ACTAX-023: End-to-end fixture: YAML pack → intercept decision

- **Intent:** Prove the round-trip from a YAML pack to a daemon-side
  enforcement decision against a known fixture.
- **Expected Outcome:** Integration test loads the reference pack,
  triggers a fixture change matching a high-risk taxonomy verb, and
  observes the expected `interrupt` decision in the daemon log.
- **Dependencies:** ACTAX-013, ACTAX-022
- **Validation:** `cargo test --workspace -- actax_e2e`
- **Confidence:** medium
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### Phase D: Tool-Call Interception _(deferred — separate decision required)_

> **Status note:** Phase D tasks are listed for visibility only and MUST
> remain Proposed until a dedicated ADR captures the tool-call
> interception decision. Phase A–C deliver value at save-time without
> Phase D. Promotion of any D-task to Ready requires explicit operator
> authorisation and the ADR landing in `plans/decisions/`.

#### ACTAX-D01: ADR — tool-call interception decision

- **Intent:** Capture the architectural decision on whether Anvil
  intercepts agent tool calls (pre-execution) in addition to file saves
  (post-generation), and how it would compose with `anvil-intercept`.
- **Expected Outcome:** New ADR in `plans/decisions/` covering threat
  model, interception point (MCP shim vs. agent harness vs. hook),
  latency budget, failure mode (fail-open vs. fail-closed), and the
  relationship to the existing intercept daemon.
- **Validation:** ADR accepted per ADR process; entry added to
  `DECISION-LOG.md`
- **Confidence:** low
- **changeType:** docs
- **releaseIntent:** never
- **releaseScope:** none

#### ACTAX-D02: `anvil-tool-intercept` crate skeleton

- **Intent:** Stand up a parallel intercept crate that operates on
  tool-call payloads rather than filesystem changes.
- **Expected Outcome:** Crate compiles; exposes a synchronous evaluation
  surface returning `Allow | Review | Deny`; consumes the same YAML
  packs and `RiskScore` produced by Phase C.
- **Non-scope:** MCP wiring, agent harness integration
- **Dependencies:** ACTAX-D01 accepted
- **Validation:** `cargo test -p eddacraft-anvil-tool-intercept`
- **Confidence:** low
- **changeType:** feature
- **releaseIntent:** hold
- **holdCondition:** ACTAX-D01 accepted; explicit operator authorisation
- **releaseScope:** minor

#### ACTAX-D03: Sub-millisecond evaluation budget proof

- **Intent:** Show that a representative pack evaluates within the
  latency budget the D01 ADR commits to.
- **Expected Outcome:** A criterion benchmark publishes p50 / p99
  latency for evaluating the reference pack on a representative input;
  results land in `.bench-logs/`.
- **Dependencies:** ACTAX-D02
- **Validation:** `cargo bench -p anvil-tool-intercept`
- **Confidence:** low
- **changeType:** internal
- **releaseIntent:** hold
- **holdCondition:** ACTAX-D01 accepted
- **releaseScope:** none

#### ACTAX-D04: Agent surface wiring _(placeholder — scope deferred)_

- **Intent:** Connect `anvil-tool-intercept` to the actual agent
  surface(s) we choose to govern.
- **Expected Outcome:** TBD — depends on D01 outcome. Listed for
  visibility; do not refine until D01 is accepted.
- **Dependencies:** ACTAX-D01, ACTAX-D02, ACTAX-D03
- **Validation:** TBD
- **Confidence:** low
- **changeType:** feature
- **releaseIntent:** hold
- **holdCondition:** ACTAX-D01 accepted
- **releaseScope:** minor

---

## Decisions

### D-ACTAX-001: Curate the taxonomy rather than lift it verbatim

- **Rationale:** permit0's 22 × 159 vocabulary includes domains Anvil
  does not enforce (payment, physical). Shipping the full set creates
  false expectations and bloats the data crate.
- **Alternatives:** Lift verbatim; rebuild from scratch.
- **Trade-offs:** Verbatim lift is cheaper but misleading; from-scratch
  loses the calibration permit0 has already done. Curation keeps the
  good parts and trims the misleading ones.

### D-ACTAX-002: Rego stays the only evaluation engine

- **Rationale:** ADR-040 selected regorus. Adding a second runtime would
  fragment the engine surface, complicate witness chain provenance, and
  duplicate test infrastructure. Compiling YAML → Rego preserves a
  single IR.
- **Alternatives:** Interpret YAML directly; ship a second engine.
- **Trade-offs:** Compilation adds a build step in the pack loader; the
  alternative undermines ADR-040.

### D-ACTAX-003: Tool-call interception is held as Phase D

- **Rationale:** Anvil intercepts at save-time (post-generation). Adding
  pre-execution interception is a meaningfully different threat model —
  fail-mode, latency budget, attack surface, integration point all
  change. That decision deserves its own ADR, not a sub-task here.
  Phases A–C deliver value at save-time without it.
- **Alternatives:** Include tool-call intercept in Phase A; reject it
  outright.
- **Trade-offs:** Holding it defers a potentially-important capability;
  rushing it risks shipping the wrong shape. Phasing it preserves the
  option without committing.

### D-ACTAX-004: Anvil's intercept tier vocabulary is preserved

- **Rationale:** `warn / fence / interrupt` is already shipped and
  documented. Adopting permit0's `Minimal / Low / Medium / High /
  Critical` would create two parallel tier systems.
- **Alternatives:** Rename to permit0's tiers; ship both.
- **Trade-offs:** Anvil's vocabulary loses some granularity at the high
  end; addressable by adding a single new mode later if needed.

## Notes

### Permit0 attribution

- Source: <https://github.com/permit0-ai/permit0> at the commit referenced
  in the taxonomy file header
- License: Apache 2.0
- Obligations: preserve `LICENSE`, `NOTICE`, mark modified files,
  attribute in `ACKNOWLEDGEMENTS.md`
- Trademark note: do not use the "Permit0" name in Anvil product
  surfaces; attribution only

### Suggested index.aps.md placement

Add under **Policy Governance** (after `compliance-policy-packs`), with
dependencies listed as `ADR-040, IORISK, AGOV, POLENG, CPOL (schema
coordination)`. Status `Proposed` until coordination with CPOL / IORISK
is resolved.

### What this module does *not* introduce

- No Rust daemon parallel to `anvil-intercept`
- No axum server, no MCP server adapters, no biscuit-auth tokens — Anvil
  already has equivalents (intercept daemon, witness chain)
- No cryptographic signing of the audit chain (separate work; ADR-037
  notes hash-chain ≠ authentication)
