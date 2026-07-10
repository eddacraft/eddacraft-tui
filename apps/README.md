# Apps

Deployable applications and documentation surfaces in the Anvil monorepo.

## Structure

```
apps/
├── admin-cli/          # Operator CLI for beta/admin workflows
├── anvil-api/          # REST API (Hono + Vercel)
├── anvil-docs-private/ # Private Docusaurus docs app
├── dashboard/          # Dedicated local React/Vite dashboard host
├── docs-public/        # Public Docusaurus docs app
├── docs-shell/         # Next.js auth/proxy shell for docs.eddacraft.ai
├── docs-site/          # Legacy docs app retained during cutover
├── e2e/                # Vitest E2E harness across product surfaces
└── website/            # Next.js marketing website
```

## Applications

| App                  | Purpose                                                                    |
| -------------------- | -------------------------------------------------------------------------- |
| `admin-cli`          | TypeScript operator CLI for beta-user and migration operations             |
| `anvil-api`          | Beta auth, waitlist, admin, and session API                                |
| `anvil-docs-private` | Private documentation app for gated/internal content                       |
| `dashboard`          | Dedicated local React/Vite dashboard host                                  |
| `docs-public`        | Public Docusaurus documentation for APS, Kindling, and edda-stack          |
| `docs-shell`         | Public entrypoint and auth proxy for docs.eddacraft.ai                     |
| `docs-site`          | Legacy docs app kept during the docs-platform transition                   |
| `e2e`                | Cross-package Vitest harness for CLI, API, contracts, MCP, and smoke tests |
| `website`            | Next.js marketing website                                                  |

## Notes

- The shipped end-user CLI is the Rust binary in `crates/anvil-cli/`.
- The TypeScript CLI now lives in `eddacraft/anvil-archive` at
  `anvil-archive/anvil-cli-node/` and is retained for history only.
