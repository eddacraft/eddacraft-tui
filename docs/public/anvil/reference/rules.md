---
id: rule-reference
title: Compiled pattern catalogue
description: Look up source-pattern rules compiled into anvil.
---

<!-- Generated from shipped product sources. Do not edit by hand. -->

# Compiled pattern catalogue

This catalogue covers source-pattern rules in the compiled registry shipped with
anvil 0.9.4-beta. Secrets, architecture, policy, command-safety, and other gate
checks have separate engines and are not listed here. The registry contains **45
enabled rules across 11 families**.

Rule IDs are stable identifiers you may see in terminal or machine-readable
output. A warning describes a finding; it does not automatically mean a command
failed.

| Rule       | What it detects                                                | Family                    | Default severity | Applies to                                      |
| ---------- | -------------------------------------------------------------- | ------------------------- | ---------------- | ----------------------------------------------- |
| `DD-001`   | TODO/FIXME without tracking reference                          | deferred-debt             | warning          | .ts, .tsx, .js, .jsx, .mjs, .cjs, .py, .go, .rs |
| `DD-002`   | HACK comment without tracking reference                        | deferred-debt             | warning          | .ts, .tsx, .js, .jsx, .mjs, .cjs, .py, .go, .rs |
| `DD-003`   | Temporary code without expiry                                  | deferred-debt             | info             | .ts, .tsx, .js, .jsx, .mjs, .cjs, .py, .go, .rs |
| `DD-004`   | Completion claim with outstanding TODOs                        | deferred-debt             | warning          | agent-output, pr-description                    |
| `AP-008`   | eval() called with a dynamic argument                          | dynamic-execution         | error            | .ts, .tsx, .js, .jsx, .mjs, .cjs                |
| `AP-009`   | new Function() compiles a string into executable code          | dynamic-execution         | error            | .ts, .tsx, .js, .jsx, .mjs, .cjs                |
| `AP-017`   | Server-side template injection (dynamic template string)       | dynamic-execution         | error            | .py, .js, .ts, .jsx, .tsx, .mjs, .cjs           |
| `AP-006`   | Empty catch block swallows errors                              | error-visibility          | warning          | .ts, .tsx, .js, .jsx, .mjs, .cjs                |
| `AP-007`   | Console statement in production code                           | error-visibility          | info             | .ts, .tsx, .js, .jsx, .mjs, .cjs                |
| `FRAG-001` | Content authored invisible pending an entrance animation       | fragile-presentation      | warning          | .ts, .tsx, .js, .jsx, .mjs, .cjs                |
| `AP-001`   | Broad eslint-disable added                                     | guardrail-suppression     | warning          | .ts, .tsx, .js, .jsx, .mjs, .cjs                |
| `AP-002`   | Rule-specific eslint-disable                                   | guardrail-suppression     | info             | .ts, .tsx, .js, .jsx, .mjs, .cjs                |
| `AP-004`   | @ts-ignore suppresses all errors                               | guardrail-suppression     | warning          | .ts, .tsx                                       |
| `AP-005`   | @ts-expect-error used                                          | guardrail-suppression     | info             | .ts, .tsx                                       |
| `GS-001`   | Non-null assertion overrides nullability                       | guardrail-suppression     | warning          | .ts, .tsx                                       |
| `PY-001`   | # type: ignore without an error code                           | python-reliability        | warning          | .py                                             |
| `PY-002`   | bare # noqa without a rule code                                | python-reliability        | warning          | .py                                             |
| `PY-003`   | # pylint: disable suppression                                  | python-reliability        | warning          | .py                                             |
| `PY-004`   | bare except, or an inline except ...: pass swallow             | python-reliability        | warning          | .py                                             |
| `PY-005`   | wildcard import (from x import \*)                             | python-reliability        | warning          | .py                                             |
| `PY-006`   | print() in production code                                     | python-reliability        | info             | .py                                             |
| `PY-007`   | Any annotation escapes the type system                         | python-reliability        | warning          | .py                                             |
| `RL-001`   | Unverified pre-existing claim                                  | responsibility-laundering | warning          | agent-output, pr-description                    |
| `RL-002`   | Phantom follow-up tracking                                     | responsibility-laundering | warning          | agent-output, pr-description, commit-message    |
| `RL-003`   | Blanket unrelated dismissal                                    | responsibility-laundering | error            | agent-output, pr-description                    |
| `RL-004`   | Unverified "not touched" claim                                 | responsibility-laundering | warning          | agent-output, pr-description                    |
| `RL-005`   | Deferred without artifact                                      | responsibility-laundering | warning          | agent-output, pr-description, commit-message    |
| `RL-006`   | Reply disguised as fix                                         | responsibility-laundering | info             | agent-output, pr-description                    |
| `RS-001`   | unwrap() or expect() in non-test code                          | rust-reliability          | info             | .rs                                             |
| `RS-002`   | panic!() reached from non-test code                            | rust-reliability          | info             | .rs                                             |
| `RS-003`   | unsafe block without a // SAFETY comment                       | rust-reliability          | info             | .rs                                             |
| `RS-004`   | Deserialize struct without deny_unknown_fields                 | rust-reliability          | info             | .rs                                             |
| `RS-005`   | todo!() or unimplemented!() shipped                            | rust-reliability          | warning          | .rs                                             |
| `RS-006`   | catch-all serde flatten without validation                     | rust-reliability          | info             | .rs                                             |
| `RS-007`   | plaintext secret field on Deserialize type                     | rust-reliability          | info             | .rs                                             |
| `RS-008`   | clone() inside syntactic loop                                  | rust-reliability          | info             | .rs                                             |
| `AP-003`   | Explicit any type usage                                        | type-system-evasion       | warning          | .ts, .tsx, .js, .jsx, .mjs, .cjs                |
| `AP-015`   | Zod schema escape hatch (z.any / .passthrough)                 | type-system-evasion       | warning          | .ts, .tsx, .js, .jsx, .mjs, .cjs                |
| `AP-016`   | Zod z.unknown() in a schema (opt-in)                           | type-system-evasion       | warning          | .ts, .tsx, .js, .jsx, .mjs, .cjs                |
| `UR-001`   | Assignment to innerHTML / outerHTML                            | unsafe-rendering          | warning          | .ts, .tsx, .js, .jsx, .mjs, .cjs                |
| `UR-002`   | document.write() / document.writeln() call                     | unsafe-rendering          | warning          | .ts, .tsx, .js, .jsx, .mjs, .cjs                |
| `UR-003`   | React dangerouslySetInnerHTML                                  | unsafe-rendering          | warning          | .ts, .tsx, .js, .jsx, .mjs, .cjs                |
| `WC-001`   | Deprecated hash primitive (MD5 / SHA-1) in a construction call | weak-cryptography         | warning          | .ts, .tsx, .js, .jsx, .mjs, .cjs, .py, .java    |
| `WC-002`   | Broken cipher or ECB mode in a construction call               | weak-cryptography         | error            | .ts, .tsx, .js, .jsx, .mjs, .cjs, .py, .java    |
| `WC-003`   | JWT configured with the `none` algorithm                       | weak-cryptography         | error            | .ts, .tsx, .js, .jsx, .mjs, .cjs, .py, .java    |
