# ADR-074: Review Capsule v0 Format

## Status

**Accepted** — 2026-06-08, full council review (accept-with-changes; the
required changes — full-chain witness model, repo-present v0 verification
contract, exit-code table, ADR-002 cross-reference — are applied in the
accepting commit)

## Date

2026-06-06

## Context

The GITGOV wedge (`plans/modules/git-native-governance.aps.md`) packages
change-level governance into a portable artefact — a **review capsule** — that a
reviewer, auditor, or supplier can verify locally without trusting Anvil Cloud
(ADR-072). Before writing the collector/verifier, the capsule's on-disk format
and verdict model must be fixed, because everything downstream (CLI output,
tamper tests, future refs/notes packing) depends on it.

Two design risks motivate this ADR:

1. **Schema fiction.** The brainstorm's `solution.md` sketches a `WitnessExtract`
   with `L0/L2/L3/L4` sub-objects and `agent.task_id/step_id`. The **real**
   `anvil-witness::WitnessLine` has none of that — it carries `seq`, `scope`,
   `kind`, `prev_line_hash`, `project_uuid`, `commit_sha`, `parent_commits`,
   `prev_line_hashes`, `agent_tag`, `rules_sha`, `cutoff_commit`, `ts`,
   `validation_at`. A frozen `anvil.capsule.v1` must embed the *real* shapes.
2. **Overclaiming.** A verifier that returns `pass` when evidence is merely
   absent would violate Anvil's tooling-honesty doctrine.

## Decision

**Capsule v0 is a file-first, inspectable directory with a digest-protected
manifest. Git bundles and refs/notes are deferred.**

### Layout

```text
review.anvil-capsule/
├── manifest.json        # anvil.capsule.v1 — file digests + range + producer
├── commits.json         # commit SHAs, tree hashes, parents, changed paths
├── policy.json          # anvil.policy-digest.v1
├── baseline.json        # baseline cutoff + digest (from anvil/baseline.json)
├── rules.json           # anvil.rules-digest.v1 (rules_sha as witness uses)
├── witness.ndjson       # verbatim WitnessLine records — full chain (see Schema rules)
├── diagnostics.sarif    # SARIF 2.1.0 subset (reuses ADR-058 emitter)
├── exceptions.json      # applied ExceptionRecords (anvil.exception.v1, EXCEPT)
├── edda-context.json    # references only; populated only when --include-edda
├── verification.json    # anvil.capsule-verification.v1 — initial verdict
└── README.md            # human-readable summary
```

### Schema rules

- The manifest is schema-versioned (`"schema": "anvil.capsule.v1"`) and lists a
  SHA-256 digest for every other file. Digests are over **canonical JSON**
  (sorted keys, minimal whitespace) — the same discipline `WitnessLine` already
  uses — so digests are reproducible across machines.
- `witness.ndjson` embeds **verbatim** `anvil-witness::WitnessLine` records (not
  a re-modelled extract), so capsule witness verification reuses
  `anvil-witness::verify_chain_dag` rather than a parallel parser.
- **Full chain, not a range subset.** `verify_chain_dag` anchors at a genesis
  token (`GENESIS-FRESH`/`GENESIS-BASELINED`) and requires a gap-free `seq`
  walk (`crates/anvil-witness/src/verify.rs` — a mid-chain first line fails
  with `UnknownGenesis`/`SequenceGap`), so a capsule cannot embed only the
  range's lines and still reuse the shipped verifier. v0 therefore embeds the
  **complete chain** — every rollover archive segment plus the active file,
  concatenated in walk order — and the manifest records the PR-relevant range
  as `witness_seq_start`/`witness_seq_end` pointers into it. Witness lines are
  compact NDJSON; if full-chain size becomes prohibitive for long-lived repos,
  that is the trigger to build the deferred v1 subchain verifier (accepting a
  trusted anchor hash in place of a genesis token), never to silently subset.
- `rules.json`'s `rules_sha` is computed by `anvil_rules::rules_sha` — the exact
  value witnessed on the line — so the capsule's rule identity matches the
  witness chain by construction.
- SARIF output reuses the ADR-058 shared emitter; the capsule introduces **no**
  unified in-process finding model.
- `diagnostics`/`exceptions`/`edda-context` are present-but-empty rather than
  omitted when there is nothing to report, so a missing file is unambiguously a
  tamper/corruption signal, not "no findings".

### Verdict model (`anvil.capsule-verification.v1`)

Closed-state verdicts, with missing evidence never passing:

| Verdict | Meaning |
|---------|---------|
| `pass` | All required evidence present and verified; no block-level finding |
| `warn` | Verified, with non-blocking findings |
| `degraded` | Evidence missing, stale, or partially unverifiable — **not** `pass` |
| `block` | Witness break, digest mismatch, invalid/expired exception, or policy violation |
| `error` | Tool/internal failure — do not overclaim |

`degraded != pass`, `error != pass`, `missing evidence != clean evidence`.
**`block` in this table is a verification-CLI verdict, never a save-time gate.**
Per ADR-002, the capsule verdict is **advisory evidence**, not a new blocking
gate on user code; it is the closeout/verification surface, akin to the
ADR-042 carve-out class, and exits non-zero only as a verification CLI, never as
a save-time block.

`anvil capsule verify` exit codes (the CI contract — GITGOV-009/011):

| Exit | Verdict |
|------|---------|
| `0` | `pass`, `warn` (warnings over blocks, ADR-002) |
| `1` | `block` |
| `2` | `degraded` |
| `3` | `error` |

### Deferred (explicitly not v0)

Git bundles/`.anvil-bundle` packing; refs/notes namespaces
(`refs/anvil/*`, `refs/notes/anvil-*`); cryptographic signing beyond Git/content
hashes; metadata-only detached verification — **resolved for v0: verification
requires the repository to be present** (digest and witness checks run against
the repo the capsule describes; GITGOV-009 pins this contract), and verifying
from nothing but the capsule's own `commits.json` metadata is deferred to `v1`;
the v1 subchain witness verifier (see Schema rules); Graph-V2 behavioural diff;
`--include-sessions`.

## Rationale

A directory + manifest is trivially inspectable, reviewable in a PR, and cheap
to implement on top of existing crates — the right shape to prove the packaging
loop before adding Git-plumbing complexity. Embedding real producer schemas
(verbatim witness lines, `rules_sha`) means verification *reuses* shipped,
tested code paths instead of a second, drift-prone model.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| File-first directory + manifest digests (chosen) | Inspectable, PR-reviewable, reuses witness/baseline/rules/SARIF; minimal plumbing | Working-tree noise if staged in-repo (mitigated: on-demand, external by default) |
| Git bundle from day one | Single portable object; native Git verification | More plumbing; opaque to casual inspection; premature before the loop is proven |
| refs/notes-attached evidence | No working-tree files; attaches to commits | Invisible to normal developers; fetch/push refspec education; complexity before value |
| Tarball/custom extension wrapping the dir | One file to move | Loses direct inspectability; packing is a trivial later add over the dir format |

## Consequences

- **Positive:** Fast path to a demonstrable `create`/`verify`/`explain` loop;
  verification reuses `verify_chain_dag` and the SARIF emitter; honest verdicts
  by construction.
- **Positive:** A later bundle/refs/notes format can pack this directory without
  re-modelling evidence.
- **Negative:** Capsule directories are bulkier than a single bundle; in-repo
  staging adds tree noise (kept on-demand/external by default).
- **Risks:** Freezing `anvil.capsule.v1` before reconciling every sub-schema
  against its producing crate; ambiguity between "missing file" and "no
  findings".
- **Mitigations:** Embed verbatim producer schemas; present-but-empty evidence
  files; schema version gate in the manifest; tamper + missing-evidence tests
  are GITGOV acceptance criteria (GITGOV-012).

## References

- Related ADRs: ADR-002 (warnings over blocks), ADR-037 (witness / `WitnessLine`
  + `verify_chain_dag`), ADR-039 (baseline), ADR-042 (closeout-enforcement exit
  codes carve-out), ADR-058 (shared SARIF emitter, no unified finding model),
  ADR-072 (Git substrate), ADR-073 (state boundary)
- APS module: GITGOV (`plans/modules/git-native-governance.aps.md`)
- Code anchors: `crates/anvil-witness/src/line.rs`,
  `crates/anvil-baseline/src/lib.rs`, `crates/anvil-rules` (`rules_sha`),
  `crates/anvil-cli/src/output/sarif.rs`
