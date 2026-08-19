---
id: overview
title: What anvil does
description: Understand what anvil checks, when it runs, and where to begin.
sidebar_position: 1
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/main.rs
verified_against: 0.9.1-beta
---

# What anvil does

**For:** developers and teams using AI-assisted or conventional coding workflows

**Time:** 3 minutes

**Outcome:** decide whether anvil belongs in your workflow and know the next
step

anvil is a local-first code-governance tool. It checks new code changes for
unsafe shortcuts, secrets, architecture drift, and policy violations while the
change is still easy to fix.

You do not need to know anvil terminology before starting.

## The problem it solves

Fast code generation increases review volume. Tests can prove that code runs
without proving that it still respects your architecture, avoids broad
suppressions, or follows your team's policies.

anvil adds deterministic checks at the points where code changes:

1. **Before an AI tool writes**, when a supported client is connected.
2. **When a file is saved**, through the local watcher.
3. **Before a commit or push**, through optional Git hooks.
4. **In continuous integration**, through a gate command.

The same input produces the same findings. Source analysis happens locally.

## Three words to know

- A **check** evaluates one concern, such as a secret or broad lint suppression.
- A **finding** is a result produced by a check.
- A **gate** combines checks into a workflow decision.

The [glossary](concepts/glossary.md) defines every other term used in these
docs.

## What anvil changes

anvil can create project configuration, a baseline of existing findings, Git
hooks, and editor connection settings when you ask it to. The quickstart tells
you before each state-changing command.

anvil does not replace your compiler, tests, linter, code review, or deployment
system. It adds another evidence layer around them.

## Local and network activity

Code scanning and findings stay on your machine. Network access is used for
tasks that inherently need it, such as signing in, checking for updates, or
using authenticated services when you explicitly request them. `0.9.1-beta` and
later can also send a narrow anonymous usage beacon after showing its first-run
notice. It is opt-out, contains no source or free-form data, and has hard-off
controls.

Read [local data and security](operations/security.md) for the complete boundary
and [anonymous usage telemetry](operations/telemetry.md) for the payload,
timing, retention, and off controls.

## Two ways to use these docs

**Run it.** Start with [install and get first value](quickstart.md) or the
[ten-minute protection tutorial](first-gate.md).

**Look it up.** Start with
[how anvil evaluates a project](concepts/evaluation-model.md) and the short
[what anvil can do](reference/what-anvil-can-do.md) index. Then use the
[CLI reference](reference/cli.md),
[compiled pattern catalogue](reference/rules.md), and
[supported platforms and languages](reference/support.md). The
[glossary](concepts/glossary.md) defines the words used here.

## Where to begin

- New to anvil: [install and get first value](quickstart.md).
- Deciding whether it fits: [when to use anvil](when-to-use.md).
- Looking up the product model:
  [how anvil evaluates a project](concepts/evaluation-model.md) or
  [what anvil can do](reference/what-anvil-can-do.md).
- Looking up a command or rule: the [CLI reference](reference/cli.md) or
  [compiled pattern catalogue](reference/rules.md).
- Already installed: [run the ten-minute protection tutorial](first-gate.md).
- Already activated: run bare `anvil` for the daily ensure path.
- Invited tester: [beta test brief](beta-testing-guide.md).
