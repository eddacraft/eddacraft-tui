# JSIMP-001 design council — ADR-145

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Advisory | JSIMP | Draft (design-council decision; ADR-145 accepted in this PR) | 2026-09-10 — against `feat/jsimp-001-continuous-journey` / ADR-145 |

| Upstream | Downstream |
| -------- | ---------- |
| [ADR-145](../../decisions/145-continuous-command-journey.md), [transition matrix](../../specs/2026-09-10-continuous-command-journey.md), [JSIMP module](../../modules/journey-simplification.aps.md) | JSIMP-002..006 implementation items; CLI surface / quickstart docs updated in this PR |

**Date:** 2026-09-10
**Mode:** design (risk-selected)
**Target:** ADR-145 + transition matrix (pre-implementation)
**Lenses:** pragmatic-lead, kernel-maintainer, adversarial-reviewer
**Decision:** REPAIR then PASS — architecture stands; contract pins required before JSIMP-002

## Verdict

Do not mint splash, intercept ensure/restart, or a second on-switch. Tighten
the public contract so JSIMP-002/003 cannot invent flags, JSON fields, or
ungated start mutations.

## Must-fix pins applied in this PR

1. Unsigned config-Absent `anvil --json`: old auth envelope exit 3 → existing
   not-activated document exit 1. No additive fields. Predicate = Absent only.
2. `start --json` stays read-only and gated. Equivalent mutations are TUI vs
   interactive plain only.
3. `--no-tui` today auto-installs even on a TTY (ADR-103). JSIMP-002 amends
   that: TTY `--no-tui` becomes interactive plain consent.
4. Welcome shares the orchestrator implementation, not the start licence
   posture. Unsigned welcome must not run MCP/hook/daemon/workflow mutations.
5. Unattended intent reuses `--mcp-client` / `--all-mcp-clients` / `--no-mcp`.
   No new `--yes`. Hooks are spine. SafeDrift repair is not a new choice.
6. Unattended = stdin or stderr not a TTY, or `CI` / `ANVIL_NO_PROMPT` /
   `NONINTERACTIVE`. Stdout redirect alone is presentation.

## Later items (not this PR)

- Unattended MCP selector exclusivity tests — JSIMP-002
- Envelope swap tests — JSIMP-003
- Durable-intent inference — JSIMP-004
- Intercept-ensure comment scrub — JSIMP-006
