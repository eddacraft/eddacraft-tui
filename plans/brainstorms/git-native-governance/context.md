# Context — Git-Native Governance for Anvil

## 1. Why this document exists

This document packages the strategic direction that emerged from a conversation about using Git inside Anvil not merely as a code versioning tool, but as what it really is underneath:

> A content-addressable, tamper-evident, distributed versioning engine that can preserve and verify almost any structured state.

The conversation started with the idea that Git can version more than source code: configs, notes, server state, datasets, policies, evidence, memory, exceptions, and AI session artefacts. The deeper conclusion was that Anvil should not simply “integrate with Git”. Anvil should treat Git as the local-first trust substrate for AI engineering.

The resulting product thesis:

> **Anvil should become the Git-native governance engine for AI-assisted software.**
>
> Every AI-assisted change should be governed, replayable, explainable, and provable using the repository itself as the trust substrate.

This does **not** mean turning Git into a database for everything. It means using Git for what it is excellent at:

- content addressing;
- tamper-evident history;
- distributed replication;
- offline/air-gapped transfer;
- durable refs and tags;
- attaching evidence to objects;
- replaying past states;
- reviewing changes through a workflow developers already understand.

The rest of the system remains layered: Anvil governs, Kindling observes, Ember proposes, Edda remembers, Graph v2 accelerates queries, and Git proves durable state.

---

## 2. Existing Anvil context

The current Anvil positioning is already strongly aligned with this direction.

Anvil’s product claim is that AI agents make software probabilistic, and Anvil makes it deterministic. It sits between AI agents and production code as a deterministic governance layer, catching architectural drift, anti-patterns, security risks, and policy violations before they leave the developer’s machine.

Important current concepts in the repo:

### 2.1 Deterministic governance before review

Anvil is not a SAST scanner, linter, observability product, or compliance dashboard. It is intended to govern AI-assisted development in the developer workflow, not after the fact in the PR queue.

This matters because the Git-native direction should not turn Anvil into a retrospective reporting product. Git-native evidence is valuable because it proves the governance that happened at change time.

### 2.2 Multi-layer protection model

The multi-layer protection architecture already defines a layered enforcement model:

| Layer | Trigger | Purpose |
| --- | --- | --- |
| L0 | Pre-write / MCP | Best-effort prevention before a file is written. |
| L1 | Mid-edit / editor driver | Best-effort feedback while editing. |
| L2 | Save-time daemon watcher | Deterministic save-time validation. |
| L3 | Pre-commit hook | Deterministic local commit gate and witness append. |
| L4 | Pre-push / CI / server receive | Deterministic server-side or outbound validation. |
| L5 | Audit | Periodic or on-demand mainline verification. |

This provides the enforcement spine. The Git-native proposal provides the durable proof layer around it.

### 2.3 Witness chain

Anvil already has a witness-chain concept. The witness line shape includes:

- project ID;
- Git tree hash being committed;
- parent commit or parent commits;
- execution scope;
- Anvil version;
- effective rule-set hash;
- agent attribution;
- L0/L2/L3 status;
- previous-line hash;
- timestamp.

This is extremely close to a Git-native governance record already. The proposal is to expand the witness chain into a broader Git-native evidence fabric.

### 2.4 Tracked `anvil/` vs local `.anvil/`

The multi-layer architecture distinguishes:

```text
anvil/   = tracked metadata that must travel with the repository
.anvil/  = local execution state, cache, runtime info, logs, scratch
```

That is the right direction. It should become a hard boundary across the product.

However, the current Edda docs describe `.anvil/edda/` as a Git-tracked memory store. That conflicts with the newer convention. The proposed resolution is:

```text
.anvil/ = local runtime/caches/databases/logs
anvil/  = durable tracked governance/memory/evidence
```

### 2.5 Local-first and air-gapped posture

The architecture already emphasises that Anvil v1 should work with zero hosted infrastructure and no cloud calls in normal operation. Rule pack distribution is already expected to be Git-based rather than HTTPS-based in future.

This is a major strategic advantage. The Git-native direction should lean into it:

> Anvil Cloud may amplify the experience, but the repository remains the source of truth.

---

## 3. Existing Edda Stack context

The broader Edda Stack changes the answer to “should Git replace Kindling?” The answer is no.

The Edda Stack already has a strong three-layer architecture:

```text
Kindling observes — captures without judgement
Ember reflects — meaning without authority
Edda remembers — memory with restraint
```

It separates:

| Layer | Question | Role |
| --- | --- | --- |
| Kindling | What occurred? | Observation/facts. |
| Ember | Might this matter later? | Candidate meaning/proposals. |
| Edda | What do we know well enough to keep? | Human-curated institutional memory. |

The stack exists to avoid a common failure mode:

```text
logs become memory
memory becomes noisy
noise becomes institutional truth
```

That separation is valuable and should be preserved.

### 3.1 Kindling

Kindling is the observation layer. It captures activity, tool usage, gate evaluations, actions, constraints, human input, errors, and session lifecycle facts.

Its intended properties:

- local SQLite storage;
- write-only emission by Anvil;
- read-only bounded queries for AI/callers;
- facts only;
- no inference;
- no authority;
- sanitised and query-limited.

Kindling remains necessary because Git is not a good hot event store or live query surface.

### 3.2 Ember

Ember is the interpretive/candidate-memory layer. It consumes high-volume, low-trust Kindling observations and emits lower-volume, medium-trust proposals.

Its intended properties:

- SQLite-backed;
- ephemeral by default;
- TTL-based decay;
- heuristic scoring;
- allowed to be wrong;
- optional AI assistance, but no dependency on AI;
- no authority to create durable memory.

Built-in rule families include:

- repetition;
- escalation;
- resolution;
- convergence;
- surprise/anomaly.

Ember is useful because it prevents humans from needing to inspect raw Kindling event streams.

### 3.3 Edda

Edda is the canonical institutional memory layer. It stores only what a human has deliberately chosen to keep.

Memory types:

- decision;
- pattern;
- constraint;
- warning;
- doctrine;
- lesson.

Edda’s intended properties:

- low-volume;
- high-trust;
- human-promoted;
- versioned;
- auditable;
- append-biased;
- no automatic decay;
- AI-assisted, not AI-authored;
- provenance back to Ember and Kindling.

Edda is not a log, transcript, or database of everything. It is organisational memory.

### 3.4 Critical Edda Stack refinement

Edda currently stores provenance links back to Ember proposal IDs and Kindling observation IDs. That is good, but insufficient if Ember and Kindling are local and may decay or be pruned.

The proposed refinement:

> When an Ember proposal is promoted to Edda, Anvil should seal a minimal provenance bundle into Git.

This lets Kindling/Ember remain local and ephemeral while Edda remains durable and auditable.

---

## 4. Strategic product thesis

The core thesis that emerged:

> **Anvil turns Git from source control into a tamper-evident governance substrate for AI-assisted engineering. It records what changed, what governed it, what evidence supported it, what exceptions were used, what humans approved, and what the organisation learned.**

This can be reduced to a simple mental model:

```text
Anvil governs change.
Git proves governance.
Kindling observes work.
Ember proposes meaning.
Edda preserves institutional memory.
Graph v2 accelerates deterministic queries.
```

Or:

```text
Kindling remembers the work.
Ember notices the signal.
Edda preserves the lesson.
Git proves the chain.
```

The product category line:

> **Git-native AI governance.**

A stronger external line:

> **The trust layer for AI-generated software, built into the repository itself.**

---

## 5. Why Git is the right substrate

Git is useful here because it is already the universal trust and collaboration substrate for engineering teams.

Anvil can exploit Git’s native primitives:

| Git primitive | Governance use |
| --- | --- |
| Blob | Evidence, diagnostics, policy, sealed provenance, eval result. |
| Tree | Exact source/config/policy/evidence state. |
| Commit | Governed change event. |
| Tag | Release, policy version, audit anchor. |
| Ref | Named governance stream. |
| Notes | Attach evidence to commits without rewriting commits. |
| Bundle | Portable review/audit capsule for offline or air-gapped verification. |
| Worktree | Isolated agent workspace. |

Git is excellent for:

- durable evidence;
- append-biased records;
- policy versions;
- reviewable exceptions;
- replayable states;
- offline transfer;
- supplier/vendor trust exchange;
- air-gapped enterprise workflows.

Git is poor for:

- high-frequency telemetry;
- mutable queues;
- large raw datasets without a manifest strategy;
- secret-bearing runtime dumps;
- millisecond hot-path queries.

Therefore the architecture should be:

```text
Git = canonical evidence and transport
SQLite/Graph cache = fast local query
Anvil daemon = enforcement and projection
Cloud = optional federation/amplifier
```

---

## 6. Ideas captured from the brainstorm

The conversation surfaced a broad set of Git-native product ideas. They should not all be built immediately, but they define the strategic map.

### 6.1 Every Anvil decision as a Git object

Every governance decision can become content-addressed evidence:

```text
refs/anvil/witness/main
refs/anvil/sessions/<session-id>
refs/anvil/gates/<gate-eval-id>
refs/anvil/evidence/<control-id>/<period>
refs/anvil/policies/<policy-pack>/<version>
refs/notes/anvil-l3
refs/notes/anvil-l4
refs/notes/anvil-evidence
```

Product commands:

```sh
anvil attest HEAD
anvil verify HEAD
anvil explain HEAD --evidence
anvil trust-log --since main~20
```

### 6.2 Anvil Review Capsules

Portable proof that a change was governed:

```sh
anvil capsule create --range main..HEAD --out review.anvil-capsule
anvil capsule verify review.anvil-capsule
anvil capsule explain review.anvil-capsule
```

Capsules should include commits, policy, baseline, witnesses, diagnostics, exceptions, session attribution, Edda memory references, and a verification manifest.

This is the recommended first product wedge.

### 6.3 Git-native policy packs

Policy packs can be Git repos or Git refs:

```text
refs/anvil-policy/stable
refs/anvil-policy/eu-ai-act/high-risk
refs/anvil-policy/soc2/change-management
refs/anvil-policy/internal/platform-boundaries
```

Install by Git identity:

```sh
anvil policy add git@github.com:eddacraft/policy-eu-ai-act.git --tag v2026.08
anvil policy pin eu-ai-act@v2026.08
anvil policy verify
```

Every witness line should be meaningful only when it can prove the exact rule set that produced it.

### 6.4 Time-travel governance

Replay and compare decisions across time:

```sh
anvil replay --at <commit>
anvil replay --from v0.6.0 --to main --policy eu-ai-act@v2026.08
anvil would-pass HEAD --as-of 2026-05-01
anvil policy bisect
anvil bisect-trust
```

This lets teams answer:

- When did this repo become non-compliant?
- Which policy change caused this false-positive wave?
- Would last month’s release pass today’s rules?
- Did an exception exist before the risky change?

### 6.5 Exceptions as Git objects

Exceptions should be reviewable, reversible, scoped, expiring, and auditable.

```sh
anvil exception grant AI-001 \
  --scope src/billing/** \
  --expires 2026-07-01 \
  --owner sarah@example.com \
  --reason "Temporary migration shim"

anvil exception revoke <exception-id>
anvil exception list
```

This prevents governance exceptions from becoming invisible PR comments.

### 6.6 Git-backed compliance evidence workspace

CEWS can store control/evidence mappings under Git-tracked structures:

```text
anvil/evidence/soc2/CC8.1/2026-Q2
anvil/evidence/eu-ai-act/article-50/2026-Q2
anvil/evidence/iso27001/A.8.32/2026-Q2
```

Commands:

```sh
anvil evidence export --framework soc2 --period 2026-Q2
anvil evidence verify evidence-pack.bundle
```

### 6.7 AI session black box

Kindling can capture the live operational stream. Anvil can seal selected session summaries into Git:

```sh
anvil session seal <session-id>
```

Sealed session summaries can include:

- session ID;
- agent/tool identity;
- actions;
- files touched;
- gates evaluated;
- constraints applied;
- human inputs;
- diagnostic hashes;
- redaction policy;
- witness head.

### 6.8 Agent capability manifests

AI tools can declare capabilities as Git-pinned manifests:

```yaml
agent: claude-code
version: 1.2.8
capabilities:
  can_write_code: true
  can_modify_infra: false
  can_touch_secrets: false
  max_files_per_change: 12
allowed_paths:
  - src/**
  - tests/**
forbidden_paths:
  - infra/**
  - .github/workflows/**
```

Anvil can enforce:

> This agent modified infrastructure, but its capability manifest did not permit infrastructure changes.

### 6.9 Runtime/config state snapshots

Anvil can normalise and redacted operational state into Git trees:

```sh
anvil snapshot github-rulesets
anvil snapshot vercel
anvil snapshot pulumi
anvil snapshot terraform
anvil snapshot kubernetes
anvil snapshot local-dev-env
```

This supports state accountability without storing secrets.

### 6.10 Semantic graph snapshots

Graph v2 can store deterministic graph snapshots/deltas as derivable Git objects:

```text
refs/anvil/graph/snapshots/<commit-sha>
refs/anvil/graph/deltas/<from>..<to>
```

Commands:

```sh
anvil graph diff main~1..main
anvil edge trace src/api -> infra/db
anvil graph verify --from-source
anvil bisect-edge --finding ARCH-BOUNDARY
```

### 6.11 Deterministic eval storage

Anvil can version eval corpora and results:

```text
refs/anvil/evals/cases/<suite-id>
refs/anvil/evals/results/<run-id>
```

Command:

```sh
anvil policy test --against-history main --last 500 commits
```

### 6.12 Branches as governance experiments

Policies can have canary branches:

```text
policy/eu-ai-act-v1
policy/eu-ai-act-v2
policy/internal-strict
policy/relaxed-beta
```

Compare impact:

```sh
anvil policy compare policy/eu-ai-act-v1..policy/eu-ai-act-v2 --against main~500..main
```

### 6.13 Governance PRs for policy changes

Policy changes should generate impact PRs:

```sh
anvil policy propose ./new-policy-pack
```

The PR body can explain new blocks, new warnings, resolved false positives, changed control coverage, and exception impact.

### 6.14 Supplier/vendor trust exchange

Vendors can send Anvil bundles/capsules as trust artefacts:

```sh
anvil supplier verify vendor-component-v2.3.1.anvil-bundle \
  --policy our-supplier-policy.yml
```

This avoids forcing all suppliers into a SaaS tenant.

### 6.15 Decision DAGs

Important decisions can be preserved as small Git object trees:

```text
decision.json
inputs/
  plan.md
  evidence.json
  diagnostics.sarif
  policy.yml
approvals/
  josh.sig
  security.sig
outputs/
  verdict.json
```

Referenced under:

```text
refs/anvil/decisions/<decision-id>
refs/notes/anvil-decisions
```

### 6.16 Release black box

Seal a release:

```sh
anvil release seal v1.2.0
anvil release verify v1.2.0
anvil release diff v1.2.0..v1.2.1 --trust
```

Release record includes source commit, build inputs, dependency lockfiles, policy versions, witness verification, tests, evals, exceptions, approvals, and deployment state snapshots.

---

## 7. The recommended first wedge

The recommended first wedge is **Anvil Review Capsules**.

Why this first:

- it packages the entire Git-native strategy into one concrete user experience;
- it demonstrates local-first / no-cloud trust;
- it is useful to regulated teams, auditors, internal security, and external vendors;
- it builds mostly on existing Anvil concepts: witness, policy, baseline, diagnostics;
- it avoids premature Graph v2, cloud, marketplace, or full evidence workspace complexity.

The first capsule can be simple:

```text
review-capsule/
  manifest.json
  commits.json
  policy.json
  baseline.json
  witness.ndjson
  diagnostics.sarif
  exceptions.json
  edda-context.json
  verification.json
```

It can later become a Git bundle or include Git refs/notes.

Success criterion:

> A developer can create a capsule from a branch and another person can verify it locally without trusting Anvil Cloud.

---

## 8. What not to do yet

Avoid building the universe.

Do not start with:

- full Git refs/notes architecture;
- hosted Anvil Cloud;
- policy marketplace;
- full Graph v2 dependency;
- config/server snapshots;
- AI-heavy Ember intelligence;
- Edda as an enforcement source;
- generic dashboards;
- broad GRC workflow replacement.

Instead:

> Prove governed-change evidence first.

---

## 9. Key unresolved decisions

1. **Path model** — move durable Edda state to `anvil/edda/` and reserve `.anvil/` for local runtime.
2. **Capsule format v0** — directory/tarball first, Git bundle later?
3. **Witness inclusion** — exact source of L3/L4 witness evidence in current code.
4. **Edda provenance sealing** — schema and timing during promotion.
5. **Exception storage** — tracked files vs refs/notes for v0.
6. **Policy digest semantics** — what exactly defines `rules_sha` and effective policy identity.
7. **Signing** — when to add cryptographic signatures beyond Git hashes.
8. **Cloud role** — confirm cloud remains optional amplifier, never source of truth.

---

## 10. Repo source anchors to inspect

These paths are the most relevant source anchors from the current repo:

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

Before implementation, agents should re-open and verify these files against current `main`, because the repository is moving quickly.

