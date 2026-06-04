# ADR-069: AST-aware anti-pattern detection (off the daemon hot path)

## Status

Accepted 2026-06-04 (design council — SOUND-WITH-CHANGES, changes folded in)

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

1. **New crate `eddacraft-anvil-checks-ast`** (`crates/anvil-checks-ast/`),
   depending only on:
   - `anvil-checks` — rule/registry types, `ScanResult`/`Warning`, the shared
     suppression parser, and the `CompiledPattern → Warning` metadata mapping
     (so AST warnings carry the same `family` / `fingerprint` / `severity` /
     `definition_ref` / `spectrum_position` fields as regex warnings — see §4);
   - `tree-sitter` + `tree-sitter-rust`, both via the **workspace** version pins
     — `Query` / `QueryCursor` and the Rust grammar.

   It does **not** depend on `anvil-kernel` (council kernel-maintainer MAJOR).
   The kernel exposes only a one-line `Language::Rust => tree_sitter_rust::LANGUAGE`
   binding plus an FNV cache-key fingerprint that a stateless gate-time scanner
   does not need; depending on the kernel would drag its file-watcher stack
   (`notify` / `walkdir` / `ignore`) into the scanner for no benefit. The single
   grammar-version source of truth is the **workspace `tree-sitter-rust` pin** —
   the kernel and this crate resolve the same `Cargo.lock` entry, so grammar
   drift between them is impossible by construction (no shared runtime API
   needed).

   It is a **terminal command-path crate**: only `anvil-cli` and test crates may
   depend on it — never a library crate the daemon links (`anvil-checks`,
   `anvil-rules`, `anvil-intercept`, `anvil-graph-cache`). A `Cargo.toml` comment
   records this, mirroring the `anvil-intercept → anvil-checks` no-parser comment.
   The intercept daemon therefore does not (and must not) reach it; ADR-064 is
   preserved unchanged. The boundary is enforced by extending
   `crates/anvil-intercept/tests/daemon_dep_boundary.rs` with the same
   three-assertion shape it already uses for tree-sitter: (a) the daemon's
   normal-edge tree contains neither `tree-sitter` nor `eddacraft-anvil-checks-ast`
   **by name**; (b) `anvil-graph-cache`'s tree is likewise clean; (c) a
   **positive control** asserting `eddacraft-anvil-checks-ast`'s own tree *does*
   carry `tree-sitter-rust`, so the guard can't pass vacuously if the grammar dep
   is ever dropped.

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
   owns them), not "until supported". To stop the silent-drop bug from simply
   relocating (council generalist MAJOR), the AST scanner ships a
   **registry-completeness guard**: a test iterates every `Detection::Ast` rule in
   the compiled registry and fails unless each has an entry in the predicate table
   (§4) — so a registry `ast` rule with no scanner support fails loudly at build,
   never silently produces nothing.

4. **AST rule shape = query + typed predicate.** A rule's `ast_query` is a
   tree-sitter S-expression selecting candidate nodes; context the query language
   cannot cleanly express is handled by a rule-specific, Rust-coded predicate in
   an `ast_rewrite_spec`-style data table that mirrors the existing regex
   `rewrite_spec` (and carries the same drift-guard snapshot of the registry
   `ast_query`). Each matched warning is built through the **same
   `CompiledPattern → Warning` mapping** the regex loader uses, so AST findings
   carry identical `family` / `fingerprint` / `severity` / `nudge` /
   `definition_ref` / `spectrum_position` metadata and emit consistently to
   SARIF. The three predicates RSTLAN-003 needs (predicate specs hardened per
   council adversarial MAJOR-2 / kernel NIT):
   - **cfg(test) exclusion:** drop a match whose ancestors include a `mod_item`
     gated by a `#[cfg(...)]` whose predicate tree **contains the `test` token**
     (parse the `cfg` meta-item; match `cfg(test)`, `cfg(all(test, …))`,
     `cfg(any(test, …))`, and reject the negation `cfg(not(test))` — a substring
     check is both too broad and too narrow). **Plus** a path allowlist for
     integration tests (`**/tests/**`, `**/benches/**`, `**/examples/**`), which
     are separate Cargo targets with no `cfg(test)` ancestor. A standalone
     `#[test] fn` outside a `cfg(test)` mod is **still flagged** (it should live in
     a test module); documented as intended, suppressible.
   - **preceding-SAFETY-comment:** keep an `unsafe_block` match only when its
     **immediately-preceding non-trivia sibling node** is not a line/block comment
     matching `(?i)^\s*//+\s*SAFETY` (AST-sibling semantics, not byte proximity —
     so an intervening blank line does not defeat the rule, but an unrelated
     statement between the comment and the block does).
   - **serde hygiene:** match a `struct_item` carrying `#[derive(… Deserialize …)]`
     whose attribute set lacks `deny_unknown_fields` (and the related flatten /
     secret-field shapes). The suppression anchor for a multi-line struct is its
     `struct_item` start line; the rule doc carries a worked example.

5. **Suppression** resolves on the matched node's **start line** via the existing
   `// @anvil-ignore <ID> -- <reason>` parser (already multi-comment-syntax aware,
   so Rust `//` works). To match the regex scanner's existing convention — the
   directive sits on the line **immediately preceding** the finding's anchor line
   — an AST finding is suppressed by placing the directive on the line directly
   above the node's start-line (for a multi-line match like a serde `struct_item`,
   that is the line above the `struct` keyword / first attribute). Node start-line
   is the deterministic, author-predictable anchor (Copilot review on #2310).

6. **Determinism** is preserved: tree-sitter parse and query evaluation are
   deterministic; AST results merge into the same `(line, column, id)` sort the
   regex scanner already applies, so `anvil check` output stays
   same-input-same-output.

7. **Any `anvil check` or `anvil gate` invocation** (local *or* CI — "gate-time"
   means "not the save-time daemon path", not "CI only") runs both scanners and
   merges results: the regex scan (`anvil-checks`) and, when the file's language
   has AST rules, the AST scan (`anvil-checks-ast`). A cross-scanner ordering
   normalisation keeps the merged output deterministic.

8. **Degradation contract (fail-safe, warnings-over-blocks).** A tree-sitter
   parse failure or `has_error()` partial tree on one file skips that file's AST
   rules and emits a single skipped-file `Warning` (mirroring the save-time
   oversized-file diagnostic); it never aborts the run, and the process still
   exits 0 by default. A malformed `ast_query` in the registry is a
   **scanner-init error** surfaced loudly (not a per-file silent skip), caught by
   the registry-completeness guard (§3) and a `Query::new` smoke test.

9. **Observability + tier legibility.** The AST scan path carries `tracing` — a
   `#[instrument]` span over the pass and a per-file `debug` (path, language,
   matched-rule count) + `warn` on parse-skip — so `RUST_LOG=debug anvil gate` is
   diagnosable. AST-family findings are marked legible in output and SARIF (an
   `"ast"` entry in the SARIF `ReportingDescriptor.properties` bag) so consumers
   can see that a rule is a gate-tier AST rule, not a save-time regex rule —
   addressing the priority-inversion risk (cheap `todo!` fires live at save-time
   while the higher-value `unwrap`/`unsafe` fire only at gate).

The mechanism is language-general (query + predicate table), so it is reusable by
the future Python anchor and any later AST-needing TS rules; RSTLAN-003 is its
first consumer.

## Rationale

The new-crate boundary is the only option that satisfies ADR-064 under workspace
feature unification (resolver 2 unifies normal-dep features across workspace
members in a single `cargo build --workspace`, so a feature flag on `anvil-checks`
would still link tree-sitter into the daemon — verified against the root
`Cargo.toml` and `daemon_dep_boundary.rs`). The new crate depends on the
`tree-sitter-rust` workspace pin directly rather than on `anvil-kernel`, so it
gets the grammar without the kernel's watcher stack. Tiering AST rules to
gate-time is not a compromise forced by the boundary — it is the same hot/non-hot
split ADR-061 already chose for structural checks, so it keeps the save-time
attestation narrow and fast and the daemon parser-free.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **New crate `anvil-checks-ast`, gate-time, direct `tree-sitter-rust` dep (chosen)** | Preserves ADR-064 (daemon links no tree-sitter); registry stays single source; reusable for Python anchor; daemon untouched; no kernel watcher-stack coupling | Two scan paths to keep coherent; AST rules don't fire at save-time, only gate-time (a tiering users must understand) |
| New crate but depending on `anvil-kernel` for the grammar | One nominal "grammar source" | Drags the kernel's `notify`/`walkdir`/`ignore` watcher stack into the scanner for a one-line `LANGUAGE` binding; the workspace `tree-sitter-rust` pin already *is* the single grammar source — rejected (council kernel-maintainer MAJOR) |
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
  only at `anvil check` / `anvil gate`, not at save-time — a deliberate
  **priority inversion** (the cheap-to-detect `todo!()` fires live at save-time
  while the higher-value `unwrap()` / `unsafe` fire only at gate). §9's SARIF
  `properties` tag + rule docs make the tier legible so "save-time is clean"
  is not misread as "fully clean". The implementer must not leave the tier
  signal to prose alone.
- **Required process deliverables** (council operations — CI will redline if
  skipped):
  - extend `daemon_dep_boundary.rs` with the three-assertion guard from §1;
  - regenerate the Hakari workspace-hack (`cargo hakari generate`) — the new
    crate shifts the unified feature set;
  - regenerate `ACKNOWLEDGEMENTS` (`tools/starters/acknowledgements/generate-acknowledgements.sh`)
    — the new crate is linked into the shipped `anvil` binary (no new licence is
    expected since tree-sitter is already in the tree, but the freshness gate runs);
  - add the new crate to any `nx run-many -t build` / CI "build test dependencies"
    list if applicable.
- **Risks:**
  - *Grammar drift* — a `tree-sitter-rust` bump could shift node kinds and break a
    query. Pinned via the workspace `Cargo.lock` entry (not a runtime check); a
    `cargo update` scoped to tree-sitter without a concurrent drift-guard run is
    the failure mode. Mitigated by snapshot tests over the compiled `ast_query`
    set (mirroring the regex `expected_registry_*` drift guards).
  - *Cross-path parity confusion* — DSV-009 established a save-time↔gate parity
    gate for the regex antipattern family. AST rules are out of that set **by an
    explicit filter**, not by naming convention: the parity gate
    (`crates/anvil-intercept/tests/diagnostic_parity.rs`) reads `detection.type`
    from the loaded registry and excludes `Detection::Ast` rules from the
    expected-findings corpus, so a future `ast` rule cannot create a phantom pass
    or a spurious failure.
  - *Daemon boundary regression* — a future edit could make the daemon depend on
    `anvil-checks-ast` transitively. Mitigated by the §1 guard (asserts the crate
    *and* tree-sitter absent from the daemon closure, with a positive control).
  - *Doubled parse at gate-time* — `anvil check` already parses `.rs` via the
    kernel for architecture analysis; a fresh `Parser` in the AST scanner would
    parse each file twice. Acceptable for a non-hot gate operation; the
    implementation should note whether it reuses a kernel `ParseResult` or parses
    independently, and why.
- **Mitigations:** see each risk above; all are guard-test-backed.

## Implementation Sequencing

(Council pragmatic-lead — recorded so the implementer doesn't serialise the
module behind this work.)

- **RSTLAN-004 (entry points) and RSTLAN-005 (boundary enforcement — the Rust T3
  product headline) do not depend on this ADR or on RSTLAN-003.** They build on
  the merged RSTLAN-001/-002 extractor and may proceed **in parallel** with the
  AST-mechanism work. The anti-pattern catalogue is governance depth on a surface
  the user can already see; the boundary work is the surface itself — it should
  not wait behind the catalogue.
- **First implementation PR is a one-rule proof slice**, not the full catalogue:
  the new crate skeleton + the §1 boundary guard + a single end-to-end rule
  (`unsafe` without `// SAFETY:` — the simplest predicate, adjacent-sibling only,
  no cfg(test) walk) + good/bad fixtures + the `Detection::Ast` retarget. That
  proves the scan path, registry integration, suppression anchor, and boundary
  guard before the cfg(test) and serde predicates land as follow-ons. This likely
  motivates a **RSTLAN-003 / -003b split** (regex-clean rules + AST mechanism vs
  the fuller AST catalogue), recorded in the module when the work starts.

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
