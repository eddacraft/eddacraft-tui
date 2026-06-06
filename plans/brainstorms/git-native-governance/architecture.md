# Architecture — End-State Git-Native Governance for Anvil

## 1. Architecture summary

The end-state solution is a local-first, Git-native governance architecture for AI-assisted software development.

At the centre:

```text
Anvil governs change.
Git proves governance.
Kindling observes work.
Ember proposes meaning.
Edda preserves institutional memory.
Graph v2 accelerates deterministic queries.
```

The architectural goal is not to replace Anvil’s current enforcement layers. It is to make the outputs of those layers durable, replayable, portable, and tamper-evident using Git.

---

## 2. System boundaries

### 2.1 Canonical principle

```text
Git is canonical for durable governance evidence and institutional memory.
SQLite/local files are canonical only for live operational working state.
Graph/cache stores are projections, not authority.
```

### 2.2 State boundary

```text
anvil/   = tracked durable state that travels with the repository
.anvil/  = local runtime state that should be gitignored
```

### 2.3 Proposed repository layout

```text
anvil/
├── project-id
├── policy.yml
├── baseline.json
├── rules/
│   └── *.rego
├── witness/
│   ├── manifest/
│   │   └── chain.ndjson
│   ├── active.ndjson
│   └── archive/
├── evidence/
│   ├── controls/
│   ├── capsules/
│   └── exports/
├── exceptions/
│   ├── active/
│   └── revoked/
├── edda/
│   ├── memories/
│   │   ├── decision/
│   │   ├── pattern/
│   │   ├── constraint/
│   │   ├── warning/
│   │   ├── doctrine/
│   │   └── lesson/
│   ├── provenance/
│   └── index.yaml
└── releases/
```

```text
.anvil/
├── kindling.db
├── ember.db
├── cache/
│   ├── graph/
│   └── policy/
├── runtime/
│   └── intercept.info.json
├── logs/
└── scratch/
```

### 2.4 Optional Git namespaces

For high-volume or non-working-tree evidence, later versions may use Git refs and notes:

```text
refs/anvil/witness/*
refs/anvil/sessions/*
refs/anvil/evidence/*
refs/anvil/exceptions/*
refs/anvil/policies/*
refs/anvil/graph/*
refs/anvil/releases/*
refs/notes/anvil-l3
refs/notes/anvil-l4
refs/notes/anvil-evidence
refs/notes/anvil-edda
refs/notes/anvil-decisions
```

Do not start with all of these. The v0 architecture can use files under `anvil/` first and migrate selected high-value artefacts to refs/notes later.

---

## 3. Component model

```text
┌───────────────────────────────────────────────────────────────────┐
│                         Developer Workflow                        │
│  editor / AI agent / CLI / hooks / PR / CI / release               │
└─────────────────────────────┬─────────────────────────────────────┘
                              │
                              ▼
┌───────────────────────────────────────────────────────────────────┐
│                              Anvil                                │
│                                                                   │
│  L0 MCP       L1 driver       L2 daemon       L3 hook       L4 CI  │
│  best-effort  mid-edit        save-time       pre-commit    push   │
│                                                                   │
│  - evaluates policy                                               │
│  - produces diagnostics                                            │
│  - writes witnesses                                                │
│  - enforces exceptions                                             │
│  - seals capsules/evidence                                         │
└───────────────┬──────────────────────────┬────────────────────────┘
                │                          │
                ▼                          ▼
┌──────────────────────────┐    ┌───────────────────────────────────┐
│        Edda Stack         │    │            Graph v2/cache          │
│                          │    │                                   │
│ Kindling -> Ember -> Edda │    │ fast deterministic projections     │
│ facts       proposals mem │    │ rebuildable, not canonical         │
└───────────────┬──────────┘    └───────────────────────────────────┘
                │
                ▼
┌───────────────────────────────────────────────────────────────────┐
│                       Git Trust Substrate                         │
│                                                                   │
│ anvil/witness/      anvil/evidence/      anvil/exceptions/         │
│ anvil/policy.yml    anvil/edda/          refs/notes/anvil-*        │
│                                                                   │
│ durable, local-first, tamper-evident, replayable, portable         │
└───────────────────────────────────────────────────────────────────┘
```

---

## 4. Responsibilities by component

### 4.1 Anvil core

Anvil owns deterministic governance.

Responsibilities:

- evaluate policy and rules;
- emit diagnostics;
- block or warn at the correct layer;
- write L3/L4 witnesses;
- manage baselines;
- enforce exceptions;
- create review capsules;
- verify capsules;
- seal release attestations;
- expose CLI/TUI/CI surfaces.

Non-responsibilities:

- storing raw long-term event streams;
- becoming a generic GRC dashboard;
- relying on cloud for local enforcement;
- treating memory proposals as enforceable without promotion to policy.

### 4.2 Git trust substrate

Git owns durable proof.

Responsibilities:

- store tracked governance metadata;
- preserve policy and rule history;
- preserve witness-chain artefacts;
- preserve sealed evidence;
- preserve exception grants/revocations;
- preserve Edda institutional memory;
- support offline verification;
- support review capsules and release attestations.

Non-responsibilities:

- hot-path queries;
- high-frequency telemetry;
- raw secret-bearing dumps;
- mutable operational queues.

### 4.3 Kindling

Kindling owns operational observation.

Responsibilities:

- capture session/activity facts;
- record actions, gates, constraints, human inputs, errors;
- provide bounded read-only queries;
- remain local and facts-only;
- provide source material for Ember and sealed provenance.

Storage:

```text
.anvil/kindling.db
```

Non-responsibilities:

- deciding what matters;
- creating durable institutional memory;
- being a permanent audit warehouse;
- being the source of policy truth.

### 4.4 Ember

Ember owns candidate meaning.

Responsibilities:

- aggregate Kindling observations;
- detect repetition, escalation, resolution, convergence, and surprise;
- produce candidate proposals;
- score proposals heuristically;
- decay proposals via TTL;
- allow humans to promote or dismiss.

Storage:

```text
.anvil/ember.db
```

Non-responsibilities:

- creating Edda memory directly;
- enforcing policy;
- making authoritative claims;
- retaining permanent proof.

### 4.5 Edda

Edda owns institutional memory.

Responsibilities:

- store human-promoted decisions, patterns, constraints, warnings, doctrines, and lessons;
- preserve provenance;
- maintain evolution chains;
- support supersession and retirement;
- provide memory context to Anvil and agents;
- optionally seed policy proposals.

Storage:

```text
anvil/edda/
```

Non-responsibilities:

- automatic enforcement;
- raw logging;
- speculative memory;
- AI-authored truths;
- unreviewed promotion.

### 4.6 Graph v2

Graph v2 owns deterministic projection and fast reads.

Responsibilities:

- model symbols, imports, calls, references, exports;
- model dependency/impact graph;
- model trust/policy graph;
- model control/session/provenance graph;
- provide hot-path indexes;
- support deterministic diff/replay queries.

Storage:

```text
.anvil/cache/graph/
```

Later optional durable snapshots:

```text
refs/anvil/graph/snapshots/<commit-sha>
refs/anvil/graph/deltas/<from>..<to>
```

Non-responsibilities:

- becoming canonical authority;
- replacing source/policy/evidence files;
- generic graph database product work.

### 4.7 Optional Anvil Cloud

Cloud is an amplifier, not a foundation.

Responsibilities, when present:

- fleet-level visibility;
- organisation policy distribution;
- GitHub App enforcement;
- hosted review UX;
- team analytics;
- supplier portal.

Non-responsibilities:

- being required for local enforcement;
- being the only place evidence exists;
- weakening air-gapped operation.

---

## 5. Data ownership matrix

| Data | Canonical owner | Durable path/ref | Local projection/cache |
| --- | --- | --- | --- |
| Policy | Git/Anvil | `anvil/policy.yml`, policy pack refs | `.anvil/cache/policy/` |
| Rules | Git/Anvil | `anvil/rules/`, policy pack refs | compiled rule cache |
| Baseline | Git/Anvil | `anvil/baseline.json` | policy cache |
| Witnesses | Git/Anvil | `anvil/witness/`, later refs/notes | chain-head cache |
| Diagnostics | Anvil | capsule/evidence artefact, SARIF | transient output/cache |
| Exceptions | Git/Anvil | `anvil/exceptions/` | evaluation cache |
| Kindling observations | Kindling | sealed summaries only | `.anvil/kindling.db` |
| Ember proposals | Ember | sealed promotion summaries only | `.anvil/ember.db` |
| Edda memories | Edda/Git | `anvil/edda/` | Edda index/cache |
| Graph state | Graph v2 | optional snapshot refs | `.anvil/cache/graph/` |
| Capsules | Anvil/Git | `anvil/evidence/capsules/` or external file | none required |
| Release attestations | Anvil/Git | `anvil/releases/`, tags/refs | none required |
| Runtime daemon info | Anvil daemon | none | `.anvil/runtime/` |

---

## 6. Core flows

## 6.1 Change governance flow

```text
AI agent / human edits file
        │
        ▼
L0 pre-write check, if available
        │
        ▼
L1 editor/mid-edit feedback, if available
        │
        ▼
L2 save-time daemon validation
        │
        ▼
L3 pre-commit validation
        │
        ├── pass/warn/block diagnostics
        ├── witness line written
        └── Kindling gate/action observations emitted
        │
        ▼
Git commit
        │
        ▼
L4 pre-push / CI validation
        │
        ├── validates witness chain
        ├── validates missing/no-witness commits if policy permits
        ├── writes L4 evidence/notes if required
        └── blocks/warns/allows
        │
        ▼
PR / merge / audit
```

Durable outputs:

```text
anvil/witness/*
anvil/evidence/*
refs/notes/anvil-l4  # later
```

Live outputs:

```text
.anvil/kindling.db
.anvil/cache/*
```

---

## 6.2 Review Capsule flow

```text
Developer branch ready for review
        │
        ▼
anvil capsule create --range main..HEAD
        │
        ├── collect commits/tree hashes
        ├── collect policy/rules/baseline digests
        ├── collect witness evidence
        ├── collect diagnostics summary
        ├── collect exception records
        ├── collect sealed Edda context, if enabled
        ├── write manifest
        └── write verification summary
        │
        ▼
review.anvil-capsule
        │
        ▼
Reviewer / CI / auditor runs:
        │
        ▼
anvil capsule verify review.anvil-capsule
        │
        ├── verify manifest hashes
        ├── verify witness chain
        ├── verify policy/rules digest
        ├── verify baseline anchor
        ├── verify exceptions scope/expiry
        └── return pass/warn/block/degraded
```

This is the recommended first end-to-end product slice.

---

## 6.3 Kindling → Ember → Edda flow

```text
Anvil emits observations
        │
        ▼
Kindling stores facts locally
        │
        ▼
Session completes or batch event fires
        │
        ▼
Ember aggregates observations
        │
        ├── repetition
        ├── escalation
        ├── resolution
        ├── convergence
        └── surprise
        │
        ▼
Candidate proposals created
        │
        ├── active
        ├── promoted
        ├── dismissed
        └── expired
        │
        ▼
Human reviews proposal
        │
        ▼
anvil edda promote <proposal-id> --seal-provenance
        │
        ├── create Edda memory
        ├── human attribution required
        ├── confidence human-asserted
        ├── seal Ember summary
        ├── seal Kindling digests/summaries
        └── commit tracked memory/provenance
```

Durable outputs:

```text
anvil/edda/memories/<type>/<memory-id>.yaml
anvil/edda/provenance/<memory-id>.source.json
```

Local-only inputs:

```text
.anvil/kindling.db
.anvil/ember.db
```

---

## 6.4 Memory → policy proposal flow

This flow should be deliberate and human-gated.

```text
Edda memory identifies durable lesson/warning/doctrine
        │
        ▼
anvil policy propose-from-memory <memory-id>
        │
        ├── generate draft policy/rule/check
        ├── link to memory provenance
        ├── evaluate against history
        ├── produce impact summary
        └── open policy PR or write proposal artefact
        │
        ▼
Human review
        │
        ▼
Policy accepted
        │
        ▼
Anvil enforces future changes
        │
        ▼
Witnesses/capsules prove enforcement
```

Important boundary:

```text
Edda memory is not automatically enforceable.
Edda can seed policy, but policy must be explicitly accepted.
```

---

## 6.5 Exception flow

```text
Anvil finding requires intentional deviation
        │
        ▼
anvil exception grant <finding-id> --scope <path> --reason ... --expires ...
        │
        ├── validate finding identity
        ├── validate scope
        ├── validate expiry
        ├── require actor/reason
        ├── write exception record
        └── commit/stage exception
        │
        ▼
Future L3/L4 evaluations
        │
        ├── load active exceptions
        ├── validate scope/expiry/policy digest
        ├── apply only to matching findings
        └── record exception use in witness/capsule
```

Revocation:

```sh
anvil exception revoke <exception-id>
```

Exception records should be soft-deleted/revoked, never erased.

---

## 6.6 Release seal flow

Future phase:

```text
Release candidate ready
        │
        ▼
anvil release seal v1.2.0
        │
        ├── source commit/tree
        ├── build inputs
        ├── dependency lockfiles
        ├── policy/rules/baseline
        ├── witness-chain verification
        ├── test/eval evidence
        ├── exceptions used
        ├── approvals
        └── deployment/config snapshots, if enabled
        │
        ▼
anvil/releases/v1.2.0/
        │
        ▼
anvil release verify v1.2.0
```

---

## 7. Git object model

### 7.1 V0: file-first model

Start with simple tracked files under `anvil/`:

```text
anvil/witness/*
anvil/evidence/capsules/*
anvil/exceptions/*
anvil/edda/*
anvil/releases/*
```

Advantages:

- easy to inspect;
- easy to review in PRs;
- easier for agents to implement;
- minimal Git plumbing required;
- matches current witness design.

### 7.2 V1+: refs/notes model

Use refs and notes for high-volume or object-attached evidence:

```text
refs/notes/anvil-l4
refs/notes/anvil-evidence
refs/anvil/sessions/<session-id>
refs/anvil/releases/<version>
```

Advantages:

- avoids working-tree noise;
- attaches evidence to commits without rewriting commits;
- supports server-side validation artefacts;
- supports fetchable governance namespaces.

Trade-off:

- less visible to normal developers;
- more Git plumbing complexity;
- requires fetch/push refspec education.

### 7.3 V2: bundles/capsules as portable Git artefacts

Use Git bundles or bundle-like archives to transport evidence offline:

```text
review.anvil-bundle
release-v1.2.0.anvil-bundle
supplier-component.anvil-bundle
```

The bundle should include all objects required for independent verification.

---

## 8. Data schemas at architecture level

Detailed schemas live in `solution.md`. At the architecture level, these are the key entities:

```text
CapsuleManifest
CapsuleVerificationResult
WitnessExtract
ExceptionRecord
SealedEddaProvenance
PolicyDigest
RuleSetDigest
BaselineAnchor
AgentCapabilityManifest
ReleaseAttestation
EvidenceRecord
ControlEvidenceMap
```

---

## 9. Verification model

A verification run should never silently claim full trust if evidence is incomplete.

Use closed-state verdicts:

```text
pass        = all required evidence verified
warn        = evidence verified with non-blocking findings
degraded    = evidence incomplete, stale, missing, or partially unverifiable
block       = policy violation, witness break, invalid exception, or disallowed state
error       = tool/internal failure; do not overclaim
```

Important distinction:

```text
error != pass
degraded != protected
missing evidence != clean evidence
```

This aligns with Anvil’s existing “honest claim only” doctrine.

---

## 10. Security and privacy architecture

### 10.1 No secrets in durable Git evidence

Never store raw secrets, raw environment dumps, raw token-bearing command output, or unredacted runtime state in Git.

Instead store:

- redacted summaries;
- digests;
- policy-relevant fields;
- source pointers;
- sealed hashes;
- optional local-only raw record references.

### 10.2 Redaction policy as part of evidence

Every sealed evidence object that summarises local observations should include:

```text
redaction_policy_sha
redaction_version
fields_omitted
fields_hashed
```

### 10.3 Ephemeral stores may decay; sealed evidence may not

Kindling and Ember may prune/decay local records. Edda provenance, witness evidence, exception records, and capsule manifests should remain durable once created.

### 10.4 Human attribution is required for durable memory and exceptions

Human action should be required for:

- Edda promotion;
- exception grant/revoke;
- policy proposal acceptance;
- release seal approval, where applicable.

### 10.5 AI may propose but not author durable truth

AI can assist with summaries, proposal detection, or policy drafts. Durable truth requires human promotion or review.

---

## 11. How this plugs into Anvil today

### 11.1 Existing assets to reuse

Current Anvil already has many of the building blocks:

```text
crates/anvil-witness           # witness writer/verifier concepts
crates/anvil-l4                # L4 policy and witness-chain validation
crates/anvil-hook              # Git hook installation and witness writes
crates/anvil-cli               # command surfaces
crates/anvil-config            # multi-format config discovery
crates/anvil-policy            # OPA/Rego wrapper
crates/anvil-policy-engine     # policy-engine internals
crates/anvil-kernel            # watcher/parser/semantic graph/policy
crates/anvil-kernel-types      # shared kernel types
packages/kindling-integration  # observation contracts/query contract
packages/edda-stack            # Kindling/Ember/Edda contracts and services
```

### 11.2 Add new command group: `anvil capsule`

Suggested CLI structure:

```text
anvil capsule create
anvil capsule verify
anvil capsule explain
anvil capsule inspect
```

The first implementation can live inside `crates/anvil-cli/src/commands/capsule.rs` and call existing crates for policy/witness/baseline reads.

### 11.3 Add new command group: `anvil exception`

Suggested CLI structure:

```text
anvil exception grant
anvil exception revoke
anvil exception list
anvil exception show
anvil exception verify
```

### 11.4 Adjust Edda storage path

Move durable Edda memory from:

```text
.anvil/edda/
```

to:

```text
anvil/edda/
```

Keep Ember/Kindling local:

```text
.anvil/ember.db
.anvil/kindling.db
```

### 11.5 Add sealed Edda provenance

Add a promotion-time seal step:

```text
anvil edda promote <proposal-id> --seal-provenance
```

Or make sealing the default once implemented.

### 11.6 Avoid Graph v2 dependency for capsule v0

Capsule v0 should collect existing evidence. Do not block on full Graph v2. Later versions can add semantic graph diffs and behavioural explanations.

---

## 12. Future-state architecture map

```text
                         ┌────────────────────────┐
                         │     Optional Cloud      │
                         │ fleet, GitHub App, UX   │
                         └───────────┬────────────┘
                                     │ optional sync/amplify
                                     ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                               Git Repository                              │
│                                                                            │
│  Source code       anvil/policy      anvil/witness      anvil/edda         │
│  Config            anvil/rules       anvil/evidence     anvil/exceptions   │
│  History           anvil/baseline    anvil/releases     refs/notes/*       │
└────────────────────────────────────────────────────────────────────────────┘
       ▲                    ▲                    ▲                    ▲
       │                    │                    │                    │
       │                    │                    │                    │
┌──────┴──────┐     ┌───────┴───────┐    ┌──────┴──────┐     ┌───────┴───────┐
│   Anvil L0  │     │   Anvil L2/L3 │    │   Anvil L4  │     │     Edda       │
│ MCP/driver  │     │ daemon/hooks  │    │ push/CI/app │     │ memory ledger  │
└──────┬──────┘     └───────┬───────┘    └──────┬──────┘     └───────┬───────┘
       │                    │                   │                    │
       ▼                    ▼                   ▼                    ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                              Local Runtime                                 │
│                                                                            │
│  .anvil/kindling.db     .anvil/ember.db     .anvil/cache/graph             │
│  .anvil/runtime         .anvil/logs         .anvil/cache/policy            │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## 13. Architectural non-goals

This architecture is not trying to build:

- a generic Git database;
- a general compliance platform;
- a knowledge graph search engine;
- a centralised cloud-only governance product;
- an AI memory product separate from Anvil;
- a replacement for code review;
- a replacement for GitHub/GitLab/Forgejo;
- a system that stores all raw agent activity forever.

The core job remains:

> Govern AI-assisted changes at creation time, and make the evidence durable, replayable, and portable.

