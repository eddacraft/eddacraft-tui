# Anvil VS Code Extension (Archived)

> **Archived (2026-04-29) under [ADR-033](../../plans/decisions/033-park-ide-mcp-retire-ts-scanner.md).**
> This package was moved from `packages/vscode-extension/` to
> `archive/anvil-vscode-extension/` because the in-process
> TypeScript scanner it imported is being retired alongside it.
> It is **not built, tested, released, or published to the
> Marketplace**. The `pnpm-workspace.yaml` `'!archive/**'` glob
> excludes it from the active workspace.
>
> **Use this instead.** The `anvil` CLI runs the authoritative
> Rust scanner this extension used to bridge to:
>
> ```bash
> anvil check          # one-shot validation
> anvil watch          # save-time watcher
> anvil mcp install    # configure MCP for Cursor / Claude Code
> ```
>
> **When IDE integration returns.** A new active package will be
> created on the daemon-driver path
> ([DRVR-003](../../plans/modules/surface-drivers.aps.md)) once
> INTD reaches a stable IPC surface, or via another return path
> resolved by a follow-up ADR. This archive is reference material
> for that work; the new package is not expected to be a literal
> un-archive of this one.
>
> **Why archived rather than evolved in place:** The extension
> imports the TS scanner; carrying the TS scanner alive for a
> surface that is not on the current release path is dual-engine
> cost without realised benefit. ADR-033 retires the TS scanner
> now and accepts the IDE gap until a daemon-shaped extension
> lands.
>
> The documentation below describes the pre-archive feature set
> and is preserved for historical context.

---

Deterministic validation and quality gates for AI-generated code plans.

## Features

### Plan Validation

- **Real-time validation** of plan files (SpecKit, BMAD, Generic Markdown, APS
  JSON)
- **Problem panel integration** showing validation errors and warnings
- **Inline diagnostics** with error highlighting

### Quality Gates

- **One-click gate execution** from the editor toolbar or CodeLens
- **Gate results panel** in the Explorer sidebar
- **Support for all gates**: lint, test, coverage, secrets, dependencies

### IDE Integration

- **Status bar indicator** showing validation state
- **CodeLens actions** above plan files for quick access
- **Auto-validation** on file save (configurable)
- **Format detection** for SpecKit, BMAD, and generic markdown plans

## Commands

| Command                            | Description                     |
| ---------------------------------- | ------------------------------- |
| `Anvil: Validate Plan`             | Validate a selected plan file   |
| `Anvil: Validate Current File`     | Validate the active editor file |
| `Anvil: Run Quality Gates`         | Run all quality gates           |
| `Anvil: Run Gates on Current File` | Run gates on the active file    |
| `Anvil: Export Plan`               | Convert plan to another format  |
| `Anvil: Show Output`               | Open the Anvil output channel   |
| `Anvil: Refresh`                   | Refresh the gate results view   |

## Supported File Types

- `*.plan.md` - Plan markdown files
- `plan.md` - Default plan file name
- `*.spec.md` - Specification files
- `*.aps.json` - APS JSON format
- `*prd*.md` - BMAD PRD documents
- `*architecture*.md` - BMAD architecture documents

## Configuration

Configure Anvil in your VS Code settings:

```json
{
  "anvil.autoValidate": true,
  "anvil.validateOnOpen": true,
  "anvil.showStatusBar": true,
  "anvil.showCodeLens": true,
  "anvil.defaultFormat": "auto",
  "anvil.gates.enabled": [
    "lint",
    "test",
    "coverage",
    "secrets",
    "dependencies"
  ],
  "anvil.gates.skipInDevelopment": [],
  "anvil.coverage.threshold": 80,
  "anvil.cli.path": ""
}
```

### Settings Reference

| Setting                         | Default  | Description                  |
| ------------------------------- | -------- | ---------------------------- |
| `anvil.autoValidate`            | `true`   | Auto-validate on save        |
| `anvil.validateOnOpen`          | `true`   | Validate when file is opened |
| `anvil.showStatusBar`           | `true`   | Show status bar item         |
| `anvil.showCodeLens`            | `true`   | Show CodeLens actions        |
| `anvil.defaultFormat`           | `"auto"` | Default format detection     |
| `anvil.gates.enabled`           | all      | Which gates to run           |
| `anvil.gates.skipInDevelopment` | `[]`     | Gates to skip in dev mode    |
| `anvil.coverage.threshold`      | `80`     | Minimum coverage %           |
| `anvil.cli.path`                | `""`     | Custom CLI path              |

## Requirements

- Node.js 20+
- Anvil CLI installed globally (`pnpm add -g`, `npm install -g`,
  `yarn global add`, or `bun add -g @eddacraft/anvil-cli`) or available via npx

## Development

```bash
# Install dependencies
pnpm install

# Build the extension
pnpm build

# Watch for changes
pnpm watch

# Package the extension
pnpm package
```

## Troubleshooting

### CLI Not Found

If you see "anvil command not found", either:

1. Install globally: `npm install -g @eddacraft/anvil-cli` (or pnpm/yarn/bun
   equivalent)
2. Set the CLI path in settings: `anvil.cli.path`

### Validation Not Running

Check that:

1. The file matches a supported pattern (see File Types above)
2. `anvil.autoValidate` is enabled
3. The Anvil output channel shows activity

### Gates Failing

Check the Anvil output channel for detailed error messages. Common issues:

- Missing dependencies (run `npm install`)
- Test configuration issues
- Coverage below threshold

## Contributing

See the main [Anvil repository](https://github.com/eddacraft/anvil-001) for
contribution guidelines.

## Licence

Copyright (c) 2026 eddacraft, Inc. All rights reserved. See [LICENSE](../../LICENSE)
for details.
