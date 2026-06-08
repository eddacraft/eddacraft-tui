# Git-Native Governance Substrate

| ID | Owner | Status |
|----|-------|--------|
| GITGOV | @josh | In Progress |

**Last reviewed:** 2026-06-08

> **Decision gate cleared (2026-06-08).** [ADR-072](../decisions/072-git-native-governance-substrate.md)
> (Git substrate) and [ADR-074](../decisions/074-review-capsule-v0-format.md)
> (capsule v0 format) were Accepted via full council review
> (accept-with-changes; changes applied). Capsule implementation items
> (GITGOV-003+) are authorised. Brainstorm:
> [`../brainstorms/git-native-governance/`](../brainstorms/git-native-governance/).

## Purpose

Establish Git as the durable substrate for Anvil governance evidence, delivered
first as **Anvil Review Capsules**: a portable, file-first artefact that packages
a commit range's governance evidence (policy/rules/baseline digests, witness
lines, diagnostics, applied exceptions) so a reviewer, auditor, or supplier can
verify locally — `pass`/`warn`/`degraded`/`block`/`error` — without trusting
Anvil Cloud.

## In Scope

- `anvil capsule create|verify|explain|inspect` command surface.
- File-first capsule directory + digest manifest (`anvil.capsule.v1`, ADR-074).
- Collectors that **reuse existing crates** — no parallel evidence models:
  commit range, policy/rules/baseline digests, verbatim witness lines, SARIF
  diagnostics, applied exceptions (from EXCEPT).
- Verification engine with honest closed-state verdicts; tamper + missing-
  evidence tests.

## Out of Scope (v0 — see ADR-074 "Deferred")

Git bundles / `.anvil-bundle` packing; `refs/anvil/*` and `refs/notes/anvil-*`
namespaces; cryptographic signing beyond Git/content hashes; Graph-V2
behavioural diff; `--include-sessions`; policy-pack distribution (POLICY-GIT);
release seals (RELEASE-SEAL); supplier bundles (SUPPLIER). Sealed Edda context
(`--include-edda`) is reference-only and gated on EDDA-SEAL.

## Interfaces

- New crate `crates/anvil-capsule` (`manifest.rs`, `collect.rs`, `verify.rs`,
  `explain.rs`, `format.rs`, `errors.rs`), CLI surface in
  `crates/anvil-cli/src/commands/capsule.rs`.
- Reuses: `anvil-witness` (`WitnessLine`, `verify_chain_dag`), `anvil-baseline`
  (`Baseline`, cutoff), `anvil-rules` (`rules_sha`), `anvil-policy`
  (`ExceptionStore`), `anvil-config` (discovery), `anvil-sarif` SARIF emitter
  (ADR-058; relocated from `anvil-cli` by GITGOV-008), `anvil-kernel-types`
  (diagnostics).

## Work Items

### GITGOV-001: ADR — Git-native governance substrate
- **Intent:** State that Git is Anvil's durable trust substrate.
- **Expected Outcome:** [ADR-072](../decisions/072-git-native-governance-substrate.md) reviewed and Accepted.
- **Validation:** `pnpm adr:check`
- **Status:** Done 2026-06-08 (Accepted via full council review, accept-with-changes applied)

### GITGOV-002: ADR — state boundary
- **Intent:** Ratify `anvil/` durable vs `.anvil/` local; record reconciliations.
- **Expected Outcome:** [ADR-073](../decisions/073-durable-vs-local-anvil-state.md) reviewed and Accepted.
- **Validation:** `pnpm adr:check`
- **Status:** Done 2026-06-08 (Accepted via full council review, accept-with-changes applied)

### GITGOV-003: Capsule manifest schema
- **Intent:** Define `anvil.capsule.v1` manifest + `anvil.capsule-verification.v1`, digesting every file with SHA-256 over canonical JSON; carries `witness_seq_start`/`witness_seq_end` range pointers per the ADR-074 full-chain witness model.
- **Expected Outcome:** Versioned, round-trippable manifest types in `anvil-capsule`.
- **Validation:** `cargo test -p eddacraft-anvil-capsule`
- **Dependencies:** GITGOV-001
- **Status:** Merged 2026-06-08 via PR #2353

### GITGOV-004: Capsule create command
- **Intent:** `anvil capsule create --range <base>..<head> --out <dir>` writes the capsule directory.
- **Expected Outcome:** A capsule directory with manifest + collected evidence files is produced for a real range.
- **Validation:** `cargo test -p eddacraft-anvil capsule_create` (the CLI
  package is `eddacraft-anvil`; earlier text named a nonexistent
  `eddacraft-anvil-cli`)
- **Dependencies:** GITGOV-003, GITGOV-005, GITGOV-006
- **Status:** Merged 2026-06-08 via PR #2385. Also closed the GITGOV-006 review
  follow-up: `anvil_rules::OPA_RUNTIME_VERSION` is the shared constant (hook
  aliases it) and the CLI fills `ToolIdentity`/`Producer` from a single
  binding. Witness/diagnostics capsule entries are structural placeholders
  until GITGOV-007/-008 land their collectors.

### GITGOV-005: Commit/range collector
- **Intent:** Resolve a commit range to commits, tree hashes, parents, and changed paths.
- **Expected Outcome:** `commits.json` reflects the range deterministically.
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- collect_commits`
- **Dependencies:** GITGOV-003
- **Status:** Merged 2026-06-08 via PR #2378 (parallel PR #2377 closed as superseded)

### GITGOV-006: Policy/baseline/rules digest collector
- **Intent:** Capture effective policy digest, `rules_sha` (via `anvil_rules::rules_sha`), and baseline cutoff/digest (from `anvil/baseline.json`).
- **Expected Outcome:** `policy.json`/`rules.json`/`baseline.json` match the witnessed identity by construction.
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- collect_digests`
- **Dependencies:** GITGOV-003
- **Status:** Merged 2026-06-08 via PR #2379

### GITGOV-007: Witness collector
- **Intent:** Collect the **complete witness chain** — every rollover archive segment plus the active file, in walk order — verbatim into `witness.ndjson`; the manifest's `witness_seq_start`/`witness_seq_end` mark the PR-relevant range (ADR-074: `verify_chain_dag` is genesis-anchored with a gap-free `seq` walk and cannot verify a mid-chain subset).
- **Expected Outcome:** Capsule witness verification reuses `verify_chain_dag` with no re-modelled extract and no partial-chain special-casing.
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- collect_witness`
- **Dependencies:** GITGOV-005
- **Status:** Merged 2026-06-08 via PR #2390

### GITGOV-008: Diagnostics collector
- **Intent:** Include a SARIF 2.1.0 diagnostics summary via the ADR-058 shared emitter.
- **Expected Outcome:** `diagnostics.sarif` present (empty when none), no unified finding model introduced.
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- collect_diagnostics`
- **Dependencies:** GITGOV-004
- **Status:** In Progress

### GITGOV-009: Verification engine
- **Intent:** Verify manifest digests, witness chain, digests, and applied exception scope/expiry; emit `pass`/`warn`/`degraded`/`block`/`error` with the ADR-074 exit-code contract (`0` pass/warn, `1` block, `2` degraded, `3` error). v0 contract per ADR-074: verification requires the repository present; metadata-only detached verification is deferred to `v1`.
- **Expected Outcome:** `anvil capsule verify` returns honest verdicts; missing evidence is `degraded`, never `pass`.
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- verify`
- **Dependencies:** GITGOV-004, GITGOV-007, EXCEPT-005 (exception scope/expiry logic; capsule *collection* of exceptions is EXCEPT-009, which depends back on this engine — kept acyclic)
- **Status:** Proposed

### GITGOV-010: Capsule explain UX
- **Intent:** Human-readable `anvil capsule explain` (range, commits, policy/rules/baseline, witness coverage, diagnostics counts, exceptions, verdict).
- **Expected Outcome:** Golden-output tests for pass/warn/degraded/block.
- **Validation:** `cargo test -p eddacraft-anvil capsule_explain`
- **Dependencies:** GITGOV-009
- **Status:** Proposed

### GITGOV-011: JSON output
- **Intent:** `--json` on verify/inspect for CI consumption.
- **Expected Outcome:** Stable machine-readable verdict + manifest summary.
- **Validation:** `cargo test -p eddacraft-anvil capsule_json`
- **Dependencies:** GITGOV-009
- **Status:** Proposed

### GITGOV-012: Tamper tests
- **Intent:** Prove digest-mismatch and witness-break detection, missing-evidence → `degraded`, and the ADR-072 §3 scan-on-write line: a planted secret in any evidence file fails capsule creation.
- **Expected Outcome:** Mutating a capsule file fails verification; removing witness evidence degrades (never passes); secret-bearing evidence never reaches a tracked write.
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- tamper`
- **Dependencies:** GITGOV-009
- **Status:** Proposed

### GITGOV-013: Capsule retention + prune policy
- **Intent:** Decide retention for in-repo staged capsules (`anvil/evidence/capsules/`) and ship a prune surface before `v1`; document that the v0 default is on-demand/external (`--out` outside the repo) with in-repo staging explicitly opt-in and unpruned.
- **Expected Outcome:** Stated retention policy (ADR-074 amendment or sub-decision) plus `anvil capsule prune` or a documented manual path; indefinite accumulation is a stated choice, not an accident.
- **Validation:** `pnpm adr:check`
- **Dependencies:** GITGOV-004
- **Status:** Proposed

### GITGOV-014: State-boundary enforcement (ADR-073)
- **Intent:** Make the `anvil/` vs `.anvil/` boundary enforced, not asserted: (a) `anvil init`/`welcome` seed `.anvil/` wholesale into consumer `.gitignore` (today only `.anvil/cache/` + `.anvil/gates.json` — `crates/anvil-cli/src/commands/init.rs`); (b) a check warns when `.anvil/` paths are tracked or `anvil/` paths are ignored (`git check-ignore` sweep); (c) reconcile this repo's dogfood deviation — `anvil/witness/` + `anvil/kindling/` gitignored, and the bare `memory.json` ignore pattern would silently swallow a future `anvil/edda/memory.json` — by un-ignoring or recording the justification in ADR-072/073, and anchoring loose patterns (`/memory.json`).
- **Expected Outcome:** Consumer repos cannot accidentally commit runtime state or ignore durable evidence; the dogfood repo stops falsifying the ADR-072 premise.
- **Validation:** `cargo test -p eddacraft-anvil init_gitignore`
- **Dependencies:** GITGOV-002
- **Status:** Proposed
