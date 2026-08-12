---
id: quickstart
title: Install and get first value
description:
  Install anvil, see a real result from your project, and turn protection on.
sidebar_position: 2
owner: DOCSYNC
upstream:
  - install.sh
  - crates/anvil-cli/src/commands/welcome.rs
  - crates/anvil-cli/src/commands/auth.rs
  - crates/anvil-cli/src/commands/start.rs
verified_against: 0.9.3-beta
---

# Install and get first value

**For:** first-time users

**Time:** about 10 minutes

**Outcome:** anvil is installed, you have a real project result, and you know
the daily path

You do **not** need an account for discovery. Ongoing protection is invite-gated
during beta.

## 1. Install

Pick **one** method. Do not mix package managers on the same machine.

### macOS / Linux

Standalone:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh | sh
```

Homebrew:

```bash
brew install eddacraft/tap/anvil
```

### Windows

PowerShell installer:

```powershell
irm https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.ps1 | iex
```

WinGet:

```powershell
winget install eddacraft.anvil
```

Scoop:

```powershell
scoop bucket add eddacraft https://github.com/eddacraft/scoop-bucket
scoop install anvil
```

If another method already owns the binary, upgrade or uninstall through that
method instead of overwriting it.

## 2. Verify

Open a **new** terminal:

```text
anvil version
```

You should see a version (currently `0.9.4-beta` or newer), the install method,
and upgrade guidance when an update exists. Prefer `anvil version` over
`anvil --version` when you care about how the binary was installed.

Not found? Reopen the terminal once, then
[installation troubleshooting](operations/troubleshooting.md#anvil-is-not-found).

## 3. First value (no sign-in)

From the root of a real source project:

```text
anvil welcome
```

Discovery scans a sample of the tree and explains what it found. A clean result
still counts — it means the sample did not match an enabled rule.

Success: the command finishes with either findings or an explicit clean result.

## 4. Sign in

Ongoing protection needs approved beta access. Request access at
[eddacraft.ai](https://eddacraft.ai) if you are not invited yet; keep using
`anvil welcome` until then.

When you are approved:

```text
anvil auth login
anvil auth whoami
```

Default is **GitHub device sign-in** (works headless over SSH/tmux). No GitHub?
`anvil auth login --otp`. OTP changes the identity method; it does **not**
bypass the invite. `whoami` confirms identity without printing secrets.

## 5. Activate this project

```text
anvil start
```

In a real terminal this opens interactive activation. It may write project
config, record a baseline, and offer MCP install for every supported AI client
(unticked by default — nothing is written until you select one).

Note the **final protection state**. Common ones:

| State                    | What it means                      | What you do                                       |
| ------------------------ | ---------------------------------- | ------------------------------------------------- |
| `protecting`             | Pre-write validation is live       | Keep working                                      |
| `ready_restart_required` | Client config is installed         | Restart the named client → `anvil start --verify` |
| `watching`               | Local daemon knows the project     | Optional: `anvil watch` for a visible save loop   |
| `needs_action`           | A named setup step is incomplete   | Follow the suggested action                       |
| `unsupported` / `error`  | Coverage gap or activation failure | Support matrix · `anvil doctor`                   |

Read-only probe (changes nothing):

```text
anvil start --verify
```

No editor MCP? Use `anvil start --no-mcp` for the daemon-backed path only.

Full state vocabulary: [activation states](guides/start-output-contracts.md).

## 6. Day two: bare `anvil`

After one successful `anvil start`, the daily command is:

```text
anvil
```

That ensures the local daemon and **already configured** MCP entries. It does
not open a setup picker and does not reinstall clients you skipped. Never
activated? Recovery names `anvil start` or `anvil welcome`.

Machine-readable:

```text
anvil --json
```

Use `anvil start` again only to reconfigure (new client, hooks, repair).

## If something fails

| Problem             | First move                                                             |
| ------------------- | ---------------------------------------------------------------------- |
| Sign-in fails       | Retry `anvil auth login`; confirm the shown URL is reachable           |
| Unsupported project | Compare file types with the [support matrix](reference/support.md)     |
| Needs a restart     | Fully quit and reopen the named AI client, then `anvil start --verify` |
| Odd environment     | `anvil doctor`                                                         |

## Next

With approved access and a confirmed identity, prove detection end-to-end:

→ [Ten-minute protection tutorial](first-gate.md)

Invited beta tester recording evidence:

→ [Beta test brief](beta-testing-guide.md)

Still deciding fit: [when to use anvil](when-to-use.md) ·
[support matrix](reference/support.md) · [eddacraft.ai](https://eddacraft.ai)
