# Claude Code Enhanced Configuration

This repository contains an enhanced Claude Code setup with delegator
integration, custom agents, hooks, and skills.

## Quick Start

```bash
# Install Codex CLI for delegator functionality
pnpm add -g @openai/codex
codex login

# Make hook scripts executable
chmod +x .claude/hooks/*.sh

# Verify setup
claude --version
```

## Use in Another Project

Copy this configuration to any project using the setup script:

```bash
# Copy mode (independent config per project)
./setup-claude-config.sh /path/to/your/project

# Symlink mode (shared config, changes affect all projects)
./setup-claude-config.sh /path/to/your/project --symlink
```

The script will:

- Copy (or symlink) the `.claude/` directory and `CLAUDE.md`
- Add `CLAUDE-README.md` with usage documentation
- Make hook scripts executable
- Create the logs directory

After setup, edit `CLAUDE.md` in your project to customize instructions for that
specific codebase.

## Features

### MCP Integrations

- **Codex Delegator**: GPT specialist delegation for architecture, security, and
  code review
- **Memory Server**: Persistent context across sessions
- **Filesystem Server**: Enhanced file operations

### Custom Agents

| Agent              | Purpose                          | Model  |
| ------------------ | -------------------------------- | ------ |
| `architect`        | System design, complex debugging | opus   |
| `code-reviewer`    | Quality analysis, PR review      | sonnet |
| `security-analyst` | Vulnerability assessment         | opus   |
| `tdd-coach`        | Test-driven development guidance | sonnet |
| `debugger`         | Systematic root cause analysis   | sonnet |
| `autonomous`       | Long-running multi-step tasks    | opus   |
| `planner`          | Implementation planning          | sonnet |

### Slash Commands

| Command         | Description                         |
| --------------- | ----------------------------------- |
| `/think-harder` | Deep analytical thinking            |
| `/plan`         | Create implementation plan          |
| `/review`       | Comprehensive code review           |
| `/commit`       | Git commit with conventional format |
| `/test`         | Run tests and fix failures          |
| `/debug`        | Systematic debugging                |
| `/delegate`     | Delegate to expert via Codex        |
| `/autonomous`   | Execute long-running task           |

### Hooks

| Event        | Hook              | Purpose                         |
| ------------ | ----------------- | ------------------------------- |
| PreToolUse   | security-guard.sh | Block dangerous commands        |
| PreToolUse   | tdd-guard.sh      | Enforce test-driven development |
| PostToolUse  | post-edit.sh      | Auto-format and lint            |
| Stop         | on-stop.sh        | Notifications and logging       |
| SessionStart | session-start.sh  | Environment check               |

#### TDD Guard Configuration

The `tdd-guard.sh` hook enforces test-driven development practices. Configure
via environment variables:

| Variable               | Default | Description                                                      |
| ---------------------- | ------- | ---------------------------------------------------------------- |
| `CLAUDE_TDD_STRICT`    | `false` | Block edits to source files if no corresponding test file exists |
| `CLAUDE_TDD_RUN_TESTS` | `false` | Run related tests before allowing edits (blocks if tests fail)   |

### Skills

- `test-driven-development` - TDD workflow and patterns
- `systematic-debugging` - Scientific debugging methodology
- `parallel-agents` - Multi-agent coordination
- `autonomous-execution` - Long-running task management
- `code-review` - Review methodology
- `security-analysis` - Vulnerability assessment

## Configuration

### Extended Tool Limits

```json
{
  "CLAUDE_MAX_TOOL_USES": "500",
  "CLAUDE_MAX_TOKENS": "200000",
  "CLAUDE_TIMEOUT_MS": "600000",
  "CLAUDE_CODE_MAX_SUBAGENTS": "10"
}
```

### Permissions

Pre-configured permissions for:

- Build tools (npm, yarn, cargo, go, etc.)
- Version control (git, gh)
- Quality tools (eslint, prettier, ruff)
- MCP servers (codex, memory, filesystem)

Blocked patterns:

- Destructive operations (rm -rf /, etc.)
- System modification commands
- Fork bombs and resource exhaustion

## Directory Structure

```
.claude/
├── settings.json          # Main configuration
├── agents/                # Custom AI assistants
│   ├── architect.md
│   ├── code-reviewer.md
│   ├── security-analyst.md
│   ├── tdd-coach.md
│   ├── debugger.md
│   ├── autonomous.md
│   └── planner.md
├── commands/              # Slash commands
│   ├── think-harder.md
│   ├── plan.md
│   ├── review.md
│   ├── commit.md
│   ├── test.md
│   ├── debug.md
│   ├── delegate.md
│   └── autonomous.md
├── hooks/                 # Lifecycle hooks
│   ├── security-guard.sh
│   ├── post-edit.sh
│   ├── on-stop.sh
│   ├── session-start.sh
│   └── tdd-guard.sh
├── skills/                # Domain knowledge
│   ├── test-driven-development/
│   ├── systematic-debugging/
│   ├── parallel-agents/
│   ├── autonomous-execution/
│   ├── code-review/
│   └── security-analysis/
├── prompts/               # Expert system prompts
│   ├── architect.md
│   ├── code-reviewer.md
│   ├── security-analyst.md
│   ├── plan-reviewer.md
│   └── scope-analyst.md
└── logs/                  # Session logs
```

## Usage Examples

### Delegate to Expert

```
/delegate Analyze the authentication system for security vulnerabilities
```

### Autonomous Execution

```
/autonomous Refactor all API endpoints to use new validation middleware
```

### Deep Analysis

```
/think-harder Why are users experiencing intermittent timeouts?
```

### Plan Feature

```
/plan Add real-time notifications with WebSocket support
```

## Development Workflow

1. **Start Session**: Environment auto-checked via session-start hook
2. **Plan Work**: Use `/plan` to break down tasks
3. **TDD Approach**: Use TDD coach agent for test-first development
4. **Review Code**: Use `/review` before commits
5. **Commit Changes**: Use `/commit` for conventional commits
6. **Debug Issues**: Use `/debug` for systematic root cause analysis

## Customization

### Adding New Agents

Create `.claude/agents/<name>.md`:

```yaml
---
name: agent-name
description: When to use this agent
model: sonnet|opus|haiku
tools:
  - Tool1
  - Tool2
---
# Agent instructions...
```

### Adding New Commands

Create `.claude/commands/<name>.md`:

```yaml
---
name: command-name
description: What it does
---
# Command instructions...

$ARGUMENTS
```

### Adding New Skills

Create `.claude/skills/<name>/SKILL.md`:

```yaml
---
name: skill-name
description: Keywords that trigger this skill
---
# Skill documentation...
```

## Resources

Based on patterns from:

- [claude-delegator](https://github.com/jarrodwatts/claude-delegator)
- [superpowers](https://github.com/obra/superpowers)
- [claude-code-showcase](https://github.com/ChrisWiles/claude-code-showcase)
- [claude-code-settings](https://github.com/feiskyer/claude-code-settings)
- [awesome-claude-code](https://github.com/hesreallyhim/awesome-claude-code)
