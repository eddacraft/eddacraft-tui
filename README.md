# anvil

| Type   | Authority | Owner  | Status | Freshness                                                                         |
| ------ | --------- | ------ | ------ | --------------------------------------------------------------------------------- |
| README | Advisory  | DOCGOV | Live   | Operator cockpit 2026-07-13; agent-vault local secrets; product tag `v0.9.0-beta` |

| Upstream                                               | Downstream                      |
| ------------------------------------------------------ | ------------------------------- |
| `RELEASE-PLAN.md`, `plans/index.aps.md`, docs policies | Repository operators and agents |

<p align="center">
  <img src="apps/website/public/images/anvil-brandmark-ember.svg" alt="anvil brandmark" width="120" />
</p>

> **AI agents make software probabilistic. anvil makes it deterministic.**

anvil is a pure-Rust governance layer that sits between AI coding agents and
production code. It watches saves, runs checks and policies, and warns on
**new** violations — architecture drift, anti-patterns, secrets, policy breaks —
before they leave the machine. Warnings over blocks; new edges only.

**Shipped product:** Rust binary + MCP shim · **Monorepo also has:** TypeScript
docs/API/tooling, Pulumi infra, APS plans.

| Surface      | Link                                                                |
| ------------ | ------------------------------------------------------------------- |
| Early access | [eddacraft.ai](https://eddacraft.ai)                                |
| Public docs  | [docs.eddacraft.ai/anvil](https://docs.eddacraft.ai/anvil/overview) |
| Install      | [install.eddacraft.ai](https://install.eddacraft.ai)                |
| Latest tag   | **`v0.9.0-beta`**                                                   |
| Live work    | [`plans/index.aps.md`](./plans/index.aps.md)                        |
| Release cut  | [`RELEASE-PLAN.md`](./RELEASE-PLAN.md)                              |

> **Orienting in this repo?** Start with [`CONTEXT.md`](CONTEXT.md) —
> vocabulary, where things live, and where to go next. Behaviour contract:
> [`AGENTS.md`](AGENTS.md).

---

## Operator cockpit

Use this table when you open the repo and need the right door fast.

| I need to…                                     | Go here                                                                                           |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| See active modules / pick work                 | [`plans/index.aps.md`](plans/index.aps.md)                                                        |
| Check release cut-line and blockers            | [`RELEASE-PLAN.md`](RELEASE-PLAN.md)                                                              |
| Understand the tree (apps / packages / crates) | [`CONTEXT.md`](CONTEXT.md)                                                                        |
| Know workflow, commits, validation policy      | [`AGENTS.md`](AGENTS.md)                                                                          |
| Set up machine / open a PR                     | [`CONTRIBUTING.md`](CONTRIBUTING.md)                                                              |
| Branch / worktree rules                        | [branching](docs/guides/branching-strategy.md) · [worktrees](docs/guides/worktree-policy.md)      |
| Architecture / scope guard                     | [overview](docs/architecture/overview.md) · [scope](docs/vision/anvil-scope-guard.md)             |
| ADR index                                      | [`plans/decisions/DECISION-LOG.md`](plans/decisions/DECISION-LOG.md)                              |
| Run a release                                  | [release runbook](docs/runbooks/release-runbook.md) · [cadence](docs/policies/release-cadence.md) |
| Approve waitlist / invite beta                 | [admin CLI runbook](docs/runbooks/admin-cli.md) · § [Admin CLI](#admin-cli-anvil-admin) below     |
| Dogfood candidate vs prod                      | [ANVIL_HOME runbook](docs/runbooks/anvil-home-side-by-side.md)                                    |
| Full CLI command catalogue                     | [cli-surface runbook](docs/runbooks/cli-surface.md)                                               |
| Docs authority model                           | [documentation governance](docs/guides/documentation-governance.md)                               |
| Feature flags                                  | [flag governance](docs/guides/feature-flag-governance.md) · `flags/manifest.json`                 |
| Local secrets / `.env`                         | § [agent-vault](#local-secrets-agent-vault) below                                                 |
| Agent skills / commands inventory              | [agent-surface-inventory](docs/guides/agent-surface-inventory.md)                                 |

### Local secrets (agent-vault)

We use **`agent-vault`** (CLI on `PATH`, typically `~/.local/bin/agent-vault`)
for local developer secrets — API keys, admin tokens, `.env` values, and
anything that must not land in agent transcripts or git.

**Hard rule for agents and for you when pairing with agents:** do not
`Read`/`Write`/`Edit` secret-bearing files (`.env`, credentials, compose with
keys). Use agent-vault instead so values stay redacted in context.

```bash
# --- Safe for agents (and day-to-day) ---
agent-vault list                         # key names only (no values)
agent-vault has <key> [keys...]          # existence check; --json ok
agent-vault read <file>                  # file with secrets as <agent-vault:key>
agent-vault write <file> --content '…'   # write; placeholders resolved from vault
# placeholders: <agent-vault:key-name>   (lowercase, hyphens)

# --- Human / TTY only (you run these; agents must not) ---
agent-vault init                         # first-time vault on this machine
agent-vault set <key>                    # store a secret (interactive)
agent-vault get <key> --reveal           # show a value
agent-vault import <file>                # bulk import from .env
agent-vault rm <key>                     # remove a key
agent-vault scan <file>                  # audit a file for secrets
```

Typical local-dev flow:

```bash
# 1. Put keys in the vault once (you)
agent-vault set anvil-admin-key
agent-vault set openai-key   # example

# 2. Confirm without revealing
agent-vault has anvil-admin-key openai-key

# 3. Materialise or update a .env with placeholders (agents can do this)
agent-vault write .env --content $'ANVIL_ADMIN_KEY=<agent-vault:anvil-admin-key>\n'

# 4. Or inject for one command via your shell after you reveal/export yourself
#    Prefer anvil admin auth set / 1Password for admin when possible.
```

For `anvil admin`, prefer `anvil admin auth set 1password op://…` or
`anvil admin auth set key -` over long-lived plaintext `.env` exports. Use
agent-vault when a file-backed env or agent-editable config is required.

### Setup

Prerequisites (root `package.json` engines): **Node >= 24**, **pnpm >= 11**,
**Git >= 2.54**, plus a Rust toolchain via [rustup](https://rustup.rs/).

```bash
gx clone eddacraft/anvil-001 && gx anvil-001
pnpm install
pnpm build                     # TypeScript workspace (Nx); needed before many JS tests
```

Full setup and package-filter recipes: [`CONTRIBUTING.md`](CONTRIBUTING.md).
Repo-manager conventions:
[`docs/guides/repository-operations.md`](docs/guides/repository-operations.md).

### Build locally

```bash
# Whole monorepo (TypeScript apps/packages via Nx)
pnpm build

# Rust CLI binary (debug)
cargo build -p eddacraft-anvil
# → target/debug/anvil

# Rust CLI binary (release — dogfood / candidate)
cargo build -p eddacraft-anvil --release
# → target/release/anvil

# One-shot run without installing
cargo run -p eddacraft-anvil -- --help
cargo run -p eddacraft-anvil -- status
```

Package-scoped TS work (after `pnpm build` at least once for project
references):

```bash
pnpm -F @eddacraft/anvil-aps test
pnpm exec nx test @eddacraft/anvil-core
pnpm exec nx build @eddacraft/anvil-docs-meta

# Local app dev servers (secrets via agent-vault — never commit .env)
# agent-vault read .env   /   agent-vault write .env …
pnpm -F @eddacraft/anvil-website dev
pnpm -F @eddacraft/anvil-api dev          # wrangler
pnpm -F @eddacraft/anvil-api migrate:dry-run
pnpm -F @eddacraft/docs-public start      # Docusaurus public docs
```

### Dev auth override (`ANVIL_DEV`)

Licence-gated CLI commands need a beta session unless you opt into the local
developer bypass. Set **`ANVIL_DEV=1`** — it is a documented local override on
`cli.licence-gate` (and unlocks the APS dashboard flag). Use only on your
machine; never in CI or user-facing docs.

```bash
export ANVIL_DEV=1

# Debug binary
cargo run -p eddacraft-anvil -- status
cargo run -p eddacraft-anvil -- start

# Or point at a release build
./target/release/anvil status
./target/release/anvil welcome
```

Real device-flow login (no bypass) against a chosen API host:

```bash
ANVIL_API_URL="https://api.eddacraft.ai" anvil auth login
```

Flag inventory / override contract:
[`docs/guides/feature-flag-inventory.md`](docs/guides/feature-flag-inventory.md).

### Dogfood a candidate (`ANVIL_HOME`)

Run a local build beside the production install without clobbering credentials
or the prod daemon socket:

```bash
export ANVIL_HOME="$HOME/.anvil-candidate"
install -d -m 700 "$ANVIL_HOME"
export ANVIL_DEV=1

cargo build -p eddacraft-anvil --release
ANVIL_HOME="$ANVIL_HOME" ./target/release/anvil status --json
ANVIL_HOME="$ANVIL_HOME" ./target/release/anvil start
```

Under a non-default `ANVIL_HOME`, durable **project** writes (baseline, witness,
hooks, init, …) are gated read-only unless you pass `--touch-project-state` or
set `ANVIL_TOUCH_PROJECT_STATE=1`. Full procedure:
[`docs/runbooks/anvil-home-side-by-side.md`](docs/runbooks/anvil-home-side-by-side.md).

Older symlink helper (still useful for a named `anvil-beta` on PATH):
`scripts/dev/run-candidate.sh` (`--status` / `--restore`).

### Product CLI (local dogfood)

After `cargo build -p eddacraft-anvil` (and usually `export ANVIL_DEV=1`):

```bash
anvil=./target/debug/anvil   # or target/release/anvil

$anvil welcome                 # discovery scan; no login
$anvil start                   # daemon + hooks + (by default) MCP config
$anvil start --no-mcp          # activation without writing editor MCP config
ANVIL_NO_MCP=1 $anvil start    # same via env

$anvil status                  # project + daemon health
$anvil status --json
$anvil doctor                  # cheap diagnostics (witness, hooks, daemon)
$anvil doctor --fix            # apply safe repairs (refused under gated ANVIL_HOME)

$anvil check                   # anti-pattern / secret / policy pass
$anvil gate --profile ai       # curated AI guardrail gate
$anvil hooks status
$anvil hooks install           # file-mode hooks (default)
$anvil hooks install --config  # Git 2.54 native hook.<event>.command

$anvil mcp-config --verify     # drift-check editor MCP config
$anvil mcp serve --stdio       # MCP server (what editors launch)

$anvil auth login              # real beta device-flow (needs network + API)
$anvil auth status
$anvil auth logout
```

Useful env vars (operator machine only):

| Env                           | Purpose                                                                    |
| ----------------------------- | -------------------------------------------------------------------------- |
| `ANVIL_DEV=1`                 | Local licence-gate bypass (`cli.licence-gate`)                             |
| `ANVIL_HOME=<dir>`            | Re-root install state (daemon socket, credentials, cache)                  |
| `ANVIL_TOUCH_PROJECT_STATE=1` | Allow project mutations under non-default `ANVIL_HOME`                     |
| `ANVIL_API_URL`               | API host for auth / admin (default `https://api.eddacraft.ai`)             |
| `ANVIL_ADMIN_KEY`             | Admin bearer (prefer `anvil admin auth set` or agent-vault — never commit) |
| `ANVIL_NO_MCP=1`              | Skip MCP config writes on `start`                                          |
| `ANVIL_NO_PROMPT=1`           | Non-interactive; no TTY prompts                                            |

Catalogue + flags:
[`docs/runbooks/cli-surface.md`](docs/runbooks/cli-surface.md). Activation
without MCP:
[`docs/runbooks/anvil-no-mcp-activation.md`](docs/runbooks/anvil-no-mcp-activation.md).

### Admin CLI (`anvil admin`)

`anvil admin` is the **Rust** operator surface on the same binary (legacy Node
`anvil-admin` is archived). It talks to `/admin/*` on the API — waitlist,
invites, revoke, audit, migration mail. **Does not use `ANVIL_DEV`**; it uses
the admin key.

```bash
# One-time credential setup (preferred: 1Password; key never in shell history)
anvil admin auth set 1password op://Anvil/admin-key/credential
# Or store key locally (0600):  anvil admin auth set key -   # paste then Enter
anvil admin auth status

# Optional host override
export ANVIL_API_URL="https://api.eddacraft.ai"

# Day-to-day
anvil admin list
anvil admin list --status pending --limit 20
anvil admin show someone@example.com
anvil admin approve someone@example.com
anvil admin approve --batch 10
anvil admin invite someone@example.com --name "Name"
anvil admin revoke someone@example.com --yes
anvil admin audit --action user.approved --limit 20
anvil admin send-migration                    # dry-run default
anvil admin send-migration --no-dry-run --yes # actually send

# Scoped one-shot without stored config
op run --env-file=admin.env -- anvil admin list
# admin.env: ANVIL_ADMIN_KEY="op://Anvil/admin-key/credential"
```

Resolution order: `ANVIL_ADMIN_KEY` env → configured source in `admin-auth.json`
(`1password` or `key`). Do not paste keys on the command line. If the key lives
in a local `.env` or agent-edited config, store it with **agent-vault**
(`agent-vault set …` / `agent-vault write .env …`) rather than plain files in
git. Full procedures, exit codes, per-operator keys, rotation:
[`docs/runbooks/admin-cli.md`](docs/runbooks/admin-cli.md). Waitlist email ops:
[`docs/runbooks/waitlist-email-operations.md`](docs/runbooks/waitlist-email-operations.md).

### Release and post-deploy

Command-driven release (do not hand-edit release state on a normal cut):

```bash
bash scripts/release/assess.sh --help
bash scripts/release/preflight.sh --help
bash scripts/release/prepare.sh --help
bash scripts/release/promote.sh --help
bash scripts/release/tag.sh --help
bash scripts/release/monitor.sh --help
bash scripts/release/verify.sh --help
bash scripts/release/closeout.sh --help
```

Post-deploy smoke (API + site):

```bash
curl -sS https://api.eddacraft.ai/api/v1/health
curl -I https://eddacraft.ai/
```

- [Release runbook](docs/runbooks/release-runbook.md)
- [Post-deploy smoke](docs/runbooks/post-deploy-smoke-check.md)
- [Release plan](RELEASE-PLAN.md) · [cadence](docs/policies/release-cadence.md)

### Key pnpm tasks

| Task                                         | What it does                                        |
| -------------------------------------------- | --------------------------------------------------- |
| `pnpm build`                                 | Build all Nx TS targets                             |
| `pnpm test`                                  | JS unit tests + `cargo test --workspace`            |
| `pnpm test:js` / `pnpm test:rust`            | Stack-split tests                                   |
| `pnpm test:e2e:harness`                      | Vitest E2E harness (`apps/e2e`)                     |
| `pnpm test:coverage`                         | TS + Rust coverage (slow; also nightly CI)          |
| `pnpm typecheck`                             | TS typecheck + Rust `check` via Nx                  |
| `pnpm lint` / `pnpm lint:check`              | oxlint + ESLint + Rust + markdownlint               |
| `pnpm format` / `pnpm format:check`          | oxfmt write / CI check                              |
| `pnpm validate:changed`                      | Narrow local gate on git changes                    |
| `pnpm validate:staged`                       | Same, staged only                                   |
| `pnpm validate:full`                         | Full local confidence before PR                     |
| `pnpm docs:check`                            | Docs metadata, links, APS, ADR, indexes             |
| `pnpm docs:index`                            | Regenerate `docs/indexes/`                          |
| `pnpm aps:index`                             | Refresh APS `N/M` counts                            |
| `pnpm aps:index:check`                       | Fail if stored counts drift                         |
| `pnpm aps:active-lint`                       | Lint active APS modules                             |
| `pnpm aps:drift`                             | APS drift check                                     |
| `pnpm adr:check`                             | ADR index integrity                                 |
| `pnpm release-plan:check`                    | `RELEASE-PLAN.md` shape                             |
| `pnpm bench`                                 | Kernel / resource bench harness                     |
| `pnpm ci-log:status` / `pnpm ci-log:harvest` | Continuous-improvement log                          |
| `pnpm agent:run`                             | Local agent-run helper (`tools/local-agent-run.sh`) |
| `pnpm licenses:verify`                       | Third-party acknowledgements freshness              |

### Runbooks worth bookmarking

| Runbook                                                                 | When                                       |
| ----------------------------------------------------------------------- | ------------------------------------------ |
| [admin-cli](docs/runbooks/admin-cli.md)                                 | Waitlist, invite, revoke, audit, admin key |
| [cli-surface](docs/runbooks/cli-surface.md)                             | Full `anvil` command catalogue             |
| [anvil-home-side-by-side](docs/runbooks/anvil-home-side-by-side.md)     | Candidate install root                     |
| [anvil-hook-coexistence](docs/runbooks/anvil-hook-coexistence.md)       | Hooks + Husky / config-mode                |
| [anvil-witness-chain](docs/runbooks/anvil-witness-chain.md)             | Witness break / doctor                     |
| [github-device-flow](docs/runbooks/github-device-flow.md)               | Auth login troubleshooting                 |
| [release-runbook](docs/runbooks/release-runbook.md)                     | Cut a release                              |
| [post-deploy-smoke-check](docs/runbooks/post-deploy-smoke-check.md)     | After deploy                               |
| [emergency-hotfix](docs/runbooks/emergency-hotfix.md)                   | Hotfix path                                |
| [secret-rotation](docs/runbooks/secret-rotation.md)                     | Rotate CI/prod secrets                     |
| [waitlist-email-operations](docs/runbooks/waitlist-email-operations.md) | Email / Resend ops                         |

### Iterate / open a branch

```bash
# Prefer narrow validation while coding
pnpm validate:changed          # or: pnpm validate:staged
pnpm validate:full             # before PR

# Branch work (from main)
wt switch --create feat/<short-name>
```

Testing detail: [`docs/guides/testing.md`](docs/guides/testing.md).

### Performance snapshot

Save-time path is effectively free vs the 100 ms ADR-031 budget (latest clean
quiet-box run **2026-06-26** on the deus reference box):

```
28.3 µs   incremental single-file (reparse + graph + call lift)
1.6 µs    full policy evaluation
6.5 ms    cold graph build, 100 files
~700K/s   parallel anti-pattern scan (320-artefact corpus)
```

History, gates, and how to re-run:

- [`benchmarks/history/2026-06-26.json`](./benchmarks/history/2026-06-26.json)
- [`crates/anvil-bench/README.md`](./crates/anvil-bench/README.md)
- `pnpm bench` / `cargo bench --bench kernel`

---

## Product install (end users)

Get the recommended command for your OS from
[**install.eddacraft.ai**](https://install.eddacraft.ai).

```bash
# Linux / macOS
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh | sh

# macOS
brew install eddacraft/tap/anvil
```

```powershell
# Windows
winget install eddacraft.anvil
# or: irm .../eddacraft-anvil-installer.ps1 | iex
```

First value after install: `anvil welcome` (no login) or `anvil start` (daemon,
beta auth). Details: [Quick Start](./docs/public/anvil/quickstart.md).

On Windows, `anvil update` cannot self-replace while an MCP process holds the
binary — quit the IDE / stop `anvil mcp serve`, then re-run the installer or
`winget upgrade --id eddacraft.anvil`.

---

## Related repos

| Repo                                                                          | Purpose                                                        |
| ----------------------------------------------------------------------------- | -------------------------------------------------------------- |
| [`eddacraft/eddacraft-gtm`](https://github.com/eddacraft/eddacraft-gtm)       | GTM, positioning, competitive radar, marketing bench write-ups |
| [`eddacraft/brand-and-design`](https://github.com/eddacraft/brand-and-design) | Brand and design system                                        |
| [`eddacraft/anvil-plan-spec`](https://github.com/eddacraft/anvil-plan-spec)   | APS format used in `plans/`                                    |

---

## Docs map

| Audience                     | Start here                                                                                    |
| ---------------------------- | --------------------------------------------------------------------------------------------- |
| New users                    | [Quick Start](./docs/public/anvil/quickstart.md)                                              |
| This monorepo (you / agents) | [`CONTEXT.md`](./CONTEXT.md) → [`AGENTS.md`](./AGENTS.md)                                     |
| Contributors                 | [`CONTRIBUTING.md`](./CONTRIBUTING.md)                                                        |
| Operators / release          | [Release runbook](./docs/runbooks/release-runbook.md), [`RELEASE-PLAN.md`](./RELEASE-PLAN.md) |
| Architecture                 | [Overview](./docs/architecture/overview.md)                                                   |
| Planners                     | [`plans/index.aps.md`](./plans/index.aps.md)                                                  |
| Everything else              | [`docs/indexes/README.md`](./docs/indexes/README.md)                                          |

Generated indexes stay current via `pnpm docs:index`. Do not invent parallel
module lists — `plans/index.aps.md` is the only work index.

---

## CI at a glance

| Workflow                                | Role                                                     |
| --------------------------------------- | -------------------------------------------------------- |
| `ci.yml`                                | PR/main TypeScript gates, docs, E2E harness              |
| `rust.yml`                              | clippy, tests, fmt, OPA, cross-platform smoke            |
| `ci-nightly.yml`                        | coverage + Node matrix (coverage is not a PR merge gate) |
| `release.yml` + readiness/sign/homebrew | cargo-dist publish on `v*` tags                          |
| `security.yml` / `codeql.yml`           | audits and static analysis                               |
| `bench.yml` / `bench-nightly.yml`       | kernel benches                                           |

Reusable check action: `.github/actions/anvil-check/`.

Deploy surfaces (docs, website, API) ship via Vercel on `main`; the CLI ships
via GitHub Releases (macOS/Linux/Windows x86_64 + aarch64 where cargo-dist
supports the target). Detail lives in the workflows under `.github/workflows/`.

---

## Conventions (short)

- **UK English** in plans and docs
- **ESM** with `.js` import extensions in TypeScript
- **Zod-first** schemas where TS packages expose contracts
- **Tests co-located** (`file.ts` + `file.test.ts`)
- Multi-step work uses **APS**; implementation on Worktrunk branches from `main`
- **Local secrets** go through **agent-vault**, not plain commits or agent
  Read/Write
