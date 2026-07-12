# Policy Pack Validation Guide

| Type  | Authority     | Owner  | Status | Freshness                                                                                                       |
| ----- | ------------- | ------ | ------ | --------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | POLVAL | Live   | Last reviewed 2026-07-04 against `crates/anvil-policy-engine/src/pack/` and the `anvil policy validate` command |

| Upstream                                                                                                                                                                                       | Downstream                                                              |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `crates/anvil-policy-engine/src/pack/` (metadata, manifest, validator, test runner), [`opa-policy-testing.md`](./opa-policy-testing.md), `plans/archive/modules/policy-pack-validation.aps.md` | Policy pack authors, CI pack checks, policy governance, gate evaluation |

`anvil policy validate` checks that a policy pack is complete, well-formed, and
tested before it is loaded for gate evaluation. It catches missing metadata,
broken manifests, absent policy files, and missing or failing tests up front, so
a pack does not fail silently at evaluation time.

For how to write and unit-test individual Rego policies, see
[`opa-policy-testing.md`](./opa-policy-testing.md). This guide covers pack-level
admission.

## What a pack is

A pack is a manifest plus the policies it lists. The manifest is a YAML file —
`pack.yaml` by convention — that describes the pack and its member policies:

```yaml
id: baseline-pack
name: Baseline Security Pack
version: 1.0.0
description: Core architectural guardrails.
owner: platform-security
policies:
  - path: policies/no-network-imports.rego
    metadata:
      id: no-network-imports
      title: Disallow new network imports
      severity: high
      owner: platform-security
      rationale: New network edges widen the blast radius of a breach.
      scope: src/**/*.rs
      tags: [security, imports]
```

Each member carries a `path` (relative to the manifest's directory) and the
policy's metadata. Every metadata field shown above is required: `id`, `title`,
`severity` (`low`, `medium`, `high`, or `critical`), `owner`, `rationale`,
`scope`, and at least one `tag`.

Member paths must stay within the manifest's own directory — an absolute path or
one containing `..` is rejected. Unknown fields on the manifest root or on a
member are rejected too, so a newer manifest read by an older engine fails
loudly rather than silently dropping entries.

## Running it

```
anvil policy validate <PATH>
```

`<PATH>` is either the manifest file itself or a directory containing a
`pack.yaml`. The command runs, in order:

1. **Load** the manifest (parse and schema-check).
2. **Structural and metadata validation** — every member's `.rego` file exists,
   metadata is complete, and policy ids are unique.
3. **Test execution** — each member's sibling `*_test.rego` is loaded alongside
   its policy into a fresh engine, its `test_*` rules are discovered, and each
   is evaluated.
4. **Test enforcement** — a missing test file, a test file with no `test_*`
   rules, or any failing rule is reported as an error.

A test rule passes when it evaluates to `true`; `false` or an undefined result
is a failure, following Open Policy Agent test semantics.

### Test discovery

Test rules are discovered by a conservative scan of each `*_test.rego` file: a
line beginning `test_` at column zero declares a test rule. A rule produced by
an unusual construct (for example generated under another rule) is not
discovered. The test package is read from the file's `package` declaration; a
package that does not end in `_test` is reported as a warning but still run.

## Output

By default the command prints a remediation-first, severity-tagged listing: each
issue names the offending policy, states the problem, and gives the fix. Pass
`--json` to emit the machine-readable validation report instead — an ordered
list of issues, each with a stable `code`, `severity`, the offending
`policy_id`/`path`, a `message`, and `remediation` — for CI and tooling.

Issue codes:

| Code                  | Severity | Meaning                                                     |
| --------------------- | -------- | ----------------------------------------------------------- |
| `missing-policy-file` | error    | A member's `.rego` file does not exist under the pack.      |
| `metadata-incomplete` | error    | A member is missing a required metadata field.              |
| `duplicate-policy-id` | error    | Two members share a policy id.                              |
| `missing-test-file`   | error    | A member has no sibling `*_test.rego`.                      |
| `no-tests-discovered` | error    | A test file exists but declares no `test_*` rules.          |
| `policy-test-failed`  | error    | A `test_*` rule failed, or a policy/test failed to compile. |
| `test-package-naming` | warning  | A test package does not follow the `<name>_test` naming.    |

## Exit codes

- **0** — no error-class issues. A pack with only warnings is valid and exits 0.
- **non-zero** — one or more error-class issues, or an operational failure such
  as a missing or unreadable manifest (reported with a distinct message).

Warnings never fail validation, following Anvil's warnings-over-blocks posture
(ADR-002).

## Not yet wired

Gate preflight — blocking gate evaluation when a pack fails validation — is not
yet connected. That wiring lands with the OPAE-003 gate repoint; today the
command is a standalone check for authors and CI.
