---
id: agent-skills
title: Agent Skill
description:
  Install the Anvil developer-functions skill into a supported coding agent.
sidebar_position: 4
---

# Agent Skill

The Anvil binary includes the customer-readable `anvil-developer-functions`
skill. It teaches a coding agent to inspect Anvil graph context, validate
proposed writes, respect `block` decisions, and report live evidence without
claiming that configuration alone means protection is active.

Install it interactively:

```bash
anvil skill install
```

Anvil strongly detects installed clients, preselects only skill-capable ones,
asks which clients to use, and offers global or project scope. Global is the
default for the beta.

For scripts, select clients and scope explicitly:

```bash
anvil skill install --client codex
anvil skill install --client claude-code --client cursor
anvil skill install --client opencode --scope project
anvil skill install --client codex --verify
anvil skill install --client codex --dry-run
```

| Client             | Global root                 | Project root       |
| ------------------ | --------------------------- | ------------------ |
| Claude Code        | `~/.claude/skills`          | `.claude/skills`   |
| Cursor             | `~/.cursor/skills`          | `.cursor/skills`   |
| Codex              | `~/.agents/skills`          | `.agents/skills`   |
| OpenCode           | `~/.config/opencode/skills` | `.opencode/skills` |
| Gemini CLI         | `~/.agents/skills`          | `.agents/skills`   |
| OpenClaw           | `~/.agents/skills`          | `.agents/skills`   |
| GitHub Copilot CLI | `~/.agents/skills`          | `.agents/skills`   |

Clients that share `.agents/skills` share one installed bundle; Anvil reports
all selected clients but writes the destination once.

## Managed updates

Each installation includes `.anvil-managed.json` with the source catalogue
commit, Anvil version, bundle digest, and SHA-256 hash for every managed file.
Reinstalling the same bundle is a no-op. Anvil updates an older managed bundle
only when its current files still match their recorded hashes.

An existing directory without the managed manifest, or a managed file changed
after installation, is never overwritten. Move or rename that directory, or
choose the other scope, then run the installer again. This is deliberate: Anvil
cannot safely infer that user-authored skill content belongs to it.

The beta snapshot updates with Anvil binary releases. The binary does not fetch
the private skill catalogue during build or installation.
