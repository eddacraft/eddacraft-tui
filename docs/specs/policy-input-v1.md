# Policy Input Contract — `PolicyInput` v1

| Type | Authority     | Owner                                                                                     | Status | Freshness                                                                    |
| ---- | ------------- | ----------------------------------------------------------------------------------------- | ------ | ---------------------------------------------------------------------------- |
| Spec | Authoritative | POLENG ([`plans/modules/policy-engine.aps.md`](../../plans/modules/policy-engine.aps.md)) | Draft  | Last reviewed 2026-05-25 against `main`; implementation landed by POLENG-002 |

| Upstream                                                                                              | Downstream                                                                                                               |
| ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `crates/anvil-policy-engine/src/input.rs` (`PolicyInput` and member types — the authoritative source) | Rego policies (CPACKS packs, architecture boundary rules), `anvil policy eval` (POLENG-007), POLENG-003 builtins surface |

**Version:** 1.0.0 **Status:** Draft (POLENG-002) **Created:** 2026-05-25 **Last
Updated:** 2026-05-25

---

## Purpose

Every Anvil policy evaluation receives a single JSON document as the Rego
`input`. POLENG-001 shipped the engine facade with an empty placeholder input;
this spec pins the v1 shape so policy authors and downstream crates have a
durable contract to bind against.

`PolicyInput` v1 is a **stability contract**. Rego policies reference fields by
path (`input.repo_state.files`, `input.diff.new_edges`,
`input.baseline.findings`, …). Renaming or restructuring a field is a breaking
change governed by the deprecation policy below, not a free refactor.

The authoritative definition is the Rust type in
`crates/anvil-policy-engine/src/input.rs`. This document describes the JSON wire
format that type serialises to; the schema-stability snapshot test
(`input::tests::schema_stability_snapshot`) pins that wire format so an
accidental change to the Rust struct fails CI.

## Design principles

- **Self-describing.** The document mirrors the shapes of
  `anvil_kernel::graph::DependencyGraph` (files + edges) and
  `anvil_baseline::BaselineFinding` (rule_id / file_path / fingerprint) without
  taking a crate dependency on either. The input contract can be constructed,
  serialised, and snapshot-tested in isolation, and stays decoupled from
  internal kernel refactors.
- **No missing keys.** A defaulted `PolicyInput` still serialises every
  top-level key with an empty collection, so policies never have to guard
  against `undefined` on `input.repo_state`, `input.diff`, etc. Optional _leaf_
  fields (`PlanFile.id`, `DecisionEntry.title`) are omitted when absent.
- **Deterministic.** Field order is fixed by the struct definition; collections
  preserve caller-supplied order, and the input document itself carries no
  clock, environment, or filesystem state. POLENG-004 makes Anvil's first-party
  `anvil.*` builtins pure; note that impure Rego stdlib builtins (`time.now_ns`,
  `rand.*`, `uuid.*`) remain reachable from policy text until POLENG-009 fences
  them, so byte-identical evaluation is guaranteed only for policies that avoid
  them.
- **Versioned at the root.** `schema_version` lets a policy branch defensively
  (`input.schema_version == "v1"`) and lets future revisions coexist.

## Schema

### Top level — `PolicyInput`

| Field            | JSON type       | Description                                       |
| ---------------- | --------------- | ------------------------------------------------- |
| `schema_version` | string          | Always `"v1"` for this revision.                  |
| `repo_state`     | object          | Repository structure: files and dependency edges. |
| `plans`          | array of object | APS plan files visible to policies.               |
| `decisions`      | array of object | Architecture decision record entries.             |
| `diff`           | object          | The change set under evaluation.                  |
| `baseline`       | object          | Pre-existing finding fingerprints (ADR-003).      |

### `repo_state` — `RepoState`

| Field   | JSON type                 | Description                                 |
| ------- | ------------------------- | ------------------------------------------- |
| `files` | array of string           | Repo-relative paths of known files.         |
| `edges` | array of `DependencyEdge` | Directed import edges: `from` imports `to`. |

### `diff` — `Diff`

| Field           | JSON type                 | Description                                                               |
| --------------- | ------------------------- | ------------------------------------------------------------------------- |
| `changed_files` | array of string           | Repo-relative paths changed by the evaluated change set.                  |
| `new_edges`     | array of `DependencyEdge` | Dependency edges introduced by the change set (ADR-003 "new edges only"). |

### `DependencyEdge`

| Field  | JSON type | Description                         |
| ------ | --------- | ----------------------------------- |
| `from` | string    | Repo-relative path of the importer. |
| `to`   | string    | Repo-relative path of the imported. |

### `plans[]` — `PlanFile`

| Field    | JSON type        | Description                                    |
| -------- | ---------------- | ---------------------------------------------- |
| `path`   | string           | Repo-relative path of the plan file.           |
| `id`     | string, optional | Module/work-item id when the file carries one. |
| `status` | string, optional | Plan status when known (e.g. `In Progress`).   |

### `decisions[]` — `DecisionEntry`

| Field    | JSON type        | Description                   |
| -------- | ---------------- | ----------------------------- |
| `id`     | string           | ADR id (e.g. `040`).          |
| `title`  | string, optional | ADR title.                    |
| `status` | string, optional | ADR status (e.g. `Accepted`). |

### `baseline` — `Baseline`

| Field      | JSON type                  | Description                            |
| ---------- | -------------------------- | -------------------------------------- |
| `findings` | array of `BaselineFinding` | Fingerprints of pre-existing findings. |

### `BaselineFinding`

| Field         | JSON type | Description                                                        |
| ------------- | --------- | ------------------------------------------------------------------ |
| `rule_id`     | string    | Rule that produced the finding.                                    |
| `file_path`   | string    | Repo-relative path the finding was attached to.                    |
| `fingerprint` | string    | Move-resilient digest (see `anvil_baseline::compute_fingerprint`). |

## Example

A fully-populated document (this is the schema-stability snapshot fixture):

```json
{
  "schema_version": "v1",
  "repo_state": {
    "files": ["src/app.rs", "src/db.rs"],
    "edges": [{ "from": "src/app.rs", "to": "src/db.rs" }]
  },
  "plans": [
    {
      "path": "plans/modules/policy-engine.aps.md",
      "id": "POLENG",
      "status": "In Progress"
    }
  ],
  "decisions": [
    {
      "id": "040",
      "title": "Adopt regorus as the Anvil Policy Engine",
      "status": "Accepted"
    }
  ],
  "diff": {
    "changed_files": ["src/app.rs"],
    "new_edges": [{ "from": "src/app.rs", "to": "src/db.rs" }]
  },
  "baseline": {
    "findings": [
      {
        "rule_id": "anti-pattern:guardrail-suppression",
        "file_path": "src/legacy.rs",
        "fingerprint": "f00dcafe12345678"
      }
    ]
  }
}
```

A defaulted document keeps every top-level key:

```json
{
  "schema_version": "v1",
  "repo_state": { "files": [], "edges": [] },
  "plans": [],
  "decisions": [],
  "diff": { "changed_files": [], "new_edges": [] },
  "baseline": { "findings": [] }
}
```

## Versioning and deprecation policy

The contract follows semantic versioning at the document level. `schema_version`
carries only the **major** component (`"v1"`); minor/patch evolution is tracked
in this spec's header and the snapshot.

- **Additive (minor) — non-breaking.** Adding a new top-level key, a new
  optional leaf field, or a new enum string is non-breaking. Policies that do
  not reference the new field are unaffected. `schema_version` stays `"v1"`. The
  schema-stability snapshot is updated in the same change.
- **Breaking (major).** Renaming or removing a field, changing a field's JSON
  type, or changing the meaning of an existing field bumps the major version to
  `"v2"`. A new struct (`PolicyInputV2`) is introduced; v1 is retained and
  marked deprecated for at least one minor release of the engine so packs can
  migrate. The engine continues to accept `schema_version == "v1"` policies
  during the deprecation window.
- **Deprecation window.** A field slated for removal is documented as
  _Deprecated_ in this spec one release before removal, with the replacement
  named. The snapshot test makes any unplanned change loud.
- **Process.** Any change to `PolicyInput` updates (1) `input.rs`, (2) the
  schema-stability snapshot, and (3) this spec — in the same PR. A change that
  touches only one of the three is a contract drift bug.

## Out of scope for v1

- Populating the document from live repository state — POLENG-003 (builtins) and
  the callers in POLENG-007 own the producers. v1 defines the shape and proves
  `regorus` can read it; it does not wire kernel data in.
- Per-builtin determinism declarations — POLENG-004.
- Severity / new-edge post-processing of _results_ — POLENG-005 (note the
  distinction: `diff.new_edges` here is _input_; result annotation is separate).
