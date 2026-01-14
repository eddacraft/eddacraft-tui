# Claude Code Configuration

This project includes an enhanced Claude Code setup with custom agents, hooks,
commands, and skills.

## Getting Started

```bash
# Start a Claude Code session
claude

# Or with a specific task
claude "help me understand this codebase"
```

## Available Commands

| Command         | Description                                   |
| --------------- | --------------------------------------------- |
| `/think-harder` | Deep analytical thinking for complex problems |
| `/plan`         | Create a detailed implementation plan         |
| `/review`       | Comprehensive code review of recent changes   |
| `/commit`       | Stage and commit with conventional format     |
| `/test`         | Run tests and fix any failures                |
| `/debug`        | Systematic debugging with root cause analysis |
| `/delegate`     | Delegate task to GPT expert via Codex         |
| `/autonomous`   | Execute long-running tasks autonomously       |

## Custom Agents

Agents are specialized assistants that can be invoked for specific tasks:

| Agent              | Use Case                                                 |
| ------------------ | -------------------------------------------------------- |
| `architect`        | System design, architecture review, technology decisions |
| `code-reviewer`    | Code quality analysis, PR review, bug detection          |
| `security-analyst` | Vulnerability assessment, security testing               |
| `tdd-coach`        | Test-driven development guidance                         |
| `debugger`         | Systematic debugging, error analysis                     |
| `autonomous`       | Long-running multi-step workflows                        |
| `planner`          | Implementation planning, task breakdown                  |

## Hooks

Hooks run automatically during Claude Code sessions:

| Hook                | When             | Purpose                        |
| ------------------- | ---------------- | ------------------------------ |
| `session-start.sh`  | Session begins   | Environment validation         |
| `security-guard.sh` | Before tool use  | Block dangerous commands       |
| `tdd-guard.sh`      | Before edits     | Enforce test-first development |
| `post-edit.sh`      | After file edits | Auto-format and lint           |
| `on-stop.sh`        | Session ends     | Logging and notifications      |

### TDD Guard Configuration

Control test-driven development enforcement with environment variables:

```bash
# Strict mode: block edits if no test file exists
export CLAUDE_TDD_STRICT=true

# Run tests before allowing edits
export CLAUDE_TDD_RUN_TESTS=true
```

## Skills

Skills provide domain expertise that Claude can reference:

- **test-driven-development** - TDD workflow, red-green-refactor
- **systematic-debugging** - Scientific debugging methodology
- **code-review** - Review best practices
- **security-analysis** - Vulnerability assessment
- **parallel-agents** - Multi-agent coordination
- **autonomous-execution** - Long-running task management

## Directory Structure

```
.claude/
├── settings.json     # Permissions, limits, MCP config
├── agents/           # Custom AI assistants
├── commands/         # Slash commands
├── hooks/            # Lifecycle automation
├── skills/           # Domain knowledge
├── prompts/          # Expert system prompts
└── logs/             # Session logs
```

## Customization

### Edit CLAUDE.md

The `CLAUDE.md` file in your project root contains instructions that Claude
follows. Customize it for your project's specific needs, conventions, and
workflows.

### Add Custom Commands

Create `.claude/commands/<name>.md`:

```yaml
---
name: my-command
description: What it does
---
Your command instructions here.

$ARGUMENTS
```

### Add Custom Agents

Create `.claude/agents/<name>.md`:

```yaml
---
name: my-agent
description: When to use this agent
model: sonnet
tools:
  - Read
  - Write
  - Bash
---
Your agent instructions here.
```

### Modify Hooks

Edit scripts in `.claude/hooks/`. Remember to keep them executable:

```bash
chmod +x .claude/hooks/*.sh
```

## Troubleshooting

### Hooks not running

```bash
chmod +x .claude/hooks/*.sh
```

### Codex delegator not working

```bash
pnpm add -g @openai/codex
codex login
```

### Permission denied errors

Check `.claude/settings.json` and add the required command to
`permissions.allow`.

## Resources

- [Claude Code Documentation](https://docs.anthropic.com/en/docs/claude-code)
- [MCP Protocol](https://modelcontextprotocol.io/)
