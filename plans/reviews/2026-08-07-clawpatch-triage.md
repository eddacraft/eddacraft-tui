# Clawpatch triage — 2026-08-07 (verify-first batch from 2026-08-06 runs)

**Prior scans:** three completed reviews on 2026-08-06

| Run | Window | Scope |
| --- | ------ | ----- |
| `20260806T194543-cde571` | 19:45–19:46Z | review wave |
| `20260806T194809-eb5f63` | 19:48–19:49Z | review wave |
| `20260806T195005-a1a059` | 19:50–19:55Z | `--limit 200 --jobs 5` |

**Status command:** `clawpatch status`  
**Findings input:** `plans/audits/2026-08-06-clawpatch-periodic-scan.json`  
**Corpus SHA:** `824779d65` (`main` head at last run)  
**Predecessor:** `plans/reviews/2026-07-18-clawpatch-triage.md`

## Why this run

Entry state after the 2026-08-06 batch: **954 findings / 301 open / 2 open
high**. This pass is a **verify-first triage** of every finding produced by the
three runs (60 IDs), checked against current `main` source rather than trusting
scanner line refs alone.

Local clawpatch verdicts live in `.clawpatch/` (gitignored). Durable record is
this doc plus the exported audit summary.

Open count **301 → 298** after three local dispositions this session.

## Session verdicts recorded

| Finding | Verdict | Basis |
| ------- | ------- | ----- |
| Prerelease headings are mistaken for the requested final release (high) | **false-positive** | Idempotency check is `startsWith(\`## [${version}]\`)` — the closing `]` means `## [0.9.2-beta]` does **not** match version `0.9.2` |
| Unknown APS tags do not fail the validator (medium) | **false-positive** | Unknown tags intentionally emit **WARN**; only malformed tags are ERROR. Not silent accept |
| Composition drops surviving re-export edges when their target is overlaid (medium) | **wont-fix** | Known ADR-105 §3 imports-only exclusion; golden `reexport_call_divergence_is_exactly_the_recorded_gap` pins the gap |

### 1. Promote-changelog “prerelease prefix” — false-positive

Scanner claimed promoting `0.9.2` would treat an existing `## [0.9.2-beta]`
section as already done. Verified false:

```text
'## [0.9.2-beta] — …'.startsWith('## [0.9.2]')  → false
```

The pattern includes the closing bracket, so prerelease and longer patch
versions do not collide with the final token. No code change.

### 2. check-tags unknown tags — false-positive

`scripts/docs/check-tags.mjs` pushes `severity: 'WARN'` for tags absent from
the catalogue. Malformed tags remain ERROR. Scanner’s “confirmed-bug / does not
fail” framing overstates this: it is soft guidance, not a silent pass.

Note: `docs/guides/documentation-governance.md` still says the surface
“rejects” unknown tags — that prose is slightly stronger than WARN. Follow-up
docs wording only if desired; not a product defect.

### 3. Graph-cache re-export composition — wont-fix (known gap)

The finding restates the **recorded** composed-vs-cold divergence under ADR-105
§3. Closing it requires reconstructing re-exports during composition; until
then the golden must keep pinning the single allowed gap. Do not re-file as a
surprise defect.

## Remaining open high (1)

| Finding | Path | Disposition |
| ------- | ---- | ----------- |
| Concurrent tracked-log writers can lose or duplicate CI-log entries | `scripts/ci-log/lib.mjs` | **Confirmed** — fix queue P0 (internal tooling, not customer product) |

**Why confirmed:** `appendTrackedEntry`, `setWatermark`, and `harvestPending`
all do unlocked read-modify-write of the same tracked path. Harvest uses a
fixed `${logPath}.harvest-tmp`, so concurrent harvests can clobber each other,
and a harvest rename can overwrite a concurrent append. `merge=union` only
helps multi-branch *git* merges, not same-checkout writers.

**Recommended fix:** exclusive lock under the git common directory for all
tracked-log mutations; unique harvest temp path; hold the lock from read
through rename + pending deletion.

**Risk context:** harvest is bookkeeping-path traffic, usually serial. Severity
high is correct for data-loss *correctness*; blast radius is the CI-log, not
licence/auth product paths. Prefer a small CIB item + fix rather than an
emergency hotfix unless concurrent harvest is already burning notes.

## Confirmed-bug / security queue (still open from this batch)

Prioritised for CIB / small fix PRs — **not** fixed in this triage session.

| Pri | Severity | Title | Primary path | Notes |
| --- | -------- | ----- | ------------ | ----- |
| P0 | high | Concurrent tracked-log writers can lose or duplicate CI-log entries | `scripts/ci-log/lib.mjs` | See above |
| P1 | medium / security | GitHub OAuth callback can return before revoking the upstream bearer token | `apps/anvil-api/src/routes/auth-github.ts` | Fire-and-forget `revokeGitHubToken`; serverless freeze can leave token live. Await with bounded timeout |
| P1 | medium / data-loss | Baseline regeneration accepts tooling-failed surface output | `scripts/docs/docs-check.mjs` | `regenerateBaseline` only fails on empty/unparsable stdout; ignores `EXIT_TOOLING_FAILURE` verdict → can overwrite a known-good baseline |
| P2 | medium | Aggregate each bucket by recorded timestamp, not response order | `history-aggregation.ts` | Last array element wins, not latest `recorded_at` |
| P2 | medium | Selecting an affected file can show evidence for a different warning | `protection-overview.tsx` | Resolve only against `filteredWarnings`; silent fallback to first row |
| P2 | medium | Warning detail can show a previously selected warning for stale URL evidence | `routes/warnings.tsx` | Falls back to retained `selected` when evidence ID missing |
| P2 | medium | Critical severity missing from search state / warning table filter | `search-params.ts`, `warning-table.tsx` | Theme knows `critical`; URL + selector do not |
| P2 | medium | Date options accept impossible calendar dates | `scripts/ci-log/lib.mjs` | Filter/watermark accept non-calendar strings |
| P2 | medium | Failed existing-content lookup treated as create unless specifically 404 | `publish-public-contents.sh` | Auth/rate-limit failures clear `existing_sha` |
| P2 | medium | Dialog title/description rendered outside `DialogContent` | `components/ui/command.tsx` | Common shadcn pattern; a11y association may be wrong — verify Radix requirements before “fix” churn |
| P2 | medium | Enabled approval button has no action | `plan-detail.tsx` | No `onClick` when `actions_enabled` | Incomplete UI, not a logic crash |
| P3 | medium | Ignore key-repeat for command-palette shortcut | `dashboard-shell.tsx` | Holding ⌘K toggles open/closed rapidly |
| P3 | low | Failed status capture can concatenate curl status with fallback | `validate-publication-token.sh` | Classic `status="$(curl …) \|\| echo 000"` shape if present on another path — verify before patching |

## Contract-mismatch highlights (open)

| Title | Path | Disposition |
| ----- | ---- | ----------- |
| Non-token invites acknowledge scopes that are never persisted | `apps/anvil-api/src/routes/admin.ts` | **Likely real contract smell** — default invite returns `scopes` but only token-only path inserts `access_tokens`. Either persist grants or stop echoing scopes |
| Existing release tags can silently generate a mixed-version reference | `generate-anvil-public-reference.mjs` | **Risk** for release docs — fail closed when tag exists but an input is missing |
| Critical severity cannot be represented in dashboard search state | `search-params.ts` | Pair with warning-table P2 |
| DataTable rejects typed TanStack column definitions | `data-table.tsx` | Typing hygiene |
| Registry exposes mutable nested manifest state despite freezing | `modules/registry.ts` | Shallow `Object.freeze` — low practical risk |
| EmptyDescription props vs element (`div` vs `p`) | `empty.tsx` | Two near-duplicate findings; cosmetic a11y typing |
| Reject content types that merely contain `application/json` | `telemetry.ts` | Substring media-type check |

## Test-gap batch (26 open)

Advisory only. Clustered in:

- dashboard UI primitives / shell / command palette / sheets / badges / charts
- dashboard-server OpenAPI error-contract binding, protection-history cap order,
  workspace symlink error codes
- install PowerShell param-block detection, publish-public-contents oversize
  “no network” assertion, CLI skill-install path-renderer tripwire

Do **not** open one CIB per test gap. Prefer opportunistic coverage when
touching those files, or one grouped “dashboard component smoke” item later.

## Risk / build-release leftovers (sample)

Temp-file cleanup on failure, missing `anvil` binary misreported as doc content
failure, sidebar version hard-coding, generate-api temp dir leak on
`process.exit` — keep open; fix when next editing those scripts.

## Corpus summary (post-session)

| Metric | Entry (post-scan) | After triage |
| ------ | ----------------- | ------------ |
| Total findings | 954 | 954 |
| Open | 301 | **298** |
| Open highs | 2 | **1** |
| False-positive (lifetime) | 11 | **13** |
| Wont-fix (lifetime) | 567 | **568** |

### Open mix (298)

- **Severity:** 1 high · 154 medium · 143 low  
- **Triage labels (scanner):** ~67 confirmed-bug · ~103 test-gap · ~89 risk ·
  ~37 contract-mismatch · 2 docs-gap  

(Exact labels include backlog from earlier scans, not only this batch.)

### This batch only (60 findings)

| Status after triage | Count |
| ------------------- | ----- |
| Still open | 57 |
| false-positive | 2 |
| wont-fix | 1 |

Still-open batch mix: 1 high · 33 medium · 23 low; 13 confirmed-bug · 26
test-gap · 9 risk · 9 contract-mismatch.



## CIB intake (2026-08-07 bookkeeping)

Filed as Draft items **CIB-305..314** on bookkeeping branch
`docs/cib-305-clawpatch-intake` (numbers skip 301/302 reserved by
`docs/cib-301-302-dave-pack-04`). Awaiting operator membrane checkpoint for
promotion to Ready.

| CIB | Pri | Title |
| --- | --- | ----- |
| CIB-305 | P0 | Concurrent CI-log tracked writers can lose or duplicate entries |
| CIB-306 | P1 | GitHub OAuth callback can return before revoking the upstream token |
| CIB-307 | P1 | Baseline regeneration accepts tooling-failed surface output |
| CIB-308 | P2 | Protection history aggregation uses response order, not timestamp |
| CIB-309 | P2 | Warning selection can show evidence for a different warning |
| CIB-310 | P2 | Critical severity missing from dashboard search and warning filter |
| CIB-311 | P2 | CI-log date options accept impossible calendar dates |
| CIB-312 | P2 | publish-public-contents treats non-404 lookup failures as create |
| CIB-313 | P2 | Non-token admin invites acknowledge scopes that are never persisted |
| CIB-314 | P2 | Existing release tags can silently mix workspace and tag sources |

Residual clawpatch open items (test-gaps, low risks, cosmetic a11y typing)
remain advisory in `.clawpatch` and were **not** filed as one-CIB-per-gap.

## Residual backlog / next actions

1. **Fix or CIB-file the P0** ci-log lock + unique harvest temp.  
2. **P1 security:** await bounded GitHub token revocation in
   `auth-github.ts` (pair with existing auth session race doctrine from
   2026-07-18).  
3. **P1 data-loss:** honour tooling-failure verdict in
   `docs-check` baseline regeneration.  
4. **Dashboard UX cluster (P2):** history aggregation order, warning selection
   fallbacks, critical severity in filters/search.  
5. Leave test-gaps advisory; do not block product work on them.

No further local `clawpatch triage` dispositions this session beyond the three
recorded above. Remaining open items stay in `.clawpatch` until fixed or
explicitly closed.
