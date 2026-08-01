---
id: when-to-use
title: When to use anvil
description:
  Decide whether anvil fits your project, workflow, and current language
  coverage.
---

# When to use anvil

**For:** developers evaluating anvil

**Time:** 3 minutes

**Outcome:** a clear use, trial, or defer decision

## Use anvil now when

- AI-assisted changes increase review volume.
- New architecture edges matter more than old debt.
- You want local findings before code review.
- Your project is covered by the [support matrix](reference/support.md).
- You want the same checks available in a terminal, Git hooks, and continuous
  integration.

## Trial anvil in one project when

- Your language has parsing support but limited specialised rules.
- You need to measure false positives before team rollout.
- Client pre-write integration is unavailable but save-time watching is useful.
- You want to begin with advisory findings rather than blocking gates.

Start with `anvil welcome`. It does not require sign-in or modify editor
configuration.

## Defer adoption when

- The project uses only unsupported file types.
- Policy requires a security or data review that has not happened yet.
- You need an editor extension that the current support reference does not list.
- Your workflow cannot tolerate beta interfaces changing between releases.

## What anvil does not replace

anvil complements rather than replaces:

- compilers and type checkers;
- unit, integration, and security tests;
- linters and formatters;
- human code review; and
- deployment controls.

## Decision table

| Need                              | Best first command                  |
| --------------------------------- | ----------------------------------- |
| Evaluate value without an account | `anvil welcome`                     |
| Scan named files                  | `anvil check path/to/file`          |
| Activate or reconfigure a project | `anvil start`                       |
| Turn protection on day-to-day     | bare `anvil` (after first `start`)  |
| Run a workflow decision           | `anvil gate`                        |
| Diagnose an installation          | `anvil doctor`                      |
| Inspect current coverage          | Use the generated support reference |

## Next step

If the fit is promising, [install and get first value](quickstart.md). If a
required capability is absent, record that gap before rolling anvil out to a
team.
