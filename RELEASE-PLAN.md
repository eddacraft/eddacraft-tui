# Anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ------------ | --------- | ----------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | Last reviewed 2026-05-28 (fact-refresh: latest tag bumped to `v0.7.2-beta`; daemon-working window shipped as `v0.7.0-beta` 2026-05-21 with patches `v0.7.1-beta` and `v0.7.2-beta`; Current State + Next Release Window headings updated, strategic reframe of next window pending). Prior 2026-05-19 (final release-plan/index sweep: ADOPT reconciled to 6/6 Merged, MLP2 reconciled to 60/76 after MLP2-068, WOUT added to release freight, Draft/Blocked items explicitly deferred). 2026-05-18 closed N4 docs lanes and added CIB-005 + CIB-007 as sit-on freight after the abandoned v0.6.4-beta hotfix attempt; base `v0.6.3-beta` + APS modules |

| Upstream                                                                                                                          | Downstream                                                        |
| --------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| [`plans/index.aps.md`](./plans/index.aps.md), `git tag v0.7.2-beta`, [`ROADMAP.md`](./ROADMAP.md), MLP/INTL/WATCHUX/RMCPF modules | Release runbooks, PR planning, [`ROADMAP.md`](./ROADMAP.md) links |

**Last updated:** 2026-05-31 (added the "Save-Time CPU & Daemon Arc" tracking
section after the `v0.7.3-beta` window — registers the beta-tester high-CPU
remediation (GH #2156) as a two-tier arc: Tier 1 → **`v0.7.4-beta`** (the CPU
bugfix, no council; `RLB-001`/`RLB-007` flipped to Ready; `DISTRIB-006`
`ANVIL_HOME` override noted as co-freight since it is internal + default-inert),
Tier 2 the daemon/Graph V2 save-time pivot (ADR-061) → **`v0.8.0-beta`** as a
deferred minor gated on a planning council. Records the version-override framing
(assess.sh recommends a minor; operator overrides to a patch under the six-week
hold).)

Previous update: 2026-05-30 (added the `v0.7.3-beta` "Surfacing the Signal"
next-release window — a product-surface patch on the sit-on slate whose scope is
already assembled on `main`; resolves the "strategic reframe of what's next
pending" note from the 2026-05-28 refresh.)

Previous update: 2026-05-28 (fact-refresh — latest tag bumped from `v0.6.3-beta`
to `v0.7.2-beta`; the daemon-working window below shipped as `v0.7.0-beta` on
2026-05-21 with patches `v0.7.1-beta` (2026-05-22) and `v0.7.2-beta`
(2026-05-25); Current State + Next Release Window headings updated, strategic
reframe of "what's next" pending.)

Previous update: 2026-05-19 (MLP2-025 bullet narrowed to "on Linux" after the
DeepSec #1671 triage exposed a production wire-up gap closed by PR #1717 —
cross-platform spoof-detection now explicitly rides on MLP2-027/-028 per the
existing deferred entry. Earlier same-day sweep: final release-plan/index
reconciliation — ADOPT to 6/6 Merged via PR #1700, MLP2 to 60/76 after MLP2-068,
WOUT to the freight bundle, and explicit deferral of remaining Draft/Blocked
candidate-intent items not needed for the daemon-working tag-time claim.)

Previous update: 2026-05-18 (docs-phase reconciliation records **N4 closed
6/6**: `docs/runbooks/anvil-air-gapped.md`,
`docs/runbooks/anvil-hook-coexistence.md`,
`docs/runbooks/anvil-witness-chain.md`, `docs/runbooks/anvil-adoption.md`,
`docs/archive/runbooks/v0.6.x-to-v0.7.0-beta-migration.md`, and
`docs/runbooks/anvil-run.md` are all live; the doc-lane release-gate evidence
for `v0.7.0-beta` is complete. Third-pass freight: CIB-005 + CIB-007 joined the
sit-on bundle after an abandoned `v0.6.4-beta` hotfix attempt against
`v0.6.3-beta`; tracking issue #1694 closed. Prior structural amendment
2026-05-17 — two-level release claim — tag-time **daemon-working** verifiable
from code state alone, post-tag **sit-on** graduates via release-notes /
web-copy update after Wave 5 Boring Week confirmation. Wave 5 relocated from
pre-tag hard gate to post-tag validation gate, resolving the chicken-and-egg of
testers running unreleased branch builds. Cut-line extended with **MLP2-025 +
MLP2-025b + MLP2-025c** (security chain finish), **MLP2-026** (operator-recovery
clear path; contract Accepted via PR #1617), and **ADOPT-003** (AI auto-detect —
sit-on adoption leverage). **MLP2-051 respec landed on `732eef55`** splitting it
into an umbrella + five sub-tasks (-051a..-051e); the cut-line picks up **-051a
(`anvil doctor` typed claim) + -051b (MCP shim typed claim) + -051c (TS
driver-client mirror) + -051e (cross-surface parity test)** — the three Ready
render-surface items plus the parity test that closes the HARD-GATE once they
land. **MLP2-051d** (GH Action check render) remains Blocked on the Marketplace
publishing track (MLP2-042/-043) and stays deferred. Current final-sweep MLP2
counter is 60/76 after MLP2-068 reconciliation.)

**Second-pass freight (2026-05-17):** seven additional items added as release
freight after a fresh-cut review without inherited "previously deferred"
reasoning. None are load-bearing for the daemon-working claim; all are tightly
scoped operator-visible improvements that earned their place on independent
merit:

- **RCLI3-016b** — `anvil mcp install --client <cursor|claude-code>` one-command
  wrapper (Ready since 2026-04-26, pulled forward for the demo runbook).
  Adoption-friction killer.
- **RCLI3-017b** — `anvil intercept unblock --worktree` operator CLI wrapper
  (Ready, INTD-007 already provides the IPC verb).
- **DISTRIB-003** — Homebrew Formula Automation (`releaseIntent: candidate`).
  Hands-free `brew upgrade anvil`.
- **MLP2-068** — `git cat-file --batch` perf follow-on to MLP2-016; merged via
  `d54a5f86` and included as release freight.
- **MLP2-069** — `EngineUnavailableReason::IoError` variant — clearer operator
  errors when `TempDir`/disk-full collapses onto the wrong reason.
- **ADOPT-004** — Complete Local-Noise Ignore Policy across every surface
  (extends WATCHUX-002's shared list to `watch` / `audit` / `hooks` /
  `anvil-run` / `baseline`).
- **OPSUP-006** — File-presence guards and wall-time caps. Defensive guards
  against absent files and runaway checks.
- **WOUT** — stable `anvil --json watch` NDJSON contract for downstream
  consumers; WOUT-001..006 are Merged and ride as developer-facing release
  freight.

**Third-pass freight (2026-05-18):** two additional MCP-shim friction fixes
joining the bundle after a hotfix-cut attempt against `v0.6.3-beta` was
abandoned (tracking issue #1694, closed). The CIB-005 fix depended on
intermediate `apply_patch` infrastructure (`f03f8aa9`) and `protection_claim`
integration (`7ff0e123`) that landed between `v0.6.3-beta` and PR #1692, so the
cherry-pick was entangled with v0.7.0-beta tool-UX work; deferring as freight is
the lower-risk path:

- **CIB-005** — Pre-write validator patch-mode support. `anvil_validate_write`
  accepts patch-only payloads via the existing `apply_patch` helpers; token cost
  scales with the change, not the file. Closes the primary friction from the
  2026-05-18 beta tester incident (single-Read budget hit on a 2770-line JSON
  metadata file). Merged via PR #1692.
- **CIB-007** — Untrusted-workspace-root preflight returns a recoverable
  `expectedWorkspaceRoot` field on rejection so callers can self-correct without
  operator round-trip. Triage option **(b)**; option (a) (worktree-aware accept)
  widens the trust boundary and remains deferred behind an ADR. Merged via PR
  #1692.

Release notes for the freight bundle ride under a "Sit-on quality improvements"
section, not under the protection-surface claim, so the daemon-working claim's
verifiability stays clean.

The 2026-05-16 release-plan refresh (the immediate prior baseline that brought
the document from `v0.6.2-beta` to the shipped `v0.6.3-beta` patch, refreshed
MLP2 from `35/66` to `41/66` after Group M closed via PRs #1602 + #1604, WATCHUX
to Complete 8/8, ADTRUST to Complete 6/6 archived, ADOPT to 2/6, DISTRIB to 3/5
against module truth after DISTRIB-004) is preserved as the prior commit on this
file's history.

> Companion: [ROADMAP.md](./ROADMAP.md) for thematic horizons. Execution source
> of truth: [`plans/index.aps.md`](./plans/index.aps.md) and the linked APS
> modules. This file selects the release slate and shows what can run in
> parallel; it does not duplicate every APS work item.

---

## Current State

**Latest tag in repo:** `v0.7.2-beta` (shipped 2026-05-25)

The daemon-working slate documented below as the "Next Release Window" has since
shipped: `v0.7.0-beta` (2026-05-21) tagged the slate, followed by patches
`v0.7.1-beta` (2026-05-22, Activation Diagnostic Honesty) and `v0.7.2-beta`
(2026-05-25, Save-Time Scanning & Tooling Honesty). See
[`CHANGELOG.md`](./CHANGELOG.md) for per-release notes.

`v0.6.3-beta` was the last pre-0.7 patch — Homebrew-aware curl installer, shared
local-noise ignore policy in watch/audit, initial-watch-as-baseline semantics,
immediate watch startup feedback, and `anvil uninstall`. It sat on top of
`v0.6.2-beta`, which closed the release-operating-model window (main-first
branch model, targeted CI/readiness checks, and deterministic release commands
on `main`).

| Area                              | Status  | Evidence                                                                                                                                                |
| --------------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `OPMODEL` main-first cutover      | Shipped | 12/12, archived 2026-05-11. Cutover SHA `b6f236e9`; `main` ruleset 16217152; `dev-retired-2026-05-11` tag.                                              |
| `RELORCH` release orchestration   | Shipped | 12/12, archived. Deterministic `assess`, `preflight`, `prepare`, `promote`, `tag`, `monitor`, `verify`, and `closeout` command surface exists.          |
| `CICD` targeting + drift controls | Shipped | 12/12, archived 2026-05-12. Fast PR targeting, integration SHA readiness, workflow contract map, and APS/repo/release drift checks are live.            |
| Beta watch UX hotfix              | Shipped | WATCHUX 8/8 closed in `v0.6.3-beta`; ADOPT-005 `anvil uninstall` shipped 2026-05-14 via PR #1521 and rode the same tag.                                 |
| Release tag                       | Shipped | Latest tag `v0.7.2-beta` (2026-05-25); `v0.6.3-beta` was the last pre-0.7 patch. Changelog entries live in `CHANGELOG.md` and the public release notes. |

The next release should be a **product-surface** release, not another operating
model release. Its claim moves from "the operating model is executable" to
"Anvil protects this project end-to-end through the daemon, hooks, witness
chain, baseline, and wrapped agent launch surfaces."

---

## Next Release Window — `v0.7.3-beta` "Surfacing the Signal"

**Candidate tag:** `v0.7.3-beta` (patch on the `v0.7.0-beta` sit-on slate)

**Claim:** A product-surface **patch** that makes Anvil's existing signal
**visible and exportable** — native read-only TUI dashboards, SARIF 2.1.0
findings export, and new `anvil insights` views — without widening the
protection claim. This is freight that accumulated on `main` since
`v0.7.2-beta`; the bulk is already Merged, so the remaining work is cut hygiene
and reconciliation, not implementation.

**Version framing (deliberate):** `scripts/release/assess.sh` always recommends
the next minor for a beta cut (`v{major}.{minor+1}.0` — here `v0.8.0-beta`)
regardless of what changed; its recommendation is mechanical, not
content-driven, so the version is an operator override in every case. Strict
semver does point the same way — the landed work adds new commands and flags
(`anvil dashboard`, `anvil migrate schema`, `--format sarif`, new `insights`
views), which is feature-additive surface. We override down to a **patch**
(`v0.7.3-beta`) anyway to honour the Hotfix Iteration Plan's "no minor beta
before six weeks post-tag" rule (`v0.7.0-beta` shipped 2026-05-21). Pass the
version explicitly to `assess.sh` / `prepare.sh` so the override is on the
record. This is a patch on the sit-on tag — **no Boring Week gate applies**
(that gates the sit-on claim, not weekly patches).

**Scope (all Merged on `main` unless noted):**

| Theme                                                           | Module / source         | State                                                 |
| --------------------------------------------------------------- | ----------------------- | ----------------------------------------------------- |
| SARIF 2.1.0 on `check` / `gate` / `audit` + `--format` selector | `sarif-output`          | 6/6 Complete                                          |
| `anvil dashboard` (architecture / drift / suppressions TUI)     | `native-tui-dashboards` | 4/4                                                   |
| `anvil insights --suppressions` / `--drift`                     | `usage-insights`        | 3/4 (the two new views Merged; INSIGHTS-004 deferred) |
| Working-tree secret scanning (not just git history)             | `scan-performance`      | Merged                                                |
| `anvil migrate schema`; `anvil welcome` gitignore count         | CLI / CIB               | Merged                                                |
| Policy-engine hardening (panic-catch + determinism fence)       | policy lane (#1952)     | Merged (preview-gated; output shape may still change) |

**Engineering-only churn** (recorded in `ENGINEERING-HISTORY.md`, not the
customer changelog): `dev-environment-hardening` (disk/worktree),
`lang-ts-audit` (TS symbol extraction internals), CIB items, CI/dependency-audit
scoping, `email-broadcast`.

**Tag-time hygiene** (mechanical, tag-blocking):

- Bump `Cargo.toml` `0.7.2-beta` → `0.7.3-beta`.
- Regenerate `Cargo.lock` + `ACKNOWLEDGEMENTS.md` atomically via
  `bash tools/starters/acknowledgements/generate-acknowledgements.sh`.
- `cargo hakari generate` + `cargo hakari verify`.
- Date-stamp the `## [0.7.3-beta] — TBD` CHANGELOG heading at tag time.
- Create `plans/releases/v0.7.3-beta.md` from the `v0.7.2-beta.md` template.

**Cut sequence:** the deterministic `scripts/release/*` chain —
`preflight → assess (--version v0.7.3-beta) → prepare → promote → tag → monitor → verify → closeout`.
Homebrew formula auto-bump (DISTRIB-003) rides the tag.

---

## Tracking — Save-Time CPU & Daemon Arc (post-`v0.7.3-beta`)

Tracking anchor for the work flowing out of the beta-tester high-CPU report (GH
[#2156](https://github.com/eddacraft/anvil-001/issues/2156)). The arc splits
into two release tiers with different governance and versioning: a near-term
bugfix **patch** (`v0.7.4-beta`) and an architectural **minor** window
(`v0.8.0-beta`) gated on a planning council. Neither is on `main` yet, so
neither is in the `v0.7.3-beta` cut above.

**Version framing.** `scripts/release/assess.sh` mechanically recommends the
next minor (`v0.8.0-beta`) for any beta cut regardless of content; the version
is an operator override in every case. With the six-week cadence hold retired
(2026-06-01), version is set by scope, not calendar: additive-surface work that
is **internal and default-inert** still maps to a patch, while the minor
(`v0.8.0-beta`) is reserved for the daemon work that genuinely widens the
product / protection surface — and it cuts when that slice is ready, not on a
date.

### Tier 1 — watch save-time CPU remediation → `v0.7.4-beta`

- **releaseIntent:** `candidate` for **`v0.7.4-beta`** (the immediate patch
  after `v0.7.3-beta`) once merged to `main`. Cannot ride `v0.7.3-beta` (that
  window is cut-hygiene on already-merged scope; this is unbuilt).
- **Co-freight (`v0.7.4-beta`):** `DISTRIB-006` — `ANVIL_HOME` / `--anvil-home`
  side-by-side install-root override (ADR-060 `Accepted` 2026-05-31; **Merged
  2026-05-31 via PR #2185**, after the v0.7.3-beta tag commit, so it is now
  v0.7.4-beta freight on `main`). Additive surface, but **internal-facing and
  default-inert** (unset `ANVIL_HOME` = byte-for-byte default), so it overrides
  down to the patch rather than forcing `v0.8.0-beta` — the same call the
  `v0.7.3-beta` window already made for its additive commands. Not a blocker for
  the CPU fix; both are already on `main` for the cut.
- **Claim it supports:** `anvil watch` stops spawning a full-repo `check --all`
  per save; single-agent save-time CPU drops from ~7 cores toward
  proportional-to-changed-files, and concurrent-agent save storms stay bounded.
  Directly answers the field report.
- **Deliverable:** `RLB-007` (scope the per-save action to changed paths;
  coalesce + cap concurrency), proven by the `RLB-001` load-ramp harness
  (before/after process-tree CPU at the measured tipping points).
- **Governance:** **bugfix tier — no planning council.** Reversible,
  single-crate, no contract change; aligns with ADR-002 (warnings over blocks)
  and the existing architecture.
- **Status:** `RLB-001` and `RLB-007` are **Ready** (greenlit as Tier-1
  freight); the rest of
  [`resource-load-benchmarking`](./plans/modules/resource-load-benchmarking.aps.md)
  stays `Proposed` pending the Tier-2 council.
- **Evidence:** prototype `benchmarks/prototypes/anvil-load-probe.py`; field
  report GH #2156.

### Tier 2 — daemon-mediated save-time validation → `v0.8.0-beta` (deferred)

- **releaseIntent:** `proposed` for **`v0.8.0-beta`** — the next **minor**, a
  product-surface window (not a patch); cut when the sub-phase A slice is ready
  and the gates are green (no calendar hold). Themes around "save-time
  governance without stealing the machine." Earns the minor because it widens
  the protection / product surface, not just additive flags.
- **Decision contract:**
  [ADR-061](./plans/decisions/061-save-time-daemon-delta-validation.md)
  (**Proposed**) — `anvil watch`/MCP/intercept become thin clients of one
  per-`(uid, os)` daemon (ADR-036) that validates changed paths over a warm
  Graph V2 hot-read slice; whole-repo scan becomes explicit/background with a
  `clean|stale|pending|running` workspace-assurance state.
- **Governance: gated on a planning council** before it enters a release window
  — to accept ADR-061 (Proposed → Accepted), resolve the dependent **Proposed**
  ADRs (015 daemon, 030 surface drivers, 031 latency rubric), reconcile against
  `MLP2-067` (daemon-hosted graph cache, Draft) so two graph-cache designs don't
  diverge, and tick the GV2 Ready item _"hot-/non-hot-path boundary agreed with
  INTD and DRVR owners."_
- **Modules to sequence:** `RLB-002/003/004/005/008`,
  [`graph-v2-foundation`](./plans/modules/graph-v2-foundation.aps.md)
  GV2-010/011/020/021/022 (currently **Draft**), INTD `validate_paths` method,
  DRVR (MCP re-point).
- **Do not** theme a release window around Tier 2 until the council output lands
  and GV2 reaches **Ready** — the release plan is `Derived` and follows
  Ready/Accepted modules, it does not lead them.

---

<a id="next-release-window-proposed--post-v060-beta-daemon-working-slate"></a>

## Next Release Window — `v0.7.0-beta` "Let's Use This" — Shipped 2026-05-21

> **Status (2026-05-28 fact-refresh):** This window shipped as `v0.7.0-beta` on
> 2026-05-21, followed by patches `v0.7.1-beta` (2026-05-22) and `v0.7.2-beta`
> (2026-05-25). The wave-by-wave planning, two-level claim, and module table
> below describe the now-closed window and are retained as historical context.
> Selection of the next release window is pending a strategic pass.

**Candidate tag:** `v0.7.0-beta`

**Two-level claim** (amended 2026-05-17 — see "Wave 5 timing amendment" below):

1. **Tag-time claim** (applies the moment `v0.7.0-beta` is cut): _Anvil protects
   this project end-to-end through the daemon, hooks, witness chain, baseline,
   and wrapped agent launch surfaces, with protection posture legible during
   sustained use and a signature-verified update path to every install method._
   This is the **daemon-working** claim and it is verifiable from code state
   alone — every gate listed under "Hard release gates" below is a checkable
   artefact.

2. **Sit-on claim** (graduates post-tag, after Wave 5 Boring Week confirmation):
   _Anvil is ready to live on a senior engineer's machine for a month without
   being uninstalled._ This is the operator-facing marketing claim. It graduates
   by release-notes / web-copy update on the existing tag — no re-tag required.
   If Boring Week surfaces a blocker, the next tag is `v0.7.1-beta` (per the
   existing patch cadence — `v0.6.3-beta` already followed this shape for the
   WATCHUX hotfix lane).

The original framing was documented in
[`plans/specs/2026-05-14-release-plan-v0.7.0-sit-on.md`](./plans/specs/2026-05-14-release-plan-v0.7.0-sit-on.md)
and accepted 2026-05-14. The two-level split was a follow-on amendment on
2026-05-17 to resolve the chicken-and-egg of a pre-tag gate that requires
post-tag usage data (Boring Week testers running unreleased branch builds is not
the install-path the release exists to enable).

**Primary APS modules:**

| Pick | Module                                                              | Status      | Progress | Role                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ---- | ------------------------------------------------------------------- | ----------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| N1   | [`MLP`](./plans/archive/modules/multilayer-protection.aps.md)       | Complete    | 18/18    | Multi-layer protection v1 primitives: project identity, witness chain, hooks, L4 policy, baseline, audit, attribution, workflow template, and protection-claim vocabulary. MLP-018 closed by splitting follow-up integration work into MLP2.                                                                                                                                                                                                                                                                                                                          |
| N1b  | [`MLP2`](./plans/modules/multilayer-protection-v2.aps.md)           | In Progress | 60/76    | Active follow-up integration module from the MLP-018 split plus Council hardening. Groups B, F, H, K, L, M all Complete; Group O is 1/2 after MLP2-068 merged as release freight. MLP2-025 split into -025/-025b/-025c during implementation; MLP2-051 split into an umbrella + -051a..-051e per the 2026-05-17 respec. MLP2-067 and MLP2-069 remain future work. It is part of the current protection claim only at the cut-line named below; 76/76 is not required for `v0.7.0-beta`.                                                                               |
| N2   | [`INTL`](./plans/archive/modules/intercept-launcher.aps.md)         | Complete    | 9/9      | `anvil-run` launcher, session registration, process-group control, shell wrappers, side-channel register. Shipped via PR #1528 (merged 2026-05-14 at `5d38e546`); Released/Shipped via `v0.7.0-beta` (2026-05-21); module Complete and archived.                                                                                                                                                                                                                                                                                                                      |
| N3   | Carry-forward gates                                                 | Confirmed   | 6/6      | ADR-036..039 Accepted (2026-05-13); project-id, noise-discipline policy, AIGUARD envelope, INTR-004 promoted, DRVR forward-compat — all hold. G5 closed 2026-05-13 when INTR-004 (path-deny rule) was promoted Draft → Ready in `intercept-rules.aps.md`.                                                                                                                                                                                                                                                                                                             |
| N4   | Documentation lanes                                                 | —           | 6/6      | **Done 2026-05-18.** All six runbooks live: air-gap (`docs/runbooks/anvil-air-gapped.md`), hooks-integration (`docs/runbooks/anvil-hook-coexistence.md`), witness-chain operator (`docs/runbooks/anvil-witness-chain.md`), adoption (`docs/runbooks/anvil-adoption.md`), `v0.6.x → v0.7.0-beta` migration (`docs/archive/runbooks/v0.6.x-to-v0.7.0-beta-migration.md`), and `anvil-run` (INTL) manpage (`docs/runbooks/anvil-run.md`). Owner: @aneki. Status column is `—` because these are not APS modules — see Wave 0 outcome row for the actual ownership scope. |
| N5   | [`WATCHUX`](./plans/archive/modules/watch-ux-advisory-rules.aps.md) | Complete    | 8/8 done | Beta incident remediation and first-run/watch UX. WATCHUX-001..-004 merged via PR #1497; -005..-007 via PR #1524; -008 on `feat/watchux-008-config-cache`. Rode `v0.6.3-beta`. Module archived.                                                                                                                                                                                                                                                                                                                                                                       |
| N6   | [`ADTRUST`](./plans/archive/modules/adoption-trust-surface.aps.md)  | Complete    | 6/6      | Adoption Trust Surface — all six shipped 2026-05-14 (PRs #1531, #1532, #1533, #1534, #1536, #1537); module archived. Cross-crate wire-ups for -002 (watch TUI + hook bridge) and -004 (anvil-hook + kernel embedded fallback) tracked under MLP2 group J.                                                                                                                                                                                                                                                                                                             |
| N7   | [`ADOPT`](./plans/archive/modules/adoption-friction.aps.md)         | Complete    | 6/6      | Adoption Friction Removal: **ADOPT-001 hook coexistence Done**, **ADOPT-002 resource budget Done**, **ADOPT-003 AI auto-detect Merged via PR #1700**, **ADOPT-004 shared ignore policy Merged via PR #1658**, **ADOPT-005 clean uninstall Released/Shipped via PR #1521** (rode `v0.6.3-beta`), and **ADOPT-006 editor coexistence Merged via PR #1682**. All items Released/Shipped (ADOPT-005 via `v0.6.3-beta`; the rest via `v0.7.0-beta` 2026-05-21); module Complete and archived. Wave 3A.                                                                     |
| N8   | [`DISTRIB`](./plans/modules/distribution-and-update.aps.md)         | In Progress | 4/5      | Distribution & Self-Update: DISTRIB-001 signature verification, DISTRIB-002 version check, and DISTRIB-003 Homebrew formula automation are Merged; DISTRIB-004 cadence policy is Done; `anvil migrate` remains. ADR-044 §9 makes DISTRIB-001 / -002 load-bearing for the MCP-backend swap discovery gap. Wave 3A.                                                                                                                                                                                                                                                     |
| N9   | [`INSIGHTS`](./plans/modules/usage-insights.aps.md)                 | In Progress | 1/4      | Usage Insights: INSIGHTS-001 `anvil insights` weekly summary is Done; suppression health, drift trend, and first-week adoption hint remain. Local-only, no telemetry. Wave 4.                                                                                                                                                                                                                                                                                                                                                                                         |

**Hard release gates** (all must be Merged before the tag is cut — these are the
tag-time-claim gates, verifiable from code state alone):

1. `MLP-009` protection-claim vocabulary suite — **Done**.
2. `ADTRUST-001` and `ADTRUST-002` — a non-Anvil developer reads `anvil status`
   once and explains what it means, and degraded states surface within 60s of
   next save-time interaction. **Module Complete 6/6.**
3. `ADOPT-001` hook coexistence with lefthook/husky/pre-commit-framework.
   **Done.**
4. `ADOPT-002` measured resource ceiling (CPU < 5% steady-state, RSS < 200MB on
   reference repo) green in CI. **Done.**
5. `DISTRIB-001` signature-verified update path on all install methods.
   **Merged.**

**MLP2 cut-line for `v0.7.0-beta`:** required before tag are MLP2-011 (including
merge-parent hash binding), MLP2-013, MLP2-014, reopened MLP2-016, reopened
MLP2-048, MLP2-061, MLP2-062 — **all Merged** — plus the security-chain finish
and operator-recovery additions named below:

- **MLP2-025 + MLP2-025b + MLP2-025c** (Critical, security surface) — **all
  three Merged.** Registry-side spoof rejection is live end-to-end: Phase 1
  primitives PR #1597 (2026-05-15), Phase 2 daemon control-lane PR #1603
  (2026-05-16), Phase 3 launcher migration PR #1608 (2026-05-16 at `1ea23349`).
  Umbrella status closed 2026-05-18; the production wire-up gap surfaced by
  DeepSec #1671 triage was closed by PR #1717, which installs the
  `CrossCheckContext` in `run_foreground` under a `cfg(target_os = "linux")`
  gate and extends the oversized scan_buffer fast-path validator to accept
  `env_agent_tag`. The daemon-working claim can honestly include **"on Linux,
  agent attribution survives a spoof attempt"**; cross-platform parity
  (`pid_starttime` / `parent_pid` on macOS, peer-PID + lineage on Windows) rides
  on MLP2-027 (macOS) and MLP2-028 (Windows), both deferred per the "Deferred or
  out-of-scope" entry below. The cfg gate in `run_foreground` widens
  automatically when those tickets land — no release-plan re-amendment needed.
- **MLP2-026** (High, operator recovery) — `degraded:fence-cascade` detection +
  `anvil intercept unblock --acknowledge-cascade` clear path. Contract spec
  Accepted via PR #1617; partial implementation landed on main. Without it,
  fence cascade reaches a refused state with no operator-clear verb.
- **ADOPT-003** (High, sit-on adoption leverage) — AI Tool Auto-Detect
  primitive, CLI wiring in `start.rs`, and the `anvil-run` cache consumer all
  merged via PR #1700. Boring Week testers no longer have to manually wire each
  AI tool just to produce a fair friction signal.
- **MLP2-051a + MLP2-051b + MLP2-051c + MLP2-051e** (Critical, HARD-GATE close)
  — protection-claim conformance pass across `anvil doctor`, MCP shim, and TS
  driver-client, plus the cross-surface parity test that closes the gate once
  the three render surfaces are wired. Per the 2026-05-17 respec on `732eef55`,
  the original MLP2-051 was an umbrella; the audit showed only `anvil status`
  actually renders a `ProtectionClaim` today and the other surfaces emit no
  claim at all (additive rendering work, not a string-render migration). All
  three render items are `Ready`; the parity test is `Blocked` only on those
  three landing. **Required for the daemon-working claim to include
  cross-surface protection-claim parity** — without it the tag ships with the
  narrower "CLI-only protection-claim parity" framing.

**Fresh-cut additions (2026-05-17 second pass):**

The cut-line above was filtered by the daemon-working theme. On a fresh review
without inherited "previously deferred" reasoning, the items below earn their
place on independent operator-visible merit. They are not load-bearing for the
daemon-working claim, but they are tightly scoped, near-complete, and improve
real sit-on quality regardless of theme.

- **RCLI3-016b** (Ready, High) —
  `anvil mcp install --client <cursor|claude-code>`. One-command MCP install
  wrapper that resolves the client config path, writes the entry, prints the
  restart hint. Already marked `🔒 PULLED FORWARD TO A1` since 2026-04-26
  because the demo runbook's §1.4 install step calls it directly. Without it,
  every new user hand-edits `~/.cursor/mcp.json` or Claude Code's config —
  exactly the kind of first-five-minutes friction Boring Week exists to surface.
  Tightly scoped: `crates/anvil-cli/src/commands/mcp.rs` (new) +
  `commands/mcp_config.rs` shared resolver.
- **RCLI3-017b** (Ready, High) — `anvil intercept unblock --worktree <path>`.
  Operator CLI wrapper for clearing a fenced worktree without restarting the
  daemon. INTD-007 already provides the IPC verb; this is just the CLI surface
  that the demo runbook's §3.1 soft-reset path invokes. Carved out 2026-04-26.
  Tightly scoped extension to `crates/anvil-cli/src/commands/intercept.rs`.
- **DISTRIB-003** (Draft, `releaseIntent: candidate`) — Homebrew Formula
  Automation. Auto-bump the `eddacraft/tap/anvil` formula on release so
  `brew upgrade anvil` actually updates without manual maintainer action. For a
  "sit-on for a month" claim, the dominant macOS install path needs hands-free
  update delivery. Three new files: workflow + script + runbook.
- **MLP2-068** (Merged, High) — Replace per-blob `git show` spawns in
  `CommitAntipatternEngine` with a single `git cat-file --batch` pipe per
  `validate_commit` call. Direct perf follow-on to MLP2-016, which just bound
  the real antipattern engine into production. Filed 2026-05-17 in Group O after
  the audit.
- **MLP2-069** (Draft, High; deferred) — Add `EngineUnavailableReason::IoError`
  variant to `anvil-l4`. Today `TempDir` / disk-full / blob-write failures
  collapse onto `BinaryMissing`, which misleads operators. Operator-error
  clarity follow-on to MLP2-016. Group O sibling.
- **ADOPT-004** (Draft) — Complete Local-Noise Ignore Policy Across All
  Surfaces. WATCHUX-002 established the shared ignore list in
  `anvil-cli/src/util.rs`; this extends it across `watch`, `audit`, `hooks`,
  `anvil-run`, and `baseline` so every surface honours the same list. Existing
  pattern, low-risk extension. Operators routinely hit noise from `.claude` /
  `.opencode` / `.gemini` / `.worktrees` / common cache dirs during Boring Week.
  Surface-by-surface conformance tests added.
- **OPSUP-006** (Complete) — File-presence guards and wall-time caps. Reusable
  check guards and runtime budgets so absent files do not break the run and
  runaway checks cannot exceed their wall-time budget. Operationally defensive —
  exactly the kind of footgun that surfaces during a real-use observation
  window. Self-contained kernel-internal helpers; no wire-shape change.

These items are bundled as **freight on the tag**, not as part of the
daemon-working release claim. The release notes should describe them under a
"Sit-on quality improvements" section rather than under the protection-surface
claim, so the claim's verifiability stays clean.

**Additional developer-facing freight:** WOUT-001..006 are Merged and should be
included in the release notes as a stable `anvil --json watch` NDJSON contract
for downstream consumers.

**Deferred or out-of-scope for this cut:**

- **MLP2-050** (TypeScript e2e mirror of protection-claim states) and
  **MLP2-051d** (GH Action check render). MLP2-051d is Blocked on the
  Marketplace publishing track (MLP2-042/-043 — GH Action exists), so its
  natural ship slot is alongside that track. MLP2-050 sits beside the
  driver-client mirror MLP2-051c covers; the e2e dimension can ride
  `v0.7.1-beta` or later if Boring Week exposes a gap.
- **Marketplace publishing** (Group I MLP2-042..-047), **observation fan-out**
  (Group A MLP2-004/-005/-007/-008), **cross-platform attribution** (Group E
  MLP2-027/-028), and **audit-chain rescoring** follow-ons are deferred unless
  Boring Week exercises those surfaces.
- **MLP2-069**, **DISTRIB-005**, and **INSIGHTS-002..004** are explicitly
  deferred from the tag-time claim. They retain candidate intent in their APS
  modules, but their current Draft state does not block `v0.7.0-beta`.

**Tag-time hygiene** (mechanical but tag-blocking):

- `Cargo.lock` + `ACKNOWLEDGEMENTS.md` regenerated atomically via
  `bash tools/starters/acknowledgements/generate-acknowledgements.sh` (workflow
  rule 13).
- `CHANGELOG.md [Unreleased]` populated from the `releaseNote` fields on every
  Merged item between `v0.6.3-beta..HEAD`.
- `workspace-hack` regeneration verified by `cargo hakari verify`.
- `plans/releases/v0.7.0-beta.md` release-record file created per the
  `v0.6.3-beta.md` template.

### Wave 5 timing amendment (2026-05-17)

The original 2026-05-14 framing placed Wave 5 Boring Week as a hard **pre-tag**
gate. That is inverted: Boring Week requires real install paths, real users, and
signed binaries — none of which exist for an untagged candidate. Running it
pre-tag means testers run `feat/*` branch builds on personal machines, which is
not the path `v0.7.0-beta` exists to validate.

**Amended placement:** Boring Week runs **post-tag** as the **sit-on-claim
graduation gate**. The tag is cut against the tag-time-claim gates listed above.
Wave 5 then determines:

| Outcome                          | Action                                                                                             |
| -------------------------------- | -------------------------------------------------------------------------------------------------- |
| 3+ testers finish the week clean | Sit-on claim **graduates** via release-notes / web-copy update on the existing `v0.7.0-beta`.      |
| Single blocker surfaces          | `v0.7.1-beta` (or later patch) cuts with the gap closed — same shape as `v0.6.3-beta` for WATCHUX. |
| Catastrophic regression          | Yank via `release-record discarded / yanked` lifecycle (RELORCH-012 surface).                      |

**Exit criteria** (unchanged from original framing, just relocated):

- Three or more internal users finish the observation window with Anvil still
  enabled, the same config they started with, and no fence / suppression /
  bypass workarounds added.
- At least one journal entry describing a real catch the tester would not have
  wanted to ship without Anvil's intervention.

If scope must be cut from the tag-time gates, rename the **tag-time claim**
before tagging. The **sit-on claim** never tags without Wave 5 evidence
regardless.

The MLP wave rows below are retained as release evidence. Current integration
debt that did not belong in the v1 primitive module now lives in MLP2.

---

## Parallel Delivery Shape

<a id="required-prerequisites-cross-cutting-glue"></a>

### Wave 0: Promote Contracts

**Status:** Complete (2026-05-13). These ran before broad implementation. Each
item removed ambiguity from the release claim so later lanes don't diverge.
Outcomes recorded inline.

| Work                                  | Parallel? | Outcome (2026-05-13)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| MLP readiness review                  | First     | **Done.** MLP-009 confirmed as the hard release gate (module §17–22, recommended landing order §17). MLP promoted **Proposed → Ready** in [`multilayer-protection.aps.md`](./plans/archive/modules/multilayer-protection.aps.md) and `plans/index.aps.md`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| INTL readiness review                 | First     | **Done.** `AgentTag` stub landed in `crates/anvil-intercept-proto/src/session.rs` (struct + `ANVIL_AGENT_TAG_ENV` / `ANVIL_TASK_ID_ENV` constants + 3 tests, all green via `cargo test -p eddacraft-anvil-intercept-proto`). INTL-003 / INTL-004 reference the real type; planning text now has a backing type definition. INTL promoted **Draft → Ready** at module level (`Draft` normalises to `Proposed` per `plans/aps-rules.md`, so the canonical lifecycle is `Proposed → Ready`). Module-level Ready means ready-to-start-Wave-3; INTL-003 / INTL-004 promoted to task-Ready, the other seven INTL tasks remain Draft pending their direct prerequisites.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Carry-forward gate reconciliation     | First     | **6/6 confirmed.** G1 ADR-036/-037/-038/-039 promoted Proposed → **Accepted (2026-05-13)**; `DECISION-LOG.md` updated; `pnpm adr:check` green (`43 ADR files; 43 indexed; no duplicates, no orphans`). G2 `anvil/project-id` schema reaffirmed (ADR-036 §D-2 + MLP-001) — no code yet, but the schema is pinned. G3 **policy** confirmed — ADR-038 codifies the Serena rule + hook surface table; a **behavioural** audit is deferred to Wave 2 when MLP-003 ships shippable hook output to audit. G4 AIGUARD envelope re-run: `cargo test -p eddacraft-anvil-kernel-types` green; the `diagnostic_schema_version_constant_matches_spec` test pins `anvil.diagnostic.v1`, which ADR-037 §D-1 reuses inside the witness line envelope. G5 **closed 2026-05-13:** INTR-004 (path-deny rule) promoted **Draft → Ready** in `intercept-rules.aps.md`. G6 DRVR forward-compat: `crates/anvil-intercept-proto/src/protocol.rs` already owns the editor-driver method names (DRVR-002 / DRVR-008); the new `session.rs` lives in the same crate without touching the existing `IpcCommand` / `IpcEnvelope` types — compatibility confirmed by full proto suite (`cargo test -p eddacraft-anvil-intercept-proto`, 28 passed, 0 failed).                                                                                                                                                                                                                               |
| Release-doc/runbook ownership refresh | First     | **Done; closed 2026-05-18.** All 16 pre-existing files in `docs/runbooks/` enumerated during Wave 0. Not changed by this slate (general operations; no MLP/INTL surface): `admin-cli`, `branch-reconciliation`, `db-migrations`, `emergency-hotfix`, `intd-012-windows-evidence`, `main-first-cutover`, `neon-db-operations`, `observability-triage`, `post-deploy-smoke-check`, `release-token-scope`, `rollback-bad-candidate-artefact`, `rollback-bad-main`, `rollback-bad-published-release`, `v0.6.0-beta-release-runbook`, `v0.6.0-beta-security-note`, `waitlist-email-operations`. **Net-new for `v0.7.0-beta` (six N4 lanes, owner @aneki, all live as of 2026-05-18):** `docs/runbooks/anvil-air-gapped.md`, `docs/runbooks/anvil-hook-coexistence.md`, `docs/runbooks/anvil-witness-chain.md`, `docs/runbooks/anvil-adoption.md`, `docs/archive/runbooks/v0.6.x-to-v0.7.0-beta-migration.md`, and `docs/runbooks/anvil-run.md`. **Additional pre-tag operator runbook also live as of 2026-05-18:** `docs/runbooks/v0.7.0-beta-release-runbook.md` (cut from the `v0.6.0-beta` template per GH #1706, refreshed for the daemon-working slate — 10-section operator procedure for cutting the tag, including pre-flight checklists for protection-claim render parity, witness-chain audit, hook-coexistence round-trip, and `anvil-run` smoke). `intd-012-windows-evidence.md` flagged for re-read when MLP-014 lands (multi-agent Windows scope). |

**Wave 0 follow-ups (next-window scope, not blocking Wave 1):** (1) ADR-039
`@anvil-ignore` hardening — forbid wildcards and same-diff
suppress-on-introduction for hard-pinned classes — filed against MLP-013. (2)
Formal council session for ADR-037 before the MLP-002 spike (recommended; the
witness-chain primitive is the single load-bearing point of failure).

### Wave 1: Build The Load-Bearing Backbone

These items should stay small and reviewable. `MLP-002` is the single point of
failure for most downstream work.

| Work                                    | Parallel? | Status            | Notes                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --------------------------------------- | --------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `MLP-001` project identity              | First     | Done (2026-05-13) | Establishes `anvil/project-id`; unblocks witness, baseline, and policy. Shipped via `activation/identity.rs` (22 tests green). v1 scope: idempotent UUID v7 write at `anvil start`; concurrent-rename convergence; symlink refusal. Deferred to follow-ups (see MLP-001 footnotes): composite-identity attach-check (with MLP-014), `--new-identity` flag (with MLP-007), `anvil baseline` integration (MLP-007).                                                         |
| `MLP-002` witness chain                 | After 001 | Done (2026-05-13) | Spike as a standalone PR with flock, rollover, DAG, and 80-writer tests. New `crates/anvil-witness/` crate: line + genesis + writer (`fs2` flock + rollover) + verifier (tamper / drop / stray-genesis detection); 25 tests green plus an `#[ignore]` 80-writer stress test. DAG-aware merge verification, manifest event stream, and `merge=union` integration test deferred to follow-ups (see module footnotes 1, 2, 3). CLI integration lands with MLP-003 hook lane. |
| `MLP-011` multi-format config           | Parallel  | Done (2026-05-13) | New `crates/anvil-config/` library: extension-based dispatch into `serde_json::Value`, canonical-JSON serialisation, detection precedence yaml > yml > json > toml. 44 tests green; cross-format equivalence pinned. CLI flag wiring deferred (see footnote 1).                                                                                                                                                                                                           |
| `MLP-013` hard-pinned rule classes      | Parallel  | Done (2026-05-13) | New `validation` module in `crates/anvil-config/`: rejects five disable-attempt shapes for `secrets` / `command-safety` (canonical + legacy locations + mode-disabled); tuning passes through; error messages cite ADR-039 and the `@anvil-ignore` bypass. 19 new tests green; 5 cross-format hard-pinned integration tests. `anvil-checks` rule-registration mirror deferred (see footnote 1).                                                                           |
| `MLP-017` air-gapped guarantee scaffold | Parallel  | Done (2026-05-13) | Linux network-namespace harness at `tools/test-harness/network-blocked/run.sh` (probes the kernel; exits 77 to skip on restricted hosts and non-Linux). Integration test suite at `crates/anvil-cli/tests/air_gapped.rs` (3 tests green covering `anvil version --offline`, `anvil status --verify --json`, and an executable-bit guard). Runbook at `docs/runbooks/anvil-air-gapped.md` documenting the extend-per-command protocol.                                     |

### Wave 1A: Beta Watch UX Hotfix — Shipped in `v0.6.3-beta`

This lane shipped in `v0.6.3-beta` (2026-05-15) because beta feedback showed
first-run watch could look hung, scan local agent worktrees, and render advisory
baseline findings as failures. WATCHUX-001..-004 closed the hotfix subset (PR
#1497); WATCHUX-005..-007 (PR #1524) and WATCHUX-008 closed the follow-up
UX/config-cache work. Module archived.

| Work                                   | Parallel? | Status | Notes                                                                                                       |
| -------------------------------------- | --------- | ------ | ----------------------------------------------------------------------------------------------------------- |
| `WATCHUX-001` Homebrew installer       | Parallel  | Done   | Detect existing Homebrew Anvil before curl installer runs standalone install; redirect to `brew upgrade`.   |
| `WATCHUX-002` shared ignore policy     | Parallel  | Done   | Skip `.claude`, `.opencode`, `.gemini`, `.serena`, `.worktrees`, generated/cache dirs in audit/watch paths. |
| `WATCHUX-003` initial watch baseline   | Parallel  | Done   | Initial scan builds graph/readiness state without emitting existing API surface as new violations.          |
| `WATCHUX-004` watch startup feedback   | Parallel  | Done   | Immediate startup feedback; non-TTY falls back to plain output instead of attempting TUI.                   |
| `WATCHUX-005` warning/failing language | Follow-up | Done   | Advisory findings render as `Warning`, not `Failing`. Merged via PR #1524.                                  |

### Wave 2: Hook, Policy, And Baseline Surfaces

Start once the witness primitive is stable enough for dependent lanes to write
against it.

| Work                                          | Parallel?         | Status            | Notes                                                                                                                                                                                                                                                                                                                                                                                               |
| --------------------------------------------- | ----------------- | ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `MLP-012` `rules_sha` in witnesses            | After 002 (+ 011) | Done (2026-05-13) | Shipped as a new `crates/anvil-rules/` library: `RulesShaInput` + `rules_sha` over canonical JSON of `{anvil_version, config_sha, opa_runtime_version, rules}` + `RequiredAnvilVersion` semver-floor parser. 29 tests green incl. yaml/json/toml cross-format determinism. (Merged via PR #1489.)                                                                                                   |
| `MLP-007` baseline command                    | After 001 + 002   | Done (2026-05-13) | v1 library primitive shipped via `crates/anvil-baseline/`: `Baseline` schema + move-resistant `compute_fingerprint` + TOCTOU-hardened I/O (incl. broken-symlink + tmp-path refusal, atomic-replace for Windows) + diff partition; 44 tests green. CLI command, scanner integration, cutoff_commit policy pinning, witness genesis emission, hook install deferred to consumers (MLP-003 / MLP-006). |
| `MLP-003` pre-commit hook                     | After 002 + 012   | Done (2026-05-13) | v1 library primitive shipped via `crates/anvil-hook/`: ADR-038 §D-1/§D-4/§D-5/§D-6/§D-7 primitives (Verdict, SuppressionLog, detect_framework, shell_template, panic_catcher_hook); 47 tests green. CLI subcommands, framework install, witness append wiring, daemon RPC deferred to consumers.                                                                                                    |
| `MLP-005` post-commit/post-merge/post-rewrite | After 003         | Done (2026-05-13) | `anvil hook post-commit/post-merge/post-rewrite` CLI subcommands; `anvil-witness` extended with `parent_commits[]` / `prev_line_hashes[]` for DAG-aware merge writes (parent enumeration via `git rev-list --parents`).                                                                                                                                                                             |
| `MLP-006` L4 policy framework                 | After 002 + 007   | Done (2026-05-13) | v1 schema + resolver shipped via `crates/anvil-l4/`: `Policy` / `BranchRule` schema (yaml/json/toml via anvil-config) + globset first-match-wins resolver + `commit_is_before_cutoff` ancestry check + four boundary-rejection error variants. 24 tests green.                                                                                                                                      |
| `MLP-008` hook bootstrap recovery             | After 003         | Done (2026-05-13) | `anvil hook bootstrap [--dry-run]` executes `BootstrapPlan` (Husky regenerate / `.git/hooks/` install / NothingToDo) with the ADR-038 §D-5 3-line wrapper. `--witness-recent` walk deferred.                                                                                                                                                                                                        |

### Wave 3: Coordination And Launcher Ingress

These lanes turn protection from a git-only mechanism into an agent-aware
runtime loop.

| Work                                        | Parallel?      | Notes                                                                            |
| ------------------------------------------- | -------------- | -------------------------------------------------------------------------------- |
| `MLP-014` multi-session + task fences       | With INTL      | Coordinates directly with INTL session registration and `AgentTag` schema.       |
| `INTL-001` launcher scaffold                | First INTL     | Creates `crates/anvil-run/` and workspace wiring.                                |
| `INTL-002` daemon connectivity/fence check  | After INTL-001 | Refuses launch when daemon is unreachable or worktree is fenced.                 |
| `INTL-003` session registration             | After INTL-002 | Registers tool/worktree/cwd/tmux context before spawn.                           |
| `INTL-004` process-group launch             | After INTL-003 | Uses Unix PGIDs or Windows named Job Objects so daemon can target interruptions. |
| `INTL-005` cleanup and `INTL-009` heartbeat | After INTL-004 | Keeps daemon state accurate and reapable.                                        |
| `INTL-006` shell wrappers                   | After INTL-001 | Adds zsh/bash integrations for common tool commands.                             |
| `INTL-007` side-channel registration        | After INTL-003 | Supports sessions not launched through `anvil-run`, with downgraded enforcement. |
| `INTL-008` blocked-launch UX                | After INTL-002 | Makes refusal states actionable.                                                 |

### Wave 3A: Adoption Friction

Parallel-safe with Wave 3 and 3B. No shared single-point-of-failure inside the
wave. ADOPT-004 waits for WATCHUX-002 (shared ignore helper, already in flight);
the rest are independent.

| Work          | Parallel?           | Status     | Notes                                                                                                                                     |
| ------------- | ------------------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `ADOPT-001`   | Parallel            | **Done**   | Hook coexistence with lefthook / husky / pre-commit-framework — shipped 2026-05-15. Runbook at `docs/runbooks/anvil-hook-coexistence.md`. |
| `ADOPT-002`   | Parallel            | **Done**   | Resource budget — CPU/RSS measurement and CI ceiling in `crates/anvil-bench`.                                                             |
| `ADOPT-003`   | Parallel            | **Merged** | AI tool auto-detect (Claude Code, Cursor, Aider, Windsurf, Codex) via PR #1700.                                                           |
| `ADOPT-004`   | After WATCHUX-002   | **Merged** | Complete the local-noise ignore policy across watch, audit, hooks, `anvil-run` via PR #1658.                                              |
| `ADOPT-005`   | —                   | **Done**   | `anvil uninstall` shipped 2026-05-14 via PR #1521; rode `v0.6.3-beta`.                                                                    |
| `ADOPT-006`   | Parallel            | **Merged** | Editor surface coexistence matrix via PR #1682.                                                                                           |
| `DISTRIB-001` | First (sig scheme)  | **Done**   | Minisign verification + ADR-045 — Merged via PR #1562.                                                                                    |
| `DISTRIB-002` | After `DISTRIB-001` | **Done**   | `anvil version --check` advisory surface + watch/status hint — Merged via PR #1569.                                                       |
| `DISTRIB-003` | Parallel            | **Merged** | Homebrew formula auto-bump on release via PR #1652.                                                                                       |
| `DISTRIB-004` | Parallel            | **Done**   | `docs/policies/release-cadence.md` + EOL policy.                                                                                          |
| `DISTRIB-005` | After `DISTRIB-002` | Draft      | `anvil migrate` for cross-version config reconciliation.                                                                                  |

### Wave 3B: Trust Surface — Shipped 2026-05-14

All six ADTRUST items shipped on 2026-05-14 and the module is archived. Cross-
crate wire-ups for -002 (watch TUI + hook bridge) and -004 (anvil-hook + kernel
embedded fallback) carry forward under MLP2 group J.

| Work          | Parallel? | Status   | Notes                                                                |
| ------------- | --------- | -------- | -------------------------------------------------------------------- |
| `ADTRUST-001` | First     | **Done** | `anvil status` plain-mode legibility — PR #1531.                     |
| `ADTRUST-002` | Sequenced | **Done** | Degraded-state banner primitive, rate-limited — PR #1534.            |
| `ADTRUST-003` | Parallel  | **Done** | `anvil doctor --fix` recovery for documented bad states — PR #1536.  |
| `ADTRUST-004` | Parallel  | **Done** | `anvil start` idempotency contract pin — PR #1537.                   |
| `ADTRUST-005` | Sequenced | **Done** | `anvil status --json` schema pinned at `anvil-status.v1` — PR #1532. |
| `ADTRUST-006` | Parallel  | **Done** | First-run claim summary + verification recipe — PR #1533.            |

### Wave 4: Release-Gate Closure

Do not tag until these are green and reflected in release evidence.

| Work                                  | Status | Notes                                                                                                                                                                               |
| ------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `MLP-004` pre-push hook               | Done   | Shipped via PR #1499; walks pushed ranges, verifies witnesses, and applies L4 fallback.                                                                                             |
| `MLP-009` protection-claim vocabulary | Done   | Closed-set protection vocabulary and serde contract shipped with MLP v1; rendered correctness is gated by reopened MLP2-048 plus MLP2-049/050/051 as scoped.                        |
| `MLP-010` workflow template primitive | Done   | Shipped via PR #1504; in-tree workflow template/accessor exists. External Marketplace publishing remains MLP2-042..045 and is not implied by this row.                              |
| `MLP-015` L5 audit                    | Done   | Shipped via PR #1500; audit-chain lane covers bypassed-layer detection.                                                                                                             |
| `MLP-016` L1 editor driver → Kindling | Done   | Shipped via PR #1503; mid-edit Kindling observation builder completed.                                                                                                              |
| Documentation lanes                   | Gate   | **6/6 closed 2026-05-18.** Air-gap, hooks-integration, witness-chain operator, adoption, `v0.6.x → v0.7.0-beta` migration, and `anvil-run` manpage all live under `docs/runbooks/`. |
| `INSIGHTS-001`                        | Done   | `anvil insights` weekly summary derived from the witness chain; JSON schema pinned at `anvil.insights.v1` / `schemas/anvil-insights.v1.json`.                                       |
| `INSIGHTS-002`                        | Draft  | Suppression health view; flags stale suppressions where the underlying violation is gone.                                                                                           |
| `INSIGHTS-003`                        | Draft  | Drift trend sparkline — 8 weeks of new cross-boundary edges; reports "insufficient data" honestly when applicable.                                                                  |
| `INSIGHTS-004`                        | Draft  | First-week adoption hint nudging new users at `anvil insights` once per week for 14 days.                                                                                           |

### Wave 5: Boring Week Validation Gate

**Post-tag sit-on graduation gate.** No amount of test fixture coverage proves
"ready to use." The only proof is using it. The `v0.7.0-beta` tag may be cut
once the tag-time daemon-working claim is satisfied; Boring Week graduates the
separate sit-on claim after signed binaries and real install paths exist.

**Entry criteria — which work must be green before Wave 5 can start:**

| Wave                          | Required for entry                                                                                                                                                                                          | Rationale                                                                                                                                                                                                                                                                                                                    |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Wave 3 (Coordination + INTL)  | `MLP-014` + `INTL-001..-005` only                                                                                                                                                                           | INTL-001..-005 deliver the wrapped-launch ingress that the protection-claim render path depends on. INTL-006..-009 (shell wrappers, side-channel registration, blocked-launch UX, heartbeat) are sequenced for the same release but can defer to `v0.7.1` per the Risks table — they refine UX rather than enable the claim. |
| Wave 3A (Adoption Friction)   | All six ADOPT items + DISTRIB-001..-003                                                                                                                                                                     | DISTRIB-004 (cadence doc) and DISTRIB-005 (`anvil migrate`) are not Boring-Week-blocking.                                                                                                                                                                                                                                    |
| Wave 3B (Trust Surface)       | All six ADTRUST items                                                                                                                                                                                       | All are load-bearing for the legibility gate.                                                                                                                                                                                                                                                                                |
| Wave 4 (Release-Gate Closure) | MLP-004 / -009 / -010 / -015 / -016 (v1 primitives), MLP2 cut-line (`MLP2-011`, `MLP2-013`, `MLP2-014`, `MLP2-016`, `MLP2-048`, `MLP2-061`, `MLP2-062`), INSIGHTS-001, documentation lanes, release runbook | INSIGHTS-002 / -003 / -004 are not Boring-Week-blocking; deferred MLP2 surfaces must be named honestly in release notes.                                                                                                                                                                                                     |

If any Wave 3 INTL item from the deferrable set (-006..-009) is _not_ green at
freeze time, it must be explicitly listed as deferred-to-v0.7.1 in the release
notes so the cut claim still matches reality.

**Protocol:**

1. With the entry criteria above met, freeze the candidate SHA.
2. Three or more internal users install the candidate via the fresh-user path
   (Homebrew install or curl installer; no developer overrides) on their primary
   work machine.
3. For one calendar week, each user runs Anvil against their normal daily work.
4. Each user keeps a journal of every visible warning, every suppression, every
   bypass, every disabled check, every daemon failure, and every `anvil doctor`
   invocation.
5. End-of-week review: any disabled check or unresolved suppression is a cut
   blocker. Any "I gave up and turned it off" event is a cut blocker. Any daemon
   failure that did not auto-recover is a cut blocker.

**Exit criterion:** All three users finish the week with Anvil still enabled,
the same configuration they started with, and at least one journal entry
describing a real catch they would not have wanted to ship.

**Non-goal:** Wave 5 is not a perf test or a stress test. It is a sustained-use
trust test. The instrument is the journal, not a benchmark.

**Participants:** TBD by @aneki before tag. Journals land in
`plans/audits/2026-XX-XX-boring-week-v0.7.0.md` as the release record.

---

## Cut Criteria For `v0.7.0-beta`

`v0.7.0-beta` is cuttable when the release evidence supports the tag-time
daemon-working claim without caveats. The stronger sit-on claim ("Anvil is ready
to live on a senior engineer's machine for a month without being uninstalled")
graduates only after post-tag Boring Week evidence.

| Criterion                    | Required Evidence                                                                                                                                                         |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Protection claim correctness | `MLP-009` vocabulary green plus reopened `MLP2-048` render path and scoped fixture/conformance items green; do not claim non-CLI parity unless MLP2-050/051 are complete. |
| Protection claim legibility  | `ADTRUST-001` and `ADTRUST-002` green: a non-Anvil developer reads `anvil status` once and explains what it means.                                                        |
| Save-time and hook layers    | Pre-commit, pre-push, post-commit/post-merge/post-rewrite hooks run with ADR-038 noise discipline.                                                                        |
| Hook coexistence             | `ADOPT-001` green: install + run alongside lefthook / husky / pre-commit-framework on representative configs.                                                             |
| Witness integrity            | Concurrent writes, rollover, tamper detection, DAG verification, post-rollover append (`MLP2-061`), and L4 chain verification (`MLP2-062`) pass.                          |
| Baseline adoption            | Existing repositories can adopt without broad warning noise; hard-pinned classes remain enforced.                                                                         |
| Agent launch path            | `anvil-run` registers sessions, isolates process groups / Job Objects, heartbeats, cleans up, and reports refusals.                                                       |
| Resource budget              | `ADOPT-002` green: CPU < 5% steady-state, RSS < 200MB on the reference repo, measured in CI.                                                                              |
| Update path                  | `DISTRIB-001` green: signature-verified `anvil update` on Homebrew, curl-installer, and library paths.                                                                    |
| Clean uninstall              | `ADOPT-005` green (shipped 2026-05-14): `anvil uninstall` returns a repo to byte-identical pre-install state for tracked files.                                           |
| Air-gapped guarantee         | Core commands pass under a network-blocked sandbox.                                                                                                                       |
| Release machinery            | `scripts/release/*` can assess, prepare, tag, monitor, verify, and close out the exact `main` SHA being released.                                                         |
| Docs/runbooks                | User-facing docs and runbooks match the shipped claim; no protection state is described more strongly than evidence.                                                      |
| Boring Week                  | Post-tag sit-on graduation evidence; not a tag-time blocker for the daemon-working claim.                                                                                 |

**Anti-goal:** do not ship a partial MLP/INTL/ADTRUST/ADOPT slice under the
daemon-working claim. If scope must be cut, rename the tag-time release claim
before tagging.

**Anti-goal:** do not bypass Wave 5 because the candidate looks good. The
candidate always looks good. The Boring Week exists because looks-good and
gets-used diverge.

---

## Hotfix Iteration Plan (Post-Tag)

**The six-week sit-on hold is retired (2026-06-01, authorised by Josh).** The
priority is continuous feature delivery toward an investor-ready solution;
releases are gated by quality (releasable `main`, green release gates, APS
authorisation), not by a calendar. Iteration shape:

| Cadence                | Channel                               | Scope                                                                 |
| ---------------------- | ------------------------------------- | --------------------------------------------------------------------- |
| `v0.7.x` patch         | Weekly while user signal is non-empty | Bug fixes, false-positive reductions, doc corrections.                |
| `v0.7.x` patch         | Within 48h of any P0 bug              | Crash, data loss, false-claim regression, daemon corruption.          |
| Next minor beta        | When ready — green gates + APS auth   | Feature additions. No calendar gate; cut when the slice is ready.     |
| Breaking beta or major | Demand-pulled                         | Driven by a real adopter requirement, not by completion of a backlog. |

Patches still ship continuously on user signal. See the authoritative
[release-cadence policy](docs/policies/release-cadence.md) and DISTRIB-004.

---

## Risks

| Risk                                                            | Mitigation                                                                                                                                |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Scope expansion makes ship date slip indefinitely.              | Each new module has explicit defer-to-`v0.7.x` items marked in its task body. Cut to the gate criteria, not below.                        |
| Boring Week protocol is performative, not real.                 | Require journal artefacts as a release record. Journals land in `plans/audits/2026-XX-XX-boring-week-v0.7.0.md`.                          |
| `anvil-run` (INTL) is not actually needed for "let's use this." | Defer cleanly: INTL-001..-005 ship in `v0.7.0`; INTL-006..-009 can land in `v0.7.1` if Wave 3 slips and they would block Boring Week.     |
| ADTRUST surface increases noise.                                | ADTRUST-002 has an explicit noise budget (≤1 banner per 60s, never more than one concurrent). Tests pin this.                             |
| MCP-backend swap silently fails for existing users.             | ADR-044 §9 + DISTRIB-001 / -002. Until DISTRIB ships, the release notes carry the manual "run `anvil start` after upgrading" instruction. |
| New modules clash with existing in-flight work.                 | All Wave 3A / 3B work depends only on Done MLP surfaces and one WATCHUX item (WATCHUX-002, already in flight).                            |

---

## Later Windows

After the daemon-working release, promote the next slice based on real adoption
signals rather than pre-allocating a fixed version.

### RMCPF: Rust MCP Full-Port Phasing

RMCPF is the likely next MCP-focused lane after the daemon-working slate, but it
should not block `v0.7.0-beta`. The v0.7 claim is daemon/hooks/witness/launcher
protection; RMCPF replaces the archived TypeScript MCP server once parity is
defined and demand for each surface is clear.

| Phase                              | Scope                                                                                                                                       | Gate                                                                                                                           |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| 0 — Inventory lock                 | Complete RMCPF-001 against `archive/anvil-mcp-server/src/`; confirm client matrix and Streamable HTTP demand.                               | Inventory matrix reviewed; remaining module-level readiness blockers are explicit client, transport, and retirement decisions. |
| 1 — Core tool parity               | Implement RMCPF-010 and RMCPF-011 in Rust under `anvil mcp serve`, preserving DRVR-006 daemon/local classifications and DRVR-007 redaction. | Fixture parity for `anvil_check`, `anvil_gate`, `anvil_status`, `anvil_fix`, `anvil_suppress`, and `anvil_query_boundary`.     |
| 2 — Resources, prompts, transports | Implement or retire RMCPF-012, RMCPF-020, and RMCPF-021 using `docs/architecture/rust-mcp-server-spec.md` as authority.                     | Resource read/list tests pass; prompt and HTTP retain/retire decisions documented.                                             |
| 3 — Cutover and retirement         | Ship RMCPF-030 compatibility harness and RMCPF-031 TypeScript MCP retirement/archive decision.                                              | Generated configs and release-critical docs point at Rust MCP; migration doc names all intentional incompatibilities.          |

| Future slice                                 | Source             | Gate before promotion                                                           |
| -------------------------------------------- | ------------------ | ------------------------------------------------------------------------------- |
| Rust MCP full port                           | RMCPF              | Phase 0 inventory lock complete and supported-client demand confirmed.          |
| Team-lead browser surface                    | Dashboard/export   | Daemon-working evidence stream exists and can be exported reliably.             |
| Enterprise / compliance / language expansion | Queued APS modules | Demand-pulled by a design partner or customer; do not pre-build as speculation. |
| Wider language and rule-pack coverage        | Queued APS modules | Core protection loop stable enough that added breadth does not dilute signal.   |
