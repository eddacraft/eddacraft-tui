# Ember Candidate Review Workflow

| Type  | Authority | Owner   | Status | Freshness                                    |
| ----- | --------- | ------- | ------ | -------------------------------------------- |
| Guide | Derived   | EMBERRS | Live   | Last reviewed 2026-09-05 against EMBERRS-001 |

| Upstream                                                                                                     | Downstream                                |
| ------------------------------------------------------------------------------------------------------------ | ----------------------------------------- |
| `crates/anvil-cli/src/commands/ember.rs`, `flags/manifest.json`, `plans/modules/ember-rust-migration.aps.md` | CLI users and Rust migration contributors |

## Current availability

Ember is inactive by default. Candidate generation, evaluation, decay and
observation hooks are not running. The old TypeScript implementation is retired;
there is no JavaScript fallback. The full capability is tracked in the
[Rust migration module](../../plans/modules/ember-rust-migration.aps.md).

The existing Rust reader can inspect historical `.anvil/ember.db` proposals. It
is hidden from normal CLI discovery and controlled by `ember.enabled`.
`anvil ember list` returns exit 1 while disabled; either placement of `--json`
returns a `feature_disabled` document naming the flag before database access.
Existing databases are not modified or deleted.

## Explicit historical inspection

POSIX shells:

```bash
ANVIL_EMBER=1 anvil ember list --json
```

PowerShell:

```powershell
$env:ANVIL_EMBER = '1'
anvil ember list --json
Remove-Item Env:ANVIL_EMBER
```

The opt-in enables only the Rust reader. It does not create a database, start
candidate processing or enable promotion. A missing database is an explicit
error, not proof that the pipeline ran and found no candidates. `ANVIL_DEV=1`
and admin credentials do not enable Ember. Unset `ANVIL_EMBER` or set it to `0`
to disable the reader again.

## Intended migration outcome

Kindling observations become bounded, replay-safe candidate proposals in Rust.
Candidates retain source provenance and expire under a defined lifecycle. People
can review, dismiss or explicitly promote them into Edda; Ember itself never
authorises durable memory. Storage compatibility, publication recovery, review
commands and Windows/Linux/macOS verification are acceptance criteria in
EMBERRS, not available capabilities implied by the historical TS plans.
