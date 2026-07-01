# anvil-observability — As-Built

| Type     | Authority | Owner | Status | Freshness                                                                                                                                                                                                                                                                                   |
| -------- | --------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| As-built | Derived   | TRACE | Live   | Line-count reconciliation 2026-07-02 against main `d1fded280` (`redaction.rs` 409→638; SENSITIVE_FIELDS/REDACTED/lib.rs/traceparent.rs counts re-verified unchanged); prior targeted delta review 2026-06-10 against main `45dd1047a`; full content review 2026-05-07 against `v0.6.0-beta` |

| Upstream                               | Downstream                                      |
| -------------------------------------- | ----------------------------------------------- |
| `crates/anvil-observability/`, ADR-035 | CLI tracing, intercept tracing, MCP correlation |

> **Status:** Live (beta) **Last reviewed:** line-count reconciliation
> 2026-07-02 against main `d1fded280`; prior targeted delta review 2026-06-10
> (live redaction layer, trace sink, TS mirror, TRACE-004 namespaces) against
> main `45dd1047a`; full content review 2026-05-07 against `v0.6.0-beta` (HEAD
> `d223b8d9`) **Crate / location:** `crates/anvil-observability` (package
> `eddacraft-anvil-observability`, lib name `anvil_observability`) **Module
> owner (APS):** TRACE
> ([`plans/modules/tracing-foundation.aps.md`](../../plans/modules/tracing-foundation.aps.md),
> In Progress 2/4 — TRACE-001 Complete, TRACE-004 Complete (2026-05-11,
> call-path instrumentation + `anvil.cli.*` / `anvil.intercept.*` namespaces),
> TRACE-002 / TRACE-003 Blocked after partial implementation — local slices
> shipped; the dashboard-consumer / EXPORT-parity slices are blocked); co-owned
> with OBS
> ([`plans/modules/observability-foundation.aps.md`](../../plans/modules/observability-foundation.aps.md),
> Draft 0/5 — OBS-006 migrated to TRACE-001 on 2026-04-30) **Used by:**
> `anvil-cli` (subscriber init), `anvil-intercept` daemon (subscriber init,
> JSON-RPC `traceparent` envelope, fanout cross-pipe correlation), MCP shim
> (correlation traceparent round-trip via the daemon envelope). The redaction
> deny-list is **live**: `init_tracing` installs `RedactingJsonFields` /
> `RedactingJsonEventFormatter`, which replace any span/event value whose field
> name matches `SENSITIVE_FIELDS` with the `REDACTED` marker before JSON output
> (TRACE-003 partial slice, `crates/anvil-observability/src/lib.rs:146-147`).
> The residual cross-binary / EXPORT-policy-parity slice is tracked in §"Known
> gaps" G-02.

## 1. Overview

The cross-cutting observability primitives shared across the Rust workspace: a
W3C `traceparent` parser/generator, a per-binary `tracing-subscriber` JSON
initialiser with a local trace-sink selector, and a live field-name redaction
layer that replaces sensitive span/event values before JSON output. Modest
surface (three files, ~1,500 lines), load-bearing footprint — every binary
entrypoint calls into it once, the JSON-RPC envelope validates traceparent on
every request, and the namespace-registry contract documented at
`docs/observability/namespace-registry.md` cites this crate as the Rust producer
of record.

Per [ADR-035](../../plans/decisions/035-three-pipe-observability-rule.md), the
tracing pipe is **debugging context, not source-of-truth** — durable governance
facts go to Kindling and live state goes on the notification envelope.
`traceparent` is the cross-pipe correlation key that joins those pipes.

## 2. Architecture diagram

```text
┌──────────────────────┐    ┌──────────────────────┐    ┌──────────────────────┐
│  anvil-cli           │    │  anvil-intercept     │    │  MCP correlation     │
│  main.rs:985         │    │  main.rs:49          │    │  envelope            │
│  init_tracing(Cli)   │    │  init_tracing(Daemon)│    │  ipc.rs:1769..2015   │
└──────────┬───────────┘    └──────────┬───────────┘    └──────────┬───────────┘
           │                           │                           │
           └─────────────┬─────────────┴─────────────┬─────────────┘
                         ▼                           ▼
                ┌────────────────────────────────────────────┐
                │           anvil-observability              │
                │                                            │
                │   ┌─────────────────┐  ┌─────────────────┐ │
                │   │  lib.rs         │  │  traceparent.rs │ │
                │   │  init_tracing   │  │  TraceContext   │ │
                │   │  BinaryKind     │  │  parse / format │ │
                │   │  EnvFilter      │  │  W3C v00 only   │ │
                │   │  ANVIL_TRACE_   │  └─────────────────┘ │
                │   │  SINK selector  │                      │
                │   └─────────────────┘                      │
                │                                            │
                │   ┌─────────────────────────────────────┐  │
                │   │  redaction.rs  (live, TRACE-003)    │  │
                │   │  SENSITIVE_FIELDS deny-list         │  │
                │   │  RedactingJsonFields /              │  │
                │   │  RedactingJsonEventFormatter        │  │
                │   │  REDACTED  (replacement marker)     │  │
                │   └─────────────────────────────────────┘  │
                └────────────────────────────────────────────┘
                                     │
                              tracing pipe →
                              (redacting JSON formatter;
                               CLI → stderr, daemon → stdout,
                               or ANVIL_TRACE_SINK=file=<path>;
                               filtered by ANVIL_LOG / RUST_LOG)
```

The SHA-256 redaction primitive (`hash_of_path`) cited from intercept-as-built
§13 / §4.4 lives in **`crates/anvil-intercept/src/fanout.rs`**, not in this
crate — see §6.

## 3. Crate layout

Three source files, plus a thin `Cargo.toml`. Workspace-pinned dependency set
(`serde`, `serde_json`, `thiserror`, `tracing`, `tracing-subscriber`); no crypto
dependency, no external observability SDK.

| File                                            | Lines | Role                                                                                                                                                                                                     |
| ----------------------------------------------- | ----- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/anvil-observability/Cargo.toml`         | 21    | Crate manifest. Description: "Cross-cutting tracing baseline: TraceContext, W3C traceparent propagation, subscriber init, redaction, and namespace registry hooks (TRACE-001, ADR-035)"                  |
| `crates/anvil-observability/src/lib.rs`         | 531   | Module entry, `BinaryKind`, `init_tracing`, trace-sink selection (`ANVIL_TRACE_SINK`), `bind_traceparent_to_current_span` / `bind_traceparent_to_span`, re-exports (`TraceContext`, `TraceContextError`) |
| `crates/anvil-observability/src/traceparent.rs` | 333   | W3C Trace Context v00 parser + `TraceContext` value type                                                                                                                                                 |
| `crates/anvil-observability/src/redaction.rs`   | 638   | Live TRACE-003 redaction layer: `SENSITIVE_FIELDS` deny-list, `is_sensitive_field`, `RedactingJsonFields` / `RedactingJsonEventFormatter`, and the `REDACTED` marker constant                            |

`#![forbid(unsafe_code)]` is set at the crate root
(`crates/anvil-observability/src/lib.rs:27`).

## 4. Subscriber initialisation (`init_tracing`)

`init_tracing(BinaryKind)` at
[`crates/anvil-observability/src/lib.rs:135-200`](../../crates/anvil-observability/src/lib.rs)
is the **only** entry point that installs a global `tracing` subscriber for an
Anvil Rust binary. Library crates emit through the global `tracing` macros and
MUST NOT install their own subscriber
(`crates/anvil-observability/src/lib.rs:23-25`).

### Filter precedence

1. `ANVIL_LOG` env var if set
2. `RUST_LOG` env var if set
3. `BinaryKind::default_filter()` — `Cli => "warn"`, `InterceptDaemon => "info"`
   (`crates/anvil-observability/src/lib.rs:69-80`)

### Output format

JSON formatter, ANSI off, `with_target(true)`, `with_level(true)`, with the
TRACE-003 redaction pair installed as the field and event formatters —
`fmt_fields(RedactingJsonFields)`,
`event_format(RedactingJsonEventFormatter::default())`
(`crates/anvil-observability/src/lib.rs:141-147`). The first emitted event on
install is at `anvil_observability:::"tracing subscriber installed"` carrying
`binary` and `filter` fields (`crates/anvil-observability/src/lib.rs:193-198`);
operators rely on it as the boot-completed marker.

**Stream routing (CIB-024):** on the default sink, `BinaryKind::Cli` routes the
layer to **stderr** so stdout stays reserved for `--json` command output; the
daemon keeps the default **stdout**, where its host captures it
(`crates/anvil-observability/src/lib.rs:157-163`, rationale at `lib.rs:151-155`;
regression-pinned by `crates/anvil-cli/tests/policy_eval.rs`).

### Trace sink (`ANVIL_TRACE_SINK`)

The `ANVIL_TRACE_SINK` env var (`crates/anvil-observability/src/lib.rs:45`)
selects where the formatted JSON lands. Unset or empty means the default stream
above (stderr for the CLI, stdout for the daemon).

- `file=<path>` opens an append-mode local trace file
  (`crates/anvil-observability/src/lib.rs:166-180`). On Unix the open path is
  safety-validated: no symlinks, regular files only, mode `0600` (no group/other
  access), and dev/ino comparison before and after open to close the TOCTOU
  window (`crates/anvil-observability/src/lib.rs:202-327`).
- `otlp` is **rejected** — exporter wiring is deferred to the EXPORT module; the
  error message points operators at `file=<path>` for local use
  (`crates/anvil-observability/src/lib.rs:181-185`).
- Any other value is rejected as an unsupported sink.

`InitTracingError` gained a `TraceSink(String)` variant for these rejection
paths (`crates/anvil-observability/src/lib.rs:92-95`).

### Traceparent span binding (TRACE-004)

Two public helpers record a parsed `traceparent` onto span correlation fields
(`trace_id`, `parent_id`, `trace_flags`, declared as `field::Empty`):
`bind_traceparent_to_current_span` (`crates/anvil-observability/src/lib.rs:105`)
and `bind_traceparent_to_span` (`crates/anvil-observability/src/lib.rs:112`).
The latter is consumed by the daemon's JSON-RPC dispatch span at
`crates/anvil-intercept/src/ipc.rs:3283` (TRACE-004 span correlation). They
record correlation fields only — no OpenTelemetry parent relationship; true
parent propagation is owned by the EXPORT module.

### Single-install discipline

`set_global_default` is the sole atomic guard (called per sink branch,
`crates/anvil-observability/src/lib.rs:157-164` and `lib.rs:178-179`); a second
call returns `InitTracingError::AlreadyInstalled`. The doc comment at
`crates/anvil-observability/src/lib.rs:124-126` explains why a separate sentinel
flag is intentionally absent (TOCTOU window between guard check and install).

### Call sites

| Site                                    | Use                                                                |
| --------------------------------------- | ------------------------------------------------------------------ |
| `crates/anvil-cli/src/main.rs:985`      | `init_tracing(BinaryKind::Cli)` once at CLI startup                |
| `crates/anvil-intercept/src/main.rs:49` | `init_tracing(BinaryKind::InterceptDaemon)` once at daemon startup |

No other Rust call sites. The TS API (`anvil-api`) does not consume this crate —
it consumes the shipped TypeScript mirror `@eddacraft/anvil-observability`
instead (see §7).

## 5. Namespace registry (state in this repo)

There is **no Rust-side registry implementation** in this crate. The
authoritative registry document is
[`docs/observability/namespace-registry.md`](../../docs/observability/namespace-registry.md);
this crate contributes nothing to it as code beyond the `TraceContext` parser
and redaction deny-list cited as "validation hooks" at
`docs/observability/namespace-registry.md:100-116`.

### What the registry currently records

Five live namespace rows (`docs/observability/namespace-registry.md:34-40`):

| Namespace           | Owner                                                                                | Pipe(s)                                                | Wired in code?                            |
| ------------------- | ------------------------------------------------------------------------------------ | ------------------------------------------------------ | ----------------------------------------- |
| `anvil.flags.*`     | FLAGS module / [ADR-019](../../plans/decisions/019-flags-observability-alignment.md) | Tracing (per-eval), Kindling (gate-affecting outcomes) | Yes — feature-flagging module is Complete |
| `anvil.cli.*`       | CLI / TRACE-004                                                                      | Tracing                                                | Yes — TRACE-004 Complete (2026-05-11)     |
| `anvil.intercept.*` | Intercept daemon / TRACE-004                                                         | Tracing                                                | Yes — TRACE-004 Complete (2026-05-11)     |
| `kindling.*`        | Kindling system (Edda Stack)                                                         | Kindling                                               | Out-of-tree (Edda Stack project)          |
| `anvil.rtai.*`      | RTAI module (provisional)                                                            | Tracing                                                | Provisional — pending RTAI promotion      |

### Stability contract

The contract is enforced by **founder-reviewed PR**, not by code
(`docs/observability/namespace-registry.md:92-98`). Field-naming rules (lower
snake_case, singular nouns, two-segment hierarchy after the domain, units in the
name) are documented at `docs/observability/namespace-registry.md:62-77` and
reviewed at the PR-to-add gate. Pipe allocation must comply with the ADR-035
three-pipe matrix (`docs/observability/namespace-registry.md:79-90`).

### Known drift (registry vs. code)

The deny-list at `crates/anvil-observability/src/redaction.rs:36-56` is now
consulted **at runtime**: the formatters `init_tracing` installs call
`is_sensitive_field` on every span/event field and substitute `REDACTED` for
matches (`crates/anvil-observability/src/redaction.rs:349-357`). The registry's
"Validation hooks" bullet still describes the deny-list as **advisory-only / NOT
enforced** (`docs/observability/namespace-registry.md:102-109`) — that wording
lags the code. Whether a new attribute name conflicts with the deny-list
semantically (vs. matching it exactly) is still reviewed visually at PR-to-merge
time. See §"Known gaps" G-02 for the residual TRACE-003 slice.

## 6. Redaction primitive — what is and isn't here

This crate ships the **live** TRACE-003 local redaction layer. `init_tracing`
installs `RedactingJsonFields` / `RedactingJsonEventFormatter`
(`crates/anvil-observability/src/lib.rs:146-147`), which replace any span/event
value whose field name matches `SENSITIVE_FIELDS` with the `REDACTED` marker
before the JSON formatter writes it (consumption at
`crates/anvil-observability/src/redaction.rs:349-357`; module doc at
`redaction.rs:1-13`; behaviour pinned by the formatter tests at
`redaction.rs:550-623`). It still does **not** ship the SHA-256 hashing
primitive that fan-out cross-session redaction relies on — `hash_of_path`
remains in `crates/anvil-intercept/src/fanout.rs` (below).

### What `redaction.rs` exposes

| Symbol                                    | Kind         | Purpose                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| ----------------------------------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `REDACTED: &str` (`= "<redacted>"`)       | `pub const`  | Canonical replacement marker the live TRACE-003 layer emits. Pinned by test (`crates/anvil-observability/src/redaction.rs:543-548`); changing the value is a contract break across binary boundaries.                                                                                                                                                                                                                                                                                                        |
| `SENSITIVE_FIELDS: &[&str]`               | `pub const`  | 19-entry lower-case deny-list of sensitive field names (`api_key`, `apikey`, `access_key`, `auth`, `authorization`, `bearer`, `client_secret`, `context`, `credential`, `credentials`, `notification.context`, `notification_context`, `password`, `passwd`, `pwd`, `private_key`, `secret`, `session_token`, `token`; `redaction.rs:36-56`). Sourced from the OWASP secret-name patterns plus the deny-list the INTD-013 reviewers flagged on `notification.context` (source comment `redaction.rs:33-35`). |
| `is_sensitive_field(field: &str) -> bool` | `pub fn`     | Case-insensitive **exact-match** lookup. Substrings deliberately do not match (`token_type` is allowed). Consulted at runtime by both formatters via the shared visitor (`redaction.rs:349-357`).                                                                                                                                                                                                                                                                                                            |
| `RedactingJsonFields`                     | `pub struct` | JSON `FormatFields` implementation that redacts sensitive span field values before subscriber output is formatted (`redaction.rs:215-216`).                                                                                                                                                                                                                                                                                                                                                                  |
| `RedactingJsonEventFormatter`             | `pub struct` | JSON event formatter paired with `RedactingJsonFields` — the stock JSON event formatter bypasses the configured `FormatFields`, so this routes event fields through the same redaction visitor (`redaction.rs:224-227`).                                                                                                                                                                                                                                                                                     |

### What the SHA-256 redaction primitive actually is — and where it lives

The H2 security-note primitive cited from intercept-as-built §13, §4.4 and the
redaction filter site is
[`hash_of_path`](../../crates/anvil-intercept/src/fanout.rs) at
`crates/anvil-intercept/src/fanout.rs:436-441` — `Sha256(input.as_bytes())`
hex-encoded, prefixed `[redacted:<hex>]`. It is **not exported from
anvil-observability**; it is a private helper inside the intercept fanout
module. The `[redacted]` literal at `crates/anvil-intercept/src/fanout.rs:455`
(`REDACTED_MARKER`) is locally defined and does not import the
`anvil-observability` `REDACTED` constant.

### MCP-side redaction (`validate_write`)

The §4.4 redaction filter site at
`crates/anvil-cli/src/mcp/tools/validate_write.rs:374-424` uses
`redact_secret_values` (regex-driven default-pattern masking) and a separate
SHA-256-prefixed id at lines `403-413`. Neither path imports from
`anvil-observability`.

### Determinism and the unsalted-hash trade-off

`hash_of_path` is unsalted across daemon lifetimes — same input, same hash,
forever. Subscribers can dedupe across runs on the redacted form. The
operational cost is that a same-UID subscriber with a candidate corpus (common
paths, `.env.local`, repo tree) can rainbow-table the hash back to plaintext.
This is an accepted v1 trade-off; details, operator guidance, and the
per-startup HMAC fix tracked for the next tag are in
[`docs/archive/runbooks/v0.6.0-beta-security-note.md`](../../docs/archive/runbooks/v0.6.0-beta-security-note.md)
**H2**.

## 7. Traceparent helper (`traceparent.rs`)

W3C Trace Context spec, version `00` only. Pure, no I/O, no allocations beyond
the canonicalised hex strings stored in `TraceContext`.

### Format pinned

```text
00-<32 lower-hex trace-id>-<16 lower-hex parent-id>-<2 lower-hex flags>
```

Total length is exactly 55 bytes (`TRACEPARENT_LEN`,
`crates/anvil-observability/src/traceparent.rs:26-27`). Constants for each
segment length live at `crates/anvil-observability/src/traceparent.rs:20-23`.

### Public surface

| Symbol                                                                                                                                                    | Site                     |
| --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| `TRACEPARENT_LEN: usize = 55`                                                                                                                             | `traceparent.rs:26-27`   |
| `TraceContext` (struct, no `Serialize` / `Deserialize` derives — see comment at `traceparent.rs:38-44`)                                                   | `traceparent.rs:46-50`   |
| `TraceContext::parse`, `::trace_id`, `::parent_id`, `::flags`, `::is_sampled`, `::as_header`                                                              | `traceparent.rs:84-179`  |
| `TraceContext: Display` and `TraceContext: FromStr`                                                                                                       | `traceparent.rs:181-193` |
| `TraceContextError` (eight variants: `Length`, `Shape`, `NotHex { field }`, `ReservedVersion`, `UnsupportedVersion`, `AllZeroTraceId`, `AllZeroParentId`) | `traceparent.rs:53-82`   |

### Strictness

Parsing rejects: upper-case hex, wrong total length, the reserved `ff` version
byte, any version other than `00`, the all-zero trace-id form, the all-zero
parent-id form, and any non-ASCII input
(`crates/anvil-observability/src/traceparent.rs:94-148`). The non-ASCII guard is
a separate up-front check (`traceparent.rs:99-101`) — without it, multi-byte
UTF-8 falling outside the hex range would otherwise leak through length
validation; the test at `traceparent.rs:305-319` pins this case at the exact
55-byte length where the length check alone would not catch it.

`UnsupportedVersion` deliberately does not echo the rejected version bytes back
into the error message (`traceparent.rs:69-75`) — every error path is reflected
into log streams, and the rejected bytes have no diagnostic value.

### Consumers

| Site                                          | Use                                                                                                                                                                       |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/anvil-intercept/src/ipc.rs:40`        | `use anvil_observability::TraceContext;` — JSON-RPC envelope                                                                                                              |
| `crates/anvil-intercept/src/ipc.rs:1306-1340` | scan_buffer batch traceparent extraction; uses `TRACEPARENT_LEN` to bound input length                                                                                    |
| `crates/anvil-intercept/src/ipc.rs:1769-2015` | The full envelope handler — `extract_traceparent` then echoes the validated header on every response shape (`jsonrpc_success`, `jsonrpc_error`, scan-buffer batch errors) |

The TS-side mirror **shipped** as `@eddacraft/anvil-observability`
(`packages/anvil/observability/src/traceparent.ts`, 210 lines), and `anvil-api`
validates `traceparent` at ingress via the `traceContext` middleware
(`apps/anvil-api/src/middleware/trace-context.ts:16`, mounted at
`apps/anvil-api/src/index.ts:86`). The residual TRACE-002 slice — the dashboard
live-feed consumer — is still missing, so cross-binary trace joining via the
dashboard is not yet possible; TRACE-002 stays Blocked
(`plans/modules/tracing-foundation.aps.md:256`).

## 8. Cross-cutting concerns

### No PII in span keys / namespaces

The namespace-registry stability contract requires lower snake_case, singular
nouns, dotted hierarchy with the domain immediately after `anvil.`, and units in
the name when ambiguous (`docs/observability/namespace-registry.md:62-77`). The
contract is reviewed by founder PR, not enforced by code.

### Determinism

`TraceContext::parse → TraceContext::as_header` is byte-for-byte identical for
every valid input — pinned by
`crates/anvil-observability/src/traceparent.rs:222-225`. The live TRACE-003
redaction marker `REDACTED = "<redacted>"` is a constant string with a
contract-pinning unit test at `redaction.rs:543-548`. (Note again that the
unsalted-SHA-256 cross-session hash is **not** in this crate — it lives in
`anvil-intercept/src/fanout.rs:436-441`.)

### Zero non-workspace dependencies

Everything in `Cargo.toml` is `workspace = true` (serde, serde_json, thiserror,
tracing, tracing-subscriber, workspace-hack;
`crates/anvil-observability/Cargo.toml:12-18`). No crypto crate, no OTLP SDK, no
HTTP client. The kernel-style "zero deps where feasible" stance holds.

### `#![forbid(unsafe_code)]`

Set at the crate root (`crates/anvil-observability/src/lib.rs:27`); also
inherited via `[lints] workspace = true`
(`crates/anvil-observability/Cargo.toml:20-21`).

### ADR pins

- [ADR-019](../../plans/decisions/019-flags-observability-alignment.md) —
  feature-flag telemetry alignment; the `anvil.flags.*` precedent the registry
  generalises.
- [ADR-034](../../plans/decisions/034-cross-cutting-modules-as-aps-primitive.md)
  — promotes the cross-cutting module convention; TRACE is the second trial.
- [ADR-035](../../plans/decisions/035-three-pipe-observability-rule.md) —
  tracing pipe is debugging context, not source-of-truth.

## 9. Known gaps

G-02 and G-03 re-assessed 2026-06-10 against main `45dd1047a`; G-01, G-04, and
G-05 dated against `v0.6.0-beta` (HEAD `d223b8d9`). Each entry has the tracking
work item that closes it.

### G-01: Cross-session telemetry redaction hash is unsalted SHA-256

`hash_of_path` at `crates/anvil-intercept/src/fanout.rs:436-441` (the primitive
on which `Delivery::Redact` rests) is unsalted. A same-UID subscriber can
rainbow-table a known corpus to recover plaintext paths. The documented operator
posture is to leave `telemetry.allow_cross_session = false`
(`crates/anvil-intercept/src/fanout.rs:7-11`).

**Risk:** Medium for operators who enable cross-session telemetry on a
multi-tenant developer machine; Low otherwise (default-off). **Fix:**
Per-startup HMAC salt minted on daemon launch, rotated every cold start. Tracked
under TRACE-003 alongside the subscriber-side redaction layer. Full
operator-facing detail in
[`docs/archive/runbooks/v0.6.0-beta-security-note.md`](../../docs/archive/runbooks/v0.6.0-beta-security-note.md)
**H2** (the primary cross-link for this gap; do not duplicate the writeup here).

### G-02: TRACE-003 cross-binary / EXPORT-policy-parity slice outstanding

The local-output half of TRACE-003 is **done**: `init_tracing` installs
`RedactingJsonFields` / `RedactingJsonEventFormatter`
(`crates/anvil-observability/src/lib.rs:146-147`), so span attributes named
`password` / `token` / `api_key` are replaced with `REDACTED` before JSON output
on every supported sink (consumption at
`crates/anvil-observability/src/redaction.rs:349-357`; pinned by the formatter
tests at `redaction.rs:550-623`). What remains is the cross-binary /
EXPORT-policy-parity slice — redaction hardening across binary boundaries so the
same deny-list semantics hold for exported spans and the notification pipe. That
slice is **Blocked** on INTD-015 (`plans/modules/tracing-foundation.aps.md`,
TRACE-003).

**Risk:** Low while the daemon is same-UID local-IPC only and output stays on
local sinks. Becomes Medium when spans are exported off-host (EXPORT module).
**Fix:** the remaining TRACE-003 slice, unblocked by INTD-015.

### G-03: Dashboard live-feed trace consumer missing

The TS-side `traceparent` parser **shipped** as `@eddacraft/anvil-observability`
(`packages/anvil/observability/src/traceparent.ts`, 210 lines), and `anvil-api`
validates `traceparent` at ingress
(`apps/anvil-api/src/middleware/trace-context.ts:16`, mounted at
`apps/anvil-api/src/index.ts:86`). The residual gap is the dashboard live-feed
consumer slice: no dashboard surface joins traces across producers yet, so
TRACE-002 stays **Blocked** on concrete consumer ownership
(`plans/modules/tracing-foundation.aps.md:256`).

**Risk:** Low — operational, not security. **Fix:** the remaining TRACE-002
slice — a dashboard joined-view consumer.

### G-04: No registry-side validation hook in code

The `anvil.<domain>.*` namespace contract is reviewed by founder PR, not by a
build-time check. A producer can emit `anvil.totally.new.namespace.*` in code
and the binary will install and ship; the drift is caught (or not) at PR review.

**Risk:** Low at current scale (five namespaces, founder-reviewed). **Fix:** Out
of TRACE-001 scope. Tracked indirectly under TRACE R2 in `plans/index.aps.md`
(namespace fragmentation risk).

### G-05: `anvil-observability` `REDACTED` constant not adopted by ad-hoc redactors

`crates/anvil-intercept/src/fanout.rs:455` defines
`REDACTED_MARKER = "[redacted]"` locally;
`crates/anvil-cli/src/mcp/tools/validate_write.rs:421` and
`crates/anvil-checks/src/secret/patterns.rs:265` use the literal `"[REDACTED]"`
(upper-case). The `REDACTED` constant in this crate is `"<redacted>"`. Three
redaction marker shapes coexist; none of the ad-hoc sites import the
observability constant.

**Risk:** Low. Cosmetic until the markers are parsed downstream. **Fix:** Folded
into TRACE-003 / RMCPF-010 — when the unified redaction layer lands, callers
converge on the observability `REDACTED` constant.

## 10. Source references

- [`crates/anvil-observability/Cargo.toml`](../../crates/anvil-observability/Cargo.toml)
  — manifest; workspace-pinned dependency set, `forbid_unsafe_code` via
  workspace lints.
- [`crates/anvil-observability/src/lib.rs`](../../crates/anvil-observability/src/lib.rs)
  — `BinaryKind`, `init_tracing`, the single subscriber-install path, the
  `ANVIL_TRACE_SINK` sink selection and file-safety validation, the traceparent
  span-binding helpers, and the module-level ADR-035 framing comment (lines
  1-25).
- [`crates/anvil-observability/src/traceparent.rs`](../../crates/anvil-observability/src/traceparent.rs)
  — W3C v00 `traceparent` parser, the `TraceContext` value type, the
  `TraceContextError` taxonomy.
- [`crates/anvil-observability/src/redaction.rs`](../../crates/anvil-observability/src/redaction.rs)
  — live TRACE-003 redaction layer: `SENSITIVE_FIELDS` deny-list, `REDACTED`
  marker, the `is_sensitive_field` exact-match helper, and the
  `RedactingJsonFields` / `RedactingJsonEventFormatter` formatter pair.

## 11. Related docs

- [`docs/observability/namespace-registry.md`](../../docs/observability/namespace-registry.md)
  — namespace registry conceptual home (the as-built side is §5 here).
- [`docs/architecture/intercept-as-built.md`](./intercept-as-built.md) §11 / §13
  — telemetry fanout and the §4.4 redaction filter; primary consumer for
  `TraceContext` and the (out-of-crate) SHA-256 redaction primitive.
- [`docs/architecture/mcp-shim-as-built.md`](./mcp-shim-as-built.md) — the
  `validate_write` correlation envelope round-trips traceparent via the daemon
  envelope; the §4.4 redaction filter site is in `validate_write.rs`.
- [`docs/runbooks/observability-triage.md`](../../docs/runbooks/observability-triage.md)
  — operator runbook (OBS-005-owned; cross-link only).
- [`docs/archive/runbooks/v0.6.0-beta-security-note.md`](../../docs/archive/runbooks/v0.6.0-beta-security-note.md)
  **H2** — operator-facing detail on the unsalted SHA-256 redaction-hash
  trade-off (G-01 above).
- [`plans/modules/tracing-foundation.aps.md`](../../plans/modules/tracing-foundation.aps.md)
  — TRACE module plan (In Progress 2/4 — TRACE-001 Complete, TRACE-004 Complete
  2026-05-11; TRACE-002 / TRACE-003 Blocked after partial implementation).
- [`plans/modules/observability-foundation.aps.md`](../../plans/modules/observability-foundation.aps.md)
  — OBS module plan (Draft; OBS-006 superseded by TRACE-001 on 2026-04-30).
- [ADR-035](../../plans/decisions/035-three-pipe-observability-rule.md) —
  three-pipe rule (Kindling / notification / tracing).
- [ADR-034](../../plans/decisions/034-cross-cutting-modules-as-aps-primitive.md)
  — cross-cutting module primitive.
- [ADR-019](../../plans/decisions/019-flags-observability-alignment.md) —
  feature-flag observability alignment; the `anvil.flags.*` precedent.
