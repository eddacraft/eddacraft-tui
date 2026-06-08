<!-- APS: Design spec for versioned GitHub Releases on the acknowledgements-starter public mirror -->

# Acknowledgements starter kit — versioned releases on the public mirror

Date: 2026-06-08
Module: `ATTRIB` (adds ATTRIB-017; follow-on to ATTRIB-011)
Status: Draft
Coordinates with:
[`plans/archive/modules/attribution-pipeline-v3.aps.md`](../archive/modules/attribution-pipeline-v3.aps.md),
[`plans/execution/ATTRIB-011.steps.md`](../execution/ATTRIB-011.steps.md),
[`.github/workflows/mirror-acknowledgements-starter.yml`](../../.github/workflows/mirror-acknowledgements-starter.yml),
[`.github/workflows/publish-eddacraft-tui.yml`](../../.github/workflows/publish-eddacraft-tui.yml),
[`docs/runbooks/eddacraft-tui-release.md`](../../docs/runbooks/eddacraft-tui-release.md),
[`tools/starters/acknowledgements/MIRROR-README.md`](../../tools/starters/acknowledgements/MIRROR-README.md)

## Goal

Give external consumers of `eddacraft/acknowledgements-starter` a way to **know
when the kit changed**, **read what changed**, and **pin to a known-good
version** — without disturbing the existing rolling-`main` mirror.

Today the mirror's `main` is force-pushed on every kit change
(`mirror-acknowledgements-starter.yml`, ATTRIB-011). That is the right shape
for "track latest", but it gives consumers no notification, no changelog, and
no immutable ref to pin: a `git subtree pull … main` always grabs HEAD, and
that HEAD is rewound on the next sync.

This spec pins the shape of a **second, deliberate surface** — semver tags +
GitHub Releases on the mirror — layered on top of the rolling mirror, modelled
on the proven `eddacraft-tui` release flow (D-TUIR-002/006/009/011).

## Non-goals

- Replacing or changing the rolling `main` mirror. It stays exactly as-is —
  "bleeding edge".
- Per-push / automatic releases. Releases are deliberate (see Cadence).
- Publishing the kit to any package registry (npm, crates.io). The kit is a
  `git subtree` artifact; the release surface is git tags + GitHub Releases
  only.
- SBOM / attestation. That is the separate `supply-chain-attestation` module.

## The core tension this resolves

Releases need **immutable commits** to pin to. The mirror's `main` is
**force-pushed** (history rewritten) every run, by design ("no merge surface",
ATTRIB-011 divergence policy). You cannot cut a stable Release off a branch
that gets rewound seconds later.

Resolution (the tui pattern): keep force-pushing `main` for "latest", and add
an **append-only tag** surface. Tags are pushed `--force`-free; a tagged commit
stays reachable on the mirror even though `main` is later rewritten over it.
GitHub will not GC a commit a tag (or Release) points at.

## Decisions

| Question | Decision | Rationale |
| --- | --- | --- |
| Keep rolling `main`? | Yes, unchanged | It is the correct "latest" surface; releases are additive. |
| Starting version | `1.0.0` | The four ecosystem drivers (Rust/Node/Go/Python) + bundled-binaries have shipped; the kit's contract is stable enough to call 1.0. Not `0.x`. |
| Versioning scheme | SemVer. **major** = a breaking change to a consumer contract (`attribution.toml` schema, driver CLI, marker format, freshness-gate exit semantics); **minor** = additive (new driver/ecosystem, new optional field); **patch** = fixes/docs/determinism with no contract change | Communicates upgrade risk at a glance; matches how the kit's surface actually evolves. |
| Source-side tag format | **Prefixed**: `acknowledgements-starter-vX.Y.Z` on anvil-001 `main` | Same reasoning as D-TUIR-002 — the monorepo ships multiple independently-versioned taggables; a bare `vX.Y.Z` would collide with Anvil **product** release tags. |
| Mirror-side tag format | **Bare**: `vX.Y.Z` on `eddacraft/acknowledgements-starter` | The mirror is single-product and flat-rooted — the `acknowledgements-starter-` prefix is redundant there and `git subtree add … v1.0.0` reads cleanly for consumers. The release workflow maps prefixed→bare when pushing to the mirror. The mirror has **no** pre-existing tags, so there is no legacy-tag preservation concern (contrast tui's pre-cutover `v0.x`). |
| Version source of truth in-tree | `tools/starters/acknowledgements/VERSION` (single line) + `CHANGELOG.md` (Keep-a-Changelog style) | The version + notes travel **into** the mirror, so consumers see them after a `subtree pull` too, not only on the Release object. The git tag is the *trigger*; these files are the durable record. |
| Tag → release consistency | Release workflow parses `X.Y.Z` from the prefixed source tag (`acknowledgements-starter-vX.Y.Z`) and asserts a **version triple match**: that `X.Y.Z` == `VERSION` file == top `CHANGELOG.md` `## [X.Y.Z]` heading. Mismatch fails the run | Same discipline as the tui "version-bump PR then tag the merge commit" — prevents a tag that disagrees with the tree. |
| Tag → release coupling | One source tag push → one mirror tag (bare) + one GitHub Release on the mirror, notes drawn from that version's `CHANGELOG.md` section, marked **latest** | Single deliberate action; the Release is the consumer notification surface. |
| Append-only guarantee | Mirror tag push omits `--force`; if the bare tag already exists on the mirror the push is rejected | D-TUIR-009/011 guarantee — a cut version is immutable; re-cutting requires a new version. |
| Cadence / who cuts | **Deliberate, operator-driven.** Trivial changes (doc tweaks, `oxfmt` reflows, comment fixes) flow to rolling `main` only. A release is cut when there is a consumer-meaningful change | Per-push releases would be noise; the kit sees frequent trivial commits. |
| Auth | Reuse the existing `MIRROR_PUSH_TOKEN` (fine-grained PAT, `Contents: Read and write`, scoped to `eddacraft/acknowledgements-starter`) | Fine-grained `Contents: write` covers both tag push **and** GitHub Release creation. No new secret. (Contrast tui, which uses a GitHub App because it also pushes to a *different* mirror; here the existing PAT already targets the right repo.) |
| Where the Release lives | On the **mirror** (`eddacraft/acknowledgements-starter`) only, marked latest | The mirror is the user-facing repo; consumers watch *it* for releases. anvil-001 is private — a Release there is invisible to consumers. (Contrast tui, which also cuts a pre-release on anvil-001 because the crate has an anvil-side audience; the kit does not.) |

## Design

### Two surfaces, kept independent

1. **Rolling `main`** — `mirror-acknowledgements-starter.yml`, unchanged.
   Force-pushed on every kit change. "Latest / bleeding edge."
2. **Versioned releases** — new `release-acknowledgements-starter.yml`,
   tag-triggered, append-only. "Pin to a known-good version + get notified."

A consumer chooses their risk posture:

```bash
# Track latest (existing behaviour)
git subtree add  --prefix tools/starters/acknowledgements \
  https://github.com/eddacraft/acknowledgements-starter.git main   --squash

# Pin to a release (new)
git subtree add  --prefix tools/starters/acknowledgements \
  https://github.com/eddacraft/acknowledgements-starter.git v1.0.0 --squash
```

### Release workflow (`release-acknowledgements-starter.yml`)

- **Trigger:** `push` of a tag matching `acknowledgements-starter-v*`;
  plus `workflow_dispatch` (with a `tag` input) for backfill / re-run after a
  transient failure.
- **Ref guard:** the tagged commit must be reachable from `main`
  (`git merge-base --is-ancestor "$GITHUB_SHA" origin/main`) — refuse to
  release a tag that points off-trunk. Mirrors ATTRIB-011's refs/heads/main
  guard intent.
- **Version-triple assertion:** extract `X.Y.Z` from the prefixed source tag
  (`acknowledgements-starter-vX.Y.Z` → `X.Y.Z`); require it equals `VERSION`
  and the top `## [X.Y.Z]` heading in `CHANGELOG.md`. Fail-fast with an
  actionable error otherwise. (`vX.Y.Z` is the bare form later pushed to the
  mirror.)
- **Re-assert kit health (cheap):** run `expand-licences.sh --check` (the
  single-source drift gate). The full `acknowledgements-kit.yml` suite already
  ran on the merge that produced the tag; this is a belt-and-braces guard that
  the tagged tree is internally consistent.
- **Subtree split + README swap:** identical to the mirror workflow — swap
  `MIRROR-README.md` onto `README.md` on a throwaway commit, then
  `git subtree split --prefix=tools/starters/acknowledgements`.
- **Push the mirror tag (append-only):** map `acknowledgements-starter-vX.Y.Z`
  → `vX.Y.Z`, push `<split>:refs/tags/vX.Y.Z` **without** `--force` via the
  `http.extraheader` Basic-auth header (the URL-embedded `x-access-token:…@`
  form is fragile to stray bytes — same lesson the mirror workflow already
  bakes in).
- **Cut the GitHub Release:** `gh release create vX.Y.Z
  --repo eddacraft/acknowledgements-starter --target <split-sha>
  --title vX.Y.Z --notes-file <extracted CHANGELOG section> --latest`.
- **Least privilege:** top-level `permissions: contents: read`; all mirror
  writes leave via `MIRROR_PUSH_TOKEN`.
- **Concurrency:** group `release-acknowledgements-starter`, no
  cancel-in-progress (a release must not be interrupted mid-push).
- **Workflow contract:** add to the Workflow Contract Map in
  `.github/workflows/README.md` (auxiliary contract, like the mirror);
  `scripts/ci/workflow-contracts.test.sh` enforces presence.

### In-tree additions to the kit

- `tools/starters/acknowledgements/VERSION` — single line, e.g. `1.0.0`.
- `tools/starters/acknowledgements/CHANGELOG.md` — Keep-a-Changelog style,
  newest first; the `1.0.0` entry summarises the kit as it stands (dispatcher
  + four drivers + bundled-binaries). This file mirrors to the public repo, so
  it is written for **external** readers (no ATTRIB-NNN ids — same discipline
  as the README de-internalisation in 1e314345a / 10162687a).

### Operator runbook

`docs/runbooks/acknowledgements-starter-release.md`, modelled on
`docs/runbooks/eddacraft-tui-release.md`: the cut procedure (version-bump PR →
tag the merge commit → watch the workflow → verify), the mirror tag-protection
gotcha (see Risks), verification commands, and rollback.

## Cut procedure (summary; full detail in the runbook)

1. **Version-bump PR:** bump `VERSION`, add the `## [X.Y.Z]` `CHANGELOG.md`
   entry. Normal review + merge to `main`.
2. **Tag the merge commit:** `git tag acknowledgements-starter-vX.Y.Z <sha>`
   and push the tag.
3. **Workflow runs:** asserts the triple, splits, pushes bare `vX.Y.Z` to the
   mirror append-only, cuts the GitHub Release.
4. **Verify:** `gh release view vX.Y.Z --repo eddacraft/acknowledgements-starter`
   and `gh api repos/eddacraft/acknowledgements-starter/git/refs/tags/vX.Y.Z`.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Mirror has **tag protection** rules blocking the PAT from creating tags | Medium | Pre-flight check in the runbook (tui runbook gotcha #5 documents the exact `gh api` probe); add a tag-protection exception or confirm none exist before the first cut. |
| A bad version is cut and consumers pin it | Medium | Append-only means you cannot silently overwrite — you cut a corrected `vX.Y.Z+1` and (optionally) mark the bad Release as not-latest with a note. Same immutability the tui flow relies on. |
| Tag pushed but tree disagrees (VERSION / CHANGELOG drift) | Low | Version-triple assertion fails the run before any mirror write. |
| `VERSION`/`CHANGELOG.md` drift from the rolling mirror confuses consumers | Low | Both files mirror to `main` too, so rolling-mirror readers see the same version markers; releases just freeze a point. |
| Fine-grained PAT lacks Release scope | Low | `Contents: write` covers Releases for fine-grained PATs; verified at first cut. No new secret needed. |
| Operator cuts a release for a trivial change (noise) | Low | Cadence is documented as deliberate; the rolling mirror already covers "every change". |

## Open question for planning (module home)

The `attribution-pipeline-v3` module is **archived Complete** (15/16, archived
2026-05-26). This is genuinely new scope. Options, in preference order:

1. **New Proposed module** `acknowledgements-starter-releases` (retains the
   ATTRIB lineage; room for follow-ons such as a second starter kit reusing the
   release pattern). *Recommended.*
2. **CIB entry** if it should stay a single self-contained improvement rather
   than a module.
3. Un-archive `attribution-pipeline-v3` — **discouraged** (archive/un-archive
   is a multi-file cascade and the module is genuinely closed).

This spec carries the work as **ATTRIB-017** regardless of the chosen home.
