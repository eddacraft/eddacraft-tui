# Edda Memory Management

| Type  | Authority     | Owner | Status | Freshness                                                                                                             |
| ----- | ------------- | ----- | ------ | --------------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | RCLI3 | Live   | Last reviewed 2026-05-25 against `crates/anvil-cli/src/commands/edda.rs` and `packages/edda-stack/src/edda/README.md` |

| Upstream                                                                                                                                                            | Downstream                                 |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ |
| `packages/edda-stack/src/contracts/edda-memory.ts`, `packages/edda-stack/src/edda/`, `crates/anvil-cli/src/commands/edda.rs`, `plans/modules/rust-cli-tier3.aps.md` | `anvil edda` CLI workflows, RCLI3 planning |

This guide explains what Edda memories are, how they are created, and how to
manage them using the Anvil CLI.

## What Is Edda?

Edda is the canonical memory layer of the Kindling · Ember · Edda stack. It
stores what you have deliberately chosen to keep: decisions that shaped the
codebase, patterns that recur, constraints that bite, warnings worth heeding,
doctrines that govern how work is done, and lessons learned from failure and
success.

> Edda is not a log. It is not a transcript. It is not a database of everything.
> It is an institutional memory: decisions, lessons, patterns, constraints,
> truths. Edda forgets aggressively by default. That is a feature, not a bug.

Edda is the opposite of a wiki. You do not fill it constantly. You add to it
deliberately, when something is true enough, specific enough, and important
enough to keep.

## How Memories Are Created

Memories enter Edda through one path: human promotion. No tool, agent, or
automated process can write to Edda directly.

The typical flow is:

1. **Kindling** observes what happens during work sessions — tool calls, errors,
   completions, constraint violations — and records them as raw facts.
2. **Ember** analyses those observations, detects patterns, and creates
   candidate proposals when something might be worth remembering.
3. **You** review the Ember queue, decide which proposals are true enough to
   keep, and promote them with a reason and a confidence level.
4. **Edda** stores the memory, links it to the Ember proposal and Kindling
   observations that originated it, and records your name and reason as
   attribution.

Every memory carries full provenance. If you later ask "where did this come
from?", `anvil edda trace` will show you the exact Ember proposal and Kindling
observations that led to it.

## Memory Types

Edda uses six fixed memory types:

| Type         | Use for                                              |
| ------------ | ---------------------------------------------------- |
| `decision`   | Choices made with observable, lasting consequences   |
| `pattern`    | Recurring structures or behaviours worth encoding    |
| `constraint` | Discovered limitations or hard boundaries            |
| `warning`    | Known failure modes or signals of potential problems |
| `doctrine`   | Principles or rules that govern how work is done     |
| `lesson`     | Learnings extracted from failure or success          |

Choose the type that best describes the nature of the memory, not the nature of
the Ember proposal it came from. An Ember `pattern` proposal might become an
Edda `doctrine` if it reflects a principle you want to enforce, not just
observe.

## Confidence Levels

When promoting a memory, you assign a confidence level. This reflects your
judgement, not Ember's score.

| Level    | When to use                                             |
| -------- | ------------------------------------------------------- |
| `low`    | Plausible but unverified; treat as a working hypothesis |
| `medium` | Supported by evidence; likely correct in scope          |
| `high`   | Confirmed by multiple sources or direct human review    |

Ember's numeric confidence score (e.g. `0.82`) informs your decision but does
not determine the Edda confidence level. You choose.

## CLI Usage

### List memories

```sh
anvil edda list
```

Shows all active memories, ordered by creation date (newest first).

```
mem_a1b2c3d4  decision    high    All cross-package imports use published
                                  package names, never relative paths.
              2026-03-01  tags: imports, monorepo, esm

mem_b2c3d4e5  warning     medium  Auth service 503 pattern appears under
                                  high concurrency.
              2026-02-28  tags: auth, performance

mem_c3d4e5f6  lesson      high    Skipping the build step before tests
                                  causes false failures.
              2026-02-20  tags: ci, build

3 memories (3 active, 0 retired, 0 superseded)
```

**Options:**

```sh
# Filter by type
anvil edda list --type decision

# Filter by multiple types
anvil edda list --type decision,doctrine

# Filter by confidence
anvil edda list --confidence high

# Include retired and superseded memories
anvil edda list --include-retired

# JSON output for CI/CD integration
anvil edda list --json
```

### Inspect a memory

```sh
anvil edda show mem_a1b2c3d4
```

Shows the full memory: statement, context, confidence, attribution, provenance
links, and evolution history.

```
Memory: mem_a1b2c3d4
Type:   decision
Status: active

Statement
  All cross-package imports use published package names, never relative
  paths.

Context
  Why:    Relative paths across package boundaries break under pnpm
          workspace hoisting and produce runtime errors.
  When:   2026-03-01T14:23:00Z
  Scope:  monorepo
  Tags:   imports, monorepo, esm

Confidence
  Level:    high
  Rationale: Confirmed by multiple build failures; rule enforced by ESLint.

Attribution
  By:     alice
  When:   2026-03-01T14:25:00Z
  Reason: Pattern appeared in three Ember proposals; confirmed during
          onboarding incident review.

Provenance
  Ember proposal:  prop_ember_001 (pattern, confidence 0.82)
  Sessions:        ses_xyz001
  Observations:    obs_abc001, obs_abc002

Evolution
  Supersedes:      (none)
  Superseded by:   (none)
```

Use `--json` to get the full YAML structure as JSON, suitable for scripting or
piping into other tools.

### Promote an Ember candidate to memory

Promotion is the only path from Ember to Edda. You must supply a reason, an
actor, a confidence level, and a memory type.

```sh
anvil edda promote prop_ember_001 \
  --reason "Pattern confirmed in three separate incidents" \
  --by "alice" \
  --confidence high \
  --type decision
```

If you want to write your own statement rather than using the Ember proposal's
summary:

```sh
anvil edda promote prop_ember_001 \
  --reason "Pattern confirmed in three separate incidents" \
  --by "alice" \
  --confidence high \
  --type decision \
  --statement "All cross-package imports use published package names."
```

Promotion is irreversible in the sense that it cannot be undone — the Ember
proposal is marked `promoted` and the memory is created. The memory itself can
later be retired or superseded if it turns out to be wrong.

**Output:**

```
Promoted prop_ember_001 to Edda memory mem_a1b2c3d4

  decision  high
  All cross-package imports use published package names, never
  relative paths.

  Attribution: alice  2026-03-01T14:25:00Z
  Provenance:  prop_ember_001 -> obs_abc001, obs_abc002
```

### Retire a memory

When a memory is no longer valid — because circumstances changed, the pattern no
longer applies, or the decision was reversed — retire it.

```sh
anvil edda retire mem_a1b2c3d4 \
  --reason "Policy changed: paths now resolved via tsconfig paths" \
  --by "alice"
```

Retirement is a soft operation. The memory file stays on disk with
`status: retired`. It is excluded from `anvil edda list` by default but remains
queryable with `--include-retired`.

**Output:**

```
Retired mem_a1b2c3d4

  Status:  retired
  Reason:  Policy changed: paths now resolved via tsconfig paths
  By:      alice
  At:      2026-03-05T09:12:00Z
```

### Trace a memory

`trace` shows the full evolution history and provenance chain of a memory: where
it came from, how it has changed, and what it was replaced by (if anything).

```sh
anvil edda trace mem_a1b2c3d4
```

**Output:**

```
Trace: mem_a1b2c3d4

Evolution chain
  [1/1] mem_a1b2c3d4  decision  active
        Created 2026-03-01 by alice
        (no previous versions)

Provenance
  Ember proposal:  prop_ember_001
    Type:          pattern
    Confidence:    0.82
    Created:       2026-02-28T09:00:00Z
    Status:        promoted

  Kindling observations:
    obs_abc001  session ses_xyz001
    obs_abc002  session ses_xyz001

  Sessions:
    ses_xyz001  2026-02-28
```

If a memory has been superseded multiple times, `trace` walks the full chain
from the original version to the current one:

```
Evolution chain
  [1/3] mem_old001  decision  superseded
        Created 2026-01-10 by bob

  [2/3] mem_old002  decision  superseded
        Created 2026-02-01 by carol
        Superseded mem_old001

  [3/3] mem_a1b2c3d4  decision  active
        Created 2026-03-01 by alice
        Superseded mem_old002
```

## Promotion Workflow

The full lifecycle from Ember candidate to Edda memory:

```
Ember proposal (active)
       │
       │  Review with: anvil ember list
       │                anvil ember show <ember-id>
       │
       ▼
 Human decision:
   Promote ──► anvil edda promote <ember-id> ...  ──► Edda memory (active)
   Dismiss ──► (planned — use CandidateService API) ──► Ember proposal (dismissed)
   (wait)  ──► TTL expires                          ──► Ember proposal (expired)
```

To find Ember proposals ready for review:

```sh
anvil ember list
anvil ember list --type pattern,decision
```

Once you have identified a proposal worth keeping:

```sh
anvil ember show prop_ember_001
anvil edda promote prop_ember_001 --reason "..." --by "..." \
  --confidence high --type decision
```

## Evolution and Retirement

### Superseding a memory

When a memory needs to be replaced by a newer, more accurate version, use the
`--supersedes` flag on `promote`:

```sh
anvil edda promote prop_ember_002 \
  --reason "Updated policy after tsconfig paths migration" \
  --by "alice" \
  --confidence high \
  --type doctrine \
  --supersedes mem_a1b2c3d4
```

This creates a new memory, marks the old one as `superseded`, and links the two
together via the `evolution.supersedes` and `evolution.superseded_by` fields.
The old memory is preserved on disk.

### Retiring a memory without replacement

When a memory is simply no longer relevant — not superseded by a new version,
but withdrawn entirely:

```sh
anvil edda retire mem_a1b2c3d4 \
  --reason "Constraint no longer applies after migration to Bun" \
  --by "alice"
```

### Finding the current version

If you have an old memory ID and want to find the current active version:

```sh
anvil edda show mem_old001 --follow
```

With `--follow`, Anvil traces the `superseded_by` chain and displays the latest
version. Without it, it displays the memory at the exact ID you provided.

## Querying Memories

### Filter by type

```sh
anvil edda list --type decision
anvil edda list --type warning,constraint
```

### Filter by confidence

```sh
anvil edda list --confidence high
anvil edda list --confidence medium,high
```

### Search by text

```sh
anvil edda list --search "import"
```

Performs a case-insensitive substring match on the memory statement.

### Include retired and superseded

```sh
anvil edda list --include-retired
```

### JSON output

```sh
anvil edda list --json
anvil edda show mem_a1b2c3d4 --json
```

JSON output is stable and suitable for scripting. The structure matches the YAML
memory file format.

## CI/CD Integration

Use `--json` to integrate Edda queries into build pipelines or scripts.

Check for high-confidence warnings before a deployment:

```sh
#!/bin/sh
warnings=$(anvil edda list --type warning --confidence high --json \
  | jq '.memories | length')
if [ "$warnings" -gt 0 ]; then
  echo "Active high-confidence warnings: $warnings"
  anvil edda list --type warning --confidence high
fi
```

Check whether a specific memory exists:

```sh
anvil edda show mem_a1b2c3d4 --json | jq '.status'
# "active"
```

## Configuration

Configure Edda in your `.anvilrc` file under the `edda` key:

```json
{
  "edda": {
    "enabled": true,
    "storage": {
      "type": "git",
      "path": ".anvil/edda/",
      "format": "yaml"
    },
    "promotion": {
      "require_reason": true,
      "require_attribution": true,
      "min_ember_confidence": 0.5
    },
    "limits": {
      "max_statement_length": 2000,
      "max_context_length": 10000
    }
  }
}
```

**Key options:**

| Option                           | Default        | Effect                                   |
| -------------------------------- | -------------- | ---------------------------------------- |
| `storage.path`                   | `.anvil/edda/` | Where memory files are stored            |
| `promotion.require_reason`       | `true`         | Require a reason on every promotion      |
| `promotion.require_attribution`  | `true`         | Require an actor name on every promotion |
| `promotion.min_ember_confidence` | `0.5`          | Minimum Ember score to allow promotion   |

## FAQ

### Why can't Ember promote itself?

Human-in-the-loop is a core constraint of Edda, not a limitation. Ember is
heuristic and probabilistic — it is allowed to be wrong. Edda is not. Requiring
a human decision at the promotion step is what makes Edda trustworthy.

### Can I create a memory without an Ember proposal?

Not via `anvil edda promote`. If you want to create a memory directly — for
example, to capture a decision made in a meeting with no prior Ember signal —
contact your team lead or see the `MemoryService.createMemory` API in the
technical README.

### Can I edit a memory after promoting it?

Yes. `anvil edda` supports updating the statement, context, or confidence level
on an existing memory. The attribution and provenance fields are immutable. If
the changes are significant enough to constitute a new version, supersede the
old memory rather than editing it.

### Can I recover a retired memory?

No. Retirement is a terminal state. The memory remains on disk for inspection
but cannot be made active again. If the memory was retired in error, create a
new memory with the same content and use `--supersedes` to link it to the
retired one for traceability.

### Does Edda ever delete memories automatically?

No. There is no decay cycle in Edda. Unlike Ember, Edda has no TTL and no
automatic pruning. Memories remain until you explicitly retire or supersede
them. If you want to remove clutter, retire memories that no longer apply.

### Why is the storage directory a git repo?

Edda memories are stored as YAML files under `.anvil/edda/`, which is intended
to be tracked in version control. This gives you a permanent, diffable audit
trail without any database. Every promotion, update, and retirement becomes a
commit with the actor's name and reason as the commit message. If something
changes unexpectedly, `git log` will tell you who changed it and why.

### How do I search across all memories?

```sh
anvil edda list --search "auth"
```

Search performs a substring match on the memory statement. For more complex
queries, use `--json` and pipe to `jq`.

## Mental Model

Edda sits above two layers: Kindling, which records everything without
judgement, and Ember, which notices patterns without authority.

Edda's role is to be right. It does not speculate. It does not accumulate noise.
It stores only what has crossed a deliberate threshold: what you chose to call
true enough to keep.

The ledger is slow to fill. That is a virtue.

```
Kindling sees everything. Ember notices patterns.
Edda remembers what you choose.

The queue empties itself.
The ledger does not.
```
