---
id: security
title: Local data and security
description:
  Understand what anvil reads, writes, stores, and sends over the network.
---

# Local data and security

anvil is local-first: source analysis and normal findings are produced on your
machine.

## What anvil reads

Depending on the command, anvil may read:

- source files selected by the command;
- Git metadata and changed-file lists;
- project configuration and architecture definitions;
- the existing baseline and retained local evidence; and
- supported AI-client configuration during activation or verification.

Use a read-only command such as `anvil start --verify` when you need diagnosis
without setup changes.

## What anvil writes

State-changing commands can write:

- project configuration and baseline files;
- anvil-managed Git hooks;
- local evidence and caches;
- user credentials;
- daemon state; and
- supported client configuration.

The command help and guide should tell you before a write occurs.

## Network activity

Network access is used for installation, authentication, licence refresh,
updates, and public feedback. Local checks do not need to upload source code.
The current beta records local command-usage observations for local insights;
set `DO_NOT_TRACK=1` for a process that must not write that local usage record.

`0.9.1-beta` and later can also send a disclosed opt-out anonymous usage beacon
after an eligible interactive first run has shown its notice. This beacon is
separate from local observations and excludes source code, paths, command
arguments, findings, output, and free-form text. Read
[anonymous usage telemetry](telemetry.md) for its complete payload, timing,
transient IP processing, retention, and the `anvil telemetry off`,
`ANVIL_TELEMETRY=off`, and `DO_NOT_TRACK=1` controls.

## AI-client and source-snippet egress

Graph context shared through a configured AI client is **identity-only by
default**: symbol names, file locations, and relationships can be returned, but
source snippets are withheld. Sending a matched source snippet requires both an
explicit request from the client and per-workspace operator consent.

Inspect the effective state without changing it:

```text
anvil gctx egress status
```

Enable snippet egress only after reviewing the connected assistant or model
provider's own data policy:

```text
anvil gctx egress enable
```

The consent is stored for that workspace. Revoke it and return to identity-only
results with:

```text
anvil gctx egress disable
```

`ANVIL_GCTX_EGRESS=0` is a process-level kill switch that keeps snippet egress
off even when workspace consent exists. Once a snippet reaches an AI client, the
client's provider controls any onward network processing and retention; anvil's
local-first boundary does not extend into that provider.

## Sharing output

Logs, JSON, SARIF, scorecards, and review capsules can contain paths, commit
metadata, rule IDs, and excerpts. Review them before sharing and never publish
credentials or private source.

## Remove state

Use [uninstall and clean up](uninstall.md) to preview project-only or global
removal.
