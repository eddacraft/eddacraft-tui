# CLI Package (@eddacraft/anvil-cli)

> Commander.js CLI with Ink-based TUI, format detection, plan loading

**Parent**: See root `AGENTS.md` for project-wide conventions.

## Structure

```
src/
├── commands/           # CLI commands (24+ files)
│   ├── architecture.ts # Architecture analysis
│   ├── audit.ts        # Audit trail
│   ├── authorship.ts   # AI authorship tracking
│   ├── beta.ts         # Beta authentication
│   ├── check.ts        # Anti-pattern checking
│   ├── doctor.ts       # Diagnostic checks
│   ├── drift.ts        # Drift detection
│   ├── explain.ts      # Rule explanation
│   ├── export.ts       # Format conversion
│   ├── gate.ts         # Quality gate execution
│   ├── gate-config.ts  # Gate configuration
│   ├── hooks.ts        # Git hooks management
│   ├── init.ts         # Project initialisation
│   ├── login.ts        # Authentication
│   ├── logout.ts       # Sign out
│   ├── mcp-config.ts   # MCP server configuration
│   ├── new.ts          # Project scaffolding
│   ├── plan.ts         # Plan subcommands
│   ├── policy.ts       # OPA policy management
│   ├── release.ts      # Release management
│   ├── stack.ts        # Edda stack management
│   ├── status.ts       # Workspace status
│   ├── tutorial.ts     # Interactive tutorial
│   ├── validate.ts     # Plan validation
│   ├── watch.ts        # File watching
│   ├── welcome.ts      # Welcome screen
│   ├── whoami.ts       # Current user info
│   └── plan/           # Plan subcommands (load, lock, status, unlock, validate)
├── services/           # Business logic (33 files)
│   ├── format-detection.ts  # Adapter-based detection
│   ├── plan-loader.ts       # Multi-format loading
│   └── environment.ts       # Runtime detection
├── tui/                # Terminal UI (Ink/React)
│   ├── components/     # Reusable components (13 files)
│   └── commands/       # Full-screen TUI views
├── types/              # TypeScript definitions
└── index.ts            # Entry point with shebang
```

## Where to Look

| Task              | Location                       | Notes                                        |
| ----------------- | ------------------------------ | -------------------------------------------- |
| Add CLI command   | `commands/`                    | Use create{Name}Command() factory            |
| Add service       | `services/`                    | Implement interface from `types/services.js` |
| Add TUI component | `tui/components/`              | Ink/React with useInput hook                 |
| Format detection  | `services/format-detection.ts` | Uses AdapterRegistry                         |
| Plan loading      | `services/plan-loader.ts`      | Multi-format with validation                 |

## Command Pattern

All commands use factory functions returning Commander.js Command:

```typescript
import { Command } from 'commander';

export function createMyCommand(): Command {
  const command = new Command('my-command')
    .description('Does something useful')
    .option('--json', 'Output as JSON')
    .option('--no-tui', 'Disable TUI mode')
    .action(async (options) => {
      const useTUI = isTUIAvailable({ tui: options.tui, noTui: options.noTui });

      if (useTUI) {
        // Render Ink component
        renderTUI(MyComponent, { onComplete: handleComplete });
      } else {
        // Plain text execution
        const result = await runPlain();
        console.log(formatResult(result, options.json));
      }
    });
  return command;
}
```

Register in `index.ts` → `program.addCommand(createMyCommand())`.

## TUI Component Pattern

Ink/React components with keyboard navigation:

```typescript
import { Box, Text, useInput, useApp } from 'ink';
import { theme } from '../theme.js';

interface MyComponentProps {
  onComplete: (result: Result) => void;
}

export function MyComponent({ onComplete }: MyComponentProps) {
  const [selected, setSelected] = useState(0);
  const { exit } = useApp();

  useInput((input, key) => {
    if (key.upArrow) setSelected(s => Math.max(0, s - 1));
    if (key.downArrow) setSelected(s => Math.min(items.length - 1, s + 1));
    if (key.return) {
      onComplete(items[selected]);
      exit();
    }
  });

  return (
    <Box flexDirection="column">
      <Text color={theme.colours.steel}>Select an option:</Text>
      {items.map((item, i) => (
        <Text key={i} color={i === selected ? theme.colours.ember : theme.colours.ash}>
          {i === selected ? '>' : ' '} {item.label}
        </Text>
      ))}
    </Box>
  );
}
```

## Service Pattern

Services use interface-first design with dependency injection:

```typescript
import type { IFormatDetectionService } from '../types/services.js';
import { AdapterRegistry } from '@eddacraft/anvil-adapters';

export class FormatDetectionService implements IFormatDetectionService {
  private registry: AdapterRegistry;

  constructor(options?: { minConfidence?: number }) {
    this.registry = AdapterRegistry.getInstance();
    this.minConfidence = options?.minConfidence ?? 0.5;
  }

  async detectFormat(content: string): Promise<FormatDetectionResult | null> {
    const detected = this.registry.detectAdapter(content, this.minConfidence);
    return detected ? { format: detected.adapter.metadata.name, ... } : null;
  }
}
```

## Theme System

Centralised theme in `tui/theme.ts`:

```typescript
export const theme = {
  colours: {
    steel: '#6B7280', // Primary text
    ember: '#F59E0B', // Highlights, selections
    ash: '#9CA3AF', // Secondary text
    smoke: '#4B5563', // Disabled, muted
    slag: '#EF4444', // Errors, warnings
  },
  icons: {
    success: '✓',
    error: '✗',
    warning: '⚠',
    info: 'ℹ',
  },
};
```

## Scripts

```bash
pnpm link:cli             # Build and link globally
pnpm unlink:cli           # Unlink when done
nx test cli               # Run CLI tests
node dist/index.js --help # Test without linking
```

## Anti-Patterns (This Package)

- Never use `console.log` directly - use ora spinners or chalk formatting
- Never block TUI with synchronous operations
- Always provide `--json` option for CI/CD integration
- Always handle both TUI and plain text modes

## Testing

- Integration tests in `__tests__/cli-*.test.ts`
- Mock file system with `memfs` or temp directories
- Test both TUI and non-TUI code paths
