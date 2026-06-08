<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# TUI Reintegration

| ID   | Owner      | Status      | Progress |
| ---- | ---------- | ----------- | -------- |
| TUIR | joshuaboys | Done | 10/10     |

**Last reviewed:** 2026-06-08 — TUIR-008 `Done` by operator evidence:
canonical publishes from Anvil are proven, the mirror is healthy and
TUIMIRROR is archived, the legacy mirror `CARGO_REGISTRY_TOKEN` has been
revoked, and the private `eddacraft/eddacraft-skills` `[patch.crates-io]`
consumer check passed. This closes the final TUIR work item and advances
Progress **9/10 → 10/10**. Prior: 2026-06-08 — TUIR-010 `Merged 2026-06-08` via PR #2392:
deploys the `mirror-drift-check.yml` watchdog D-TUIR-018
specified and the mirror workflow flagged as not-yet-deployed.
Replicates the push-side subtree-split + banner-swap inline
(standalone-replication decision), diffs the reconstructed tree against
the public mirror, and opens/refreshes a labelled tracked issue on any
drift. Algorithm verified locally against the live mirror (reconstructed
tree byte-identical to `eddacraft/eddacraft-tui:main`; simulated
out-of-band push flagged) and Council-hardened against the
propagation-race false positive. Unblocks TUIR-008's drift-check
validation line and starts TUIN's 7-consecutive-green-runs gate.
Progress **8/10 → 9/10**. TUIR-008 execution-token remains `open`.
Prior: 2026-06-07 — TUIR-009 `Merged 2026-06-07` via PR #2339 
(squash at `817b359b1`): ratifies D-TUIR-021 and closes the
structural gap surfaced while backfilling `eddacraft-tui-v0.2.4` on
the public mirror. The publish workflow's `gh release create` step
on anvil-001 had no `--prerelease`, so the crate release pinned as
the anvil-001 `latest` and shadowed Anvil product releases; the
mirror itself never received a Release object, so its `/releases`
page stayed pinned at the legacy `v0.2.2`. Both gaps were closed
manually on 2026-06-07 (operational, see TUIR-009 body) and the
structural fix landed in PR #2339: step 8 of
`publish-eddacraft-tui.yml` now uses `--prerelease`; the runbook
gains a "Mirror Release backfill" recovery subsection plus a
`gh api …/releases/latest` + `target_commitish` + `prerelease` +
CHANGELOG/README byte-diff verify step; the README banner offset
in the verify step is derived from
`wc -l < MIRROR-README.md` (Copilot review on PR #2339) so the
offset survives banner growth. Progress **7/9 → 8/9** (TUIR-008
execution-token still `open`). Earlier: 2026-05-27 — TUIR-008 Ready Checklist closed out
(docs-only): the three outstanding prep items (cutover/history-rewrite
runbook section + `pre-canonical-archive` preservation, two-layer
migration rollback, `deny.toml` review) are now documented in
`docs/runbooks/eddacraft-tui-release.md`. Progress at that point was
**7/8** — TUIR-008's body is the operator-driven E2E cut (live
`eddacraft-tui-v0.2.3` publish, mirror tag propagation, legacy token
revocation, downstream consumer check against the private
`eddacraft/eddacraft-skills`), which is irreversible/outward-facing and
is **not** done by this PR. Drafting the cutover section surfaced a live
gap: the content force-push already ran (2026-05-25) without the
D-TUIR-010 archive — see the checklist note and the runbook's
corrective step. Earlier: 2026-05-25 — TUIR-005 publish workflow +
release runbook landed via PR #1919 (6/8 → 7/8); the
`mirror-eddacraft-tui.yml` workflow (TUIR-004) migrated to the
`eddacraft-mirror-bot` GitHub App so both adjacent mirror auth paths
converge on the org-owned credential.

> **Execution gate:** Implements ADR-047 (Accepted 2026-05-22). Module
> is In Progress. TUIR-001 (baseline capture) is `Done` — baseline
> spec landed at
> [`plans/specs/2026-05-22-tui-reintegration-baseline.md`](../specs/2026-05-22-tui-reintegration-baseline.md)
> with the byte-exact published-tarball file list at
> [`plans/specs/2026-05-22-tui-reintegration-baseline/package-list.txt`](../specs/2026-05-22-tui-reintegration-baseline/package-list.txt).
> Subsequent tasks (TUIR-002..TUIR-008) keep `Status: open` and become
> executable as their own prerequisites land (mirror PAT for TUIR-004,
> crates.io token for TUIR-005, MIRROR-README + docs/policies drafts
> for TUIR-007, runbook + pre-canonical-archive plan for TUIR-008 — see
> Ready Checklist). Per `plans/aps-rules.md`, the canonical
> execution-token vocabulary is `open` / `locked` / `completed` /
> `cancelled`; the parser is lenient and normalises planning-vocabulary
> words (`Done`, `In Progress`, `Ready`, etc.) onto the same tokens —
> `Done` → `completed`. TUIR-001's `Status: Done` reflects that
> normalisation (drift-check accepts it; the underlying execution token
> is still `completed`).
>
> **Supersedes:** [`eddacraft-tui-canonical-source`](../archive/modules/eddacraft-tui-canonical-source.aps.md)
> (TUIMIRROR, 0/8, Superseded — archived 2026-06-08 via TUIR-008). TUIR
> carried the same intent at a higher resolution — the policy questions left
> implicit in TUIMIRROR (read-only vs release mirror, sync direction,
> versioning ownership, CI gate split, backport policy) became first-class
> work items here.

## Purpose

Bring `eddacraft-tui` back into the Anvil monorepo as the canonical source,
keep `eddacraft/eddacraft-tui` as a read-only public mirror, and continue
publishing the crate to crates.io as the supported external distribution
channel.

**Why:** Anvil is the load-bearing consumer of `eddacraft-tui`. The current
"Anvil consumes crates.io" topology forces a release-bump-and-re-verify loop
for every widget, theme, or snapshot change driven by Anvil's own TUI
surfaces. The acknowledgements starter (ATTRIB-011, shipped 2026-05-18) has
already proven the inverse topology: canonical source in `anvil-001`, public
mirror via a least-privilege workflow, downstream consumers via the mirror.
TUIR applies that pattern to `eddacraft-tui` while preserving the public
trust surface and crates.io contract.

## In Scope

- Decide and record where the crate lives in the monorepo.
- Decide whether `eddacraft/eddacraft-tui` becomes a pure read-only mirror or
  a release-tag mirror, and pin the consequences.
- Define the sync direction, automation, and credentials.
- Pin the crates.io publish source (which tree, which workflow, what
  triggers a publish).
- Assign versioning and changelog ownership for the published crate.
- Define the CI gate split between Anvil and the public mirror.
- Define how Anvil consumes the workspace path crate while external users
  consume the crates.io release.
- Define the backport / mirror conflict policy (drift, drive-by PRs,
  emergency fixes on the mirror side).

## Out of Scope

- Redesigning `eddacraft-tui` widgets, themes, or Anvil TUI surfaces.
- Changing the published public API. Migration is byte-equivalent at
  the API surface; if relocation appears to force a public API change,
  halt and surface as a separate decision.
- Changing `anvil-plan-spec` or `kindling` source-topology policy.
- Accepting direct source PRs into the public mirror after migration.
- Releasing a new Anvil product version solely because this migration
  lands.
- Re-vendoring `eddacraft-tui` privately or sunsetting the public crate.
- CLI fallback helpers, terminal lifecycle ownership migration, or any
  new feature work on `eddacraft-tui` that isn't required for the
  migration itself. These are forward-looking design questions, not
  source-of-truth questions, and pre-empting them inside this module
  expands blast radius for no migration benefit.
- Adding `clap` (or any argument-parser) as a dependency of
  `eddacraft-tui` core. If a future helper needs argument parsing it
  must live behind an opt-in feature flag or in a separate crate; this
  module does not authorise the dependency.

## Interfaces

**Depends on:**

- [ADR-047](../decisions/047-eddacraft-tui-canonical-source-mirror.md) —
  canonical-source + public-mirror decision; pending acceptance.
- ATTRIB-011 mirror precedent —
  `.github/workflows/mirror-acknowledgements-starter.yml` and the
  least-privilege PAT pattern (`http.extraheader` Basic auth, not
  URL-embedded `x-access-token:${TOKEN}@github.com` — the embedded form
  is brittle to stray bytes in the secret and fails as
  `CURLE_URL_MALFORMAT`).
- `eddacraft/eddacraft-tui` — current public source repo and crates.io
  package owner.
- `crates/anvil-tui/` — primary in-repo consumer.
- `Cargo.toml` / `Cargo.lock` workspace dependency surface.
- crates.io — external package distribution channel.

**Exposes:**

- `crates/eddacraft-tui/` — canonical workspace crate location,
  including preserved `LICENSE` (Apache-2.0) from the public source.
  Standalone repo does not currently ship a `NOTICE`; none is created
  by the migration.
- `crates/eddacraft-tui/MIRROR-README.md` — mirror banner header,
  prepended onto `README.md` by the mirror workflow before push
  (ATTRIB-011 pattern; the banner file itself is removed from the
  mirrored tree post-prepend).
- `.github/workflows/mirror-eddacraft-tui.yml` — mirror automation
  (mirrors `crates/eddacraft-tui/` subtree to
  `eddacraft/eddacraft-tui:main`).
- `.github/workflows/publish-eddacraft-tui.yml` — crates.io publish
  automation, triggered by `eddacraft-tui-v*` tags on `anvil-001`.
- Documented external consumption contract (crates.io tag, not git
  `main`).
- Documented crate release runbook
  (`docs/runbooks/eddacraft-tui-release.md`, new).
- Documented backport / mirror conflict policy
  (`docs/policies/eddacraft-tui-mirror.md`, new).

## Decisions

**D-TUIR-001:** Canonical source location

- **Resolution:** Anvil owns the canonical source at
  `crates/eddacraft-tui/`. Sits alongside other first-party Rust crates
  (`crates/anvil-tui/`, `crates/anvil-l4/`, etc.) and is a workspace
  member. The public repository mirrors that subtree and is not
  independently edited.
- **Status:** Proposed by ADR-047.

**D-TUIR-002:** Public repo role — read-only mirror with release tags

- **Resolution:** `eddacraft/eddacraft-tui:main` is mirror-managed and
  force-pushed by automation. Release tags follow the prefixed form
  `eddacraft-tui-vX.Y.Z` everywhere — Anvil canonical source, mirror,
  and crates.io. The prefixed form is mandatory because Anvil ships
  other crates with independent semver from the same monorepo; an
  unprefixed `vX.Y.Z` would collide with Anvil product tags. Release
  tags are protected by branch/tag rules and are NEVER rewritten by
  the mirror job. The repo is therefore a hybrid: read-only on `main`,
  append-only on `eddacraft-tui-v*` tags. Issues remain open; source
  PRs are closed with a redirect template.
- **Status:** Proposed.

**D-TUIR-003:** Sync direction

- **Resolution:** One-way Anvil → public mirror. There is no reverse sync.
  Drive-by PRs against the mirror are closed with a redirect; if the
  change is accepted, a maintainer ports it into `crates/eddacraft-tui/`
  inside Anvil and the next mirror run carries it out.
- **Status:** Proposed.

**D-TUIR-004:** Sync automation

- **Resolution:** A GitHub Actions workflow (`mirror-eddacraft-tui.yml`)
  in `anvil-001` watches `crates/eddacraft-tui/**` on `main` and
  force-pushes the subtree to `eddacraft/eddacraft-tui:main` on every
  change. Subtree extraction uses `git subtree split
  --prefix=crates/eddacraft-tui -b _mirror_split` followed by force-push
  of `_mirror_split:main`, matching the ATTRIB-011
  `.github/workflows/mirror-acknowledgements-starter.yml` pattern. The
  mirror is **flat-rooted** — the crate's files sit at the public repo's
  top level, not wrapped under `crates/eddacraft-tui/`. Auth uses a
  fine-scoped PAT via `http.extraheader` Basic auth (NOT URL-embedded
  `x-access-token:${TOKEN}@github.com`, which is brittle to stray bytes
  in the secret and fails as `CURLE_URL_MALFORMAT`), scoped to
  `eddacraft/eddacraft-tui` only, stored as the repo secret
  `EDDACRAFT_TUI_MIRROR_PUSH_TOKEN`. Manual `workflow_dispatch` is
  supported for catch-up; the workflow guards against dispatch from any
  ref other than `refs/heads/main`.
  **Runner constraint:** `git subtree split --prefix=` with a
  multi-segment prefix is broken under uutils coreutils 0.2.2's
  `dirname` (returns `a/b` for `a/b/c/.` instead of `a/b/c`). GitHub
  Actions `ubuntu-latest` ships GNU coreutils and is unaffected. Local
  reproduction on uutils boxes requires the `/tmp/gnu-shim/dirname`
  PATH shim; the runbook (TUIR-005 deliverable) documents this.
- **Status:** Proposed.

**D-TUIR-005:** crates.io publish source

- **Resolution:** Crates are published from the Anvil canonical source
  (`crates/eddacraft-tui/` on `main`), not from the public mirror. The
  publish workflow tags `eddacraft-tui-vX.Y.Z` on `anvil-001`, runs
  `cargo publish` from the workspace crate, and the mirror job propagates
  the tag to `eddacraft/eddacraft-tui` as an append-only tag (no
  rewrite). The public mirror's `main` will track the same tree but
  publishing never originates from the mirror.
  **First post-migration version:** the next published version is
  `0.2.3` (continues the existing crates.io series; no version reset and
  no semver bump from the migration alone — D-TUIR-006 forbids
  migration-driven version bumps).
  **crates.io token transfer:** a new least-privilege token scoped to
  the `eddacraft-tui` crate (publish-only) is generated by the current
  crate owner and stored as repo secret
  `CRATES_IO_EDDACRAFT_TUI_TOKEN` in `anvil-001`. The previous token
  used by any publish workflow on the public mirror is revoked after
  the first successful publish from canonical source (TUIR-008
  verifies). Crate ownership on crates.io is unchanged — only the
  publish surface moves.
- **Status:** Proposed.

**D-TUIR-006:** Versioning and changelog ownership

- **Resolution:** `eddacraft-tui` keeps independent semver from Anvil
  product releases. `crates/eddacraft-tui/CHANGELOG.md` is the canonical
  changelog; entries are written in Anvil PRs. An Anvil product release
  MUST NOT bump the crate version automatically; a crate release MUST NOT
  imply an Anvil product release. Version bumps land in their own PR
  with `releaseIntent: candidate` on the crate side and `none` on the
  Anvil product side.
- **Status:** Proposed.

**D-TUIR-007:** CI gate split

- **Resolution:**
  - **Anvil side (`anvil-001`)** — authoritative gates on PRs touching
    `crates/eddacraft-tui/**`:
    - `cargo fmt --all --check` (workspace clippy with `-D warnings`
      does NOT run rustfmt, so tests + clippy can be green while CI's
      Format check fails).
    - `cargo clippy --workspace --all-targets -- -D warnings`
      (per-crate `-p` invocations miss doc-markdown errors in sibling
      crates).
    - `cargo test -p eddacraft-tui --all-features` and `cargo test -p
      eddacraft-tui --no-default-features` (catch feature-flag
      regressions in both directions).
    - `cargo test --workspace` (consumer-side regression catch).
    - `cargo check -p eddacraft-tui --release` (catches release-only
      compile failures: items defined behind
      `#[cfg(debug_assertions)]` that are referenced from
      `debug_assert!` expansions break release builds because
      `debug_assert!` skips _evaluation_ in release but name
      resolution still runs unconditionally — so the dead branch
      type-checks against an item that no longer exists. The
      standalone repo's `ci.yml` enforces this after
      [eddacraft/eddacraft-tui#29](https://github.com/eddacraft/eddacraft-tui/issues/29)
      hit 0.2.0 / 0.2.1 — a cfg-gated `ids_are_unique` helper was
      called inside `debug_assert!`, producing
      `error[E0425]: cannot find function` for every release
      consumer).
    - `cargo doc --no-deps -p eddacraft-tui` (docs.rs build proxy).
    - `cargo deny check` (re-uses workspace `deny.toml`).
    - `cargo publish --dry-run --allow-dirty -p eddacraft-tui` as a
      tarball gate on tag pushes and on Ready-for-publish PRs.
    - `cargo package --list -p eddacraft-tui` diff against the baseline
      captured in TUIR-001 — fails CI if the published file set drifts
      unexpectedly (catches "we shipped an Anvil-only file" and "we
      stopped shipping a README" in one check).
    - Snapshot tests: `INSTA_UPDATE=no` enforced in CI; updates land
      via `cargo insta review` in PR.
  - **Public mirror side:** retain only `cargo test` and `cargo publish
    --dry-run --all-features` as a smoke gate against a fresh checkout
    of the mirrored tree, plus the mirror-drift check from D-TUIR-018.
    The mirror does not re-run the full Anvil matrix; that would be
    theatre.
- **Status:** Proposed.

**D-TUIR-008:** Consumption contract

- **Resolution:**
  - **Anvil (internal):** `crates/anvil-tui/Cargo.toml` and other
    consumers resolve `eddacraft-tui` via `path =
    "../eddacraft-tui"` workspace inheritance, NOT crates.io. Local
    development gets atomic widget + consumer changes for free.
  - **External users:** depend on the crates.io release
    (`eddacraft-tui = "X.Y"`), NOT the public git `main`. Git `main`
    is explicitly documented as mirror-managed and rewritable.
- **Status:** Proposed.

**D-TUIR-009:** Backport / mirror conflict policy

- **Resolution:**
  - **Drive-by PR on the mirror:** auto-closed with a template pointing
    at the canonical source path and the contribution guide. Maintainer
    discretion to port the change.
  - **Mirror force-push collision (someone pushed to the mirror
    directly):** the next scheduled mirror run wins; lost work is the
    pusher's responsibility, documented up front. Public README warns
    against direct pushes.
  - **Emergency security fix:** lands in `anvil-001` first, mirror
    propagates within one workflow run, crates.io publish follows the
    standard publish workflow. No "fix on mirror first" path is
    sanctioned.
  - **Tag conflict (someone tagged on the mirror):** mirror automation
    refuses to overwrite existing tags; the operator resolves manually
    and the conflicting tag is either renamed or yanked from crates.io.
- **Status:** Accepted 2026-05-23 — ratified by TUIR-007 (PR #1886) via
  [`docs/policies/eddacraft-tui-mirror.md` §Backport and Conflict
  Policy](../../docs/policies/eddacraft-tui-mirror.md#backport-and-conflict-policy).
  The policy doc expands each bullet above into a sub-section and adds
  an ongoing-mirror-history-rewrite sub-policy (per D-TUIR-020) that
  belongs alongside the four conflict cases originally listed here.

**D-TUIR-020:** Ongoing mirror history rewrite contract

- **Resolution:** The mirror's `main` is force-pushed by automation on
  every change to `crates/eddacraft-tui/**` on Anvil's `main`. External
  git consumers tracking the mirror's `main` will see history rewrites
  on every sync — that is intended behaviour and is distinct from the
  one-shot cutover handled by D-TUIR-010. The public README banner
  (per D-TUIR-012) warns external consumers to depend on the crates.io
  release or on a tag, not on `main`. ADR-050's runner-helper consumers
  follow the same guidance via the documented dependency form. No
  response is required when the rewrite happens; this decision exists
  so the policy doc's "Mirror history rewrite" sub-section has a
  numbered decision to ratify rather than being slipped into D-TUIR-009.
- **Status:** Accepted 2026-05-23 — ratified by TUIR-007 (PR #1886).

**D-TUIR-010:** Existing public history at cutover

- **Resolution:** Accept the history rewrite. The first mirror push
  after canonical import force-replaces `eddacraft/eddacraft-tui:main`
  with the `git subtree split` output of `crates/eddacraft-tui/`. The
  pre-cutover public history is preserved as a one-off archive branch
  `pre-canonical-archive` on the public mirror, pushed once during
  TUIR-008 and never updated. No attempt is made to graft, replay, or
  `--rejoin` the old history onto the new subtree-split history; the
  archive branch is the durable record. This matches the ATTRIB-011
  precedent and preserves auditability without paying ongoing
  complexity cost.
- **Status:** Proposed.

**D-TUIR-011:** Existing public tag handling

- **Resolution:** Existing unprefixed `v0.x.y` tags on
  `eddacraft/eddacraft-tui` (e.g. `v0.2.2`) are left in place on the
  mirror, untouched. They continue to point at the pre-cutover commits
  reachable via `pre-canonical-archive` (D-TUIR-010). All NEW tags from
  cutover onwards use the prefixed `eddacraft-tui-vX.Y.Z` form
  (D-TUIR-002), pushed by the publish workflow. The mirror workflow
  refuses to delete or move existing tags. crates.io versions are
  unaffected — the old tags' commits are still the source for already-
  published `0.x.y` artifacts.
- **Status:** Proposed.

**D-TUIR-012:** Mirror README banner mechanism

- **Resolution:** `crates/eddacraft-tui/MIRROR-README.md` contains a
  banner that explains the mirror model, points at `anvil-001` as
  canonical, and directs contributions to the Anvil issue queue.
  Before force-push, the mirror workflow concatenates
  `MIRROR-README.md` and `README.md` (in that order) into the new
  `README.md`, then removes `MIRROR-README.md` from the staged tree.
  This matches the ATTRIB-011 prepend pattern verbatim. The canonical
  `README.md` inside `anvil-001` stays focused on the crate; the
  public framing is owned entirely by the banner file.
- **Status:** Proposed.

**D-TUIR-013:** Crate `Cargo.toml` metadata fields

- **Resolution:** `crates/eddacraft-tui/Cargo.toml` keeps
  `repository = "https://github.com/eddacraft/eddacraft-tui"` so
  crates.io click-through lands on the public visible source, not on
  private `anvil-001`. `homepage` matches. `documentation` continues
  to point at docs.rs. The `description`, `license`, and `keywords`
  fields are preserved verbatim from the pre-migration crate.
  `package.metadata.docs.rs` (if present in the standalone crate) is
  preserved verbatim so docs.rs build configuration carries over. No
  metadata fields are mutated by the migration itself.
- **Status:** Proposed.

**D-TUIR-014:** Dependency boundaries

- **Resolution:** `eddacraft-tui` has zero dependencies on
  Anvil-internal crates (`anvil-*`, `eddacraft-anvil-*`,
  `anvil-plan-spec`, `kindling`, etc.). It may depend only on
  external crates already in the standalone crate's `Cargo.toml`,
  plus tightly justified additions that would also make sense for an
  external consumer building from crates.io. A CI guard runs
  `cargo tree -p eddacraft-tui --prefix=none --no-default-features
  --edges normal | grep -E '^(anvil-|eddacraft-anvil-|kindling)'`
  and fails if any match. This guard runs in the Anvil-side gates per
  D-TUIR-007.
- **Status:** Proposed.

**D-TUIR-015:** MSRV

- **Resolution:** Preserve `eddacraft-tui`'s declared `rust-version`
  verbatim from the standalone crate. Do NOT inherit Anvil's
  workspace MSRV (if/when one is declared). The crate's MSRV is its
  contract with crates.io consumers; migrating canonical location is
  not a license to raise it silently. Any future MSRV bump for the
  crate lands in its own PR with a CHANGELOG entry and explicit
  approval; an Anvil-side toolchain bump never implies a crate MSRV
  bump.
- **Status:** Proposed.

**D-TUIR-016:** Workspace lint inheritance

- **Resolution:** `crates/eddacraft-tui/Cargo.toml` does NOT use
  `lints.workspace = true`. The crate carries its own `[lints]` block
  verbatim from the standalone repo (or an empty block if the
  standalone crate has none). Reason: Anvil's workspace lints
  (`unsafe_code = "forbid"`, `clippy::pedantic = warn`, etc.) are
  Anvil-shape, and `cargo publish` inlines workspace-inherited values
  into the published `Cargo.toml` — letting Anvil's lint posture
  silently change the published crate's build behaviour. Crate
  develops against its own lints; sibling Anvil crates keep
  inheriting the workspace lints. The workspace clippy job still
  covers the crate via `--workspace`, but the crate's `[lints]` block
  is what governs lint-driven build failures.
- **Status:** Proposed.

**D-TUIR-017:** Snapshot tests (insta)

- **Resolution:** `crates/eddacraft-tui/src/snapshots/` (insta's
  default colocation, matching the standalone repo's layout) and any
  `*.snap.new` policy carry over verbatim. CI runs with
  `INSTA_UPDATE=no` so unreviewed snapshot drift fails the build.
  Snapshot updates land via `cargo insta review` in PR, not via
  blind regeneration. The Ratatui version pin in `eddacraft-tui`
  Cargo.toml is the load-bearing input for snapshot stability;
  bumping Ratatui requires a snapshot review pass documented in the
  PR description.
- **Status:** Proposed.

**D-TUIR-018:** Mirror drift verification

- **Resolution:** A scheduled GitHub Actions job
  (`mirror-drift-check.yml`) runs daily and on workflow_dispatch.
  It clones both `anvil-001` and `eddacraft/eddacraft-tui`, runs
  `git subtree split --prefix=crates/eddacraft-tui` on the Anvil
  side, applies the same banner swap as the mirror workflow
  (D-TUIR-012), and diffs the resulting tree against the mirror's
  `main`. Any non-empty diff fails the job and opens (or refreshes)
  a tracked issue. This catches: a mirror force-push collision
  (D-TUIR-009), a missed `paths:` trigger on the mirror workflow,
  and silent banner-swap-step regressions.
- **Status:** Accepted — implemented by TUIR-010
  (`.github/workflows/mirror-drift-check.yml`). The job replicates the
  push-side transform inline rather than sharing a script (TUIR-010
  standalone-replication decision); both workflows carry a
  cross-reference comment so the transforms stay aligned.

**D-TUIR-019:** Workspace clippy `-D warnings` × verbatim
`clippy::pedantic = "warn"` collision (resolved by in-crate fixes)

- **Resolution:** Surfaced at TUIR-002 import time. The standalone
  crate's `[lints.clippy]` block carries `pedantic = { level = "warn",
  priority = -1 }`, preserved verbatim per D-TUIR-016. D-TUIR-007's
  Anvil-side gate adds `-- -D warnings` at the CLI, escalating those
  pedantic warnings to errors — 20 of them at v0.2.2 (`map_unwrap_or`,
  `doc_markdown`, `manual_let_else`, `format_collect`,
  `uninlined_format_args`, `too_many_lines`). Standalone CI runs
  `cargo clippy --all-targets` without `-D warnings`, so the
  standalone tree is clean at the error level it gates against, but
  not at the level Anvil gates against. The plan didn't surface this
  interaction until first contact. **Resolution adopted in TUIR-002:**
  apply mechanical clippy fixes in-crate — `map_or` for
  `map_unwrap_or` (5 sites in `src/pretext/{layout,prepare,segment}.rs`
  + `src/widgets/pretext.rs`), backtick the bare identifiers in
  `///` docs for `doc_markdown` (3 sites), `let ... else` for
  `manual_let_else` (1 site in `src/pretext/segment.rs`), a small
  `repeat_words(count, prefix)` test-only helper function added to
  `src/pretext/layout.rs`'s `mod tests` that uses `write!(s, ...)`
  inside a `for` loop instead of `(0..n).map(|i| format!(...)).collect()`
  to resolve `format_collect` at 4 test-input sites, `{var}` form for
  `uninlined_format_args` (~5 sites), and a single
  `#[allow(clippy::too_many_lines)]` on the deliberately long
  internal `layout_with_cap`. Net effect: no intentional public API
  changes were made — every edit is at a non-`pub` site or inside a
  `///` doc comment, and a source review of the staged diff confirms
  mechanical-only transformations. Published `0.2.3` carries the
  cleaner source. Mirror reflects the same. The `[lints]` block
  itself stays verbatim — pedantic is still `warn` for downstream
  consumers building from crates.io who don't pass `-D warnings`.
  Future migrations under this plan should expect the first run of
  D-TUIR-007's clippy gate to surface a similar delta and budget for
  mechanical fixes (not for a plan reopen).
- **Status:** Accepted in TUIR-002 (2026-05-23).

## Risks

- **Public git consumers of `main` see rewritten history.** Mitigation:
  document crates.io as the supported external consumption path; protect
  release tags so they are never rewritten; add a public README banner
  naming `main` as mirror-managed.
- **Cutover force-push destroys existing public history.** Distinct
  from ongoing rewrites (above) because this is a one-shot event that
  drops years of pre-existing public commits from `main`'s reachable
  graph. Mitigation: D-TUIR-010 preserves the pre-cutover graph as a
  durable `pre-canonical-archive` branch and D-TUIR-011 leaves the
  old `v0.x.y` tags pointing at it; pre-cutover release commits remain
  reachable via tag or branch even after `main` is rewritten. Announce
  the cutover on the mirror's README banner before the first mirror
  push.
- **`git subtree split` breaks under uutils coreutils on
  multi-segment prefixes.** Affects any local reproduction of the
  mirror workflow on uutils boxes (uutils 0.2.2 `dirname` returns
  `a/b` for `a/b/c/.`). Mitigation: D-TUIR-004 pins the workflow to
  GitHub Actions `ubuntu-latest` (GNU coreutils); the runbook lists
  the `/tmp/gnu-shim/dirname` PATH shim for local reruns. CI is the
  source of truth — local reruns are debug-only.
- **Crate release accidentally couples to Anvil product release.**
  Mitigation: D-TUIR-006 keeps independent versioning; publish workflow
  is gated on an explicit `eddacraft-tui-vX.Y.Z` tag, not on Anvil
  release tags.
- **Mirror PAT scope creep.** Mitigation: PAT is fine-scoped to
  `eddacraft/eddacraft-tui` only and stored in repo secrets; rotation
  cadence documented alongside ATTRIB-011's PAT.
- **Drift between Anvil docs and mirror docs.** Mitigation: README,
  CONTRIBUTING, and SECURITY for `eddacraft-tui` live under
  `crates/eddacraft-tui/` and are mirrored verbatim; no parallel mirror
  edits.
- **Atomicity loss if the mirror job fails mid-release.** Mitigation:
  publish workflow runs `cargo publish` only after the mirror push
  succeeds AND the tag propagation step succeeds; failure leaves a clean
  rollback point on Anvil side.
- **Issue traffic on the mirror is ignored.** Mitigation: issues remain
  open on the mirror; a triage rotation forwards relevant items to
  `anvil-001` issues. PRs are auto-closed with redirect.
- **Workspace lint posture silently mutates published build
  behaviour.** If `lints.workspace = true` slips into
  `crates/eddacraft-tui/Cargo.toml`, `cargo publish` inlines Anvil's
  forbid-unsafe + pedantic-warn config into the published crate.
  Mitigation: D-TUIR-016 forbids the inheritance; a CI grep
  (`grep -Fq 'lints.workspace' crates/eddacraft-tui/Cargo.toml` must
  exit 1) backs it up.
- **MSRV drift between Anvil and the crate.** Mitigation: D-TUIR-015
  preserves the crate's `rust-version` independently; CI matrix runs
  `cargo check -p eddacraft-tui` on the crate's declared MSRV
  toolchain, not Anvil's current pin.
- **Anvil-internal crate accidentally added as `eddacraft-tui` dep.**
  Migration brings the crate alongside dozens of Anvil-shape crates;
  natural to reach for `anvil-l4` types. Mitigation: D-TUIR-014 CI
  guard plus reviewer awareness — the crate's `Cargo.toml` is a
  red-flag file in code review.
- **Downstream consumer breakage missed at cutover.** Mitigation:
  TUIR-008 validation includes an operator-driven `cargo check`
  against internal eddacraft consumers that live in separate repos
  from `anvil-001` (notably `eddacraft/eddacraft-skills`, private),
  pulling the dry-run candidate via `[patch.crates-io]` before the
  first publish from canonical source. The validation is not in
  `anvil-001` CI because the consumer repos are not workspace
  members; the operator records the result on the TUIR-008 PR.
- **Migration blocks a release that needs to ship.** Mitigation: the
  rollback path (Ready Checklist) is the standalone repo and the
  previous publish workflow. Until TUIR-008 closes, the standalone
  repo is retained but frozen — emergency rollback un-freezes it,
  reverts Anvil consumers to `eddacraft-tui = "0.2.2"` on crates.io,
  and resumes standalone publishing. Public mirror force-push
  reversal is impossible (history is gone), but `pre-canonical-archive`
  (D-TUIR-010) keeps the pre-cutover graph reachable for forensics.

## Work Items

### TUIR-001: Lock the import baseline

- **Status:** Done

**Intent:** Record the exact public source, version, tags, release
workflow, and crates.io state that will be imported into Anvil so the
relocation has a recorded provenance.

**Outcome:** A baseline document at
`plans/specs/2026-05-22-tui-reintegration-baseline.md` capturing:
- Current `eddacraft-tui` source SHA and branch.
- Latest crates.io version (currently `0.2.2`) and full tag list.
- Public CI surface (workflow names + gates).
- Declared `rust-version` (MSRV) of the standalone crate.
- Full list of feature flags with brief descriptions (e.g. `image`,
  `big-text`, `test-utils`) — used to drive D-TUIR-007 per-feature
  validation.
- `cargo package --list -p eddacraft-tui` output captured byte-for-byte
  as the published-tarball baseline (used by the D-TUIR-007 drift gate).
- Documented downstream consumers known to depend on the crate:
  (a) Anvil's first-party consumers (`anvil-cli`, `anvil-tui`,
  `workspace-hack`); (b) internal eddacraft consumers in separate
  repos from `anvil-001` (notably `eddacraft/eddacraft-skills`,
  private), with repo URL, current `eddacraft-tui` version pin, and
  responsible maintainer recorded for TUIR-008 operator-driven
  validation.
- Standalone repo's `[lints]` block content (for verbatim carry per
  D-TUIR-016).
- Identified deltas to fold into the in-repo crate.

**Validation:** `cargo test` against the standalone repo at the recorded
SHA; `cargo publish --dry-run --all-features` succeeds against the same
tree; `cargo package --list -p eddacraft-tui` output stored under
`plans/specs/2026-05-22-tui-reintegration-baseline/package-list.txt`.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIR-002: Import `eddacraft-tui` into `crates/eddacraft-tui/`

- **Status:** Merged 2026-05-23 via PR #1875

**Intent:** Move the canonical source into the workspace without
behaviour or API change.

**Outcome:** `crates/eddacraft-tui/` contains the imported crate with
package metadata, docs, tests, feature flags (`image`, `big-text`,
`test-utils` — full list from TUIR-001 baseline), insta snapshot
files under `src/snapshots/` (insta-default colocation, matching the
standalone repo's layout), and CHANGELOG preserved. `LICENSE`
(Apache-2.0) is copied verbatim from the public source (do not
regenerate). The standalone repo does not currently ship a `NOTICE`;
do not create one absent an attribution need.
Repo-management directories at the standalone repo's root that are
NOT part of the published crate — `bin/`, `lib/`, `plans/`,
`aps-planning/`, `tools/`, `docs/` (planning, scaffold shell scripts,
ATTRIB-011 starter relocation, repo-level APS planning) — are NOT
carried into `crates/eddacraft-tui/`. The Anvil monorepo provides
its own equivalents at canonical paths. Only crate files migrate.
`Cargo.toml` keeps `repository`, `homepage`, `documentation`,
`description`, `license`, `keywords`, `rust-version`, `edition`, and
`package.metadata.docs.rs` fields verbatim per D-TUIR-013 /
D-TUIR-015. In particular,
`repository = "https://github.com/eddacraft/eddacraft-tui"` is
unchanged. The crate carries its own `[lints]` block per D-TUIR-016
(NOT `lints.workspace = true`). `MIRROR-README.md` is added at the
crate root with the mirror banner content (D-TUIR-012). Workspace
`Cargo.toml` lists the crate as a member but does NOT add it to a
workspace `default-members` list that would pull it into
`cargo run`-style implicit builds.
The empty `[workspace]` table the standalone crate's `Cargo.toml`
carries (the conventional trick that makes the crate its own
workspace root when consumed outside a parent workspace) is stripped
on import — leaving it in place would make `crates/eddacraft-tui/`
declare itself a nested workspace root and conflict with anvil-001
workspace membership. This is a structural requirement of workspace
membership, not a metadata change covered by D-TUIR-013, and does
not appear in the published `Cargo.toml.orig` (cargo strips empty
tables on publish), so it has zero downstream visibility. The
20-site clippy delta caused by D-TUIR-007's `-D warnings` × the
verbatim `clippy::pedantic = "warn"` lint policy is resolved
in-crate per D-TUIR-019; the migration is byte-equivalent at the
public API surface but no longer at the source-line level.
The upstream tarball's `docs/council-review-issues.md` (a
development-branch council session log carried into the v0.2.2
tarball by accident) is dropped on import — it has no shipped-crate
value and would be noise inside the canonical source. The published
file list therefore loses one entry on top of the deltas tracked in
the baseline; the validation `package --list` diff treats this as an
expected omission.
**Transitional consumer state:** TUIR-002 does NOT switch consumers.
The workspace dep `eddacraft-tui = "0.2.2"` in root `Cargo.toml`
stays a crates.io version requirement, and `workspace-hack` keeps
its own direct `version = "0.2"` reference. Consumers
(`eddacraft-anvil-tui`, `eddacraft-anvil`, `workspace-hack`)
continue resolving `eddacraft-tui` to the registry copy after
TUIR-002 merges; the path crate at `crates/eddacraft-tui/` is an
orphan member in `Cargo.lock` until TUIR-003. This is the documented
hand-off shape, but it means `cargo test --workspace` does NOT
exercise the path crate's source — only the per-crate gates
(`cargo test -p eddacraft-tui ...`) do. The root `Cargo.toml`
workspace dep line carries an explicit comment marking the
transitional state.

**Validation:**
- `cargo test -p eddacraft-tui --all-features`;
- `cargo test -p eddacraft-tui --no-default-features`;
- per-feature spot checks: `cargo test -p eddacraft-tui --features
  image`, `cargo test -p eddacraft-tui --features big-text`, `cargo
  test -p eddacraft-tui --features test-utils`;
- `cargo test --workspace`;
- `cargo fmt --all --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo doc --no-deps -p eddacraft-tui`;
- `cargo deny check`;
- `cargo publish --dry-run --allow-dirty -p eddacraft-tui` against
  the in-workspace crate;
- `diff <(cargo package --list -p eddacraft-tui) <(cat plans/specs/2026-05-22-tui-reintegration-baseline/package-list.txt)`
  shows no unexpected additions or omissions;
- `grep -Fq 'lints.workspace' crates/eddacraft-tui/Cargo.toml` exits 1;
- `cargo tree -p eddacraft-tui --prefix=none --no-default-features
  --edges normal | grep -E '^(anvil-|eddacraft-anvil-|kindling)'`
  returns no matches (D-TUIR-014 guard);
- `ls crates/eddacraft-tui/LICENSE crates/eddacraft-tui/MIRROR-README.md`
  returns both files.

**changeType:** internal
**releaseIntent:** hold
**holdCondition:** Hold publishing until TUIR-003 (Anvil consumes the
workspace crate) and TUIR-004 (mirror automation) are merged.
**releaseScope:** none

### TUIR-003: Switch Anvil consumers to the workspace path crate

- **Status:** Merged 2026-05-23 via PR #1879

**Intent:** Replace the crates.io dependency on `eddacraft-tui` with the
in-workspace path crate inside Anvil.

**Outcome:** `crates/anvil-tui/` (package `eddacraft-anvil-tui`),
`crates/anvil-cli/`, and any other Anvil crates that consume
`eddacraft-tui` resolve it via workspace `path =` inheritance. Root
`Cargo.toml`'s workspace dependency for `eddacraft-tui` is rewritten
from a crates.io version pin to a path dependency, and the TUIR-002
transitional-state comment on that line is removed in the same edit.
`crates/workspace-hack/Cargo.toml` is regenerated with `cargo hakari
generate`; the pre-migration `eddacraft-tui = { version = "0.2", ...
features = ["test-utils"] }` entry is dropped entirely rather than
rewritten to a path reference — hakari only aggregates external
versions across a workspace, and path crates need no aggregation, so
the entry is simply absent from the regenerated file. **Hakari
ordering is load-bearing:** the workspace dep rewrite MUST land
before `cargo hakari generate` runs, otherwise `workspace-hack`
regenerates from the still-registry workspace dep and the split-
resolution graph (most consumers on path, `workspace-hack` on
registry) persists until a follow-up regen.
The crates.io `eddacraft-tui` entry no longer appears in the
workspace `Cargo.lock` as an external dependency for first-party
crates.

**Validation:** `cargo tree -p eddacraft-anvil-tui -i eddacraft-tui`
shows the path crate; `cargo tree -p eddacraft-anvil -i eddacraft-tui`
shows the path crate; `cargo hakari verify`; `cargo test --workspace`;
`grep -F 'eddacraft-tui = "' Cargo.toml crates/*/Cargo.toml` returns
zero hits (no version pins remain on first-party crates).

**changeType:** internal
**releaseIntent:** candidate
**releaseScope:** patch

### TUIR-004: Mirror `crates/eddacraft-tui/` to the public repo

- **Status:** Merged 2026-05-24 via PR #1894

**Prerequisite:** TUIR-007 (`CONTRIBUTING.md` mirror-aware update)
MUST land before this workflow runs even once against
`eddacraft/eddacraft-tui:main`. The `MIRROR-README.md` banner points
contributors at `CONTRIBUTING.md`, and TUIR-002 left
`CONTRIBUTING.md` carrying the standalone repo's `dev`/`main`
branching language — running the mirror before TUIR-007 lands would
publish a banner whose redirect target actively misleads external
contributors. Either land TUIR-007 first, or land TUIR-004 and
TUIR-007 in the same PR.

**Intent:** Ship `.github/workflows/mirror-eddacraft-tui.yml` modelled
on `mirror-acknowledgements-starter.yml`, mirroring the subtree to
`eddacraft/eddacraft-tui:main` on every change, with tag protection
honoured.

**Outcome:** A `workflow_dispatch` + path-filtered push trigger that
extracts the subtree via `git subtree split
--prefix=crates/eddacraft-tui -b _mirror_split`, prepends
`MIRROR-README.md` onto `README.md` and removes the banner file on a
throwaway commit before the split, force-pushes `_mirror_split:main`
to `eddacraft/eddacraft-tui` using fine-scoped PAT via
`http.extraheader` Basic auth (secret name
`EDDACRAFT_TUI_MIRROR_PUSH_TOKEN`), refuses to overwrite existing
tags, guards against dispatch from any ref other than
`refs/heads/main`, and emits a run summary linking the mirrored SHA.
Workflow runs on `ubuntu-latest` (GNU coreutils — required by `git
subtree split` per D-TUIR-004 runner constraint).

**Validation:** Manual `workflow_dispatch` against `main` succeeds;
public mirror tree byte-matches `crates/eddacraft-tui/` modulo the
`README.md` banner swap and the absence of `MIRROR-README.md`;
existing `v0.x.y` release tags remain intact across the run; the
`pre-canonical-archive` branch (D-TUIR-010) is present on the mirror
and is not modified by the run.

**changeType:** internal
**releaseIntent:** never
**releaseScope:** none

### TUIR-005: Wire the crates.io publish workflow from canonical source

- **Status:** Merged 2026-05-25 via PR #1919

**Intent:** Add a publish workflow that releases `eddacraft-tui` to
crates.io from `anvil-001` canonical source, independent of Anvil
product releases.

**Outcome:** `.github/workflows/publish-eddacraft-tui.yml` triggers on
`eddacraft-tui-v[0-9]+.[0-9]+.[0-9]+` tags (prefixed semver per
D-TUIR-002), enforces a ref-reachability guard refusing tags not on
`main`, verifies the tag version matches `crates/eddacraft-tui/Cargo.toml`,
re-runs the full D-TUIR-007 publish-side gate matrix
(`fmt --all --check`, workspace clippy with `-D warnings`,
`-p eddacraft-tui` test under `--all-features` AND
`--no-default-features`, `cargo doc --no-deps` with
`RUSTDOCFLAGS=-D warnings`, `cargo deny check`, `cargo publish
--dry-run --all-features`, and a byte-diff of `cargo package --list`
against the TUIR-001 baseline at
`plans/specs/2026-05-22-tui-reintegration-baseline/package-list.txt`),
runs `cargo publish --all-features` using
`CRATES_IO_EDDACRAFT_TUI_TOKEN` (crates.io publish-only token scoped
to the `eddacraft-tui` crate), propagates the tag (append-only,
single tag refspec, no `--force`, no `--mirror`) to
`eddacraft/eddacraft-tui` via the `eddacraft-mirror-bot` GitHub App
(short-lived installation token minted at runtime via
`actions/create-github-app-token@v3.2.0`), then creates a GitHub
Release on anvil-001. `docs/runbooks/eddacraft-tui-release.md`
documents the cut, verification, rollback, and App private-key
rotation. The `mirror-eddacraft-tui.yml` workflow (TUIR-004) is
migrated to the same App in the same PR so both adjacent mirror auth
paths converge on the org-owned credential; the legacy
`EDDACRAFT_TUI_MIRROR_PUSH_TOKEN` PAT becomes dead weight and is
slated for deletion after the first successful App-auth run (runbook
documents the retirement step).

**Validation:**
- `yamllint` on both modified workflows passes (warnings inherited
  from `rust.yml` SHA-pin comment style and the `on:` truthy form);
- `bash scripts/ci/workflow-contracts.test.sh` passes after the
  README contract-map row + per-file section update;
- `node scripts/aps/drift-check.mjs` returns no drift;
- `node scripts/docs/docs-check.mjs` returns 7/7 surfaces pass;
- `oxfmt --check` clean on touched markdown;
- live publish validation deferred to TUIR-008's E2E pass (requires
  cutting a real `eddacraft-tui-v0.2.3` tag, which is an operator
  action and the closing step of the migration).

**changeType:** internal
**releaseIntent:** never
**releaseScope:** none

### TUIR-006: Split CI gates between Anvil and mirror

- **Status:** Merged 2026-05-23 via PR #1885

**Intent:** Pin which gates run where so the mirror does not duplicate
Anvil's full matrix and Anvil retains authoritative validation.

**Outcome:** Anvil side: `cargo test -p eddacraft-tui --all-features`,
`cargo test --workspace`, workspace clippy with `-D warnings`, and
`cargo fmt --all --check` are required on PRs touching
`crates/eddacraft-tui/**`. Mirror side: a minimal `cargo test` + `cargo
publish --dry-run` smoke workflow runs on every mirror push. Both gate
contracts are documented in `docs/policies/eddacraft-tui-mirror.md`.

**Validation:** `pnpm test:ci-workflow-contracts` (or successor) lists
the new workflows; doc check passes; intentional failures on each side
surface only the gate that should run there.

**changeType:** internal
**releaseIntent:** never
**releaseScope:** none

### TUIR-007: Document mirror policy and update public surfaces

- **Status:** Merged 2026-05-23 via PR #1886

**Intent:** Make the public repo accurately describe itself as a
read-only mirror, redirect contributions, and document the backport /
conflict policy.

**Outcome:** `crates/eddacraft-tui/MIRROR-README.md` carries the mirror
notice; the mirror workflow prepends it onto `README.md` at push time
(D-TUIR-012), so the public repo's `README.md` opens with the mirror
banner followed by the canonical crate README. `CONTRIBUTING.md` and
`SECURITY.md` under `crates/eddacraft-tui/` are updated in-place with
mirror-aware language (canonical source path, where issues / PRs
actually land) and mirror out verbatim — no separate public copies.
A GitHub Actions-driven PR-redirect template on
`eddacraft/eddacraft-tui` auto-closes drive-by source PRs against the
mirror with a link to the contribution guide.
`docs/policies/eddacraft-tui-mirror.md` (in Anvil) is the canonical
copy of the backport / conflict policy and is linked from the banner.

**Validation:** Public mirror docs contain the mirror notice after a
successful sync; `pnpm docs:check`; `pnpm adr:check`; targeted search
for stale "eddacraft-tui is independently canonical" claims returns
zero hits.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIR-008: End-to-end verification and TUIMIRROR retirement

- **Status:** Done
- **Intent:** Prove the in-repo crate, Anvil consumers, mirror, publish
  path, and policy docs work as one operating model, and archive the
  superseded TUIMIRROR module.
- **Expected Outcome:** A full dry-run cuts a candidate
  `eddacraft-tui-v0.2.3` tag in Anvil, the mirror propagates with the
  `pre-canonical-archive` branch (D-TUIR-010) preserved, `cargo publish
  --dry-run` succeeds, no Anvil product release is implied, the public
  mirror reflects the canonical tree (banner-swapped), and existing
  unprefixed `v0.x.y` tags remain untouched (D-TUIR-011). The previous
  crates.io publish token used by the standalone repo is identified and
  revoked after the first real publish. The standalone repo is set to
  read-only with a notice pointing at the canonical source, but not
  deleted (preserves emergency-rollback path per Risks). TUIMIRROR is
  `git mv`'d to `plans/archive/modules/` with a redirect note pointing
  at TUIR.
- **Validation:**
- `cargo test --workspace`;
- `pnpm adr:check`;
- `pnpm docs:check`;
- mirror drift check (D-TUIR-018) reports a clean tree;
- `gh api repos/eddacraft/eddacraft-tui/branches/pre-canonical-archive
  --jq .name` returns `pre-canonical-archive`;
- existing tag preservation: `gh api repos/eddacraft/eddacraft-tui/tags
  --jq '.[].name' | grep -E '^v0\.'` returns the historical tags;
- crate `cargo publish --dry-run --all-features`;
- downstream consumer check: the operator runs `cargo check` against
  internal eddacraft consumers that are separate repos from
  `anvil-001` (notably `eddacraft/eddacraft-skills`, a private
  internal repo in the eddacraft org — not a workspace member here),
  pulling the candidate via `[patch.crates-io]`. The check is
  operator-driven (not in `anvil-001` CI, since those consumer repos
  are not accessible from this workspace's CI) and the outcome is
  recorded in the TUIR-008 PR description;
- index.aps.md reflects archive.
- **Close-out progress (2026-06-08):** most deliverables are satisfied —
  the candidate path is proven by three real publishes from canonical
  source (`eddacraft-tui` 0.2.3 / 0.2.4 / 0.3.0 on crates.io);
  `pre-canonical-archive` and the legacy `v0.x.y` tags are preserved on
  the mirror; no Anvil product release is implied (TUIR-009 `--prerelease`);
  the **drift check reports a clean tree** — first live green run on `main`
  via `workflow_dispatch` (Actions run 27117186011, 2026-06-08: `Diff`
  succeeded, the propagation-lag and fail steps both skipped → drift
  `false`, no `mirror-drift` issue opened) satisfying the D-TUIR-018
  validation line; the mirror's **read-only posture** is held by the soft
  mechanism (`pr-redirect.yml` auto-close + `MIRROR-README.md` banner +
  the authoritative `docs/policies/eddacraft-tui-mirror.md`) rather than a
  branch ruleset, which the public mirror cannot use because the
  `eddacraft-mirror-bot` App force-pushes `main`; and **TUIMIRROR is now
  archived** to `plans/archive/modules/` with a redirect note + index row
  repointed. **Operator update 2026-06-08:** the legacy crates.io token
  (`CARGO_REGISTRY_TOKEN` on the `eddacraft/eddacraft-tui` repo secrets,
  set 2026-04-10) has been revoked, and the operator-driven `cargo check`
  of the private `eddacraft/eddacraft-skills` against the candidate via
  `[patch.crates-io]` passed. Status flips from `open` to `Done`.

**changeType:** internal
**releaseIntent:** candidate
**releaseScope:** patch

### TUIR-009: Mirror-side GitHub Release backfill + workflow gap

- **Status:** Merged 2026-06-07 via PR #2339 (squash at `817b359b1`)
- **Intent:** Close two structural gaps surfaced 2026-06-07 during
  v0.2.4 backfill: (a) the publish workflow
  (`.github/workflows/publish-eddacraft-tui.yml` step 8) creates
  `gh release create` on `eddacraft/anvil-001` only — the public
  mirror `eddacraft/eddacraft-tui` never gets a Release object created
  automatically, so the mirror's `/releases` page stays pinned at the
  most recent legacy `v0.x.y` Release (v0.2.2 as of 2026-06-07), and
  `…/releases/latest` does not reflect the new prefixed
  `eddacraft-tui-vX.Y.Z` releases even though the tags themselves
  propagate cleanly; and (b) the anvil-001 release is created WITHOUT
  `--prerelease`, which causes it to pin as the anvil-001 repo's
  `latest` and shadow Anvil product releases (e.g. v0.7.4-beta).
  This work item (i) documents the mirror backfill recovery path in
  the runbook so the next operator doesn't have to discover it, (ii)
  adds the missing `--prerelease` flag to the publish workflow so the
  anvil-001 release stops shadowing Anvil product releases, and (iii)
  patches the existing v0.2.3 (orphan — crate was never published to
  crates.io) and v0.2.4 anvil-001 releases with `--prerelease` so the
  v0.7.4-beta product release correctly shows as `latest`.
- **Expected Outcome:** (1) `docs/runbooks/eddacraft-tui-release.md`
  carries a "Mirror Release backfill" subsection under Rollback that
  gives the exact `gh release create` + `target_commitish` patch
  commands and explains why `--target` matters (default `main` would
  point the release at whatever `main` HEAD is at backfill time, not
  at the vX.Y.Z tag's commit). (2) The same runbook's "Verify"
  section gains a `gh api …/releases/latest` check, a
  `target_commitish` pin check, a `prerelease` flag check, and a
  CHANGELOG/README byte-diff check so the operator can confirm mirror
  sync and anvil-001 release posture at the end of every cut. (3)
  The decision (D-TUIR-021, ratified by this work item) is that the
  publish workflow's Release step is intentionally scoped to
  `anvil-001` with `--prerelease` — the mirror Release is a backfill,
  not an automation step, to keep the publish workflow free of any
  second GitHub App or PAT and to preserve D-TUIR-009 / D-TUIR-011
  tag-protection guarantees. (4) The existing v0.2.3 and v0.2.4
  anvil-001 releases are re-flagged as `prerelease` so the v0.7.4-beta
  Anvil product release is correctly the repo's `latest`.
- **Validation:**
- `gh api repos/eddacraft/eddacraft-tui/releases/latest
  --jq .tag_name` returns the most recent prefixed
  `eddacraft-tui-vX.Y.Z` after the backfill is run;
- byte-diff of `crates/eddacraft-tui/CHANGELOG.md` against
  `gh api …/contents/CHANGELOG.md -H "Accept: application/vnd.github.raw"`
  reports no drift (the mirror README is `MIRROR-README.md + README.md`,
  so the byte-diff is on the body, not the full file);
- new `gh release view` of the mirror release shows
  `targetCommitish` equal to the tag's commit, not `main`;
- runbook diff is link-clean (`pnpm docs:check`).

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIR-010: Deploy `mirror-drift-check.yml` (D-TUIR-018)

- **Status:** Merged 2026-06-08 via PR #2392
- **Intent:** Land the scheduled mirror-drift watchdog that D-TUIR-018
  specified and the mirror workflow's own header flagged as "a
  documented follow-up and NOT yet deployed." Until this job exists,
  the only way to confirm the public mirror still matches canonical
  source is the manual diff an operator runs by hand — which also
  means TUIR-008's "mirror drift check (D-TUIR-018) reports a clean
  tree" validation has nothing to invoke, and TUIN's Ready Checklist
  ("drift check green for at least 7 consecutive runs") cannot even
  begin counting.
- **Expected Outcome:**
  `.github/workflows/mirror-drift-check.yml` runs daily
  (`cron: '27 5 * * *'`) and on `workflow_dispatch`. It reconstructs
  the tree the mirror workflow *would* push — `git subtree split
  --prefix=crates/eddacraft-tui` plus the MIRROR-README banner swap,
  replicated inline to keep zero blast radius on the proven push-side
  workflow (per the TUIR-010 standalone-replication decision) — then
  diffs that **tree** (never commit SHAs) against a read-only shallow
  fetch of `eddacraft/eddacraft-tui:main`. A clean diff passes; any
  non-empty diff opens or refreshes a labelled (`mirror-drift`)
  tracked issue on anvil-001 and fails the job, per D-TUIR-018. The
  mirror is public, so the fetch needs no credentials and the job
  mints no App token — its only write is the issue, via the default
  `GITHUB_TOKEN` (`permissions: contents: read`, `issues: write`).
  Both this file and `mirror-eddacraft-tui.yml` carry a
  cross-reference comment on the subtree-split + banner-swap steps so
  a future edit to one is mirrored in the other (the documented cost
  of replication over a shared script).
- **Validation:** algorithm verified locally against the live mirror
  before merge — the reconstructed split tree
  (`bb6cba45…`) was byte-identical to `eddacraft/eddacraft-tui:main`'s
  tree (clean/green), and a simulated out-of-band mirror commit
  (D-TUIR-009 model) was correctly flagged as drift (red). On merge,
  a `workflow_dispatch` run on `main` must report clean — this is the
  artefact TUIR-008's drift-check validation line consumes.
- **Council (standard pack) — addressed:** propagation-race false
  positive (a daily run racing an in-flight, push-triggered mirror)
  is the failure mode that would reset TUIN's 7-green gate — handled
  by a "Tolerate propagation lag" step that skips (no issue, no
  failure) when a mirror run is `in_progress`/`queued`
  (`actions: read`); `cancel-in-progress: false` so a 14-min split
  isn't killed; `gh label create` no longer swallows real failures;
  `gh issue list --limit 1`; tree-only scope limit + untrusted-mirror
  trust boundary documented. Security pack: no critical/major — the
  job holds no write-capable mirror credential, never checks out the
  fetched tip, and a hostile mirror can at worst cause a spurious
  issue + red job.
- **Deferred follow-up (F-002):** the two workflows' transforms are
  kept aligned by a comment contract, not a CI gate. A future TUIR-010
  follow-up may add a lint asserting both produce the same tree (or
  revisit the shared-script option the standalone-replication decision
  set aside). Tracked here; not blocking.

**changeType:** ci
**releaseIntent:** never
**releaseScope:** none

## Ready Checklist

- [x] ADR-047 accepted (moves from Proposed → Accepted in
      `DECISION-LOG.md`). ✓ Accepted 2026-05-22 via PRs #1846 and the
      direct status update on main.
- [x] TUIMIRROR superseded note added to
      `eddacraft-tui-canonical-source.aps.md` and to the index row. ✓
      Landed in the original TUIR module PR.
- [x] Current `eddacraft-tui` source SHA, latest crates.io version, and
      tag list recorded in the baseline spec (TUIR-001 deliverable). ✓
      Captured 2026-05-22 — v0.2.2 at commit `7ca0c420`, all 5 tags with
      dates and SHAs.
- [x] First post-migration crate version pinned (`0.2.3` per D-TUIR-005)
      and recorded in the baseline spec. ✓
- [x] Mirror push credential wired. ✓ Superseded by the
      `eddacraft-mirror-bot` GitHub App (org-owned by `eddacraft`,
      installed on `eddacraft/eddacraft-tui` only, permissions
      `Contents: Read and write` + `Metadata: Read`). Backed by repo
      secrets `EDDACRAFT_MIRROR_BOT_APP_ID` +
      `EDDACRAFT_MIRROR_BOT_PRIVATE_KEY` on anvil-001. The earlier
      `EDDACRAFT_TUI_MIRROR_PUSH_TOKEN` PAT is no longer referenced
      by any workflow and is slated for deletion after the first
      successful App-auth run (runbook documents the retirement
      step).
- [x] crates.io publish token (least-privilege, scoped to
      `eddacraft-tui` crate) generated by current crate owner and
      stored in `anvil-001` repo secrets as
      `CRATES_IO_EDDACRAFT_TUI_TOKEN`. ✓ Stored 2026-05-24T16:04Z.
      Previous token (`CARGO_REGISTRY_TOKEN` on
      `eddacraft/eddacraft-tui` repo secrets, set 2026-04-10) is the
      one slated for revocation in TUIR-008 after the first
      successful publish from canonical source.
- [x] History-rewrite acknowledged: cutover force-push to
      `eddacraft/eddacraft-tui:main` is one-shot and irreversible;
      `pre-canonical-archive` preservation step (D-TUIR-010) drafted
      in the TUIR-008 runbook before cutover. ✓ Drafted in
      `docs/runbooks/eddacraft-tui-release.md` → "First cutover
      (one-shot, TUIR-008)". ⚠️ **Live-state caveat surfaced while
      drafting:** the content force-push already ran (mirror `main`
      HEAD is the banner-swap commit, 2026-05-25;
      `compare/v0.2.2...main` → 404 "No common ancestor", confirming a
      disconnected subtree-split root) **without** the D-TUIR-010
      archive being created first — `pre-canonical-archive` is still
      404 on the mirror. The pre-cutover graph survives only via the
      preserved `v0.x.y` tags (`v0.2.2` → `5078007b`) and the leftover
      `fix/relocate-acknowledgements-subtree` branch. The runbook now
      carries a corrective step to create `pre-canonical-archive`
      retroactively from the recoverable pre-cutover tip **before**
      cutting `eddacraft-tui-v0.2.3`. This remains an operator action
      (mutates the public mirror).
- [x] Existing public tag policy confirmed (D-TUIR-011) — old
      unprefixed `v0.x.y` tags remain on mirror untouched, new tags
      use `eddacraft-tui-v*` prefix. ✓ Tags + SHAs captured in baseline.
- [x] `crates/eddacraft-tui/MIRROR-README.md` draft reviewed before
      TUIR-007 implementation begins. ✓ Landed via TUIR-007 (PR #1886) —
      banner explains read-only mirror role, points consumers at the
      crates.io release, and references the auto-close PR-redirect
      workflow on the mirror.
- [x] `LICENSE` file identified in the public source for verbatim copy
      during TUIR-002. ✓ Apache-2.0, 11.2K, confirmed verbatim-copyable.
      (Standalone repo has no `NOTICE` file as of 2026-05-22 — do not
      regenerate one absent an attribution need.)
- [x] Standalone crate's `rust-version` (MSRV), `edition`, and
      `[lints]` block content recorded in the TUIR-001 baseline for
      verbatim carry (D-TUIR-015 / D-TUIR-016). ✓ MSRV `1.88`, edition
      `2024`, `[lints]` block captured verbatim.
- [x] Feature flag inventory captured in the baseline (`image`,
      `big-text`, `test-utils`, plus any others) for D-TUIR-007
      per-feature gates. ✓ Three flags total, all captured with
      transitive-cost notes.
- [x] `package-list.txt` baseline captured via `cargo package --list
      -p eddacraft-tui` against the standalone repo (D-TUIR-007 drift
      gate input). ✓ 60 files captured byte-for-byte at
      `plans/specs/2026-05-22-tui-reintegration-baseline/package-list.txt`.
- [x] Internal downstream consumers (in separate repos from
      `anvil-001`, notably `eddacraft/eddacraft-skills`) enumerated in
      the baseline for TUIR-008 operator-driven validation. ✓ Anvil
      first-party consumers (`anvil-cli`, `anvil-tui`, `workspace-hack`)
      with exact manifest lines + `eddacraft-skills` recorded with the
      operator-driven check note.
- [x] `cargo hakari` invocation confirmed in the workspace-hack
      regeneration step of TUIR-003. ✓ TUIR-003 (PR #1879) regenerated
      `crates/workspace-hack/Cargo.toml` with `cargo hakari generate`;
      the pre-migration `eddacraft-tui = { version = "0.2", ... }`
      entry is absent from the regenerated file (path crates need no
      hakari aggregation). Verify with `grep -Fq 'eddacraft-tui'
      crates/workspace-hack/Cargo.toml` — expected to exit 1 (no
      matches). The earlier `grep -c` form prints `0` to stdout but
      also exits 1, so the count-only form looks like success on
      stdout while failing the exit-status check; use `grep -Fq` for
      script use.
- [x] `deny.toml` reviewed for any rules that would unexpectedly fail
      against `eddacraft-tui`'s dependency graph. ✓ Reviewed 2026-05-27.
      `cargo deny check` is clean — `advisories ok, bans ok, licenses
      ok, sources ok`. The crate's transitive tree (ratatui / crossterm
      / `image` + `big-text` + `test-utils` feature deps) is fully
      covered by the broad-permissive `[licenses].allow` list
      (MIT / Apache-2.0 / BSD-2/3 / ISC / Unicode-3.0 / Zlib / MPL-2.0,
      etc., generated from `licences.toml`); `[bans].wildcards = "deny"`
      does not bite (crates.io forbids wildcard deps on published
      crates); `[sources]` allows only the crates.io registry and the
      crate carries no git deps (D-TUIR-014 guard). The only warnings
      are the expected `multiple-versions = "warn"` duplicate (windows
      sys crates) and an unrelated `unnecessary-skip` on
      `workspace-hack` — neither is an error and neither is
      `eddacraft-tui`-specific.
- [x] Rollback path documented at two layers:
      (a) **dependency rollback** — how to revert
      `crates/anvil-tui/Cargo.toml` to the crates.io dep without
      losing in-flight Anvil work;
      (b) **full-migration rollback** — un-freeze the standalone
      repo, revert Anvil consumers to `eddacraft-tui = "0.2.2"`,
      resume standalone publishing, document that public mirror
      history rewrite is not reversible (forensics via
      `pre-canonical-archive`). ✓ Documented in
      `docs/runbooks/eddacraft-tui-release.md` → "Migration rollback
      (two layers)", with the hakari-ordering constraint (Layer a),
      the do-not-revoke-the-legacy-token-while-rollback-is-live caveat
      (Layer b), and the irreversibility note on the `main` history
      rewrite.
- [x] `docs/policies/eddacraft-tui-mirror.md` draft reviewed before
      TUIR-007 implementation begins. ✓ Landed via TUIR-007 (PR #1886) —
      policy doc carries the Backport / Conflict Policy section
      (ratifying D-TUIR-009) plus the Mirror history rewrite
      sub-section (ratifying D-TUIR-020).
