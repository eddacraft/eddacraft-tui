# LSPNAV-001 Reference-Tier Certification Implementation Plan

**Goal:** Select and certify the first language/client tier from reproducible
evidence before any occurrence snapshot or references RPC is implemented.

**Architecture:** A checked-in closed taxonomy and fail-closed evidence manifest
drive a candidate audit over current extractors and authoritative language
engines. The audit may select exactly one tier or select none; it may not weaken
the taxonomy to force a winner. The resulting certification identity becomes an
input to the later graph snapshot and dynamic capability registration.

**Tech Stack:** Rust integration tests, tree-sitter extractors, JSON fixtures,
Node.js evidence checker, native language reference engines available in CI.

---

## File Map

- `plans/specs/lspnav-reference-taxonomy-v1.json` — canonical closed v1
  construct and occurrence taxonomy plus invalidation keys.
- `crates/anvil-kernel/tests/lspnav_certification.rs` — candidate extractor and
  state-parity certification harness.
- `crates/anvil-kernel/tests/fixtures/lspnav/<candidate>/` — source corpus and
  exact expected occurrence/range goldens for each candidate.
- `scripts/lspnav/certify-tier.mjs` — runs native-engine differential adapters,
  classifies every difference, and emits deterministic evidence.
- `scripts/lspnav/check-certification.mjs` — fails on missing candidates,
  unclassified differences, stale digests, zero tests, multiple winners, or no
  explicit no-winner disposition.
- `plans/reviews/lspnav-tier-certification.md` — human-readable evidence record
  and the selected tier or fail-closed no-selection decision.
- `package.json` — `lspnav:certify` and `lspnav:certification:check` commands.
- `plans/modules/lsp-graph-navigation.aps.md` — records the evidence-selected
  outcome; changes status only after dependencies and approval permit it.

## Task 1: Pin the closed taxonomy and invalidation identity

**Files:**

- Create: `plans/specs/lspnav-reference-taxonomy-v1.json`
- Test: `scripts/lspnav/check-certification.mjs`
- Modify: `package.json`

- [ ] Write a failing checker fixture proving the taxonomy must include stable
      IDs for definition/declaration, import, re-export, alias, overload,
      nested/shadowed scope, read, write, call, type use, multiple uses,
      comments/strings, generated code, and unresolved constructs.
- [ ] Run `pnpm lspnav:certification:check` and verify it fails because the
      canonical taxonomy and evidence manifest do not exist.
- [ ] Add the taxonomy with explicit supported, ignored, and fail-closed
      dispositions; do not encode a preferred language.
- [ ] Add invalidation inputs for taxonomy, parser grammar, extractor/resolver,
      coordinate conversion, snapshot schema, scheduler and protocol versions.
- [ ] Make the checker canonicalise the JSON and compute one evidence-input
      digest without source paths or machine-local values.
- [ ] Run `pnpm lspnav:certification:check`; verify it now fails only on missing
      corpus/evidence, not taxonomy shape.
- [ ] Commit: `test(lspnav): pin reference certification taxonomy`

## Task 2: Build candidate corpora and exact-range goldens

**Files:**

- Create: `crates/anvil-kernel/tests/lspnav_certification.rs`
- Create: `crates/anvil-kernel/tests/fixtures/lspnav/typescript/`
- Create: `crates/anvil-kernel/tests/fixtures/lspnav/rust/`
- Create: `crates/anvil-kernel/tests/fixtures/lspnav/python/`

- [ ] Write one explicit `--test lspnav_certification` harness that enumerates
      every taxonomy ID for every candidate, parses the corpus through the real
      extractor, and compares exact byte ranges with committed JSON goldens.
- [ ] Include alias/re-export chains, overloads, shadowing, generics, nested
      scopes, Unicode before and inside identifiers, comments/strings,
      generated/unresolved constructs, and multiple occurrences per container.
- [ ] Make a missing fixture, empty test set, unsupported construct without a
      fail-closed disposition, duplicate range, or out-of-bounds/non-UTF-8-boundary
      range fail the test.
- [ ] Run `cargo test -p eddacraft-anvil-kernel --test lspnav_certification` and
      verify red against current declaration-only extraction.
- [ ] Add only the minimum audit adapter needed to serialise current extractor
      evidence; do not add a resident occurrence index or daemon RPC in this
      item.
- [ ] Make the harness pass by recording truthful eligible/ineligible
      classifications for all three candidates. A candidate with incomplete
      extraction remains ineligible; do not delete its hard cases.
- [ ] Commit: `test(lspnav): add candidate reference corpora`

## Task 3: Differentially compare authoritative engines

**Files:**

- Create: `scripts/lspnav/certify-tier.mjs`
- Modify: `scripts/lspnav/check-certification.mjs`
- Create: `plans/reviews/lspnav-tier-certification.md`
- Modify: `package.json`

- [ ] Write failing script tests for missing native engine, non-zero engine exit,
      malformed location data, unclassified differences, engine extensions
      outside the taxonomy, and non-deterministic ordering.
- [ ] Run `pnpm lspnav:certify` and verify it refuses to select a tier from
      current incomplete or unavailable evidence.
- [ ] Implement adapters that invoke the repository-pinned authoritative engine
      for each candidate where CI support exists, normalise only to taxonomy ID,
      relative fixture file and exact byte range, and sort deterministically.
- [ ] Require every native/anvil difference to be classified as an Anvil defect,
      a native-engine extension outside the closed taxonomy, or an adjudicated
      golden correction. Unclassified differences block selection.
- [ ] Record engine versions, fixture digest, taxonomy/invalidation digest,
      platform, exact command, and result counts in the evidence document; never
      record user workspace paths or source outside committed fixtures.
- [ ] Run `pnpm lspnav:certify` twice and verify byte-identical evidence.
- [ ] Commit: `test(lspnav): add native reference differential audit`

## Task 4: Select exactly one tier or fail closed

**Files:**

- Modify: `plans/reviews/lspnav-tier-certification.md`
- Modify: `plans/modules/lsp-graph-navigation.aps.md`
- Test: `scripts/lspnav/check-certification.mjs`

- [ ] Make the checker require full/incremental/restore/restart parity evidence,
      exact range coverage, unsupported-construct failures, Unix and Windows
      results, and a launch-client dynamic-registration probe for a winner.
- [ ] Run the checker before evidence is complete and verify it refuses to emit
      a certification identity.
- [ ] If exactly one candidate satisfies every gate, record it and its digest.
      If none do, record `no tier certified` plus the blocking taxonomy IDs and
      leave LSPNAV-001 Proposed; do not lower the gate. More than one eligible
      tier is also a planning decision, not an automatic preference.
- [ ] Run `cargo test -p eddacraft-anvil-kernel --test lspnav_certification`,
      `pnpm lspnav:certify`, and `pnpm lspnav:certification:check`.
- [ ] Run `pnpm aps:active-lint`, `pnpm aps:index:check`, and
      `pnpm format:check`.
- [ ] Present the evidence-selected tier or no-winner result for operator
      approval before promoting LSPNAV-001 or starting LSPNAV-002.
- [ ] Commit: `docs(lspnav): record first-tier certification decision`
