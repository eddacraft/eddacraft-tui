# Scanner parity fixtures (RSCAN-007)

`fixtures.json` is the canonical fixture set used by both the Rust scanner
and the TypeScript scanner to prove they emit matching warnings for the
same input.

## How it works

Two integration tests load this file and assert each fixture's expected
warning set fires exactly:

- **Rust side:** `crates/anvil-checks/tests/scanner_parity.rs`
- **TS side:** `packages/anvil/core/src/antipattern/scanner-parity.test.ts`

If both suites pass on the same `expected_matches`, the engines are in
parity on the covered rules. If they diverge, one suite fails loudly and
the offending fixture becomes a concrete repro for the drift.

## Fixture format

```json
{
  "fixtures": [
    {
      "name": "human-readable description",
      "artifact_kind": "source | pr-description | commit-message | agent-output",
      "reference": "file path or identifier surfaced on warnings",
      "content": "the artifact body to scan",
      "expected_matches": [
        { "id": "AP-003", "line": 1 }
      ]
    }
  ]
}
```

Columns are deliberately omitted. Regex engines differ in how they count
match offsets on multi-byte content and on patterns with alternation, so
locking to column position invites spurious drift without catching real
engine disagreement. Line number + rule id is the durable shape.

## Adding a fixture

1. Pick a rule id from `patterns/compiled/registry.json`.
2. Craft `content` that triggers **only** that rule. Several rules share
   keyword roots (e.g. `follow-up` triggers both RL-002 and RL-005), so
   you may need to add an escape phrase that satisfies one rule's
   negative lookahead while still tripping the target rule. The RL-002
   fixtures use `in issue #N` to escape RL-005.
3. Run both suites and confirm they agree. If the Rust suite fails but
   the TS suite passes (or vice versa), you have found real engine
   drift — fix the engine, not the fixture.

## Known divergence

These rules currently fire in the TS scanner but silently do **not**
fire in the Rust scanner because their compiled regex uses PCRE
lookaround and the `regex` crate (RE2-style) cannot compile them:

- DD-001, DD-002, DD-003 (TODO / HACK / temporary without tracking ref)
- GS-001 (non-null assertion)
- RL-001 (unverified pre-existing claim)
- RL-005 (deferred without artifact)

AP-001 has the same shape but the Rust scanner hand-splits it into two
lookaround-free regexes, so it fires correctly in both engines.

The case-insensitive `flags: "i"` on every RL-* and DD-004 pattern is
honored by the TS scanner (via `new RegExp(pattern, flags)`) but
dropped by the Rust scanner (`compiled_to_antipattern` discards
`detection.flags`). Fixtures deliberately use lowercase content to
avoid exercising this gap until it's fixed.

These gaps are tracked for the broader registry-rewrite follow-up
referenced in ADR-026; they are intentionally **not** covered by
fixtures yet because a fixture covering them would require either a
Rust-side workaround or a registry rewrite, both of which exceed
RSCAN-007's "prove parity" scope.
