# Review Capsules — Historical As-Built

| Type     | Authority  | Owner | Status     | Freshness                                                                                                   |
| -------- | ---------- | ----- | ---------- | ----------------------------------------------------------------------------------------------------------- |
| As-built | Historical | CAPS  | Deprecated | Migrated 2026-08-20 to `crates/anvil-capsule/README.md`; this pre-migration snapshot remains for provenance |

| Upstream                                                        | Downstream                                        |
| --------------------------------------------------------------- | ------------------------------------------------- |
| `crates/anvil-capsule/README.md`, ADR-072, ADR-073, and ADR-074 | historical inbound links and migration provenance |

Current format, lifecycle, invariants, failure behaviour, source links, and
local validation now live in
[`crates/anvil-capsule/README.md`](../../crates/anvil-capsule/README.md). The
public explanation remains
[`docs/public/anvil/concepts/review-capsules.md`](../public/anvil/concepts/review-capsules.md),
while ADR-072/-073/-074 preserve decision authority.

The content below is the pre-DOCRB-005 snapshot. It is not live component
authority; keep it for migration/source-link history and use Git history at this
path for earlier revisions.

## Historical snapshot

## Overview

A **review capsule** is a file-first, inspectable directory that packages the
governance evidence for one commit range — commits, policy/rules/baseline
digests, the witness chain, diagnostics, and applied exceptions — so a reviewer,
auditor, or downstream supplier can verify what Anvil observed locally, without
trusting Anvil Cloud or any service (`crates/anvil-capsule/src/lib.rs:1-12`).
Two frozen schemas define the format: `anvil.capsule.v1` for the manifest
(`crates/anvil-capsule/src/manifest.rs:18`) and `anvil.capsule-verification.v1`
for the recorded verdict (`crates/anvil-capsule/src/verification.rs:11`).
Capsules are created on-demand to an external `--out` directory by default; when
staged in-repo they live under `anvil/evidence/capsules/`, the durable side of
the ADR-073 state boundary (`crates/anvil-cli/src/commands/capsule.rs:985`,
[ADR-073](../../plans/decisions/073-durable-vs-local-anvil-state.md)). Retention
is keep-until-explicitly-pruned per
[ADR-078](../../plans/decisions/078-capsule-retention-and-prune.md) — nothing
auto-deletes.

## Architecture diagram

```text
┌─────────────────────┐
│ git repo + .anvil   │
│ state + witness     │
│ chain (NDJSON)      │
└──────────┬──────────┘
           │ collect (commits, digests, witness, diagnostics)
┌──────────▼──────────┐        ┌──────────────────────────┐
│ capsule create      │───────▶│ capsule directory        │
│ (scan-on-write gate)│        │ manifest.json + 10 files │
└─────────────────────┘        └──────────┬───────────────┘
                                          │
                  ┌───────────────────────┼───────────────────┐
                  │                       │                   │
           ┌──────▼───────┐        ┌──────▼──────┐     ┌──────▼──────┐
           │ verify       │        │ explain     │     │ prune       │
           │ (4 checks →  │        │ (read-only, │     │ (dry-run    │
           │  exit code)  │        │  repo-free) │     │  default)   │
           └──────────────┘        └─────────────┘     └─────────────┘
```

## Lifecycle / data flow

### Create

1. CLI entry — `run()` dispatches to `run_create`
   (`crates/anvil-cli/src/commands/capsule.rs:144-148`,
   `crates/anvil-cli/src/commands/capsule.rs:199`).
2. Range parse — `<base>..<head>` two-dot form only; `...`, whitespace, and
   empty endpoints are rejected
   (`crates/anvil-cli/src/commands/capsule.rs:965-982`). An `--out` inside
   `.git` is refused (`crates/anvil-cli/src/commands/capsule.rs:1167-1193`).
3. Identity fill — single site stamps the tool version (`CARGO_PKG_VERSION`) and
   OPA runtime version (`crates/anvil-cli/src/commands/capsule.rs:204-213`).
4. Commit collection — endpoints resolved to full SHAs, then
   `git rev-list --topo-order --reverse` walks the range; each commit records
   tree, parents, and a first-parent diff summary
   (`crates/anvil-capsule/src/collect.rs:120-166`,
   `crates/anvil-capsule/src/collect.rs:199-243`). Shallow clones are refused
   (`crates/anvil-capsule/src/collect.rs:125-132`).
5. Digest collection — policy file (candidate order at
   `crates/anvil-capsule/src/collect_digests.rs:291-303`), `.anvil.*` config via
   `anvil_config::discover`
   (`crates/anvil-capsule/src/collect_digests.rs:309-320`), rules via
   `anvil_rules::rules_sha`
   (`crates/anvil-capsule/src/collect_digests.rs:237-248`), baseline via
   `anvil_baseline::load`
   (`crates/anvil-capsule/src/collect_digests.rs:345-359`).
6. Witness collection — the **complete** chain (every rollover segment plus the
   active file) is copied verbatim into `witness.ndjson`; the range-relevant
   window is recorded as `witness_seq_start` / `witness_seq_end` pointers
   derived from per-line `seq` / `commit_sha`
   (`crates/anvil-capsule/src/collect_witness.rs:84-143`,
   `crates/anvil-capsule/src/collect_witness.rs:123-131`).
7. Diagnostics — a SARIF 2.1.0 document is rendered via the shared `anvil-sarif`
   emitter; v0 passes an empty finding slice, producing a complete empty SARIF
   file (`crates/anvil-capsule/src/collect_diagnostics.rs:63-112`,
   `crates/anvil-cli/src/commands/capsule.rs:227`).
8. Write — `write_capsule` builds the manifest with the range and witness
   window, then writes the ten required files including placeholders
   (`exceptions.json` = `[]`, `edda-context.json` = `{}`) and a degraded
   placeholder `verification.json` (`crates/anvil-capsule/src/format.rs:73-135`,
   `crates/anvil-capsule/src/manifest.rs:30-41`).
9. Scan-on-write gate — all evidence text is scanned for secret-shaped content
   **before any filesystem write**; a hit fails creation outright
   (`crates/anvil-capsule/src/format.rs:112`,
   `crates/anvil-capsule/src/format.rs:180-207`). The out directory must be
   empty and not a symlink; files are created exclusively; the manifest is
   written last (`crates/anvil-capsule/src/format.rs:220-243`,
   `crates/anvil-capsule/src/format.rs:255-270`,
   `crates/anvil-capsule/src/format.rs:131-132`).

### Verify

Engine entry is `verify_capsule` / `verify_capsule_at`
(`crates/anvil-capsule/src/verify.rs:55-90`); the CLI persists the resulting
`verification.json`, re-records its manifest digest, and exits with the
verdict's exit code (`crates/anvil-cli/src/commands/capsule.rs:159-181`,
`crates/anvil-cli/src/commands/capsule.rs:261-281`). Four checks combine
worst-of:

1. `manifest-digests` — every recorded file present with a matching SHA-256;
   tamper = `block`, missing = `degraded`, foreign file = `warn`, traversal path
   = `block` (`crates/anvil-capsule/src/verify.rs:113-183`).
2. `witness-chain` — reuses `anvil_witness::verify_chain_dag` over the embedded
   `witness.ndjson`; absent or empty = `degraded`, chain break = `block`
   (`crates/anvil-capsule/src/verify.rs:186-235`).
3. `digests-vs-repo` — re-collects commits and policy/rules/baseline digests
   from the live repository and compares; divergence or inability to re-collect
   = `degraded`, never `block` (`crates/anvil-capsule/src/verify.rs:250-306`).
4. `exceptions` — reuses `anvil_policy::exceptions::verify_exception_at` for
   scope/expiry; expired, revoked, or invalid = `block`, unattributed =
   `degraded` (`crates/anvil-capsule/src/verify.rs:311-375`).

An unreadable manifest produces a single `error` check rather than any overclaim
(`crates/anvil-capsule/src/verify.rs:73-82`). The verdict vocabulary is `pass` /
`warn` / `degraded` / `block` / `error`
(`crates/anvil-capsule/src/verification.rs:19-33`) with the
[ADR-074](../../plans/decisions/074-review-capsule-v0-format.md) exit-code
contract — `0` pass/warn, `1` block, `2` degraded, `3` error
(`crates/anvil-capsule/src/verification.rs:39-47`). Verification requires the
repository present; metadata-only detached verification is deferred to v1
(ADR-074 "Deferred").

### Prune

Retention is keep-until-explicitly-pruned
([ADR-078](../../plans/decisions/078-capsule-retention-and-prune.md)).
`plan_prune` orders schema-gated capsule directories by head **committer date**
(the frozen manifest carries no creation timestamp) and keeps the newest N
(`crates/anvil-capsule/src/prune.rs:167-177`; date resolution at
`crates/anvil-capsule/src/prune.rs:230-242`); `apply_prune` removes via `git rm`
for tracked capsules and filesystem removal for untracked ones — staged
disposal, never a silent delete (`crates/anvil-capsule/src/prune.rs:185-196`,
`crates/anvil-capsule/src/prune.rs:200-225`). The CLI defaults to dry-run,
printing the would-delete list on stdout
(`crates/anvil-cli/src/commands/capsule.rs:990-1068`); `--keep-last 0` is
refused at both the clap layer and the planner
(`crates/anvil-cli/src/commands/capsule.rs:136-137`,
`crates/anvil-capsule/src/prune.rs:85-91`), and the prune root must be in-repo
and outside `.git` (`crates/anvil-cli/src/commands/capsule.rs:1097-1141`).

## Surfaces

Subcommands are declared on `CapsuleCommand`
(`crates/anvil-cli/src/commands/capsule.rs:58-85`). There is no MCP tool and no
hook integration for capsules; `explain` is the only repo-independent verb
(`crates/anvil-cli/src/commands/capsule.rs:182-194`).

| Surface                 | Kind | Stability | Notes                                                                                                                                            |
| ----------------------- | ---- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `anvil capsule create`  | CLI  | beta      | `--range <base>..<head>`, `--out <dir>` (`crates/anvil-cli/src/commands/capsule.rs:112-124`)                                                     |
| `anvil capsule verify`  | CLI  | beta      | positional dir, `--json` emits `anvil.capsule-verification.v1`; exit code carries the verdict (`crates/anvil-cli/src/commands/capsule.rs:87-98`) |
| `anvil capsule explain` | CLI  | beta      | positional dir, `--json` emits `anvil.capsule-explain.v1`; works without a repository (`crates/anvil-cli/src/commands/capsule.rs:100-110`)       |
| `anvil capsule prune`   | CLI  | beta      | `--root` (optional), `--keep-last N` (required, ≥1), `--apply`; dry-run by default (`crates/anvil-cli/src/commands/capsule.rs:126-142`)          |

## Internals

### Invariant: canonical JSON digesting

Digests are SHA-256 over canonical JSON — recursively sorted object keys, array
order preserved, minimal whitespace, no trailing newline
(`crates/anvil-capsule/src/canonical.rs:19-54`,
`crates/anvil-capsule/src/canonical.rs:59-61`). The manifest records the exact
written bytes, so even a whitespace difference is a digest mismatch
(`crates/anvil-capsule/src/manifest.rs:116-125`).

### Invariant: present-but-empty discipline

Every required file is written, never omitted — a missing file at verify time
means tamper, not "no findings" (`crates/anvil-capsule/src/format.rs:1-9`,
`crates/anvil-capsule/src/manifest.rs:20-41`).

### Invariant: closed schemas, no verdict laundering

All capsule documents parse with `deny_unknown_fields` behind a schema-gate
probe (`crates/anvil-capsule/src/lib.rs:42-60`). A stored verification
document's verdict is re-derived from its checks on parse; a mismatch is
rejected as `InconsistentVerdict`
(`crates/anvil-capsule/src/verification.rs:174-188`), and an empty check list is
`degraded`, never `pass` (`crates/anvil-capsule/src/verification.rs:136-142`).
The worst-of ranking places `error` above `block`
(`crates/anvil-capsule/src/verification.rs:64-85`).

### Invariant: full witness chain, range pointers

The chain ships whole because `verify_chain_dag` is genesis-anchored with a
gap-free `seq` walk and cannot verify a mid-chain extract
(`crates/anvil-capsule/src/collect_witness.rs:1-13`,
`crates/anvil-witness/src/verify.rs:159-198`). The manifest's seq window is a
min/max span over the range, not an exclusive ownership claim
(`crates/anvil-capsule/src/manifest.rs:77-92`).

### Invariant: evidence-collection git integrity

All collection subprocesses run with `--no-replace-objects` and a scrubbed
environment (`GIT_DIR`, `GIT_INDEX_FILE`, …) so collection cannot be steered to
another repository or fed a replacement object
(`crates/anvil-capsule/src/collect.rs:252-289`).

### Invariant: deterministic output

Capsule creation is byte-identical for identical content — no timestamps in any
written file (`crates/anvil-capsule/src/format.rs:272-273`, pinned by
`crates/anvil-capsule/src/format.rs:496-510`).

Exception verification is deliberately acyclic: the verify engine reuses
EXCEPT-005's scope/expiry logic, while capsule _collection_ of applied
exceptions (EXCEPT-009) depends back on this engine
(`plans/archive/modules/git-native-governance.aps.md`). Today `exceptions.json`
is an inert `[]` placeholder (`crates/anvil-capsule/src/format.rs:102`).

## Known gaps

### G-01: diagnostics input not wired

Every v0 capsule carries a complete-but-empty SARIF document; the adapter is
implemented and tested but no finding source feeds it yet
(`crates/anvil-capsule/src/collect_diagnostics.rs:18-23`). **Risk:** Low.
**Fix:** GITGOV-009 follow-on wiring when a diagnostics source is selected.

### G-02: exceptions and Edda context are placeholders

`exceptions.json` = `[]` and `edda-context.json` = `{}` until EXCEPT-009 and the
Edda seal land; when real free-text evidence arrives, the scan-on-write gate
must re-enable entropy scanning (`crates/anvil-capsule/src/format.rs:171-179`).
**Risk:** Medium — the gate's coverage limitation is documented in code.
**Fix:** tracked with EXCEPT-009.

### G-03: metadata-only detached verification deferred

Verification requires the repository present; ADR-074 defers detached
(metadata-only) verification and a v1 subchain witness verifier, along with
git-bundle packing, `refs/anvil/*` namespaces, and cryptographic signing beyond
content hashes ([ADR-074](../../plans/decisions/074-review-capsule-v0-format.md)
"Deferred"). **Risk:** Low — by design for v0. **Fix:** v1 scope.

### G-04: `inspect` subcommand not shipped

Only `create` / `verify` / `explain` / `prune` ship; the richer `inspect`
surface mentioned in the public concepts page is a GITGOV-010/-011 follow-up
(`crates/anvil-cli/src/commands/capsule.rs:24`,
`docs/public/anvil/concepts/review-capsules.md`). **Risk:** Low. **Fix:**
unscheduled.

### G-05: per-commit collection cost

`collect_commits` spawns two git subprocesses per commit; a batch path is noted
in code but not implemented (`crates/anvil-capsule/src/collect.rs:149-154`).
**Risk:** Low for typical PR ranges. **Fix:** unscheduled performance follow-up.

### G-06: prune absent from the CLI-surface runbook — resolved

Resolved 2026-06-11: `docs/runbooks/cli-surface.md` now documents
`capsule create/verify/explain/prune`, including the `prune` retention flags and
dry-run-by-default behaviour added via GITGOV-013. **Risk:** Closed.

## Source references

- `crates/anvil-capsule/src/lib.rs` — crate root, public re-exports,
  `schema_gate` probe
- `crates/anvil-capsule/src/manifest.rs` — `anvil.capsule.v1` manifest,
  required-file list, range + witness seq window
- `crates/anvil-capsule/src/verification.rs` — verification document, verdict
  vocabulary, exit-code contract, worst-of combination
- `crates/anvil-capsule/src/canonical.rs` — canonical JSON + SHA-256
- `crates/anvil-capsule/src/collect.rs` — commit/range collector and hardened
  `git` subprocess helper
- `crates/anvil-capsule/src/collect_digests.rs` — policy / rules / baseline
  digest collector
- `crates/anvil-capsule/src/collect_witness.rs` — verbatim full-chain witness
  collector + seq window
- `crates/anvil-capsule/src/collect_diagnostics.rs` — SARIF 2.1.0 adapter
- `crates/anvil-capsule/src/format.rs` — capsule directory writer +
  scan-on-write secret gate
- `crates/anvil-capsule/src/verify.rs` — four-check verification engine
- `crates/anvil-capsule/src/prune.rs` — retention planner / staged disposal
- `crates/anvil-capsule/src/errors.rs` — error variants
- `crates/anvil-capsule/Cargo.toml` — dependency surface (`anvil-witness`,
  `anvil-checks`, `anvil-config`, `anvil-rules`, `anvil-baseline`,
  `anvil-sarif`, `anvil-kernel-types`, `anvil-policy`)
- `crates/anvil-cli/src/commands/capsule.rs` — CLI lane: subcommand wiring,
  `--json` renderers, range parsing, root guards (55 inline tests)

Test evidence is inline `#[cfg(test)]` — there is no separate `tests/`
directory. Tamper coverage (GITGOV-012): secret-in-evidence fails creation
before any write (`crates/anvil-capsule/src/format.rs:579-606`), digest tamper →
`block` (`crates/anvil-capsule/src/verify.rs:551-565`), witness break → `block`
(`crates/anvil-capsule/src/verify.rs:802-840`,
`crates/anvil-capsule/src/verify.rs:869-912`), path traversal → `block`
(`crates/anvil-capsule/src/verify.rs:755-776`).

## Related docs

- ADRs: [ADR-072](../../plans/decisions/072-git-native-governance-substrate.md)
  (substrate, no-secrets-in-evidence),
  [ADR-073](../../plans/decisions/073-durable-vs-local-anvil-state.md) (durable
  vs local state boundary),
  [ADR-074](../../plans/decisions/074-review-capsule-v0-format.md) (v0 format,
  verdict + exit-code contract, deferred scope),
  [ADR-078](../../plans/decisions/078-capsule-retention-and-prune.md)
  (retention + prune)
- Module plan:
  [git-native-governance.aps.md](../../plans/archive/modules/git-native-governance.aps.md)
  (GITGOV-001..014, all terminal)
- Runbook: [cli-surface.md](../runbooks/cli-surface.md) (`anvil capsule` rows)
- Public docs: [review-capsules.md](../public/anvil/concepts/review-capsules.md)
  (concepts page)
- Template: [\_as-built-template.md](_as-built-template.md)
