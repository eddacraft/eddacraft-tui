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
[quickstart](quickstart.md) before using it.

## What to test first

Run these journeys in order:

1. Install with the method appropriate for your operating system.
2. Verify the version and run `anvil welcome` without signing in.
3. Sign in and run `anvil start`.
4. Confirm the reported protection state and follow any restart instruction.
5. Complete the [ten-minute protection tutorial](first-gate.md).
6. Run `anvil start --verify` after restarting a configured client.
7. Exercise [save-time validation](guides/save-time-validation.md).
8. Install, inspect, and remove [Git hooks](operations/git-hooks.md).
9. Run a CI-shaped gate with machine-readable output.
10. Check that uninstall guidance removes only the state you selected.

## Success criteria

Record:

- operating system and CPU architecture;
- installation method;
- `anvil --version` output;
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
- Guided pre-write configuration currently targets Cursor and Claude Code.
- Other editors use terminal checks or save-time watching unless the
  [generated support reference](reference/support.md) says otherwise.
- Language parsing and specialised rule depth are not identical; the support
  reference distinguishes them.
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

## Report feedback

Open a [public issue](https://github.com/eddacraft/anvil/issues/new/choose) with
the evidence above. Remove tokens, source code, private repository names, and
personal paths before posting.

For a suspected false positive, use `anvil report-fp --help` and review the
displayed data boundary before including a source snippet.

## Next step

After the core pass, test the workflow closest to your real use:
[AI-assisted writes](guides/agent-harness.md),
[team gates](guides/team-flow.md), or
[continuous integration](integrations/github.md).
