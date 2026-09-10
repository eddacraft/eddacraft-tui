# Roadmap — From Idea to Implementation

## 1. Roadmap principle

Do not build the whole vision at once.

Build the smallest product slice that proves the thesis:

> A governed AI-assisted change can be packaged and verified locally using Git-backed evidence.

Recommended first wedge:

```text
GITGOV: Anvil Review Capsules
```

Then layer in:

```text
EDDA-SEAL: Sealed Edda provenance
EXCEPT: Git-native exceptions
POLICY-GIT: Git-native policy packs
POLICY-MEM: policy proposals from Edda memory
RELEASE-SEAL: release attestations
SUPPLIER: external trust bundles
```

---

## 2. Proposed modules

## 2.1 GITGOV — Git-Native Governance Substrate

Purpose:

> Establish Git as the durable substrate for Anvil governance evidence, starting with review capsules.

Work items:

| ID | Item | Outcome | Priority |
| --- | --- | --- | --- |
| GITGOV-001 | ADR: Git-native governance substrate | Architecture decision accepted. | Critical |
| GITGOV-002 | State boundary decision | `.anvil/` local, `anvil/` tracked. | Critical |
| GITGOV-003 | Capsule manifest schema | `anvil.capsule.v1` schema. | Critical |
| GITGOV-004 | Capsule create command | `anvil capsule create --range`. | Critical |
| GITGOV-005 | Commit/range collector | Commit list, tree hashes, changed paths. | Critical |
| GITGOV-006 | Policy/baseline/rule digest collector | Effective governance input digest. | Critical |
| GITGOV-007 | Witness collector | Relevant witness lines included. | Critical |
| GITGOV-008 | Diagnostics collector | SARIF/diagnostics summary included. | High |
| GITGOV-009 | Verification engine | Pass/warn/degraded/block/error verdicts. | Critical |
| GITGOV-010 | Capsule explain UX | Human-readable explanation. | High |
| GITGOV-011 | JSON output | CI-friendly output. | High |
| GITGOV-012 | Tamper tests | Digest mismatch and witness break tests. | Critical |

Validation:

```sh
cargo test -p anvil-capsule
cargo test -p anvil-cli capsule
anvil capsule create --range main..HEAD --out /tmp/review.anvil-capsule
anvil capsule verify /tmp/review.anvil-capsule
```

---

## 2.2 EDDA-SEAL — Sealed Edda Provenance

Purpose:

> Preserve the provenance behind Edda memories even if Kindling and Ember local stores decay or are pruned.

Work items:

| ID | Item | Outcome | Priority |
| --- | --- | --- | --- |
| EDDA-SEAL-001 | Edda storage path migration decision | Durable Edda moves to `anvil/edda/`. | Critical |
| EDDA-SEAL-002 | Sealed provenance schema | `anvil.edda-provenance.v1`. | Critical |
| EDDA-SEAL-003 | Promotion-time sealing | `anvil edda promote` writes provenance bundle. | Critical |
| EDDA-SEAL-004 | Kindling summary/digest extractor | Summaries and digests, no raw logs. | High |
| EDDA-SEAL-005 | Ember proposal seal | Proposal summary, confidence, rationale preserved. | High |
| EDDA-SEAL-006 | Trace fallback | `anvil edda trace` works after local pruning. | High |
| EDDA-SEAL-007 | Capsule integration | Capsules can include relevant Edda context. | Medium |
| EDDA-SEAL-008 | Redaction policy metadata | Sealed provenance names redaction policy. | Critical |

Validation:

```sh
anvil edda promote <proposal-id> --reason "..." --by "..." --confidence high --type warning
anvil edda trace <memory-id>
anvil capsule create --range main..HEAD --include-edda --out /tmp/review.anvil-capsule
```

Implementation caution:

Before making Edda load-bearing, fix known correctness issues from the Edda/Ember review:

- invalid Git `--author` handling;
- evolution-chain cycle guard;
- supersede status guard;
- fake UUID fallback for provenance;
- inconsistent CLI JSON error output;
- storage-not-found exit code inconsistencies.

---

## 2.3 EXCEPT — Git-Native Exceptions

Purpose:

> Make intentional deviations scoped, expiring, attributed, reviewable, and included in governance evidence.

Work items:

| ID | Item | Outcome | Priority |
| --- | --- | --- | --- |
| EXCEPT-001 | Exception schema | `anvil.exception.v1`. | Critical |
| EXCEPT-002 | Grant command | `anvil exception grant`. | Critical |
| EXCEPT-003 | Revoke command | `anvil exception revoke`. | Critical |
| EXCEPT-004 | List/show commands | Inspect active/revoked exceptions. | High |
| EXCEPT-005 | Scope matcher | Paths/globs/finding ID matching. | Critical |
| EXCEPT-006 | Expiry validation | Expired exceptions do not apply. | Critical |
| EXCEPT-007 | L3/L4 integration | Gates apply valid exceptions only. | Critical |
| EXCEPT-008 | Witness inclusion | Exception use recorded in witness envelope. | High |
| EXCEPT-009 | Capsule inclusion | Capsule includes used exceptions. | Critical |
| EXCEPT-010 | Revocation audit trail | Revoked exceptions preserved. | High |

Validation:

```sh
anvil exception grant ARCH-BOUNDARY-001 --scope src/billing/** --reason "Migration shim" --owner sarah@example.com --expires 2026-07-01
anvil exception list
anvil capsule create --range main..HEAD --out /tmp/review.anvil-capsule
anvil capsule verify /tmp/review.anvil-capsule
```

---

## 2.4 POLICY-GIT — Git-Native Policy Packs

Purpose:

> Distribute, pin, and verify policy packs through Git so Anvil stays local-first and air-gap friendly.

Work items:

| ID | Item | Outcome | Priority |
| --- | --- | --- | --- |
| POLICY-GIT-001 | Policy pack manifest | `anvil.policy-pack.v1`. | High |
| POLICY-GIT-002 | Add command | `anvil policy add <git-url> --tag`. | High |
| POLICY-GIT-003 | Pin command | `anvil policy pin <pack>@<version>`. | High |
| POLICY-GIT-004 | Verify command | Pack digest and compatibility verification. | High |
| POLICY-GIT-005 | Rule-set digest integration | Witness/capsule rules SHA includes packs. | Critical |
| POLICY-GIT-006 | Offline install path | Works from local clone/bundle. | High |
| POLICY-GIT-007 | Pack tests | Policy pack test suite support. | Medium |
| POLICY-GIT-008 | Pack changelog impact | Diff policy versions. | Medium |

Validation:

```sh
anvil policy add ./policy-packs/architecture --tag v1.0.0
anvil policy pin architecture@v1.0.0
anvil policy verify
anvil capsule create --range main..HEAD --out /tmp/review.anvil-capsule
```

---

## 2.5 POLICY-MEM — Policy Proposals from Edda Memory

Purpose:

> Turn repeated organisational learning into deliberate policy proposals without allowing Edda memory to enforce automatically.

Work items:

| ID | Item | Outcome | Priority |
| --- | --- | --- | --- |
| POLICY-MEM-001 | Proposal schema | `anvil.policy-proposal.v1`. | Medium |
| POLICY-MEM-002 | Propose-from-memory command | Draft policy proposal from memory. | Medium |
| POLICY-MEM-003 | Impact evaluator | Evaluate proposed rule against history. | High |
| POLICY-MEM-004 | Human acceptance flow | Accepted proposal updates policy pack/config. | High |
| POLICY-MEM-005 | Memory link in policy | Policy records source Edda memory. | Medium |
| POLICY-MEM-006 | Capsule link | Capsules can explain policy origin via Edda. | Medium |

Validation:

```sh
anvil policy propose-from-memory mem_abc123 --pack architecture-boundaries --mode warn
anvil policy impact <proposal-id> --against main~500..main
```

Boundary:

```text
Edda memory cannot block code. Only accepted policy can block code.
```

---

## 2.6 RELEASE-SEAL — Release Attestations

Purpose:

> Seal release-level governance evidence in a durable, replayable, portable record.

Work items:

| ID | Item | Outcome | Priority |
| --- | --- | --- | --- |
| RELEASE-SEAL-001 | Release attestation schema | `anvil.release-attestation.v1`. | Medium |
| RELEASE-SEAL-002 | Seal command | `anvil release seal <version>`. | Medium |
| RELEASE-SEAL-003 | Verify command | `anvil release verify <version>`. | Medium |
| RELEASE-SEAL-004 | Dependency/build inputs | Include lockfiles/build metadata. | Medium |
| RELEASE-SEAL-005 | Witness-chain verification | Release seal references verified witness range. | High |
| RELEASE-SEAL-006 | Exception summary | Release includes active/used exceptions. | High |
| RELEASE-SEAL-007 | Test/eval evidence | Include test/eval artefact digests. | Medium |

---

## 2.7 SUPPLIER — External Trust Bundles

Purpose:

> Allow vendors/suppliers to provide Anvil-verifiable governance proof without joining the buyer’s SaaS tenant.

Work items:

| ID | Item | Outcome | Priority |
| --- | --- | --- | --- |
| SUPPLIER-001 | Supplier bundle profile | Minimal externally shareable capsule profile. | Low |
| SUPPLIER-002 | Verify against buyer policy | `anvil supplier verify --policy`. | Low |
| SUPPLIER-003 | Redaction/export profile | Safe sharing of evidence without source leakage. | Medium |
| SUPPLIER-004 | Trust report | Human-readable supplier verification report. | Low |

---

## 3. Recommended sequence

## Phase 0 — Decision and scope lock

Timebox: first planning slice.

Deliverables:

- ADR: Git-native governance substrate.
- ADR or design note: `anvil/` vs `.anvil/` boundary.
- Capsule v0 scope accepted.
- Edda path migration direction accepted.

Do not write large implementation before these are settled.

## Phase 1 — Capsule skeleton

Deliverables:

- `anvil capsule create --range` writes a capsule directory.
- Manifest schema exists.
- Commit list and digests are included.
- Policy/baseline/rules digest included.
- Basic `verify` checks manifest hashes.

Value:

- Proves packaging concept.

## Phase 2 — Witness and diagnostics

Deliverables:

- Capsule includes witness evidence.
- Capsule includes diagnostics/SARIF.
- Verification can distinguish pass/warn/degraded/block.
- Tamper tests pass.

Value:

- Proves governance evidence, not just packaging.

## Phase 3 — Exceptions

Deliverables:

- Exception schema and grant/revoke/list commands.
- Exception application in L3/L4.
- Exception inclusion in capsule.
- Invalid/expired exception blocks or degrades correctly.

Value:

- Closes the most dangerous governance loophole.

## Phase 4 — Edda sealed provenance

Deliverables:

- Edda durable path moved to `anvil/edda/`.
- Promotion seals provenance summary.
- Trace works after local pruning.
- Capsules can include relevant Edda context.

Value:

- Connects organisational learning to governance proof.

## Phase 5 — Policy packs and impact

Deliverables:

- Git-native policy pack install/pin/verify.
- Policy impact over history.
- Rule-set digest includes pack identity.

Value:

- Makes governance rollout first-class and local-first.

## Phase 6 — Memory to policy loop

Deliverables:

- Policy proposal from Edda memory.
- Human acceptance flow.
- Capsule can explain policy origin.

Value:

- Creates the learning loop: experience → memory → policy → enforcement → evidence.

## Phase 7 — Release/supplier bundles

Deliverables:

- Release seal.
- Supplier verification profile.
- Portable external evidence workflows.

Value:

- Enterprise/audit/supply-chain expansion.

---

## 4. Candidate ADR draft

# ADR: Git-Native Governance Substrate

## Status

Proposed

## Context

Anvil already enforces AI-assisted software governance across save-time, pre-commit, pre-push/CI, and audit layers. It already writes witness evidence and uses tracked `anvil/` metadata for project identity, policy, baseline, rules, and witness chain artefacts.

The next product claim requires durable, portable, replayable evidence that can be verified without trusting a hosted service.

Git is already present in every target workflow and provides content-addressed storage, tamper-evident history, distributed replication, review workflows, refs, notes, tags, and bundles.

## Decision

Anvil will use Git as the durable governance substrate for:

- witness evidence;
- policy and rule history;
- baselines;
- exception grants/revocations;
- review capsules;
- release attestations;
- sealed Edda memory/provenance;
- future compliance evidence exports.

Local runtime state remains under `.anvil/`. Durable project governance state lives under `anvil/` and/or dedicated Git refs/notes.

## Consequences

Positive:

- local-first and air-gapped by default;
- portable review/audit evidence;
- no cloud dependency for verification;
- developer-native review model;
- strong enterprise trust posture.

Negative/trade-offs:

- Git storage can become noisy if not disciplined;
- refs/notes add complexity if introduced too early;
- redaction must be strict;
- evidence schemas must be stable and migration-aware;
- large artefacts need careful handling.

## Non-goals

- Git as a hot event database;
- raw telemetry storage in Git;
- generic GRC replacement;
- cloud-only verification.

---

## 5. Planning agent prompts

Use these prompts to create executable plans.

### Prompt 1 — Capsule MVP plan

```text
Read context.md, architecture.md, and solution.md. Inspect the current Anvil repo for existing witness, policy, baseline, diagnostic, and CLI command surfaces. Produce an APS-style implementation plan for GITGOV-003 through GITGOV-012. Keep v0 file-first; do not introduce Git refs/notes yet. Include exact files, crates, tests, validation commands, risks, and acceptance criteria.
```

### Prompt 2 — Path model ADR

```text
Read the Edda docs and multi-layer protection spec. Produce an ADR resolving the durable/local state boundary: `.anvil/` for runtime/cache/SQLite/logs and `anvil/` for tracked governance/memory/evidence. Include migration implications for Edda and compatibility handling.
```

### Prompt 3 — Edda sealed provenance plan

```text
Read the Edda Stack docs, Edda memory guide, Ember README, and solution.md. Produce an implementation plan for EDDA-SEAL. Focus on promotion-time sealed provenance, trace fallback after Ember/Kindling pruning, redaction, and migration from `.anvil/edda/` to `anvil/edda/`.
```

### Prompt 4 — Git-native exceptions plan

```text
Read solution.md and current Anvil exception/ignore/baseline handling. Produce an implementation plan for EXCEPT. Design the exception schema, grant/revoke/list CLI, L3/L4 integration, witness envelope inclusion, and capsule verification.
```

### Prompt 5 — Policy pack impact plan

```text
Read the policy/rule crates and this roadmap. Produce a design for Git-native policy pack install/pin/verify and policy impact over history. Keep it after capsule v0 and exceptions. Avoid introducing a marketplace.
```

