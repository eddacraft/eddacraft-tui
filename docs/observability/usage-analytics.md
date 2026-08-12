# Usage Analytics Privacy Contract

| Type  | Authority     | Owner              | Status | Freshness                                                                                            |
| ----- | ------------- | ------------------ | ------ | ---------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | USAGE, FLEET, BACT | Live   | Live as of 2026-08-13 (BACT-007 DAI/DAA pointer to admin-cli.md; two-pipe contract still 2026-08-11) |

| Upstream                                                                                                                                                                                                                                                                                                                                                                                                                                      | Downstream                                                                                                |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| ADR-035, ADR-041, ADR-019, ADR-107 (incl. 2026-08-11 BACT amendment), ADR-121 (account plan / DAA), USAGE, FLEET, and BACT modules `plans/decisions/035-three-pipe-observability-rule.md`, `plans/decisions/041-flag-snapshot-usage-join-contract.md`, `plans/decisions/019-flags-observability-alignment.md`, `plans/decisions/107-fleet-telemetry-consent-posture.md`, `plans/decisions/121-account-plan-activity-and-flag-entitlements.md` | Usage-analytics producers, fleet beacon, signed-in account activity, public telemetry docs, and reviewers |

> **Status:** Live as of 2026-08-11 (BACT two-pipe clarification; FLEET-006
> beacon contract still as of 2026-07-16). This is the founder-confirmed privacy
> contract for local command-invocation observations, the ADR-107 remote
> aggregate, and the complementary signed-in beta account-activity pipe (BACT).
> Any change to the FLEET allowlist requires founder review and a dated ADR-107
> amendment. BACT identity-bound fields must never be joined onto the anonymous
> beacon.
>
> Normative references:
> [ADR-035](../../plans/decisions/035-three-pipe-observability-rule.md)
> (three-pipe rule — Kindling is the source-of-truth pipe),
> [ADR-041](../../plans/decisions/041-flag-snapshot-usage-join-contract.md)
> (inline `flag_set` join contract), and
> [ADR-019](../../plans/decisions/019-flags-observability-alignment.md)
> (gate-affecting-only Kindling flag facts).

This guide distinguishes **three** separate stories:

- **Local Kindling observations** retain detailed command and governance facts
  on the device. They are not uploaded.
- **The remote fleet beacon** is the narrow ADR-107 aggregate described below.
  It does not upload Kindling rows and cannot contain command names, arguments,
  paths, repository names, hostnames, emails, findings, or file contents. It
  answers population questions only (“is anyone using this feature?”).
- **Signed-in account activity (BACT)** is identity-bound server-side
  bookkeeping for customer success on authenticated accounts (login stamps on
  interactive session mint; allowlisted per-account feature touch; phase 2 adds
  account **`plan`**, **`last_activity_at`**, and **DAA** — daily active
  _accounts_, not installs, distinct from FLEET **DAI** — daily active
  _installs_). Never treat FLEET DAI as a customer login count. It answers “did
  _this_ user log in / stay active / use a core surface?” and must **not**
  re-identify fleet `install_id` values or put email on the anonymous beacon.
  Operator vocabulary for `plan`, `last_activity_at`, DAI vs DAA, and “quiet”
  (activity-idle) vs “never (interactively) logged in” is documented in
  [admin-cli.md](../runbooks/admin-cli.md#plan-activity-and-daa-vocabulary). See
  also
  [beta-account-activity](../../plans/modules/beta-account-activity.aps.md),
  [account plan / activity / entitlements](../guides/account-plan-activity-and-entitlements.md),
  [ADR-121](../../plans/decisions/121-account-plan-activity-and-flag-entitlements.md),
  and the 2026-08-11 amendment to ADR-107.

## Purpose

USAGE-001 records one durable usage observation per user-initiated CLI command
so the founder can answer **"who is using what"** for dev-investment decisions.
It records _that_ a command ran — never _what it produced_. The data lives on
Kindling, the source-of-truth pipe (ADR-035), as `command.invoked` rows.

## What is captured

Per invocation, the `command.invoked` row carries:

- **Command name** — the canonical command (e.g. `check`, `status`). For rows
  produced by the JSON-RPC daemon (USAGE-004, see below) this is the dispatched
  method name (e.g. `anvil/gctx/search_symbols`, `unblock-cascade`).
- **Anonymised principal** — a one-way SHA-256 hash of the user's email with a
  per-deployment salt. When no identity is on the call path the literal
  `anonymous` is recorded instead. The raw identity never appears in any field.
  On the daemon path the client supplies this already-hashed value on the
  JSON-RPC envelope (USAGE-004); the raw identity is never on the wire.
- **Timestamp** — RFC 3339.
- **Per-argument shape only** — for each argument: its name, plus shape fields
  (value type, a **coarse length bucket** — `empty` / `short` / `medium` /
  `long`, never the exact length — and presence). **Raw argument values are
  never recorded.** The length is bucketed deliberately so a fixed-length secret
  (a token or digest of known size) cannot be confirmed from an exact count.
- **Redacted sensitive arguments** — for arguments whose _name_ matches
  `anvil_observability::redaction::SENSITIVE_FIELDS`, even the shape fields are
  elided and replaced with the literal `<redacted>` marker (the value of the
  `REDACTED` constant). A sensitive argument's _existence_ (its name) stays
  visible; nothing about its value or length leaks.
- **Inline flag set** — the `flag_set` field defined by ADR-041. Each entry is
  `{ key, variant, source, gate_affecting }`, sorted by canonical manifest
  `key`. USAGE-002 populates it with the feature flags resolved **while
  authorising/routing the command** (today principally `cli.licence-gate`) — the
  observed invocation context, never a re-evaluation or a full manifest dump.
  `source` is the ADR-041 vocabulary (`override` / `snapshot` / `default`),
  mapped from the resolver's reason; `gate_affecting` is `true` for fail-closed
  classes (entitlement / ops-kill-switch) per ADR-019. **Scope (v1):** a flag
  resolved _inside_ a command (after routing) is **not** captured — see the
  scope note below.
- **`traceparent`** — the W3C cross-pipe correlation context (ADR-035) when one
  is bound on the invocation; omitted otherwise.
- **Producing binary version** (`version`, CIB-197) — the crate version the
  binary was built as (`CARGO_PKG_VERSION`). A low-risk dimension: a version
  string only — no path, no PII. Absent on rows written by pre-CIB-197 binaries;
  readers treat absence as "older producer".
- **Install method** (`install_method`, CIB-197) — the LAUNCH-013 detection
  already behind `anvil version`, as its closed snake_case vocabulary
  (`homebrew` / `scoop` / `winget` / `cargo_dist` / `cargo_install` /
  `dev_build` / `unknown`). A low-risk dimension: the label only — never the
  binary's filesystem path. Absent on rows written by pre-CIB-197 binaries.

## What is NOT captured

- **Raw argument values, ever.** Widening to a fuller capture is a follow-up
  review, not a routine code change.
- **Shape fields for sensitive-named arguments** — these collapse to the
  `<redacted>` marker.
- **Command results, output, stdout, stderr.**
- **File contents** touched by the command.
- **Network traffic in the Kindling observation.** The separate ADR-107 fleet
  beacon contains only the aggregate allowlist below.
- **Stack traces or error messages** — those stay on the tracing pipe.

### Residual risk: secrets under non-sensitive flag names

Redaction keys off the argument _name_ only. A secret passed under a
non-sensitive flag name (e.g. `--output <token>`) or as a positional is **not**
name-redacted: its coarse type and length _bucket_ are recorded (never the
value, and never an exact length). The deny-list is not complete protection
against a user who deliberately puts a secret in an arbitrary flag — the coarse
length bucket is the backstop. Operators should not rely on the deny-list as a
guarantee for arbitrary flag names.

## Anonymisation policy

The principal is a one-way SHA-256 hash of the email with a per-deployment salt
held at `<credentials_dir>/usage.salt` (mode `0600` on Unix), generated once on
first use from 256 bits of OS entropy. Rotating the salt is a deliberate privacy
reset — every historical principal hash becomes unjoinable — not a routine
operation.

## Remote fleet beacon (ADR-107)

An eligible interactive `anvil start` session on a beta, release-candidate, or
stable build sends at most one anonymous beacon per install per 24 hours. Alpha,
nightly, and other pre-beta builds never beacon. The first eligible start shows
the disclosure before any send. Network work runs in a detached worker with a
short timeout; failures are silent, are not queued, and do not delay the command
path.

The canonical body contains exactly:

- `schema_version`;
- random UUID v4 `install_id`, derived from nothing about the user or device;
- anvil `version` and the existing closed-set `install_method` label;
- the platform target triple and release `channel`;
- `flag_snapshot_version` (`0` while no remote snapshot is installed); and
- feature-key usage counts derived from local `flag_set` observations since the
  last successful beacon.

No local observation row, salted principal, command name, timestamp, argument,
path, repository fact, hostname, email, finding, output, or file content enters
the body. The ingest service coarsens arrival time to a date, retains raw rows
for 90 days, retains aggregates indefinitely, and does not retain source IPs.

All of these permanently suppress emission: `anvil telemetry off`,
`ANVIL_TELEMETRY=off`, and `DO_NOT_TRACK=1`. A non-default `ANVIL_HOME`, a
non-terminal first run that could not mark the disclosure as shown, and an
unreadable consent file also fail closed. `anvil telemetry reset-id` rotates the
random identifier, making previous and future beacons unjoinable.

Run `anvil telemetry` to see the exact canonical next body, or the exact reason
none can be sent. The JSON form is `anvil --json telemetry`. The public user
contract is
[Anonymous usage telemetry](../public/anvil/operations/telemetry.md); the
dimension and consent decision is
[ADR-107](../../plans/decisions/107-fleet-telemetry-consent-posture.md).

## Storage and retention

Usage is a cross-cutting, **user-scoped** signal, so rows are appended to
`<credentials_dir>/kindling/usage.ndjson` (the user/deployment state directory,
which re-roots under a gated `ANVIL_HOME` per DISTRIB-006). This mirrors the
audit-chain NDJSON sidecar; the Kindling-integration consumer tails the file.

On Unix the sidecar is created owner-only (`0600`) under an owner-only parent
(`0700`), matching the salt's posture so a shared host cannot read the usage
history; a symlinked target is refused. No permission restriction is applied on
Windows (the platform state-hardening gap tracked alongside DSV-010/011).

The salt and the state directory are created on **first run regardless of
authentication** — running any command (including help/probe commands)
materialises them, recording an `anonymous` row when no identity is present.

Retention is enforced by a **lazy in-process trim** applied before each append:
the sidecar is bounded to a rolling **7-day** age window and a **64 MiB** size
cap, whichever is tighter. Stale leading rows (older than the age window) are
dropped first; if the file is still over the byte cap the oldest remaining rows
are dropped until it fits. The trim is best-effort housekeeping (a failure
leaves the existing file intact) and rare on the hot path — a fast-path check
reads only the file's first line and skips the full read+rewrite unless a size
trim is due or the oldest line is already stale. A malformed or non-UTF-8 line
is skipped rather than aborting the trim, so a torn write cannot wedge retention
and let the file grow unbounded. The trim never rewrites through a symlink.

A long-lived consumer (the Kindling integration) is still expected to tail and
archive rows before they age out of this local window; the trim bounds the
on-disk sidecar, it is not the system of record.

### Operator controls (environment variables)

The producers honour a small set of environment variables. They are read fresh
per invocation (CLI) or per daemon start, so an operator can change behaviour
without a code change or redeploy. Unless noted otherwise the opt-out value is
the literal `1`.

| Variable                              | Scope              | Effect                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ------------------------------------- | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ANVIL_USAGE_DISABLE`                 | CLI producer       | `=1` declines CLI usage collection: the `command.invoked` producer writes nothing (no sidecar is created).                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `DO_NOT_TRACK`                        | CLI producer       | `=1` is honoured as an alias for `ANVIL_USAGE_DISABLE`, following the cross-tool [Console Do Not Track](https://consoledonottrack.com/) consent convention, so an operator who already sets it is opted out.                                                                                                                                                                                                                                                                                                                          |
| `ANVIL_INTERCEPT_DISABLE_OBSERVATION` | CLI + daemon       | `=1` is the whole-observation break-glass: it disables the daemon save-time / fence producers AND, for parity, the CLI `command.invoked` producer — one toggle silences every usage producer.                                                                                                                                                                                                                                                                                                                                         |
| `ANVIL_USAGE_SIDECAR_NO_TRIM`         | All sidecar writes | Any non-empty value disables the lazy retention trim described above, so the sidecar grows unbounded (for an operator who archives it externally and does not want the local trim mutating it).                                                                                                                                                                                                                                                                                                                                       |
| `ANVIL_OBSERVATION_INCLUDE_PATHS`     | Daemon producers   | `=1` **changes the privacy posture**: the daemon save-time `gate_evaluated` and fence `constraint_applied` rows then record the **absolute validated paths**, not just the path count. Off by default.                                                                                                                                                                                                                                                                                                                                |
| `ANVIL_KINDLING_SINK`                 | Daemon producer    | Selects the daemon `command.invoked` sink backend (KDS-002/-005): `daemon` (the **default**) writes to the local Kindling daemon (SQLite) via `kindling-client`, buffering to a bounded spool when the daemon is down; `off` disables the daemon `command.invoked` producer entirely. Case-insensitive. Unset, an unrecognised value, and the **retired** `ndjson` value (KDS-005 deleted the bespoke NDJSON sink) all resolve to `daemon` (with a warn for `ndjson`/typos) so capture is never lost. Local-only; see the note below. |

> **Privacy note — `ANVIL_OBSERVATION_INCLUDE_PATHS`.** With this set, absolute
> filesystem paths from the validated workspace are written into the usage
> sidecar. This is a deliberate diagnostics opt-in; leave it unset for the
> default count-only posture in any shared or privacy-sensitive environment.

> **`ANVIL_KINDLING_SINK` (KDS-002/-005).** This selects only the **daemon**
> `command.invoked` sink; it does not change the privacy posture (the backend is
> local-only). **As of KDS-005 the default is `daemon`** — the bespoke NDJSON
> sink is retired, so the daemon `command.invoked` rows go to the Kindling
> daemon and a daemon outage buffers them to
> `<credentials_dir>/kindling/spool.ndjson`, now **bounded** to a rolling 7-day
> / 64 MiB window (replayed when the daemon returns). `off` silences the daemon
> producer only — the CLI `command.invoked` producer is governed separately by
> `ANVIL_USAGE_DISABLE` / `DO_NOT_TRACK`, and
> `ANVIL_INTERCEPT_DISABLE_OBSERVATION` remains the whole-observation
> break-glass. (The CLI producer still writes `usage.ndjson`; the
> `anvil kindling usage` views read it unioned with the daemon, so they stay
> complete.)

## Dev-investment query views (USAGE-003)

`anvil kindling usage <view>` is the first-class surface for the founder's
standing questions — "what is being used and what is not" — over the local usage
data. It is local-only and needs no authentication (like `anvil insights`). It
reads `<credentials_dir>/kindling/usage.ndjson` and — unless
`ANVIL_KINDLING_SINK=off` — **also the Kindling daemon** (KDS-004; the daemon is
the default sink since KDS-005) — unioning the two so the views see every row
(degrading to sidecar-only, with a note, if the daemon can't be read — see
below). Pass `--json` for machine-readable output; the default is a small human
table.

> **Source for the views (KDS-004/-005).** The CLI producer always records to
> the sidecar; the daemon JSON-RPC producer records to the Kindling daemon (the
> default sink since KDS-005). So the views read **both** — the daemon rows (via
> `kindling-client` 0.3's exhaustive `list_observations`) unioned with the
> sidecar rows — for the full picture. If the daemon can't be read (e.g. a
> CLI-only host with no daemon running) the views degrade to sidecar-only and
> print a note to stderr (so `--json` stdout stays clean). Under
> `ANVIL_KINDLING_SINK=off` only the sidecar is read.

> **Rollback / hosts without the `kindling` binary (KDS-005).** With the daemon
> now the default sink, the resident `anvil intercept start` daemon writes
> `command.invoked` rows to the Kindling daemon (auto-spawning `kindling serve`
> on first use). On a host where the `kindling` binary is **not** installed,
> each emit instead spends the client's connect budget and buffers to the
> (capped) spool, and the client appends to `~/.kindling/spawn.log` — observable
> cost on the producer's drain thread (never the dispatch hot path). To opt out
> until kindling is deployed, set **`ANVIL_KINDLING_SINK=off`** (or the
> whole-observation **`ANVIL_INTERCEPT_DISABLE_OBSERVATION=1`** break-glass);
> both are read at **daemon start**, so restart the daemon after setting them.

| View         | Command                           | Answers                                                                                                                            |
| ------------ | --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Top commands | `anvil kindling usage top`        | Most-invoked commands. `--period week\|month\|all` (default `all`), `--limit N` (default 10, `0` = no cap).                        |
| Never used   | `anvil kindling usage unused`     | Registered commands with zero recorded invocations.                                                                                |
| Flag paths   | `anvil kindling usage flags`      | Flag-dependent paths _exercised_: each flag key seen in `flag_set`, its invocation count, variant breakdown, and whether it gates. |
| Principals   | `anvil kindling usage principals` | Anonymised principals by activity level (per the OQ2 contract — never a raw identity).                                             |

> **OQ3 decision (USAGE-003):** ship **both** a first-class CLI surface and this
> runbook. The CLI views are the supported path (tested end-to-end); the raw
> `jq` recipes below remain available for ad-hoc questions the views do not yet
> cover.

### Reading these views: signal, not evidence

These views inform **direction, not decisions in isolation**. Small populations,
flag bias (a path only runs when its gate is open), and survivorship effects all
distort the raw counts. Treat a "never invoked" command as a prompt to ask
_why_, not as proof it is dead. Known fidelity caveats:

- **Self-observation.** Running a view is itself a `kindling` invocation, and
  the producer records it _before_ the view reads the log. So `top` includes
  (and is inflated by) `kindling`; the running user's own principal appears in
  (and is inflated by) `principals`; and after the first `kindling usage unused`
  run, `kindling` permanently leaves the unused set. Discount your own analytics
  activity when reading the views.
- **`unused` is all-time.** It reports commands with _zero_ recorded invocations
  ever, not "unused lately" — a command run once long ago never appears, even if
  it has been idle for months. There is no `--period` on `unused`; use `top`'s
  window to reason about recent activity.
- **Sub-command naming.** `unused` compares clap's top-level command names
  against recorded canonical names. A command recorded under a finer-grained
  name than its clap name — `auth` runs as `auth-login` / `auth-logout` — can
  show as unused even though a sub-command ran.
- **`flags` reports "ever".** A flag's invocation count is the number of rows
  that carried it; its `[gate]` marker is `true` if it was gate-affecting in
  _any_ row, which reflects "ever gated", not the flag's current configuration.
- **Flag complement.** `flags` reports only the _exercised_ side (flags actually
  observed). The complement — manifest flags never observed ("not exercised") —
  needs the flag catalogue and is intentionally not computed by the read
  surface; cross-reference `flags/manifest.json` for the full set.

### Raw access (`jq` fallback)

The sidecar is newline-delimited JSON; any of the views can also be answered
ad-hoc (read-only, against `<credentials_dir>/kindling/usage.ndjson`):

- **Top-N commands:** count rows grouped by `.command`, sort descending.
- **Commands never invoked:** diff the registered command set against the
  distinct `.command` values present.
- **Which flag set was active for command X this week (USAGE-002):** filter rows
  to `.command == "X"` and the week's timestamps, then group by `.flag_set` (or
  by `.flag_set[].key` + `.flag_set[].variant`) to see the resolved flag context
  across those invocations. Example with `jq`:

  ```sh
  jq -c 'select(.command=="check") | {ts:.timestamp, flags:.flag_set}' usage.ndjson
  ```

- **Which gates fired for whom:** join `.flag_set[]` entries where
  `gate_affecting == true` to ADR-019 `flags_consulted` data by canonical `key`.
  Non-gate-affecting flags are inline invocation context only — not a standalone
  join row (ADR-019 / ADR-041 D-3).

## Change control

Any change to the captured / not-captured lists requires founder review. This
contract doc lives outside the code so a PR diff is always visible.

## Scope note: flag capture (USAGE-002 v1)

`flag_set` captures flags resolved **while authorising/routing** the command.
The CLI opens a thread-local capture window before the auth gate and the
resolver records each resolution into it; the usage row is emitted right after
that phase (on both the auth-pass and auth-fail branches, so every invocation
still gets exactly one row).

For a gated command, `check_auth` resolves `cli.licence-gate` once on **both**
the production path (manifest default → `source: default`) and the `ANVIL_DEV`
local-override path (`source: override`), so production gated invocations carry
the gate, not just developer sessions.

**USAGE-005 — flag-driven enforcement (landed).** The resolution is no longer
observe-only: `check_auth` now branches on the resolved variant via
`feature_flags::local_auth_precheck`. A `disabled` gate **skips** the local
credential pre-check; an `enabled` gate (including the manifest default)
**enforces** it. Precedence: the `ANVIL_DEV=1` developer bypass takes priority,
then the variant decides; `CLI_GATED_COMMANDS` stays orthogonal (it selects
which commands consult the gate at all). Because most gated commands run locally
and never call the server, for them the local pre-check _is_ the licence
enforcement — a `disabled` gate runs them ungated, which is the intended
operator control; the network-touching commands (`auth`, `mcp`) still require a
valid server token. The manifest default is `enabled`, so production enforcement
is unchanged unless an operator/targeting rule disables the gate.

A flag resolved **inside** a command — after routing, deep in execution — is
intentionally **not** captured in v1: several command paths exit via
`process::exit`, which would bypass an end-of-run emission and drop the row.
Widening capture to command-internal flags is deferred until those exit paths
are cleaned up, so row coverage is never traded for flag completeness.

## JSON-RPC daemon producer (USAGE-004)

USAGE-001 wired the producer at the CLI entrypoint only, deferring the JSON-RPC
daemon path because the dispatch boundary carried no user principal and no flag
resolver. USAGE-004 closes that gap; the contract below is the daemon-path
addition to everything above.

**Principal — supplied by the client.** The JSON-RPC envelope carries an
optional top-level `principal` field. Clients (the GCTX MCP query tools and the
`anvil intercept unblock` verbs) attach the **same** one-way salted hash the CLI
records — never a raw identity. The field is optional and length-capped. An
**absent** principal resolves to the literal `anonymous` (parity with an
unauthenticated CLI run), so existing clients stay wire-compatible; a
**malformed** value (non-string, or over the length cap) is a hard dispatch
rejection — the daemon rejects the request and records no row, rather than
attributing it to `anonymous`. The raw principal is never on the wire.

**Method scope — an explicit allowlist.** Only user-initiated methods emit a
`command.invoked` row: the GCTX query tools (`search_symbols`,
`find_dependents`, `find_callers`, `impact_of_change`, `affected_tests`) and the
operator `unblock-cascade` / `unblock-worktree` verbs. Internal machinery —
`scan_buffer`, `validate_paths`, `workspace_status`, `request_full_scan`, status
queries, and session-lifecycle verbs — is **excluded** and never recorded. A
two-directional test pins every protocol method as exactly one of
allowlisted/excluded, so a new method forces a deliberate decision rather than
silently leaking or under-counting.

**No double-counting.** The GCTX tools have no CLI-side row (only the `mcp`
startup command is CLI-recorded), so daemon rows are net-new signal. The
`unblock-*` verbs _do_ run as a CLI command, so the generic CLI-side `intercept`
row is **suppressed** for `anvil intercept unblock` — the daemon row is the
single source of truth for that operator action.

**Same captured/not-captured contract.** The daemon row reuses the exact shape,
redaction (`SENSITIVE_FIELDS` / `<redacted>`), and storage of the CLI row: the
JSON-RPC method name is the `command`, the request `params` object is reduced to
per-argument _shapes_ (no raw values), and rows append to the **same**
user-scoped `<credentials_dir>/kindling/usage.ndjson` sidecar. The path is
resolved by the CLI (which owns the `ANVIL_HOME`/credentials re-rooting) and
injected into the daemon, so the daemon and CLI never diverge on where rows
land. `flag_set` is empty on the daemon path (no resolver there). Emission is
keyed on _invocation_ — recorded before dispatch — but only for a frame the
dispatcher would accept: a malformed `principal`/`traceparent` or an
empty/over-limit JSON-RPC batch records nothing, because the dispatcher rejects
it (no phantom rows). It is strictly best-effort: a sink failure is logged once
on entering a failing run and suppressed per-call until recovery, never coupling
the dispatch path to sink health.

### Trust boundary, attribution, and nested params

- **The daemon-path principal is client-asserted, not authenticated.** The
  daemon records whatever salted hash the client put on the envelope; it does
  not derive or verify it from peer credentials. This is sound under the
  single-owner model — `usage.ndjson` is the per-user, owner-only (`0600`/`0700`
  on Unix) sidecar, so attribution lives entirely within one trust domain. If
  cross-user attribution integrity ever matters, the principal would need to be
  derived from the peer PID rather than the envelope. The hash is length-capped
  (256 bytes) and never interpreted; it is not a secret (it appears in every
  row).
- **Daemon `session_id` is the daemon's per-startup id**, stable across every
  row a given daemon instance produces — unlike the CLI's per-invocation
  `session_id`. Consumers grouping by `session_id` will see a bimodal
  distribution (one coarse daemon id + many fine CLI ids); individual daemon
  calls are correlated by `traceparent`, not `session_id`.
- **Nested params are recorded as a fixed marker, never measured.** A nested
  object/array argument is recorded as a presence-only fixed placeholder, so
  neither its values nor its _size_ leak — the coarse length bucket the flat CLI
  path uses is not applied to nested structure. A sensitive key nested inside a
  non-sensitive parent is therefore not individually marker-redacted, but its
  value is never captured either, so nothing sensitive leaks.
