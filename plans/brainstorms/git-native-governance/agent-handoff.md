# Agent Handoff — Planning and Implementation Guidance

## 1. Start here

Read these docs in order:

1. `context.md`
2. `architecture.md`
3. `solution.md`
4. `roadmap.md`

Then inspect the repo source anchors listed below. Do not assume the docs are perfectly aligned with current `main`; the repo is moving quickly.

---

## 2. Core thesis to preserve

> Anvil is the Git-native governance engine for AI-assisted software.
>
> Git is the durable trust substrate. Kindling observes. Ember proposes. Edda remembers. Anvil enforces and proves.

Do not turn this into:

- a generic memory product;
- a generic compliance dashboard;
- a source-code scanner with extra metadata;
- a cloud-only governance platform;
- a Git database experiment.

The first deliverable should be concrete and product-shaped:

```sh
anvil capsule create --range main..HEAD --out review.anvil-capsule
anvil capsule verify review.anvil-capsule
anvil capsule explain review.anvil-capsule
```

---

## 3. Repo files to inspect first

Inspect these files before planning:

```text
README.md
plans/specs/2026-05-07-anvil-multilayer-protection-architecture.md
plans/index.aps.md
packages/kindling-integration/README.md
docs/architecture/edda-stack.md
packages/edda-stack/README.md
packages/edda-stack/src/ember/README.md
packages/edda-stack/src/edda/README.md
docs/guides/edda-memory.md
packages/edda-stack/src/contracts/edda-memory.ts
plans/modules/graph-v2-foundation.aps.md
plans/modules/compliance-evidence-workspace.aps.md
plans/reviews/edda-ember-stack-review.md
```

Then inspect likely implementation crates:

```text
crates/anvil-cli/
crates/anvil-witness/
crates/anvil-l4/
crates/anvil-hook/
crates/anvil-config/
crates/anvil-policy/
crates/anvil-policy-engine/
crates/anvil-baseline/
crates/anvil-rules/
crates/anvil-kernel-types/
crates/anvil-observability/
packages/edda-stack/
packages/kindling-integration/
```

---

## 4. Non-negotiable architectural boundaries

## 4.1 State boundary

```text
.anvil/ = local runtime/cache/SQLite/logs/scratch
anvil/  = tracked durable governance/memory/evidence
```

If current docs or code disagree, plan migration rather than silently continuing the conflict.

## 4.2 Layer authority

```text
Kindling cannot judge.
Ember cannot decide.
Edda cannot speculate.
Anvil policy enforces.
Git proves durable evidence.
```

## 4.3 Edda is not enforcement

Edda memory may inform policy proposals and explanations. It must not automatically block changes.

## 4.4 Missing evidence is not a pass

Use `degraded` when evidence is missing or unverifiable. Do not claim protected.

## 4.5 Do not store secrets in Git evidence

Store redacted summaries and digests, not raw logs or raw environment dumps.

---

## 5. First planning target: GITGOV / Review Capsules

Produce a plan for:

```text
GITGOV-003 Capsule manifest schema
GITGOV-004 Capsule create command
GITGOV-005 Commit/range collector
GITGOV-006 Policy/baseline/rule digest collector
GITGOV-007 Witness collector
GITGOV-008 Diagnostics collector
GITGOV-009 Verification engine
GITGOV-010 Capsule explain UX
GITGOV-011 JSON output
GITGOV-012 Tamper tests
```

Do not plan advanced Git refs/notes for v0.

Use a file-first capsule:

```text
review.anvil-capsule/
├── manifest.json
├── commits.json
├── policy.json
├── baseline.json
├── rules.json
├── witness.ndjson
├── diagnostics.sarif
├── exceptions.json
├── edda-context.json
├── verification.json
└── README.md
```

---

## 6. Suggested implementation structure

Preferred if a new crate is warranted:

```text
crates/anvil-capsule/
  src/
    lib.rs
    manifest.rs
    collect.rs
    verify.rs
    explain.rs
    format.rs
    errors.rs

crates/anvil-cli/src/commands/capsule.rs
```

Alternative if the repo prefers fewer crates:

```text
crates/anvil-cli/src/commands/capsule/
  mod.rs
  create.rs
  verify.rs
  explain.rs
  manifest.rs
```

Choose based on current crate boundaries and dependency direction.

---

## 7. Key schemas to implement or draft

Start with:

```text
anvil.capsule.v1
anvil.capsule-verification.v1
anvil.policy-digest.v1
anvil.rules-digest.v1
anvil.exception.v1
anvil.edda-provenance.v1
```

Do not over-model future entities. Keep v0 stable and inspectable.

---

## 8. Verification rules

A capsule verification result must return one of:

```text
pass
warn
degraded
block
error
```

Mapping guidance:

```text
All required evidence verifies, no block findings      -> pass
Evidence verifies, warnings exist                      -> warn
Evidence missing/stale/partially unverifiable          -> degraded
Witness break / invalid exception / policy violation   -> block
Tool/internal failure                                  -> error
```

Important:

```text
degraded != pass
error != pass
missing evidence != clean evidence
```

---

## 9. Known issues to avoid inheriting

The Edda/Ember review found issues that matter for trust:

- Git `--author` format issue in Edda version tracker.
- Evolution chain can loop on corrupt data if cycle guard is wrong.
- Supersede operation needs status guard.
- Fallback generated fake UUIDs for provenance when source links were absent.
- CLI JSON error output can mix JSON and plain text.
- Storage-not-found exit codes were inconsistent.

When planning EDDA-SEAL, treat these as preconditions or include them in the work.

Most important rule:

> Never invent provenance.

If provenance is missing, record:

```json
{
  "provenance_status": "degraded",
  "reason": "source observation unavailable"
}
```

Do not fabricate observation IDs or session IDs.

---

## 10. Suggested ADRs to create

### ADR 1 — Git-native Governance Substrate

Decision:

```text
Anvil uses Git as the durable substrate for governed evidence, policy history, witness chains, exceptions, review capsules, release attestations, and sealed Edda memory/provenance.
```

### ADR 2 — Durable vs Local Anvil State

Decision:

```text
`.anvil/` is local runtime state. `anvil/` is tracked durable governance/memory/evidence state.
```

### ADR 3 — Review Capsule v0 Format

Decision:

```text
Capsule v0 is a file-first directory/archive with a manifest and digests. Refs/notes/bundles are deferred.
```

### ADR 4 — Edda Sealed Provenance

Decision:

```text
Edda promotions seal a minimal, redacted provenance summary into Git so traceability survives Kindling/Ember pruning.
```

### ADR 5 — Git-native Exceptions

Decision:

```text
Exceptions are tracked governance objects with scope, expiry, actor, reason, revocation, and capsule/witness inclusion.
```

---

## 11. Development planning prompt

Use this prompt for the next dev agent:

```text
You are planning the first implementation slice of Anvil's Git-native governance substrate. Read the docs in anvil-git-native-governance-docs/ and inspect the current repo files listed in agent-handoff.md. Produce an APS-style module plan for GITGOV: Review Capsules. Keep v0 file-first, offline-verifiable, and focused on commit range, policy/rules/baseline digest, witness collection, diagnostics summary, exception inclusion, manifest digests, and verify/explain commands. Do not introduce Git refs/notes, Graph v2 dependency, cloud services, or policy marketplace work in v0. Include exact files to touch, crate boundaries, tests, command UX, risk mitigations, and acceptance criteria.
```

---

## 12. Implementation checklist for capsule v0

- [ ] Decide crate/module location.
- [ ] Define `CapsuleManifest` schema.
- [ ] Define `CapsuleVerification` schema.
- [ ] Implement commit range resolution.
- [ ] Implement file digest helper.
- [ ] Collect policy digest.
- [ ] Collect rules digest.
- [ ] Collect baseline digest/cutoff.
- [ ] Collect witness lines.
- [ ] Collect diagnostics/SARIF or run validation.
- [ ] Collect exceptions, if present.
- [ ] Optionally collect Edda references only if already safe.
- [ ] Write capsule directory.
- [ ] Verify manifest hashes.
- [ ] Verify witness coverage/integrity.
- [ ] Verify exception scope/expiry.
- [ ] Produce verdict.
- [ ] Explain verdict.
- [ ] Add JSON output.
- [ ] Add tamper tests.
- [ ] Add missing-evidence/degraded tests.
- [ ] Add docs.

---

## 13. Implementation checklist for Edda path and sealed provenance

- [ ] Confirm current Edda storage implementation path.
- [ ] Draft migration plan from `.anvil/edda/` to `anvil/edda/`.
- [ ] Update docs/config defaults.
- [ ] Define sealed provenance schema.
- [ ] Add redaction policy metadata.
- [ ] Seal Ember proposal summary on promotion.
- [ ] Seal Kindling observation summaries/digests on promotion.
- [ ] Ensure `trace` falls back to sealed provenance.
- [ ] Do not require live Kindling/Ember records for historical trace.
- [ ] Fix known provenance correctness issues.

---

## 14. Implementation checklist for exceptions

- [ ] Define exception schema.
- [ ] Define storage path under `anvil/exceptions/`.
- [ ] Implement grant command.
- [ ] Implement revoke command.
- [ ] Implement list/show commands.
- [ ] Validate actor/reason/scope/expiry.
- [ ] Integrate with L3/L4 policy evaluation.
- [ ] Include used exceptions in witness envelope.
- [ ] Include exceptions in capsules.
- [ ] Verify expired exception blocks/degrades.
- [ ] Verify revoked exception does not apply.

---

## 15. Design guardrails

- Build from existing Anvil witness/policy/baseline concepts.
- Keep everything local-first.
- Do not require cloud sign-up.
- Make artefacts inspectable.
- Prefer deterministic canonical JSON for digests.
- Use redaction by default.
- Treat AI as assistive, not authoritative.
- Treat Git as durable proof, not hot query storage.
- Treat Graph v2 as projection/cache, not authority.
- Treat Edda as memory, not policy.
- Treat exceptions as dangerous and therefore first-class.

---

## 16. Definition of done for the first meaningful demo

A convincing demo should show:

1. A branch with one or more commits.
2. Anvil policy/rules/baseline active.
3. At least one witness generated.
4. Optional warning finding.
5. Optional valid exception.
6. `anvil capsule create --range main..HEAD` produces a capsule.
7. `anvil capsule explain` shows what changed and what governed it.
8. `anvil capsule verify` returns pass/warn.
9. Mutating a capsule file causes digest mismatch.
10. Removing witness evidence causes `degraded`, not `pass`.
11. Expiring or revoking an exception changes verification outcome.
12. Demo works without Anvil Cloud.

This is enough to validate the category thesis.

