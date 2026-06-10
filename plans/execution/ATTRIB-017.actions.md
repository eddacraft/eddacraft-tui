# ATTRIB-017 — Versioned releases on the acknowledgements-starter mirror

## Purpose

Land ATTRIB-017: give consumers of `eddacraft/acknowledgements-starter` a
deliberate, versioned release surface (semver tags + GitHub Releases) **on top
of** the existing rolling-`main` mirror (ATTRIB-011), so they can be notified of
updates, read a changelog, and pin to an immutable version.

Design contract:
[`plans/specs/2026-06-08-acknowledgements-starter-releases.md`](../specs/2026-06-08-acknowledgements-starter-releases.md).
Direct predecessor:
[`plans/archive/execution/ATTRIB-011.steps.md`](../archive/execution/ATTRIB-011.steps.md).
Pattern source: `eddacraft-tui` release flow
([`docs/runbooks/eddacraft-tui-release.md`](../../docs/runbooks/eddacraft-tui-release.md),
[`.github/workflows/publish-eddacraft-tui.yml`](../../.github/workflows/publish-eddacraft-tui.yml)).

## Decisions (recorded on kickoff, 2026-06-08)

| Question | Decision | Rationale |
| --- | --- | --- |
| Starting version | `1.0.0` | Four drivers + bundled-binaries shipped; contract stable. (Operator directive 2026-06-08.) |
| Rolling `main` mirror | Untouched | Releases are additive; `main` stays "latest". |
| Source tag format | `acknowledgements-starter-vX.Y.Z` (prefixed) | Monorepo collision avoidance — D-TUIR-002 reasoning. |
| Mirror tag format | `vX.Y.Z` (bare) | Mirror is single-product, flat-rooted; no legacy tags to preserve. Workflow maps prefixed→bare. |
| Append-only | Mirror tag push omits `--force` | D-TUIR-009/011 immutability. |
| Version source of truth | `VERSION` + `CHANGELOG.md` in the kit; tag is the trigger | Version + notes travel into the mirror; consumers see them after `subtree pull`. |
| Auth | Reuse `MIRROR_PUSH_TOKEN` (fine-grained PAT, `Contents: write`) | Covers both tag push and Release creation; no new secret. |
| Release location | Mirror repo only, marked latest | Mirror is the user-facing repo; anvil-001 is private. |
| Cadence | Deliberate / operator-driven | Trivial commits flow to rolling `main`; releases mark meaningful changes. |

## Actions

### 1. Add the in-tree version + changelog (TDD: drift assertion first)

- **Purpose:** Establish the durable version record the release asserts against.
- **Produces:**
  - `tools/starters/acknowledgements/VERSION` — single line `1.0.0`.
  - `tools/starters/acknowledgements/CHANGELOG.md` — Keep-a-Changelog style,
    `## [1.0.0] - 2026-06-08` summarising the kit as shipped (dispatcher + Rust
    / Node / Go / Python drivers + bundled-binaries). **External-reader voice:**
    no ATTRIB-NNN ids (same de-internalisation discipline as commits
    1e314345a / 10162687a).
- **Test first:** add `tests/version-changelog-consistency.sh` to the kit's
  self-tests — asserts `VERSION` is valid semver and matches the newest
  `## [X.Y.Z]` heading in `CHANGELOG.md`. Wire it into
  `.github/workflows/acknowledgements-kit.yml`. Red before the files exist,
  green after.
- **Checkpoint:** new self-test green locally; `markdownlint` clean on the
  CHANGELOG.

### 2. Author the release workflow

- **Purpose:** Reproducible tag-triggered split → append-only mirror tag → cut
  the GitHub Release.
- **Produces:** `.github/workflows/release-acknowledgements-starter.yml`
  - Triggers: `push` of tags `acknowledgements-starter-v*`; `workflow_dispatch`
    with a `tag` input for backfill.
  - Steps:
    1. **Ref guard** — tagged commit reachable from `origin/main`
       (`git merge-base --is-ancestor`), else refuse.
    2. `actions/checkout@v4`, `fetch-depth: 0`, `persist-credentials: false`.
    3. **Version-triple assertion** — parse `X.Y.Z` from the prefixed source
       tag (`acknowledgements-starter-vX.Y.Z`), then assert
       `X.Y.Z` == `VERSION` == the `## [X.Y.Z]` `CHANGELOG.md` heading;
       fail-fast with an actionable message.
    4. **Sanity-check `MIRROR_PUSH_TOKEN`** non-empty (reuse the mirror
       workflow's guard).
    5. **Re-assert kit health** — `bash tools/starters/acknowledgements/expand-licences.sh --check`.
    6. **README swap** on a throwaway commit (`MIRROR-README.md` →
       `README.md`), identical to the mirror workflow.
    7. `git subtree split --prefix=tools/starters/acknowledgements -b _release_split`.
    8. **Append-only tag push** — map prefix→bare, push
       `_release_split:refs/tags/vX.Y.Z` **without** `--force` via
       `http.extraheader` Basic auth (no URL-embedded token).
    9. **Extract notes** — the `## [X.Y.Z]` section of `CHANGELOG.md` to a temp
       file.
    10. `gh release create vX.Y.Z --repo eddacraft/acknowledgements-starter
        --target <split-sha> --title vX.Y.Z --notes-file <notes> --latest`
        (auth via `GH_TOKEN=${MIRROR_PUSH_TOKEN}`).
    11. Step summary: source tag, mirror tag, Release URL, verify commands.
  - `permissions: contents: read`; concurrency group
    `release-acknowledgements-starter` (no cancel-in-progress).
  - Add to the Workflow Contract Map in `.github/workflows/README.md`
    (auxiliary contract, alongside the mirror entry);
    `scripts/ci/workflow-contracts.test.sh` enforces it.
- **Checkpoint:** `actionlint` clean; the contract test passes; a dry-run on a
  feature branch via `workflow_dispatch` reaches the push step (or a `--dry-run`
  guard short-circuits before the live push).

### 3. Pre-flight the mirror for tag/release creation (operator step)

- **Purpose:** Confirm the mirror won't reject tag creation or Release writes.
  Surfaces in the PR; not done autonomously (touches the public repo's
  protection config).
- **Produces:** evidence that:
  - No **tag protection** rule blocks `MIRROR_PUSH_TOKEN` from creating
    `v*` tags (probe per the tui runbook gotcha #5; add an exception if needed).
  - The PAT can create Releases (`Contents: write` confirmed; expiry still
    valid — renew if near horizon).
- **Checkpoint:** documented probe output in the PR; runbook records the
  procedure.

### 4. Write the operator runbook

- **Purpose:** Make the cut repeatable by any operator.
- **Produces:** `docs/runbooks/acknowledgements-starter-release.md` — cut
  procedure (version-bump PR → tag merge commit → watch → verify), the
  tag-protection pre-flight, verification commands, rollback (cut a corrected
  `vX.Y.Z+1`; mark a bad Release not-latest), and PAT rotation.
- **Checkpoint:** `docs:check` clean (governance + Upstream/Downstream tables
  per the docs authoring rules); referenced from the kit README's Mirror
  section and `docs/runbooks/` index.

### 5. First release (operator step — v1.0.0)

- **Purpose:** Validate the workflow end-to-end against the live mirror.
- **Produces:** tag `acknowledgements-starter-v1.0.0` on anvil-001 `main`;
  bare `v1.0.0` tag + GitHub Release on `eddacraft/acknowledgements-starter`,
  marked latest, notes from the CHANGELOG.
- **Checkpoint:**
  - `gh release view v1.0.0 --repo eddacraft/acknowledgements-starter` shows
    the Release marked latest.
  - `gh api repos/eddacraft/acknowledgements-starter/git/refs/tags/v1.0.0`
    resolves.
  - `git subtree add … v1.0.0 --squash` into a scratch repo produces the kit
    tree (round-trip proof).

### 6. Document the release surface in the kit README

- **Purpose:** Tell consumers (and the next agent) that pinned releases exist.
- **Produces:** extend the Mirror/adoption section of
  `tools/starters/acknowledgements/MIRROR-README.md` (and the in-tree
  `README.md` Mirror note) with the `git subtree add … v1.0.0` pinning example
  and a "watch releases for updates" pointer.
- **Checkpoint:** `markdownlint` clean; the mirrored README reads naturally for
  an external consumer.

### 7. Mark ATTRIB-017 Complete; update module + index

- **Purpose:** Close the work item.
- **Produces:** status flip in the chosen module home (see below) and the
  matching `plans/index.aps.md` done/total bump.
- **Checkpoint:** `pnpm aps:active-lint` + `aps:index:check` + `aps:drift`
  clean.

## Module home (decide before activating)

`attribution-pipeline-v3` is archived Complete. Per the spec's open question,
preferred home is a **new Proposed module** `acknowledgements-starter-releases`
(retains ATTRIB lineage, room for follow-ons). Do **not** un-archive v3.
Whatever the choice, ATTRIB-017 keeps its id. This steps file does not mutate
`index.aps.md` — that edit lands when the item is activated.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Mirror tag protection blocks the PAT | Medium | Action 3 pre-flight probe + exception before first cut (tui runbook #5). |
| Bad version cut and pinned by a consumer | Medium | Append-only — cut a corrected `vX.Y.Z+1`, mark the bad Release not-latest. |
| Tag/tree version disagreement | Low | Version-triple assertion (Action 2, step 3) fails before any mirror write. |
| Release notes leak internal ids | Low | CHANGELOG authored in external voice (Action 1); review gate. |
| Operator over-releases trivial changes | Low | Deliberate cadence documented in the runbook; rolling `main` already covers "every change". |
