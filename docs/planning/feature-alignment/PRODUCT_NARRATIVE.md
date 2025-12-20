# PRODUCT_NARRATIVE — Anvil

## Headline
Ship AI-generated code with confidence.

## Subheading
Anvil is a developer-first safety layer that helps you use AI at full speed without breaking your system’s architecture or intent.

## Core problem
AI generates code faster than humans can review it. The failures that hurt most are the ones that look correct but violate deep constraints:
- boundary erosion (cross-context calls)
- escape-hatch anti-patterns (`eslint-disable`, `any`, broad suppressions)
- “technically valid but wrong” abstractions

## What Anvil does
Anvil sits between AI and production:
- warns in-flow (ideally at file save)
- explains what’s wrong and why it matters
- suggests safer alternatives
- records human intent when exceptions are necessary
- mirrors warnings later in PR/CI as a fail-safe

## What Anvil is (and is not)
**Anvil is** a trust broker for AI ↔ human collaboration.
**Anvil is not** just another linter, CI rules engine, documentation replacement, or a process framework you must adopt to get value.
