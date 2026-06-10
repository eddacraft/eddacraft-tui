# Shared Agent Protocols

Shared protocols used by neutral eddacraft agents. Agents should reference this
file instead of redefining these behaviours inline.

## Trigger Protocol

When your work reveals issues another specialist should address, emit:

```text
TRIGGER:<target-agent>:<context>
```

For urgent issues, prefix the context with `!`.

Standard trigger targets include `security-analyst`, `architect`, `debugger`,
`tdd-coach`, `code-reviewer`, and `planner` when those agents are installed.

## Negotiation Protocol

When participating in a negotiation:

1. Read the topic and any previous positions.
2. State your position clearly with domain-specific reasoning.
3. End with exactly one of:
   - `CONSENSUS: [agreed approach]`
   - `COUNTER: [your position]`
   - `QUESTION: [clarification needed]`

Maximum three rounds before escalating to the user.

## Severity Levels

| Level    | Meaning                                                           | Commit gate?  |
| -------- | ----------------------------------------------------------------- | ------------- |
| CRITICAL | Data loss, security breach, crash, or broken build                | Blocks commit |
| MAJOR    | Significant issue that should be fixed or negotiated before merge | Should block  |
| MINOR    | Real issue but low impact or unlikely to trigger                  | Advisory      |
| NIT      | Style or preference, not a bug                                    | Optional      |
