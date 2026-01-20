# Apps

Deployable applications and their E2E test suites.

## Structure

```
apps/
├── anvil-cli/      # @eddacraft/anvil-cli - CLI application
├── anvil-api/      # (future) REST/GraphQL API gateway
├── anvil-ui/       # (future) Web UI for plans, runs, and audits
├── website/        # (future) Marketing website
├── docs-site/      # (future) Public documentation (Docusaurus)
└── e2e/            # E2E test suites
    ├── cli-e2e/
    ├── api-e2e/
    └── ui-e2e/
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
| anvil-api | Future   | New                    |
| anvil-ui  | Future   | New                    |
| website   | Future   | New                    |
| docs-site | Future   | New                    |
