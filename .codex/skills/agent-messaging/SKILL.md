---
name: agent-messaging
description:
  Structured message passing between agents for findings, questions, and alerts
---

# Agent Messaging Skill

## Overview

Send and receive structured messages between agents during multi-agent
workflows. Prefer direct `send_input` messages. For asynchronous coordination,
optionally persist messages in local JSONL mailbox files.

## When to Apply

- Sharing findings across specialist agents
- Asking questions to other agents for later response
- Sending recommendations based on analysis
- Alerting other agents to critical issues
- Building context for agent negotiations

## Message Types

| Type             | Purpose                                     | Example                                               |
| ---------------- | ------------------------------------------- | ----------------------------------------------------- |
| `finding`        | Share discovered issues or insights         | Security vulnerability, code smell, performance issue |
| `question`       | Request information from another agent      | Architecture decision rationale, testing approach     |
| `recommendation` | Suggest actions based on analysis           | Refactoring strategy, dependency update               |
| `alert`          | Notify of urgent issues requiring attention | Critical vulnerability, breaking change               |

## Priority Levels

| Priority   | When to Use                       |
| ---------- | --------------------------------- |
| `low`      | Informational, no action required |
| `medium`   | Standard importance (default)     |
| `high`     | Should be addressed soon          |
| `critical` | Requires immediate attention      |

## Sending Messages

### Direct Agent-to-Agent (Preferred)

Use `send_input` with a structured JSON payload in the message body.

```json
{
  "from": "code-reviewer",
  "to": "architect",
  "type": "finding",
  "priority": "medium",
  "payload": {
    "file": "src/auth.ts",
    "issue": "Missing rate limiting"
  }
}
```

### Linked to Negotiation

Include `negotiationId` when the message belongs to a negotiation thread.

## Receiving Messages

### Active Agent Threads

- Use `wait` to collect completed agent responses.
- Parse the returned content for message payloads.

### Optional Mailbox Files

If persistence is needed, write/read mailbox files in:

- `.codex/agent-bus/messages/{agent-name}.jsonl`
- `.codex/agent-bus/messages/all-messages.jsonl`

## Message Schema

```json
{
  "id": "msg-1705234567-a1b2c3",
  "from": "code-reviewer",
  "to": "architect",
  "type": "finding",
  "priority": "medium",
  "payload": {
    "file": "src/auth.ts",
    "line": 42,
    "issue": "Password not hashed before storage",
    "severity": "critical"
  },
  "timestamp": "2024-01-14T10:30:00Z",
  "negotiationId": "neg-1234567890"
}
```

## Payload Conventions

### Finding Payload

```json
{
  "file": "path/to/file.ts",
  "line": 42,
  "issue": "Description of the finding",
  "severity": "low|medium|high|critical",
  "suggestion": "How to fix (optional)"
}
```

### Question Payload

```json
{
  "question": "The question to answer",
  "context": "Relevant background (optional)",
  "deadline": "ISO timestamp if time-sensitive (optional)"
}
```

### Recommendation Payload

```json
{
  "action": "What should be done",
  "rationale": "Why this is recommended",
  "effort": "low|medium|high",
  "priority": "Suggested implementation order"
}
```

### Alert Payload

```json
{
  "vulnerability": "Type of issue",
  "severity": "critical|high|medium|low",
  "file": "Affected file (optional)",
  "cve": "CVE ID if applicable (optional)",
  "remediation": "How to fix"
}
```

## Integration with Agents

### Checking for Messages at Start

Before analysis:

1. Check open negotiation/context files.
2. Review queued messages for your role.
3. Prioritise `critical` alerts.

### Sending Findings on Completion

After analysis:

1. Send one structured message per significant finding.
2. Set priority based on severity.
3. Include actionable payload details.

## Configuration

| Variable            | Default | Description            |
| ------------------- | ------- | ---------------------- |
| `CODEX_PROJECT_DIR` | `pwd`   | Project root directory |

## Error Handling

- Invalid JSON payload: reject and request corrected format.
- Invalid message type: reject and list allowed types.
- Missing recipient: fall back to shared mailbox log.
- No messages found: return empty list `[]`.
