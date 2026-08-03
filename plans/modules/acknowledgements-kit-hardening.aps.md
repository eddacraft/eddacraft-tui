<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Acknowledgements Kit Hardening

| ID     | Owner      | Status      |
| ------ | ---------- | ----------- |
| ATTRIB | joshuaboys | In Progress |

**Last reviewed:** 2026-08-03 — opened from a full read-through of the shipped
kit (`v1.0.0`, unchanged since 2026-06-08). Two splice-integrity defects and one
config-parser defect were reproduced against the real scripts; the rest are
gaps in the kit's own verification and mirror surface. The four design decisions
the items depend on were taken the same day and are recorded in the design
contract below. Operator authorised **ATTRIB-018..-023** on 2026-08-03 and all
six are now **Merged** — five via PR #3492, with ATTRIB-021 held back until PR
#3495 fixed the macOS runner's missing Go and the first green both-legs run
(30792191535) supplied its evidence. Module stays **In Progress**: ATTRIB-024
(release cut) and ATTRIB-025 (`--version`) remain Proposed, so the hardening is
not yet in any consumer's hands.

Design contract:
[`plans/specs/2026-08-03-acknowledgements-kit-hardening.md`](../specs/2026-08-03-acknowledgements-kit-hardening.md).

## Purpose

Close the gap between what the acknowledgements starter kit **promises** and
what it **enforces**, before the next release is cut.

The kit is published to external consumers at
[`eddacraft/acknowledgements-starter`](https://github.com/eddacraft/acknowledgements-starter)
(`v1.0.0`, plus a rolling `main` mirror). Its README states two load-bearing
invariants — *hand-curated content outside the markers is preserved verbatim*,
and *`--check` is the freshness gate* — and both can be violated today without
a non-zero exit. A licence-attribution tool that silently deletes curated prose,
or reports green over stale attribution data, fails at the one thing it exists
to do.

No consumer-facing contract is redefined: every item makes the kit enforce a
rule it already documents, verifies a property it already claims, or adds a
purely additive flag.
This module retains the **ATTRIB lineage** (continuing after ATTRIB-017) rather
than re-opening the archived
[`attribution-pipeline-v3`](../archive/modules/attribution-pipeline-v3.aps.md)
or
[`acknowledgements-starter-releases`](../archive/modules/acknowledgements-starter-releases.aps.md)
modules, both of which are genuinely Complete — the same precedent
ATTRIB-017 set.

## In Scope

- Splice-integrity gates the dispatcher documents but does not enforce
  (marker ordering, orphaned marker pairs).
- Fidelity of the dispatcher's own `attribution.toml` parser and of the
  per-driver markdown renders.
- The kit's self-verification: self-tests that cannot pass by skipping, static
  analysis, and proof of the portability the scripts are hand-written for.
- The public mirror's consumability (working links, a runnable test entrypoint).
- The `VERSION` + `CHANGELOG.md` bump and release cut that publish the above.

## Out of Scope

- New ecosystem drivers (Java/Kotlin, Ruby, Swift) — still deferred until a
  real consumer needs one; unchanged from the README's roadmap note.
- The `attribution.toml` schema, marker syntax, driver-invocation contract, and
  the `--check` exit-code table. The only additive surface is a dispatcher
  `--version` flag (ATTRIB-025), which makes the release these items feed a
  **minor** bump — see decision 4 in the design contract for why the two new
  gates are read as fixes rather than breaks.
- SBOM / CycloneDX / attestation — owned by
  [`supply-chain-attestation`](./supply-chain-attestation.aps.md).
- The release and mirror workflows themselves
  (`.github/workflows/release-acknowledgements-starter.yml`,
  `.github/workflows/mirror-acknowledgements-starter.yml`) — verified working
  at the `v1.0.0` cut; only the kit-side content they publish changes here.

## Interfaces

**Depends on:** the shipped kit under `tools/starters/acknowledgements/`; the
kit self-test job in `.github/workflows/acknowledgements-kit.yml`; the release
path documented in `docs/runbooks/acknowledgements-starter-release.md`.

**Exposes:** a kit whose splice gates match its documented invariants, whose CI
cannot go green without exercising every driver, and whose published mirror is
self-contained.

## Evidence (2026-08-03)

Findings 1–3 were reproduced against the real scripts with stub drivers
(`ATTRIB_DRIVERS_DIR`); the rest are read-verified against the cited lines.

**F1 — reversed markers silently delete curated content.** The per-block gate in
`generate-acknowledgements.sh` counts BEGIN and END occurrences but never
compares their positions. With an `END` line above its `BEGIN` line (one of
each, so the count gate passes), the splice `awk` drops every line from `BEGIN`
to EOF. A probe target lost its entire hand-written tail section; the command
printed `Updated <path>` and exited **0**. Contradicts README "Hand-curated
content" and "Atomic write".

**F2 — orphaned marker pairs go stale forever and `--check` passes.** The splice
loop only visits blocks declared in `attribution.toml`. Removing or renaming a
block leaves its marker pair holding the last generated content indefinitely. A
probe left a fabricated `GPL-3.0` row inside a retired block; `--check` exited
**0**. The freshness gate reports green over stale attribution.

**F3 — a quoted TOML value is truncated at a literal `#`.** The dispatcher's
`read_scalar` / `read_array_entry` strip inline comments *before* handling
quotes, so `manifest_path = "vendor/c#sharp/Cargo.toml"` reaches the driver as
`…/vendor/c`. `drivers/bundled-binaries.sh` already parses quote-first and is
correct — the dispatcher's older parser never got the same treatment.

**F4 — markdown-cell escaping exists in exactly one driver.** The
bundled-binaries driver escapes `|` in every cell; the node and go renders and
the python passthrough do not, so a package name, licence expression, or
repository URL containing `|` breaks the rendered table.

**F5 — self-tests can pass by skipping.** Eight of the sixteen self-tests
`exit 0` when a tool is missing or an install fails (Node, Go and Python
render/strict tests). Nothing in the workflow asserts they ran, so a network
blip or a broken install step yields a green **Kit Self-Tests** with three
ecosystem drivers entirely unexercised.

**F6 — no static analysis gate.** ShellCheck over the kit is clean today (one
known false positive at `check-version.sh:145`, four benign `SC2034`s in tests),
so wiring it in is free and prevents regression.

**F7 — macOS is never tested.** The scripts carry deliberate portability work —
manual symlink resolution instead of `readlink -f`, a hand-written wrapper
replacing `fold` because coreutils implementations differ — yet CI runs
`ubuntu-latest` only, so none of it is exercised (including bash 3.2 behaviour
on stock macOS).

**F8 — the mirror ships a dead link.** README's release section links
`../../../docs/runbooks/acknowledgements-starter-release.md`, a path outside the
subtree and inside the private repo. Confirmed live at line 780 of the public
`README.md`; it can never resolve for an external reader.

**F9 — external consumers cannot run the suite.** There is no test entrypoint in
the kit; the list of sixteen tests exists only in
`.github/workflows/acknowledgements-kit.yml`, which is not mirrored. Anyone
adopting the public repo gets tests with no runner and no CI.

**F10–F12 — ergonomics.** `drivers/node.sh` invokes `license-checker` twice (the
strict gate, then the render) where one `--json` run could feed both;
`expand-licences.sh` hardcodes its consumer files beside `licences.toml` with no
per-path configuration, unlike the fully parameterised dispatcher; and
`drivers/python.sh` assumes a POSIX `venv/bin/` layout.

## Work Items

**ATTRIB-018..-023 are Ready** (operator authorisation 2026-08-03) and execute in
that order — **-018** first, then **-019** and **-020** (independent of each
other), then **-021** (needs -020's skip gate) and **-022**, then **-023**.
**ATTRIB-025** and **ATTRIB-024** stay Proposed; the release cut is a separate
authorisation once the six land.

| Item        | Findings   | Theme                              |
| ----------- | ---------- | ---------------------------------- |
| ATTRIB-018  | F1, F2     | Splice integrity                   |
| ATTRIB-019  | F3, F4     | Parse + render fidelity            |
| ATTRIB-020  | F5, F6     | Self-verification honesty          |
| ATTRIB-021  | F7         | Portability proof                  |
| ATTRIB-022  | F8, F9     | Mirror consumability               |
| ATTRIB-023  | F10–F12    | Ergonomics backlog                 |
| ATTRIB-025  | —          | Dispatcher `--version`             |
| ATTRIB-024  | —          | Release cut                        |

### ATTRIB-018: Splice gates enforce the documented marker invariants

- **Status:** Merged 2026-08-03 via PR #3492
- **Intent:** The generator never destroys hand-curated content, and `--check`
  never reports green over generated content it no longer maintains.
- **Expected Outcome:** A mis-ordered marker pair is refused with an actionable
  error and the on-disk target untouched; a marker pair with no matching block
  in `attribution.toml` exits non-zero naming the orphan rather than being
  silently retained. Both are hard errors in write and `--check` alike
  (decision 1) — no warn tier, no config key, and the consumer's escape hatch is
  deleting the two marker lines.
- **Validation:** New fixture cases in
  `tools/starters/acknowledgements/tests/dispatcher-schema-validation.sh` cover
  (a) `END` above `BEGIN` → non-zero exit, target byte-identical, and (b) an
  orphaned marker pair → non-zero exit naming the orphan; the full kit
  self-test list in `.github/workflows/acknowledgements-kit.yml` passes.
- **Files:** `tools/starters/acknowledgements/generate-acknowledgements.sh`,
  `tools/starters/acknowledgements/tests/dispatcher-schema-validation.sh`,
  `tools/starters/acknowledgements/README.md`.
- **Confidence:** high — both defects reproduced; the gates sit beside the
  existing marker-count gate.

### ATTRIB-019: Config parsing and rendered cells survive punctuation

- **Status:** Merged 2026-08-03 via PR #3492
- **Intent:** A quoted `attribution.toml` value reaches the driver intact, and
  no scanner-supplied string can break the rendered markdown table.
- **Expected Outcome:** A `#` inside a quoted value is data, not a comment
  start, matching `drivers/bundled-binaries.sh`'s existing behaviour and real
  TOML; every driver escapes `|` in emitted cells.
- **Validation:** A fixture block whose path value contains `#` round-trips to
  the driver unchanged; a fixture package/licence/repository value containing
  `|` renders as a single well-formed row; existing dispatcher and driver
  render tests still pass.
- **Files:** `tools/starters/acknowledgements/generate-acknowledgements.sh`,
  `tools/starters/acknowledgements/drivers/node.sh`,
  `tools/starters/acknowledgements/drivers/go.sh`,
  `tools/starters/acknowledgements/drivers/python.sh`,
  `tools/starters/acknowledgements/tests/dispatcher-schema-validation.sh`,
  `tools/starters/acknowledgements/tests/node-driver-render.sh`.
- **Dependencies:** none (independent of ATTRIB-018).
- **Confidence:** high — the correct parser shape already exists in-kit.

### ATTRIB-020: Self-tests cannot pass by skipping, and are lint-gated

- **Status:** Merged 2026-08-03 via PR #3492
- **Intent:** A green **Kit Self-Tests** run proves every driver was actually
  exercised, and shell regressions are caught by static analysis.
- **Expected Outcome:** Skips remain available for local runs but are a failure
  under CI; ShellCheck runs over the kit's scripts, drivers and tests as part
  of the same workflow.
- **Validation:** `.github/workflows/acknowledgements-kit.yml` fails when a
  driver test skips; ShellCheck exits clean over
  `tools/starters/acknowledgements/**/*.sh` with any suppression carrying an
  inline justification.
- **Files:** `.github/workflows/acknowledgements-kit.yml`,
  `tools/starters/acknowledgements/tests/*.sh`,
  `tools/starters/acknowledgements/check-version.sh`.
- **Confidence:** high — the kit is ShellCheck-clean today, so the gate lands
  without a cleanup wave.

### ATTRIB-021: Prove the portability the kit is written for

- **Status:** Merged 2026-08-03 via PR #3492 (matrix) and PR #3495 (Go
  provisioning). Held at In Progress until the evidence existed; released once
  both legs ran green — see the run record below.
- **Intent:** The macOS-portability work already in the scripts is verified, not
  assumed.
- **Expected Outcome:** The kit self-tests run on macOS as well as Linux, so
  symlink resolution, the `fold`-free note wrapper, and bash-version-sensitive
  constructs are exercised on the platform they were written to accommodate.
- **Validation:** The kit self-test job passes on both legs of a
  `ubuntu-latest` / `macos-latest` matrix; any platform-specific divergence is
  fixed in the kit rather than skipped.
- **Evidence gap (recorded 2026-08-03, raised in verification):** the macOS leg
  is deliberately excluded from `pull_request` runs, because this repository
  keeps matrices off the PR path on cost grounds and repository policy outranks
  this item's wording. The consequence is that **the PR implementing this item
  cannot produce the both-legs evidence its Validation asks for** — the first
  macOS run happens on the post-merge push to `main`, the weekly cron, or a
  manual `gh workflow run acknowledgements-kit.yml`. `workflow_dispatch` cannot
  supply it earlier: GitHub only exposes the trigger once the workflow file is
  on the default branch. This item therefore stays **In Progress** past merge
  until that first macOS run is green, and any divergence it surfaces is
  follow-up work, not a reason to weaken the gate.
- **First macOS run — FAILED (run 30790996229, 2026-08-03).** The leg earned its
  keep on its first execution, though not where expected: the divergence is in
  the workflow, not the kit's bash. `macos-latest` does not preinstall Go, so
  `go install github.com/google/go-licenses@…` died with
  `go: command not found` (exit 127) and every later step — the self-tests,
  ShellCheck, the drift check — was skipped. The Linux leg of the same run
  passed. Fix: provision Go explicitly rather than inheriting it from the
  runner image. **As of that run** the item's Expected Outcome was unproven,
  because the kit's symlink resolution, `fold`-free wrapper and
  bash-3.2-sensitive constructs had not yet executed on macOS even once —
  which is why the item was held at In Progress rather than flipped. Resolved
  by the run recorded below.
- **Second macOS run — GREEN (run 30792191535, 2026-08-03), Validation met.**
  After PR #3495 provisioned Go explicitly, the post-merge push ran both legs
  to success. The macOS leg reported `16 passed, 0 skipped, 0 failed (of 16)`
  on `darwin/arm64` with `go1.26.5` — a real tally, not a trivial pass, because
  `--require-all` (ATTRIB-020) makes a skipped test a failure. This is the
  first time the kit's hand-written portability work has executed on the
  platform it was written for, and it needed no changes: the symlink
  resolution, the `fold`-free note wrapper, and the driver scripts all behaved
  identically to Linux. The portability was real; it had simply never been
  proven.
- **Files:** `.github/workflows/acknowledgements-kit.yml`,
  `tools/starters/acknowledgements/expand-licences.sh`,
  `tools/starters/acknowledgements/generate-acknowledgements.sh`.
- **Dependencies:** ATTRIB-020 (land the skip gate first, or a macOS leg can go
  green by skipping).
- **Confidence:** medium — unknown how much real divergence the second leg
  surfaces; that discovery is the point of the item.

### ATTRIB-022: The published mirror is self-contained

- **Status:** Merged 2026-08-03 via PR #3492
- **Intent:** An external consumer of the public repo can follow every link and
  run the kit's tests without access to the private upstream.
- **Expected Outcome:** No kit-internal document links to a path outside the
  subtree; `tests/run-all.sh` becomes the single source of the kit's test list
  and upstream CI invokes it rather than duplicating sixteen steps; an inert
  `kit-tests.yml.snippet` carries the four pinned tool versions for a fork to
  copy into its own `.github/workflows/`. The mirror stays content-only —
  no live workflow file inside the kit directory (decision 2).
- **Validation:** No relative link in the kit's mirrored markdown escapes the
  kit directory; the runner executes the same set the workflow previously
  listed, and the workflow invokes the runner; the public `README.md` after the
  next mirror sync contains no unresolvable relative link.
- **Files:** `tools/starters/acknowledgements/README.md`,
  `tools/starters/acknowledgements/MIRROR-README.md`,
  `tools/starters/acknowledgements/tests/run-all.sh`,
  `tools/starters/acknowledgements/kit-tests.yml.snippet`,
  `.github/workflows/acknowledgements-kit.yml`.
- **Confidence:** high — the dead link is confirmed live; the runner is a
  mechanical extraction of the workflow's step list.

### ATTRIB-023: Kit ergonomics backlog

- **Status:** Merged 2026-08-03 via PR #3492
- **Intent:** Record the non-blocking rough edges so they are not rediscovered
  by the next reader.
- **Expected Outcome:** Each of F10–F12 is either fixed or explicitly declined
  with a reason: the node driver's duplicate `license-checker` invocation, the
  expander's hardcoded consumer-file locations, and the python driver's
  POSIX-only venv layout assumption.
- **Validation:** Each of the three carries a decision (fixed, with a test, or
  declined with the rationale recorded here).
- **Files:** `tools/starters/acknowledgements/drivers/node.sh`,
  `tools/starters/acknowledgements/expand-licences.sh`,
  `tools/starters/acknowledgements/drivers/python.sh`.
- **Confidence:** medium — the expander's path configuration is a design
  question (whose default is deliberate), not a defect.

**Decisions recorded 2026-08-03:**

- **F10 — duplicate `license-checker` invocation: declined.** The strict gate
  (`--onlyAllow`) and the render (`--json`) are two different tool contracts;
  collapsing them means reimplementing `--onlyAllow`'s SPDX matching in `jq`,
  moving the licence decision out of the tool that owns it and into the kit.
  The cost is one extra scan of an already-installed tree. Revisit only if a
  consumer reports it as a real bottleneck.
- **F11 — expander path configuration: declined as designed.** Resolving the
  consumer files beside `licences.toml` is what makes the single-source
  guarantee legible: the canonical file and everything generated from it live
  together. Per-file path keys would let them drift apart, which is the failure
  mode the script exists to prevent.
- **F12 — Windows venv layout: declined, out of platform scope.** The kit is
  bash throughout, so a Windows consumer is already under Git Bash or WSL where
  the POSIX `venv/bin` layout is the norm. Native `Scripts/` support would be
  the smallest part of a much larger Windows story the kit does not tell.
- **Carried in from ATTRIB-019 — go and python cell escaping: not done.** The
  node driver now escapes `|` in every cell it builds, matching
  bundled-binaries. Go and python are not covered: go's rows come from
  `templates/go-licenses.tmpl` and python's table is written wholesale by
  `pip-licenses --format markdown`, so in neither case does the driver
  construct the cells. Covering them needs either a template-contract change
  that would silently break a consumer's custom `template_path`, or reparsing
  tool output. Exposure is theoretical — Go import paths and SPDX identifiers
  do not contain `|`. Recorded here rather than narrowed silently inside
  ATTRIB-019.

### ATTRIB-025: Dispatcher reports its own kit version

- **Status:** Proposed
- **Intent:** A vendored or symlinked copy of the kit can say which version it
  is, without the reader resolving the symlink by hand.
- **Expected Outcome:** `generate-acknowledgements.sh --version` prints the
  `VERSION` found beside its symlink-resolved real path, degrading to `unknown`
  when the file is absent. The dispatcher only (decision 3) — `expand-licences.sh`
  and `check-version.sh` keep their current surface, since both are run from a
  known checkout.
- **Validation:** Invoking the flag through a symlink on `PATH` prints the kit's
  `VERSION`, and a copy with `VERSION` removed prints `unknown` at exit 0;
  covered by a case in the kit self-tests.
- **Files:** `tools/starters/acknowledgements/generate-acknowledgements.sh`,
  `tools/starters/acknowledgements/tests/version-changelog-consistency.sh`,
  `tools/starters/acknowledgements/README.md`.
- **Confidence:** high — the symlink resolution it depends on already exists.

### ATTRIB-024: Cut the release that publishes the hardening

- **Status:** Proposed
- **Intent:** External consumers can pin a version containing the fixes and read
  what changed.
- **Expected Outcome:** `VERSION` and `CHANGELOG.md` are bumped to **1.1.0**
  together (decision 4) and a release is cut per the existing runbook. Two
  conditions travel with the bump: the entry calls out the two newly-failing
  cases (mis-ordered and orphaned markers) under their own heading so an
  upgrader meets them before their CI does, and the CHANGELOG's own
  major/minor/patch definition is reworded to state the narrow reading of
  "freshness-gate exit semantics" that justifies a minor here.
- **Validation:** `bash tools/starters/acknowledgements/check-version.sh` is
  clean; the `1.1.0` entry contains both the newly-failing-cases heading and the
  reworded semver definition; the tag push produces a mirror tag plus a GitHub
  Release marked latest; `git subtree add … v1.1.0 --squash` into a scratch repo
  reproduces the kit tree. Procedure:
  `docs/runbooks/acknowledgements-starter-release.md`.
- **Files:** `tools/starters/acknowledgements/VERSION`,
  `tools/starters/acknowledgements/CHANGELOG.md`.
- **Dependencies:** whichever of ATTRIB-018..-022 and -025 land; do not cut a
  release per item.
- **Confidence:** high — mechanical, and the release path is proven from the
  `v1.0.0` cut.

## Decisions (2026-08-03)

All four were taken in a design pass on the day the module opened. Rationale and
rejected alternatives are in the design contract; only the outcomes are repeated
here.

1. **Orphaned marker pairs hard-error in both write and `--check`** — no warn
   tier, no `orphan_markers` config key. Consistent with the marker-count gate;
   escape hatch is deleting the two marker lines. → ATTRIB-018.
2. **The mirror stays content-only: `tests/run-all.sh` plus an inert
   `kit-tests.yml.snippet`** carrying the pinned tool versions, not a live
   workflow inside the kit directory. Follows the `ci-freshness.yml.snippet`
   precedent and cannot activate by accident in a repo that vendors the kit at
   its root. → ATTRIB-022.
3. **`--version` on the dispatcher only**, degrading to `unknown`. →
   ATTRIB-025.
4. **The next release is `1.1.0`**, reading "freshness-gate exit semantics" as
   the exit-code table rather than the set of inputs reaching exit 1. The
   opposite reading was raised and narrowed deliberately; the two conditions on
   ATTRIB-024 are what keep that honest. → ATTRIB-024.

No open questions remain.

## Notes

Opened 2026-08-03 after an operator question about whether the kit needed a
release. It did not — the kit tree is byte-identical to the `v1.0.0` tag, with
no commits under `tools/starters/acknowledgements/` since 2026-06-08, and the
mirror's `main` tree matches the tag. The review that answered that question is
what produced this module: the release surface is healthy, the kit's own
enforcement is what has gaps.

Findings are recorded in the **Evidence** section above rather than in
`plans/issues.md` so the reproductions travel with the work items that fix them.
ATTRIB-024 exists because none of the fixes reach an external consumer until a
release is cut — the same reason ATTRIB-017 folded the first cut into its own
work item.

The four decisions were grilled the same day the module opened, before any item
was authorised, so the items are specified against settled choices rather than
open ones. ATTRIB-025 was spawned from that pass: the dispatcher `--version`
question started as an open question and became a work item once accepted, which
is why the numbering runs out of order against the recommended Ready sequence.
