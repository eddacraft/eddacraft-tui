---
id: quickstart
title: Install and get first value
description:
  Install anvil, verify it, and run a first local check in under ten minutes.
sidebar_position: 2
---

# Install and get first value

**For:** first-time users

**Time:** 5–10 minutes

**Outcome:** a verified anvil installation and a real result from your project

## Before you begin

You need:

- macOS, Linux, or Windows;
- a terminal;
- a project containing source code; and
- internet access for installation.

You do **not** need an account for the first discovery run. Ongoing protection
uses beta authentication.

## 1. Install anvil

Choose one method. Do not combine package managers on the same machine.

### macOS or Linux — standalone installer

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh | sh
```

### macOS or Linux — Homebrew

```bash
brew install eddacraft/tap/anvil
```

### Windows PowerShell — standalone installer

```powershell
irm https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.ps1 | iex
```

### Windows — WinGet

```powershell
winget install eddacraft.anvil
```

### Windows — Scoop

```powershell
scoop bucket add eddacraft https://github.com/eddacraft/scoop-bucket
scoop install anvil
```

If the command reports that another installation method already owns anvil,
upgrade or uninstall through that method instead of overwriting it.

## 2. Verify the binary

Open a new terminal, then run:

```text
anvil --version
```

Success looks like:

```text
anvil 0.9.1-beta
```

A newer beta version is also valid. If the command is not found, reopen the
terminal once, then see
[installation troubleshooting](operations/troubleshooting.md#anvil-is-not-found).

## 3. Get first value without signing in

Change to the root of a source-code project and run:

```text
anvil welcome
```

The guided discovery scans a sample of the project and explains what it found. A
clean result is still useful evidence; it means the scanned sample did not match
an enabled rule.

Success means the command completes and shows either findings or an explicit
clean result.

## 4. Sign in for ongoing protection

Ongoing protection is invite-gated during beta. `anvil auth login` registers the
chosen identity and provisions a local credential only after access is approved.
If you are not yet invited, request beta access at
[eddacraft.ai](https://eddacraft.ai) and continue using the account-free
`anvil welcome` path meanwhile.

When access is approved and you are ready to activate save-time or pre-write
protection, run:

```text
anvil auth login
```

Follow the displayed device-login instructions. Then confirm the identity:

```text
anvil auth whoami
```

If you do not have a GitHub account, run `anvil auth login --otp`. This changes
the identity method; it does not bypass beta approval. `anvil auth whoami`
confirms that the approved credential is active without printing its secret.

## 5. Activate the project

From the project root, run:

```text
anvil start
```

This command may create project configuration, record the existing baseline, and
offer MCP install for every supported AI client (interactive consent; nothing is
written until you select a client). It reports one final protection state:

| State                    | Meaning                                          | What to do                                                |
| ------------------------ | ------------------------------------------------ | --------------------------------------------------------- |
| `protecting`             | Pre-write validation is active                   | Continue working                                          |
| `ready_restart_required` | Client configuration is ready                    | Restart the named client, then run `anvil start --verify` |
| `watching`               | The local protection service is available        | Run `anvil watch` for a visible save-time loop            |
| `needs_action`           | Setup needs a repair                             | Follow the command's suggested action                     |
| `unsupported`            | The detected project is outside current coverage | Check the support reference                               |
| `error`                  | Activation failed                                | Run `anvil doctor` and use troubleshooting                |

For a read-only diagnosis that changes nothing, use:

```text
anvil start --verify
```

## 6. Day two: turn protection on without reinstalling

After the project has been activated once with `anvil start`, the daily path is
the bare command:

```text
anvil
```

This ensures the local protection daemon and already-configured MCP entries. It
does not open a setup picker and does not install clients you previously
skipped. If the project was never activated, the command exits with recovery
that names `anvil start` or `anvil welcome`.

For a machine-readable check:

```text
anvil --json
```

Use `anvil start` again only when you need to change configuration (new client,
hooks, or repair). For a read-only diagnosis, keep using `anvil start --verify`.

## Common problems

- **Sign-in fails:** retry `anvil auth login` and check that the shown URL is
  reachable.
- **The project is unsupported:** compare its file types with the
  [generated support matrix](reference/support.md).
- **Protection needs a restart:** fully quit and reopen the named AI client.
- **You want no editor changes:** use `anvil start --no-mcp` for the
  daemon-backed path without client configuration.

## Next step

If beta access is approved and `anvil auth whoami` confirms your identity, run
the [ten-minute protection tutorial](first-gate.md) to create, detect, fix, and
remove a deliberate finding.

If you are waiting for access, keep using the account-free `anvil welcome`
journey. You can also review [when anvil fits](when-to-use.md), compare the
[supported languages and clients](reference/support.md), or request access at
[eddacraft.ai](https://eddacraft.ai). The protection tutorial uses an
authenticated command and will not work until your beta access is approved.
