# Repository Policy Contract

Load repository policy **before** asking the user for mode. Search in order;
use the first file that exists and contains a `devLoop` (or `dev_loop`) key:

1. `dev-loop.policy.yaml`
2. `dev-loop.policy.yaml` in the active harness adapter's config directory
   (e.g. `.claude/`, `.codex/`, `.opencode/`, `.grok/`)
3. `plans/dev-loop.policy.yaml`
4. Fragment under project docs if linked from `AGENTS.md` / `CLAUDE.md`

If **no** policy file exists:

| Context                        | Default                                                                                            |
| ------------------------------ | -------------------------------------------------------------------------------------------------- |
| Interactive human session      | `interactive` — terminal state `review-ready`                                                      |
| Unattended / batch / CI agent  | still no self-merge; park `awaiting-merge-authority` unless `autonomous` + `autonomousMerge: true` |
| Explicit user "run autonomous" | session override; record in evidence                                                               |

**Never infer merge authority from silence.**

## Schema (semantics)

```yaml
devLoop:
  defaultMode: interactive # or autonomous
  integrationBranch: main
  isolation:
    provider: worktrunk # worktrunk | git-worktree | harness
    # Optional CLI name/path when provider is worktrunk (for example git-wt on Windows Winget).
    # When unset, isolate-workspace resolves WORKTRUNK_BIN, then PATH (git-wt before wt).
    command: null
    requireFor: [module, autonomous, parallel]
    alwaysRunSetup: true
  harnessIdentityCheck: auto # auto | strict | off — see Harness identity check below
  pullRequests:
    defaultBoundary: invocation-target
    autonomousMerge: false
    # Durable fix: enable GitHub "Automatically delete head branches".
    deleteBranchOnMerge: true
    allowStacks: true
  claims:
    provider: git-ref # git-ref | manual | anvil (reserved; behaviour defined by the next-phase coordination module)
    leaseMinutes: 30
    heartbeatMinutes: 10
  repair:
    maxCycles: 3
    stopOnNoProgress: true
  execution:
    wallclockMinutes: 240
    maxConcurrentWriters: 2
    maxItemsPerWave: 2
  verifier:
    aggregateMinutes: 45
    commandMinutes: 15
    staleMinutes: 20
    maxAttempts: 2
  risk:
    default: standard
    pathRules: []
    mandatoryDifferentialDesign:
      [architectural, security-sensitive, irreversible, materially-ambiguous]
    crossModelVerification: [high, critical, disputed] # preference only while differential mode is enabled
  gates:
    required: []
    postMerge: []
  aps:
    reconcileOnLand: true
    # When a human merges outside the loop, where may APS/docs reconcile land?
    # bookkeeping-pr     — open a small PR to integration (default, safest)
    # integration-direct — allow docs/APS-only commits on integration (must match
    #                      repo history culture; still never for product code)
    # forbid             — stop and ask; no reconcile write until a loop-owned PR
    outOfBandMergeReconcile: bookkeeping-pr
  checkpoints: # the three human gates (chiefly for drain mode)
    design: human # (1) approach approval        — human | auto
    ready: human # (2) the Ready membrane        — human | auto
    merge: human # (3) entry to shared history   — human | auto
  budget: # one shared repair/spend ceiling per run; on exhaust, park and escalate
    unit: repair-cycles # repair-cycles | tokens | wallclock | usd
    limit: 3
  preset: collaborative # sugar; explicit fields above override preset values
  differential:
    mode: off # off is an explicit hard privacy disable
    minimumProviders: 2
    requiredFor: [] # may elevate preferred to required; never enables an off policy
    allowedAdapters: [] # empty = any safely discovered adapter
    roles:
      worker:
        weight: fast
        candidates: [active]
      orchestrator:
        weight: balanced
        candidates: [active] # fixed: authority transfer requires a new invocation
      advisor:
        weight: frontier
        candidates: [claude, grok, codex, active]
        differentFrom: orchestrator
        separation: preferred
      verifier:
        weight: frontier
        candidates: [codex, claude, grok, active]
        differentFrom: worker
        separation: preferred
```

`execution.wallclockMinutes` is an aggregate run limit, not a per-child limit.
It is mandatory for module, drain, and autonomous runs. `maxConcurrentWriters`
and `maxItemsPerWave` bound fan-out; child runs do not receive fresh budgets.

`verifier.aggregateMinutes` covers the initial attempt, commands, waits, and the
single permitted retry. It must exceed `commandMinutes`; stale detection never
fires while a command is still inside its declared timeout. Expiry cancels the
whole verifier run and returns `blocked` with partial evidence.

All failed evidence, verifier, CI, and review transitions consume the same
repair budget only when they cause another implementation-writing repair. Hash
the bounded diff plus open finding ids after every repair. If that progress
fingerprint repeats while `stopOnNoProgress` is true, stop immediately; do not
spend another cycle reproducing the same state.

### Isolation provider and Worktrunk CLI

| Field                      | Meaning                                                                       |
| -------------------------- | ----------------------------------------------------------------------------- |
| `isolation.provider`       | `worktrunk`, `git-worktree`, or `harness`                                     |
| `isolation.command`        | Optional executable name or absolute path for Worktrunk                       |
| `isolation.requireFor`     | Contexts that always receive a dedicated worktree                             |
| `isolation.alwaysRunSetup` | Whether dependency setup is rerun after worktree creation or first attachment |

When `isolation.command` is set, it wins and must not be silently replaced when
missing. Otherwise resolve `WORKTRUNK_BIN`, then probe `git-wt` before `wt` so a
Windows Terminal executable is not mistaken for Worktrunk. Ignore Worktrunk
resolution when another provider is selected unless cleanup explicitly needs
the executable recorded at isolation time.

```yaml
isolation:
  provider: worktrunk
  command: git-wt
```

### Differential model routing

`dev-loop-differential` composes this policy with the active harness adapter. A
normal `dev-loop` run defaults to `mode: off` when repository policy omits the
field. Direct invocation of `dev-loop-differential` defaults to `preferred` only
when the field is absent; an explicit repository `off` always wins. The overlay
does not change merge or write authority.

Weights (`fast`, `balanced`, `frontier`) are capability and cost
classes, not model identifiers. Adapters map them to the current harness. The
advisor and verifier resolve independently because their authority differs:

- `worker` is the routing-policy name for the optional delegated implementation
  role. Under ADR-0025 the lead implements directly by default, so a run may
  legitimately resolve no worker at all;
- advisors provide strategy, design, risk, decomposition, conflict, or taste
  judgement and never implement;
- verifiers inspect the bounded change and run evidence, remain read-only with
  respect to implementation, and emit the canonical evidence bundle.

Candidate values are logical adapter identifiers. `active` means the current
harness. Users may reorder, restrict, or disable worker, advisor, and verifier
candidates without storing credentials in this file. The `orchestrator` key is
the historical spelling of the **lead** role and is fixed to
`candidates: [active]`: moving lead authority to another harness is a separate
invocation and explicit hand-off, not role routing inside one run.

#### Safe discovery

Discover adapters from executable presence, plugin manifests, harness capability
reports, and non-secret authentication probes. Do not read tokens, API keys, or
secret-bearing runtime configuration contents. Availability requires both a
transport and the capabilities needed by the role; an installed but
unauthenticated or write-only adapter is not a usable read-only verifier.

#### Effective mode

Resolution order is built-in default → repository policy → invocation → risk
elevation:

1. An explicit repository `mode: off` is a hard privacy disable. Neither risk
   nor a session override may enable external routing for that run.
2. If repository policy omits `differential.mode`, an ordinary `dev-loop` run
   defaults to `off`; direct invocation of `dev-loop-differential` defaults to
   `preferred`.
3. A risk or dispute in `requiredFor` elevates an already-enabled `preferred`
   mode to `required`; it never elevates `off`.
4. Effective global `required` upgrades every used role that declares
   `differentFrom` to required separation. A role-level
   `separation: required` also fails closed while global mode is `preferred`.
5. Any permitted human override must be explicit and recorded, but cannot
   override an explicit repository `off`.

#### Role-specific fallback

| Missing or failed route | `preferred`                                                                 | `required`                                                 |
| ----------------------- | --------------------------------------------------------------------------- | ---------------------------------------------------------- |
| Advisor                 | Try next candidate, then fresh same-harness advisor or policy-approved skip | Stop if a required advisor gate cannot run                 |
| Verifier                | Try next candidate, then fresh same-harness verifier and record degradation | Stop `blocked`; never manufacture independent verification |
| Worker                  | Use active harness at the nearest supported weight                          | Stop only when policy requires the unavailable capability  |

The lead has no adapter fallback: it remains the active harness. To change it,
finish or block the run and start a new invocation with an explicit authority
hand-off.

The verifier gate never disappears. Under `preferred`, only its degree of model
or provider separation may degrade.

Write the resolved routing to checkpoint `modelRouting` using
`differential-routing.schema.json`. Record every adapter switch, missing
capability, and same-provider fallback. Model diversity is evidence about
independence, never proof of correctness.

Set `differentialAchieved: true` only when `providersUsed` contains at least
`minimumProviders` distinct values, the lead is selected, the verifier is
selected, and every used role with `differentFrom` resolved to
`separation: different-provider`. Record false while routing is incomplete or
any relevant role degraded. Advisor omission represents an unused optional
gate; the verifier slot is mandatory and cannot be omitted or skipped.

### Checkpoint semantics and presets

A checkpoint set to `auto` is **not deleted**: the gate still runs, still emits
its artifact (design doc / readiness record / land record), and writes a
self-approval entry to the journal — the audit trail is identical in structure
to a human approval. `checkpoints.merge: auto` still requires
`pullRequests.autonomousMerge: true`, branch-protection compliance, and the
integration-ancestor gate; the safety rails (`aps-safety-rails`) forbid
auto-merging contract- or migration-class changes regardless of any field here.

| Preset          | design | ready | merge | Merge path            | Notes                                              |
| --------------- | ------ | ----- | ----- | --------------------- | -------------------------------------------------- |
| `collaborative` | human  | human | human | PR only               | DEFAULT — all gates human                          |
| `design-once`   | human  | auto  | auto  | PR, merge on green CI | approve the approach, then drive                   |
| `autopilot`     | auto   | auto  | auto  | PR only               | rails still apply; contract-class changes escalate |
| `spike`         | auto   | auto  | auto  | direct merge          | throwaway repos only                               |

Resolution is later-wins per field: built-in default (`collaborative`) →
repository policy file → session-scoped explicit override. Record the fully
resolved policy in the journal at run start.

## Resolution

1. Load repository policy (paths above).
2. Overlay APS-declared requirements.
3. Raise risk/gates when project truth demands it.
4. Apply human override only when session-scoped, explicit, recorded, and permitted.

Mode controls checkpoints and terminal authority, never target scope.

### Out-of-band merge reconcile

Humans often merge GitHub PRs while the loop is idle. APS status then has no
feature branch left.

| Policy value         | Loop may                                                                                                                                            |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bookkeeping-pr`     | Branch from integration, APS/docs-only commit, open PR (or attach to existing bookkeeping PR)                                                       |
| `integration-direct` | Commit APS/docs-only updates directly on a clean, up-to-date integration branch after ancestor proof of the feature merge. **Not** for product code |
| `forbid`             | Report `needs-plan-update` / ask user; do not write plan files                                                                                      |

Always run **integration-ancestor** verification before any APS `Merged` write.

## Override record

Record operator, session, target, policy varied, authority, rationale, issued
and expiry timestamps, resulting actions. Overrides never bypass branch
protection or destructive-action safeguards outside their scope.

## Harness identity check

`dev-loop-router` is projected byte-identically into every harness skill root and
resolves the active harness at runtime from shell-verified environment signals
(ADR-0025). It then loads exactly one uniquely named `dev-loop-adapter-*` skill.
`harnessIdentityCheck` controls whether that resolution runs. Full mechanics:
`harness-identity-check.md` (same directory).

- `auto` (default) and `strict` — run the shell-verified environment check
  before loading an adapter. The two values are equivalent since the
  per-harness bindings were retired; `strict` remains accepted so existing
  policy files keep validating.
- `off` — skip the probe. This is only valid together with an explicit
  `pinnedAdapter`, which names the adapter to load. Without a pin the router
  still fails closed: it never guesses an adapter and never proceeds unbound.

```yaml
devLoop:
  harnessIdentityCheck: off
  pinnedAdapter: dev-loop-adapter-codex
```

The check never changes checkpoints, merge authority, or the safety rails; it
only decides which adapter's native choreography applies, or stops to avoid
running the wrong one.
