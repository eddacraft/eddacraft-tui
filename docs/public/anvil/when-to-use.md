---
id: when-to-use
title: When to Use Anvil
description:
  Understand when Anvil adds value and when other tools are more appropriate.
sidebar_position: 2
---

# When to Use Anvil

Anvil is powerful but focused. This page helps you understand when it's the
right tool.

## Anvil is for you if...

### You ship with AI assistance

Anvil is designed for teams using AI coding tools (Copilot, Claude, Cursor,
etc.). If you're not using AI assistance, the ROI is lower—though the
architecture safety features still provide value.

### You care about architecture

Anvil shines when you have intentional architectural boundaries:

- Layer separation (UI → Services → Data)
- Domain boundaries (Payments shouldn't import User internals)
- Package boundaries (Public API vs internal implementation)

### You want save-time feedback

Anvil's watch mode gives you instant feedback. If you only want CI-time
validation, you can use Anvil in CI mode—but you lose the developer experience
benefit.

### You value audit trails

Every Anvil run produces evidence. If compliance, traceability, or post-incident
analysis matters to you, this is valuable.

## Anvil is NOT for you if...

### You want a linter replacement

Anvil complements ESLint, not replaces it. Anvil catches _structural_ issues;
ESLint catches _stylistic_ and _semantic_ issues.

**Use both.**

### You want test coverage enforcement

Anvil can check coverage thresholds, but it's not a test runner or coverage
tool. Use your existing coverage tooling and integrate with Anvil.

### You have no architecture to protect

If your codebase is a single module with no intentional boundaries, Anvil's
architecture checks won't find violations. The anti-pattern detection still
helps, but you're not using Anvil's primary value.

### You need real-time collaboration

Anvil is a development tool, not a collaboration platform. It doesn't sync state
between developers or provide real-time editing features.

## Team Size Considerations

### Solo developers

Anvil works well for solo devs who:

- Use AI assistance heavily
- Want to maintain quality without manual review overhead
- Value the peace of mind from automated checks

### Small teams (2-10)

Sweet spot. Anvil prevents the "AI generated this, nobody really reviewed it"
problem. Save-time validation means issues don't pile up for PR review.

### Large teams (10+)

Anvil scales via CI integration. The architecture safety features become more
valuable as more developers (and more AI agents) touch the codebase.

## Integration with Existing Tools

Anvil integrates with your existing workflow, not replaces it:

| Tool           | Anvil's Role                         |
| -------------- | ------------------------------------ |
| ESLint         | Anvil runs ESLint as a gate check    |
| Prettier       | Anvil doesn't touch formatting       |
| Jest/Vitest    | Anvil can gate on test pass/coverage |
| GitHub Actions | Anvil provides a GitHub Action       |
| VS Code        | Anvil provides an extension          |

## Decision Framework

Ask yourself:

1. **Do I use AI coding tools?** → If no, consider skipping Anvil
2. **Do I have architectural boundaries?** → If no, value is limited
3. **Do I want save-time feedback?** → If no, use CI-only mode
4. **Do I need audit trails?** → Anvil provides this automatically

If you answered "yes" to 2+ questions, Anvil will likely add value.

---

**Ready to try it?** [Quickstart →](/anvil/quickstart)
