# DOCGOV-008 Task 1 — Dead-Doc Audit

**Status:** Operator-signed-off 2026-05-23
**Audit date:** 2026-05-23
**Auditor:** Claude (Opus 4.7)
**Sign-off action:** Locked operator decisions captured below; Task 2 + Task 3 authorised.

## Operator decisions (2026-05-23)

| # | Question | Decision |
| - | -------- | -------- |
| 1 | v0.6.0-beta runbooks (3 files) | **Archive all three** to `docs/archive/runbooks/`. v0.7.x replacements exist for what's still relevant; v0.6.x-specific content is past. Relink public/** referrers to v0.7.x equivalents or drop the link where the reference no longer applies. |
| 2 | `anvil-full-architecture.md` + `edda-stack.md` | **Backfill, do not archive.** Both stay live; metadata + freshness anchor in DOCGOV-009. |
| 3 | Edda-named guides (`edda-memory.md`, `stack-migration.md`) | **Keep both live.** `anvil edda` surface is still in v0.7.x — under-used today but growing, not retiring. No public docs yet; public link removed but content kept. Backfill in DOCGOV-009. |
| 4 | Pitch-deck content (`docs/marketing/pitch-deck/`, `pitch-deck-oc/`, `pitch-deck-cc.md`) | **Move out of repo to `~/Projects/anvil-gtm-wip/`.** Not archived — relocated. `git rm` from this repo after copy; update any in-repo referrers. |
| 5 | `docs/public/edda-stack/` (7 files) | **Keep.** Public Edda Stack branding has been toggled off (public link removed) but content is retained — will come back. No archive. |
| 6 | Deeper sweep of `docs/public/anvil/**` + `docs/public/kindling/**` | **Keep all.** Trust the recommend-keep call. No archive moves in those subtrees. |

## Sign-off scope for Task 3

Task 3 archival moves authorised:
- 7 class-a files (Section "High-Confidence Archive Candidates")
- 8 class-b files (Section "Medium-Confidence Archive Candidates"), with in-repo relink in same commit
- 3 v0.6.0-beta runbooks (Section A) — class (c), `docs/archive/runbooks/` destination, with in-repo relink + stubs where external mirrors apply
- `docs/architecture/monorepo-structure.md` (Section B item) — class (b), 2 referrers

Task 3 archival moves **not** authorised (decision #2, #3, #5, #6): they remain live and get metadata backfill in DOCGOV-009.

Pitch-deck removal (decision #4) is a separate slice from Task 3 archival — handled as `git rm` after copying out of repo.

---

## Method

1. Enumerated 218 markdown files under `docs/**` outside `docs/archive/**`, `docs/indexes/**`, and `*.template.md`.
2. Captured `git log -1 --date=short` per candidate for last-meaningful-touch.
3. Inspected H1 + first ~10 lines of every plausible-dead candidate for self-declared status (`Status: Draft`, `Reference — synthesised from…`, "historical migration plan", explicit "supersedes" notes, branding mismatches).
4. For every flagged candidate, ran `rg -l <basename>.md` across `*.md` `*.rs` `*.ts` `*.toml` `*.yaml` `*.yml` and counted inbound references, split into: `total`, `active` (excluding `docs/archive/**`, `plans/archive/**`, `docs/indexes/**`), `archive`, `public` (any referrer under `docs/public/**` — these get externally published, so an archive needs a stub).
5. Classified each candidate by inbound-link class per the steps file:
   - **(a)** no inbound active refs → simple archive
   - **(b)** inbound active refs only inside repo → relink callers in same change
   - **(c)** referenced from `docs/public/**` or external mirror → archive with redirect stub
6. Buckets I scanned exhaustively: `docs/specs/`, `docs/plans/`, `docs/reviews/`, `docs/internal/`, `docs/architecture/` (suspects only), `docs/guides/` (suspects only), `docs/runbooks/` (suspects only), `docs/marketing/`, `docs/strategy/`, `docs/observability/`, `docs/testing/`.
7. Buckets I did **not** sweep file-by-file: `docs/public/anvil/**` (34 files — all currently live for v0.7.x), `docs/public/kindling/**` (16 files — kindling docs, live), `docs/public/aps/**` (9 files — APS public docs, live), `docs/public/edda-stack/**` (7 files — flag for review, see below), `docs/public/start-here/**` (3 files — entrypoint), `docs/guides/**` (non-suspect), `docs/policies/**` (3 files — release-cadence, resource-budget, editor-coexistence all live), `docs/architecture/**` (non-suspect — most as-built docs are governed and live).

> If you want exhaustive per-file inspection of the unswept buckets, say so and I'll do a deeper sweep before Task 3 moves. Otherwise the proposals below are what I'd execute on operator approval.

---

## High-Confidence Archive Candidates (class a — no active inbound refs)

| Path | Last commit | Active refs | Why dead | Destination |
| --- | --- | --- | --- | --- |
| `docs/guides/first-rust-release-rehearsal.md` | 2026-04-?? | 0 | Self-titled "Status: Draft, never executed" — rehearsal artefact for the first Rust cargo-dist release. That release shipped months ago. | `docs/archive/guides/first-rust-release-rehearsal.md` |
| `docs/guides/rust-cli-release-scope.md` | 2026-03-?? | 0 | Scoped specifically to "Rust CLI Release Scope — v0.3.x". We are on v0.7.x. | `docs/archive/guides/rust-cli-release-scope.md` |
| `docs/reviews/deep-research-report.md` | 2026-??-?? | 0 | One-off "Deep Code Review: anvil-001" assessment. The follow-ups it identified are either landed or tracked in APS. No live process points at it. | `docs/archive/reviews/deep-research-report.md` |
| `docs/plans/2026-03-09-aps-vs-gh-projects-trial-decision-space.md` | 2026-03-?? | 0 | Decision-space document for an APS vs GitHub Projects trial. The trial concluded — APS is the system of record. | `docs/archive/plans/2026-03-09-aps-vs-gh-projects-trial-decision-space.md` |
| `docs/plans/2026-03-11-verifiable-governance-technical-design.md` | 2026-03-?? | 0 | Design doc; whatever shipped from it is now in APS/code. | `docs/archive/plans/2026-03-11-verifiable-governance-technical-design.md` |
| `docs/plans/2026-03-17-lineage-authorship-confidence-v1.md` | 2026-03-?? | 0 | "v1" design doc; superseded by current authorship/lineage implementation in code. | `docs/archive/plans/2026-03-17-lineage-authorship-confidence-v1.md` |
| `docs/marketing/anvil-product-sheet.md` | 2026-04-14 | 0 | Last touched only by a cosmetic case-rename commit ("lowercase EddaCraft → eddacraft"). No live referrers. Marketing copy is moment-in-time. | `docs/archive/marketing/anvil-product-sheet.md` |

**Action:** `git mv` each. No stubs, no relinking. Baseline shrinks by N entries (the metadata findings for these files drop out).

---

## Medium-Confidence Archive Candidates (class b — relink small number of in-repo callers)

| Path | Last commit | Active refs | Referrers to relink | Why dead | Destination |
| --- | --- | --- | --- | --- | --- |
| `docs/specs/edda-api-contracts.md` | 2026-01-28 | 1 | `docs/archive/edda-pre-implementation/edda-component-dependencies.md` (already archived — does not need relinking) | Status: Draft. References `/docs/architecture/edda-system-architecture.md` which does not exist. Orphaned pre-anvil-rename spec. | `docs/archive/specs/edda-api-contracts.md` |
| `docs/specs/edda-authority-trust.md` | 2026-01-28 | 1 | Same — referrer is already in archive | Same | `docs/archive/specs/edda-authority-trust.md` |
| `docs/specs/edda-enforcement-hooks.md` | 2026-01-28 | 1 | Same | Same | `docs/archive/specs/edda-enforcement-hooks.md` |
| `docs/specs/2026-03-12-product-licensing-design.md` | 2026-03-12 | 2 | TBD — likely cross-refs from sibling 2026-03-* design specs | Design spec, March 2026. Anything shipped is in code; anything not shipped is in APS. | `docs/archive/specs/2026-03-12-product-licensing-design.md` |
| `docs/specs/2026-03-15-beta-auth-streamline-design.md` | 2026-03-15 | 2 | TBD — sibling design specs | Same | `docs/archive/specs/2026-03-15-beta-auth-streamline-design.md` |
| `docs/specs/2026-03-18-pitch-deck-direction-design.md` | 2026-03-18 | 1 | `plans/specs/2026-03-18-pitch-deck-production.md` (pitch-deck production plan; relink to archive or note supersession) | Status: "Reviewed (spec review: approved with notes — all addressed)". Pitch deck has long since produced. | `docs/archive/specs/2026-03-18-pitch-deck-direction-design.md` |
| `docs/specs/2026-03-27-rust-cli-cutover-design.md` | 2026-03-27 | 2 | TBD — cutover-era plans | Status: Draft. The Rust CLI cutover is shipped (per the v0.6 / v0.7 release history). | `docs/archive/specs/2026-03-27-rust-cli-cutover-design.md` |
| `docs/specs/command-safety-validation.md` | 2025-12-28 | 1 | TBD | Status: Draft, dated **2025-12-28**. Far older than any live work. Likely superseded by current command-safety implementation. | `docs/archive/specs/command-safety-validation.md` |
| `docs/guides/first-rust-release-rehearsal.md` | covered above |  |  |  |  |

**Action:** For each, `rg -l '<basename>.md'` to confirm exact referrers, then `git mv` + relink in same commit. Stubs not needed.

---

## Higher-Risk / Needs Operator Judgement

These have meaningful inbound refs and/or are referenced from `docs/public/**`. I recommend a decision before moving.

### A. Old release runbooks (v0.6.0-beta line)

| Path | Active refs | Public refs | Note |
| --- | --- | --- | --- |
| `docs/runbooks/v0.6.0-beta-release-runbook.md` | 16 | 2 (`docs/public/anvil/releases/{changelog,upgrade-notes}.md`) | Historical operator runbook for the v0.6.0-beta cut. Still valid for users on v0.6.x. |
| `docs/runbooks/v0.6.0-beta-security-note.md` | 16 | 3 | Companion security note for v0.6.0-beta. |
| `docs/runbooks/v0.6.x-to-v0.7.0-beta-migration.md` | 13 | 4 (`docs/public/anvil/releases/{changelog,upgrade-notes}.md`, `docs/public/anvil/{quickstart,overview}.md`) | Migration guide for v0.6.x→v0.7.0-beta. Likely still useful for late adopters. |

**Recommendation:** **Keep all three live**, not archive. They're historical-but-still-valid, with significant external exposure via `docs/public/**` and `RELEASE-PLAN.md`. Backfill metadata + freshness anchor in DOCGOV-009 instead.

Alternative: move them to a `docs/runbooks/releases/` subfolder for clarity. Same authority; cleaner taxonomy. Not required for DOCGOV-008.

**Operator decision needed:** keep / move / archive-with-stub.

### B. Transitional architecture docs

| Path | Active refs | Note |
| --- | --- | --- |
| `docs/architecture/anvil-architecture-evolution.md` | 8 | "Supersedes ADR-011; defines Current → H1 → H2 migration." Evolution is past — Rust kernel landed. But several ADRs and as-built docs still reference it. |
| `docs/archive/architecture/anvil-full-architecture.md` | 3 | Dated 2026-03-13, "Current vs Proposed End State" reference. Stale by ~2 months. |
| `docs/architecture/rust-architecture-overview.md` | 4 | "Compiled from APS modules KERN, RENG, RATS, PORT, RSTLAN, TUI (superseded)". Reference doc, no enforcement role. Status: ungoverned. |
| `docs/architecture/rust-kernel-spec.md` | 10 | "Proposed — H1 Implementation Target". H1 is done. But referenced from current as-built docs and ADRs. |
| `docs/architecture/monorepo-structure.md` | 2 | Self-says: "historical monorepo migration plan plus the archived target shape." Strong candidate. |
| `docs/architecture/edda-stack.md` | 6 | Old branding ("Kindling/Ember/Edda" three-layer doc). Still referenced from `docs/README.md`, `adapter-packages-as-built.md`, and the architecture README. |

**Recommendation:**
- **Archive immediately (class b):** `docs/architecture/monorepo-structure.md` — self-declared historical, only 2 refs to fix (likely `architecture/README.md` + maybe one more).
- **Backfill metadata, keep live (class — none):** `rust-kernel-spec.md`, `anvil-architecture-evolution.md`, `rust-architecture-overview.md`. These have too many active in-repo callers; relinking would be a separate effort. Backfill their governance tables in DOCGOV-009 with `Authority: Historical` or `Status: Deprecated` and a freshness pin to the tag where the transition landed.
- **Operator call:** `anvil-full-architecture.md` (3 refs, stale), `edda-stack.md` (6 refs, old branding).

### C. Edda-named guides

| Path | Active refs | Note |
| --- | --- | --- |
| `docs/guides/edda-memory.md` | 1 | Documents `anvil edda memory` commands. Confirm whether `anvil edda` subcommand still exists. |
| `docs/guides/stack-migration.md` | 2 | "How to coordinate schema changes across the Edda Stack layers." Live if Kindling/Ember/Edda integration still in scope. |

**Operator call:** are `anvil edda`/Kindling/Ember/Edda surfaces still live in v0.7.x? If yes → keep both, backfill in DOCGOV-009. If no → archive both.

### D. Internal docs

| Path | Active refs | Note |
| --- | --- | --- |
| `docs/internal/weave-feature-brief.md` | 3 | Referenced from active weave specs in `plans/specs/`. Weave is current work per the architecture README. **Recommend keep live**, backfill in DOCGOV-009. |
| `docs/internal/realtime-feed-contract.md` | 2 | Status: Draft. Referenced from `architecture/README.md` and `plans/modules/observability-foundation.aps.md`. **Recommend keep live**, backfill in DOCGOV-009 (it's a contract that observability dashboards consume). |

### E. Pitch deck content

`docs/marketing/pitch-deck/` (18 files), `docs/marketing/pitch-deck-oc/` (1 file), `docs/marketing/pitch-deck-cc.md` (1 file).

**Recommendation:** **Operator decides as a group**. Pitch decks are moment-in-time artefacts. If a deck shipped to a specific audience and is no longer the canonical pitch, the whole `pitch-deck/` subtree could move to `docs/archive/marketing/pitch-deck-<date>/` as a single batch. The README + status.md files inside suggest these may already be self-organising.

### F. `docs/public/edda-stack/` (7 files)

Not file-by-file inspected, but flagging the directory: if the "Edda Stack" public branding is being deprecated, the entire `docs/public/edda-stack/**` subtree may be a single archive move. Check with public-docs ownership before doing anything here — these likely publish to the public docs site.

---

## Recommended NOT to archive (inspected & kept)

These showed up in my candidate scan but on inspection are live or near-live. Backfill metadata in DOCGOV-009.

- `docs/archive/architecture/rust-architecture-endstate.md` — already governed (`Status: Live`, freshness anchored).
- `docs/architecture/rust-mcp-server-spec.md` — `Status: Ready`, owner `RMCPF-002` (active module).
- `docs/architecture/jsts-release-surfaces.md` — already governed, `Status: Live`, freshness 2026-05-20.
- `docs/architecture/oss-surface.md` — describes the public eddacraft OSS posture. Still authoritative.
- `docs/specs/watch-output-contract.md` — governed, active WOUT module owner; status was `Draft` during the DOCGOV-008 audit and is now `Live`.
- `docs/observability/local-tracing.md`, `docs/observability/namespace-registry.md` — live observability surface.
- `docs/strategy/borrow-adopt-candidates.md`, `docs/strategy/competitor-tier2-tracking.md` — live tracking docs with their own workflow.
- All `docs/policies/**` — release-cadence, resource-budget, editor-coexistence — current policy.
- `docs/runbooks/release-runbook.md` (the active one at `docs/guides/release-runbook.md`) — handled by Task 2.
- All other `docs/runbooks/**` not listed in section A.

---

## Coverage Notes

- I did not deep-inspect `docs/public/anvil/**` (34 files) or `docs/public/kindling/**` (16 files). My working assumption: these are current product docs and stay live. DOCGOV-009 backfill will pick them up. If any are stale (e.g. kindling content describing a removed surface) flag specifically and I'll spot-archive.
- I did not deep-inspect every `docs/architecture/**` and `docs/guides/**` file — only the obvious suspects. If you want me to widen the net (e.g. flag every guide whose freshness is undefined and whose last commit is >60 days), say so and I'll extend this audit.

---

## Decisions Required Before Task 3

To unlock the archive moves I need:

1. **Section A (v0.6.0-beta runbooks):** keep / move-to-`releases/` subfolder / archive-with-stub?
2. **Section B operator-call items:** `anvil-full-architecture.md` and `edda-stack.md` — archive or backfill?
3. **Section C (Edda-named guides):** is `anvil edda` still a live surface in v0.7.x?
4. **Section E (pitch-deck):** archive the entire `pitch-deck/` and `pitch-deck-oc/` subtrees as a batch?
5. **Section F (`docs/public/edda-stack/`):** is the public Edda Stack branding still live? If not, batch-archive.
6. **Coverage:** approve the recommended NOT-archive list, or ask for a deeper sweep before Task 3 runs.

Once those are answered, Task 3 archives high-confidence + approved-from-judgement-calls in a single commit per bucket (specs, plans, marketing, guides, architecture).

---

## Working artefact disposition

This file lives in `plans/execution/DOCGOV-008.audit.md` for the duration of DOCGOV-008. At Task 6 closeout it gets either deleted (if you want a clean execution trail) or moved to `plans/archive/audits/DOCGOV-008-dead-doc-audit.md` (if you want it as a historical record of what got archived and why). Default: keep as a historical record under `plans/archive/audits/`.

---

## Closeout evidence (2026-05-24)

### Files moved

| Slice | Files | Destination |
| --- | ---:| --- |
| Task 2 — release-runbook relocation | 1 | `docs/guides/release-runbook.md` → `docs/runbooks/release-runbook.md` |
| Task 3a — class-a archive | 7 | `docs/archive/{guides,reviews,plans,marketing}/` |
| Task 3b — class-b archive (specs) | 8 | `docs/archive/specs/` |
| Task 3c — v0.6.x runbook archive | 3 | `docs/archive/runbooks/` |
| Task 3d — monorepo-structure | 1 | `docs/archive/architecture/monorepo-structure.md` |
| Pitch-deck out-of-repo | 20 | `~/Projects/anvil-gtm-wip/marketing/` (`git rm` from this repo) |

### Relink scope

- ~30 in-repo files relinked to point at new archive paths (architecture as-built docs, runbooks, plans, RELEASE-PLAN.md, CHANGELOG.md, README.md, CONTRIBUTING.md, etc.).
- 5 public docs (`docs/public/anvil/{operations/security,releases/{changelog,upgrade-notes},quickstart,overview}.md`) had their GitHub URL references to v0.6.x runbooks stripped — text retained, link removed, per the "public docs don't link to internal docs" principle.

### Baseline delta (`docs/governance/docs-check.baseline.json`)

| Surface | Before | After | Δ |
| --- | ---:| ---:| ---:|
| metadata | 179 | 140 | −39 |
| tags | 1 | 1 | 0 |
| links | 137 | 134 | −3 |
| asbuilt-paths | 10 | 9 | −1 |

`metadata` shrinkage is the headline signal: archived docs leave the active-corpus glob, so their findings drop out. The remaining 140 metadata entries are docs still live in `docs/**` that need backfill — that's DOCGOV-009's scope.

### Validation

- `pnpm docs:check` → 7/7 surfaces passing (`metadata`, `tags`, `links`, `aps`, `adr`, `index-freshness`, `asbuilt-paths`).
- `pnpm docs:index:check` → 0 errors, 6 files checked.
- `pnpm format:check` → clean across 1373 files.
- Generated indexes regenerated via `pnpm docs:index` to reflect the post-archive corpus.

### Out of scope (deferred)

- Non-v0.6.x public docs still contain GitHub URLs into `docs/runbooks/*` and `docs/architecture/*-as-built.md`. The same "public docs don't link to internal docs" principle applies, but rewriting them is a broader public-docs editorial pass and is out of scope for DOCGOV-008. Suggest a separate work item under DOCGOV or DOCSYNC.
- DOCGOV-009 (`Backfill metadata on existing live documentation`) is unblocked.
- DOCGOV-010 (`Reorganise live documentation under canonical taxonomy`) depends on DOCGOV-009 completion.
