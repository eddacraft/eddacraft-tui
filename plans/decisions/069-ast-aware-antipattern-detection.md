# ADR-069: AST-aware anti-pattern detection (off the daemon hot path)

## Status

Proposed

## Date

2026-06-04

## Context

RSTLAN-003 (the Rust T2 anti-pattern catalogue) calls for rules that the current
anti-pattern scanner cannot express. The `anvil-checks` scanner is **regex plus
same-line post-filter only**: `scanner.rs::rewrite_spec` carries six hand-coded
PCRE rewrites whose `FilterSpec::Negative` / `TrailingByteOrEol` predicates act
on the **matched line**, with no adjacent-line or syntactic context.

Scoping RSTLAN-003 against that mechanism splits its rules by feasibility:

- **Regex-clean (already feasible):** `todo!()` / `unimplemented!()` shipped —
  rare in tests, no context needed.
- **Needs `#[cfg(test)]`-module awareness:** `unwrap()` / `expect()` in non-test
  code, `panic!` in library code. A path-based allowlist cannot see an inline
  `#[cfg(test)] mod tests { … }`, so a pure-regex version false-positives on
  Anvil's own inline unit tests and would fail the §16.5 #9 false-positive bar at
  the RSTLAN-008 dogfood.
- **Needs adjacent-line context:** `unsafe` block without a `// SAFETY:` comment
  (the justification comment conventionally sits on the preceding line).
- **Needs real AST:** serde hygiene (`#[serde(deny_unknown_fields)]` missing on
  external-input structs, `#[serde(flatten)]` without validation, `Deserialize`
  on secret-bearing types) and `.clone()` in a hot loop.

The wire format is already half-built for this: `Detection::Ast { ast_query }`
exists in `crates/anvil-checks/src/antipattern/registry_loader.rs`, the TS
`compile-patterns` schema (`packages/anvil/core/src/anvil-format/schemas.ts`)
already accepts `detection.type: ast`, and the loader **silently drops** AST
rules (`compiled_to_antipattern` returns `None`, pinned by the
`skips_ast_detection_until_scanner_supports_it` test). What is missing is a
scanner that consumes them.

**The load-bearing constraint is ADR-064.** The intercept daemon links
`anvil-checks` on the save-time hot path: `validate_paths` calls
`run_antipattern_check_bytes`, and the `daemon_dep_boundary` guard test asserts
the resident daemon links **no tree-sitter** (ADR-064 §4 — "the daemon links no
tree-sitter; the cache write-path receives already-parsed `FileSymbols` from the
kernel feed"). AST detection needs tree-sitter. If `anvil-checks` gained a
tree-sitter dependency — even behind a Cargo feature — Cargo's workspace feature
unification would build `anvil-checks` once with the union of features for a
`cargo build --workspace`, so the daemon binary (which depends on `anvil-checks`)
would link tree-sitter anyway. A feature flag on `anvil-checks` therefore does
**not** preserve the boundary; the `daemon_dep_boundary` guard would (correctly)
fail.

RSTLAN-001/-002 (ADR-065, merged via PR #2303) already wired `tree-sitter-rust`
into `anvil-kernel` with a `Language::Rust` variant and a Rust symbol extractor,
so the grammar and a parse entry point exist to reuse.

A decision is needed now because RSTLAN-003 is blocked on it, and the choice
between "accept regex false-positives" and "invest in AST detection" determines
the whole shape of the remaining Rust-anchor catalogue work.

## Decision

Add AST-aware anti-pattern detection in a **new crate the resident daemon does
not link**, and run AST rules on the whole-repo `anvil check` / `anvil gate`
path only — never on the save-time daemon hot path.

1. **New crate `eddacraft-anvil-checks-ast`** (`crates/anvil-checks-ast/`). It
   depends on:
   - `anvil-checks` — for the rule/registry types, `ScanResult`/`Warning`
     shapes, and the shared suppression parser;
   - `anvil-kernel` — to reuse the already-wired `Language` + tree-sitter
     grammars (one grammar-version source of truth, RSTLAN-001's
     `grammar_version`);
   - `tree-sitter` — for `Query` / `QueryCursor`.

   The intercept daemon (`anvil-intercept`) **does not** depend on it. Only the
   `anvil-cli` check/gate path does. ADR-064 is preserved unchanged; the
   `daemon_dep_boundary` guard stays green. A new guard test asserts
   `anvil-checks-ast` is absent from the daemon's dependency closure.

2. **AST rules are a gate-time tier, not a save-time tier.** This is consistent
   with ADR-061, which deliberately keeps structural/expensive checks on
   whole-repo `anvil gate` and narrows the save-time daemon attestation to the
   cheap antipattern family. The regex antipattern catalogue continues to run at
   both save-time (daemon) and gate-time; the AST catalogue runs at gate-time
   only. `coverage: certified` on the save-time wire keeps attesting **only** the
   regex `antipattern` family (ADR-061 B2) — AST rules never widen it.

3. **`Detection::Ast { ast_query }` becomes live**, consumed only by the AST
   scanner. The registry stays the single source of truth: the regex scanner
   keeps skipping `Ast` rules (its existing behaviour), and the AST scanner skips
   `Regex` rules. The `skips_ast_detection_until_scanner_supports_it` test is
   retargeted to assert the regex loader skips them *by design* (the AST scanner
   owns them), not "until supported".

4. **AST rule shape = query + typed predicate.** A rule's `ast_query` is a
   tree-sitter S-expression selecting candidate nodes; context the query language
   cannot cleanly express is handled by a rule-specific, Rust-coded predicate in
   a `ast_rewrite_spec`-style data table that mirrors the existing regex
   `rewrite_spec` (and carries the same drift-guard snapshot of the registry
   `ast_query`). The three predicates RSTLAN-003 needs:
   - **cfg(test) exclusion:** drop a match whose ancestors include a `mod_item`
     preceded by an `attribute_item` containing `cfg(test)`.
   - **preceding-SAFETY-comment:** keep an `unsafe_block` match only when no
     `// SAFETY:` / `// Safety:` line comment immediately precedes it.
   - **serde hygiene:** match a `struct_item` carrying `#[derive(… Deserialize …)]`
     whose attribute set lacks `deny_unknown_fields` (and the related flatten /
     secret-field shapes).

5. **Suppression** resolves on the matched node's **start line** via the existing
   `// @anvil-ignore <ID> -- <reason>` parser (already multi-comment-syntax aware,
   so Rust `//` works). Node start-line is the deterministic, author-predictable
   anchor.

6. **Determinism** is preserved: tree-sitter parse and query evaluation are
   deterministic; AST results merge into the same `(line, column, id)` sort the
   regex scanner already applies, so `anvil check` output stays
   same-input-same-output.

7. **`anvil check` / `anvil gate`** run both scanners and merge results: the
   regex scan (`anvil-checks`) and, when the file's language has AST rules, the
   AST scan (`anvil-checks-ast`). A cross-scanner ordering normalisation keeps the
   merged output deterministic.

The mechanism is language-general (query + predicate table), so it is reusable by
the future Python anchor and any later AST-needing TS rules; RSTLAN-003 is its
first consumer.

## Rationale

The new-crate boundary is the only option that satisfies ADR-064 under workspace
feature unification while still reusing the kernel's grammar. Tiering AST rules
to gate-time is not a compromise forced by the boundary — it is the same
hot/non-hot split ADR-061 already chose for structural checks, so it keeps the
save-time attestation narrow and fast and the daemon parser-free.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **New crate `anvil-checks-ast`, gate-time (chosen)** | Preserves ADR-064 (daemon links no tree-sitter); reuses kernel grammar; registry stays single source; reusable for Python anchor; daemon untouched | Two scan paths to keep coherent; AST rules don't fire at save-time, only gate-time (a tiering users must understand) |
| **Feature flag `ast` on `anvil-checks`** | No new crate | Cargo workspace feature unification builds `anvil-checks` once with the union of features, so `cargo build --workspace` links tree-sitter into the daemon → **violates ADR-064**, `daemon_dep_boundary` guard fails |
| **Parse-then-feed tree from kernel to daemon; run AST at save-time** | AST rules would fire at save-time too | Adds a parse to the hot path; couples the watcher `FileSymbols` feed to a second consumer; risks tree-sitter re-entering the daemon; contradicts ADR-061's deliberately narrow save-time attestation |
| **Stay regex-only; accept FP with aggressive allowlists** | No new mechanism | Heavy false-positives on inline `#[cfg(test)]` unwraps; fails the §16.5 #9 FP bar at RSTLAN-008; serde hygiene is simply impossible with regex |
| **Put AST detection inside `anvil-kernel`** | Kernel already has tree-sitter | The rule/registry/suppression machinery lives in `anvil-checks`; pulling the catalogue into the kernel is a layering inversion; a dedicated crate that depends on both is cleaner |

## Consequences

- **Positive:** The full RSTLAN-003 catalogue becomes feasible at high fidelity
  (cfg(test)-aware unwrap/expect/panic, SAFETY-comment-aware unsafe, serde
  hygiene). The AST mechanism is reusable for the Python anchor and future
  AST-needing TS rules. The daemon, ADR-064, and the save-time latency budget are
  untouched. The already-present `Detection::Ast` wire format and TS schema stop
  being dead code.
- **Negative:** A second scan path and a new crate to maintain. AST rules fire
  only at `anvil check` / `anvil gate`, not at save-time — so an agent editing in
  a watched session sees regex findings live but AST findings only at gate. This
  tiering must be documented (rule docs + `anvil check` output should make the
  tier legible).
- **Risks:**
  - *Grammar drift* — a `tree-sitter-rust` bump could shift node kinds and break a
    query. Mitigated by reusing the kernel's `grammar_version` and snapshot tests
    over the compiled `ast_query` set (mirroring the regex `expected_registry_*`
    drift guards).
  - *Cross-path parity confusion* — DSV-009 established a save-time↔gate parity
    gate for the regex antipattern family; AST rules are explicitly **out** of
    that parity set (they are gate-only by design). The parity gate's scope note
    must record that AST-family findings are gate-tier, not a parity regression.
  - *Daemon boundary regression* — a future edit could make the daemon depend on
    `anvil-checks-ast` transitively. Mitigated by a `daemon_dep_boundary`-style
    guard test asserting `anvil-checks-ast` (and tree-sitter via it) is absent
    from the daemon closure.
- **Mitigations:** see each risk above; all are guard-test-backed.

## References

- Related ADRs: ADR-061 (save-time daemon delta validation — hot/non-hot split,
  narrow attestation), ADR-064 (daemon links no tree-sitter), ADR-065 (Rust T3
  architecture enforcement — Rust-native), ADR-067 (daemon symbol-feed parse
  hook)
- APS modules: RSTLAN-003 (first consumer), RSTLAN-008 (dogfood FP bar), DSV-009
  (regex-family parity gate scope note)
- Code: `crates/anvil-checks/src/antipattern/registry_loader.rs` (`Detection::Ast`,
  `compiled_to_antipattern`), `crates/anvil-checks/src/antipattern/scanner.rs`
  (`rewrite_spec`, suppression, determinism sort),
  `packages/anvil/core/src/anvil-format/schemas.ts` (AST detection schema),
  `crates/anvil-intercept/Cargo.toml` + `daemon_dep_boundary` guard
