# Solution — PRD and Technical Design

## 1. Product definition

## 1.1 Product name

Working name:

> **Git-Native Governance Substrate**

First user-facing feature:

> **Anvil Review Capsules**

## 1.2 Product thesis

> Anvil turns Git from source control into a tamper-evident governance substrate for AI-assisted engineering.
>
> Every AI-assisted change can be packaged, replayed, explained, and verified using evidence stored in or derived from the repository itself.

## 1.3 Primary user problem

AI-assisted development increases the speed and volume of change. Existing review and compliance processes struggle to answer basic governance questions:

- Was this AI-assisted change checked before it left the developer machine?
- Which policy and rule version governed it?
- Did it use an exception?
- Was the exception scoped and approved?
- What agent/session produced the change?
- What evidence supports the decision?
- Can this be verified without trusting a hosted dashboard?
- Can an auditor, security reviewer, or supplier verify the same facts later?

## 1.4 Product answer

Anvil packages change-level governance into portable, Git-backed evidence:

```sh
anvil capsule create --range main..HEAD --out review.anvil-capsule
anvil capsule verify review.anvil-capsule
anvil capsule explain review.anvil-capsule
```

The capsule proves:

- what changed;
- what Anvil version and policy evaluated it;
- what rule set was active;
- what baseline applied;
- what witnesses exist;
- what diagnostics were produced;
- what exceptions were used;
- what session/agent attribution exists;
- what relevant institutional memory applied;
- whether verification succeeds, warns, degrades, or blocks.

---

## 2. Personas

| Persona | Need |
| --- | --- |
| Developer | Show that their AI-assisted PR was governed without extra manual evidence work. |
| Tech lead | Understand whether a risky change is safe to review or merge. |
| Security engineer | Verify policy, exceptions, and witness integrity. |
| Compliance/audit reviewer | Receive portable evidence for a release, PR, or supplier component. |
| Platform engineer | Roll out governance without adding hosted infrastructure or developer friction. |
| External vendor/supplier | Prove a delivered component was developed under agreed governance controls. |

---

## 3. Goals and non-goals

## 3.1 Goals

- Create a portable review capsule for a commit range.
- Verify a capsule locally without Anvil Cloud.
- Include Anvil witness, policy, baseline, rule digest, diagnostics, and exceptions.
- Produce honest verification states: `pass`, `warn`, `degraded`, `block`, `error`.
- Preserve local-first and air-gapped operation.
- Avoid storing secrets or raw noisy logs in durable Git evidence.
- Connect to Edda memory through sealed provenance summaries, not raw Kindling dumps.
- Establish `anvil/` vs `.anvil/` durable/local boundary.

## 3.2 Non-goals for MVP

- Full Git notes/ref namespace implementation.
- Hosted cloud dashboard.
- Policy marketplace.
- Full compliance evidence workspace.
- Graph v2 behavioural diff.
- Full server/config snapshotting.
- Cryptographic signing beyond existing Git/content hashes.
- Automatic policy generation from Edda memory.
- Supplier portal.
- General event lake.

---

## 4. MVP: Anvil Review Capsules

## 4.1 Command surface

```sh
anvil capsule create --range <base>..<head> --out <path>
anvil capsule verify <path>
anvil capsule explain <path>
anvil capsule inspect <path> --json
```

Optional later flags:

```sh
anvil capsule create --pr <number> --out <path>
anvil capsule create --range main..HEAD --include-edda
anvil capsule create --range main..HEAD --include-sessions
anvil capsule create --range main..HEAD --format directory|tar|bundle
anvil capsule verify <path> --policy <policy.yml>
anvil capsule explain <path> --finding <id>
```

## 4.2 MVP capsule format

Start with a directory or tarball-like format. Do not begin with Git bundles. Keep the initial shape inspectable.

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

Later this can be packed into a single archive or Git bundle.

## 4.3 Capsule creation algorithm

Input:

```text
base/head commit range
repository root
current Anvil config
current policy/baseline/rules
available witness files
available diagnostics
active exceptions
optional Edda context
```

Algorithm:

1. Resolve repository root.
2. Resolve commit range.
3. Compute list of commits and tree hashes.
4. Load effective Anvil policy.
5. Compute policy digest.
6. Load effective rules.
7. Compute rules digest.
8. Load baseline anchor.
9. Collect witness lines relevant to commits/trees in range.
10. Run or load diagnostics for the range.
11. Collect active exceptions that apply to findings or paths in range.
12. Optionally collect relevant Edda memories and sealed provenance summaries.
13. Write capsule files.
14. Hash every file.
15. Write `manifest.json` with file digests.
16. Run initial verification and write `verification.json`.

## 4.4 Capsule verification algorithm

Input:

```text
capsule path
optional repository path
optional policy override
```

Algorithm:

1. Read `manifest.json`.
2. Verify schema version compatibility.
3. Verify every declared file hash.
4. Verify commit list and tree hashes, if repository is present.
5. Verify policy digest and rule digest consistency.
6. Verify baseline anchor.
7. Verify witness chain integrity.
8. Verify every witness line references a commit/tree in range or relevant parent.
9. Verify diagnostics match witness policy outcomes, where possible.
10. Verify exceptions:
    - scope matches;
    - finding IDs match;
    - expiry valid at evaluation time;
    - policy/rule digest compatible;
    - not revoked.
11. Verify Edda context, if present:
    - memory files hash;
    - sealed provenance hash;
    - no raw Kindling dependency required.
12. Emit `pass`, `warn`, `degraded`, `block`, or `error`.

## 4.5 Verification states

| State | Meaning | Merge/review implication |
| --- | --- | --- |
| `pass` | Required evidence verified and no block-level finding remains. | Safe to rely on. |
| `warn` | Evidence verified, but warnings exist. | Review warning details. |
| `degraded` | Evidence incomplete, missing, stale, or partially unverifiable. | Do not claim protected. Human review required. |
| `block` | Policy violation, invalid exception, witness break, or disallowed state. | Must not proceed under strict policy. |
| `error` | Tool/internal failure. | Do not overclaim; retry or inspect logs. |

## 4.6 `anvil capsule explain`

Explain should produce human-readable output:

```text
Capsule: review.anvil-capsule
Range: main..HEAD
Commits: 4
Policy: anvil/policy.yml sha256:...
Rules: 42 rules sha256:...
Baseline: cutoff <sha>
Witness: verified, 4/4 commits covered
Diagnostics: 0 block, 2 warn
Exceptions: 1 used, valid until 2026-07-01
Edda context: 3 memories referenced
Verdict: warn
```

For a finding:

```sh
anvil capsule explain review.anvil-capsule --finding ARCH-BOUNDARY-001
```

Output should answer:

- what triggered;
- which files/paths;
- which rule;
- what policy decided;
- whether exception applied;
- what witness recorded;
- what Edda memory/policy doctrine is relevant;
- what the user can do next.

---

## 5. Data model

## 5.1 CapsuleManifest

```jsonc
{
  "schema": "anvil.capsule.v1",
  "capsule_id": "cap_01J...",
  "created_at": "2026-06-05T12:34:56Z",
  "created_by": "josh",
  "anvil_version": "0.7.2-beta",
  "repository": {
    "project_id": "01997e4a-...",
    "origin_canonical": "github.com/eddacraft/anvil",
    "base_commit": "...",
    "head_commit": "...",
    "range": "main..HEAD"
  },
  "policy": {
    "path": "anvil/policy.yml",
    "digest": "sha256:...",
    "required_anvil_version": "0.7.2-beta"
  },
  "rules": {
    "digest": "sha256:...",
    "count": 42
  },
  "baseline": {
    "cutoff_commit": "...",
    "digest": "sha256:..."
  },
  "files": [
    { "path": "commits.json", "digest": "sha256:..." },
    { "path": "witness.ndjson", "digest": "sha256:..." },
    { "path": "diagnostics.sarif", "digest": "sha256:..." }
  ],
  "verification": {
    "initial_verdict": "warn",
    "verified_at": "2026-06-05T12:34:56Z"
  }
}
```

## 5.2 CommitRecord

```jsonc
{
  "sha": "...",
  "tree": "...",
  "parents": ["..."],
  "author": {
    "name": "...",
    "email_hash": "sha256:..."
  },
  "committer": {
    "name": "...",
    "email_hash": "sha256:..."
  },
  "timestamp": "...",
  "changed_paths": ["src/foo.ts", "tests/foo.test.ts"]
}
```

Hash or omit email depending on privacy posture.

## 5.3 WitnessExtract

Use existing witness line fields where possible.

```jsonc
{
  "v": 1,
  "project_id": "...",
  "tree": "<git-tree-hash>",
  "parent_commit": "...",
  "parent_commits": ["..."],
  "scope": "linux:8d3f1a2c",
  "anvil_version": "0.7.2-beta",
  "rules_sha": "sha256:...",
  "agent": {
    "task_id": "...",
    "step_id": "...",
    "parent_session_id": "..."
  },
  "L0": { "status": "miss", "reason": "no-mcp" },
  "L2": { "status": "ok", "watcher_health": "ok" },
  "L3": { "status": "ok", "rules": 42, "mode": "block", "latency_ms": 120 },
  "L4": { "status": "ok", "backend": "ci" },
  "prev_line_hash": "sha256:...",
  "line_hash": "sha256:...",
  "ts": "2026-06-05T12:34:56Z"
}
```

## 5.4 PolicyDigest

```jsonc
{
  "schema": "anvil.policy-digest.v1",
  "effective_policy_files": [
    { "path": "anvil/policy.yml", "digest": "sha256:..." }
  ],
  "effective_config_files": [
    { "path": ".anvilrc", "digest": "sha256:..." }
  ],
  "policy_digest": "sha256:...",
  "normalisation": {
    "format": "canonical-json",
    "version": 1
  }
}
```

## 5.5 RuleSetDigest

```jsonc
{
  "schema": "anvil.rules-digest.v1",
  "built_in_rules": {
    "anvil_version": "0.7.2-beta",
    "registry_digest": "sha256:..."
  },
  "custom_rules": [
    { "path": "anvil/rules/no-cross-boundary.rego", "digest": "sha256:..." }
  ],
  "policy_packs": [
    {
      "name": "architecture-boundaries",
      "version": "v1.0.0",
      "source": "git:...",
      "digest": "sha256:..."
    }
  ],
  "rules_sha": "sha256:..."
}
```

## 5.6 ExceptionRecord

```jsonc
{
  "schema": "anvil.exception.v1",
  "id": "exc_01J...",
  "status": "active",
  "finding": {
    "id": "ARCH-BOUNDARY-001",
    "finding_hash": "sha256:..."
  },
  "scope": {
    "paths": ["src/billing/**"],
    "commit_range": null,
    "policy_digest": "sha256:..."
  },
  "reason": "Temporary migration shim",
  "owner": "sarah@example.com",
  "created_by": "josh",
  "created_at": "2026-06-05T12:34:56Z",
  "expires_at": "2026-07-01T00:00:00Z",
  "revoked_at": null,
  "revoked_by": null,
  "revoked_reason": null
}
```

Stored at:

```text
anvil/exceptions/active/<exception-id>.json
anvil/exceptions/revoked/<exception-id>.json
```

Or as status-mutated tracked files, depending on implementation preference.

## 5.7 SealedEddaProvenance

```jsonc
{
  "schema": "anvil.edda-provenance.v1",
  "memory_id": "mem_a1b2c3d4",
  "sealed_at": "2026-06-05T12:34:56Z",
  "sealed_by": "alice",
  "redaction": {
    "policy_sha": "sha256:...",
    "version": 1,
    "fields_omitted": ["raw_command_output"],
    "fields_hashed": ["user_email"]
  },
  "ember_proposal": {
    "id": "prop_ember_001",
    "type": "pattern",
    "confidence": 0.82,
    "summary": "Repeated cross-package import boundary drift",
    "rationale": "Observed in three sessions across billing and auth work",
    "created_at": "2026-06-04T09:00:00Z",
    "digest": "sha256:..."
  },
  "kindling_sources": [
    {
      "observation_id": "obs_abc001",
      "kind": "gate_evaluated",
      "session_id": "ses_xyz001",
      "summary": "Architecture boundary gate warning",
      "digest": "sha256:..."
    }
  ],
  "source_sessions": ["ses_xyz001"],
  "provenance_status": "sealed"
}
```

Stored at:

```text
anvil/edda/provenance/<memory-id>.source.json
```

## 5.8 EddaMemoryReference for capsules

Capsules should not include all Edda memory by default. They should include references and selected relevant memories.

```jsonc
{
  "memory_id": "mem_a1b2c3d4",
  "type": "doctrine",
  "status": "active",
  "confidence": "high",
  "statement": "All cross-package imports use published package names.",
  "scope": "monorepo",
  "tags": ["imports", "boundaries"],
  "memory_digest": "sha256:...",
  "sealed_provenance_digest": "sha256:...",
  "relevance": {
    "reason": "Policy rule ARCH-BOUNDARY-001 references this doctrine",
    "paths": ["packages/**"]
  }
}
```

---

## 6. Path migration design

## 6.1 Current conflict

Current Edda docs describe `.anvil/edda/` as tracked. The newer protection architecture reserves `.anvil/` for local runtime state and `anvil/` for tracked state.

## 6.2 Decision

Adopt:

```text
.anvil/ = local runtime only
anvil/  = durable tracked state
```

## 6.3 Migration command

```sh
anvil edda migrate-storage --from .anvil/edda --to anvil/edda
```

Behaviour:

1. Detect existing `.anvil/edda/`.
2. Validate memory schema.
3. Copy to `anvil/edda/`.
4. Rebuild `anvil/edda/index.yaml`.
5. Write migration witness/evidence record.
6. Update config recommendation.
7. Leave backup or instructions for cleanup.
8. Do not delete old data automatically unless explicitly requested.

## 6.4 Config shape

```jsonc
{
  "edda": {
    "enabled": true,
    "storage": {
      "type": "git",
      "path": "anvil/edda/",
      "format": "yaml"
    },
    "promotion": {
      "require_reason": true,
      "require_attribution": true,
      "min_ember_confidence": 0.5,
      "seal_provenance": true
    }
  },
  "kindling": {
    "enabled": true,
    "database_path": ".anvil/kindling.db"
  },
  "ember": {
    "enabled": true,
    "database": ".anvil/ember.db"
  }
}
```

---

## 7. Edda integration design

## 7.1 Promotion lifecycle

```text
Ember proposal active
        │
        ▼
Human runs anvil edda promote
        │
        ├── validate proposal exists
        ├── validate confidence threshold
        ├── require actor
        ├── require reason
        ├── human chooses memory type and confidence
        ├── create memory YAML
        ├── seal provenance summary
        ├── mark proposal promoted
        └── stage/commit or instruct user to commit
```

## 7.2 Edda memory should not directly enforce policy

Edda memory can inform:

- capsule explanation;
- policy proposals;
- reviewer context;
- agent context;
- documentation.

Edda memory should not automatically block code. Blocking requires explicit policy.

## 7.3 Policy proposal from memory

Later command:

```sh
anvil policy propose-from-memory <memory-id> \
  --pack architecture-boundaries \
  --mode warn
```

Generated artefact:

```text
anvil/policy-proposals/<proposal-id>/
  proposal.json
  source-memory.yaml
  draft-policy.yml
  draft-rule.rego
  impact-summary.json
```

This is a future phase, not capsule MVP.

---

## 8. Exception design

## 8.1 Why exceptions are first-class

Governance systems fail when exceptions become invisible or permanent. Anvil should make exceptions:

- scoped;
- expiring;
- attributed;
- reasoned;
- reviewable;
- traceable;
- included in capsules;
- revocable without erasure.

## 8.2 Command surface

```sh
anvil exception grant <finding-id> \
  --scope <glob> \
  --reason <reason> \
  --owner <owner> \
  --expires <date>

anvil exception revoke <exception-id> --reason <reason> --by <actor>
anvil exception list
anvil exception show <exception-id>
anvil exception verify
```

## 8.3 Enforcement algorithm

During L3/L4 evaluation:

1. Load active exceptions.
2. Filter by finding ID.
3. Filter by path/scope.
4. Check expiry at evaluation timestamp.
5. Check policy/rules digest compatibility if pinned.
6. Check revocation status.
7. Apply only to matching finding instance.
8. Record exception use in witness/capsule.

## 8.4 Exception use in witness

Extend witness line or outer envelope with:

```jsonc
{
  "exceptions_used": [
    {
      "id": "exc_01J...",
      "finding_id": "ARCH-BOUNDARY-001",
      "scope_match": "src/billing/**",
      "expires_at": "2026-07-01T00:00:00Z",
      "digest": "sha256:..."
    }
  ]
}
```

Do not mutate canonical `Diagnostic` if the architecture requires layer metadata to live in outer envelopes.

---

## 9. Policy pack design

## 9.1 V0

Capsules should only digest existing policy and rules.

## 9.2 V1

Support Git-native policy packs:

```sh
anvil policy add <git-url> --tag <tag>
anvil policy pin <pack>@<version>
anvil policy verify
```

Pack structure:

```text
policy-pack/
├── manifest.json
├── policy.yml
├── rules/
│   └── *.rego
├── controls/
│   └── *.yaml
├── tests/
│   └── *.jsonl
└── CHANGELOG.md
```

Manifest:

```jsonc
{
  "schema": "anvil.policy-pack.v1",
  "name": "eu-ai-act-high-risk",
  "version": "2026.08",
  "description": "Policy pack for EU AI Act high-risk engineering evidence",
  "rules_digest": "sha256:...",
  "controls_digest": "sha256:...",
  "compatibility": {
    "min_anvil_version": "0.8.0"
  }
}
```

## 9.3 Policy impact PRs

Future command:

```sh
anvil policy propose ./policy-pack
```

Impact summary:

```jsonc
{
  "history_range": "main~1000..main",
  "new_blocks": 12,
  "new_warnings": 48,
  "resolved_false_positives": 7,
  "controls_added": 4,
  "controls_removed": 0,
  "exceptions_impacted": 3
}
```

---

## 10. Review Capsule PRD

## 10.1 User stories

### Developer

As a developer using AI tools, I want to create a capsule for my branch so that reviewers can verify that my change was governed.

Acceptance:

- `anvil capsule create --range main..HEAD` succeeds.
- The capsule includes policy, baseline, witness, diagnostics, and exceptions.
- The capsule explain output is readable.

### Reviewer

As a reviewer, I want to verify a capsule locally so I can see whether a change was protected, degraded, or blocked.

Acceptance:

- `anvil capsule verify <path>` returns a clear verdict.
- Missing evidence returns `degraded`, not `pass`.
- Invalid exceptions return `block` or `degraded` depending on policy.

### Security engineer

As a security engineer, I want to inspect exceptions used in a change so I can ensure they were scoped and approved.

Acceptance:

- Capsule includes exception records.
- Verification checks expiry and scope.
- Explain output names exception owner/reason.

### Auditor

As an auditor, I want portable evidence for a PR/release so I can verify it later without needing access to Anvil Cloud.

Acceptance:

- Capsule is self-describing.
- Manifest contains digests.
- Verification can run offline if required Git objects are present.

## 10.2 User experience principles

- Silent or terse on success.
- Honest on missing evidence.
- No false “protected” claims.
- Human-readable by default.
- JSON output for CI.
- No secret leakage.
- Every block has an actionable next step.

## 10.3 CLI output examples

### Create

```text
anvil: capsule created review.anvil-capsule
range: main..HEAD
commits: 4
witness: 4/4 covered
policy: sha256:8bf...
verdict: warn
```

### Verify pass

```text
anvil: capsule verified
verdict: pass
commits: 4
witness: ok
policy: ok
exceptions: none
```

### Verify degraded

```text
anvil: capsule verification degraded
reason: 1 commit has no L3 witness and no L4 fallback evidence
next: run anvil capsule explain review.anvil-capsule --missing-evidence
```

### Verify block

```text
anvil: capsule blocked
reason: exception exc_01J expired before commit 3f91...
next: revoke or refresh exception, then re-run validation
```

---

## 11. Technical implementation sketch

## 11.1 Crate/module proposal

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

Or start inside `crates/anvil-cli` and extract once stable.

## 11.2 Core traits

```rust
pub trait CapsuleCollector {
    fn collect(&self, input: CapsuleCreateInput) -> Result<CapsuleDraft, CapsuleError>;
}

pub trait CapsuleWriter {
    fn write(&self, draft: CapsuleDraft, output: &Path) -> Result<CapsuleManifest, CapsuleError>;
}

pub trait CapsuleVerifier {
    fn verify(&self, input: CapsuleVerifyInput) -> Result<CapsuleVerification, CapsuleError>;
}

pub trait CapsuleExplainer {
    fn explain(&self, capsule: &Capsule, options: ExplainOptions) -> Result<Explanation, CapsuleError>;
}
```

## 11.3 Dependencies to reuse

Potential existing crates/surfaces:

```text
anvil-config       -> config discovery
anvil-policy       -> policy loading/evaluation
anvil-policy-engine -> finding/verdict concepts
anvil-witness      -> witness read/verify
anvil-l4           -> chain validation / L4 policy
anvil-baseline     -> baseline anchor/finding model
anvil-rules        -> deterministic rules digest
anvil-kernel-types -> diagnostic/protection claim types
anvil-observability -> redaction/tracing conventions
```

Agents should inspect exact APIs before planning implementation.

## 11.4 Error handling

Capsule commands should avoid panic paths and preserve Anvil’s noise discipline.

Suggested error classes:

```text
CapsuleError::RepoNotFound
CapsuleError::InvalidRange
CapsuleError::PolicyMissing
CapsuleError::WitnessMissing
CapsuleError::WitnessChainBroken
CapsuleError::DigestMismatch
CapsuleError::ExceptionInvalid
CapsuleError::SchemaUnsupported
CapsuleError::Io
CapsuleError::Internal
```

Map to verdicts carefully:

```text
Invalid range        -> error
Digest mismatch      -> block
Witness missing      -> degraded or block, depending policy
Policy missing       -> degraded or error, depending context
Invalid exception    -> block
Internal failure     -> error
```

---

## 12. Test strategy

## 12.1 Unit tests

- manifest serialisation/deserialisation;
- digest calculation;
- schema version handling;
- exception scope matching;
- exception expiry;
- witness extract parsing;
- verification state mapping;
- redaction field handling.

## 12.2 Integration tests

Fixtures:

```text
fixtures/capsule/pass/
fixtures/capsule/warn/
fixtures/capsule/degraded-missing-witness/
fixtures/capsule/block-invalid-exception/
fixtures/capsule/block-digest-mismatch/
```

Test commands:

```sh
cargo test -p anvil-capsule
cargo test -p anvil-cli capsule
```

## 12.3 End-to-end smoke

Create a temp Git repo:

1. initialise Anvil;
2. create baseline;
3. make passing commit;
4. make warning commit;
5. grant exception;
6. create capsule;
7. verify capsule;
8. mutate capsule file;
9. verify detects digest mismatch.

## 12.4 Golden output tests

Store expected `anvil capsule explain` output for pass/warn/degraded/block.

---

## 13. Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Scope creep into full GRC platform | Keep MVP to review capsules and deterministic evidence. |
| Git storage becomes noisy | Start file-first, selective evidence only, refs/notes later. |
| Secrets leak into capsules | Redaction policy, default summaries/digests, no raw logs. |
| Missing evidence falsely passes | Strict closed verdict states; `degraded` instead of `pass`. |
| Edda provenance breaks after pruning | Seal promotion provenance into Git. |
| Exceptions become permanent bypasses | Expiry, scope, revocation, capsule inclusion. |
| Graph v2 dependency delays MVP | Avoid Graph v2 for capsule v0. |
| Existing Edda implementation bugs undermine trust | Fix provenance/status/version tracker issues before making Edda load-bearing. |
| Developers dislike extra files | Generate only on demand; keep capsule outside repo by default or under `anvil/evidence/capsules/` only when requested. |

---

## 14. Open technical decisions

1. Should capsule v0 be a directory, tarball, or custom extension wrapping a tarball?
2. Should `anvil capsule create` run fresh validation or only package existing evidence?
3. How strict should missing L3 witness be if L4 evidence exists?
4. Should capsule verify require the original repository, or support detached verification with included commit metadata only?
5. How much commit metadata can be included without privacy concerns?
6. How should policy/rule digest canonicalisation work across YAML/JSON/TOML/Rego?
7. Where should active exceptions live: one file per exception or append-only log?
8. Should Edda sealed provenance include summaries or only digests in v0?
9. Should capsule creation stage/commit evidence or leave it as an external artefact?
10. When should signing be introduced?

---

## 15. Suggested acceptance criteria for first release

A first release of Git-native governance should be considered successful when:

- `anvil capsule create --range main..HEAD` works in a real Anvil repo.
- `anvil capsule verify` can detect pass/warn/degraded/block scenarios.
- Capsule manifest is schema-versioned and digest-protected.
- Witness evidence is included and checked.
- Policy/baseline/rules digests are included and checked.
- Exceptions are included if used and validated for scope/expiry.
- The command works offline.
- No raw Kindling logs are included by default.
- Documentation explains `.anvil/` vs `anvil/` state boundary.
- Tests include tampering/digest mismatch.

---

## 16. Future phases

After capsules prove the product loop:

1. **Sealed Edda provenance** — make memory provenance durable even if Kindling/Ember decay.
2. **Git-native exceptions** — full exception lifecycle and enforcement integration.
3. **Policy packs** — Git-native distribution and pinning.
4. **Policy impact PRs** — evaluate policy changes against history.
5. **Memory-to-policy loop** — deliberate policy proposals from Edda memories.
6. **Release seal** — release-level trust capsule.
7. **Supplier bundles** — vendor/supplier verification workflow.
8. **Graph snapshots** — behavioural diff and semantic replay.
9. **Config/state snapshots** — controlled, redacted operational state evidence.
10. **Cloud amplifier** — optional fleet and GitHub App surfaces.

