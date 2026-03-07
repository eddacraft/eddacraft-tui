# Apps

Deployable applications and their E2E test suites.

## Structure

```
apps/
├── anvil-cli/      # @eddacraft/anvil-cli - CLI application
├── anvil-api/      # REST/GraphQL API gateway (Hono + Vercel)
├── anvil-ui/       # (future) Web UI for plans, runs, and audits
├── website/        # Next.js marketing website (Vercel)
├── docs-site/      # Docusaurus documentation hub (Vercel)
└── e2e/            # E2E test suites
    └── src/
        ├── adapters/
        ├── api/
        ├── cli/
        ├── contracts/
        ├── core/
        ├── helpers/
        ├── mcp/
        └── smoke/
```

## Applications

### anvil-cli (@eddacraft/anvil-cli)

The Anvil command-line interface for development automation.

```bash
# Build and link globally
pnpm link:cli

# Run directly
npx @eddacraft/anvil-cli --help
anvil check
anvil gate
```

## Migration Status

| App       | Status   | Source                 |
| --------- | -------- | ---------------------- |
| anvil-cli | Complete | cli/ -> apps/anvil-cli |
| anvil-api | Active   | apps/anvil-api         |
| anvil-ui  | Future   | New                    |
| website   | Active   | apps/website           |
| docs-site | Active   | apps/docs-site         |
