# Graph v2 Persisted Provenance/Session Fields — Privacy Review Council Verdict

**Date:** 2026-06-08
**Session:** `gv2-privacy-20260608` (formal privacy-review council)
**Panel:** security-analyst (lead), kernel-maintainer, adversarial-reviewer,
operations-reviewer
**Artifact under review:** the persistable identity field shape frozen by
GV2-002/GV2-010, reviewed against
[ADR-069](../decisions/069-graph-v2-persistence.md) §8,
[`graph-v2-foundation-spec.md`](../../docs/architecture/graph-v2-foundation-spec.md)
(cross-graph identity + C-6),
[`intercept-as-built.md`](../../docs/architecture/intercept-as-built.md) §10, the
[2026-06-05 wave verdict](2026-06-05-gv2-wave-planning-council-verdict.md) (M6),
and the live types in `crates/anvil-kernel-types` / `crates/anvil-graph-cache` /
`crates/anvil-intercept-proto`.
**Gate:** the open GV2 Ready-checklist item — *"Privacy review completed for
persisted provenance/session fields"* (hard blocker for GV2-002 and GV2-010;
output feeds the GV2-030 allowlist).

---

## Verdict

**APPROVE-WITH-CONDITIONS — unanimous (4/4), no BLOCKs.** ADR-069 §8's privacy
line for semantic structural identity is sound, operationally correct, and the
live type set is clean (no `Vec<u8>`, no `serde(flatten)`, no source bodies).
The load-bearing framing decision, which every panel member independently
converged on:

> **The v1 `SnapshotPayload` is a semantic + dependency snapshot only. No
> session, worktree, attribution, or plan/provenance field persists in v1.**
> GV2-002 may *define* identity for all five key rows of the spine spec's
> identity table, but only the semantic/dependency subset is *persistable*
> until GV2-013 and GV2-014 land their own per-graph privacy ADRs (spec
> condition C-6 already requires these).

That framing makes the gate honestly satisfiable now: every session/provenance
exposure is resolved by exclusion-from-DTO, and the new identity fields GV2-002
introduces are constrained by the conditions below. The gate flips once
PV-1..PV-5 are folded into GV2-002's item text and PV-6..PV-12 into
GV2-030's (this verdict's companion module edit does both).

| Dim | Topic | Lead | Verdict |
|-----|-------|------|---------|
| P1 | Session/worktree identity persistence | security-analyst | APPROVE — excluded from v1 DTO |
| P2 | Plan/provenance anchors (APS ids, SHAs, Edda refs) | security-analyst | APPROVE — DEFER to GV2-014 ADR, ref-only |
| P3 | GV2-002 new identity fields (disambiguator, hashes, rename) | kernel-maintainer | APPROVE-WITH-CONDITIONS (PV-1/2/4/5) |
| P4 | Leak scenarios (filename, export, graveyard, telemetry) | adversarial-reviewer | APPROVE-WITH-CONDITIONS (PV-8/9/10) |
| P5 | Operational boundary (perms, GC, backup/sync, flags) | operations-reviewer | APPROVE-WITH-CONDITIONS (PV-11/12) |

---

## Per-field-class verdict table (the GV2-030 allowlist input)

Verdicts: **ALLOW** (cleartext) · **ALLOW-T** (allowed transformed — transform
named) · **DENY** (never persisted) · **DEFER** (not in the v1 DTO; persistable
form decided by a later per-graph ADR).

| # | Field class | Exists today (evidence) | v1 snapshot | Future persistence |
|---|---|---|---|---|
| a1 | `SessionId` | unvalidated free-form `String` (`anvil-intercept-proto/src/lib.rs:37-44`) | **DEFER** | ref-only; daemon-minted format-constrained id or keyed hash — GV2-013 ADR |
| a2 | Worktree path / `WorktreeKey` | absolute canonicalised `PathBuf` (`anvil-intercept-proto/src/lib.rs:287`; `anvil-intercept/src/rule_cache.rs:99`) | **ALLOW-T** — pinned key-hash in the snapshot *filename* only; **DENY** any `PathBuf`/absolute path inside the payload | same; the G-05 shared root-relativisation type is the only worktree→file bridge |
| b1 | APS work-item ids | repo-public identifiers | **DEFER** | **ALLOW** as refs — GV2-014 ADR |
| b2 | Commit SHAs | already in git | **DEFER** | **ALLOW** as refs — GV2-014 ADR |
| b3 | Edda memory refs | TS-side, git-committed store | **DEFER** | **ALLOW-T** — opaque ref tokens only, never inline bodies (C-6) |
| c1 | `driver_id` | registry-controlled vocabulary, never user-supplied (`anvil-intercept-proto/src/session.rs:55-58`) | **DEFER** | **ALLOW** cleartext ref |
| c2 | `claimed_agent_id`, usernames, hostnames | free-form (`session.rs:60-63`); username reachable via absolute paths | **DENY** cleartext | **ALLOW-T** — keyed HMAC with domain separator (MLP2-071 primitive, `anvil-intercept/src/fanout.rs:402-412`), never unsalted SHA-256 — GV2-013 ADR |
| c3 | `pid` / `pgid` / liveness fields | `anvil-intercept-proto/src/lib.rs:288-298` | **DENY** | **DENY** — restart-meaningless liveness mechanics; behavioural metadata |
| c4 | Session timestamps | `lib.rs:293-298` | **DEFER** — telemetry-only | provenance timestamps ride the GV2-014 ADR (inherent to git anyway) |
| d1 | Symbol names / qualified names | `SymbolNode.name` (`anvil-kernel-types/src/graph.rs:39`) | **ALLOW** — extends ADR-069 §8 acceptance; same named cleartext residual | same |
| d2 | Overload disambiguator | does not exist yet (GV2-002 introduces) | **ALLOW-T** — structural signature only (PV-1) | same |
| d3 | Rename-tracking metadata | does not exist yet (GV2-002 introduces) | **DEFER** — in-memory only (PV-4) | own ADR required |
| e1 | File-content hashes | unsalted SHA-256 (`anvil-intercept/src/validate_paths.rs:198-200`) | **ALLOW** — correlation residual named (PV-12), same class as git blob hashes, bounded by the 0700/0600 same-uid boundary | re-evaluate before any cross-machine surface |
| e2 | New GV2-002 hash/id fields | do not exist yet | **ALLOW-T** — input domain declared (PV-5), deterministic algorithm named (PV-2) | same |
| f1 | `GraphDelta` (all fields, incl. `errors: Vec<String>`, baseline `HashSet`s) | transient per-update state (`anvil-graph-cache/src/incremental.rs:14-25`) | **DENY** — entirely absent from the DTO; reconstructed at warm-start reconcile (PV-7) | n/a — join-time state by construction |

---

## Conditions

### On GV2-002 item text (fold before implementation starts)

- **PV-1 — Structural overload disambiguator.** The disambiguator derives from
  structural identity only — symbol kind, arity, parameter *type-identity*
  names/refs or a fixed-width hash thereof, and/or source-offset ordinal among
  same-`(file, kind, name)` symbols. **Never** parameter source text,
  default-value expressions (`foo(key = "sk-live-…")` is a literal-value leak,
  ADR-069 §8 DENY class), function-body or comment text. Represent as a sealed
  struct/enum, not a free-form `String` derived from a source span.
  _(security F1, kernel C-3, adversarial P-4)_
- **PV-2 — Deterministic, named hash algorithm.** The stable-id hash must be a
  named deterministic content hash (SHA-256, Blake3, or fixed-seed FxHash).
  `std::hash::Hash` over Rust's default randomly-seeded hasher (SipHash) is
  **not stable across restarts** and silently defeats the entire item.
  _(kernel C-1)_
- **PV-3 — Session/provenance identity is join-time-only.** The identity
  contract's "session/worktree identity, and APS/provenance references" rows
  mean *resolvable at join time from their graph authorities*
  (control/session graph, plan/provenance graph) — they are **absent from
  `SnapshotPayload`**. No denormalised session id, worktree key, or APS ref may
  appear in any persisted graph artefact in v1. _(security F5, kernel C-2)_
- **PV-4 — No persisted rename history in v1.** Rename = delete-old-id +
  create-new-id; old names are implicit in the id change and are **not
  retained** in any persisted form. Persisted rename chains would be
  authoritative (non-derivable) state requiring its own ADR, and would extend
  the retention window of a secret-shaped name beyond
  "rebuilt-from-current-source". Rename continuity lives in resident memory
  for the session. _(security F2, adversarial S-5)_
- **PV-5 — Hash input domains declared.** Every new hash field declares its
  input domain in the identity contract; inputs are restricted to ALLOW-class
  data or whole-file content. No truncated/low-entropy hashes over literal
  values (brute-force-invertible). _(security F6)_

### On GV2-030 item text (fold now; verify at implementation)

- **PV-6 — v1 DTO scope pinned.** `SnapshotPayload` covers the
  semantic + dependency graphs only; zero session/attribution/provenance
  fields. GV2-013 and GV2-014 each require their own privacy ADR before their
  graphs become persistable (cross-reference spec condition C-6 in the item
  text so the DTO cannot be "conveniently" extended). _(security F5,
  adversarial P-3)_
- **PV-7 — No-leak test scope widened.** The structural test asserts:
  (a) **no `PathBuf`-typed field exists in the payload at all** (stronger than
  "paths are relative"); (b) every path-bearing `String` is
  workspace-root-relative; (c) `GraphDelta` is entirely absent from the DTO —
  including `errors: Vec<String>` and the `previously_*` baseline sets (which
  embed `file::kind::name` concatenations); (d) identity strings
  (name/file/specifier) are the *only* permitted `String` fields, so a
  message/error channel cannot drift in; (e) any future span type is a no-text
  `ByteRange`. _(kernel B-1/B-2, security F3/F8)_
- **PV-8 — Snapshot filename derivation pinned.** The `WorktreeKey`→filename
  derivation is a named, stable, one-way hash of the canonical path (not the
  default hasher, not the rendered path). The no-leak test extends to assert no
  filename/directory component under `graph-cache/` encodes an absolute path
  prefix. Worktree identity persists *only* as that filename key-hash.
  _(adversarial P-1, operations C-2)_
- **PV-9 — Machine-local boundary pinned as a gate.** Add to GV2-030
  acceptance criteria: *"Identity keys in this snapshot are machine-local. Any
  feature that exports, syncs, or transmits snapshot bytes off the originating
  machine requires a new privacy review before that export surface ships."*
  This answers the wave verdict's open question §7.3: the cleartext residual is
  accepted for default-on graduation **only within** the same-uid machine-local
  boundary; no scrub pass is required for v1. _(adversarial P-2)_
- **PV-10 — Telemetry labels are enums only.** `snapshot_load_result` /
  `snapshot_write_result` counters bind to a machine-local ADR-035 pipe
  (tracing or notification); labels carry outcome enum values only — never
  `WorktreeKey` paths, absolute paths, or symbol names. Routing via Kindling
  would cross the same-uid boundary (git-committed store) and requires the C-6
  per-graph review first. _(operations C-3)_
- **PV-11 — `ANVIL_PERSIST_GRAPH` enters the flag catalogue.** Add it to
  `flags/manifest.json` (defaultVariant=disabled) before graduation criteria
  are evaluated, or ADR-069 must state explicitly why an operator env-var is
  exempt from the FLAGCAT drift gate. _(operations, MAJOR)_

### ADR-069 residual-risk note extensions (docs follow-up; may ride GV2-030)

- **PV-12 — Name the boundary-erosion residuals honestly.** Extend the §8
  residual-risk note to name: (a) backup tools, dotfile syncers, and
  cloud-synced home dirs picking up `~/.local/state/anvil/graph-cache/`;
  (b) CI/containerised `ANVIL_HOME` mounts readable by orchestrators;
  (c) unsalted SHA-256 content hashes being cross-machine correlatable (same
  exposure class as git blob hashes). Clarify the §10 GC startup predicate
  (session-reconnect vs path-existence — the former would GC every snapshot on
  restart) and note in the operator runbook that toggling
  `ANVIL_PERSIST_GRAPH` off does **not** delete existing snapshots.
  _(operations C-1/C-2, security F6)_

---

## Notes (no action gate)

- **N-1** — The synthetic external-module nodes copy the raw import specifier
  into both `name` and `file` (`incremental.rs:180-188`); the
  token-in-URL-specifier residual therefore lands twice. Covered by the
  existing named residual; worth one line in the GV2-030 no-leak test notes.
- **N-2** — Cleartext symbol names/specifiers/relative paths are
  **operationally correct** (operations P5): hashing names breaks the
  `dependents_of` reverse index, hashing specifiers breaks
  `re_resolve_imports`, hashing paths breaks warm-start reconcile, and an
  opaque snapshot is uninspectable during incident response. No ALLOW-T
  substitution in the §8 ALLOW set improves privacy without destroying
  function.
- **N-3** — `SnapshotLoadError` variants structurally cannot leak field
  values (the payload is undecoded when they fire); the risk is confined to
  counter labels (PV-10).
- **N-4** — The pre-GV2-002 snapshot persists session-local `u64` ids that are
  consistent within a snapshot generation only — correct per ADR-069 §3
  ("restore indexes, never the verdict"); cross-snapshot identity comparison
  becomes meaningful only after GV2-002.

---

## Gate disposition

The Ready-checklist item *"Privacy review completed for persisted
provenance/session fields"* is **satisfied** by this review: the completed
answer is that **no provenance/session field persists in v1**, the GV2-030
allowlist opens with exactly ADR-069 §8's ALLOW set plus GV2-002's new identity
fields under PV-1..PV-5, and GV2-013/GV2-014 inherit the per-field-class table
above as the starting constraint for their own privacy ADRs. The checkbox
flips in the same change that folds PV-1..PV-11 into the GV2-002/GV2-030 item
text. GV2-002 (dependency GV2-001 Merged + ratified) becomes **Ready**;
GV2-010 remains Draft on its GV2-002/GV2-003 dependencies.
