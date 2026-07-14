# Skill Manifest Schema

**Module:** SKOBS (Skill Discovery & Observability)
**Status:** Draft
**Date:** 2026-05-01

## Overview

This document defines the JSON schema for skill inventory snapshots produced
by the SKOBS scanner. All SKOBS consumers (scanner, hooks, commands, and
downstream AGOV integration) use this schema as the single contract for skill
inventory data.

## SkillInventory (root)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `schemaVersion` | `string` | yes | Schema version, semver (e.g. `"1.0.0"`) |
| `timestamp` | `string` | yes | ISO-8601 timestamp of scan |
| `machine` | `string` | yes | Machine hostname |
| `projectDir` | `string` | yes | Absolute path to project root |
| `entries` | `SkillEntry[]` | yes | All discovered skills |
| `summary` | `InventorySummary` | yes | Aggregate counts |

## SkillEntry

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | yes | Skill name (filename without extension) |
| `scope` | `"machine" \| "user" \| "project"` | yes | Which scope the skill was found in |
| `type` | `"command" \| "agent" \| "hook" \| "mcp"` | yes | Skill category |
| `path` | `string` | yes | Absolute path to the skill file |
| `contentHash` | `string` | yes | SHA-256 hex digest of file contents |
| `lastModified` | `string` | yes | ISO-8601 timestamp from filesystem |
| `sizeBytes` | `number` | yes | File size in bytes |
| `source` | `SourceInfo` | yes | How the file arrived at this location |
| `version` | `string \| null` | no | Version from frontmatter (if present) |
| `capabilities` | `string[] \| null` | no | Declared capabilities from frontmatter |
| `flags` | `Flag[]` | no | Suspicious patterns detected in content |
| `shadowedBy` | `string[] \| null` | no | Paths of skills in higher-precedence scopes that shadow this one |

## SourceInfo

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | `"local" \| "symlink" \| "copied" \| "anvil-bundled"` | yes | How the file exists on disk |
| `symlinkTarget` | `string \| null` | no | Resolved symlink target (if type is `"symlink"`) |
| `sourceCommit` | `string \| null` | no | Catalogue commit for an Anvil-bundled skill |
| `anvilVersion` | `string \| null` | no | Anvil version that installed a bundled skill |

For symlinks, `symlinkTarget` is the fully resolved absolute path. For copied
files, the scanner cannot determine the original source — `type` is `"local"`
unless the file is a symlink.

An `anvil-bundled` source is installed from an asset embedded in the Anvil
binary. Its adjacent `.anvil-managed.json` manifest is authoritative for
`sourceCommit`, `anvilVersion`, `bundleDigest`, and the managed file hashes. A scanner must
verify those hashes before reporting the source as managed; a missing or
mismatched manifest is local or modified content, not `anvil-bundled`.

## Flag

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `pattern` | `string` | yes | Pattern that triggered the flag (e.g. `"curl"`, `"eval"`) |
| `severity` | `"info" \| "warning" \| "critical"` | yes | Risk level |
| `line` | `number \| null` | no | Line number where pattern was found |
| `context` | `string` | yes | Human-readable explanation (e.g. `"External HTTP request"`) |
| `category` | `string` | yes | Pattern category: `"network"`, `"shell"`, `"filesystem"`, `"obfuscation"` |

### Suspicious Pattern Catalogue

| Pattern | Category | Severity | Context |
|---------|----------|----------|---------|
| `curl`, `wget`, `fetch` | network | warning | External HTTP request |
| `http://`, `https://` (non-doc URLs) | network | info | URL reference — may be documentation or may be exfiltration |
| `bash -c`, `` ` `` (backticks), `exec` | shell | warning | Shell command execution |
| `/etc/`, `~/.ssh/`, `.env` | filesystem | critical | Access to sensitive system/credential paths |
| `base64`, `eval` | obfuscation | critical | Possible code obfuscation |
| `chmod`, `chown` | filesystem | warning | Permission modification |

## InventorySummary

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `totalEntries` | `number` | yes | Total skill count |
| `byScope` | `{ machine: number, user: number, project: number }` | yes | Count per scope |
| `byType` | `{ command: number, agent: number, hook: number, mcp: number }` | yes | Count per type |
| `flaggedCount` | `number` | yes | Entries with at least one flag |
| `shadowedCount` | `number` | yes | Entries shadowed by a higher-precedence scope |

## Example

```json
{
  "schemaVersion": "1.0.0",
  "timestamp": "2026-05-01T14:30:00Z",
  "machine": "dev-laptop",
  "projectDir": "/home/user/anvil-001",
  "entries": [
    {
      "name": "commit",
      "scope": "project",
      "type": "command",
      "path": "/home/user/anvil-001/.claude/commands/commit.md",
      "contentHash": "a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456",
      "lastModified": "2026-04-28T10:15:00Z",
      "sizeBytes": 1234,
      "source": {
        "type": "local"
      },
      "version": null,
      "capabilities": null,
      "flags": [],
      "shadowedBy": null
    },
    {
      "name": "council-reviewer",
      "scope": "project",
      "type": "agent",
      "path": "/home/user/anvil-001/.claude/agents/council-reviewer.md",
      "contentHash": "f6e5d4c3b2a1098765432109876543210fedcba9876543210fedcba987654321",
      "lastModified": "2026-04-25T08:00:00Z",
      "sizeBytes": 2456,
      "source": {
        "type": "symlink",
        "symlinkTarget": "/home/user/code-env/.claude/agents/council-reviewer.md"
      },
      "version": null,
      "capabilities": null,
      "flags": [],
      "shadowedBy": null
    },
    {
      "name": "git-safety",
      "scope": "project",
      "type": "hook",
      "path": "/home/user/anvil-001/.claude/hooks/git-safety.sh",
      "contentHash": "1234abcd5678901234567890123456789012345678901234567890123456abcd",
      "lastModified": "2026-04-20T12:00:00Z",
      "sizeBytes": 890,
      "source": {
        "type": "symlink",
        "symlinkTarget": "/home/user/code-env/.claude/hooks/git-safety.sh"
      },
      "version": null,
      "capabilities": null,
      "flags": [
        {
          "pattern": "bash",
          "severity": "info",
          "line": null,
          "context": "Hook is a shell script — expected for hooks",
          "category": "shell"
        }
      ],
      "shadowedBy": null
    }
  ],
  "summary": {
    "totalEntries": 34,
    "byScope": { "machine": 0, "user": 0, "project": 34 },
    "byType": { "command": 9, "agent": 21, "hook": 4, "mcp": 0 },
    "flaggedCount": 1,
    "shadowedCount": 0
  }
}
```

## Snapshot History Convention

Snapshots are stored in `.claude/.skill-snapshots/` with ISO-8601 timestamp
filenames:

```
.claude/.skill-snapshots/
  2026-05-01T14-30-00Z.json
  2026-05-01T09-15-00Z.json
  2026-04-30T16-45-00Z.json
```

This directory is git-ignored by default (local observability data, not
committed to the repo). The scanner writes a new snapshot on each invocation.
The most recent snapshot is also written to `.claude/.skill-snapshot.json`
for fast access by the change detection hook.

## Schema Alignment with AGOV-007

The `capabilities` field in `SkillEntry` is intentionally aligned with
AGOV-007's capability-manifest schema. When AGOV-007 ships, skill entries
that include capability declarations can be validated against the
capability-manifest contract:

| SkillEntry field | AGOV-007 manifest field | Relationship |
|------------------|------------------------|-------------|
| `capabilities` | `capabilities.operations` | Subset — skill declares what it does |
| `flags[].category` | — | SKOBS-specific; feeds trust scoring |
| `contentHash` | — | Integrity verification for both schemas |

## Skill Policy Schema

The optional `.claude/skill-policy.json` config:

```json
{
  "allowlist": ["commit", "test", "review", "plan", "council"],
  "blocklist": [],
  "requireDeclaredCapabilities": false,
  "alertOnNewSkills": true,
  "alertOnFlaggedSkills": true,
  "minimumFlagSeverityToAlert": "warning"
}
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `allowlist` | `string[]` | `[]` (all allowed) | Skill names explicitly permitted |
| `blocklist` | `string[]` | `[]` (none blocked) | Skill names to warn about |
| `requireDeclaredCapabilities` | `boolean` | `false` | Warn if skills lack capability declarations |
| `alertOnNewSkills` | `boolean` | `true` | Warn when skills appear that weren't in last snapshot |
| `alertOnFlaggedSkills` | `boolean` | `true` | Warn when flagged skills are present |
| `minimumFlagSeverityToAlert` | `"info" \| "warning" \| "critical"` | `"warning"` | Minimum flag severity that triggers an alert. Aligned with `Flag.severity` enum so policy config can be validated against the same set of values. |

When both `allowlist` and `blocklist` are non-empty, `blocklist` takes
precedence (a skill on both lists is blocked).
