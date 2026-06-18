# Pack Coverage Survey — Full

| Type  | Authority | Owner | Status | Freshness          |
| ----- | --------- | ----- | ------ | ------------------ |
| Guide | Advisory  | BETA  | Live   | Created 2026-06-18 |

| Upstream                                                                        | Downstream                                                   |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `patterns/compiled/registry.json`, `crates/anvil-checks`, `crates/anvil-policy` | `patterns/` fixtures, pack/profile gaps, new check proposals |

A tester questionnaire for confirming **language packs**, **semantic packs**,
and **governance packs** cover what real codebases need. Each section hunts for
three things:

- **Gaps** — a language, policy, or pattern you use that Anvil has no pack for.
- **False negatives** — a real problem Anvil should have caught and didn't.
- **False positives** — noise Anvil flagged that is genuinely fine, eroding
  trust.

> **How to fill this in:** Answer **per repository**, not in the abstract. For
> every "missed" or "false positive" claim, paste the **actual `file:line` +
> rule ID** — that turns an impression into a fixture we can add to `patterns/`.

## What Anvil ships today (for reference)

- **Language packs** (prefix = language): `RS` Rust (5, AST-based), `PY` Python
  (7, regex), `AP` TypeScript/JS (11, regex), `GS` Go/guardrail (1).
- **Semantic packs** (families across languages): guardrail-suppression,
  type-system-evasion, error-visibility, dynamic-execution, plus per-language
  reliability families.
- **Governance packs** (process integrity): `DD` deferred-debt (4), `RL`
  responsibility-laundering (6), plus secrets / git-history scanning.
- **Config**: profiles (`minimal` / `standard` (default) / `strict` / `custom`),
  `.anvilrc`, per-rule glob allowlists, inline
  `// @anvil-ignore RS-001 -- reason`.

---

## Tester & repository

```
Tester name:
Repository / project:
Primary languages (%):
Profile (minimal / standard / strict / custom):
Anvil version / commit:
```

---

## A. Language packs — "is the right language covered, and covered well?"

**A1.** What languages and file types are in this repo? List every language by
rough % of the codebase.

```
your answer:
```

**A2.** For each language Anvil supports (Rust / Python / TS-JS): did it scan
the files you expected? Were any extensions silently skipped (`.mjs`, `.cjs`,
`.pyi`, `.tsx`)?

```
your answer:
```

**A3.** Did it miss anything obvious? Point at a file where you _know_ there's
an `unwrap()`/`panic!`, a bare `except`, an `any`, etc. — did the matching check
fire?

| `file:line` | What's there | Rule that should have fired | Fired? (Y/N) |
| ----------- | ------------ | --------------------------- | ------------ |
|             |              |                             |              |
|             |              |                             |              |

**A4.** False positives — did any check flag code that is genuinely fine
(generated code, vendored deps, migrations, test helpers)?

| `file:line` | Rule ID | Why it's actually fine |
| ----------- | ------- | ---------------------- |
|             |         |                        |
|             |         |                        |

**A5.** (Rust only, AST-based) Did the AST checks correctly **exclude test
code** (`#[cfg(test)]`, `tests/`, `benches/`, `examples/`) and macro bodies? Any
false hit inside `macro_rules!`?

```
your answer:
```

**A6.** Are there language-specific anti-patterns you care about that aren't
checked at all? (e.g. Go `err` ignored with `_`, Python mutable default args, TS
non-null `!` assertions.)

```
your answer:
```

---

## B. Semantic packs — "do the concept families catch your real escape hatches?"

**B7.** How does your team actually bypass safety today? List the escape hatches
you really use (linter-disable comments, type ignores, `eval`, dynamic imports,
reflection). Did the matching family catch each one?

| Escape hatch you use | Caught? (Y/N) | Rule ID (if caught) |
| -------------------- | ------------- | ------------------- |
|                      |               |                     |
|                      |               |                     |

**B8.** Were there suppression styles Anvil didn't recognise (block vs line
`eslint-disable`, `# noqa` variants, `@ts-expect-error` with codes, per-rule
disables)?

```
your answer:
```

**B9.** Type-system-evasion: does it catch your stack's evasions (`any`,
`z.any()`, `as unknown as`, Python `Any`, `# type: ignore`)? Anything it should
but doesn't?

```
your answer:
```

**B10.** Dynamic-execution (security family): is the severity right that
`eval()`/`new Function()` = **error/block**, or too aggressive for your
codebase?

```
your answer:
```

**B11.** Do the family groupings make sense, or would you expect a check to live
in a different family?

```
your answer:
```

---

## C. Governance packs — "do these match your org's actual policies?"

**C12.** Deferred-debt: what is your real convention for tracking TODOs (e.g.
`TODO(JIRA-123)`, `# FIXME #456`)? Did `DD-001..004` recognise _your_ format, or
flag correctly-tracked TODOs as untracked?

```
your TODO convention:
result:
```

**C13.** Responsibility-laundering / accountability: do `RL-001..006` reflect
language your team actually uses in PRs/commits ("pre-existing", "not touched",
"will follow up")? Any false accusations? Any laundering pattern they missed?

```
your answer:
```

**C14.** Secrets / git-history scanning: did it scan history as expected and
catch planted test secrets — _without_ flagging documented example keys (e.g.
`AKIAIOSFODNN7EXAMPLE`)? Any real-looking secret it missed?

```
your answer:
```

**C15.** Are there governance policies your org enforces that have no Anvil pack
— licence/SPDX headers, dependency-licence allowlists, file-structure/naming
rules, banned imports, copyright headers?

```
your answer:
```

**C16.** Do the severities of governance checks match your tolerance (block the
build vs warn)?

```
your answer:
```

---

## D. Configuration & profiles — "is the default the right default?"

**D17.** Which profile did you run? Was the default `standard` the right
balance, or did you immediately switch?

```
your answer:
```

**D18.** Did you need to enable opt-in checks (e.g. `AP-002`, `AP-016`)? Was it
discoverable that they existed?

```
your answer:
```

**D19.** Did you have to **disable** any check repo-wide to make Anvil usable?
Which, and why? (A disabled check = a coverage gap to note.)

| Rule ID | Why disabled |
| ------- | ------------ |
|         |              |
|         |              |

**D20.** Was registry/config discovery (`.anvilrc`, `ANVIL_REGISTRY_PATH`,
upward walk, embedded fallback) predictable, or did it pick up the wrong config?

```
your answer:
```

**D21.** Did inline `// @anvil-ignore RS-001 -- reason` suppression work, and is
requiring a reason the behaviour you want?

```
your answer:
```

---

## E. Execution model — "does warn/baseline/exit-code behaviour fit your workflow?"

**E22.** Baseline / "new edges only": on an existing dirty codebase, did Anvil
warn only on _new_ violations rather than drowning you in pre-existing ones?
Where did this surprise you?

```
your answer:
```

**E23.** Exit codes: did `error`-severity findings block (non-zero) and
`warning`/`info` pass (exit 0) as expected in CI?

```
your answer:
```

**E24.** Determinism: run twice on the same input — identical output? Any
ordering or path-dependent flakiness?

```
your answer:
```

**E25.** Performance: acceptable on your largest repo? Where did it feel slow?

```
your answer:
```

**E26.** Output quality: were findings actionable — clear `file:line`, rule ID,
and _why_ — enough to fix without opening docs?

```
your answer:
```

---

## F. Coverage closing questions (answer all)

**F27.** Name one problem in your codebase you **expected Anvil to catch and it
didn't**.

```
your answer:
```

**F28.** Name one finding you **didn't trust or ignored**.

```
your answer:
```

**F29.** If you could add **one pack or one check**, what would it be?

```
your answer:
```

**F30.** Was there a language, framework, or policy where Anvil felt like it
**wasn't built for your stack**?

```
your answer:
```
