# Git-Native Governance Substrate

| ID | Owner | Status |
|----|-------|--------|
| GITGOV | @josh | Proposed |

**Last reviewed:** 2026-06-06

> **Decision-gated.** Execution of the capsule implementation items
> (GITGOV-003+) is authorised once [ADR-072](../decisions/072-git-native-governance-substrate.md)
> (Git substrate) and [ADR-074](../decisions/074-review-capsule-v0-format.md)
> (capsule v0 format) are Accepted via council. The schema/decision items are
> ready to review now. Brainstorm:
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
  (`ExceptionStore`), `anvil-config` (discovery), `anvil-cli` SARIF emitter
  (ADR-058), `anvil-kernel-types` (diagnostics).

## Work Items

### GITGOV-001: ADR — Git-native governance substrate
- **Intent:** State that Git is Anvil's durable trust substrate.
- **Expected Outcome:** [ADR-072](../decisions/072-git-native-governance-substrate.md) reviewed and Accepted.
- **Validation:** `pnpm adr:check`
- **Status:** Proposed

### GITGOV-002: ADR — state boundary
- **Intent:** Ratify `anvil/` durable vs `.anvil/` local; record reconciliations.
- **Expected Outcome:** [ADR-073](../decisions/073-durable-vs-local-anvil-state.md) reviewed and Accepted.
- **Validation:** `pnpm adr:check`
- **Status:** Proposed

### GITGOV-003: Capsule manifest schema
- **Intent:** Define `anvil.capsule.v1` manifest + `anvil.capsule-verification.v1`, digesting every file with SHA-256 over canonical JSON.
- **Expected Outcome:** Versioned, round-trippable manifest types in `anvil-capsule`.
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- manifest`
- **Dependencies:** GITGOV-001
- **Status:** Proposed

### GITGOV-004: Capsule create command
- **Intent:** `anvil capsule create --range <base>..<head> --out <dir>` writes the capsule directory.
- **Expected Outcome:** A capsule directory with manifest + collected evidence files is produced for a real range.
- **Validation:** `cargo test -p eddacraft-anvil-cli capsule_create`
- **Dependencies:** GITGOV-003, GITGOV-005, GITGOV-006
- **Status:** Proposed

### GITGOV-005: Commit/range collector
- **Intent:** Resolve a commit range to commits, tree hashes, parents, and changed paths.
- **Expected Outcome:** `commits.json` reflects the range deterministically.
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- collect_commits`
- **Dependencies:** GITGOV-003
- **Status:** Proposed

### GITGOV-006: Policy/baseline/rules digest collector
- **Intent:** Capture effective policy digest, `rules_sha` (via `anvil_rules::rules_sha`), and baseline cutoff/digest (from `anvil/baseline.json`).
- **Expected Outcome:** `policy.json`/`rules.json`/`baseline.json` match the witnessed identity by construction.
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- collect_digests`
- **Dependencies:** GITGOV-003
- **Status:** Proposed

### GITGOV-007: Witness collector
- **Intent:** Collect **verbatim** `anvil-witness::WitnessLine` records covering the range into `witness.ndjson`.
- **Expected Outcome:** Capsule witness verification reuses `verify_chain_dag` with no re-modelled extract.
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- collect_witness`
- **Dependencies:** GITGOV-005
- **Status:** Proposed

### GITGOV-008: Diagnostics collector
- **Intent:** Include a SARIF 2.1.0 diagnostics summary via the ADR-058 shared emitter.
- **Expected Outcome:** `diagnostics.sarif` present (empty when none), no unified finding model introduced.
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- collect_diagnostics`
- **Dependencies:** GITGOV-004
- **Status:** Proposed

### GITGOV-009: Verification engine
- **Intent:** Verify manifest digests, witness chain, digests, and applied exception scope/expiry; emit `pass`/`warn`/`degraded`/`block`/`error`. Resolve the detached-verification question (repo-present vs metadata-only) before `v1` freeze.
- **Expected Outcome:** `anvil capsule verify` returns honest verdicts; missing evidence is `degraded`, never `pass`.
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- verify`
- **Dependencies:** GITGOV-004, GITGOV-007, EXCEPT-009
- **Status:** Proposed

### GITGOV-010: Capsule explain UX
- **Intent:** Human-readable `anvil capsule explain` (range, commits, policy/rules/baseline, witness coverage, diagnostics counts, exceptions, verdict).
- **Expected Outcome:** Golden-output tests for pass/warn/degraded/block.
- **Validation:** `cargo test -p eddacraft-anvil-cli capsule_explain`
- **Dependencies:** GITGOV-009
- **Status:** Proposed

### GITGOV-011: JSON output
- **Intent:** `--json` on verify/inspect for CI consumption.
- **Expected Outcome:** Stable machine-readable verdict + manifest summary.
- **Validation:** `cargo test -p eddacraft-anvil-cli capsule_json`
- **Dependencies:** GITGOV-009
- **Status:** Proposed

### GITGOV-012: Tamper tests
- **Intent:** Prove digest-mismatch and witness-break detection, and missing-evidence → `degraded`.
- **Expected Outcome:** Mutating a capsule file fails verification; removing witness evidence degrades (never passes).
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- tamper`
- **Dependencies:** GITGOV-009
- **Status:** Proposed
