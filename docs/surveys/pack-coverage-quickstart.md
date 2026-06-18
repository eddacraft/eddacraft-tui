# Pack Coverage Survey — Quick

| Type  | Authority | Owner | Status | Freshness          |
| ----- | --------- | ----- | ------ | ------------------ |
| Guide | Advisory  | BETA  | Live   | Created 2026-06-18 |

| Upstream                                                                        | Downstream                                                   |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `patterns/compiled/registry.json`, `crates/anvil-checks`, `crates/anvil-policy` | `patterns/` fixtures, pack/profile gaps, new check proposals |

The short version of the [full pack-coverage survey](./pack-coverage-survey.md).
Use this when you have five minutes with a tester. It is built around the single
highest-yield question — the one that surfaces the languages, escape hatches,
and policies we never knew to ask about — plus three sharper probes.

> **How to fill this in:** Answer **per repository**. For any "missed" or "false
> positive" claim, paste the **actual `file:line` + rule ID**.

---

## Tester & repository

```
Tester name:
Repository / project:
Primary languages (%):
```

---

## The catch-all question

> **Walk me through a real repo you work in: every language and file type in it,
> how your team bypasses safety checks today, and what conventions you follow
> for TODOs, secrets, and licences — show me actual examples from the code.**

This forces testers to enumerate _their_ reality instead of reacting to our
list, which is where the unknown unknowns hide.

```
your answer:
```

---

## Three sharper probes

**1. Languages / semantic gaps** — Show me the file types in your repo and point
to a place where you _knowingly_ worked around a linter, type checker, or safety
rule. How did you do it?

```
your answer (include file:line):
```

**2. Governance gaps** — What rules does your org enforce in review that a tool
_should_ catch — licence headers, banned imports, secret handling, TODO-tracking
format — and what's your exact convention for each?

```
your answer:
```

**3. The catch-all miss** — Name one problem in your codebase you'd expect a
tool like this to catch that nothing currently does.

```
your answer:
```

---

Need the deep version (per-pack tables, false-positive logs, config and
exit-code coverage)? Use the
[full pack-coverage survey](./pack-coverage-survey.md).
