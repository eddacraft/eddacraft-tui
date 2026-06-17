# Save-time Dependency Advisory Pack — Product Thesis

| Field       | Value                                                                 |
| ----------- | --------------------------------------------------------------------- |
| Type        | Product thesis (pre-APS shaping)                                       |
| Status      | Draft — non-binding                                                    |
| Date        | 2026-06-17                                                            |
| Source      | anvil opportunity assessment of the GitHub Advisory Database          |
| Disposition | Product Note → APS intake once the open decisions below are resolved   |

> This is a **thesis**, not an APS module, ADR, or implementation spec. It exists
> to frame a candidate capability and the decisions that gate it. It does not
> change `plans/index.aps.md` and creates no work items. Promote to an APS module
> only after the [open decisions](#open-decisions) are settled (an ADR is
> required for the feed + pack-versioning choice).

## Summary

anvil already markets **"Dependency Auditing — validates dependencies against
known vulnerability databases before they are added to your project"**
(`apps/website/app/security/page.tsx:76`). The actual backing was a deliberate
beta shortcut: *"a known-vulnerable list (shipped as a JSON fixture, updated per
release)... **Not a full advisory database — a curated blocklist for beta**"*
(`docs/archive/specs/2026-03-27-rust-cli-cutover.md`). The GitHub Advisory
Database — OSV format, 12 ecosystems, CC-BY-4.0 — is the authoritative feed that
makes that claim honest.

The opportunity is **not** a new product line and **not** an SCA platform. It is
to upgrade the data backing of an existing, in-scope check from a hand-curated
fixture to a **versioned, signed advisory pack**, matched at save time and scoped
by anvil's existing **new-edges-only** baseline so it warns only when an AI
introduces a *newly* vulnerable dependency. The freshness mechanism — check the
pinned pack against the published version on `start`, notify, let the user run an
update command — rides rails anvil already ships for the CLI binary.

## The gap (why now)

- **Claim is live.** The security page promises validation against "known
  vulnerability databases" today.
- **Backing is thin.** The shipped contract is a per-release JSON blocklist,
  self-described as "not a full advisory database". The earlier (archived) TS
  scanner shelled out to `npm audit`
  (`archive/anvil-ts-scanner/runtime-gate/checks/dependency.check.ts`); the Rust
  `gate` dependency check was stubbed pending this backing.
- **Promise-vs-reality.** Independent of whether we build the pack, the security
  page wording currently outruns the implementation and is worth a truthfulness
  pass.

## Thesis

> anvil intercepts AI-introduced **known-vulnerable dependencies at save time** —
> before they reach a PR — using a deterministic, signed advisory pack and the
> same new-edges-only discipline that keeps its architecture warnings quiet.

The differentiator is **timing and scoping**, not detection. Dependabot, Snyk,
osv-scanner and GitHub itself all detect vulnerable dependencies *after* commit.
anvil catches the moment the dependency is written into the manifest, and only
flags the *new* edge — which is exactly the "too late" critique in
`docs/vision/anvil-vision.md` and the noise-control posture in ADR-003
(new-edges-only) / ADR-039 (baseline policy).

## Scope

**In scope**

- Match dependency coordinates (from `package-lock.json`, `Cargo.lock`, etc.)
  against a local advisory pack at save/gate time.
- Warn-first; human-owned suppression for accepted advisories (ADR-004).
- New-edges-only: baseline existing (possibly vulnerable) deps; warn on newly
  introduced vulnerable edges.
- A signed, version-pinned pack delivered through the existing update rails.

**Out of scope (the guardrail)**

- A full SCA / SBOM / licence-compliance product.
- Continuous post-merge monitoring, dashboards, or a vulnerability "platform"
  (out of scope per `docs/vision/anvil-scope-guard.md` — not an observability
  platform, not a Dependabot replacement).
- Live advisory-API calls on the save-time hot path (breaks determinism and the
  air-gapped guarantee).

## How it reuses what we already ship

The freshness/notify/update mechanism is **already built for the CLI binary**
(DISTRIB-001/-002, ADR-045). The advisory pack becomes a *second artefact* on the
same rails — new payload, same transport. No new notification, fetch, or signing
machinery.

> **Two distinct "advisory" meanings — do not conflate.** Today
> `anvil version --check` parses `Security-Advisory: GHSA-…` lines from anvil's
> own *release body* (`commands/version.rs::parse_advisory_tags`) — "is *anvil
> itself* affected; should you upgrade anvil." This thesis is about advisories
> affecting the *user's dependencies*. We reuse the transport/notify/update layer,
> **not** the release-note-parsing semantics; dependency advisories come from a
> structured OSV pack, never hand-written release notes.

| Need                                   | Existing primitive to reuse                                                                                          | Reference                                                                 |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Best-effort "is a newer X available?"  | `fetch_latest_version_quiet()` — 3s timeout, silent on failure, `--offline` honoured                                | `crates/anvil-cli/src/commands/version.rs`                                |
| "Couldn't check" ≠ "you're safe"       | `AdvisoryProbe` tri-state (`NotProbed` / `Unavailable` / `Probed`)                                                   | `commands/version.rs`                                                      |
| Non-nagging hint in `status` + `watch` | `update_hint.rs` — shared 24h rate-limit gate; **a fresh version bypasses the timer**; `ANVIL_DISABLE_UPDATE_HINT`   | `crates/anvil-cli/src/update_hint.rs`, `compute_update_hint()`            |
| Pull + verify a signed artefact        | `fetch_and_verify()` — minisign (Ed25519/BLAKE2b), embedded public key, HTTPS-only, 4 MiB cap, air-gapped verify     | `crates/anvil-cli/src/commands/update/fetch.rs`, ADR-045                  |
| Consent, never auto-mutate             | `anvil update` is opt-in by design ("auto-update without consent — explicit non-goal")                              | `plans/archive/modules/distribution-and-update.aps.md`                    |
| Pack as a delivered, versioned unit    | Pack architecture                                                                                                    | ADR-027                                                                   |
| Vulnerable-edge scoping + suppression  | New-edges baseline (ADR-003/-039); suppression syntax (ADR-004); GV2 dependency-edge graph + evidence/witness        | `plans/decisions/`, graph-v2-foundation                                   |
| Findings interchange                   | Shared SARIF emitter                                                                                                 | ADR-058, `plans/specs/2026-05-29-sarif-output-design.md`                  |

### Freshness model (the mechanism we discussed)

1. **Enforcement** matches deps against the **pinned local pack** — deterministic,
   offline, hot-path-safe. No network at save time.
2. **`anvil start` / `status` / `watch`** run a best-effort check of the local
   pack version against the published pack manifest — modelled on
   `fetch_latest_version_quiet` (3s, silent fail, `--offline` and
   `ANVIL_DISABLE_UPDATE_HINT` honoured; air-gapped guarantee preserved).
3. **Notify** via the shared `UpdateHint` surface in `status`/`watch`,
   rate-limited (24h; a newer pack bypasses the timer — security data should not
   wait a day).
4. **User runs `anvil update`** → the pack is pulled and **minisign-verified**
   through `fetch_and_verify` before it replaces the pinned pack.

This keeps the version *check* networked but advisory and out-of-band, while
enforcement stays deterministic against whatever pack is currently pinned —
faithful to the local-first / no-telemetry claims on the security page (pull a
signed manifest; push nothing about the user's code).

## Open decisions

These gate APS promotion. The first two warrant an ADR.

1. **Pack versioning: independent vs coupled.** Ship the pack *with* each CLI
   release (simplest; refreshes only on `anvil update` of the binary) or as an
   *independently* versioned artefact with its own manifest and cadence.
   **Recommendation: independent** — vulnerability data must outpace the CLI
   release train; coupled is an acceptable v0 if we decouple later.
   `github_release_source(owner, repo, app_name, version)` already parameterises
   by asset, so a separate `advisory-pack` stream is a natural extension.
2. **Feed: OSV ecosystem vs this repo directly.** Prefer consuming via the **OSV
   model** (OSV schema is Apache-2.0; osv.dev / osv-scanner exports) so the feed
   stays swappable and we can blend sources, rather than hard-coupling to
   `github/advisory-database`. Capture the *format*, treat any single feed as
   replaceable.
3. **Licensing / attribution.** GitHub-reviewed advisories are **CC-BY-4.0**
   (NVD-sourced records are US public domain; OSV schema Apache-2.0). Shipping a
   pack triggers the CC-BY **attribution** obligation. Route through the same
   legal gate used for ADR-055 (the APS-dashboard OSS carve-out).
4. **Matching correctness.** Per-ecosystem version-range / semver matching is
   fiddly; false positives breach the vision's "<10% of warnings suppressed
   without resolution" signal-quality bar. Reachability/dev-only-dep handling is a
   known noise source.
5. **Wedge.** Start with **one ecosystem** (npm or crates.io), warn-only,
   new-edges-only — a drop-in replacement for the beta JSON fixture.
6. **Command surface.** Have `anvil update` refresh the pack alongside the binary
   (one consent surface), plus a dedicated pack-only refresh (e.g.
   `anvil update --advisories` or `anvil advisories update`) for fast security
   refreshes without a full upgrade. Non-binding; settle with #1.

## Customer framing & differentiation

- **Buyer language:** "anvil stops your AI from adding known-vulnerable
  dependencies, the moment it writes them." Instantly legible to a security buyer
  and serves the currently under-delivered *vulnerabilities* half of the vision
  (today anvil leads with architecture drift, anti-patterns, planted secrets).
- **Differentiation rests entirely on timing + new-edges scoping.** If that
  framing is dropped, the capability collapses into "another SCA scanner" that
  competes with free tools. Hold the framing.

## Risks

- **Scope drift** into an SCA/SBOM platform — the central risk; bound hard to
  save-time + new-edges + AI-introduced.
- **Commoditisation** — detection is free elsewhere; only timing/scoping is ours.
- **Maintenance treadmill** — the feed changes daily; we own a snapshot / prune /
  sign / publish pipeline.
- **Freshness vs determinism** — resolved by the pinned-pack + signed-update
  cadence above, but it is a real ops surface.
- **False-positive noise** — directly threatens adoption and the signal-quality
  success criterion.
- **Licensing** — CC-BY attribution must be carried; needs sign-off.

## Recommended next step

1. ADR for **feed choice + pack versioning** (decisions #1–#2).
2. Legal check on **CC-BY-4.0 attribution** (decision #3), per the ADR-055 path.
3. On acceptance, open an APS module for a **single-ecosystem, warn-only,
   new-edges-only** wedge that replaces the beta JSON fixture and rides the
   existing update rails.
4. Independently: a truthfulness pass on the security-page "Dependency Auditing"
   wording to match current reality until the pack ships.

## References

**Code**

- `apps/website/app/security/page.tsx` — the live "Dependency Auditing" claim.
- `crates/anvil-cli/src/commands/version.rs` — latest-probe, `AdvisoryProbe`,
  `parse_advisory_tags`, `compute_update_hint`.
- `crates/anvil-cli/src/update_hint.rs` — shared 24h rate-limit gate.
- `crates/anvil-cli/src/commands/update/fetch.rs` + `update/signature.rs` —
  signed-artefact fetch + minisign verify.
- `docs/archive/specs/2026-03-27-rust-cli-cutover.md` — the beta JSON-fixture
  shortcut.

**Decisions / modules**

- ADR-045 (update signing — minisign), ADR-027 (pack architecture), ADR-003 /
  ADR-039 (new-edges-only / baseline), ADR-004 (suppression), ADR-058 (SARIF),
  ADR-055 (OSS carve-out / legal gate precedent).
- `plans/archive/modules/distribution-and-update.aps.md` (DISTRIB-001/-002).

**External**

- GitHub Advisory Database — <https://github.com/github/advisory-database> (OSV
  format, 12 ecosystems, CC-BY-4.0).
- OSV schema (Apache-2.0) — <https://ossf.github.io/osv-schema/>; osv.dev.
