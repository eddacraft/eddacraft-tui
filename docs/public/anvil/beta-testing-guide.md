---
id: beta-testing-guide
title: Beta test brief
description:
  Prove install, first value, activation, and recovery on the current anvil
  beta.
---

# Beta test brief

**For:** invited beta testers

**Time:** about 20 minutes

**Outcome:** one reproducible report — platform, version, protection state, and
what worked or broke

This is a **test brief**, not a second setup manual. Setup lives only in the
[quickstart](quickstart.md). Install or update there, then return here.

**Current published beta:**
[v0.9.4-beta](https://github.com/eddacraft/anvil/releases/tag/v0.9.4-beta).
Confirm with `anvil version`. A newer beta is valid.

## The product under test

anvil protects AI-assisted and ordinary edits with the same local checks:

| Moment                    | How you exercise it                               |
| ------------------------- | ------------------------------------------------- |
| Discovery (no account)    | `anvil welcome`                                   |
| Activation (invite-gated) | `anvil auth login` → `anvil start`                |
| Daily path                | bare `anvil` (daemon + existing MCP; no re-setup) |
| Deliberate proof          | [ten-minute protection tutorial](first-gate.md)   |
| CI-shaped gate            | `anvil gate --profile ci --json`                  |

Final activation states are literal: `protecting`, `ready_restart_required`,
`watching`, `needs_action`, `unsupported`, `error`. See
[activation states](guides/start-output-contracts.md).

## Core pass (do this)

From a real project root, in order:

1. **Binary** — `anvil version` (install method + upgrade guidance). If an
   update is available, take it with the printed command, then re-check.
2. **Discovery** — `anvil welcome` without signing in. Expect a guided scan and
   either findings or an explicit clean result.
3. **Identity** — `anvil auth login` (GitHub device flow by default; works over
   SSH/tmux). No GitHub? `anvil auth login --otp`. Then `anvil auth whoami`.
   Approval is required; OTP does not bypass the invite.
4. **Activate** — `anvil start` in a real terminal. Consent offers every
   supported MCP client (unticked by default). Note the final protection state
   and any restart instruction.
5. **Prove detection** — complete the [protection tutorial](first-gate.md).
6. **Verify** — after restarting any named client: `anvil start --verify`
   (read-only; same state vocabulary).
7. **Daily ensure** — bare `anvil` and `anvil --json`. Expect a short confidence
   summary when already activated; recovery that names `anvil start` if the
   project was never activated. Declined clients must not reappear.

A clean scan is not a failure. The question is whether the result is explicit,
repeatable, and recoverable without outside help.

## 0.9.4-beta focus pass

Exercise the paths that changed in this release when they apply to your setup:

| Focus                        | Exercise                                                                                                                     | Expected evidence                                                                                                                     |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Install method honesty       | On Windows or macOS after a quickstart standalone install (or after upgrade), run `anvil version` and `anvil update --check` | Method is not a false `cargo install` for official installer installs; upgrade advice matches the method (PowerShell on Windows)      |
| MCP lean allow               | With a protected client, trigger a clean pre-write allow (or inspect tool JSON)                                              | Default allow response is small (`schema` + `decision`); full envelope available with `detail: "full"` or `ANVIL_MCP_VALIDATE_DETAIL` |
| Durable workspace membership | Register a disposable Git worktree with `anvil workspace register -- <path>`, then run `anvil workspace list --json`         | Success only when the JSON list retains the worktree; no false "failed" when membership sticks after a short wait                     |
| Path false alarms            | Run `anvil check` or `anvil gate` on a tree with long hex-looking path segments in docs or config                            | Path-like tokens are less often reported as high-entropy secrets                                                                      |
| Python dynamic execution     | In a disposable `.py` file, introduce `eval(name)` or `os.system(...)`, then run `anvil check` or `anvil gate`               | `PY-008` / `PY-009` findings appear for those shapes                                                                                  |

Use disposable projects and worktrees for the mutating exercises. Do not test
installer or registration paths against a production checkout.

## Stretch (optional, 10 minutes)

Pick what matches how you work:

| Path              | Commands / docs                                                                                             |
| ----------------- | ----------------------------------------------------------------------------------------------------------- |
| Multi-harness MCP | `anvil mcp install --help`, install one client from the list, restart it, `anvil start --verify`            |
| Scripted clients  | `anvil start --mcp-client <name>` or `--all-mcp-clients` (headless); or `anvil mcp install --client <name>` |
| Save-time loop    | [save-time validation](guides/save-time-validation.md) · `anvil watch`                                      |
| Git hooks         | [git hooks](operations/git-hooks.md) · `anvil hooks install` / `status` / `uninstall`                       |
| CI gate           | `anvil gate --profile ci --json` (warnings do not fail the gate unless `--fail-on-warnings`)                |
| Skills / doctor   | `anvil skill install` · `anvil doctor`                                                                      |
| Uninstall dry-run | `anvil uninstall --dry-run` vs [uninstall](operations/uninstall.md)                                         |
| AI write path     | [agent harness](guides/agent-harness.md)                                                                    |

The client registry is versioned with the binary — run
`anvil mcp install --help` for the list your install supports.

## What is in beta (honest boundaries)

- `anvil welcome` needs no account; ongoing protection needs approved beta auth.
- Default sign-in is GitHub device flow; `--otp` is the email fallback.
- Interactive `anvil start` offers every registry client; nothing is written
  until you select one. Use `--no-mcp` if editor MCP is blocked.
- Bare `anvil` is ensure-only: no silent first-time install, no re-offer of
  declined clients.
- Language depth is uneven — [support matrix](reference/support.md) separates
  compiled patterns from parse-only coverage.
- Warning findings do not fail `anvil gate` by default.
- Command shapes and output can still change before a stable release.
- Do not assume a separate editor extension; terminal + MCP + watch are the
  shipped surfaces.

## Recovery (safe to break on purpose)

| Situation                | Expected recovery                                         |
| ------------------------ | --------------------------------------------------------- |
| Client needs restart     | Restart the named client → `anvil start --verify`         |
| Session expired          | `anvil auth refresh`; sign in again if asked              |
| Binary missing from PATH | [troubleshooting](operations/troubleshooting.md)          |
| Unsupported project      | Output names the gap; it does not claim protection        |
| Foreground watcher       | Ctrl-C ends it                                            |
| Hooks unwanted           | `anvil hooks uninstall`                                   |
| Odd environment          | `anvil doctor` (add `--fix` only when you intend repairs) |

## Report feedback

Capture once:

- OS and CPU architecture
- install method (`anvil version` prints it)
- full `anvil version` output
- sign-in method (GitHub or OTP) and whether `anvil auth whoami` matched
- project languages and AI client(s), if any
- final protection state
- the exact command that failed or surprised you
- whether recovery worked without outside help

Open a [public issue](https://github.com/eddacraft/anvil/issues/new/choose) when
a redacted public report is fine. Strip tokens, private paths, and source you
cannot share. Otherwise reply to your beta invitation with the same evidence.

Suspected false positive: `anvil report-fp --help` first — reports stay local
unless you opt into a snippet; paths are hashed by default.

## Next

After the core pass, stress the path you actually use:
[team gates](guides/team-flow.md), [CI](integrations/github.md), or
[local dashboards](guides/dashboard.md).
