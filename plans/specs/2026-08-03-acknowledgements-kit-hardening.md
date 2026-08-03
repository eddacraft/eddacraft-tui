<!-- APS: Design spec for the acknowledgements starter kit hardening wave (ATTRIB-018..-024) -->

# Acknowledgements starter kit — hardening the gates to match the contract

Date: 2026-08-03
Module: `ATTRIB` (adds ATTRIB-018..-024; follow-on to ATTRIB-017)
Status: Accepted
Coordinates with:
[`plans/modules/acknowledgements-kit-hardening.aps.md`](../modules/acknowledgements-kit-hardening.aps.md),
[`plans/archive/modules/acknowledgements-starter-releases.aps.md`](../archive/modules/acknowledgements-starter-releases.aps.md),
[`plans/specs/2026-06-08-acknowledgements-starter-releases.md`](./2026-06-08-acknowledgements-starter-releases.md),
[`docs/runbooks/acknowledgements-starter-release.md`](../../docs/runbooks/acknowledgements-starter-release.md),
[`.github/workflows/acknowledgements-kit.yml`](../../.github/workflows/acknowledgements-kit.yml),
[`tools/starters/acknowledgements/README.md`](../../tools/starters/acknowledgements/README.md)

## Goal

Close the gap between what the published kit **promises** and what it
**enforces**, and get the result to consumers as `1.1.0`.

The kit's README states two load-bearing invariants — *hand-curated content
outside the markers is preserved verbatim*, and *`--check` is the freshness
gate*. Both can be violated today without a non-zero exit. A licence-attribution
tool that silently deletes curated prose, or reports green over stale
attribution, fails at the one thing it exists to do.

This spec pins the four decisions taken on 2026-08-03 that the module's work
items depend on. The findings themselves, with reproductions, live in the
module's Evidence section and are not restated here.

## Non-goals

- No `attribution.toml` schema change, no driver-invocation contract change, no
  marker-syntax change.
- New ecosystem drivers (Java/Kotlin, Ruby, Swift) — still deferred until a real
  consumer needs one.
- SBOM / CycloneDX / attestation — owned by
  [`supply-chain-attestation`](../modules/supply-chain-attestation.aps.md).
- The release and mirror workflows themselves — proven at the `v1.0.0` cut; only
  the kit-side content they publish changes here.

## Facts established before the decisions

These were discoverable and are recorded so the decisions are not re-litigated
from assumptions:

- The mirror has **no `.github/` directory** today, and
  `mirror-acknowledgements-starter.yml` performs a plain
  `git subtree split --prefix=tools/starters/acknowledgements`. Anything placed
  inside the kit directory therefore lands at the mirror **root**. A workflow at
  `<kit>/.github/workflows/` would be inert upstream (GitHub reads only the
  repository-root `.github/workflows`) and live on the mirror.
- The mirror is a byte-identical split of the upstream directory, so upstream
  CI green already implies mirror content green. Mirror-side CI would re-test
  the same bytes.
- Identified consumers are `eddacraft-tui`, `little-termi`, and anvil-001
  itself — all owner-controlled. No third-party adopter is known.
- The kit has **no warning channel**: every dispatcher path is an error at exit
  1 or silence. A "warn first" posture would invent a new output class and
  require deciding its exit code.
- `ci-freshness.yml.snippet` already ships in the kit as an inert example
  workflow for consumers to copy — an in-kit precedent for the snippet shape.

## Decision 1 — orphaned marker pairs hard-error in both modes

A marker pair in the target with no matching block in `attribution.toml` exits
**1**, naming the orphan, in write and `--check` alike.

**Why.** It is consistent with the existing marker-count gate, needs no new
output class, and adds no schema key. The consumer's escape hatch is deleting
two marker lines from their own file. The blast radius assumed when the question
was raised — a consumer's CI failing on a block they retired mid-migration —
is small in practice, because every identified consumer is owner-controlled.

**Rejected.** *Warn now, error at the next major* defers the actual gate to a
bump with no scheduled date and leaves stale attribution passing green in the
meantime. *Error in `--check` only* creates a loop with no in-tool exit: the
gate fails, the fix-it command exits 0 without touching the orphan, and the gate
fails again. *An `orphan_markers` config key* is a schema addition, which this
wave scopes out.

## Decision 2 — the mirror stays content-only: runner plus inert snippet

Ship `tests/run-all.sh` as the single source of the kit's test list, invoked by
upstream CI instead of the workflow duplicating sixteen steps. Alongside it,
ship a `kit-tests.yml.snippet` carrying the four pinned tool versions
(`cargo-about`, `license-checker`, `go-licenses`, `pip-licenses`), which a fork
copies into its own `.github/workflows/`.

**Why.** It follows the `ci-freshness.yml.snippet` precedent already in the kit,
gives a fork everything it needs including the version pins it would otherwise
have to reverse-engineer, and keeps the mirror content-only. No live workflow
file exists inside the kit directory, so a consumer who vendors the kit at their
repository root cannot accidentally activate one.

**Rejected.** *Runner only* leaves a fork to rediscover the pinned tool
versions. *A live mirrored workflow* re-tests bytes upstream CI already tested,
fires on every force-push sync, and carries the accidental-activation hazard.

## Decision 3 — `--version` on the dispatcher only

`generate-acknowledgements.sh --version` prints the `VERSION` found beside its
symlink-resolved real path, degrading to `unknown` when the file is absent.

**Why.** The dispatcher is the entry point consumers actually invoke, and it
already resolves its own real path through symlinks — the case where
`cat VERSION` is awkward, because the user would have to resolve the link
themselves. `expand-licences.sh` and `check-version.sh` do not get the flag:
both are run from a known checkout, and three surfaces to maintain and test for
one fact is not worth it.

This moves out of the ATTRIB-023 ergonomics backlog into its own scope, because
it is an additive CLI surface rather than a rough edge.

## Decision 4 — the next release is `1.1.0`

**Why.** "Freshness-gate exit semantics" in the kit's CHANGELOG major-trigger
list is read as *the exit-code table* (what 0, 1 and 2 mean), not *the set of
inputs that reach exit 1*. On that reading, decisions 1 and the F1 marker-order
gate are defect fixes: the affected inputs were producing silently wrong output,
and the table is unchanged.

**Recorded tension.** The opposite reading is available and was raised before
the decision: a consumer whose build passes today can go red on upgrade without
changing a line, which is the ordinary test for a breaking change. The narrow
reading was chosen deliberately, not overlooked. Two follow-throughs keep it
honest, and both are conditions of ATTRIB-024:

1. The `1.1.0` CHANGELOG entry calls out the two newly-failing cases under their
   own heading, so an upgrader meets them before their CI does.
2. The CHANGELOG's own major/minor/patch definition is reworded to state the
   narrow reading explicitly. Without this, the same argument recurs at the next
   gate and the rule stops meaning anything.

## Consequences for the work items

- **ATTRIB-018** — implements decision 1 alongside the F1 marker-order gate.
- **ATTRIB-022** — implements decision 2; the runner becomes the single source
  of the test list, and the workflow invokes it.
- **ATTRIB-023** — loses the `--version` question to decision 3; retains only
  F10–F12.
- **ATTRIB-024** — cuts `1.1.0` and carries the two decision-4 follow-throughs
  as part of its expected outcome.

## Risks

- **ATTRIB-021 (macOS leg)** has genuinely unknown yield. Discovering how much
  the second platform diverges is the item's purpose, so the risk is schedule,
  not correctness.
- The `1.1.0` reading depends on the CHANGELOG rewording actually landing. If
  ATTRIB-024 ships without it, the kit carries a semver rule its own release
  history contradicts.
