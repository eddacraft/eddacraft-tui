# Command Safety Validation Specification

**Version:** 1.0.0 **Status:** Draft **Created:** 2025-12-28 **Last Updated:**
2025-12-28

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Default Block/Allow Lists](#default-blockallow-lists)
4. [Configuration System](#configuration-system)
5. [Blocking Behavior](#blocking-behavior)
6. [Implementation Details](#implementation-details)
7. [Testing Strategy](#testing-strategy)
8. [Migration Path](#migration-path)

---

## Overview

### Purpose

Provide runtime command safety validation for Anvil plans that execute shell
commands, preventing destructive operations while allowing safe variants. This
creates a defence-in-depth approach alongside Anvil's plan validation.

### Goals

1. **Prevent data loss** from destructive git and filesystem commands
2. **Allow safe variants** of commonly-blocked commands
3. **User configurable** with sensible defaults
4. **Clear explanations** when commands are blocked
5. **Low false positive rate** through semantic analysis

### Non-Goals

- Replace Anvil's gate system (complementary, not replacement)
- Block all potentially dangerous operations (focus on high-risk,
  high-frequency)
- Sandbox or containerise execution (separate concern)

### Inspiration

Based on patterns from
[claude-code-safety-net](https://github.com/kenryu42/claude-code-safety-net),
adapted for Anvil's TypeScript architecture and plan-based workflow.

---

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────┐
│              Anvil Plan (APS)                            │
│  proposed_changes: [                                     │
│    { type: "command", command: "git reset --hard" }     │
│  ]                                                        │
└────────────────────────┬────────────────────────────────┘
                         │
                         ↓
┌─────────────────────────────────────────────────────────┐
│           Gate Runner (Pre-Execution)                    │
│                                                          │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Command Safety Check (NEW)                       │  │
│  │                                                    │  │
│  │  1. Extract shell commands from plan              │  │
│  │  2. Load block/allow rules (default + user config)│  │
│  │  3. Analyse each command semantically             │  │
│  │  4. Check against block/allow lists               │  │
│  │  5. Return pass/fail with explanations            │  │
│  └───────────────────────────────────────────────────┘  │
│                                                          │
│  Other checks: lint, test, coverage, secrets...         │
└────────────────────────┬────────────────────────────────┘
                         │
                         ↓
                    Pass/Fail/Warning
                         │
                         ↓
                 Evidence Bundle
```

### Integration Points

**1. Gate System** (`core/src/gate/`)

- New check: `CommandSafetyCheck`
- Location: `core/src/gate/checks/command-safety.check.ts`
- Type: `GateCheck` interface implementation

**2. Configuration System** (`.anvilrc`)

- New section: `commandSafety`
- User overrides for block/allow lists
- Severity levels (error, warning, off)

**3. Evidence System**

- Blocked commands recorded in evidence
- Suggestions logged for safe alternatives

---

## Default Block/Allow Lists

### Design Principles

1. **Conservative defaults** - Block high-risk operations
2. **Clear rationale** - Every block includes explanation
3. **Safe alternatives** - Suggest safer variants
4. **Semantic analysis** - Distinguish `git checkout -b` (safe) from
   `git checkout --` (destructive)

### Data Structure

```typescript
interface CommandRule {
  // Identity
  id: string; // e.g., "git-reset-hard"
  category: 'git' | 'filesystem' | 'shell' | 'custom';

  // Matching
  command: string; // Base command (e.g., "git")
  subcommand?: string; // Optional subcommand (e.g., "reset")
  flags?: {
    required?: string[]; // Must have these flags to match
    forbidden?: string[]; // Must NOT have these flags
    dangerous?: string[]; // Presence of these makes it dangerous
  };
  args?: {
    pattern?: RegExp; // Regex for argument matching
    position?: number; // Specific argument position
  };

  // Action
  action: 'block' | 'warn' | 'allow';
  severity: 'error' | 'warning' | 'info';

  // Documentation
  reason: string; // Why this is blocked/warned
  suggestion?: string; // What to do instead
  references?: string[]; // Links to docs

  // Conditions
  conditions?: {
    strictModeOnly?: boolean; // Only apply in strict mode
    workingDirectory?: 'home' | 'root' | 'any'; // Path-based conditions
  };
}

interface CommandRuleset {
  version: string;
  rules: CommandRule[];
}
```

### Default Git Operation Rules

```typescript
// core/src/gate/rules/default-git-rules.ts

export const DEFAULT_GIT_RULES: CommandRule[] = [
  // === DESTRUCTIVE OPERATIONS ===

  {
    id: 'git-reset-hard',
    category: 'git',
    command: 'git',
    subcommand: 'reset',
    flags: {
      dangerous: ['--hard'],
    },
    action: 'block',
    severity: 'error',
    reason: 'git reset --hard permanently destroys uncommitted changes',
    suggestion:
      'Use "git stash" first to preserve your work, or "git reset --soft" for a safer alternative',
    references: [
      'https://git-scm.com/docs/git-reset',
      'https://ohshitgit.com/#accidental-commit-wrong-branch',
    ],
  },

  {
    id: 'git-reset-merge',
    category: 'git',
    command: 'git',
    subcommand: 'reset',
    flags: {
      dangerous: ['--merge'],
    },
    action: 'warn',
    severity: 'warning',
    reason:
      'git reset --merge can lose uncommitted changes during conflict resolution',
    suggestion:
      'Ensure all changes are committed or stashed before using --merge',
  },

  {
    id: 'git-checkout-discard',
    category: 'git',
    command: 'git',
    subcommand: 'checkout',
    flags: {
      dangerous: ['--'],
    },
    action: 'block',
    severity: 'error',
    reason: 'git checkout -- discards uncommitted changes permanently',
    suggestion:
      'Use "git stash" to preserve changes, or "git diff" to review first',
    references: ['https://git-scm.com/docs/git-checkout'],
  },

  {
    id: 'git-restore-worktree',
    category: 'git',
    command: 'git',
    subcommand: 'restore',
    flags: {
      dangerous: ['--worktree'],
      forbidden: ['--staged'], // OK if only staging area
    },
    action: 'block',
    severity: 'error',
    reason: 'git restore --worktree discards uncommitted changes permanently',
    suggestion:
      'Use "git stash" first, or "git restore --staged" to only unstage',
  },

  {
    id: 'git-clean-force',
    category: 'git',
    command: 'git',
    subcommand: 'clean',
    flags: {
      dangerous: ['-f', '--force'],
    },
    action: 'warn',
    severity: 'warning',
    reason: 'git clean -f permanently removes untracked files',
    suggestion: 'Preview with "git clean -n" (dry-run) first',
  },

  {
    id: 'git-push-force',
    category: 'git',
    command: 'git',
    subcommand: 'push',
    flags: {
      dangerous: ['-f', '--force'],
      forbidden: ['--force-with-lease'], // Safe variant
    },
    action: 'block',
    severity: 'error',
    reason:
      'git push --force rewrites remote history and can cause data loss for collaborators',
    suggestion:
      'Use "git push --force-with-lease" for safer force pushing, or coordinate with your team',
    references: [
      'https://git-scm.com/docs/git-push#Documentation/git-push.txt---force-with-leaseltrefnamegt',
    ],
  },

  {
    id: 'git-branch-force-delete',
    category: 'git',
    command: 'git',
    subcommand: 'branch',
    flags: {
      dangerous: ['-D'],
    },
    action: 'warn',
    severity: 'warning',
    reason: 'git branch -D force-deletes branches without merge verification',
    suggestion: 'Use "git branch -d" for safe deletion with merge checks',
  },

  {
    id: 'git-stash-drop',
    category: 'git',
    command: 'git',
    subcommand: 'stash',
    args: {
      pattern: /^(drop|clear)$/,
      position: 1,
    },
    action: 'warn',
    severity: 'warning',
    reason: 'git stash drop/clear permanently deletes stashed changes',
    suggestion: 'Review stashed changes with "git stash show" before dropping',
  },

  // === SAFE OPERATIONS (explicit allows to override broader blocks) ===

  {
    id: 'git-checkout-branch',
    category: 'git',
    command: 'git',
    subcommand: 'checkout',
    flags: {
      required: ['-b', '--orphan'], // Branch creation is safe
    },
    action: 'allow',
    severity: 'info',
    reason: 'Branch creation is a safe operation',
  },

  {
    id: 'git-restore-staged',
    category: 'git',
    command: 'git',
    subcommand: 'restore',
    flags: {
      required: ['--staged'], // Only unstaging is safe
      forbidden: ['--worktree'],
    },
    action: 'allow',
    severity: 'info',
    reason: 'Unstaging changes is a safe operation',
  },

  {
    id: 'git-push-force-with-lease',
    category: 'git',
    command: 'git',
    subcommand: 'push',
    flags: {
      required: ['--force-with-lease'],
    },
    action: 'allow',
    severity: 'info',
    reason: 'Force-with-lease is a safer alternative to --force',
  },

  {
    id: 'git-branch-safe-delete',
    category: 'git',
    command: 'git',
    subcommand: 'branch',
    flags: {
      required: ['-d'], // Lowercase -d is safe
      forbidden: ['-D'],
    },
    action: 'allow',
    severity: 'info',
    reason: 'Safe branch deletion with merge verification',
  },

  {
    id: 'git-clean-dry-run',
    category: 'git',
    command: 'git',
    subcommand: 'clean',
    flags: {
      required: ['-n', '--dry-run'],
    },
    action: 'allow',
    severity: 'info',
    reason: 'Dry-run preview is safe',
  },
];
```

### Default Filesystem Rules

```typescript
// core/src/gate/rules/default-filesystem-rules.ts

export const DEFAULT_FILESYSTEM_RULES: CommandRule[] = [
  // === RM -RF OPERATIONS ===

  {
    id: 'rm-rf-recursive-force',
    category: 'filesystem',
    command: 'rm',
    flags: {
      dangerous: ['-r', '-f', '--recursive', '--force'],
    },
    action: 'block',
    severity: 'error',
    reason: 'rm -rf is destructive and can cause irreversible data loss',
    suggestion:
      'List files first with "ls -la", review carefully, then delete individually or use a safer method',
    conditions: {
      strictModeOnly: false,
    },
  },

  {
    id: 'rm-rf-root',
    category: 'filesystem',
    command: 'rm',
    flags: {
      dangerous: ['-r', '-f'],
    },
    args: {
      pattern: /^(\/|~|~\/|\$HOME|\${HOME})/,
    },
    action: 'block',
    severity: 'error',
    reason: 'rm -rf on root or home paths is extremely dangerous',
    suggestion:
      'NEVER delete root or home directories. Review the target path carefully.',
  },

  {
    id: 'rm-rf-parent-traversal',
    category: 'filesystem',
    command: 'rm',
    flags: {
      dangerous: ['-r', '-f'],
    },
    args: {
      pattern: /\.\./,
    },
    action: 'block',
    severity: 'error',
    reason:
      'rm -rf with parent directory traversal (..) can escape current directory',
    suggestion: 'Use absolute paths or paths within the current directory only',
  },

  {
    id: 'rm-rf-current-dir',
    category: 'filesystem',
    command: 'rm',
    flags: {
      dangerous: ['-r', '-f'],
    },
    args: {
      pattern: /^\.$/,
    },
    action: 'block',
    severity: 'error',
    reason: 'rm -rf . deletes the entire current directory',
    suggestion: 'Specify exact subdirectories or files instead',
  },

  // === SAFE EXCEPTIONS ===

  {
    id: 'rm-rf-tmp-dir',
    category: 'filesystem',
    command: 'rm',
    flags: {
      dangerous: ['-r', '-f'],
    },
    args: {
      pattern: /^(\/tmp|\/var\/tmp|\$TMPDIR|\${TMPDIR})\//,
    },
    action: 'allow',
    severity: 'info',
    reason: 'Temporary directory deletion is safe',
  },

  {
    id: 'rm-rf-common-build-dirs',
    category: 'filesystem',
    command: 'rm',
    flags: {
      dangerous: ['-r', '-f'],
    },
    args: {
      pattern:
        /^(\.\/|)?(node_modules|dist|build|target|\.next|\.cache|coverage)$/,
    },
    action: 'allow',
    severity: 'info',
    reason:
      'Common build/cache directory deletion is safe (reproducible artifacts)',
    conditions: {
      strictModeOnly: false,
    },
  },
];
```

### Default Shell Wrapper Rules

```typescript
// core/src/gate/rules/default-shell-rules.ts

export const DEFAULT_SHELL_RULES: CommandRule[] = [
  {
    id: 'shell-wrapper-recursive-check',
    category: 'shell',
    command: 'bash',
    subcommand: '-c',
    action: 'allow', // Allow but analyse the wrapped command recursively
    severity: 'info',
    reason: 'Shell wrappers are analysed recursively',
  },

  {
    id: 'interpreter-one-liner',
    category: 'shell',
    command: 'python',
    subcommand: '-c',
    args: {
      pattern: /system|exec|subprocess/,
    },
    action: 'warn',
    severity: 'warning',
    reason: 'Python one-liners executing shell commands should be reviewed',
    suggestion: 'Review the command being executed for safety',
  },
];
```

---

## Configuration System

### Configuration File Schema

```typescript
// .anvilrc or .anvil/config.json

interface CommandSafetyConfig {
  commandSafety: {
    // Enable/disable the check
    enabled: boolean;

    // Strict mode (block unparseable commands)
    strict: boolean;

    // Rule customisation
    rules: {
      // Override default rules
      overrides?: Array<{
        id: string; // Rule ID to override
        action?: 'block' | 'warn' | 'allow' | 'disable';
        severity?: 'error' | 'warning' | 'info';
      }>;

      // Add custom rules
      custom?: CommandRule[];

      // Disable specific rules by ID
      disabled?: string[];
    };

    // Working directory restrictions
    workingDirectory?: {
      allowDeleteInCwd?: boolean; // Allow rm -rf in current working directory
      tempDirPatterns?: string[]; // Additional temp directory patterns
    };

    // Output customisation
    output: {
      verbose?: boolean; // Include full command in error messages
      showSuggestions?: boolean; // Show safe alternatives
      showReferences?: boolean; // Show reference links
    };
  };
}
```

### Example User Configuration

```json
// .anvilrc
{
  "commandSafety": {
    "enabled": true,
    "strict": false,

    "rules": {
      // Override: Allow force push in this repo (careful!)
      "overrides": [
        {
          "id": "git-push-force",
          "action": "warn",
          "severity": "warning"
        }
      ],

      // Disable specific rules
      "disabled": [
        "git-clean-force" // We know what we're doing
      ],

      // Add custom rules
      "custom": [
        {
          "id": "custom-no-docker-rmi",
          "category": "custom",
          "command": "docker",
          "subcommand": "rmi",
          "action": "warn",
          "severity": "warning",
          "reason": "Docker image removal should be reviewed",
          "suggestion": "Use 'docker image prune' for safe cleanup"
        }
      ]
    },

    "workingDirectory": {
      "allowDeleteInCwd": true,
      "tempDirPatterns": ["/tmp", "/var/tmp", "$TMPDIR", "./.anvil/cache"]
    },

    "output": {
      "verbose": true,
      "showSuggestions": true,
      "showReferences": true
    }
  }
}
```

### Configuration Loading Priority

1. **Default rules** (shipped with Anvil)
2. **User config overrides** (`.anvilrc` or `.anvil/config.json`)
3. **Environment variables** (`ANVIL_COMMAND_SAFETY_STRICT`)
4. **CLI flags** (`--skip-command-safety`, `--command-safety-strict`)

```typescript
// Configuration merge logic
function loadCommandSafetyConfig(): ResolvedCommandSafetyConfig {
  const defaults = loadDefaultRules();
  const userConfig = loadUserConfig();
  const envConfig = loadEnvConfig();
  const cliConfig = loadCliConfig();

  return merge(defaults, userConfig, envConfig, cliConfig);
}
```

---

## Blocking Behavior

### Check Execution Flow

```typescript
// core/src/gate/checks/command-safety.check.ts

async run(plan: APSPlan, config: CommandSafetyConfig): Promise<CheckResult> {
  // 1. Extract commands from plan
  const commands = this.extractCommands(plan);

  // 2. Load rules (default + user config)
  const rules = this.loadRules(config);

  // 3. Analyse each command
  const results: CommandAnalysisResult[] = [];
  for (const cmd of commands) {
    const analysis = await this.analyseCommand(cmd, rules);
    results.push(analysis);
  }

  // 4. Aggregate results
  const blocked = results.filter(r => r.action === 'block');
  const warned = results.filter(r => r.action === 'warn');

  // 5. Return check result
  return {
    check: 'command-safety',
    status: blocked.length > 0 ? 'failed' : warned.length > 0 ? 'warning' : 'passed',
    message: this.formatSummary(blocked, warned),
    details: {
      blocked: blocked.map(this.formatBlockedCommand),
      warnings: warned.map(this.formatWarning),
      total: commands.length,
    },
  };
}
```

### Command Analysis Process

```typescript
interface CommandAnalysisResult {
  command: string;
  parsedCommand: ParsedCommand;
  matchedRule?: CommandRule;
  action: 'allow' | 'warn' | 'block';
  severity: 'error' | 'warning' | 'info';
  reason?: string;
  suggestion?: string;
  references?: string[];
}

function analyseCommand(
  cmd: string,
  rules: CommandRule[]
): CommandAnalysisResult {
  // 1. Parse command (unwrap shell wrappers)
  const parsed = parseCommand(cmd);

  // 2. Match against rules (most specific first)
  const matchedRule = findMatchingRule(parsed, rules);

  // 3. Determine action
  if (!matchedRule) {
    return {
      command: cmd,
      parsedCommand: parsed,
      action: 'allow',
      severity: 'info',
    };
  }

  // 4. Check conditions (strict mode, working directory, etc.)
  const actionable = evaluateConditions(matchedRule, context);

  return {
    command: cmd,
    parsedCommand: parsed,
    matchedRule,
    action: actionable ? matchedRule.action : 'allow',
    severity: matchedRule.severity,
    reason: matchedRule.reason,
    suggestion: matchedRule.suggestion,
    references: matchedRule.references,
  };
}
```

### Shell Wrapper Unwrapping

```typescript
function unwrapCommand(cmd: string, depth = 0): string {
  if (depth > 5) return cmd; // Prevent infinite recursion

  const trimmed = cmd.trim();

  // Shell wrappers
  const shellPatterns = [
    /^bash\s+-c\s+["'](.+)["']$/,
    /^sh\s+-c\s+["'](.+)["']$/,
    /^env\s+\w+=\w+\s+(.+)$/,
    /^sudo\s+(.+)$/,
    /^command\s+(.+)$/,
  ];

  for (const pattern of shellPatterns) {
    const match = trimmed.match(pattern);
    if (match?.[1]) {
      return unwrapCommand(match[1], depth + 1);
    }
  }

  // Interpreter one-liners
  const interpreterPatterns = [
    /python\s+-c\s+["'].*?system\(["'](.+?)["']\)/,
    /node\s+-e\s+["'].*?exec\(["'](.+?)["']\)/,
  ];

  for (const pattern of interpreterPatterns) {
    const match = trimmed.match(pattern);
    if (match?.[1]) {
      return unwrapCommand(match[1], depth + 1);
    }
  }

  return trimmed;
}
```

### Output Formatting

#### Blocked Command Message

```
❌ Command Safety Check Failed

Blocked 2 dangerous command(s):

1. git reset --hard
   ├─ Reason: git reset --hard permanently destroys uncommitted changes
   ├─ Suggestion: Use "git stash" first to preserve your work, or "git reset --soft" for a safer alternative
   └─ Reference: https://git-scm.com/docs/git-reset

2. rm -rf ~/projects
   ├─ Reason: rm -rf on root or home paths is extremely dangerous
   ├─ Suggestion: NEVER delete root or home directories. Review the target path carefully.
   └─ Reference: https://en.wikipedia.org/wiki/Rm_(Unix)#Deletion_of_a_user's_home_directory

To override this check:
  - Review the commands carefully
  - Use the suggested safer alternatives
  - Or configure .anvilrc to allow these specific operations (not recommended)
  - Or skip this check: anvil gate --skip-checks=command-safety
```

#### Warning Message

```
⚠️  Command Safety Warnings

Found 1 potentially dangerous command(s):

1. git branch -D old-feature
   ├─ Reason: git branch -D force-deletes branches without merge verification
   └─ Suggestion: Use "git branch -d" for safe deletion with merge checks

These warnings do not block execution but should be reviewed.
```

### CLI Integration

```bash
# Standard gate run (includes command safety)
anvil gate plan.md

# Skip command safety check
anvil gate plan.md --skip-checks=command-safety

# Enable strict mode via CLI
anvil gate plan.md --command-safety-strict

# Show detailed analysis
anvil gate plan.md --verbose
```

---

## Implementation Details

### File Structure

```
core/src/gate/
├── checks/
│   └── command-safety.check.ts        # Main check implementation
├── rules/
│   ├── types.ts                       # CommandRule interface
│   ├── default-git-rules.ts           # Git operation rules
│   ├── default-filesystem-rules.ts    # Filesystem operation rules
│   ├── default-shell-rules.ts         # Shell wrapper rules
│   ├── rule-matcher.ts                # Rule matching engine
│   └── index.ts                       # Exports
├── parsers/
│   ├── command-parser.ts              # Shell command parsing
│   ├── git-parser.ts                  # Git-specific parsing
│   ├── filesystem-parser.ts           # Filesystem-specific parsing
│   └── wrapper-unwrapper.ts           # Shell wrapper detection
└── formatters/
    ├── message-formatter.ts           # Error/warning messages
    └── evidence-formatter.ts          # Evidence bundle formatting
```

### Core Classes

```typescript
// core/src/gate/checks/command-safety.check.ts

export class CommandSafetyCheck implements GateCheck {
  name = 'command-safety';
  description = 'Validates shell commands for destructive operations';

  constructor(
    private ruleLoader: RuleLoader,
    private commandParser: CommandParser,
    private ruleMatcher: RuleMatcher,
    private messageFormatter: MessageFormatter
  ) {}

  async run(plan: APSPlan, config?: CommandSafetyConfig): Promise<CheckResult> {
    // Implementation
  }

  private extractCommands(plan: APSPlan): ExtractedCommand[] {
    // Extract from proposed_changes, metadata.executionSteps, etc.
  }

  private async analyseCommand(
    cmd: ExtractedCommand,
    rules: CommandRule[]
  ): Promise<CommandAnalysisResult> {
    // Parse, unwrap, match rules, evaluate
  }
}
```

```typescript
// core/src/gate/rules/rule-matcher.ts

export class RuleMatcher {
  findMatchingRule(
    parsed: ParsedCommand,
    rules: CommandRule[]
  ): CommandRule | undefined {
    // Sort rules by specificity (most specific first)
    const sorted = this.sortBySpecificity(rules);

    // Find first matching rule
    for (const rule of sorted) {
      if (this.matches(parsed, rule)) {
        return rule;
      }
    }

    return undefined;
  }

  private matches(parsed: ParsedCommand, rule: CommandRule): boolean {
    // 1. Command match
    if (parsed.command !== rule.command) return false;

    // 2. Subcommand match (if specified)
    if (rule.subcommand && parsed.subcommand !== rule.subcommand) return false;

    // 3. Flag match
    if (rule.flags) {
      if (!this.matchFlags(parsed.flags, rule.flags)) return false;
    }

    // 4. Argument match
    if (rule.args) {
      if (!this.matchArgs(parsed.args, rule.args)) return false;
    }

    return true;
  }

  private sortBySpecificity(rules: CommandRule[]): CommandRule[] {
    // Specificity score: command (1) + subcommand (2) + flags (4) + args (8)
    return rules.sort((a, b) => {
      const scoreA = this.calculateSpecificity(a);
      const scoreB = this.calculateSpecificity(b);
      return scoreB - scoreA; // Most specific first
    });
  }
}
```

```typescript
// core/src/gate/parsers/command-parser.ts

export interface ParsedCommand {
  raw: string;
  command: string;
  subcommand?: string;
  flags: string[];
  args: string[];
  unwrapped: string;
  wrapperChain: string[]; // ['bash', 'python', 'git']
}

export class CommandParser {
  parse(cmd: string): ParsedCommand {
    // 1. Unwrap shell wrappers
    const { unwrapped, wrappers } = this.unwrap(cmd);

    // 2. Tokenise using shell-quote
    const tokens = parseCommand(unwrapped);

    // 3. Extract command, subcommand, flags, args
    const [command, ...rest] = tokens.map(String);
    const flags = rest.filter((t) => t.startsWith('-'));
    const args = rest.filter((t) => !t.startsWith('-'));
    const subcommand = args[0]; // First non-flag arg is usually subcommand

    return {
      raw: cmd,
      command,
      subcommand,
      flags,
      args,
      unwrapped,
      wrapperChain: wrappers,
    };
  }

  private unwrap(
    cmd: string,
    depth = 0
  ): { unwrapped: string; wrappers: string[] } {
    if (depth > 5) return { unwrapped: cmd, wrappers: [] };

    // Detect and unwrap
    // ... implementation
  }
}
```

### Dependencies

```json
// package.json additions
{
  "dependencies": {
    "shell-quote": "^1.8.1", // Shell command parsing
    "minimist": "^1.2.8" // Flag parsing
  }
}
```

---

## Testing Strategy

### Test Coverage Goals

- **Rule matching:** 100% coverage of all default rules
- **Command parsing:** All wrapper types, edge cases
- **Configuration:** Override, disable, custom rules
- **Integration:** End-to-end gate execution

### Test Structure

```
core/src/gate/checks/__tests__/
├── command-safety.check.test.ts       # Main check tests
├── git-rules.test.ts                  # Git-specific rule tests
├── filesystem-rules.test.ts           # Filesystem rule tests
├── rule-matcher.test.ts               # Rule matching engine tests
├── command-parser.test.ts             # Command parsing tests
├── wrapper-unwrapper.test.ts          # Wrapper detection tests
└── fixtures/
    ├── sample-plans/                  # APS plans with commands
    ├── dangerous-commands.ts          # Known dangerous commands
    └── safe-commands.ts               # Known safe commands
```

### Key Test Cases

```typescript
// Git operation tests (from safety-net)
describe('Git Operation Rules', () => {
  describe('destructive operations', () => {
    it('blocks git reset --hard', async () => {
      const result = await analyseCommand('git reset --hard', rules);
      expect(result.action).toBe('block');
      expect(result.reason).toContain('destroys uncommitted changes');
    });

    it('blocks git checkout -- file.txt', async () => {
      const result = await analyseCommand('git checkout -- file.txt', rules);
      expect(result.action).toBe('block');
    });

    it('blocks git push --force', async () => {
      const result = await analyseCommand(
        'git push --force origin main',
        rules
      );
      expect(result.action).toBe('block');
    });

    it('blocks force push short form (-f)', async () => {
      const result = await analyseCommand('git push -f', rules);
      expect(result.action).toBe('block');
    });
  });

  describe('safe operations', () => {
    it('allows git checkout -b (branch creation)', async () => {
      const result = await analyseCommand('git checkout -b new-feature', rules);
      expect(result.action).toBe('allow');
    });

    it('allows git push --force-with-lease', async () => {
      const result = await analyseCommand('git push --force-with-lease', rules);
      expect(result.action).toBe('allow');
    });
  });

  describe('shell wrapper detection', () => {
    it('detects git reset --hard in bash -c', async () => {
      const result = await analyseCommand('bash -c "git reset --hard"', rules);
      expect(result.action).toBe('block');
      expect(result.parsedCommand.wrapperChain).toContain('bash');
    });

    it('detects nested wrappers', async () => {
      const cmd = 'sudo env VAR=1 bash -c "git reset --hard"';
      const result = await analyseCommand(cmd, rules);
      expect(result.action).toBe('block');
      expect(result.parsedCommand.wrapperChain).toEqual([
        'sudo',
        'env',
        'bash',
      ]);
    });
  });
});

// Filesystem operation tests
describe('Filesystem Rules', () => {
  describe('rm -rf blocking', () => {
    it('blocks rm -rf /some/path', async () => {
      const result = await analyseCommand('rm -rf /some/path', rules);
      expect(result.action).toBe('block');
    });

    it('blocks rm -rf ~/projects', async () => {
      const result = await analyseCommand('rm -rf ~/projects', rules);
      expect(result.action).toBe('block');
      expect(result.reason).toContain('extremely dangerous');
    });

    it('blocks parent traversal', async () => {
      const result = await analyseCommand('rm -rf ../other', rules);
      expect(result.action).toBe('block');
    });
  });

  describe('safe exceptions', () => {
    it('allows rm -rf /tmp/test-dir', async () => {
      const result = await analyseCommand('rm -rf /tmp/test-dir', rules);
      expect(result.action).toBe('allow');
    });

    it('allows rm -rf node_modules', async () => {
      const result = await analyseCommand('rm -rf node_modules', rules);
      expect(result.action).toBe('allow');
    });

    it('allows rm -rf dist', async () => {
      const result = await analyseCommand('rm -rf dist', rules);
      expect(result.action).toBe('allow');
    });
  });
});

// Configuration tests
describe('Configuration System', () => {
  it('loads default rules', () => {
    const config = loadCommandSafetyConfig({});
    expect(config.rules.length).toBeGreaterThan(0);
  });

  it('merges user overrides', () => {
    const config = loadCommandSafetyConfig({
      rules: {
        overrides: [{ id: 'git-push-force', action: 'warn' }],
      },
    });

    const rule = config.rules.find((r) => r.id === 'git-push-force');
    expect(rule?.action).toBe('warn');
  });

  it('disables specific rules', () => {
    const config = loadCommandSafetyConfig({
      rules: {
        disabled: ['git-clean-force'],
      },
    });

    const rule = config.rules.find((r) => r.id === 'git-clean-force');
    expect(rule).toBeUndefined();
  });

  it('adds custom rules', () => {
    const config = loadCommandSafetyConfig({
      rules: {
        custom: [
          {
            id: 'custom-rule',
            category: 'custom',
            command: 'docker',
            subcommand: 'rmi',
            action: 'warn',
            severity: 'warning',
            reason: 'Test',
          },
        ],
      },
    });

    const rule = config.rules.find((r) => r.id === 'custom-rule');
    expect(rule).toBeDefined();
  });
});
```

### Test Data Fixtures

```typescript
// core/src/gate/checks/__tests__/fixtures/dangerous-commands.ts

export const DANGEROUS_GIT_COMMANDS = [
  'git reset --hard',
  'git reset --hard HEAD~1',
  'git checkout -- .',
  'git checkout -- file.txt',
  'git restore file.txt',
  'git clean -f',
  'git push --force',
  'git push -f origin main',
  'git branch -D feature',
  'git stash drop',
  'git stash clear',

  // With wrappers
  'bash -c "git reset --hard"',
  'sudo git reset --hard',
  'env VAR=1 git reset --hard',
  'python -c "import os; os.system(\'git reset --hard\')"',
];

export const DANGEROUS_RM_COMMANDS = [
  'rm -rf /',
  'rm -rf ~',
  'rm -rf ~/projects',
  'rm -rf $HOME',
  'rm -rf /some/path',
  'rm -rf ../other',
  'rm -rf .',

  // With wrappers
  'bash -c "rm -rf /tmp/../home/user"',
  'sudo rm -rf /var',
];

export const SAFE_COMMANDS = [
  'git checkout -b new-feature',
  'git push --force-with-lease',
  'git branch -d merged-feature',
  'git clean -n',
  'git restore --staged file.txt',
  'git reset --soft HEAD~1',

  'rm -rf /tmp/test',
  'rm -rf node_modules',
  'rm -rf dist',
  'rm -rf build',

  'npm install',
  'pnpm build',
  'ls -la',
];
```

---

## Migration Path

### Phase 1: Core Implementation (Week 1)

**Tasks:**

- [ ] Implement `CommandRule` type system
- [ ] Create default git operation rules
- [ ] Create default filesystem rules
- [ ] Implement `CommandParser` with wrapper unwrapping
- [ ] Implement `RuleMatcher` with specificity sorting
- [ ] Basic test suite (50+ tests)

**Deliverables:**

- `core/src/gate/rules/` - Rule definitions
- `core/src/gate/parsers/` - Command parsing
- Tests covering all default rules

### Phase 2: Gate Integration (Week 1-2)

**Tasks:**

- [ ] Implement `CommandSafetyCheck` class
- [ ] Integrate with gate runner
- [ ] Command extraction from APS plans
- [ ] Message formatting and output
- [ ] Integration tests with real plans

**Deliverables:**

- `core/src/gate/checks/command-safety.check.ts`
- CLI support: `anvil gate plan.md` includes command safety
- Evidence bundle integration

### Phase 3: Configuration System (Week 2)

**Tasks:**

- [ ] Configuration schema in `.anvilrc`
- [ ] Config loader with merge logic
- [ ] Override mechanism
- [ ] Custom rule support
- [ ] Environment variable support

**Deliverables:**

- Config loading infrastructure
- User documentation
- Example configurations

### Phase 4: Polish & Documentation (Week 2-3)

**Tasks:**

- [ ] Comprehensive user guide
- [ ] CLI help text
- [ ] Error message refinement
- [ ] Performance optimisation
- [ ] Extended test coverage (100+ tests)

**Deliverables:**

- User guide: `docs/guides/command-safety.md`
- API documentation
- Examples and best practices

### Phase 5: Advanced Features (Future)

**Tasks:**

- [ ] Path resolution (check if rm target is actually in /tmp)
- [ ] Git context awareness (detect if in rebase/merge)
- [ ] Interactive mode (ask user for confirmation)
- [ ] Machine learning for pattern detection
- [ ] Integration with IDE extensions

---

## Success Metrics

### Effectiveness

- **Zero false negatives:** Catch 100% of known dangerous patterns
- **Low false positives:** <5% false positive rate on real-world plans
- **User satisfaction:** >80% of users find it helpful (survey)

### Performance

- **Fast execution:** <50ms overhead per command analysed
- **Scalable:** Handle plans with 100+ commands efficiently

### Adoption

- **Default enabled:** Included in default gate configuration
- **Low opt-out rate:** <10% of users disable the check
- **Custom rules:** >20% of users add custom rules (shows engagement)

---

## Appendices

### Appendix A: Command Rule Examples

See [Default Block/Allow Lists](#default-blockallow-lists) section.

### Appendix B: Configuration Examples

See [Configuration System](#configuration-system) section.

### Appendix C: Related Documentation

- _claude-code-safety-net Review (referenced doc never landed)_
- [Gate System Architecture](../../architecture/overview.md) _(gate-layer section was renamed/moved post-archive)_
- _APS Specification (was `core/src/schema/aps.schema.ts` — schema moved to `packages/aps/src/`)_

---

**Document Status:** Draft for Review **Next Steps:** Team review, then proceed
with Phase 1 implementation
