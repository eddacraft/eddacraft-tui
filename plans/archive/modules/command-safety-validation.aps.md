<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Command Safety Validation

| Scope  | Owner | Priority | Status |
| ------ | ----- | -------- | ------ |
| CMDSAF | —     | high     | Ready  |

## Purpose

Prevent data loss from destructive shell commands in Anvil plans by validating
git and filesystem operations before execution. This provides runtime command
safety that complements Anvil's plan-level gate validation, inspired by patterns
from claude-code-safety-net.

**Problem:** AI-generated plans may include destructive operations like
`git reset --hard`, `git push --force`, or `rm -rf ~/` that cause irreversible
data loss. Current gate checks validate tests, coverage, and secrets, but don't
semantically analyse shell commands for dangerous patterns.

**Solution:** Implement a command safety gate check with:

- Semantic command analysis (distinguishes `git checkout -b` from
  `git checkout --`)
- Default block/allow lists for git and filesystem operations
- User-configurable rules via `.anvilrc`
- Shell wrapper unwrapping (detects `bash -c`, `python -c` obfuscation)
- Clear blocking messages with safe alternative suggestions

## In Scope

- Rule-based command validation engine with specificity sorting
- Default git operation rules (reset --hard, push --force, checkout --, etc.)
- Default filesystem rules (rm -rf on dangerous paths)
- Shell wrapper detection and unwrapping (bash -c, sudo, env, python -c)
- Configuration system for overrides and custom rules
- Integration with gate runner as `CommandSafetyCheck`
- Message formatting with reasons and safe alternative suggestions

## Out of Scope

- Path resolution (checking if `/tmp/../home/user` actually exists)
- Git repository state detection (in rebase, merge, etc.)
- Interactive confirmation prompts (gate checks are non-interactive)
- Sandboxing or containerisation (separate concern)
- Auto-fixing dangerous commands (user must modify plan)

## Interfaces

**Depends on:**

- `core/src/gate/` — Gate check interface and runner
- `core/src/types/gate.types.ts` — GateCheck interface
- `shell-quote` — Shell command parsing (new dependency)
- `minimist` — Flag parsing (new dependency)

**Exposes:**

- `CommandSafetyCheck` — Gate check implementation
- `CommandRule` — Rule definition interface
- `RuleMatcher` — Rule matching engine with specificity
- `CommandParser` — Command parsing and wrapper unwrapping
- Default rulesets: git, filesystem, shell

**Configuration:**

```typescript
// .anvilrc
{
  "commandSafety": {
    "enabled": true,
    "strict": false,
    "rules": {
      "overrides": [{"id": "git-push-force", "action": "warn"}],
      "disabled": ["git-clean-force"],
      "custom": [/* user rules */]
    }
  }
}
```

## Boundary Rules

- CMDSAF must not execute commands, only analyse them
- CMDSAF must not modify plan files
- CMDSAF must be configurable (not hard-coded blocks)
- CMDSAF must provide clear explanations for blocks/warnings
- CMDSAF must allow safe variants of commonly-blocked commands

## Acceptance Criteria

- [ ] `anvil gate plan.md` includes command safety check by default
- [ ] Blocks `git reset --hard` with explanation and suggestion
- [ ] Blocks `git push --force` but allows `git push --force-with-lease`
- [ ] Blocks `rm -rf /` and `rm -rf ~/` but allows `rm -rf /tmp/*`
- [ ] Allows `rm -rf node_modules`, `dist`, `build` (reproducible artifacts)
- [ ] Detects dangerous commands wrapped in `bash -c "..."`
- [ ] Detects nested wrappers: `sudo env VAR=1 bash -c "git reset --hard"`
- [ ] Users can override rules via `.anvilrc` configuration
- [ ] Users can add custom rules for project-specific commands
- [ ] Evidence bundle includes blocked commands with reasons
- [ ] < 50ms overhead per command analysed
- [ ] 100+ test cases covering all default rules and edge cases
- [ ] Zero false negatives on known dangerous patterns
- [ ] < 5% false positive rate on real-world plans

## Risks & Mitigations

| Risk                                  | Impact | Likelihood | Mitigation                                                           |
| ------------------------------------- | ------ | ---------- | -------------------------------------------------------------------- |
| False positives annoy developers      | high   | medium     | Make rules configurable; provide clear explanations; default to warn |
| Performance overhead slows gate       | medium | low        | Cache parsing; lightweight libraries; parallel check execution       |
| Maintenance burden for new patterns   | medium | medium     | Community contributions; focus on high-value stable patterns         |
| Users bypass with `--skip-checks`     | medium | medium     | Track bypass usage in evidence; make suggestions actionable          |
| Complex wrapper obfuscation undetecte | low    | low        | Recursion depth limit (5); fallback to regex heuristics              |

## Tasks

### CMDSAF-001: Rule system and types

- **Intent:** Define CommandRule interface and rule data structures
- **Expected Outcome:** TypeScript types for rules, matcher, and config
- **Scope:** `core/src/gate/rules/types.ts`
- **Non-scope:** Actual rule implementations
- **Files:**
  - `core/src/gate/rules/types.ts` — Rule interfaces
  - `core/src/gate/rules/types.test.ts` — Type tests
- **Dependencies:** —
- **Validation:** `nx test core --testNamePattern="rule types"`
- **Confidence:** high
- **Risks:** None, pure type definitions

### CMDSAF-002: Command parser with wrapper unwrapping

- **Intent:** Parse shell commands and unwrap wrappers (bash -c, sudo, etc.)
- **Expected Outcome:** Parse command to tokens, detect and strip wrappers
  recursively
- **Scope:** `core/src/gate/parsers/`
- **Non-scope:** Rule matching
- **Files:**
  - `core/src/gate/parsers/command-parser.ts` — Main parser
  - `core/src/gate/parsers/wrapper-unwrapper.ts` — Wrapper detection
  - `core/src/gate/parsers/command-parser.test.ts` — 30+ tests
- **Dependencies:** CMDSAF-001, `shell-quote`, `minimist`
- **Validation:** `nx test core --testNamePattern="CommandParser"`
- **Confidence:** high
- **Risks:** Edge cases in shell quoting, resolved with comprehensive tests

### CMDSAF-003: Rule matcher with specificity

- **Intent:** Match parsed commands against rules, most specific first
- **Expected Outcome:** Find matching rule for a command based on specificity
  score
- **Scope:** `core/src/gate/rules/`
- **Non-scope:** Rule definitions themselves
- **Files:**
  - `core/src/gate/rules/rule-matcher.ts` — Matcher implementation
  - `core/src/gate/rules/rule-matcher.test.ts` — 20+ tests
- **Dependencies:** CMDSAF-001, CMDSAF-002
- **Validation:** `nx test core --testNamePattern="RuleMatcher"`
- **Confidence:** high
- **Risks:** Specificity scoring complexity, mitigated with clear algorithm

### CMDSAF-004: Default git operation rules

- **Intent:** Define default block/allow rules for git operations
- **Expected Outcome:** ~15 git rules covering destructive operations
- **Scope:** `core/src/gate/rules/`
- **Non-scope:** Filesystem or custom rules
- **Files:**
  - `core/src/gate/rules/default-git-rules.ts` — Git rules
  - `core/src/gate/rules/default-git-rules.test.ts` — 40+ tests
- **Dependencies:** CMDSAF-001
- **Validation:** `nx test core --testNamePattern="git rules"`
- **Confidence:** high
- **Risks:** Missing edge cases, mitigated by porting claude-code-safety-net
  tests

### CMDSAF-005: Default filesystem rules

- **Intent:** Define default block/allow rules for filesystem operations
- **Expected Outcome:** ~10 rules for rm -rf and dangerous paths
- **Scope:** `core/src/gate/rules/`
- **Non-scope:** Git or custom rules
- **Files:**
  - `core/src/gate/rules/default-filesystem-rules.ts` — FS rules
  - `core/src/gate/rules/default-filesystem-rules.test.ts` — 30+ tests
- **Dependencies:** CMDSAF-001
- **Validation:** `nx test core --testNamePattern="filesystem rules"`
- **Confidence:** high
- **Risks:** Path edge cases (symlinks, etc.), scoped to string analysis only

### CMDSAF-006: CommandSafetyCheck gate implementation

- **Intent:** Implement GateCheck interface for command safety
- **Expected Outcome:** Extract commands from plan, analyse, return results
- **Scope:** `core/src/gate/checks/`
- **Non-scope:** Configuration loading
- **Files:**
  - `core/src/gate/checks/command-safety.check.ts` — Main check
  - `core/src/gate/checks/command-safety.check.test.ts` — 20+ tests
- **Dependencies:** CMDSAF-002, CMDSAF-003, CMDSAF-004, CMDSAF-005
- **Validation:** `nx test core --testNamePattern="CommandSafetyCheck"`
- **Confidence:** high
- **Risks:** Command extraction from various plan formats

### CMDSAF-007: Configuration system

- **Intent:** Load and merge configuration from .anvilrc and environment
- **Expected Outcome:** Config loader with override and custom rule support
- **Scope:** `core/src/gate/config/`
- **Non-scope:** Validation of user-provided rules
- **Files:**
  - `core/src/gate/config/command-safety-config.ts` — Config loader
  - `core/src/gate/config/command-safety-config.test.ts` — 15+ tests
- **Dependencies:** CMDSAF-001
- **Validation:** `nx test core --testNamePattern="command safety config"`
- **Confidence:** high
- **Risks:** Config merge complexity, mitigated with clear priority order

### CMDSAF-008: Message formatting

- **Intent:** Format blocked/warning messages with reasons and suggestions
- **Expected Outcome:** User-friendly error messages with actionable guidance
- **Scope:** `core/src/gate/formatters/`
- **Non-scope:** CLI display logic
- **Files:**
  - `core/src/gate/formatters/message-formatter.ts` — Formatter
  - `core/src/gate/formatters/message-formatter.test.ts` — 10+ tests
- **Dependencies:** CMDSAF-001
- **Validation:** `nx test core --testNamePattern="MessageFormatter"`
- **Confidence:** high
- **Risks:** None, pure formatting

### CMDSAF-009: CLI integration and documentation

- **Intent:** Wire check into CLI, add flags, document usage
- **Expected Outcome:** `anvil gate` includes command safety,
  `--skip-command-safety` flag works
- **Scope:** `cli/src/commands/`
- **Non-scope:** VS Code extension integration
- **Files:**
  - `cli/src/commands/gate.ts` — Add check to runner
  - `docs/guides/command-safety.md` — User guide
  - `docs/guides/command-safety-configuration.md` — Config reference
- **Dependencies:** CMDSAF-006, CMDSAF-007, CMDSAF-008
- **Validation:** `nx test cli --testNamePattern="gate command"` + manual
  testing
- **Confidence:** high
- **Risks:** None, integration task

## Execution

- [CMDSAF-001 to CMDSAF-003](../execution/CMDSAF-001-003.steps.md) — Core system
  (Week 1)
- [CMDSAF-004 to CMDSAF-005](../execution/CMDSAF-004-005.steps.md) — Rule
  definitions (Week 1)
- [CMDSAF-006 to CMDSAF-009](../execution/CMDSAF-006-009.steps.md) — Integration
  (Week 2)

## Decisions

**D-CMDSAF-001:** Use semantic analysis, not regex patterns

- **Rationale:** Regex can't distinguish `git checkout -b` (safe) from
  `git checkout --` (destructive). Semantic parsing understands flag
  combinations and order.
- **Alternatives:** Regex-based blocking (too many false positives)
- **Trade-offs:** More complex implementation, but much better signal quality

**D-CMDSAF-002:** Default to warnings, not errors

- **Rationale:** Align with Anvil's philosophy (inform, don't block). Users can
  configure errors if desired.
- **Alternatives:** Hard blocks (would frustrate users)
- **Trade-offs:** Users might ignore warnings, mitigated by clear messaging

**D-CMDSAF-003:** Port patterns from claude-code-safety-net, not full codebase

- **Rationale:** TypeScript architecture is cleaner than Python integration.
  Port valuable patterns, not code.
- **Alternatives:** Invoke Python as subprocess (tech stack mismatch)
- **Trade-offs:** Re-implementation effort, but better maintainability

**D-CMDSAF-004:** Focus on git and rm, defer docker/npm/etc.

- **Rationale:** Git operations and filesystem commands are highest risk. Other
  commands can be added via custom rules.
- **Alternatives:** Comprehensive command coverage (scope creep)
- **Trade-offs:** Limited initial scope, but extensible via custom rules

## Notes

**Test data sourcing:**

The claude-code-safety-net repository has comprehensive test suites covering
edge cases. We should port test cases from:

- `tests/test_safety_net_git.py` → 40+ git command test cases
- `tests/test_safety_net_rm.py` → 30+ filesystem test cases
- `tests/test_safety_net_edge.py` → Edge case patterns

**Performance considerations:**

Command parsing should be cached by command string hash to avoid re-parsing
identical commands across multiple gate runs.

**Future enhancements (post-MVP):**

- Path resolution (check if target actually exists in /tmp)
- Git state awareness (detect if in rebase/merge)
- Interactive mode (ask user for confirmation)
- Machine learning for pattern detection
- Integration with IDE extensions (inline warnings)

**Success metrics:**

- Adoption: >80% of users keep command safety enabled
- Effectiveness: Zero false negatives on known patterns
- Quality: <5% false positive rate
- Performance: <50ms overhead per command

**Related documentation:**

- [claude-code-safety-net Review](../../docs/analysis/claude-code-safety-net-review.md)
- [Command Safety Specification](../../docs/specifications/command-safety-validation.md)
- [Gate System Architecture](../../docs/ARCHITECTURE.md#gate-layer)
