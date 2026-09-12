# ADR-145: Continuous Command Journey

## Status

Accepted — 2026-09-10 (operator: JSIMP drain; autonomous complete). Decision
gate for [JSIMP](../modules/journey-simplification.aps.md). Implementation is
JSIMP-002..006 in dependency order; this record does not change runtime
behaviour by itself.

## Date

2026-09-10

## Context

JOURNEY-015 permitted simplification after the reliability rehearsal.
JOURNEY-013 re-measured first-run consent navigation on
`62e1facd70abd8de0311946e3fc6d4c401e9db7b` (help bar from `e586b6e53`) and
**closed as fixed**: no splash; no stepping-affordance change in that item.
Evidence:
[2026-09-10-journey-013-activation-navigation.md](../audits/2026-09-10-journey-013-activation-navigation.md).

The product already has the right verbs, but they do not yet feel like one
journey:

| Surface | Current role | Friction |
| ------- | ------------ | -------- |
| Bare `anvil` | Daily ensure (ADR-114) | First-use is an entitled action wall (exit 3) before unsigned discovery |
| `anvil welcome` | Ungated demo / learning (ADR-080) | Optional in copy; setup is a second implementation |
| `anvil start` | Activate / reconfigure (ADR-082, ADR-103, ADR-114) | Presentation, consent and mutation share one path; non-interactive defaults can install |
| `anvil status` / `anvil doctor` | Report / diagnose | Recovery copy still names intercept internals in places |
| `anvil intercept …` | Daemon operator/implementation | Comments reserve `ensure` / `restart`; those verbs are **not** shipped |

JSIMP-001 is the public-contract gate. Ready status authorises developing and
accepting this record through the existing ADR process, not silently changing
TTY, plain, JSON, CI or signed-out behaviour in this change.

Forces that must stay true:

- Planless-first, warnings-over-blocks, new-edges-only (ADR-001/002/003).
- MCP entries are activation-owned with the ADR-044 drift table until JSIMP-002
  lands an explicit migration (ADR-044).
- Welcome stays ungated; durable protection stays gated (ADR-080).
- Daemon auto-start belongs on activation; intercept stays an implementation
  detail (ADR-082).
- MCP is optional; Protecting is daemon-backed (ADR-092).
- TUI only on a genuine interactive terminal; `--verify` / `--json` / piped /
  CI stay deterministic (ADR-103).
- Bare ensure never first-installs NotPresent MCP, workflows or hooks
  (ADR-114).
- CIB-165: consent rows stay unticked by default.
- Dashboard, always-on app and splash are not this programme.

## Decision

Adopt **one continuous command journey** over the existing verbs. Do not add a
splash, dashboard, always-on shell, `anvil on`, `anvil ensure` public
subcommand, or `anvil intercept ensure` / `restart`.

### 1. Command roles

| Command | Role | First-use | Daily |
| ------- | ---- | --------- | ----- |
| **Bare `anvil`** | Ensure already-chosen coverage | Route before the licence wall when the project is never-activated: unsigned pointer to `welcome` / entitled pointer to `start`. No silent install. | Idempotent daemon + spine + already-owned MCP ensure (ADR-114) |
| **`anvil welcome`** | Optional learning and unsigned discovery | Not a prerequisite for `start`. May offer setup; accepted setup uses the same activation service as `start` | Returning hub; does not re-run first-run as if new |
| **`anvil start`** | Activate / reconfigure / reinstall with consent | Direct invocation is valid with no tutorial | Reconsider choices; not the daily on-switch |
| **`anvil status`** | Report the same facts the closing receipt uses | Non-mutating | Non-mutating |
| **`anvil doctor`** | Diagnose unresolved faults; `--fix` may call **shared** ensure/recycle operations | Not first-use | Not the daily on-switch |
| **`anvil intercept`** | Operator/implementation: `start --foreground`, `status`, `unblock`, `stop` | Not a user journey verb | Not a user journey verb |
| **`anvil mcp refresh --daemon restart`** | Emergency operator cascade for daemon recycle | Not first-use | Named recovery when ensure is the wrong tool |

Learning, project adoption, machine integration choices and ephemeral runtime
evidence remain separate stores. One project context is shared; progress,
explicit consent, beta gating, optional MCP, pins and security/admission
boundaries are preserved.

### 2. First-use bare routing (amends ADR-114 §2.3)

Keep ADR-114's honesty rule: never-activated bare **must not** run the full
`start` consent plan or write MCP/workflows/hooks.

**Never-activated** means `ConfigStatus::Absent` only (ADR-114). Invalid
config, welcome-seeded config, and spine-without-config stay on the entitled
ensure path. Do not invent a second predicate.

| Predicate | Licence wall | Behaviour |
| --------- | ------------ | --------- |
| Config Absent **and** unsigned | **No** — the only ADR-080 exception | Skip the wall and reuse the existing not-activated report (`report_not_activated` / `EnsureJsonReport`, human pointer to `anvil start` / `anvil welcome`). **Exit 1.** No writes, no daemon ensure, no MCP I/O, no new JSON fields, no schema bump |
| Config Absent **and** signed-in | Already authenticated; **not** an auth wall on this path | Same existing not-activated report; exit 1; no silent install |
| Config present (any other status) | **Yes** — daily ensure stays entitled | ADR-114 ensure path unchanged. Unsigned activated ensure remains exit 3 |
| `--help` / `-h` | No | Catalogue help, including the first-run blurb |

**JSON compatibility (JSIMP-003, versioned, tested):**

| Context | Old | Agreed |
| ------- | --- | ------ |
| Unsigned, config Absent, `anvil --json` | Action auth envelope (`state: "authRequired"`, `next: "anvil auth login"`), exit 3 | Existing not-activated ensure document, exit 1. **Replace**, do not add fields |
| Unsigned, config present, `anvil --json` | Auth envelope, exit 3 | Unchanged |
| Signed-in, config Absent, `anvil --json` | Not-activated ensure document, exit 1 | Unchanged |

Do **not** ungate daily ensure, `start` (except the existing
`start --verify` local-read skip), `status` (except `status --verify`), or
`watch`. `start --json` stays licence-gated. ADR-080 stands: welcome is the
demo in front of the wall; ongoing protection is not.

JSIMP-003 implements this routing.

### 3. Action versus output (JSIMP-002 contract)

Presentation cannot change intended mutations. **`--json` is never an install
channel.** Equivalent explicit intent across TUI and interactive plain yields
equivalent mutations; JSON only renders the same typed result.

**Unattended** means: stdin or stderr is not a TTY, **or** `CI` /
`ANVIL_NO_PROMPT` / `NONINTERACTIVE` is set. Stdout redirect alone (`anvil
start \| tee`) is presentation, not unattended — today's stderr consent picker
stays.

| Mode | May mutate? | Notes |
| ---- | ----------- | ----- |
| Interactive TTY (`anvil start` TUI) | Yes, after explicit consent (unticked defaults, CIB-165) | Trust boundary: stdin **and** stdout **and** stderr are TTYs (ADR-103) |
| Interactive plain (stdin and stderr TTY, TUI not selected) | Yes, after the same explicit consent | TUI vs plain must not diverge on operations |
| `--no-tui` / `ANVIL_NO_TUI=1` **today** | Auto-install branch even on a TTY (ADR-103) | Old behaviour. JSIMP-002 **amends ADR-103**: on a real TTY, `--no-tui` becomes interactive plain consent, not auto-install. Piped/CI stay on the ADR-044 table until the migration |
| `--verify` | **Never** | Report only. Signed-out `start --verify` / `status --verify` stay ungated |
| `--json` on `start` | **Never** | Read-only diagnostic. Stays byte-stable and licence-gated. A later mutating JSON start would need its own accepted ADR amendment |
| Unattended mutating `start` | Only with **explicit accepted intent** for **new** integration choices | See below |

**Explicit accepted intent** reuses existing `start` selectors. **Do not mint
a new `--yes` / `--apply` / second client-list flag.**

| Choice | After JSIMP-002 unattended migration | Until then |
| ------ | ------------------------------------ | ---------- |
| New MCP (`NotPresent`) | Requires `--mcp-client <id>` and/or `--all-mcp-clients` / `ANVIL_ALL_MCP_CLIENTS`. `--no-mcp` remains skip | ADR-044 auto-install |
| SafeDrift repair of **owned** MCP | May remain auto-repair (not a new integration choice) | ADR-044 auto-repair |
| UnsafeDrift / ExplicitOverride | Unchanged refusal / preserve | ADR-044 |
| GitHub workflows | No unattended selector exists → must **not** newly install | Already empty when `!interactive` |
| Git hooks (ADR-092 spine) | Spine, not an optional integration. Unattended silent hook install may continue; JSIMP-002 must not require a new `--hooks` switch | Current `install_activation_hooks_silent` |
| Project config / baseline / identity | First-time project files may still write; not MCP selection | Current |
| Daemon ensure | Spine; already suppressed off-TTY where ADR-082 requires | Current |

Flagless unattended `anvil start` keeps today's auto-install **until** JSIMP-002
ships the tested migration; after that it must not newly install NotPresent MCP
(recovery names the existing selectors). This ADR forbids silent change of that
script contract in any earlier item.

### 4. Shared setup and resume (JSIMP-003 contract)

Welcome, start and first-use bare share **one orchestrator implementation**,
not one ungated entry:

- One project context.
- Separate learning / adoption / runtime evidence.
- Cancelled or deferred setup resumes honestly.
- Direct `anvil start` needs no tutorial or welcome prerequisite.
- The licence gate stays on the **command**, not the shared service.
- Welcome remains ungated discovery plus the ADR-080 config-seeding it already
  has. It must **not** run start's MCP / hook / daemon / workflow mutations
  while unsigned. Entitled spine and MCP stay `anvil start` behind the wall.
- Do not add a parallel first-run product (`services/first_run.rs` stays a
  marker helper; welcome/bare call the existing activation orchestrator).

### 5. Remembered integration intent (JSIMP-004 contract)

Client, scope, executable and optional protection choices have **one durable
owner**, inferred from existing owned artefacts (project config, ADR-044
entries, hook/workflow presence). Do not add a decline-preference database
(ADR-114 already rejected that). Healthy bare ensure restores that coverage
without pickers or needless rewrites. Intentional omission or disablement is
distinct from failed installation. `anvil start` is the deliberate reconsider
path. Ordinary recovery uses the shared reliability operations already owned
by JREL; doctor is only for unresolved faults.

`doctor --fix` may invoke those shared operations for unresolved daemon-down
faults. It does not become the daily on-switch, must not grant unsigned daily
ensure (MCP ensure / spine / daemon-as-on-switch), and does not mint public
intercept verbs. `doctor` remaining ungated is unchanged; `--fix` must not
become an unsigned on-switch.

### 6. First value and closing receipt (JSIMP-005 contract)

Selected MCP coverage is proven by a real supported-client validation action;
save-time coverage by a real saved fixture. Clean and no-supported-language
outcomes stay honest. Demos stay isolated.

The persistent closing receipt names project, selected coverage,
connected/pending client, policy mode, last proof, and bare `anvil` for next
use. Incomplete setup names one actionable owner. Status and doctor consume the
same facts.

### 7. Operator verbs — no public intercept ensure/restart

**Do not ship** `anvil intercept ensure` or `anvil intercept restart`.

| Need | Name |
| ---- | ---- |
| Daily on-switch | bare `anvil` |
| Activate / reconfigure | `anvil start` |
| Daemon recycle / emergency cascade | `anvil mcp refresh --daemon restart` (existing) |
| Foreground daemon for operators | `anvil intercept start --foreground` |
| Inspect / fence / stop | `anvil intercept status` / `unblock` / `stop` |

Reserved exit codes and source comments that mention `anvil intercept ensure`
are not a public contract. JSIMP must not teach those names. Internal reuse of
ensure primitives is allowed; new public intercept verbs are not.

### 8. JOURNEY-013 residuals — no splash, no key rewrite

Consume the observation as **close as fixed**:

- Reject a splash, overlay, progress indicator, tutorial rewrite, always-on app
  and dashboard as JSIMP solutions.
- **Consent keys (superseded 2026-09-13 by
  [ADR-146](146-consent-space-toggle-enter-confirm.md)):** JSIMP itself was not
  a licence to rewrite stepping. The operator later aligned start/welcome
  consent with wizard/hooks: space ticks, Enter next/apply. `a` and `←/→`
  remain silent aliases. The no-splash rule in this section still stands.
- Residual 80-column truncation of `esc/q quit` is editorial / CIB-353 if
  promoted.

### 9. Entitlement and consent — no silent override

- ADR-080 gating list is unchanged except the never-activated unsigned bare
  pointer in §2 (a narrowing of the ensure wall, not an ungate of protection).
- ADR-044 drift classes and UnsafeDrift refusal remain. JSIMP-002 may add
  unattended-intent requirements; it may not start overwriting foreign MCP
  entries.
- ADR-092 MCP-optional spine remains. `--no-mcp` / `ANVIL_NO_MCP` stay skips.
- CIB-165 unticked defaults remain.
- Pins, beta gating, optional MCP and admission/security boundaries remain
  owned by their current ADRs and modules.

### 10. What this ADR does not do

- No runtime change in the JSIMP-001 landing PR.
- No public machine-contract change until the owning JSIMP item lands tests.
- No JOURNEY-009 / JOURNEY-010 hold lift.
- No release claim, claim freeze or publication.

## Rationale

The journey is already a small set of verbs. Reliability work (JREL) and
JOURNEY-013 removed the case for a new surface. The remaining defects are
routing (first-use hits the wall), duplicated setup, presentation leaking into
mutation, and recovery copy that names intercept internals.

Keeping intercept free of `ensure` / `restart` avoids a third on-switch after
ADR-114 just made bare `anvil` that switch. `mcp refresh --daemon restart`
already recycles a skewed daemon.

Changing non-interactive auto-install without a migration would break scripts
that depend on ADR-044. Naming the target now and migrating in JSIMP-002 is the
compatibility path.

### Alternatives Considered

| Option | Pros | Cons |
| ------ | ---- | ---- |
| **Chosen: existing verbs + routing/intent contract** | No new product; matches JOURNEY-013; preserves ADR-114 | Requires careful first-use auth exception and a later unattended migration |
| Splash / key overlay | Teaches keys | JOURNEY-013 closed this; expensive to walk back; duplicates the help bar |
| Public `anvil intercept ensure` / `restart` | Matches some comments and reserved exits | Second on-switch; leaks implementation; JREL already forbade teaching `intercept start --foreground` |
| Ungate all of bare ensure | Simplest first-use | Gives away daily protection; rejected by ADR-080's "ungate the whole set" alternative |
| Alias bare → full `start` | One implementation | Re-offers declined installs; TUI on every bare type; undoes ADR-114 |
| Change consent stepping (`Enter` applies, `↑/↓` cross sections) | Might help the residual wrap | JOURNEY-013 did not authorise it; silent change of a just-fixed help model |
| Dashboard / always-on app as the journey | One place to look | Out of scope; DASH remains flag-gated; not a CLI simplification |

## Consequences

- **Positive:** JSIMP-002..006 have a single accepted public contract. First-use
  can reach unsigned discovery without a splash. Daily ensure stays quiet.
  Scripts keep today's machine contracts until a tested migration.
- **Negative:** Never-activated unsigned bare becomes a documented exception to
  "bare is always an action wall". Unattended auto-install must migrate later,
  so two behaviours exist until JSIMP-002.
- **Risks:** Docs could describe the target as if it had already shipped.
  Intercept comments could keep teaching unshipped verbs.
- **Mitigations:** This ADR is the decision only. Public copy stays truthful
  about current behaviour and points at the transition matrix. JSIMP-006 aligns
  installer/help/quickstart after the behaviour exists. JOURNEY-016 accepts the
  integrated journey.

## References

- Related ADRs: ADR-044, ADR-080, ADR-082, ADR-092, ADR-103, ADR-114
- APS: JSIMP-001..006, JOURNEY-013, JOURNEY-015, JOURNEY-016, JREL-005
- Evidence: [JOURNEY-013 observation](../audits/2026-09-10-journey-013-activation-navigation.md)
- Transition matrix: [2026-09-10-continuous-command-journey](../specs/2026-09-10-continuous-command-journey.md)
