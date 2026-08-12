---
id: overview
title: What APS does
description:
  Understand how APS turns development intent into bounded, verifiable work.
sidebar_position: 1
owner: DOCSYNC
---

# What APS does

**For:** developers and teams who want reliable planning for human or
AI-assisted work

**Time:** 3 minutes

**Outcome:** decide whether APS fits your workflow and know where to begin

APS is a Markdown-based planning format and a local command-line tool. It keeps
the reason for a change, its boundaries, its dependencies, and its proof of
completion beside the code that will change.

You do not need to learn the full format before starting.

## The problem it solves

A chat request can explain what to build without defining what is authorised,
what must stay unchanged, or how anyone will prove the result. That ambiguity
grows when work crosses sessions, people, or AI tools.

APS gives the work a durable shape:

1. An **index** explains the overall problem and links its parts.
2. A **module** defines one bounded area of responsibility.
3. A **work item** authorises one observable outcome.
4. An optional **action plan** breaks complex execution into checkpoints.

The files remain ordinary Markdown. The `aps` CLI validates them, finds ready
work, enforces dependencies, records progress, and can audit plan claims against
the project.

## What APS changes

`aps init` creates a `plans/` directory and a small `.aps/config.yml` project
contract. Later commands read and update those Markdown files; APS does not keep
a second planning database.

APS does not replace your issue tracker, tests, code review, or source control.
It supplies the missing execution contract between an idea and those systems.

## What a first success looks like

After the quickstart you will have:

- a plan that passes `aps lint` with no issues;
- one work item in `Ready` state;
- `aps next` selecting that item; and
- a clear validation command to run before completion.

## Where to begin

- New to APS: [create and validate your first plan](getting-started.md).
- Already using APS: [run the day-to-day workflow](workflow.md).
- Choosing a project shape: [understand the document model](spec/taxonomy.md).
- Looking up commands: [use the CLI reference](tooling/validation.md).
- Planning a monorepo: [choose a monorepo tier](guides/monorepo.md).
