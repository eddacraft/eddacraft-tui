# admin-cli (archived Node operator CLI)

> **⚠️ Historical archive — retired, not the source of truth.** This is the
> legacy Node.js operator CLI (`anvil-admin`). It has been **fully superseded**
> by the Rust `anvil admin` surface (RCLI2-009 ported all seven subcommands —
> `list`, `show`, `approve`, `invite`, `revoke`, `audit`, `send-migration` —
> and added `email-update`). Use `anvil admin …` for all operator actions.
>
> Retired to `archive/` under V060F-019 (2026-06-19): excluded from the pnpm
> workspace via `!archive/**`, dropped from the root `tsconfig.json` project
> references and the `pnpm admin` script. Kept only for historical reference;
> `git log` has the full pre-archive history.
>
> Note: this CLI sent an `X-Admin-Actor` header that the current API ignores by
> design — admin attribution is derived from the API key itself
> (ADMINCLIH-002), so audit-log actor names from this tool would not match the
> live API surface. Another reason not to run it.

The original package contents follow, unchanged, for reference.
