# OPA Policy Testing Guide

| Type  | Authority     | Owner | Status | Freshness                                                                                                         |
| ----- | ------------- | ----- | ------ | ----------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | OPAG  | Live   | Last reviewed 2026-05-25 against `packages/anvil/policy/src/policy-loader.ts` and `policies/fixtures/` test packs |

| Upstream                                                                                                                                                        | Downstream                                               |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| `packages/anvil/policy/src/policy-loader.ts`, `packages/anvil/policy/src/opa-executor.ts`, `policies/fixtures/`, `plans/modules/opa-agent-orchestration.aps.md` | Policy authors, OPA fixture tests, gate policy execution |

How to write, test, and ship OPA/Rego policies for Anvil's gate. This guide
covers the canonical fixture layout, the `*_test.rego` convention, the CI
toolchain, and the minimum set of tests every policy pack should ship.

## Where policies live

| Location                       | Purpose                                                          |
| ------------------------------ | ---------------------------------------------------------------- |
| `policies/fixtures/`           | Repo-wide fixture pack used by all integration tests (TS + Rust) |
| `<workspace>/.anvil/policies/` | Default `policy_dir` resolved by `PolicyCheck` at runtime        |
| `<workspace>/<custom>/`        | Override via `gate.yaml` `checks[].config.policy_dir`            |

`PolicyLoader` walks the policy dir, treating `*.rego` files as policies while
excluding `*_test.rego` files from policy discovery, and treating each
`*_test.rego` sibling as its unit-test file. Policy packages should be
`anvil.policies.<policy_name>` and test packages
`anvil.policies.<policy_name>_test`. The loader does not validate package names
during discovery (it uses filenames), but Anvil's OPA queries read results from
`data.anvil.policies`, so policies whose package sits outside that hierarchy
will load but their results won't surface in evaluation.

## Anatomy of a policy pack

Every policy pack ships **two files**:

```
policies/fixtures/
├── change_scope.rego          # production rules
└── change_scope_test.rego     # unit tests
```

`change_scope.rego`:

```rego
package anvil.policies.change_scope

import rego.v1

default max_files := 20

violation contains msg if {
  file_count := count(input.plan.proposed_changes)
  file_count > max_files
  msg := sprintf("Plan touches %v files, maximum is %v", [file_count, max_files])
}
```

`change_scope_test.rego`:

```rego
package anvil.policies.change_scope_test

import rego.v1
import data.anvil.policies.change_scope

test_too_many_files if {
  count(change_scope.violation) > 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_create", "path": "f1.ts", "directory": "src"},
        {"type": "file_create", "path": "f2.ts", "directory": "src"},
      ]
    },
    "config": {"max_files": 1}
  }
}
```

Rules:

- One package per `.rego` file.
- Test package mirrors the production package with `_test` suffix.
- Tests use `with input as { ... }` to inject fixtures; never read the real
  filesystem from a policy or test.
- Both `violation` and `warning` rule sets are recognised by the gate;
  `violation` becomes severity `error` by default, `warning` becomes severity
  `warning`.

## Minimum tests per policy

Each policy must ship at least:

1. **Positive case** — input that should produce a violation, asserted with
   `count(<policy>.violation) > 0`.
2. **Negative case** — input that should pass, asserted with
   `count(<policy>.violation) == 0`.
3. **Threshold case** — if the policy has tunables (e.g. `max_files`), one test
   that drives the boundary.

`coverage_min_test.rego` and `security_baseline_test.rego` in
`policies/fixtures/` are the reference shape.

## Running tests locally

### Direct OPA

```bash
opa test policies/fixtures            # quiet
opa test policies/fixtures --verbose  # per-test PASS/FAIL lines
```

`PASS: N/N` with no `FAIL` lines is the success condition. The integration tests
assert exactly that.

### Via the TS executor

```bash
pnpm install --frozen-lockfile
pnpm -F @eddacraft/anvil-policy build
pnpm -F @eddacraft/anvil-policy exec vitest run src/opa-real.integration.test.ts
```

The suite skips automatically when `opa` is not on `PATH` and `ANVIL_OPA_PATH`
is unset.

### Via the Rust executor

```bash
cargo test -p eddacraft-anvil-policy --test opa_real_binary
```

Same skip behaviour.

### Current real-binary coverage

The historical TypeScript gate-pipeline integration test moved under
`archive/anvil-ts-scanner/` when the TypeScript scanner/runtime gate was
retired. Current real-binary coverage is the direct OPA fixture suite, the
TypeScript policy executor suite, and the Rust policy executor suite above.

## OPA binary version

The pinned version lives in **one** place:
`packages/anvil/policy/src/opa-binary-manager.ts` (`DEFAULT_OPA_VERSION`).
Currently `1.16.1`. CI installs the same version via
[`open-policy-agent/setup-opa`](https://github.com/open-policy-agent/setup-opa)
in `.github/workflows/ci.yml`, `.github/workflows/ci-nightly.yml`,
`.github/workflows/rust.yml`, `.github/workflows/rust-tests.yml`, and
`.github/workflows/poleng-parity.yml` (the POLENG-008 regorus-vs-Go-OPA parity
gate).

To bump:

1. Update `DEFAULT_OPA_VERSION` in `opa-binary-manager.ts`.
2. Update the `version:` input (and `EXPECTED_OPA_VERSION` env where present) in
   all five workflows. Re-run `scripts/bench-vs-go-opa.sh` and refresh the
   POLENG-008 parity result note in `plans/archive/modules/policy-engine.aps.md`
   if the reference OPA version changed.
3. Update any other files in the allowlist in `scripts/check-opa-version-pin.sh`
   (e.g. doc comments, AGENTS.md).
4. **If you rename, add, or remove any of the files listed above, edit the
   `ALLOWLIST` block in `scripts/check-opa-version-pin.sh` to match.** The guard
   only catches _unknown_ references; a stale allowlist entry will not fail CI
   and a missing entry will fail spuriously.
5. Run `./scripts/check-opa-version-pin.sh` locally — it fails the build if the
   pinned version string appears in any file not in the allowlist, which is the
   canary against silent doc rot when this runbook rots.
6. Run the direct OPA fixture suite plus both real-binary integration suites
   locally (TS executor and Rust executor).
7. Note the bump in the relevant ADR / decision log entry if the version change
   is load-bearing for a policy.

## Adding a new policy pack

1. Decide on a package name: `anvil.policies.<name>`.
2. Create `policies/fixtures/<name>.rego` with `violation`/`warning` rules.
3. Create `policies/fixtures/<name>_test.rego` covering positive, negative, and
   threshold cases (see "Minimum tests per policy").
4. Run `opa test policies/fixtures` — must show `PASS: N/N` with `N` strictly
   greater than before.
5. Add an integration assertion if the policy is part of the always-on fixture
   set: extend `packages/anvil/runtime/src/gate/policy.integration.test.ts` and
   the TS + Rust executor suites so a real run against your `.rego` produces the
   expected violation/passing case.
6. If the policy ships in a downstream user pack rather than the fixture set,
   document the input schema your rules expect (`input.plan.*`,
   `input.context.*`, `input.architecture.*`, `input.config.*`).

## Input schema reference

`PolicyCheck.buildOPAInput` provides:

- `input.plan` — id, hash, intent, schema_version, proposed_changes (each with
  `type`, `path`, `description`, `metadata`, derived `extension` and
  `directory`), provenance, validations, tags, `change_count`,
  `affected_directories`.
- `input.context` — `workspace_root`, `timestamp`, optional `git` (branch,
  base_branch, commit_sha, author, author_email), optional `ci` (provider,
  build_id, pr_number, pr_author).
- `input.architecture` — populated by `ArchitectureCheck` when it runs upstream
  of policy: `layers`, `boundaries`, `dependencies`, `summary`, `violations`.
- `input.config` — the per-check config from `gate.yaml`.

Policies should use the **most specific** field they need; reading
`input.plan.proposed_changes` directly is safe and stable, but reaching into
`input.context.git.branch` only makes sense when the gate caller includes git
context (`include_git_context: true`, the default).

## Troubleshooting

| Symptom                                                           | Cause                                                                                            | Fix                                                                                       |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------- |
| Integration tests skip silently                                   | `opa` not on `PATH` and `ANVIL_OPA_PATH` unset                                                   | Install OPA at the pinned version, or set `ANVIL_OPA_PATH=/abs/path/to/opa`.              |
| `EOF while parsing a value` from Rust executor                    | Output not piped from spawned `opa eval` (regression of fix in `crates/anvil-policy/src/opa.rs`) | Re-check `evaluate()` builds child with `.stdout(Stdio::piped()).stderr(Stdio::piped())`. |
| Policy loaded but never fires                                     | Package name mismatch (`anvil.policies.<X>` ≠ filename `<X>.rego`)                               | Rename file or package so they agree.                                                     |
| Tests pass with `opa test` but gate says "no policies configured" | Policy dir resolved relative to wrong workspace root                                             | Set `policy_dir` to a path that exists under the workspace passed to `runGate`.           |
