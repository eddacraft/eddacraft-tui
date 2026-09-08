---
name: agent-messaging
description: |
  File-based message bus for inter-agent communication; required sub-skill of
  council. Use when one agent must hand structured findings, questions, or
  alerts to another mid-run.
---

# Agent Messaging

A lightweight, file-based bus for agent-to-agent communication. Messages are
written to per-recipient queues on disk and read back on demand (pull-based —
there is no real-time delivery). Optional long-term persistence is captured
through any available observation store; the bus works fully without it.

## When to use

- A reviewer finds something the orchestrator (or a peer reviewer) needs
  before review ends — send it now rather than waiting for session close.
- An orchestrator needs to collect every reviewer's findings before it
  synthesises a verdict.
- Any fan-out workflow where sibling agents must share context without going
  back through the spawning agent.

## Runtime boundary

Use a discovered runtime adapter that declares `agent-messaging/v1` when one is
available. The required operations and state guarantees are defined in
`references/runtime-adapter-contract.md`; adapter command names and target paths
belong to the runtime binding, not this canonical skill.

The bundled scripts are the current POSIX fallback and require Bash plus `jq`.
On native Windows without a conforming adapter, report the capability as
unavailable. Do not silently invoke Git Bash and claim native support.

## Resolving fallback paths

Inside this skill, treat the bundle root as `${SKILL_DIR}`. All script paths
below are bundle-relative. Message state is stored under the consuming
project's bus directory (see [Storage](#storage)).

## Send a message

Prefer the file form so structured content does not depend on shell quoting:

```console
bash ${SKILL_DIR}/scripts/send-message.sh \
  --from <sender-agent> --to <recipient-agent> \
  --type <finding|question|recommendation|alert> \
  [--priority <low|medium|high|critical>] \
  [--negotiation-id <id>] \
  --payload-file <path-to-payload.json>
```

`--payload <json>` remains available for compatible shells but is not the
portable shared instruction.

Send a critical alert immediately — do not batch it to session end:

```console
bash ${SKILL_DIR}/scripts/send-message.sh \
  --from security-analyst --to council-reviewer \
  --type alert --priority critical \
  --payload-file <path-to-alert.json>
```

The script validates the type, priority, and JSON payload, stamps an `id` and
ISO `timestamp`, and appends the message to the recipient's queue plus a
global log. It prints the bare message id to stdout and the human-readable
`Message sent: ...` line to stderr. Capture stdout through the harness rather
than requiring command substitution.

## Receive messages

```console
bash ${SKILL_DIR}/scripts/receive-messages.sh <agent-name> \
  [--type <type>] [--from <agent>] [--priority <level>] \
  [--since <iso-timestamp>] [--limit <n>] \
  [--unread] [--mark-read] \
  [--format <json|summary>]
```

Orchestrator drains its inbox before synthesising:

```console
bash ${SKILL_DIR}/scripts/receive-messages.sh council-reviewer --format summary
```

`--unread` filters out messages already acknowledged; `--mark-read` records
the returned ids as read. Reads default to the most recent 50 messages.

## Trigger queue (optional)

For event-driven agent spawning, `check-queue.sh` produces, inspects, and
manages a pending-trigger queue (`agent-queue.json` in the bus directory):

```console
bash ${SKILL_DIR}/scripts/check-queue.sh --enqueue \
  --trigger <agent> --source <agent> \
  [--context "<text>"] [--priority <immediate|queued>]         # append a trigger
bash ${SKILL_DIR}/scripts/check-queue.sh                       # list pending triggers
bash ${SKILL_DIR}/scripts/check-queue.sh --priority immediate  # only immediate
bash ${SKILL_DIR}/scripts/check-queue.sh --pop                 # pop next trigger
bash ${SKILL_DIR}/scripts/check-queue.sh --mark-done <id>      # complete a trigger
```

`--enqueue` appends an `agentTrigger` record (see `references/schema.json`)
with `status: "pending"` and prints the new trigger id to stdout. `--pop`
returns the highest-priority pending trigger and marks it `processing`;
`--mark-done` marks it `completed`.

## Message schema

The wire format for messages, negotiation signals, and triggers is defined in
`references/schema.json` (JSON Schema draft-07). TypeScript type definitions
and constructor helpers are in `references/types.ts`. A message has:

| Field           | Required | Notes                                                  |
| --------------- | -------- | ------------------------------------------------------ |
| `id`            | auto     | `msg-<epoch>-<rand>`                                   |
| `from` / `to`   | yes      | sender / recipient agent names                         |
| `type`          | yes      | `finding` \| `question` \| `recommendation` \| `alert` |
| `priority`      | no       | `low` \| `medium` (default) \| `high` \| `critical`    |
| `payload`       | yes      | arbitrary JSON object with the message content         |
| `timestamp`     | auto     | ISO 8601 UTC                                           |
| `negotiationId` | no       | links the message to a negotiation session             |

## Storage

Messages are persisted to per-recipient JSONL queues plus an `all-messages`
log. State is **project-local, never skill-local**: the scripts store it
under `<project>/.agent-bus/`, resolving the project root via
`git rev-parse --show-toplevel` (falling back to the working directory); set
`AGENT_BUS_DIR` to relocate it (e.g. to a shared path when several agents
run from different projects). Add `.agent-bus/` to the consuming project's
`.gitignore`. Reads are pull-based and streamed (large queues are tailed,
not fully parsed).

Set `AGENT_BUS_OBSERVER` to an executable to also forward each sent message
to an external observation store for long-term persistence — invoked as
`<observer> message <json>`, best-effort and non-blocking. The file store is
canonical; the observer is additive and its absence never fails a send or
receive.

## Principles

1. **Pull-based, not real-time** — agents drain their inbox when they run;
   there is no push delivery or read receipts beyond the `--mark-read` marker.
2. **File store is canonical** — message state lives on disk and works with no
   external dependency. Any observation-store integration is additive.
3. **Critical alerts go immediately** — do not hold a `critical` finding until
   session end; send it the moment it is found.
4. **Portable means native evidence** — a POSIX fallback does not establish
   native Windows support.
