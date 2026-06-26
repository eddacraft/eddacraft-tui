# CIB-080 Mini Council — Secret-detection Fixture FP Tuning

| Type   | Authority | Owner | Status |
| ------ | --------- | ----- | ------ |
| Review | Advisory  | CIB   | Closed |

## Scope

CIB-080 tuned the language-agnostic secret scanner to suppress known benign
fixture/test-vector false positives without weakening high-confidence credential
patterns. The implementation touched:

- `crates/anvil-checks/src/secret/context.rs`
- `crates/anvil-checks/src/secret/entropy.rs`
- `crates/anvil-checks/src/secret/scanner.rs`
- `crates/anvil-checks/src/secret/types.rs`
- `crates/anvil-checks/src/secret/mod.rs`

## Council constraints applied

Mini Council guidance split into two constraints:

- Security reviewer: suppress only bounded benign fixture/test-vector contexts;
  keep real provider keys, JWTs, and database URLs high-confidence.
- Adversarial reviewer: test paths alone must not suppress; suppressions must be
  observable as structured `Suppression` records rather than silent heuristics.

The implementation follows those constraints by routing CIB-080 built-in skips
through `AllowlistProvenance::BuiltinBenignFixture` and by requiring a bounded
combination of value shape plus validator/fixture/binding context.

## Implemented behaviour

- Added shared secret context helpers for test/fixture paths, local context
  windows, validator/fixture cues, benign contexts, and sensitive bindings.
- Suppressed benign high-entropy vectors only when paired with validator or
  public alphabet/identifier context:
  - zod base64/base64url validation vectors;
  - public alphabet and base-62 digit constants bound as `chars`, `digits`, or
    `alphabet`;
  - the exact zod KSUID validator vector.
- Suppressed placeholder database URL fixtures for zod validator/template-literal
  contexts, including `${string}:${string}@...` template alternatives.
- Suppressed the exact public zod/JWT validator vector in parser/assertion
  context.
- Suppressed the low-confidence placeholder slug
  `excalidraw-oai-api-key` without suppressing real `api_key` literals.
- Preserved real-credential detection for credential-bound base64-looking values,
  real database URLs in test paths, real JWT-looking fixtures, AWS/STS textbook
  keys, Stripe test keys, and OpenAI project keys.

## Validation evidence

Focused Rust gates passed:

```text
cargo fmt --check
cargo test -p eddacraft-anvil-checks secret::
cargo test -p eddacraft-anvil-checks --test secret_detection
cargo clippy -p eddacraft-anvil-checks --all-targets -- -D warnings
```

Observed results:

- `secret::`: 91 passed.
- `secret_detection`: 21 passed.
- `cargo clippy -p eddacraft-anvil-checks --all-targets -- -D warnings`: passed.
- `cargo hakari generate --diff && cargo hakari verify`: passed
  (`workspace-hack works correctly`).

External TS/JS dogfood command:

```text
CARGO_TARGET_DIR=/tmp/anvil-cib080-target cargo build -p eddacraft-anvil --bin anvil
ANVIL_BIN=/tmp/anvil-cib080-target/debug/anvil \
  EXT_FP_WORK=/tmp/anvil-ext-fp-cib080-r5 \
  EXT_FP_OUT=/tmp/anvil-ext-fp-cib080-r5/out \
  scripts/dogfood/external-fp/run.sh langts
EXT_FP_OUT=/tmp/anvil-ext-fp-cib080-r5/out \
  python3 scripts/dogfood/external-fp/classify.py langts
```

Dogfood deltas against the pre-CIB-080 baseline:

| Corpus | Pre-CIB-080 SECRET findings | Final CIB-080 SECRET findings |
| ------ | --------------------------- | ------------------------------ |
| zod | `SECRET-HIGH-ENTROPY-STRING: 25`; `SECRET-DATABASE-URL: 10`; `SECRET-JWT-TOKEN: 2` | none |
| vite | none | none |
| excalidraw | `SECRET-API-KEY: 1`; `SECRET-HIGH-ENTROPY-STRING: 6` | `SECRET-HIGH-ENTROPY-STRING: 2` |

All three final corpus scans reported `panics(stderr): 0`.

## Residuals

The final excalidraw residuals are two Google Drive URL `id=` values in
`packages/element/tests/embeddable.test.ts`. They remain intentionally
unsuppressed in CIB-080 because a general Google/public-ID URL heuristic would
be broader than the Council-approved fixture-vector scope.
