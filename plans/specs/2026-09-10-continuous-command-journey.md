# Continuous Command Journey — Transition Matrix

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative | JSIMP | Accepted | Last reviewed 2026-09-10 against ADR-145 and JOURNEY-013 observation |

| Upstream | Downstream |
| -------- | ---------- |
| [ADR-145](../decisions/145-continuous-command-journey.md), [ADR-044](../decisions/044-mcp-entry-activation-owned.md), [ADR-080](../decisions/080-ungate-welcome-demo-surface.md), [ADR-082](../decisions/082-daemon-lifecycle-user-startup.md), [ADR-092](../decisions/092-mcp-optional-activation-spine.md), [ADR-103](../decisions/103-tty-default-activation-tui.md), [ADR-114](../decisions/114-bare-anvil-ensure-surface.md), [JOURNEY-013 observation](../audits/2026-09-10-journey-013-activation-navigation.md) | JSIMP-002..006, JOURNEY-016, `docs/public/anvil/quickstart.md`, `docs/runbooks/cli-surface.md` |

This matrix enumerates **old** (current shipped) versus **agreed** (ADR-145)
behaviour and the compatibility rule for each entry context. JSIMP-001 records
the contract only. A row marked "current until JSIMP-00N" must not change in an
earlier item.

## Command map

| Verb | Old | Agreed | Compatibility |
| ---- | --- | ------ | ------------- |
| Bare `anvil` | Daily ensure; never-activated pointer after the wall. Unsigned never-activated `anvil --json` is the action auth envelope (`state: "authRequired"`, `next: "anvil auth login"`), exit 3 | Daily ensure unchanged when config is present. Config **Absent** skips the wall and reuses `report_not_activated` (exit 1, existing ensure JSON, no new fields) | Activated unsigned ensure stays exit 3. JSIMP-003 owns the tested envelope → not-activated swap |
| `anvil welcome` | Ungated discovery / hub; guided setup seeds project config only | Optional learning; **same orchestrator implementation** as `start`, but the licence gate stays on the command. Unsigned welcome must not run start MCP/hook/daemon/workflow mutations | Ungated status stays (ADR-080). Sharing lands in JSIMP-003 |
| `anvil start` | Activate / reconfigure; TTY TUI; `--verify` / `--json` read-only; non-interactive mutating path auto-installs NotPresent (ADR-044) | Same roles. Presentation cannot change mutations. Unattended new installs need explicit accepted intent | `--verify` / `--json` byte-stable and read-only until an explicit versioned migration with tests. Auto-install remains until JSIMP-002 |
| `anvil status` | Entitled report; `--verify` probe | Same facts as the closing receipt; still non-mutating | Existing JSON fields stay or migrate explicitly in JSIMP-005 |
| `anvil doctor` | Diagnose / `--fix` | Unresolved faults only; may call shared ensure/recycle **internally**; not the daily on-switch | No new public intercept verbs |
| `anvil intercept ensure` / `restart` | **Not shipped.** Comments and reserved exits mention them | **Do not ship.** Daily ensure is bare `anvil`; recycle is `anvil mcp refresh --daemon restart` | Existing intercept subcommands (`start --foreground`, `status`, `unblock`, `stop`) unchanged |
| Splash / always-on / dashboard | Not the CLI journey | Rejected as JSIMP solutions | DASH remains flag-gated and out of this programme |

## Entry contexts

### Interactive TTY

| Path | Old | Agreed | Compatibility |
| ---- | --- | ------ | ------------- |
| `anvil start` | Activation TUI when stdin, stdout and stderr are TTYs (ADR-103) | Unchanged trust boundary | TUI eligibility tests stay |
| Consent picker | Help bar names `↑/↓` `←/→` `space` `a` `esc/q`; `↑/↓` wrap in-section | **Keep.** No splash. No stepping change | JOURNEY-013 residuals are editorial, not a key rewrite |
| Consent defaults | Unticked (CIB-165) | Unticked | Unchanged |
| `anvil welcome` then setup | Separate guided-setup implementation | Same service as `start`; resume cancelled/deferred honestly | Lands in JSIMP-003 |
| Bare `anvil` after activation | Quiet ensure; no picker | Unchanged | ADR-114 fixtures stay |

### Plain (`--no-tui` / `ANVIL_NO_TUI=1`)

| Path | Old | Agreed | Compatibility |
| ---- | --- | ------ | ------------- |
| `anvil start --no-tui` | Forces the non-interactive auto-install branch **even on a TTY** (ADR-103; `is_interactive` is false when `global.no_tui`) | JSIMP-002 **amends ADR-103**: `--no-tui` on a real TTY (stdin and stderr TTY) becomes interactive plain consent, not auto-install | Breaking; tested migration in JSIMP-002. Until then the auto-install branch stands |
| Piped `--no-tui` | Non-interactive defaults (ADR-044) | Unattended new MCP needs existing `--mcp-client` / `--all-mcp-clients` | Until JSIMP-002, ADR-044 table stands |

### JSON

| Path | Old | Agreed | Compatibility |
| ---- | --- | ------ | ------------- |
| `anvil start --json` | Read-only diagnostic; one JSON document; refused with `--watch`; **licence-gated** (exit 3 unsigned). `start --verify` is the ungated local skip | **Stay read-only and gated.** JSON is a render, never an install channel. No “equivalent mutations” via JSON | Byte-stable until a later ADR explicitly amends this |
| `anvil --json` (bare), config Absent, unsigned | Action auth envelope, exit 3 | Existing not-activated ensure document, exit 1. Replace the envelope; do not add pointer fields | Versioned JSIMP-003 swap with tests |
| `anvil --json` (bare), config present | Compact ensure document (or auth envelope if unsigned) | Unchanged | ADR-114 |
| Global `--json` | CLI-wide one-document contract | Unchanged | `json_surface_audit` remains the classifier |

### CI / piped / `ANVIL_NO_PROMPT`

| Path | Old | Agreed | Compatibility |
| ---- | --- | ------ | ------------- |
| Unattended mutating `anvil start` | Unattended = stdin or stderr not a TTY, or `CI` / `ANVIL_NO_PROMPT` / `NONINTERACTIVE`. Stdout-only redirect is **not** unattended. Today: NotPresent MCP auto-install; SafeDrift auto-repair; silent git hooks; no GitHub workflow writes; daemon auto-start suppressed off-TTY | After JSIMP-002: new MCP requires existing `--mcp-client` / `--all-mcp-clients`; `--no-mcp` skip; SafeDrift repair may stay; hooks remain spine (no new `--hooks`); workflows stay off; no new `--yes` flag | **Current auto-install remains until JSIMP-002** ships tests |
| `--verify` | Non-mutating, no hang; ungated | Unchanged | Byte-stable |
| Bare ensure | Deterministic ensure or refuse; no prompts | Unchanged when config is present | ADR-114 |

### Signed-out

| Path | Old | Agreed | Compatibility |
| ---- | --- | ------ | ------------- |
| `anvil welcome` | Ungated discovery; config-seeding only | Ungated discovery; still no unsigned MCP/hook/daemon/workflow mutations | ADR-080 |
| `anvil start` | Licence wall (exit 3) | Licence wall (exit 3) | Unchanged |
| `anvil start --verify` / `anvil status --verify` | Ungated local-read skip | Ungated local-read skip | Unchanged |
| `anvil start --json` | Licence wall (exit 3) even though read-only | Licence wall (exit 3); not a verify-style skip | Unchanged |
| Bare `anvil`, config Absent | Licence wall (exit 3) before the pointer | Existing not-activated report, exit 1, no writes/daemon/MCP I/O | Lands in JSIMP-003; other statuses stay entitled |
| Bare `anvil`, config present | Licence wall if unsigned; ensure if signed-in | Unchanged | ADR-114 |
| `anvil status` / `watch` (non-verify) | Licence wall | Licence wall | Unchanged |

## Implementation ownership

| Contract slice | Item | Runtime change allowed? |
| -------------- | ---- | ----------------------- |
| This matrix + ADR-145 | JSIMP-001 | No |
| Action / consent vs output; unattended intent; `start --json` compatibility tests | JSIMP-002 | Yes, with migration tests |
| Shared setup + first-use bare routing + resume | JSIMP-003 | Yes |
| Durable integration intent + quiet daily recovery | JSIMP-004 | Yes |
| First-value proof + closing receipt | JSIMP-005 | Yes |
| Public guidance + journey verify | JSIMP-006 | Docs/e2e after behaviour exists |
| Cross-module acceptance | JOURNEY-016 | After JSIMP-002..006 Merged |

## Non-goals

- Splash, key-hint overlay, progress indicator, always-on app, dashboard.
- Public `anvil intercept ensure` / `restart`.
- Public `anvil ensure` subcommand (bare `anvil` remains the name; `ensure` is
  only the internal canonical command id).
- Stepping-affordance rewrite.
- Entitlement expansion beyond the never-activated unsigned pointer.
- Release claim or publication.
