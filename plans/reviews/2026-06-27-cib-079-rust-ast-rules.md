# CIB-079 Mini Council — Rust AST Reliability Follow-on

| Type   | Authority | Owner | Status |
| ------ | --------- | ----- | ------ |
| Review | Advisory  | CIB   | Closed |

## Scope

CIB-079 adds the RSTLAN-003b follow-on rules to the ADR-071 gate-time AST tier:

- `RS-006` — opt-in catch-all `#[serde(flatten)]` map field without a visible validation boundary.
- `RS-007` — opt-in plaintext secret field on a derive-backed `Deserialize` type.
- `RS-008` — opt-in `.clone()` inside a syntactic `for` / `while` / `loop` body.

Changed surfaces:

- `crates/anvil-checks-ast/src/lib.rs`
- `crates/anvil-checks-ast/src/predicates.rs`
- `crates/anvil-checks-ast/src/tests.rs`
- `patterns/rust-reliability/RS-006.anvil`
- `patterns/rust-reliability/RS-007.anvil`
- `patterns/rust-reliability/RS-008.anvil`
- `patterns/rust-reliability/definition.anvil`
- `patterns/compiled/registry.json`
- `scripts/dogfood/external-fp/corpus.json`

## Council constraints applied

Mini Council returned WARN from security, adversarial, and kernel/AST reviewers.
The shared constraints were:

- Keep the implementation inside `anvil-checks-ast`; do not touch daemon-linked parser boundaries.
- Add registry entries, predicate-table entries, known ids, eval arms, and query drift guards together.
- Keep FP-prone rules opt-in unless dogfood proves default-on precision.
- Pin field/method anchors and suppression behaviour.
- Do not weaken RS-001..RS-005 or mutate their shared test/build/doc exclusions.
- Avoid misleading serde guidance: RS-004 no longer advises `deny_unknown_fields` on structs with `serde(flatten)`.

## Implemented behaviour

- `RS-006` fires only for catch-all map flatten fields on `Deserialize` structs,
  such as `HashMap<String, serde_json::Value>` / `BTreeMap<String, serde_json::Value>`.
  It is clean for typed flatten composition and visible validation boundaries
  (`serde(try_from = "...")`, field `deserialize_with`).
- `RS-007` fires only when a high-confidence credential field or `serde(rename = "...")`
  deserialises into a plaintext-ish type. It is clean for secret/redacting wrapper
  types, `skip_deserializing`, `deserialize_with`, public-key fields, token-count
  metrics, and key-path fields.
- `RS-008` fires on `.clone()` method calls inside syntactic loops. Iterator-adapter
  closures and UFCS `Clone::clone(...)` are documented/tested as out of scope;
  `Arc::clone`/shared ownership is not captured by this method-call query.
- All three rules are opt-in (`--include-opt-in`) and suppressible with
  `// @anvil-ignore RS-00x -- <reason>` at the selected field/attribute/method anchor.

## Validation evidence

RED evidence:

- New `rs006_` tests failed before registry/predicate implementation because
  RS-006 was not registered or suppressible.

Focused Rust validation passed:

```text
cargo fmt --check
cargo test -p eddacraft-anvil-checks-ast
cargo clippy -p eddacraft-anvil-checks-ast --all-targets -- -D warnings
cargo test -p eddacraft-anvil-checks-ast --test dogfood -- --ignored --nocapture
cargo hakari generate --diff && cargo hakari verify
```

Observed results:

- `cargo test -p eddacraft-anvil-checks-ast`: 80 passed.
- Internal AST dogfood: 674 files scanned, 0 parse-skips, 0 build.rs findings,
  0 tests.rs/test.rs findings, no panics.
- `cargo clippy -p eddacraft-anvil-checks-ast --all-targets -- -D warnings`: passed.
- Hakari: `workspace-hack works correctly`.

Pattern/compiler validation:

```text
pnpm --filter @eddacraft/anvil-core patterns:compile
pnpm --filter @eddacraft/anvil-core test
```

Observed results:

- `patterns:compile`: wrote 37 patterns / 8 families to `patterns/compiled/registry.json`.
- `@eddacraft/anvil-core test`: 27 files passed, 649 tests passed.
- `pnpm --filter @eddacraft/anvil-core patterns:check` was attempted but exits 1
  on pre-existing AP-prefix cross-family warnings; CIB-079 added no new warning class.

End-to-end CLI validation:

```text
cargo run -p eddacraft-anvil -- check --include-opt-in --json --no-tui /tmp/cib079-rs006-008.rs
```

The temp fixture surfaced `RS-006`, `RS-007`, and `RS-008` without `RS-004`.

External Rust FP dogfood:

```text
CARGO_TARGET_DIR=/tmp/anvil-cib079-target cargo build -p eddacraft-anvil --bin anvil
ANVIL_BIN=/tmp/anvil-cib079-target/debug/anvil \
  EXT_FP_WORK=/tmp/anvil-ext-fp-cib079 \
  EXT_FP_OUT=/tmp/anvil-ext-fp-cib079/out \
  scripts/dogfood/external-fp/run.sh rust
EXT_FP_OUT=/tmp/anvil-ext-fp-cib079/out \
  python3 scripts/dogfood/external-fp/classify.py rust
```

Default-catalogue results had no RS-006/RS-007/RS-008 findings because all three
new rules are opt-in. Final default runs reported `panics(stderr): 0` for ripgrep,
tokio, and alacritty. Opt-in-only characterisation:

| Corpus | RS-006 | RS-007 | RS-008 |
| ------ | ------ | ------ | ------ |
| ripgrep | 0 | 0 | 13 |
| tokio | 0 | 0 | 22 |
| alacritty | 0 | 0 | 9 |

The opt-in-only RS-008 hits are intentionally outside the default FP rate and are
left for worksheet classification before any future default-on promotion.
