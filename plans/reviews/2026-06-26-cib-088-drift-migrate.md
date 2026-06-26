# CIB-088 Mini Council — Drift Migrate Operability

| Type   | Authority | Owner | Status |
| ------ | --------- | ----- | ------ |
| Review | Advisory  | CIB   | Closed |

## Scope

- Work item: CIB-088
- Target: `crates/anvil-cli/src/commands/drift.rs`
- Review tier: mini
- Roles: operations, adversarial
- Date: 2026-06-26

## Verdict

**WARN → conditions addressed in implementation.**

The reviewers agreed the existing backup creation was safe from clobbering, but
`drift migrate` could still silently succeed after skipping corrupt,
unreadable, or future-version snapshots. Backup retention was also comment-only.

## Findings and resolution

| Severity | Role | Finding | Resolution |
| -------- | ---- | ------- | ---------- |
| Major | Operations | Skipped baselines were warned to stderr and omitted from `MigrateReport`. | Added structured skipped counts by reason and a `partial` report flag. |
| Major | Operations | Partial migrations exited 0 and were invisible to CI. | `run_migrate` now prints the report and returns `AlreadyReported`, producing exit 1 for partial runs. |
| Major | Adversarial | Backup pruning must be explicit and strictly matched. | Added `--prune-backups`; it prunes only exact `<snapshot>.bak` / `<snapshot>.bak.N` chains for live snapshots. |
| Major | Adversarial | Pruning must preserve a current rollback. | Prune runs before migration and retains the latest backup generation per live snapshot; fresh migration backups survive. |
| Major | Operations | Snapshot scan-cap omissions were silent partials. | The migrate listing path now reports ignored snapshots as `scan_limit_exceeded`. |

## Evidence

- `cargo test -p eddacraft-anvil migrate_`
- `cargo test -p eddacraft-anvil prune_`
- `cargo test -p eddacraft-anvil --test drift_migrate`
- `cargo clippy -p eddacraft-anvil --all-targets -- -D warnings`
- `cargo fmt --check`
- `pnpm aps:index:check`
- `pnpm aps:active-lint`
- `pnpm docs:check`

## Notes

The implementation deliberately avoids automatic backup deletion. Operators and
CI must pass `--prune-backups` to apply count-based retention. Orphan backup
garbage collection is not part of CIB-088; only backups tied to a live
`snapshot-*.json` baseline are eligible.
