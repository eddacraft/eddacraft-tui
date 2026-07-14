# ADR-106: Fleet Telemetry Consent Posture and Dimension Allowlist

## Status

Proposed

## Date

2026-07-15

## Context

Anvil ships **zero remote telemetry** today, by deliberate posture: the usage
pipe (USAGE/KDS) writes `command.invoked` and governance observations to a
local Kindling store that never leaves the machine
(`docs/observability/usage-analytics.md`), tracing is opt-in local sinks with
OTLP deferred to EXPORT, and the INSIGHTS module shipped annotated "No
telemetry". The operator therefore has no visibility into the fleet at all:
not the number of active installs, not the version or install-method
distribution, not which features are used.

On 2026-07-15 the operator resolved FLEET OQ3: fleet visibility is needed as
**evidence for investors** — credible active-install counts, retention
cohorts, version/install-method distribution, and feature adoption. Download
counts and GitHub stars cannot show retention or adoption; the local-only
Kindling store cannot be seen. This ADR is the design gate for the
[fleet-telemetry](../modules/fleet-telemetry.aps.md) (FLEET) module and
decides the consent posture and the exact data allowed on the wire.

A dimension contract already exists on paper: the feature-flagging design
(`plans/specs/2026-04-09-feature-flagging-design.md`, restated in
`docs/guides/feature-flag-governance.md`) specified session-start snapshot
telemetry plus one usage stat per feature actually used — no PII, only
low-risk dimensions (feature key, environment, runtime, snapshot version,
coarse tier/channel). Only its local half (USAGE-002 inline `flag_set`) was
ever built; this ADR adopts its remote half.

## Decision

Adopt a **disclosed opt-out telemetry beacon** for beta+ builds, with a
hard-enumerated dimension allowlist and full user transparency:

1. **Consent posture: disclosed opt-out.** The first-run surface
   (activation/welcome) shows a notice naming exactly what is sent and the
   one-line off switch *before the first beacon fires*; the docs site
   carries the same list. Hard offs, all permanent and honoured before any
   send: `anvil telemetry off` (persisted setting), `ANVIL_TELEMETRY=off`,
   and `DO_NOT_TRACK=1` (which already disables local collection and stays
   a superset off). Gated/CI environments (`ANVIL_HOME` re-root, non-TTY
   first runs that never showed the notice) do not beacon.
2. **Identity: anonymous random install id.** A UUID v4 minted on first
   beacon and stored beside the salt in the credentials dir — derived from
   nothing (no hardware, no user identity, no salted principal on the
   wire). `anvil telemetry reset-id` rotates it, which is deletion from the
   operator's ability to correlate. This is the minimum identity that
   yields unique-install and retention counts.
3. **Payload: enumerated allowlist, nothing else.** `schema_version`,
   `install_id`, anvil `version`, `install_method` (LAUNCH-013 detection),
   platform triple, release channel, flag snapshot version, and feature
   usage as `(feature key, count)` pairs since the last beacon per the
   FLAGS-design contract. Explicitly forbidden: paths, repo names,
   arguments, hostnames, emails, free-form strings; ingest does not retain
   IPs; timestamps coarsen to date. Adding any dimension requires a dated
   amendment to this ADR.
4. **Emission: best-effort, never in the way.** At most one beacon per
   install per 24h, fired from the session-start path asynchronously with
   a short timeout; failure is silent and unspooled (losing a beacon is
   acceptable; queuing one is not worth the machinery). The beacon must
   add zero blocking latency to any command (ADR-031 discipline applies).
5. **Transparency surface.** `anvil telemetry` prints the current on/off
   state and the exact payload the next beacon would send, so the
   allowlist is auditable from the binary itself, not just the docs.
6. **Ingest and retention.** A versioned `apps/anvil-api` route; raw rows
   retained 90 days, aggregates kept indefinitely; operator-only access.
7. **No covert channels.** The beacon is the *only* remote usage emission.
   Licence-gate and auth calls must not carry usage dimensions; the local
   Kindling pipe remains local (this ADR adds an aggregate derived from
   usage facts, not a new local pipe — ADR-035 unchanged).

## Rationale

Investor-grade proof requires counts that are honest about coverage.
Opt-in telemetry in developer tools converges on single-digit-percent,
self-selected samples — unusable for install counts or retention curves and
easy to challenge in diligence. Disclosed opt-out with a genuine,
discoverable off switch is the established dev-tool norm (Next.js, Homebrew,
.NET SDK, Astro) and keeps trust intact **iff** the disclosure is honest,
the payload is enumerable, and `DO_NOT_TRACK` is absolute. The random
install id gives uniques and retention without touching identity; reusing
the salted principal was rejected because fleet telemetry needs no link to
the local identity convention at all.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Disclosed opt-out beacon (chosen) | Unbiased counts; retention cohorts; industry-standard; DNT + one-line off preserves posture | A real posture change; demands first-class disclosure UX and docs |
| Opt-in telemetry | Cleanest consent story | Single-digit biased sample; cannot prove installs or retention; weak as investor evidence |
| Stay at zero telemetry (downloads, GitHub stats, surveys) | No posture change | No retention/adoption signal at all; download counts inflate; nothing to defend in diligence |
| Piggyback on licence-gate/auth calls | No new endpoint | Covert channel; poisons the auth trust story; couples consent to entitlement; rejected outright |
| Third-party analytics SDK (PostHog/Amplitude) | Dashboards for free | Ships someone else's collection surface in a trust-sensitive binary; data residency/DPA burden; overkill for one beacon |

## Consequences

- **Positive:** Active-install (DAU/WAU/MAU), retention cohorts, version and
  install-method distribution, and feature adoption become measurable —
  the investor evidence that motivated FLEET, and the same signal the
  ARCHCFG-015-style "usage gate" deferrals have been missing. Upgrade
  targeting ("who is stuck on an old version / brew install") becomes
  possible.
- **Negative:** The "nothing ever leaves the machine" line in the privacy
  contract must be rewritten honestly, and every surface that repeated it
  (docs, welcome copy) updated in the same release that ships the beacon.
  `apps/anvil-api` gains an ingest route to operate and pay for.
- **Risks:** Backlash if the disclosure ships sloppy, late, or after the
  first send; scope creep of the allowlist over time.
- **Mitigations:** Notice strictly precedes the first beacon; `anvil
  telemetry` makes the payload self-auditing; allowlist changes require a
  dated ADR amendment; `DO_NOT_TRACK` remains an unconditional off.

## References

- Related ADRs: ADR-035 (three-pipe rule — unchanged), ADR-031 (latency
  gate), ADR-018 (product tiers), ADR-066 (anvil-api brokering precedent)
- APS modules: [fleet-telemetry](../modules/fleet-telemetry.aps.md) (FLEET
  — this is its design gate), CIB-197 (local envelope enrichment; the
  beacon reuses its `version`/`install_method` fields), USAGE (archived —
  privacy contract), EXPORT (tracing pipe, unaffected)
- External: `plans/specs/2026-04-09-feature-flagging-design.md`
  (Observability section), `docs/guides/feature-flag-governance.md`,
  <https://consoledonottrack.com/>
