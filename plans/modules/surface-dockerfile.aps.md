<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Dockerfile Governance Surface (Track 3)

| ID       | Owner      | Status      |
| -------- | ---------- | ----------- |
| SURFDOCK | joshuaboys | In Progress |

**Last reviewed:** 2026-06-18

## Purpose

Bring `Dockerfile` to **T2 (Policy)** per
[2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§5.2, §8.3 row 3. Demand: 3. Blast: high. Strategic: supports.

Phase 3 deliverable — ranked #3 in Track 3, ships after Phase 1
(`surface-sql-migrations`) and Phase 2 (`surface-github-actions`).

## In Scope

- File detection: `Dockerfile`, `*.Dockerfile`, `Containerfile`.
- Pattern catalogue (per spec §8.3 row 3):
  - `ADD https://…` (use `RUN curl` + checksum instead)
  - `RUN curl … | sh` and similar pipe-to-shell installs
  - `:latest` base images
  - Running as `root` (missing `USER` directive)
  - `sudo` inside containers
  - Layered `apt-get install` without `--no-install-recommends`
  - Build secrets baked into layers (`ARG SECRET=` patterns)
- Suppression syntax: `# @anvil-ignore <ID>: <reason>`.
- Policy hook + drift baseline.

## Out of Scope

- Multi-stage build dependency analysis.
- Image vulnerability scanning (Snyk/Trivy/Grype territory).
- `docker-compose.yml` / `compose.yaml` — separate surface if demand arrives.
- Distroless / chainguard advice — keep neutral; flag root, do not
  prescribe specific base images.

## Interfaces

**Depends on:**

- [`operational-supplement`](../archive/modules/operational-supplement.aps.md) — check
  registry, drift schema versioning, per-track feature flag, file-presence
  guard.
- Rust suppression parser per
  [ADR-029](../decisions/029-suppression-parser-authority.md) — `#`
  comment style.

**Exposes:**

- Dockerfile pattern catalogue.

## Prerequisites

- OPSUP slices landed (see SURFSQL).
- [ADR-029](../decisions/029-suppression-parser-authority.md) Accepted.

## Ready Checklist

Change status to **Ready** when:

Promoted Draft → In Progress 2026-06-18. Checklist satisfied:

- [x] OPSUP slices landed — same set as SURFSQL/SURFGHA (OPSUP-001/-003/-005).
- [x] ADR-029 Accepted — `#` comment style already in the suppression parser.
- [x] Anvil's own Dockerfiles baselined (if any) — corpus scanned; the PR1
      catalogue is calibrated to be clean on standard pinned/`--no-install-recommends`
      Dockerfiles. FP target **N = 1%** (PYLAN precedent, operator-ratifiable).
- [x] External codebase validation candidate identified — a popular OSS repo
      shipping a Dockerfile (final pick recorded in SURFDOCK-006-validation).
- [x] Owner named — joshuaboys.

## Work Items

Delivered as slices mirroring SURFSQL/SURFGHA: library catalogue first, then
gate registration + flag, then validation.

### SURFDOCK-001 — File detection

- **Status:** Merged 2026-06-18 via PR #2777
- **Intent:** Identify `Dockerfile`/`Containerfile`/`*.Dockerfile` files.
- **Expected Outcome:** The three naming variants are detected; unrelated
  files (`Dockerfile.md`, `compose.yml`) are not.
- **Files:** `crates/anvil-checks/src/surface/dockerfile/scanner.rs`
- **Validation:** `cargo test -p eddacraft-anvil-checks --lib surface::dockerfile::scanner::tests::detects_dockerfile_names`
- **Confidence:** high

### SURFDOCK-002 — Build-hygiene / supply-chain catalogue

- **Status:** Merged 2026-06-18 via PR #2777
- **Intent:** Warn on the clearest build-hygiene / supply-chain risks.
- **Expected Outcome:** `ADD` of a remote URL, pipe-to-shell installs
  (`curl/wget … | sh`), `:latest` base images, `sudo` in layers, and
  `apt-get install` without `--no-install-recommends` are flagged, with
  logical-instruction assembly (`\` continuations), `#`-comment awareness and
  `# @anvil-ignore` suppression. Consolidates the anticipated network-fetch
  (-002) and base-image (-003) families.
- **Files:** `crates/anvil-checks/src/surface/dockerfile/{scanner,check}.rs`
- **Validation:** `cargo test -p eddacraft-anvil-checks --lib surface::dockerfile`
- **Confidence:** high

### SURFDOCK-005 — Gate/catalogue registration + flag gating

- **Status:** Merged 2026-06-18 via PR #2780
- **Intent:** Surface SURFDOCK in the gate behind `track.surface.dock`.
- **Expected Outcome:** `ANV-SURF-DOCK-001` registered + wired (warn-only,
  file-presence guarded), gated behind a `track.surface.dock` leaf flag under
  the OPSUP-005 `track.surface` umbrella, opt-in via `ANVIL_TRACK_SURFACE_DOCK=1`
  — the SURFSQL-005 / SURFGHA-006 pattern.
- **Validation:** `cargo test -p eddacraft-anvil commands::check_catalog`
- **Dependencies:** SURFDOCK-002, OPSUP-005 (Merged)
- **Confidence:** high

### SURFDOCK-006-validation — Anvil + external validation runs

- **Status:** Merged 2026-06-19 via PR #2798
- **Intent:** Prove the acceptance bar (FP < 1% on Anvil + ≥1 external repo).
- **Expected Outcome:** Re-run 2026-06-19 — **PASS**. The 2026-06-18 run was
  inconclusive (Anvil ships no Dockerfiles; one clean external file). A broader
  external corpus (`nvm-sh/nvm`, `dexidp/dex`, `bats-core/bats-core`,
  `golang-migrate/migrate` — **8 Dockerfiles**) closes the gap: 2 true positives
  (`apt-get` without `--no-install-recommends`) and correct true-negatives on
  pinned base images, at **0% FP**. A `SudoInRun` false positive (sudo installed
  as an apt package, not invoked) was found and fixed in the same PR; the Anvil
  leg is vacuously clean (no in-scope files), with the external corpus carrying
  the evidence. Evidence: `plans/reviews/2026-06-18-surface-validation.md`.
- **Validation:** FP report committed under `plans/reviews/`.
- **Dependencies:** SURFDOCK-002, SURFDOCK-005
- **Confidence:** medium

### SURFDOCK-007 — BuildKit heredoc `RUN` body support

- **Status:** Merged 2026-06-19 via PR #2801
- **Intent:** Close a false negative found during validation — `RUN`-family
  rules (`apt-get`, pipe-to-shell, `sudo`) were silently skipped inside
  `BuildKit` heredoc blocks (`RUN <<EOF … EOF`).
- **Expected Outcome:** The instruction assembler folds a heredoc body into its
  opening `RUN`, recognising `<<WORD`, `<<-WORD`, quoted `<<"WORD"`/`<<'WORD'`
  and `<<EOT bash` openers, and closing on the delimiter line. The `<<` redirect
  token is stripped so the folded instruction reads as a clean `RUN <commands>`
  (command-position rules like `sudo` still resolve). An arithmetic/shift `<<`
  (`$((1<<2))`) is not mistaken for a heredoc. Found via `docker/awesome-compose`
  (5 real `apt-get`-in-heredoc findings previously missed); no new FPs.
- **Files:** `crates/anvil-checks/src/surface/dockerfile/scanner.rs`
- **Validation:** `cargo test -p eddacraft-anvil-checks --lib surface::dockerfile`
  (6 new heredoc cases) + re-scan of `docker/awesome-compose` (2 → 7 findings,
  all true positives).
- **Dependencies:** SURFDOCK-002
- **Confidence:** high

### Deferred risk families

Root-user (missing `USER`) needs whole-file / final-stage analysis (multi-stage
FP-prone); build-secret `ARG`/`ENV` detection is FP-prone line-by-line; and
implicit-`latest` (a bare `FROM image` with no tag) needs build-stage-name
tracking to avoid flagging `FROM <stage>`. Revisit with the SURFDOCK-006
dogfood signal (mirrors the SURFSQL-003 / SURFGHA deferrals).

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| `:latest` rule trips on dev-only images | Low | Allow `# @anvil-ignore` with reason — that is the policy |
| Multi-stage builds confuse line-by-line analysis | Medium | Document limitation; flag at the offending line, no cross-stage reasoning in T2 |

## Open Questions

- [ ] `docker-compose.yml` — separate surface or fold in here?
- [ ] BuildKit-specific syntax (`# syntax=`) — what to do with it?
