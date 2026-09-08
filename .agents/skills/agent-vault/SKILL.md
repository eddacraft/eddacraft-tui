---
name: agent-vault
description: >
  Use for files known to hold credentials — .env files, auth/credentials
  files, or any file containing apiKey/token/password/secret fields or
  key-like high-entropy values. Not for tooling or agent-metadata configs
  (.claude/**, opencode.json, AGENTS.md and similar) unless they contain an
  inline credential.
---

# agent-vault — Secrets-Safe Config Tool

## ⚠️ Hard Rule

### Activate agent-vault only when ONE of these is true

1. Path matches a known-secrets pattern:
   - `.env` / `.env.*`
   - `*credentials*`, `*secrets*`, `*.pem`, `*.key`, `*.p12`
   - `docker-compose.yml` / `docker-compose.yaml` _and_ the file contains
     inline credential fields (otherwise read normally)
2. File contents already known to include an inline credential field
   (`apiKey`, `token`, `password`, `secret`, `credential`, `auth`, or a
   high-entropy value that looks like a key)

### Explicitly DO NOT activate for (read normally with Read/Write/Edit)

- opencode: `opencode.json`, `agents/*.md`, any `.opencode/**`
- Claude: `.claude/**`, `CLAUDE.md`
- Agent/tooling metadata: `AGENTS.md`, skill `SKILL.md` files, ADRs,
  plan/spec files
- Generic `config.yaml` / `config.toml` / `config.json` — extension alone
  is NOT a trigger; only contents are

### Tiebreaker

If you cannot tell whether a config contains secrets without reading it:
**read it normally first**, then switch to agent-vault only if you observe
an inline credential field. Do NOT pre-emptively gate on extension.

Once activated, do NOT use `Read`, `Write`, or `Edit` on the gated file —
use `agent-vault read` and `agent-vault write` instead.

### Availability gate

Before the first vault operation, check `command -v agent-vault`. If the tool is
unavailable, fail closed for every known-secret file: do not read or write that
file with ordinary tools, do not install software without authority, and do not
print its contents. Report the missing capability and ask the user to install or
authorise the repository's documented setup, or to provide another approved
secret-safe mechanism. Resume only after a capability check succeeds.

---

## Safe commands (execute freely)

```bash
agent-vault read <file>              # Read file — secrets shown as <agent-vault:key>
agent-vault write <file> --content   # Write file — placeholders replaced with real values
agent-vault has <key> [keys...]      # Check if keys exist (safe, no reveal)
agent-vault list                     # List stored key names (safe, no reveal)
```

## Sensitive commands (NEVER execute — tell the user to run these)

```bash
agent-vault set <key>                # Store a secret (user only)
agent-vault import <file>            # Bulk import from .env (user only)
agent-vault rm <key>                 # Remove a secret (user only)
agent-vault get <key> [--reveal]     # Show secret value — user only even WITHOUT --reveal
```

**Default deny:** any `agent-vault` subcommand not in the Safe list above is
user-only. Never execute it — even if it appears read-only, and even if it
would succeed non-interactively. These commands reveal or mutate secrets and
belong to the user; the policy holds regardless of whether the command works.
(They also normally require a TTY and fail without one — that is a
convenience, not the reason.) Always tell the user to run them themselves.

---

## Workflow

```
1. agent-vault has <key>             ← check what exists
2. (if missing) tell user:           ← "Please run: agent-vault set <key>"
3. (wait for user confirmation)
4. agent-vault read <file>           ← read with secrets redacted
5. agent-vault write <file> ...      ← write with placeholders resolved
```

## Placeholder format

`<agent-vault:key-name>` — lowercase alphanumeric + hyphens only.

Examples: `<agent-vault:telegram-bot-token>`, `<agent-vault:openai-key>`, `<agent-vault:db-password>`

Unvaulted high-entropy strings appear as `<agent-vault:UNVAULTED:sha256:XXXXXXXX>` — tell the user to vault them with `agent-vault set <key>`.

---

## Examples

### Check and write a config file

```bash
agent-vault has telegram-bot-token openai-key --json
# → {"telegram-bot-token": true, "openai-key": false}
```

Tell the user: `Please run: agent-vault set openai-key`

After confirmation:

```bash
agent-vault write config.json --content '{
  "botToken": "<agent-vault:telegram-bot-token>",
  "openaiKey": "<agent-vault:openai-key>"
}'
```

### Read an existing config

```bash
agent-vault read .env
#  1  TELEGRAM_BOT_TOKEN=<agent-vault:telegram-bot-token>
#  2  OPENAI_KEY=<agent-vault:openai-key>
#  3  PORT=3000
```

### Write via heredoc

```bash
agent-vault write docker-compose.yaml <<'EOF'
services:
  app:
    environment:
      API_KEY: <agent-vault:api-key>
      DB_PASSWORD: <agent-vault:db-password>
    ports:
      - "8080:8080"
EOF
```
