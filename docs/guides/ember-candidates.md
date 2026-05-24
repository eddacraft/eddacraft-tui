# Ember Candidate Review Workflow

| Type  | Authority     | Owner | Status | Freshness                                                                                                                   |
| ----- | ------------- | ----- | ------ | --------------------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | RCLI3 | Live   | Last reviewed 2026-05-25 against `packages/edda-stack/src/ember/` and `packages/edda-stack/src/contracts/ember-proposal.ts` |

| Upstream                                                                                                                                                                              | Downstream                                  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| `packages/edda-stack/src/ember/`, `packages/edda-stack/src/contracts/ember-proposal.ts`, `packages/edda-stack/src/contracts/proposal-types.ts`, `plans/modules/rust-cli-tier3.aps.md` | `anvil ember` CLI workflows, RCLI3 planning |

This guide explains what Ember candidates are, how they are generated, and how
to review, promote, or dismiss them using the Anvil CLI.

## What Are Candidates?

A candidate is a suggestion. Ember watches what happens during your work
sessions and periodically asks: "might this be worth remembering?"

When Ember thinks the answer is yes, it creates a candidate proposal. A proposal
is not a memory — it is a question. It sits in a queue waiting for you to decide
what to do with it. If you do nothing, it expires on its own.

Candidates exist because not everything that happens deserves to be remembered
permanently. Ember filters the signal from the noise, but it does not make the
final call. That decision belongs to you.

> Ember is a queue that empties itself. Not everything deserves to be
> remembered.

## How Candidates Are Generated

Ember generates candidates from Kindling observations. Kindling is the layer
beneath Ember: it records every observable event during a session — tool calls,
errors, plan completions, constraint violations, and more — as raw facts,
without interpretation.

When a session ends, Ember:

1. **Fetches observations** for the session from Kindling
2. **Groups them** into clusters by kind, timing, and repetition pattern
3. **Evaluates each cluster** against five built-in heuristic rules
4. **Scores the cluster** using weighted rule contributions
5. **Creates a proposal** if the score meets the confidence threshold

The five heuristics that drive evaluation:

| Rule        | What it detects                                            |
| ----------- | ---------------------------------------------------------- |
| repetition  | The same pattern appeared three or more times              |
| escalation  | Severity increased over time within a 24-hour window       |
| resolution  | A failure was followed by a successful resolution          |
| convergence | Two or more sessions produced the same observation pattern |
| surprise    | Observation kinds or timing were unusual or anomalous      |

Ember does not use AI to generate proposals in v1. Every confidence score comes
from deterministic heuristics. This means Ember will miss subtle patterns, but
it will never hallucinate provenance.

## Candidate Lifecycle

Every candidate moves through a fixed set of states:

```
active ──────── you promote it ──────► promoted (Edda memory created)
   │
   ├────────── you dismiss it ─────► dismissed (no memory created)
   │
   └────────── TTL expires ─────────► expired (no action taken)
```

- **active** — the proposal is open for review
- **promoted** — you decided this was worth keeping; Anvil will create an Edda
  memory from it
- **dismissed** — you decided this was noise; the proposal is closed with a
  reason
- **expired** — the TTL elapsed without action; the proposal is closed
  automatically

The default TTL is 30 days. Proposals expiring within 24 hours are flagged as
`expiring_soon` in `anvil status`.

Expired proposals are not deleted immediately. They are retained in the database
for 90 days so you can inspect what was missed. After that, they are pruned.

## CLI Usage

### List candidates

```sh
anvil ember list
```

Shows all active proposals, ordered by creation date (newest first).

**Options:**

```sh
anvil ember list --type warning          # Filter by type
anvil ember list --type pattern          # Filter by single type
anvil ember list --status expired        # Show expired proposals
anvil ember list --json                  # Machine-readable output
anvil ember list --limit 50              # Adjust page size (default: 20)
```

**Proposal types you can filter on:**

| Type         | Meaning                                         |
| ------------ | ----------------------------------------------- |
| `decision`   | A choice made with observable consequences      |
| `pattern`    | A recurring structure or behaviour              |
| `warning`    | A signal of potential problems                  |
| `lesson`     | A learning from failure or success              |
| `anomaly`    | An unexpected deviation from expected behaviour |
| `constraint` | A discovered limitation or boundary             |

### Inspect a candidate

```sh
anvil ember show <id>
```

Displays the full proposal: summary, rationale, confidence score, which
evaluation rules fired, provenance links to source Kindling observations,
creation and expiry timestamps, and current status.

The `id` is the proposal ID shown in `anvil ember list` output.

### Promote a candidate to memory

```sh
anvil ember promote <id> --reason "Confirmed in prod incident review" --by alice
```

Marks the proposal as promoted. Both `--reason` and `--by` are required.
Promotion is irreversible: the proposal's status changes to `promoted` and the
Edda memory ID is recorded on it for traceability.

| Flag       | Required | Description                   |
| ---------- | -------- | ----------------------------- |
| `--reason` | Yes      | Why this proposal is promoted |
| `--by`     | Yes      | Who is promoting (your name)  |
| `--json`   | No       | Output result as JSON         |

### Dismiss a candidate

> **Planned for a future release.** Dismissal is currently available only
> through the programmatic API (`CandidateService.dismissProposal()`). A CLI
> command (`anvil ember dismiss`) will be added in a future version.

## Configuration

Configure Ember in your `.anvilrc` file under the `ember` key:

```json
{
  "ember": {
    "enabled": true,
    "database": ".anvil/ember.db",
    "decay": {
      "default_ttl_days": 30,
      "min_ttl_days": 7,
      "max_ttl_days": 90
    },
    "evaluation": {
      "min_confidence": 0.3,
      "repetition_threshold": 3,
      "escalation_window_hours": 24
    },
    "limits": {
      "max_candidates": 1000,
      "max_proposal_size_kb": 64
    }
  }
}
```

**Key options to tune:**

| Option                               | Default | Effect                                                  |
| ------------------------------------ | ------- | ------------------------------------------------------- |
| `decay.default_ttl_days`             | `30`    | How long proposals live before auto-expiry              |
| `evaluation.min_confidence`          | `0.3`   | Lower threshold = more proposals; raise to reduce noise |
| `evaluation.repetition_threshold`    | `3`     | Raise to require more repetitions before firing         |
| `evaluation.escalation_window_hours` | `24`    | Reduce to tighten escalation detection window           |
| `limits.max_candidates`              | `1000`  | Hard cap; new proposals are rejected once reached       |

## FAQ

### Why did Ember propose this?

Run `anvil ember show <id>` to see the evaluation signals. The output lists each
rule that fired, its contribution to the confidence score, and any rule-specific
context (e.g. how many repetitions were detected, or what the severity delta
was).

The `provenance` section shows the Kindling observation IDs that triggered the
proposal. Kindling observations are not directly viewable via CLI in v1. Use the
Ember layer to surface relevant patterns.

### How do I adjust sensitivity?

Ember generates too many proposals:

- Raise `evaluation.min_confidence` (e.g. `0.5` or `0.6`)
- Raise `evaluation.repetition_threshold` (e.g. `5`)
- Shorten `decay.default_ttl_days` to empty the queue faster

Ember generates too few proposals:

- Lower `evaluation.min_confidence` (minimum is `0.0`, but `0.2` is more
  practical)
- Lower `evaluation.repetition_threshold` (minimum is `1`)

Changes take effect on the next session processed. Existing proposals are not
re-evaluated.

### What happens to expired candidates?

Expired proposals are marked with `status: expired`. They are not deleted
immediately. You can still inspect them with:

```sh
anvil ember list --status expired
anvil ember show <id>
```

After 90 days, expired (and other resolved) proposals are pruned from the
database permanently. This pruning runs automatically as part of the decay
cycle, which is triggered at the start of each session.

If you realise a proposal expired before you could review it, check its
provenance links and create an Edda memory manually if needed.

### Can I stop Ember from running?

Set `ember.enabled: false` in `.anvilrc`. Ember will not process sessions and
will not generate proposals. The database is preserved; you can re-enable at any
time.

### Does Ember ever write to Edda directly?

No. Ember proposes; it never creates Edda memories on its own. The
`anvil ember promote` command is the only path from a proposal to a memory, and
it requires explicit human action.

### Can I recover a dismissed or expired proposal?

No. Dismissal and expiry are terminal states. If a pattern was important, it
will likely recur in future sessions and Ember will propose it again.

## Mental Model

Ember sits between two layers: Kindling, which records everything without
judgement, and Edda, which stores only what is deliberately chosen to keep.

Ember's role is to be wrong some of the time. It proposes patterns it cannot be
certain about. You are the filter. Your job is to look at what Ember surfaced
and decide what is actually true enough to keep in Edda.

The queue empties itself by design. If you do nothing, proposals expire. This is
intentional — an untended queue is not a memory system, it is a backlog.

```
Kindling sees everything. Ember notices patterns.
Edda remembers what you choose.

The queue empties itself.
The ledger does not.
```
