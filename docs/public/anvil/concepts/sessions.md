---
id: sessions
title: Runs and Daemon Sessions
description:
  Understand validation runs, daemon sessions, and the state anvil records
  today.
sidebar_position: 2
---

# Runs and Daemon Sessions

anvil uses the word _session_ for a live connection to its save-time daemon. A
session is not a task tracker or a manually managed development period; the
current Rust CLI does not expose a manual task-session command group.

Keeping that distinction clear makes the operational surfaces much easier to
reason about:

- a **validation run** is one execution of `check`, `gate`, `audit`, or a
  save-time validation;
- a **daemon session** is a registered client/worktree connection served by the
  resident daemon;
- a **protection state** says whether the current repository is actually
  protected;
- a **review capsule** packages commit-range evidence for offline verification.

## Protection state

Use the activation probe for the answer most developers need:

```bash
anvil status --verify
```

It is read-only and returns the same protection-state vocabulary as
`anvil start --verify`: `protecting`, `ready_restart_required`, `watching`,
`needs_action`, `unsupported`, or `error`.

Use JSON when a script needs the typed claim:

```bash
anvil status --json
```

Protection state is deliberately separate from daemon process health. A daemon
can be running without serving the current worktree, and a scoped embedded
fallback can validate a write while daemon assurance is unavailable.

## Validation runs

Choose the run that matches the question you are asking:

| Command                    | What it proves                                                            |
| -------------------------- | ------------------------------------------------------------------------- |
| `anvil check --all`        | Source files were scanned by the planless anti-pattern and secret checks. |
| `anvil gate --profile dev` | The configured development gate reached a workflow verdict.               |
| `anvil gate --profile ci`  | The configured CI gate reached a workflow verdict.                        |
| `anvil watch --source`     | Later source saves are checked after the initial readiness scan.          |
| `anvil audit-chain`        | Reachable commits have the expected witness records.                      |

Each command owns its output and exit semantics. A passing `check` is not a
substitute for a passing `gate`: `check` is targeted analysis, while `gate`
combines configured checks into the decision used by a workflow.

For machine consumers, use the command's JSON or SARIF surface rather than
scraping terminal copy. For long-running watch output, follow the
[NDJSON contract](../integrations/watch-output.md).

## Daemon sessions

The intercept daemon keeps live session records for attached worktrees and
clients. Inspect them with:

```bash
anvil intercept status
```

The human view summarises daemon uptime, active sessions, fences, and validation
latency. The JSON form exposes the same `DaemonStatusV1` shape on Unix and
Windows:

```bash
anvil intercept status --json
```

Use this surface when activation says the daemon is unreachable, a worktree is
fenced, or an editor appears not to be reaching the MCP shim. Do not treat the
session count as a task count: it describes daemon attachments, not developer
work items.

## Recorded local state

anvil records only the state needed by the feature that owns it. Important
examples include:

| State                    | Purpose                                                                                                      |
| ------------------------ | ------------------------------------------------------------------------------------------------------------ |
| `.anvil/`                | Project configuration, architecture state, drift snapshots, suppressions, dashboards, and local caches.      |
| `anvil/witness/`         | Git-tracked witness records used by the chain audit.                                                         |
| User-level Anvil state   | Credentials, daemon state, graph cache, and local usage evidence under the platform config/data directories. |
| Review capsule directory | An explicit, portable package created for a commit range.                                                    |

There is no general-purpose `.anvil/sessions/` archive and no public session
management database. Use the owning command instead of editing these files by
hand.

## Reviewing activity over time

The shipped Rust CLI provides focused views instead of a generic session log:

```bash
anvil insights
anvil insights --cumulative
anvil drift list
anvil audit-chain
```

- `insights` summarises retained local activity without sending telemetry;
- `drift` snapshots show architecture change over time;
- `audit-chain` checks commit-to-witness coverage;
- review capsules package the evidence for a bounded commit range.

For portable evidence and its verification model, continue to
[Audit Trail](./audit-trail.md).
