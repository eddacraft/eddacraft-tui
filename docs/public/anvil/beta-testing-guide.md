---
id: beta-testing-guide
title: Test the current beta
description:
  Focus beta testing on the current user journeys, known boundaries, and useful
  feedback.
---

# Test the current beta

**For:** invited beta testers and teams evaluating anvil

**Time:** 30–60 minutes for the core pass

**Outcome:** reproducible feedback about installation, first value, protection,
and recovery

This guide is a test brief, not a second setup manual. Complete the canonical
[quickstart](quickstart.md) first. If you arrived from a beta invitation, use
the quickstart to install or update anvil, sign in, and confirm your identity;
then return here to test the current beta.

**Current beta:**
[v0.9.0-beta](https://github.com/eddacraft/anvil/releases/tag/v0.9.0-beta). A
newer beta reported by `anvil version` is also valid.

## Before you test

- **First-time tester:** follow the [quickstart](quickstart.md).
- **Returning tester:** run `anvil version`. If it reports an update, use the
  recommended command for your installation method before testing.
- **Approved access:** run `anvil auth login`, complete the GitHub device
  sign-in, then run `anvil auth whoami`. If you do not use GitHub, run
  `anvil auth login --otp`; this verifies your invited email but does not bypass
  beta approval.

## What to test first

Run these journeys in order:

1. Install or update with the method appropriate for your operating system.
2. Run `anvil version`, then run `anvil welcome` without relying on sign-in.
3. Sign in, confirm `anvil auth whoami`, and run `anvil start` (expect the
   interactive activation surface in a real terminal). Offer MCP install for any
   client you use from the full consent list.
4. Confirm the reported protection state and follow any restart instruction.
5. Complete the [ten-minute protection tutorial](first-gate.md).
6. Run `anvil start --verify` after restarting a configured client.
7. Exercise [save-time validation](guides/save-time-validation.md).
8. **Daily ensure:** from the same project, run bare `anvil` (and
   `anvil --json`). Expect a short confidence summary when already activated;
   expect recovery naming `anvil start` if config is absent. Confirm it does not
   re-offer MCP installs you declined.
9. Multi-harness MCP: run `anvil mcp install --help`, install a client your
   binary lists, restart that client, then `anvil start --verify`.
10. Optional: `anvil skill install` / `anvil doctor` for managed-skill freshness
    when your binary exposes them.
11. Install, inspect, and remove [Git hooks](operations/git-hooks.md).
12. Run `anvil gate --profile ci --json` and inspect the machine-readable
    output.
13. Run `anvil uninstall --dry-run` and confirm that the proposed scope matches
    the [uninstall guide](operations/uninstall.md).

## Success criteria

Record:

- operating system and CPU architecture;
- installation method;
- `anvil version` output, including whether it reported the current beta or an
  available update;
- sign-in method (GitHub device sign-in or email OTP), plus whether
  `anvil auth whoami` confirmed the expected identity;
- project languages;
- AI client, if any;
- the final protection state;
- the exact command that failed or surprised you; and
- whether recovery guidance worked without outside help.

A clean scan is not a test failure. The important question is whether the result
is explicit, understandable, and repeatable.

## Current beta boundaries

- `anvil welcome` is the account-free discovery path.
- Ongoing activation requires beta authentication.
- GitHub device sign-in is the default authentication path. Email OTP is the
  fallback for an approved tester who does not use GitHub.
- Interactive `anvil start` offers every supported MCP client in the consent
  list (unticked by default). Scripted install uses
  `anvil mcp install --client`, `--mcp-client`, or `--all-mcp-clients`.
- Bare `anvil` is ensure-only after activation: no silent first-time install and
  no re-offer of declined clients.
- Other editors can also use terminal checks and save-time watching without MCP.
- Language parsing and specialised rule depth are not identical; the support
  reference distinguishes them.
- Warning-severity findings do not fail `anvil gate` by default; opt in with
  `--fail-on-warnings` when you need that stricter posture.
- Beta command shapes and output may change before a stable release.
- Do not assume a separate editor extension is installed.

## Recovery tests

Deliberately verify these safe paths:

| Situation                            | Expected recovery                                                         |
| ------------------------------------ | ------------------------------------------------------------------------- |
| Client configuration needs a restart | Restart the named client, then run `anvil start --verify`                 |
| Authentication expires               | Run `anvil auth refresh`, then sign in again if asked                     |
| Installation is not found            | Follow the PATH checks in troubleshooting                                 |
| The project is unsupported           | The output names the unsupported coverage rather than claiming protection |
| A watcher must stop                  | Ctrl-C ends the foreground process                                        |
| Hooks are no longer wanted           | `anvil hooks uninstall` removes anvil-managed hooks                       |
| Managed skill is stale after upgrade | `anvil doctor`, then the skill install path from installed help           |

## Report feedback

Open a [public issue](https://github.com/eddacraft/anvil/issues/new/choose) with
the evidence above when a redacted public report is appropriate. Remove tokens,
source code, private repository names, and personal paths before posting.

For installation, sign-in, or project details that should stay private — or if
you do not use GitHub — reply to your beta invitation with the same evidence.

For a suspected false positive, use `anvil report-fp --help` and review the
displayed data boundary before including a source snippet.

## Next step

After the core pass, test the workflow closest to your real use:
[AI-assisted writes](guides/agent-harness.md),
[team gates](guides/team-flow.md), or
[continuous integration](integrations/github.md).
