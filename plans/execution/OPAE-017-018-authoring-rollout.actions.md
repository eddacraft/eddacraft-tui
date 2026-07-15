# Policy Authoring Skill and Scenario Rollout Implementation Plan

**Goal:** Ship the `authoring-anvil-policy` skill and prove it through realistic, executable policy-authoring journeys.
**Architecture:** The private `eddacraft-skills` catalogue remains the canonical content source. OPAE owns the policy skill and scenarios; SKPKG extends the managed bundle without duplicating ADR-106's client registry. Held-out, semantically diverse scenarios exercise authoring and repair, while both built-binary integration and downloaded-release smoke tests prove delivery.
**Tech Stack:** Agent Skills Markdown, Rust CLI integration tests, Rego/regorus, YAML/JSON fixtures, managed skill installer

---

**APS:** OPAE-017, OPAE-018
**Dependencies:** OPAE-014, OPAE-016, OPAE-019, OPAE-020, SKPKG-009.

## File map

| File | Responsibility |
| --- | --- |
| `../eddacraft-skills/skills/eddacraft/authoring-anvil-policy/SKILL.md` | Canonical small policy-authoring router/workflow. |
| `../eddacraft-skills/skills/eddacraft/authoring-anvil-policy/references/routing.md` | Fallback topic and capability routing for harnesses without CLI/MCP access. |
| `../eddacraft-skills/skills/eddacraft/authoring-anvil-policy/skill.meta.json` | Catalogue targets, provenance, and customer-facing classification. |
| `crates/anvil-cli/assets/skills/authoring-anvil-policy/` | Pinned vendored skill snapshot and provenance. |
| `crates/anvil-cli/src/commands/skill_catalogue.rs` | Typed registry of embedded managed skills and files. |
| `crates/anvil-cli/src/commands/skill.rs` | Multi-skill selection while preserving existing install behaviour. |
| `crates/anvil-cli/tests/skill_install.rs` | Named selection, interactive/non-interactive compatibility, provenance, drift, and verify tests. |
| `policies/scenarios/payments-companion-tests/` | Payment-change/test companion conformance pack and repositories. |
| `policies/scenarios/clinical-rules-verification/` | Clinical-rule/fixture companion conformance pack. |
| `policies/scenarios/deployment-rollback-runbook/` | Infrastructure/runbook companion conformance pack. |
| `crates/anvil-cli/tests/policy_authoring_journey.rs` | Released-binary end-to-end authoring journeys. |
| `docs/agent-guidance/policy-authoring/topics/scenarios.md` | Generated-route narrative for scenario selection and proof limits. |
| `docs/public/anvil/integrations/skills.md` | Minimal install/discovery mention only; no agent-reference links. |
| `docs/public/anvil/tutorials/policies.md` | Correct human quick path and command truth. |

## Task 1: Land the canonical catalogue skill

**Files:**

- Create in `eddacraft-skills`: `skills/eddacraft/authoring-anvil-policy/SKILL.md`
- Create in `eddacraft-skills`: `skills/eddacraft/authoring-anvil-policy/references/routing.md`
- Create in `eddacraft-skills`: `skills/eddacraft/authoring-anvil-policy/skill.meta.json`

- [ ] Start from the requirements delivered to the agent at tmux
      `skills:4.1`; reconcile any overlap with `using-anvil` explicitly.
- [ ] Keep the skill body within 1,200 estimated tokens and make comprehensive
      facts retrievable topics rather than copied prose.
- [ ] Encode the workflow: choose surface, define invariant, choose target,
      retrieve availability, author, lint, validate exact pack, eval, gate,
      optional pre-write proof, exceptions, and proof limits.
- [ ] Add version/capability checks so the skill never invents `policy lint` or
      `guidance` commands when the running binary predates them.
- [ ] Do not invent a policy scaffold command. Route to the shipped starter
      pack/show surfaces and the v2 manifest topic until a real scaffold command
      exists.
- [ ] Link only official Rego language/style/testing/debugging sources and local
      Anvil guidance topic IDs; do not link the private catalogue or public
      agent-reference pages.
- [ ] Run the catalogue repository validation and review process.
- [ ] Commit and merge in `eddacraft-skills`; record the immutable source commit.

## Task 2: SKPKG-owned multi-skill bundle extension

**Files:**

- Create: `crates/anvil-cli/src/commands/skill_catalogue.rs`
- Modify: `crates/anvil-cli/src/commands/skill.rs`
- Modify: `crates/anvil-cli/tests/skill_install.rs`

- [ ] Write failing tests for `anvil skill install authoring-anvil-policy`,
      `--verify`, `--dry-run`, global/project roots, two skills sharing a client
      root, unmanaged drift, user-modified drift, and stable JSON reports.
- [ ] Pin backward compatibility: non-interactive `anvil skill install` with no
      skill name continues to select `anvil-developer-functions`; interactive
      no-name mode may offer a multi-select without changing existing managed
      files unexpectedly.
- [ ] Define a typed embedded registry:

```rust
pub struct BundledSkill {
    pub name: &'static str,
    pub source_commit: &'static str,
    pub files: &'static [BundledSkillFile],
}

pub struct BundledSkillFile {
    pub relative_path: &'static str,
    pub contents: &'static str,
}

pub fn bundled_skills() -> &'static [BundledSkill];
```

- [ ] Route digest, provenance, destination, verification, and managed-update
      logic through the selected registry entry; remove the single-skill
      constants without weakening symlink or unmanaged-file refusal.
- [ ] Reuse ADR-106's agent-client registry for detection, destinations, and
      capabilities; the new registry contains bundle content/provenance only.
- [ ] Run `cargo test -p eddacraft-anvil --test skill_install` and Clippy.
- [ ] Commit: `refactor(skill): register managed bundles`

## Task 3: Vendor and verify `authoring-anvil-policy`

**Files:**

- Create: `crates/anvil-cli/assets/skills/authoring-anvil-policy/SKILL.md`
- Create: `crates/anvil-cli/assets/skills/authoring-anvil-policy/references/routing.md`
- Create: `crates/anvil-cli/assets/skills/authoring-anvil-policy/bundle-provenance.json`
- Modify: `crates/anvil-cli/src/commands/skill_catalogue.rs`
- Modify: `crates/anvil-cli/tests/skill_install.rs`

- [ ] Vendor only from the reviewed canonical commit; do not fetch the private
      catalogue during build or install.
- [ ] Record catalogue commit, Anvil version, bundle digest, and per-file hashes.
- [ ] Add a test that every skill topic and lint code resolves against the
      embedded guidance bundle.
- [ ] Add a context-budget test for installed skill body and fallback reference.
- [ ] Run install/verify against at least Codex and Claude Code project roots
      plus one shared `.agents/skills` client root supported by ADR-106.
- [ ] Commit: `feat(skill): ship policy authoring guidance`

## Task 4: Add three executable industry scenarios

**Files:**

- Create: `policies/scenarios/payments-companion-tests/**`
- Create: `policies/scenarios/clinical-rules-verification/**`
- Create: `policies/scenarios/deployment-rollback-runbook/**`
- Modify: `docs/agent-guidance/policy-authoring/topics/scenarios.md`

- [ ] Keep the implementation prompts held out from the agent under test. Record
      the supplied plain-language invariant, retrieved topics, interventions,
      repair iterations, and final deterministic result.
- [ ] For each scenario, first write failing conformance assertions for v2
      manifest admission, required positive/negative/boundary/malformed inputs,
      expected lint report, warning/violation outcome, and remediation text.
- [ ] Use only `diff.changed_files`, the currently guaranteed gate input, but
      prove distinct behaviours: payments requires a companion test category;
      clinical rules require a same-stem verification fixture; production
      deployment changes require both rollback and alerting evidence. Do not
      use plans, decisions, new edges, baseline, contents, or configuration
      until target wiring exists.
- [ ] Include passing and failing temporary repositories rather than prose-only
      input examples.
- [ ] Ensure all names and descriptions say industry scenario and make no
      regulatory-compliance claim.
- [ ] Run lint, validate, explicit eval, and policy-only gate for every fixture.
- [ ] Commit: `test(policy): add industry authoring scenarios`

## Task 5: Prove the built-binary authoring journey

**Files:**

- Create: `crates/anvil-cli/tests/policy_authoring_journey.rs`
- Add fixture helpers only where they are shared by multiple journey cases

- [ ] Write a failing subprocess test that uses an isolated home/runtime and a
      temporary Git repository.
- [ ] Exercise this exact sequence with the built binary:

```text
skill install authoring-anvil-policy --client codex --scope project
guidance list policy-authoring --json
guidance show policy-authoring --topic policy-authoring.input.gate
policy lint <scenario-pack> --json
policy validate <scenario-pack> --json
policy eval <member.rego> --input <case.json> --json
gate --only-checks policy
guidance materialise ...
guidance release <lease>
```

- [ ] Assert passing and failing gate cases, warning exit semantics, error exit
      semantics, remediation/exception text, no workspace guidance file, and
      released lease cleanup.
- [ ] Repeat with a pre-v2 legacy pack and assert migration warning without
      breaking validation.
- [ ] Keep the journey hermetic: no private repo, public docs, Go OPA, Regal,
      daemon, or network dependency.
- [ ] Commit: `test(policy): prove agent authoring journey`

## Task 5b: Smoke-test a downloaded release artefact

- [ ] In release CI or the beta release rehearsal, download the packaged Anvil
      artefact for each supported platform rather than invoking Cargo's freshly
      built test binary.
- [ ] Verify its checksum/signature using the release contract, then run the
      hermetic authoring sequence with explicit `--client` against the embedded
      skill and guidance bundle.
- [ ] Persist binary version/digest, platform, client, skill digest, guidance
      digest, scenario outcomes, and cleanup result in
      `docs/testing/releases/policy-authoring-beta-<version>.md`.
- [ ] Stop shipment if the packaged artefact differs behaviourally from the
      built-binary journey.

## Task 6: Run cross-client beta rollout evidence

**Files:**

- Modify: `docs/agent-guidance/policy-authoring/topics/troubleshooting.md` as
  evidence reveals genuine gaps
- Create/modify: `docs/testing/releases/policy-authoring-beta-<version>.md`
- Modify: OPAE module only after verification

- [ ] Use Claude Code, Codex, and OpenCode as the primary client matrix. Exercise
      every other first-wave install target as an explicitly labelled secondary
      client; distinguish installation support from automatic skill discovery.
- [ ] Record for each run: client, Anvil version, skill digest, topics retrieved,
      estimated tokens, lint diagnostics and repairs, commands executed, final
      verdict, materialisation state, and elapsed command bands.
- [ ] Classify failures as skill routing, guidance content, deterministic lint,
      target capability, client discovery, or environment; fix the owning
      surface rather than adding compensating prose to the skill.
- [ ] Require three consecutive clean runs per scenario on the primary clients,
      zero false-negative gate outcomes, zero destructive install/cleanup
      events, no unresolved client-contract divergence, and all context budgets
      met before calling the pilot proven.
- [ ] Do not promote advisory lint rules or expand to other documentation
      domains during this task.
- [ ] Commit: `test(policy): record authoring beta evidence`

## Task 7: Reconcile minimal public docs and release gates

**Files:**

- Modify: `docs/public/anvil/integrations/skills.md`
- Modify: `docs/public/anvil/tutorials/policies.md`
- Modify: `plans/modules/opa-enhancements.aps.md`
- Modify: release record/changelog only in the release PR that ships the work

- [ ] Keep public changes minimal: correct commands, name the installable skill,
      explain the deterministic boundary, and do not mirror or link the
      comprehensive agent reference.
- [ ] Fix the current tutorial to validate an exact pack directory and clarify
      that `policy validate`, not discovery-only `policy test`, executes pack
      tests.
- [ ] Run:

```sh
cargo test -p eddacraft-anvil --test skill_install
cargo test -p eddacraft-anvil --test policy_authoring_journey
pnpm guidance:check
cargo fmt --all -- --check
cargo clippy -p eddacraft-anvil --all-targets -- -D warnings
pnpm docs:check
pnpm aps:active-lint
pnpm aps:index:check
```

- [ ] Obtain Council and independent verification, then reconcile OPAE-017/018
      only from fresh evidence.
- [ ] Commit: `docs(policy): publish authoring entry points`

## Rollout stop conditions

Stop and return to design if any of these occur:

- the skill must preload comprehensive reference to trigger reliably;
- MCP requires multiple always-advertised verbose schemas;
- a scenario needs inputs the declared target does not populate;
- lint cannot distinguish an error rule from a high-noise heuristic;
- generated guidance cannot be reproduced offline;
- installer generalisation weakens managed-drift or symlink refusal; or
- a downloaded release artefact cannot complete the same journey as the built
  binary;
- any primary client breaches the aggregate MCP discovery budget or eagerly
  injects comprehensive guidance; or
- the public docs build must ingest the agent bundle.

## Expected handoff

- Customers can install the dedicated skill without accessing the private
  catalogue.
- Three realistic scenarios prove the deterministic authoring path.
- Token/context and cleanup evidence is recorded before generalisation.
- Any migration beyond policy authoring returns to APS planning as a separate
  decision.
