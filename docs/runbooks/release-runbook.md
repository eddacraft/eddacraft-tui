# Anvil Release Runbook

| Type    | Authority     | Owner   | Status | Freshness                                                                                   |
| ------- | ------------- | ------- | ------ | ------------------------------------------------------------------------------------------- |
| Runbook | Authoritative | RELORCH | Live   | Last reviewed 2026-05-16 against `docs/policies/release-cadence.md` and `scripts/release/*` |

| Upstream                                                           | Downstream                            |
| ------------------------------------------------------------------ | ------------------------------------- |
| `scripts/release/*`, `docs/policies/release-cadence.md`, APS state | Release operators and `release` skill |

Purpose: ship Anvil with the least manual choreography possible.

This runbook uses the command-driven release model. Deterministic release work
is owned by the per-phase commands under `scripts/release/`.

`main` is the only permanent product branch following the OPMODEL-012 cutover on
2026-05-11. Releases tag a selected green `main` SHA after release readiness —
there is no `dev -> main` promotion.

Release cadence, beta support windows, and hotfix expectations are defined in
[`docs/policies/release-cadence.md`](../policies/release-cadence.md).

## Golden Rule

Do not hand-edit release state during a normal release.

If a deterministic command fails, fix that command or use its recovery mode.
Only perform manual recovery when the operator explicitly approves it, and log
the manual steps in the release tracking issue. Recovery playbooks live under
[`docs/runbooks/`](../runbooks/) — see
[`rollback-bad-main.md`](../runbooks/rollback-bad-main.md),
[`rollback-bad-candidate-artefact.md`](../runbooks/rollback-bad-candidate-artefact.md),
[`rollback-bad-published-release.md`](../runbooks/rollback-bad-published-release.md),
and [`emergency-hotfix.md`](../runbooks/emergency-hotfix.md).

## Required Tools

These commands must work from the repository root before release day:

```bash
bash scripts/release/assess.sh --help
bash scripts/release/preflight.sh --help
bash scripts/release/prepare.sh --help
bash scripts/release/promote.sh --help
bash scripts/release/tag.sh --help
bash scripts/release/monitor.sh --help
bash scripts/release/verify.sh --help
bash scripts/release/closeout.sh --help
```

If any command is missing, the release process is not ready. Do not substitute a
manual checklist for that command surface.

Third-party attribution gates (run before tag — both wired into the
`Acknowledgements freshness` CI job so a passing pipeline already proves them,
but a local pre-tag run surfaces a stale lockfile faster than the CI fast-fail):

```bash
tools/starters/acknowledgements/generate-acknowledgements.sh --check
tools/starters/acknowledgements/expand-licences.sh --check
```

The first verifies `ACKNOWLEDGEMENTS.md` is in sync with the runtime dependency
graph and that every workspace crate has a `license` / `license-file` field
(ATTRIB-007). The second verifies `about.toml.accepted` and
`deny.toml.[licenses].allow` are in sync with the canonical `licences.toml`
(ATTRIB-006). Source of truth for licences is `licences.toml`; both consumer
arrays are regenerated from it. See the
[`attribution-pipeline-v3`](../../plans/archive/modules/attribution-pipeline-v3.aps.md)
module for design history and the
[release doc checklist](../guides/release-doc-checklist.md#pre-release-third-party-attribution-attrib)
for the per-release tick-box.

## Happy Path

Use this path for normal releases. Releases tag a selected green `main` SHA
after release readiness; promotion remains available for release branches that
need a PR before tagging.

### 1. Start The Release

Run the release skill:

```text
/release
```

The skill reads live state, checks for open release issues, and runs assessment
against the selected `main` SHA. `assess.sh` accepts target-mode inputs
`--base <previous-tag> --source-sha <main-sha>`; `--base` is required.

```bash
bash scripts/release/assess.sh --base <previous-tag> --source-sha <main-sha> --json
```

Approve or override the proposed:

- version
- release type: `beta` or `production`
- strategy: `direct` or `stabilisation`

#### 2. Run Preflight

`preflight.sh` runs deterministic local gates. `--base` and `--head` are
optional and only used for the JSON record of what was compared; preflight
itself does not require a SHA input.

```bash
bash scripts/release/preflight.sh \
  --base <previous-tag> \
  --pre-prepare \
  --version <candidate-version>
```

**Publication credential.** Preflight does not see GitHub Actions secrets, so
verify the publication credential separately before committing to a cut:

Read the token into a subshell rather than putting it on the command line — an
inline assignment lands in shell history and is visible to process inspection,
the same hazard [`release-token-scope.md`](./release-token-scope.md) already
warns about:

```bash
(
  read -rsp 'ANVIL_RELEASES_TOKEN: ' ANVIL_RELEASES_TOKEN && echo
  export ANVIL_RELEASES_TOKEN
  bash scripts/release/validate-publication-token.sh
)
```

The subshell keeps the token out of the parent shell's environment once the
check finishes.

The readiness workflow runs the same check in `readiness` mode, which is the
authoritative gate — this local run just lets you find the problem before you
start. It fails when the token is absent, rejected, lacks write access to
`eddacraft/anvil` or the tap, or expires within 14 days.

Why it exists: the `v0.9.0-beta` cut stalled ~6h on an expired token, found at
the cross-repo publish step after prep, tagging and artefact builds had already
run. `release.yml` resolves the credential as
`secrets.ANVIL_RELEASES_TOKEN || secrets.GITHUB_TOKEN`, so an absent secret does
not fail loudly — it falls back to a token with no cross-repo write and dies
late. Rotation scopes are in
[`release-token-scope.md`](./release-token-scope.md); the schedule is in
[`secret-rotation.md`](./secret-rotation.md).

This must pass before release prep starts. `--pre-prepare` is required because
`prepare.sh` owns the version bump; it checks the planned candidate version and
all formatting, linting, typechecking, test, and release-tool pin gates without
mistaking the source version for a forgotten bump.

For scanner/runtime releases, treat `pnpm test:scanner-parity` as a named
preflight gate when the script exists in the checked-out SHA. It proves the
current Rust scanner and any retained transition-window parity fixtures agree on
the rule catalogue before a tag is cut. If the script is absent, record that the
TypeScript scanner parity harness has been archived and rely on the Rust scanner
tests in the normal preflight bundle.

For releases that carry default-on save-time daemon routing (`v0.8.0-beta`
onward), the
[daemon routing rollout controls](#daemon-routing-rollout-controls-adr-075) gate
must also pass before tag — exercise the opt-out and confirm the revert signal
is monitorable.

#### 3. Prepare Release State

```bash
bash scripts/release/prepare.sh \
  --version <version> \
  --release-type <beta|production> \
  --strategy <direct|stabilisation>
```

This command owns:

- version surfaces
- release notes
- generated public docs
- release metadata
- release tracking issue
- release preparation commit

Do not manually edit these files if prepare fails.

**Changelog promotion.** `prepare.sh` moves the whole `## [Unreleased]` draft
from `CHANGELOG.md` into a `## [<version>] — <date>` section, seeds the public
changelog with the same entries at the top, and leaves `[Unreleased]` empty but
for its standing draft note. It **refuses to run** — before bumping any version,
so the tree stays clean — when there is nothing to promote. If that happens,
write the customer-facing entries first; use
`ANVIL_RELEASE_ALLOW_EMPTY_CHANGELOG=1` only for a genuinely internal-only
patch.

Two things still need a human, so budget for them rather than discovering them
at review:

- **The section title.** Promotion emits `## [<version>] — <date>`; the window's
  name (e.g. "First-Run Wins and the Assistant Graph") and any opening paragraph
  are yours to add.
- **The public changelog is a summary, not a copy.** It is seeded from the same
  entries so you trim in place; cut anything internal and reduce the rest to
  user-visible outcomes.

**Review the curation diff.** Any hand-edit you make to either changelog after
`prepare.sh` runs must get the same scrutiny as the generated content — review
it as its own commit on the promotion PR, not folded into the generated one. At
the `v0.9.0-beta` cut, hand-curation introduced a duplicated persistence bullet
that only a reviewer caught (CIB-196). Check specifically that no entry is
duplicated, that no entry was dropped from the draft, and that upgrade notes
describe the behaviour _this_ release ships.

**Manual step — bump the "latest tagged release" docs strings.** `prepare.sh`
regenerates the changelog mirror but does **not** touch the hardcoded "latest
tagged release" / "current version" claims in the public docs. After prepare,
update these three to the new tag in the same PR:

- `docs/public/anvil/overview.md` ("the latest tagged release is …")
- `docs/public/anvil/quickstart.md` ("the latest tagged release is …")
- `docs/public/anvil/beta-testing-guide.md` ("**Current version:** …")

Leave version-specific historical references (e.g. "requires `v0.7.1-beta` or
newer") unchanged — only the "latest/current" claims move per release.

#### 4. Open Release PR (stabilisation strategy only)

```bash
bash scripts/release/promote.sh \
  --version <version> \
  --strategy <direct|stabilisation> \
  --source-sha <promoted-source-sha> \
  --request-readiness \
  --channel <beta|stable> \
  --base-boundary <previous-tag-or-ref>
```

Used only when the `stabilisation` strategy applies — `main` cannot be tagged
directly and a short-lived `release/*` branch carries hardening commits. Direct
strategy invokes `promote.sh` with `--strategy direct --source-sha <main-sha>`
to record the no-op promotion and tag `main` immediately.

If the command opens a PR, review and merge it through GitHub. Re-run the
command after merge so it records the merged state.

#### 5. Pre-tag review (required gate)

Run the pre-tag review against the **frozen candidate** (the `release/*`
stabilisation branch from step 4, or the selected `main` SHA for a direct
strategy) before tagging: an adversarial `clawpatch` sweep, a `release`-tier
council, and a **human sign-off**. This is where the release-window diff gets
its last adversarial look; it has repeatedly caught real issues.

The full doctrine — including the **delta-only re-pass** model that keeps a fix
from forcing a full restart (freeze the candidate, re-review only the delta,
carry verdicts forward, defer-don't-block) — lives in
[`release-process.md`](release-process.md). Do not tag until its human gate is
signed in `plans/reviews/release-council/<date>-<tag>-pre-tag.md`.

#### 6. Tag

```bash
bash scripts/release/tag.sh --version <version> --source-sha <promoted-source-sha>
```

The tag command must verify:

- local and remote `main` agree
- the version on `main` is correct
- release-readiness passed for the exact source SHA
- the remote is `eddacraft/anvil-001`
- provenance state is recorded before tag push

#### 7. Monitor Publishing

```bash
bash scripts/release/monitor.sh --version <version>
```

This records the cargo-dist release workflow state. Until live `gh run`
monitoring is enabled, provide workflow evidence and stop for operator decision
if the command returns `blocked`. If publishing fails, decide whether to retry,
use deterministic recovery, or abort. Do not skip a failed publishing workflow
except as explicitly approved emergency recovery with compensating evidence
recorded in the release issue.

#### 8. Verify The Release

```bash
bash scripts/release/verify.sh --version <version> --source-sha <promoted-source-sha>
```

Verification must confirm:

- private GitHub release exists on `eddacraft/anvil-001`
- public GitHub release exists on `eddacraft/anvil`
- expected cargo-dist assets are present
- provenance names the build source SHA and workflow run
- a sanitised `release-evidence-<tag>.md` is published as a public release asset
  and committed to `eddacraft/anvil` under `releases/` (CIB-034; rendered
  automatically from the provenance manifest by
  `scripts/release/generate-evidence.sh`)
- Homebrew, Scoop, and WinGet publication state is recorded
- `https://install.eddacraft.ai` returns HTTP 200

Until live host and publisher checks are enabled, `verify.sh` blocks and
requires explicit verification evidence rather than inferring release state from
prose.

### 9. Approve Comms

If `verify.sh` produces a release announcement draft, approve or edit it before
posting.

Suggested minimum message:

```text
Anvil CLI <version> is live.

Install:
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh \
  | sh
```

#### 10. Close Out

```bash
bash scripts/release/closeout.sh \
  --version <version> \
  --tag <version> \
  --source-sha <promoted-source-sha> \
  --verification-record <verification-url> \
  --verification-passed
```

This owns:

- release branch cleanup (when `stabilisation` strategy ran)
- final release issue update
- release issue closure

There is no back-merge step — `main` is the single integration target.

#### 11. Reconcile APS counts

`closeout.sh` does not touch APS state. Do this as a separate step.

Per-module `N/M` progress counts are advisory-derived (ADR-053): feature PRs
flip only per-item `Status:` lines and deliberately do **not** update the
aggregate count, so that concurrent PRs on a shared module do not collide on one
cell. The reconcile is a separate single-writer pass, and nothing schedules it
(**CIB-297**) — so by tag time the stored counts are usually stale, and any
module archived during closeout freezes whatever number it happened to hold.

Run the reconcile on a bookkeeping branch, never on `main`:

```bash
# from a worktree on a chore/aps-* branch, not the main checkout
pnpm aps:index                     # single-writer reconcile; writes by default
node scripts/aps/drift-check.mjs   # must report: No drift warnings detected.
```

Then open a `chore/aps-*` PR. Shared multi-writer modules (CIB today) collide on
the count cell, so arm auto-merge with rebase rather than merging by hand:

```bash
gh pr merge <pr> --auto --rebase --delete-branch
```

If a sibling lands first, rebase and re-run `pnpm aps:index` — take the incoming
side of the count cell and recompute, rather than keeping your own number, or
you will silently drop the sibling's intake prose from the index row.

Note that `pnpm aps:index:check` reports staleness but **exits 0** by design
(ADR-053), so a green CI run is not evidence that counts are current. Use
`drift-check.mjs` as the check that actually fails.

Archiving completed modules is a separate multi-file cascade — `git mv` to
`plans/archive/modules/` plus the index path in the same change, see
[`plans/project-context.md`](../../plans/project-context.md) — and belongs in
its own PR, run **after** this reconcile so the archived counts are true.

## Daemon Routing Rollout Controls (ADR-075)

From `v0.8.0-beta`, `anvil watch --action check` routes save-time validation
through the intercept daemon **by default when a live daemon answers the
presence probe**
([ADR-075](../../plans/decisions/075-v080-graph-product-scope.md), DSV-021).
Flipping a previously opt-in persistent daemon to default-on is a rollout
problem, not only a correctness gate: ADR-075 makes a documented opt-out, a
named revert signal, and a staged rollout cut prerequisites. This section is
that procedure. It applies to every release whose tag range includes or carries
the default-on routing change.

### Pre-tag opt-out exercise

Run on a box with a live daemon (`anvil daemon` serving the workspace — the same
constraint as the DSV-021 post-merge verification). Record the four outcomes in
the release tracking issue.

1. **Default-on baseline** — with `ANVIL_WATCH_DAEMON` unset and the daemon
   live, `anvil watch --action check` routes through the daemon and
   `anvil status` shows the `Save-time:` assurance line.
2. **Opt-out** — `ANVIL_WATCH_DAEMON=0 anvil watch --action check` takes the
   subprocess-only path, and `ANVIL_WATCH_DAEMON=0 anvil status` drops the
   `Save-time:` line (pre-daemon output, unchanged). `false`/`off`/`no` are
   equivalent; matching is case-insensitive.
3. **Daemon-absent default** — stop the daemon; with the variable unset, `watch`
   keeps the quiet subprocess path with **no** `daemon-absent` warnings (the
   presence guard — non-daemon installs must see no behaviour change).
4. **Forced-on fallback** — `ANVIL_WATCH_DAEMON=1` with the daemon stopped falls
   back to the same scoped `check`, warns **once per disconnect** (the latch
   resets on reconnect), and reports save-time assurance as `unavailable`, never
   `clean`.

Any deviation is a release blocker under the normal
[failure policy](#failure-policy) — fix forward or pull the routing change out
of the cut; do not tag with a broken opt-out.

### Named revert signal

Either of the following, observed on the released build, trips the revert
decision:

- **Latency** — save-time p95 over the 80 ms
  [ADR-031](../../plans/decisions/031-validation-latency-rubric.md) budget. In
  CI this is the `Measure hot-read latency budget (GV2-025)` gate in
  `resource-budget.yml`; in the field, a reproducible report of `watch`
  save-time validation exceeding the budget counts.
- **WARN rate** — daemon-routing fallback warnings recurring in the default
  (unset) mode. The latch warns once **per disconnect** and resets on reconnect,
  so repeated WARNs within one session mean the daemon is flapping, and a
  sustained stream of routing WARNs from default-mode users means the presence
  guard or the daemon is misbehaving in the field.

### Revert procedure

1. **Per-install mitigation (immediate):** affected users set
   `ANVIL_WATCH_DAEMON=0`. This is the documented opt-out — subprocess path
   restored, no reinstall, no daemon restart. Publish the workaround in the
   release issue and channel comms.
2. **Fleet revert (patch release):** flip the unset default in
   `daemon_routing_mode_from`
   (`crates/anvil-cli/src/commands/watch_save_time.rs`) from `DefaultOnWhenLive`
   back to `Disabled` (restoring the pre-`v0.8.0-beta` opt-in posture), update
   the [configuration docs](../public/anvil/operations/config.md) to match, and
   ship through the normal patch cadence — or via
   [`emergency-hotfix.md`](emergency-hotfix.md) if the signal is P0. `=1`
   forced-on must keep working so daemon users retain the feature while the
   default reverts.
3. **Do not yank** the published release for a routing regression alone — the
   opt-out makes it recoverable per-install. Yank only if the release meets the
   [`rollback-bad-published-release.md`](rollback-bad-published-release.md)
   criteria independently.
4. **Record and track:** log the observed signal, the decision, and the evidence
   in the release tracking issue, and file an APS follow-up item for the re-flip
   criteria before default-on ships again.

### Staged rollout

Default-on routing ships to the **beta channel first**; a stable/GA release
inherits it only after the beta has soaked with no revert signal. On Windows,
default-on is additionally gated on the DSV-010b served-verb set — confirm the
`rust.yml` Windows matrix is green for the source SHA before a tag that enables
routing there.

## Strategy Guide

Use `direct` (default) when:

- `main` is already green at a SHA you want to ship
- the release diff is small
- no release hardening commits are needed

Use `stabilisation` when:

- the diff is large
- release-critical Rust, installer, auth, infra, or packaging code changed
- the release needs hardening commits in a `release/*` branch with explicit
  expiry before tag

## Failure Policy

When a command fails:

1. Stop.
2. Read the command output.
3. Choose one path: retry after fix, run command recovery mode, or abort.
4. Do not skip readiness, tag, publish, provenance, verification, or closeout
   gates as normal release procedure.
5. Do not manually perform the failed command's job unless this is emergency
   recovery approved by the operator with exact compensating evidence recorded
   in the release issue.

## Emergency Recovery

Manual recovery must be logged in the release issue with:

- what failed
- why deterministic recovery was insufficient
- exact commands run
- repos, tags, releases, or package-manager records changed
- follow-up needed to encode the recovery into `scripts/release/*`

For specific failure modes, route to the matching playbook before improvising:

- Broken `main` before any tag is pushed:
  [`docs/runbooks/rollback-bad-main.md`](../runbooks/rollback-bad-main.md).
- Bad candidate artefact from the release-readiness workflow:
  [`docs/runbooks/rollback-bad-candidate-artefact.md`](../runbooks/rollback-bad-candidate-artefact.md).
- Bad published release that needs supersession or yank:
  [`docs/runbooks/rollback-bad-published-release.md`](../runbooks/rollback-bad-published-release.md).
- Out-of-band patch release for a regression, security fix, or compliance
  window:
  [`docs/runbooks/emergency-hotfix.md`](../runbooks/emergency-hotfix.md).

## Done Definition

A release is done only when all are true:

- `verify.sh` passed
- public release assets are present
- install site returns 200
- downstream package-manager state is recorded
- comms are approved or explicitly skipped
- `closeout.sh` completed
- APS counts reconciled — `drift-check.mjs` reports no drift (step 11)
- release issue is closed

The tracking issue is an operator log only. Link live verification evidence from
the issue; do not treat the issue as release authority or shipped-state
authority.
