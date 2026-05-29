<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Dockerfile Governance Surface (Track 3)

| ID       | Owner | Status |
| -------- | ----- | ------ |
| SURFDOCK | —     | Draft  |

**Last reviewed:** 2026-04-26

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

- [`operational-supplement`](./operational-supplement.aps.md) — check
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

- [ ] OPSUP slices landed.
- [ ] ADR-029 Accepted.
- [ ] Anvil's own Dockerfiles baselined (if any).
- [ ] External codebase validation candidate identified.
- [ ] Owner named.

## Work Items

Anticipated:

- SURFDOCK-001: File detection (Dockerfile naming variants).
- SURFDOCK-002: Network-fetch and pipe-to-shell rules.
- SURFDOCK-003: Base-image and root-user rules.
- SURFDOCK-004: Build-secret rules.
- SURFDOCK-005: Suppression + policy hook + drift baseline wiring.
- SURFDOCK-006: Anvil + external validation runs.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| `:latest` rule trips on dev-only images | Low | Allow `# @anvil-ignore` with reason — that is the policy |
| Multi-stage builds confuse line-by-line analysis | Medium | Document limitation; flag at the offending line, no cross-stage reasoning in T2 |

## Open Questions

- [ ] `docker-compose.yml` — separate surface or fold in here?
- [ ] BuildKit-specific syntax (`# syntax=`) — what to do with it?
