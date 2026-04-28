# Scanner parity fixtures (RSCAN-007 + SPG-004)

`fixtures.json` is the canonical fixture set used by both the Rust scanner and
the TypeScript scanner to prove they emit matching warnings for the same input.

## How it works

Two integration tests load this file and assert each fixture's expected warning
set fires exactly:

- **Rust side:** `crates/anvil-checks/tests/scanner_parity.rs`
- **TS side:** `packages/anvil/core/src/antipattern/scanner-parity.test.ts`

If both suites pass on the same `expected_matches`, the engines are in parity on
the covered rules. If they diverge, one suite fails loudly and the offending
fixture becomes a concrete repro for the drift.

Run both in one go:

```bash
pnpm test:scanner-parity
```

## Fixture format

```json
{
  "fixtures": [
    {
      "name": "human-readable description",
      "artifact_kind": "source | pr-description | commit-message | agent-output",
      "reference": "file path or identifier surfaced on warnings",
      "content": "the artifact body to scan",
      "expected_matches": [{ "id": "AP-003", "line": 1 }],
      "scan_options": { "include_opt_in": true }
    }
  ]
}
```

`scan_options` is optional. The default mirrors the scanner's default
(`include_opt_in: false`). Set `include_opt_in: true` for fixtures targeting
opt-in rules — AP-002, AP-005, AP-007.

Columns are deliberately omitted. Regex engines differ in how they count match
offsets on multi-byte content and on patterns with alternation, so locking to
column position invites spurious drift without catching real engine
disagreement. Line number + rule id is the durable shape.

## Adding a fixture

1. Pick a rule id from `patterns/compiled/registry.json`.
2. Craft `content` that triggers **only** that rule. Several rules share keyword
   roots (e.g. `follow-up` triggers both RL-002 and RL-005), so you may need to
   add an escape phrase that satisfies one rule's negative lookahead while still
   tripping the target rule. The RL-002 fixtures use `in issue #N` to escape
   RL-005.
3. Run both suites and confirm they agree. If the Rust suite fails but the TS
   suite passes (or vice versa), you have found real engine drift — fix the
   engine, not the fixture.

## Coverage

After SPG-004 every enabled registry rule has at least one positive and one
negative fixture. Opt-in rules (AP-002, AP-005, AP-007) exercise the same
requirement with `scan_options.include_opt_in: true`. Non-source artifact kinds
— `pr-description`, `commit-message`, `agent-output` — each have at least one
fixture so the scanner's artifact-kind dispatch path is exercised end-to-end. A
multi-rule co-firing fixture (RL-001 + RL-005 on the same line) pins the
cross-rule independence of post-filters.

## Rust-side handling of PCRE lookaround rules

Six rules — **DD-001, DD-002, DD-003, GS-001, RL-001, RL-005** — carry a PCRE
lookaround (`(?!...)` or `(?=...)`) that the RE2-based Rust `regex` crate cannot
compile directly. The Rust scanner honours them via a hand-coded post-filter in
`crates/anvil-checks/src/antipattern/scanner.rs::prepare_pcre_rewrite`: the base
regex (no lookaround) matches, then a Rust predicate applies the escape/require
clause so the observable behaviour matches the TS scanner.

The post-filter code is paired with unit tests that pin each rule's positive and
escape cases (`dd001_*`, `dd002_*`, `dd003_*`, `gs001_*`, `rl001_*`, `rl005_*`).
The parity fixtures here are the end-to-end check that both engines agree.

The AP-001 broad `eslint-disable` rule uses the same technique via its own
primary/secondary regex split.

## Case-insensitive (`flags: "i"`) handling

Seven rules — DD-004 and RL-001..006 — declare `flags: "i"` in the registry. The
Rust loader inlines this as `(?i)` on the compiled regex
(`registry_loader.rs::inline_flag_prefix`) so case-varied input matches
identically on both engines.

## Known divergence

None at this time. If a fixture begins to diverge between engines:

1. Run `anvil doctor` — SPG-002 surfaces any rule whose registry regex no longer
   compiles under the Rust engine as a `registry-patterns-compile` warning.
2. Check whether the offending rule's `.anvil` source or compiled
   `registry.json` has changed shape.
3. Update `prepare_pcre_rewrite` if a newly added lookaround rule needs
   Rust-side handling.
