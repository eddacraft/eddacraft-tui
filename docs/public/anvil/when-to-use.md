---
id: when-to-use
title: When to Use anvil
description:
  Understand when anvil adds value and when other tools are more appropriate.
sidebar_position: 2
---

# When to Use anvil

anvil is powerful but focused. This page helps you understand when it's the
right tool.

:::tip Trying it now?

For the install-to-protection flow, run `anvil start` from your repo root —
it wires Cursor or Claude Code MCP entries and ends in one literal protection
state. See the [Quickstart](/anvil/quickstart) for the full walk-through.

:::

## anvil is for you if...

### You ship with AI assistance

anvil is designed for teams using AI coding tools (Copilot, Claude, Cursor,
etc.). If you're not using AI assistance, the ROI is lower—though the
architecture safety features still provide value.

### You care about architecture

anvil shines when you have intentional architectural boundaries:

- Layer separation (UI → Services → Data)
- Domain boundaries (Payments shouldn't import User internals)
- Package boundaries (Public API vs internal implementation)

### You want pre-write or save-time feedback

When the MCP path is wired (Cursor or Claude Code), anvil's
`anvil_validate_write` tool runs **before** the AI's writes. When MCP pre-write
attach is not available, watch mode is the save-time fallback. If you only
want CI-time validation, you can use anvil in CI mode — but you lose the
developer-experience benefit.

### You value deterministic records

Anvil is moving toward dedicated evidence and session commands. Today, the
public value is deterministic CLI output that can be captured in CI logs or JSON
files for review.

## anvil is NOT for you if...

### You want a linter replacement

anvil complements ESLint, not replaces it. anvil catches _structural_ issues;
ESLint catches _stylistic_ and _semantic_ issues.

**Use both.**

### You want test coverage enforcement

anvil can check coverage thresholds, but it's not a test runner or coverage
tool. Use your existing coverage tooling and integrate with anvil.

### You have no architecture to protect

If your codebase is a single module with no intentional boundaries, anvil's
architecture checks won't find violations. The anti-pattern detection still
helps, but you're not using anvil's primary value.

### You need real-time collaboration

anvil is a development tool, not a collaboration platform. It doesn't sync state
between developers or provide real-time editing features.

## Team Size Considerations

### Solo developers

anvil works well for solo devs who:

- Use AI assistance heavily
- Want to maintain quality without manual review overhead
- Value the peace of mind from automated checks

### Small teams (2-10)

Sweet spot. anvil prevents the "AI generated this, nobody really reviewed it"
problem. Save-time validation means issues don't pile up for PR review.

### Large teams (10+)

anvil scales via CI integration. The architecture safety features become more
valuable as more developers (and more AI agents) touch the codebase.

## Integration with Existing Tools

anvil integrates with your existing workflow, not replaces it:

| Tool           | anvil's role                         |
| -------------- | ------------------------------------ |
| ESLint         | anvil runs ESLint as a gate check    |
| Formatter      | anvil doesn't touch formatting       |
| Jest/Vitest    | anvil can gate on test pass/coverage |
| GitHub Actions | run the anvil CLI in a workflow      |
| VS Code        | anvil provides an extension          |

## Decision Framework

Ask yourself:

1. **Do I use AI coding tools?** → If no, consider skipping anvil
2. **Do I have architectural boundaries?** → If no, value is limited
3. **Do I want save-time feedback?** → If no, use CI-only mode
4. **Do I need deterministic validation output?** → capture Anvil's CLI or JSON
   output

If you answered "yes" to 2+ questions, anvil will likely add value.

---

**Ready to try it?** [Quickstart →](/anvil/quickstart)
