# Apps

Deployable applications and their E2E test suites.

## Structure

```
apps/
├── anvil-cli/      # CLI application (to be migrated from cli/)
├── anvil-api/      # REST/GraphQL API gateway
├── anvil-ui/       # Web UI for plans, runs, and audits
├── website/        # Marketing website
├── docs-site/      # Public documentation (Docusaurus)
└── e2e/            # E2E test suites
    ├── cli-e2e/
    ├── api-e2e/
    ├── ui-e2e/
    ├── website-e2e/
    ├── docs-e2e/
    └── oss-compat-e2e/
```

## Migration Status

| App       | Status      | Source |
| --------- | ----------- | ------ |
| anvil-cli | Planned     | `cli/` |
| anvil-api | Placeholder | New    |
| anvil-ui  | Placeholder | New    |
| website   | Placeholder | New    |
| docs-site | Placeholder | New    |

## Notes

These directories are placeholders for the v1.1 monorepo restructure. The
current CLI remains at `cli/` until migration is complete.
