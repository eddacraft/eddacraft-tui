# deepsec

This directory holds the [deepsec](https://www.npmjs.com/package/deepsec) config
for the parent repo. Checked into git so teammates inherit project context (auth
shape, threat model, custom matchers); generated scan output is gitignored.

Currently configured project: `anvil-001` (target: `..`).

## Setup

1. `pnpm install` — installs deepsec.
2. Add an AI Gateway / Anthropic / OpenAI token to `.env.local`. If you already
   have `claude` or `codex` CLI logged in on this machine, you can skip the
   token for non-sandbox runs (`process` / `revalidate` / `triage`); deepsec
   auto-detects and reuses the subscription. See
   `node_modules/deepsec/dist/docs/vercel-setup.md` after install.
3. Open the parent repo in your coding agent (Claude Code, Cursor, …) and have
   it follow `data/anvil-001/SETUP.md` to fill in `data/anvil-001/INFO.md`.

## Daily commands

```bash
pnpm deepsec scan
pnpm deepsec process     --concurrency 5
pnpm deepsec revalidate  --concurrency 5                  # cuts FP rate
pnpm deepsec export      --format md-dir --out ./findings
```

`--project-id` is auto-resolved while there's only one project in
`deepsec.config.ts`. Once you've added a second project, pass
`--project-id anvil-001` (or whichever id you want) explicitly.

`scan` is free (regex only). `process` is the AI stage (≈$0.30/file on Opus by
default). Run state goes to `data/anvil-001/`.

## Sakana / Fugu models

The AI stages (`process` / `revalidate`) accept any OpenAI-compatible endpoint
through deepsec's `pi` backend. Convenience scripts wire Sakana AI's **Fugu**
and **Fugu Ultra** as ready-to-run options (requires `deepsec >= 2.1.0` for the
`--ai-*` provider-override flags):

```bash
pnpm fugu:process            --project-id anvil-001
pnpm fugu:revalidate         --project-id anvil-001
pnpm fugu-ultra:process      --project-id anvil-001
pnpm fugu-ultra:revalidate   --project-id anvil-001
```

Each script expands to the `pi` backend with the OpenAI-compatible override,
e.g.:

```bash
deepsec process --agent pi --model openai/fugu \
  --ai-provider openai \
  --ai-base-url "$SAKANA_BASE_URL" \
  --ai-api-key-env SAKANA_API_KEY
```

The scripts wrap this in `bash -c '… "$@"' bash` so the `${…:?}` fail-fast guard
and `$SAKANA_BASE_URL` expansion behave the same regardless of the shell pnpm
picks (including `cmd.exe` on Windows), while still forwarding any extra flags
such as `--project-id` / `--concurrency`. They therefore require `bash` on PATH
— on Windows run them from Git Bash or WSL.

Before first use, provide three values (none are baked into the scripts):

1. **Base URL** — `export SAKANA_BASE_URL="https://<sakana-endpoint>/v1"` (or
   add it to `.env.local` and source it). The scripts fail fast if it's unset.
2. **API key** — add `SAKANA_API_KEY=…` to `.env.local` (gitignored). deepsec
   resolves it via `--ai-api-key-env`.
3. **Model ids** — the scripts use `openai/fugu` and `openai/fugu-ultra`. The
   `openai/` prefix is required by the override path; adjust the model name in
   `package.json` if Sakana publishes different identifiers.

The model is always a CLI flag — deepsec has no in-config model registry — so
these scripts (or your own `--agent pi --model …` invocation) are the way to pin
a non-default model per run.

## Adding another project

To scan another codebase from this same `.deepsec/`:

```bash
pnpm deepsec init-project ../some-other-package   # path relative to .deepsec/
```

Appends an entry to `deepsec.config.ts` and writes
`data/<id>/{INFO.md,SETUP.md,project.json}`. Open the new SETUP.md in your agent
to fill in INFO.md.

## Layout

```
deepsec.config.ts        Project list (one entry per scanned repo)
data/anvil-001/
  INFO.md                Repo context — checked into git, hand-curated
  SETUP.md               Agent setup prompt — checked in, deletable
  project.json           Generated (gitignored)
  files/                 One JSON per scanned source file (gitignored)
  runs/                  Run metadata (gitignored)
  reports/               Generated markdown reports (gitignored)
AGENTS.md                Pointer for coding agents
.env.local               Tokens (gitignored)
```

## Docs

After `pnpm install`:

- Skill: `node_modules/deepsec/SKILL.md`
- Full docs:
  `node_modules/deepsec/dist/docs/{getting-started,configuration,models,writing-matchers,plugins,architecture,data-layout,vercel-setup,faq}.md`

Or browse on [GitHub](https://github.com/vercel/deepsec/tree/main/docs).
