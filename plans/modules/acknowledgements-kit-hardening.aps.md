<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Acknowledgements Kit Hardening

| ID     | Owner      | Status   |
| ------ | ---------- | -------- |
| ATTRIB | joshuaboys | Proposed |

**Last reviewed:** 2026-08-03 — opened from a full read-through of the shipped
kit (`v1.0.0`, unchanged since 2026-06-08). Two splice-integrity defects and one
config-parser defect were reproduced against the real scripts; the rest are
gaps in the kit's own verification and mirror surface. Module is **Proposed**:
no work item is authorised until the operator flips it.

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

Nothing here is a contract change for consumers: every item makes the kit
enforce a rule it already documents, or verifies a property it already claims.
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
  `--check` exit-code semantics — no consumer-visible contract changes, so the
  release these items feed is a minor/patch bump, not a major.
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

This module is **Proposed**; items are specified but not authorised. Recommended
Ready order once the operator flips them: **ATTRIB-018**, then **-019** and
**-020** (independent of each other), then **-021** / **-022**, with
**ATTRIB-024** last. ATTRIB-023 can stay Proposed indefinitely.

| Item        | Findings   | Theme                              |
| ----------- | ---------- | ---------------------------------- |
| ATTRIB-018  | F1, F2     | Splice integrity                   |
| ATTRIB-019  | F3, F4     | Parse + render fidelity            |
| ATTRIB-020  | F5, F6     | Self-verification honesty          |
| ATTRIB-021  | F7         | Portability proof                  |
| ATTRIB-022  | F8, F9     | Mirror consumability               |
| ATTRIB-023  | F10–F12    | Ergonomics backlog                 |
| ATTRIB-024  | —          | Release cut                        |

### ATTRIB-018: Splice gates enforce the documented marker invariants

- **Status:** Proposed
- **Intent:** The generator never destroys hand-curated content, and `--check`
  never reports green over generated content it no longer maintains.
- **Expected Outcome:** A mis-ordered marker pair is refused with an actionable
  error and the on-disk target untouched; a marker pair with no matching block
  in `attribution.toml` is reported by name rather than silently retained. Both
  behaviours hold in write and `--check` mode.
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

- **Status:** Proposed
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

- **Status:** Proposed
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

- **Status:** Proposed
- **Intent:** The macOS-portability work already in the scripts is verified, not
  assumed.
- **Expected Outcome:** The kit self-tests run on macOS as well as Linux, so
  symlink resolution, the `fold`-free note wrapper, and bash-version-sensitive
  constructs are exercised on the platform they were written to accommodate.
- **Validation:** The kit self-test job passes on both legs of a
  `ubuntu-latest` / `macos-latest` matrix; any platform-specific divergence is
  fixed in the kit rather than skipped.
- **Files:** `.github/workflows/acknowledgements-kit.yml`,
  `tools/starters/acknowledgements/expand-licences.sh`,
  `tools/starters/acknowledgements/generate-acknowledgements.sh`.
- **Dependencies:** ATTRIB-020 (land the skip gate first, or a macOS leg can go
  green by skipping).
- **Confidence:** medium — unknown how much real divergence the second leg
  surfaces; that discovery is the point of the item.

### ATTRIB-022: The published mirror is self-contained

- **Status:** Proposed
- **Intent:** An external consumer of the public repo can follow every link and
  run the kit's tests without access to the private upstream.
- **Expected Outcome:** No kit-internal document links to a path outside the
  subtree; the kit carries its own test entrypoint so the suite is runnable
  (and forkable) from the mirror alone.
- **Validation:** No relative link in the kit's mirrored markdown escapes the
  kit directory; the new runner executes the same set the CI workflow lists,
  and the workflow invokes the runner rather than duplicating the list; the
  public `README.md` after the next mirror sync contains no unresolvable
  relative link.
- **Files:** `tools/starters/acknowledgements/README.md`,
  `tools/starters/acknowledgements/MIRROR-README.md`,
  `tools/starters/acknowledgements/tests/`,
  `.github/workflows/acknowledgements-kit.yml`.
- **Confidence:** high — the dead link is confirmed live; the runner is a
  mechanical extraction of the workflow's step list.

### ATTRIB-023: Kit ergonomics backlog

- **Status:** Proposed
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

### ATTRIB-024: Cut the release that publishes the hardening

- **Status:** Proposed
- **Intent:** External consumers can pin a version containing the fixes and read
  what changed.
- **Expected Outcome:** `VERSION` and `CHANGELOG.md` are bumped together with a
  Keep-a-Changelog entry describing the landed items, and a release is cut per
  the existing runbook. Since no consumer contract changes, the bump is
  minor/patch, not major.
- **Validation:** `bash tools/starters/acknowledgements/check-version.sh` is
  clean; the tag push produces a mirror tag plus a GitHub Release marked latest;
  `git subtree add … vX.Y.Z --squash` into a scratch repo reproduces the kit
  tree. Procedure: `docs/runbooks/acknowledgements-starter-release.md`.
- **Files:** `tools/starters/acknowledgements/VERSION`,
  `tools/starters/acknowledgements/CHANGELOG.md`.
- **Dependencies:** whichever of ATTRIB-018..-022 land; do not cut a release per
  item.
- **Confidence:** high — mechanical, and the release path is proven from the
  `v1.0.0` cut.

## Open Questions

- Does the orphaned-marker case (F2) warrant a hard error or a warning? A hard
  error is consistent with the marker-count gate, but it fails a consumer's CI
  on a block they deliberately retired mid-migration. A staged path (warn in
  this release, error in the next major) is the alternative.
- Should the mirrored kit carry its own CI workflow, or is a runnable
  `tests/` entrypoint enough for external forks (ATTRIB-022)?
- Is a `--version` flag on the dispatcher worth adding, so a vendored copy can
  report which kit version a consumer is actually running?

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
