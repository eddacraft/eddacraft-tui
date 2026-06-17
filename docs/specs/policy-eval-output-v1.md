# Policy Eval Output Contract — `anvil policy eval --json` v1

| Type | Authority     | Owner                                                                                                                    | Status | Freshness                                                                                          |
| ---- | ------------- | ------------------------------------------------------------------------------------------------------------------------ | ------ | -------------------------------------------------------------------------------------------------- |
| Spec | Authoritative | CIB ([`plans/modules/continuous-improvement-backlog.aps.md`](../../plans/modules/continuous-improvement-backlog.aps.md)) | Live   | Last reviewed 2026-06-17 against `main`; frozen at v1 by CIB-078 before EVAL binds (EVAL-001/-002) |

| Upstream                                                                                                                                                   | Downstream                                                                                                                                                                    |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/anvil-cli/src/commands/policy/eval.rs` (`EvalOutput` — the serialisation site), `crates/anvil-policy-engine/src/result.rs` (`Finding`, `Severity`) | EVAL harness adapter ([`plans/modules/eval-harness-integration.aps.md`](../../plans/modules/eval-harness-integration.aps.md)), CI gates that parse `anvil policy eval --json` |

**Version:** 1.0.0 **Status:** Live **Created:** 2026-06-17 **Last Updated:**
2026-06-17

---

## Purpose

`anvil policy eval --json` emits a single JSON document describing the outcome
of a Rego evaluation: the policy and query that ran, the findings produced, and
the process exit code. POLENG-007 shipped this surface preview-gated, with the
wire shape explicitly **not yet a stable contract**. The
[eval-harness-integration](../../plans/modules/eval-harness-integration.aps.md)
module (EVAL) is about to bind an adapter to it (`EvalRunSummary`,
`EvalRegressionReport`). This spec freezes the shape at **v1 before EVAL locks
onto it**, so the adapter has a durable contract and a later eval-output
refactor cannot silently break trust-regression gates.

This is the output dual of the [`PolicyInput` v1 contract](policy-input-v1.md):
that spec pins what enters the engine, this one pins what comes out.

The authoritative definition is the Rust `EvalOutput` type in
`crates/anvil-cli/src/commands/policy/eval.rs`. This document describes the JSON
wire format that type serialises to; the schema-stability snapshot test
(`commands::policy::eval::tests::eval_output_schema_stability_snapshot`) pins
that wire format so an accidental change to the gate-critical fields fails CI.

## Design principles

- **Frozen core, evolving diagnostics.** v1 freezes only the **gate-critical**
  fields a CI gate or eval harness must rely on: `schema_version`, `policy`,
  `query`, `exit_code`, and the `findings` array (including the `Finding` and
  `Severity` shapes). The **diagnostic** fields — `value`, `coverage`, `trace`,
  `why` — are intentionally **not** part of the stability contract and may
  change shape without a major-version bump. They are human/debugging aids
  (`--explain`, `--why`), not decision inputs. The snapshot fixture omits them
  so the test pins the frozen surface and nothing more.
- **Versioned in-band.** Unlike `PolicyInput` (consumed by Rego, which branches
  with string equality, so it carries a major-only `"v1"`), this output is
  consumed by a programmatic adapter that can do an ordered semver comparison —
  so `schema_version` carries the full `"1.0.0"` and is emitted **first**,
  before any field a consumer might fail to parse.
- **Deterministic.** Field order is fixed by the struct definition. `findings`
  preserves engine order. The `exit_code` _value_ is computed according to
  ADR-002 (warnings never block) / ADR-003 (new-edges-only); the field itself is
  frozen by this contract.
- **No surprise keys on the happy path.** A clean evaluation serialises
  `schema_version`, `policy`, `query`, an empty `findings` array, and
  `exit_code`. Diagnostic fields are omitted when absent
  (`skip_serializing_if`), so a consumer never has to guard against `null`
  `coverage`/`trace`.

## Schema

### Top level — `EvalOutput` (frozen)

| Field            | JSON type          | Stability  | Description                                                                                          |
| ---------------- | ------------------ | ---------- | ---------------------------------------------------------------------------------------------------- |
| `schema_version` | string             | **frozen** | Contract version of this document. `"1.0.0"` for this revision. Emitted first.                       |
| `policy`         | string             | **frozen** | The policy file that was evaluated (display path).                                                   |
| `query`          | string             | **frozen** | The Rego query that was run (e.g. `data.anvil.arch.findings`).                                       |
| `findings`       | array of `Finding` | **frozen** | Post-processed findings (ADR-002/003 annotated). Empty array, never omitted, when none.              |
| `exit_code`      | integer            | **frozen** | Process exit code: `0` pass, non-zero block. Mirrors the process exit (ADR-002).                     |
| `value`          | any, optional      | diagnostic | Raw query result for **non-findings** queries (a scalar, object, or scalar list). Omitted otherwise. |
| `why`            | integer, optional  | diagnostic | Finding index `--why` focused on, echoed for consumers. Omitted unless `--why` was passed.           |
| `coverage`       | object, optional   | diagnostic | Rego line coverage when `--explain` was passed. Shape may evolve.                                    |
| `trace`          | object, optional   | diagnostic | Evaluation trace when `--why` was passed. Shape may evolve.                                          |

> **Diagnostic fields are not a contract.** A consumer that gates on `coverage`
> or `trace` shape does so at its own risk; those fields can change without a
> major-version bump. Bind decisions to `exit_code` and `findings` only.

### `findings[]` — `Finding` (frozen)

The first block of fields is supplied by the policy; `is_new_edge` and
`baselined` are computed by the engine's post-processing (ADR-003) and default
to `false` on a raw finding.

| Field         | JSON type        | Description                                                                         |
| ------------- | ---------------- | ----------------------------------------------------------------------------------- |
| `severity`    | string           | `Severity` wire form: `"warning"` (default, never blocks) or `"error"` (blocks).    |
| `message`     | string           | Human-readable description of the finding.                                          |
| `from`        | string, optional | Importer side of the dependency edge this finding concerns, if any.                 |
| `to`          | string, optional | Imported side of the dependency edge this finding concerns, if any.                 |
| `fingerprint` | string, optional | Baseline fingerprint of this finding, if it has one.                                |
| `is_new_edge` | boolean          | Computed: the finding concerns an edge introduced by the change set (ADR-003).      |
| `baselined`   | boolean          | Computed: the fingerprint is in the baseline cohort, so it is suppressed (ADR-003). |

The optional fields (`from`, `to`, `fingerprint`) are **omitted when absent, not
serialised as `null`**. Consumers must guard with a key-presence check, not a
null check.

### `Severity` (frozen)

| Wire value  | Meaning                                                                                                         |
| ----------- | --------------------------------------------------------------------------------------------------------------- |
| `"warning"` | Advisory. Never blocks unless `--fail-on-warnings`. The default (ADR-002).                                      |
| `"error"`   | Blocking finding. Produces a non-zero exit code unless baselined. Use for violations that must not pass a gate. |

## Example

A blocking evaluation with two findings (this is the schema-stability snapshot
fixture — the frozen surface, with diagnostic fields omitted). The second
finding has no edge or fingerprint, showing that those optional fields are
omitted rather than serialised as `null`:

```json
{
  "schema_version": "1.0.0",
  "policy": "policies/arch_boundary.rego",
  "query": "data.anvil.arch.findings",
  "findings": [
    {
      "severity": "error",
      "message": "import crosses an architecture boundary",
      "from": "crates/app/src/ui.rs",
      "to": "crates/app/src/db.rs",
      "fingerprint": "a1b2c3d4",
      "is_new_edge": true,
      "baselined": false
    },
    {
      "severity": "warning",
      "message": "module lacks an owner annotation",
      "is_new_edge": false,
      "baselined": false
    }
  ],
  "exit_code": 1
}
```

A clean evaluation keeps the frozen keys with an empty findings array:

```json
{
  "schema_version": "1.0.0",
  "policy": "policies/arch_boundary.rego",
  "query": "data.anvil.arch.findings",
  "findings": [],
  "exit_code": 0
}
```

## Versioning and deprecation policy

The contract follows semantic versioning at the document level. `schema_version`
carries the full `major.minor.patch`; the **major** component is what consumers
gate on.

- **Additive (minor) — non-breaking.** Adding a new optional top-level field, a
  new optional `Finding` leaf, or a new `Severity` string is non-breaking;
  consumers that do not read the new field are unaffected. Bump the minor
  (`1.1.0`), update this spec and the snapshot in the same change.
- **Diagnostic-field changes — non-breaking.** Changing the shape of `value`,
  `coverage`, `trace`, or `why` does **not** bump the major version; those
  fields are outside the stability contract. Update the snapshot only if the
  change alters the frozen-surface fixture (it should not).
- **Breaking (major).** Renaming or removing a frozen field, changing a frozen
  field's JSON type, changing `Severity` wire values, or changing the meaning of
  `exit_code` bumps the major to `2.0.0`. v1 is retained and marked deprecated
  for at least one minor release of the engine so consumers can migrate.
- **Deprecation window.** A frozen field slated for removal is documented as
  _Deprecated_ here one release before removal, with the replacement named. The
  snapshot test makes any unplanned change loud.
- **Process.** Any change to the frozen surface updates (1) `EvalOutput` /
  `Finding` in code, (2) the schema-stability snapshot, (3) this spec, and (4)
  `EVAL_OUTPUT_SCHEMA_VERSION` when the version moves — in the same PR. A change
  that touches only some of these is a contract-drift bug.

## Out of scope for v1

- The plain (non-`--json`) rendering — `render_plain` is a human surface with no
  stability guarantee.
- The `coverage` / `trace` internal shapes — owned by POLENG-006; diagnostic,
  not contract (see above).
- EVAL's own normalised types (`EvalRunSummary`, `EvalRegressionReport`) — those
  are defined by the EVAL module and consume this contract; they are not part of
  it.
