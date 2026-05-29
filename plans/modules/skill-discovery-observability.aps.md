<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Skill Discovery & Observability

| ID | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| SKOBS | —     | Medium   | Draft  |

## Purpose

Provide visibility into all AI tool skills (slash commands, agents, hooks, MCP
servers) active in a developer's environment across machine, user, and project
scopes. Today there is no inventory, no version tracking, no change detection,
and no way to see what a repo contributor pushed into `.claude/` that now
affects every developer on the project.

This is the observability foundation that governance (AGOV) requires — you
cannot govern what you cannot see.

**Motivation:** The ClawdHub supply chain attack (Jan 2026) demonstrated that
a backdoored skill reached #1 on a registry and compromised 16 developers in
8 hours. The core asymmetry: "Claude reads all files, users read none." Skills
arrive from multiple scopes, may be stale or duplicated, and are not surfaced
through standard interfaces. Skill observability closes this gap.

## In Scope

- Scanning all Claude Code skill locations across three scopes (machine, user,
  project) for commands, agents, hooks, and MCP servers
- Structured manifest schema for skill inventory snapshots (name, scope, type,
  path, content hash, source, version, capabilities, suspicious-pattern flags)
- User-invocable `/skill-inventory` command showing grouped, annotated results
- Change detection between sessions (new, removed, modified skills)
- Suspicious pattern flagging (external network calls, shell execution, file
  access outside project, obfuscation patterns)
- Skill allowlist/blocklist as advisory policy (warnings, not enforcement)
- Snapshot history for audit trail

## Out of Scope

- Runtime skill sandboxing or execution blocking (AGOV territory)
- Skill registry or marketplace features
- MCP server runtime governance (future horizon work)
- Capability enforcement at execution time (AGOV-007 covers this)
- Cross-machine skill synchronisation

## Interfaces

**Depends on:**

- AGOV — Capability declaration model (AGOV-007); skill manifest schema aligns
  with capability-manifest schema. Trust scoring (AGOV-001) may consume skill
  inventory data as a signal.

**Exposes:**

- Scripts: `scripts/skill-scanner.sh` (multi-scope inventory scanner)
- Commands: `.claude/commands/skill-inventory.md`,
  `.claude/commands/skill-diff.md`
- Hooks: `.claude/hooks/skill-watch.sh` (SessionStart change detection)
- Schema: `plans/specs/skill-manifest-schema.md`
- Config: `.claude/skill-policy.json` (allowlist/blocklist)

## Acceptance Criteria

- [ ] A scanner script produces a JSON inventory of all commands, agents, hooks,
      and MCP servers across machine, user, and project scopes
- [ ] Each inventory entry includes name, scope, type, path, SHA-256 content
      hash, source (local/symlink/copied), and last-modified timestamp
- [ ] `/skill-inventory` presents results grouped by scope with annotations for
      shadowing, suspicious patterns, and symlink status
- [ ] A SessionStart hook detects and reports skill changes since last session
      (additions, removals, modifications)
- [ ] Suspicious patterns (curl, wget, bash -c, external URLs, base64 decode,
      eval) are flagged with severity levels
- [ ] A skill-policy.json config supports allowlist/blocklist with advisory
      warnings on policy violations

---

## Work Items

### Phase A: Discovery & Inventory

#### SKOBS-001: Skill scanner script

- **Intent:** Build the core scanner that walks all Claude Code skill locations
  and produces a structured JSON inventory.
- **Expected Outcome:** A shell script that discovers commands, agents, hooks,
  and MCP servers across three scopes (machine: `~/.claude/`, user:
  `$XDG_CONFIG_HOME/claude/`, project: `$PROJECT_DIR/.claude/`). Outputs a JSON
  array of skill entries with name, scope, type, path, SHA-256 content hash,
  last-modified timestamp, and source type (local/symlink with target).
- **Scope:** `scripts/skill-scanner.sh`
- **Non-scope:** Suspicious pattern analysis (SKOBS-007); governance policy
  checks (SKOBS-008)
- **Files:**
  - `scripts/skill-scanner.sh`
- **Dependencies:** —
- **Validation:** `bash scripts/skill-scanner.sh | jq .` produces valid JSON
  listing all skills in the current project
- **Confidence:** high

#### SKOBS-002: Skill manifest schema

- **Intent:** Define the canonical JSON schema for skill inventory snapshots so
  that all consumers (scanner, hooks, commands, AGOV) share a single contract.
- **Expected Outcome:** A specification document defining the SkillInventory
  and SkillEntry schemas with all required and optional fields, aligned with
  AGOV-007's capability-manifest schema where applicable.
- **Scope:** `plans/specs/skill-manifest-schema.md`
- **Non-scope:** JSON Schema file generation; TypeScript/Rust type generation
- **Files:**
  - `plans/specs/skill-manifest-schema.md`
- **Dependencies:** AGOV-007 (schema alignment)
- **Validation:** Schema document reviewed and accepted
- **Confidence:** high

#### SKOBS-003: `/skill-inventory` slash command

- **Intent:** Give developers a single command to see every skill active in
  their environment, where it came from, and whether anything looks wrong.
- **Expected Outcome:** A user-invocable Claude Code command that runs the
  scanner and presents results grouped by scope (machine/user/project) with
  annotations for: skills present in multiple scopes (shadowing), suspicious
  content flags, symlinked vs copied skills, and skills without version
  frontmatter.
- **Scope:** `.claude/commands/skill-inventory.md`
- **Non-scope:** Interactive skill management; skill installation/removal
- **Files:**
  - `.claude/commands/skill-inventory.md`
- **Dependencies:** SKOBS-001 (scanner), SKOBS-002 (schema)
- **Validation:** Running `/skill-inventory` in this repo lists all commands,
  agents, and hooks with scope annotations
- **Confidence:** high

### Phase B: Change Detection & Audit

#### SKOBS-004: Skill change detection hook

- **Intent:** Alert developers at session start when their skill environment
  has changed since the last session.
- **Expected Outcome:** A SessionStart hook that snapshots the current skill
  inventory, compares against the last-known snapshot (stored at
  `.claude/.skill-snapshot.json`), and reports additions, removals, and
  modifications (content hash changes). New snapshots are written on each run.
- **Scope:** `.claude/hooks/skill-watch.sh`
- **Non-scope:** Blocking session start; governance enforcement
- **Files:**
  - `.claude/hooks/skill-watch.sh`
- **Dependencies:** SKOBS-001 (scanner)
- **Validation:** Modify a skill file between sessions; hook reports the change
  at next session start
- **Confidence:** high

#### SKOBS-005: Skill diff command

- **Intent:** Let developers compare their skill environment across snapshots
  or scopes on demand.
- **Expected Outcome:** A user-invocable command that answers "what changed
  since last session?" (snapshot diff) and "what does the project add beyond
  my user config?" (scope diff). Output shows added, removed, and modified
  skills with content hash deltas.
- **Scope:** `.claude/commands/skill-diff.md`
- **Non-scope:** Three-way merge; automatic conflict resolution
- **Files:**
  - `.claude/commands/skill-diff.md`
- **Dependencies:** SKOBS-001 (scanner), SKOBS-004 (snapshot storage)
- **Validation:** Running `/skill-diff` after a skill change shows the delta
- **Confidence:** medium

#### SKOBS-006: Snapshot history convention

- **Intent:** Define a convention for retaining skill inventory snapshots over
  time for audit purposes.
- **Expected Outcome:** Snapshots stored in `.claude/.skill-snapshots/` with
  ISO-8601 timestamps as filenames. Directory is git-ignored by default (local
  observability). Scanner writes a new snapshot on each invocation. Convention
  documented in the manifest schema spec.
- **Scope:** Convention (documented in SKOBS-002 spec); `.gitignore` update
- **Non-scope:** Snapshot pruning policy; remote snapshot storage
- **Files:**
  - Update to `plans/specs/skill-manifest-schema.md` (convention section)
  - `.gitignore` entry for `.claude/.skill-snapshots/`
- **Dependencies:** SKOBS-002 (schema)
- **Validation:** Multiple scanner runs produce timestamped snapshot files
- **Confidence:** high

### Phase C: Governance Integration

#### SKOBS-007: Suspicious pattern scanner

- **Intent:** Flag risky patterns in skill content that may indicate malicious
  or unintended behaviour.
- **Expected Outcome:** Extension to the scanner that analyses skill file
  content for: external network calls (`curl`, `wget`, `fetch`, URL patterns),
  shell execution (`bash -c`, backticks, `exec`), file access outside project
  (`/etc/`, `~/.ssh/`, `.env`), and obfuscation (`base64 decode`, `eval`).
  Each flag has a severity level (info/warning/critical) and is included in the
  manifest entry's `flags` array.
- **Scope:** `scripts/skill-scanner.sh` (extend)
- **Non-scope:** Static analysis of skill logic; AST parsing
- **Files:**
  - `scripts/skill-scanner.sh` (extend with pattern matching)
- **Dependencies:** SKOBS-001 (scanner base)
- **Validation:** A skill containing `curl` is flagged as warning; a skill
  containing `eval` and `base64` is flagged as critical
- **Confidence:** high
- **Coordinates with:** agent-security-package brainstorm items 1
  (Skill Transparency Scanner) and 13 (Full Transparency Scanner)

#### SKOBS-008: Skill allowlist/blocklist

- **Intent:** Let teams define which skills are expected in their project and
  warn when unexpected skills appear.
- **Expected Outcome:** A `.claude/skill-policy.json` config supporting
  `allowlist` (permitted skill names), `blocklist` (blocked skill names),
  `requireDeclaredCapabilities` (boolean), and `alertOnNewSkills` (boolean).
  The SessionStart hook checks the inventory against policy and emits warnings
  for violations. Warnings only — enforcement is AGOV territory.
- **Scope:** `.claude/skill-policy.json`, `.claude/hooks/skill-watch.sh`
  (extend)
- **Non-scope:** Blocking skill execution; capability enforcement
- **Files:**
  - `.claude/skill-policy.json`
  - `.claude/hooks/skill-watch.sh` (extend with policy check)
- **Dependencies:** SKOBS-004 (hook base), AGOV-007 (capability model alignment)
- **Validation:** Configure a blocklist; start session with a blocked skill
  present; warning is emitted
- **Confidence:** medium

---

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Scanner misses skill locations as Claude Code evolves its config paths | Document known paths in spec; make path list configurable |
| Suspicious pattern regex produces false positives on legitimate skills | Flag severity is advisory (info/warning/critical), not blocking; users can inspect and dismiss |
| Snapshot storage grows unbounded | Convention recommends periodic pruning; snapshots are small JSON files |
| Allowlist/blocklist creates friction for teams onboarding new skills | Default policy is permissive (no blocklist); alertOnNewSkills defaults to true |
| Scope confusion between SKOBS (observability) and AGOV (governance) | SKOBS warns; AGOV enforces. Boundary is documented in both modules |

## Decisions

D-SKOBS-001: Observability before governance

- **Rationale:** You cannot govern what you cannot see. SKOBS provides the
  inventory and change-detection foundation that AGOV-007 (capability
  enforcement) and the agent-security-package (skill transparency scanner)
  require.
- **Alternatives:** Build governance first, add observability as needed.
- **Trade-offs:** Observability-first means early value (developers can see
  their environment) but delayed enforcement. Acceptable because AGOV is
  Draft and not yet implementing enforcement.

D-SKOBS-002: Shell scripts, not Rust/TypeScript

- **Rationale:** The Anvil config layer (`code-env/.claude/`) distributes via
  `setup-claude-config.sh` and operates in shell. Skills, hooks, and commands
  are markdown and shell. Implementation in the same medium keeps dependencies
  zero and distribution simple.
- **Alternatives:** Rust crate in `crates/`; TypeScript in `packages/`.
- **Trade-offs:** Shell is less testable and less type-safe, but matches the
  existing infrastructure and avoids build-system dependencies.

## Notes

### Skill Scopes in Claude Code

```text
Machine scope:  ~/.claude/commands/        ~/.claude/agents/
User scope:     $XDG_CONFIG_HOME/claude/   (or ~/.config/claude/)
Project scope:  $PROJECT_DIR/.claude/commands/  $PROJECT_DIR/.claude/agents/
```

Hooks are defined in `settings.json` / `settings.local.json`. MCP servers in
`mcp.json` or `settings.json` under `mcpServers`.

### Provenance

This module draws from:

- **AGOV-007** (capability declaration model) — schema alignment for skill
  manifests
- **agent-security-package brainstorm** (`plans/brainstorms/agent-security-package.md`)
  — items 1 (Skill Transparency Scanner), 13 (Full Transparency Scanner),
  16 (Capability Declaration & Enforcement)
- **ClawdHub supply chain attack case study** — the "Claude reads all, users
  read none" asymmetry that motivates full-file visibility

### Integration with AGOV

SKOBS produces the skill inventory; AGOV consumes it:

| SKOBS produces | AGOV consumes |
|----------------|---------------|
| Skill inventory with content hashes | AGOV-007 validates capabilities against declared manifest |
| Suspicious pattern flags | AGOV-001 trust scoring incorporates flag severity as signal |
| Change detection events | AGOV-006 hash-chained audit trail records skill changes |
| Allowlist/blocklist warnings | Future AGOV enforcement gates can promote warnings to blocks |
