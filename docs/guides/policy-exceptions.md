# Policy Exceptions

| Type  | Authority     | Owner  | Status | Freshness                                                                                                                 |
| ----- | ------------- | ------ | ------ | ------------------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | EXCEPT | Live   | Last reviewed 2026-07-04 against `crates/anvil-policy/src/exceptions.rs` and `crates/anvil-cli/src/commands/exception.rs` |

| Upstream                                                                                                                                     | Downstream                                                                 |
| -------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| `crates/anvil-policy/src/exceptions.rs`, `crates/anvil-cli/src/commands/exception.rs`, `plans/decisions/073-durable-vs-local-anvil-state.md` | `anvil exception` CLI workflows, L4 gate suppression, capsule verification |

This guide explains the operator contract for tracked policy exceptions: what
the store is, how to change it, what to commit, and how exceptions behave at the
gates.

## What an Exception Is

A policy exception is a **scoped, expiring, attributed, revocable** record that
suppresses a specific policy finding. It is the file-based sibling of the inline
`@anvil-ignore` suppression: where `@anvil-ignore` lives next to the offending
line, an exception lives in a repo-tracked store and can cover a glob of paths
for a bounded time.

Exceptions live in `anvil/exceptions/store.json`. The file is **tracked** — it
travels with the repository and every grant or revocation is visible in PR
review, exactly like `anvil/baseline.json`.

## Commit the Store

Treat `anvil/exceptions/store.json` like `anvil/baseline.json`: commit it, and
review changes to it like code. A grant is suppression authority — the review of
the PR that adds it is the approval step.

Writes are **explicit-only**. The store changes when a human runs
`anvil exception grant`, `anvil exception revoke`, or `anvil exception migrate`
— never as a side effect of a check, gate, or scan. If your worktree shows an
unexpected change to the store, someone (or some tool acting as someone) ran an
explicit write; checks never dirty it.

## Managing Exceptions

```
$ anvil exception grant --policy AP-001 --reason "legacy module, removal scheduled" \
    --scope "src/legacy/**" --expires-in-days 30
$ anvil exception list
$ anvil exception show <id>
$ anvil exception revoke <id> --reason "module removed"
$ anvil exception verify
```

Ground rules the CLI enforces and why:

- **Attribution is required.** A grant records who created it (`--owner` and/or
  your git identity). The store refuses unattributed grants — an anonymous
  suppression is not reviewable. Attribution is advisory: it comes from local
  git config, so reviewers should check it against PR authorship, exactly as
  with commit authors.
- **Prefer the narrowest scope and a real expiry.** An empty scope covers every
  file; a grant without `--expires-in-days` never lapses. The CLI warns on both.
  Time-bound, tightly-scoped grants are the ones that age well.
- **Revocation is a soft delete.** The record stays in the store with a
  revocation audit trail. Do not hand-delete records from the store — a vanished
  record reads as a rewritten store and degrades capsule verification.
- **Exit codes are honest.** A write that did not persist (read-only checkout,
  permission misconfiguration) warns and exits non-zero; re-run with `--verbose`
  for the underlying error.

Subcommand flags are documented in the CLI surface runbook
([cli-surface](../runbooks/cli-surface.md)).

## What Exceptions Do at the Gates

The L4 gate (pre-push hook and `anvil l4-validate`) applies only **valid**
grants:

- An attributed, in-date, in-scope grant suppresses its matching finding.
- An **unattributed** grant (legacy v0 shape) is never silently honoured: the
  finding stays visible, downgraded to a warning annotated with the grant id.
- Revoked, expired, or out-of-scope grants leave findings standing.
- A broken store fails safe: findings stand.

Governance capsules collect the active grants at create time and re-verify them
— against both the capsule snapshot and the live store — during
`anvil capsule verify`. Revoking a grant after a capsule was created blocks that
capsule's verification: the deviation the change relied on no longer holds.

## Merging Branches That Both Grant

Two branches that each add a grant will conflict on
`anvil/exceptions/store.json`. Do **not** configure `.gitattributes`
`merge=union` for this file: union merge concatenates conflicting lines and will
corrupt pretty-printed JSON (and can silently duplicate or interleave records in
a governance file — worse than a visible conflict).

Resolve the conflict by keeping **both** grants: accept both entries in the
`exceptions` array, or take one side and re-run `anvil exception grant` for the
other. `anvil exception verify` afterwards confirms the merged store parses and
every grant classifies cleanly; the CLI refuses duplicate ids, so an accidental
double-entry is caught at the next write.

## Upgrading From the Legacy Store

Repositories that used exceptions before ADR-073 have a local, gitignored
`.anvil/exceptions.json`. Those grants keep working read-only, but grant and
revoke refuse to modify a legacy-origin store until you promote it —
`anvil exception migrate` is the promotion path:

```
$ anvil exception migrate
$ git add anvil/exceptions/store.json && git commit
```

Migration copies the legacy records into the tracked store and leaves the legacy
file in place; remove it once the tracked store is committed. The promotion is
deliberately explicit — local-only suppressions becoming repo-visible is a
review event, not a silent upgrade.
