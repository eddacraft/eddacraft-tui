export const meta = {
  name: 'complete-cib-items',
  description: 'Drive eligible Continuous Improvement Backlog (CIB) items through the full Anvil dev-workflow lifecycle: readiness/sizing gate → Worktrunk worktree → TDD → local gates → Council → PR → rebase + CI-verified merge.',
  whenToUse: 'When you want to work through and complete CIB items end-to-end. Reads plans/modules/continuous-improvement-backlog.aps.md, self-selects only ready, well-scoped items (defers design-gated / oversized / low-confidence ones), and honours the dev-workflow contract per item. Merge phase is sequential to avoid shared-file collisions on the CIB module + index + CI log.',
  phases: [
    { title: 'Select', detail: 'Read the CIB module; confirm requested items are non-done and resolve current status' },
    { title: 'Readiness', detail: 'Per item: read full spec + referenced files; decide proceed/defer with a size estimate' },
    { title: 'Implement', detail: 'Per item: wt worktree + TDD + local gates → council → push + PR (pipeline)' },
    { title: 'Merge', detail: 'Sequential per PR: flip status, rebase onto main, verify CI green, gh pr merge --rebase' },
    { title: 'Report', detail: 'Summarise items merged, PRs left open, deferred, and follow-ups' },
  ],
}

// ---------------------------------------------------------------------------
// Tunables (override via Workflow args)
//   items     : explicit CIB-ID allowlist (default = curated eligible set)
//   maxItems  : max items taken all the way this run (default 5)
//   dryRun    : if true, stop after local gates + council — do NOT push/PR/merge
//   noMerge   : if true, stop after opening PRs — do NOT merge (human merge gate)
//   date      : ISO date used in "Merged <date> via PR #N" status flips
// ---------------------------------------------------------------------------
// Workflow args may arrive as strings (e.g. "5"); coerce to a positive int.
const toPosInt = (v) => {
  const n = typeof v === 'string' && v.trim() !== '' ? Number(v) : v
  return Number.isInteger(n) && n > 0 ? n : null
}
const DEFAULT_ELIGIBLE = ['CIB-014', 'CIB-016', 'CIB-026', 'CIB-029', 'CIB-030']
const REQUESTED = (args && Array.isArray(args.items) && args.items.length) ? args.items : DEFAULT_ELIGIBLE
const MAX_ITEMS = (args && toPosInt(args.maxItems)) || 5
const DRY_RUN = !!(args && args.dryRun)
const NO_MERGE = !!(args && args.noMerge)
// Date for "Merged <date> via PR #N" flips. No default stamp — Date is
// unavailable in workflow scripts, so when unset the merge agent runs
// `date +%F` itself at flip time rather than baking in a stale constant.
const DATE = (args && typeof args.date === 'string') ? args.date : null
const CIB_PATH = 'plans/modules/continuous-improvement-backlog.aps.md'
const CI_LOG = 'plans/reviews/continuous-improvement-log.md'

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------
const SELECT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['items'],
  properties: {
    items: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['id', 'title', 'status', 'confidence', 'alreadyDone'],
        properties: {
          id: { type: 'string' },
          title: { type: 'string' },
          status: { type: 'string', description: 'Raw Status: line value, e.g. Draft / Proposed / In Progress / Done' },
          confidence: { type: 'string', enum: ['low', 'medium', 'high', 'unknown'] },
          alreadyDone: { type: 'boolean', description: 'true if Status matches a DONE pattern (Done/Complete/Merged/Released/Shipped)' },
        },
      },
    },
  },
}

const READINESS_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['proceed', 'decision', 'size', 'branch', 'apsMarked', 'reason', 'plan', 'files'],
  properties: {
    proceed: { type: 'boolean' },
    decision: { type: 'string', enum: ['proceed', 'defer'] },
    size: { type: 'string', enum: ['small', 'medium', 'large'] },
    branch: { type: 'string', description: 'conventional branch, e.g. fix/cib-029-required-anvil-version-semver' },
    apsMarked: { type: 'boolean', description: 'whether you marked the item In Progress (read-only gate normally leaves false)' },
    reason: { type: 'string', description: 'why proceed, or the concrete defer reason (design-gated / oversized / low-confidence / already-done)' },
    plan: { type: 'string', description: 'one-paragraph implementation approach if proceeding' },
    files: { type: 'array', items: { type: 'string' } },
  },
}

const IMPL_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['branch', 'worktreePath', 'committed', 'testsGreen', 'gatesRun', 'filesChanged', 'blocked', 'blockReason', 'summary'],
  properties: {
    branch: { type: 'string' },
    worktreePath: { type: 'string' },
    committed: { type: 'boolean' },
    testsGreen: { type: 'boolean' },
    gatesRun: { type: 'array', items: { type: 'string' } },
    filesChanged: { type: 'array', items: { type: 'string' } },
    blocked: { type: 'boolean' },
    blockReason: { type: ['string', 'null'] },
    summary: { type: 'string' },
  },
}

const COUNCIL_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['verdict', 'critical', 'major', 'findings'],
  properties: {
    verdict: { type: 'string', enum: ['pass', 'pass-with-minors', 'changes-required'] },
    critical: { type: 'integer' },
    major: { type: 'integer' },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['severity', 'title', 'file'],
        properties: {
          severity: { type: 'string', enum: ['critical', 'major', 'minor', 'nit'] },
          title: { type: 'string' },
          file: { type: 'string' },
        },
      },
    },
  },
}

const PR_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['opened', 'prNumber', 'prUrl', 'reason'],
  properties: {
    opened: { type: 'boolean' },
    prNumber: { type: ['integer', 'null'] },
    prUrl: { type: ['string', 'null'] },
    reason: { type: 'string' },
  },
}

const MERGE_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['merged', 'ciVerified', 'reason'],
  properties: {
    merged: { type: 'boolean' },
    ciVerified: { type: 'boolean', description: 'whether gh pr checks were confirmed green before merge' },
    mergeCommit: { type: ['string', 'null'] },
    reason: { type: 'string' },
  },
}

// ---------------------------------------------------------------------------
// Shared repo lifecycle rules (from CLAUDE.md / AGENTS.md / dev-workflow skill)
// ---------------------------------------------------------------------------
const REPO_RULES = `
Anvil repo lifecycle rules you MUST follow (see .claude/skills/dev-workflow/SKILL.md):
- Branches via Worktrunk: \`wt switch --create <branch>\` from main. Naming: feat/ fix/ docs/ chore/ test/.
  You are running inside a workflow agent: create your OWN wt worktree and work ONLY there.
  Never edit the shared main checkout — siblings park on it.
- TDD: write/extend the smallest failing test first, prove red, make it pass, refactor.
- Local CI-equivalent gates MUST be green before any PR. Fresh worktrees have no node_modules,
  so run \`pnpm install\` first or oxfmt falls back to a stale global (CIB-032). Then, for files touched:
    JS/docs: pnpm format:check && pnpm lint:check && pnpm typecheck && pnpm test
    Markdown/TOML: pnpm run format:check (oxfmt formats .md AND .toml)
    Rust: cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace
- If Cargo.lock changes, regenerate ACKNOWLEDGEMENTS.md in the SAME change:
    bash tools/starters/acknowledgements/generate-acknowledgements.sh
- Single-purpose PR per CIB item. Do NOT bundle multiple CIB items or dep bumps.
- main stays releasable: never push a broken branch. If gates cannot go green, STOP and report blocked.
- Conventional commits: <type>(<scope>): <subject>. No "Generated with Claude Code" footer; the
  Authored-By and APS trailers are added automatically — do not add them manually.

CIB-specific rules:
- Flip ONLY this item's own \`- **Status:**\` line in ${CIB_PATH} (distinct line per item → no cross-PR
  conflict). Set it to "In Progress" in your first commit (status flip lands in the PR, not a later reconcile).
- Do NOT hand-edit the \`N/M\` count in plans/index.aps.md or the CIB module header — it is advisory-derived
  (ADR-053 / CIB-022). Touching it causes cross-PR collisions; leave it alone.
- Append exactly one compact continuous-improvement note via `pnpm ci-log:append` (pending queue by default; harvest separately). Tracked path remains ${CI_LOG} (merge=union; concurrent harvests are safe).
`.trim()

// ===========================================================================
// PHASE 1 — SELECT
// ===========================================================================
phase('Select')

log(`Requested CIB items: ${REQUESTED.join(', ')}${DRY_RUN ? ' (DRY-RUN)' : ''}${NO_MERGE ? ' (NO-MERGE)' : ''}`)

const selectResult = await agent(
  `Read ${CIB_PATH}. For EACH requested CIB id below, find its \`### <id>:\` heading and report its title,
its current \`- **Status:**\` value, its \`- **Confidence:**\` value (low/medium/high, or "unknown" if absent),
and whether the status matches a DONE pattern (Done / Complete / Merged / Released/Shipped).

Requested ids: ${JSON.stringify(REQUESTED)}

Return one entry per requested id. If an id is not found, still return it with status "MISSING" and alreadyDone false.`,
  { label: 'select-cib', phase: 'Select', schema: SELECT_SCHEMA }
)

const candidates = selectResult.items
  .filter((it) => !it.alreadyDone && it.status !== 'MISSING')
  .slice(0, MAX_ITEMS)
const preSkipped = selectResult.items.filter((it) => it.alreadyDone || it.status === 'MISSING')

log(`Select: ${candidates.length} candidate(s); ${preSkipped.length} already-done/missing skipped`)

if (candidates.length === 0) {
  return {
    summary: 'No actionable CIB items — all requested items are already done or missing.',
    merged: [],
    prsOpen: [],
    deferred: [],
    preSkipped: preSkipped.map((it) => ({ id: it.id, status: it.status })),
  }
}

// ===========================================================================
// PHASE 2 — READINESS / SIZING GATE (read-only, parallel)
// ===========================================================================
phase('Readiness')

const gated = await parallel(
  candidates.map((c) => () =>
    agent(
      `Readiness + sizing gate for a CIB item before any code (Anvil dev-workflow rule 1). READ-ONLY: do not edit files.

Item: ${c.id} — ${c.title}
Current status: ${c.status} | confidence: ${c.confidence}

Steps:
1. Read the full \`### ${c.id}:\` block in ${CIB_PATH} (Intent / Expected Outcome / Validation / Files / Coordinates with / design gates).
2. Read the referenced Files/source it cites to confirm the work is well-defined and the cited paths still exist.
3. Decide:
   - "proceed": the item is concrete, low-ambiguity, and completable as ONE single-purpose PR (size small/medium).
   - "defer": ANY of — explicit unresolved design gates / "needs sign-off" / "blocked on" notes; low confidence
     needing design; oversized (would honestly need multiple PRs or a dedicated module); cited files/paths missing;
     or the Expected Outcome is not concretely testable. Give the precise defer reason.
4. If proceeding: propose a conventional branch name and a one-paragraph implementation plan, and list the files
   you expect to change (including a test file — TDD is required).

Be conservative: when in doubt between proceed and defer, DEFER with the reason. Leave apsMarked=false (read-only).`,
      { label: `ready:${c.id}`, phase: 'Readiness', schema: READINESS_SCHEMA }
    ).then((r) => ({ item: c, ready: r }))
  )
)

const cleared = gated.filter(Boolean).filter((g) => g.ready.proceed && g.ready.decision === 'proceed')
const deferred = gated.filter(Boolean).filter((g) => !(g.ready.proceed && g.ready.decision === 'proceed'))
log(`Readiness: ${cleared.length} cleared, ${deferred.length} deferred`)

if (cleared.length === 0) {
  return {
    summary: 'All candidate CIB items were deferred at the readiness gate.',
    merged: [],
    prsOpen: [],
    deferred: deferred.map((g) => ({ id: g.item.id, size: g.ready.size, reason: g.ready.reason })),
    preSkipped: preSkipped.map((it) => ({ id: it.id, status: it.status })),
  }
}

// ===========================================================================
// PHASE 3 — IMPLEMENT (pipeline: code+gates → council → push+PR)
// ===========================================================================
phase('Implement')

const built = await pipeline(
  cleared,
  // -- stage 1: branch + TDD + local gates --------------------------------
  (g) =>
    agent(
      `Implement this CIB item end-to-end on its own Worktrunk worktree.

Item: ${g.item.id} — ${g.item.title}
Proposed branch: ${g.ready.branch}
Plan: ${g.ready.plan}
Expected files: ${(g.ready.files || []).join(', ')}

${REPO_RULES}

Procedure:
1. \`wt switch --create ${g.ready.branch}\` (isolated worktree from main). Capture its absolute path (\`git rev-parse --show-toplevel\`). Run \`pnpm install\` if you will touch JS/docs/markdown/toml.
2. Re-read the \`### ${g.item.id}:\` block in ${CIB_PATH} for the exact Expected Outcome + Validation.
3. TDD: failing test first (prove red), minimal fix, refactor. Use the item's Validation command(s) as the acceptance check.
4. Flip ONLY ${g.item.id}'s own Status line to "In Progress" in ${CIB_PATH}.
5. Run ALL gates relevant to the files you touched (see rules). Iterate until green.
6. Append one compact CI-log note via `pnpm ci-log:append --agent claude --task "..." ...` (pending by default).
7. Commit with conventional message(s) referencing the item (e.g. "fix(cli): … (${g.item.id})"). Keep it single-purpose.
8. Do NOT push and do NOT open a PR — later stages do that.

Return: branch, worktreePath (absolute), committed, testsGreen, gatesRun (exact commands run green), filesChanged,
blocked, blockReason, summary. If gates cannot go green, set blocked=true, committed=false, explain — never leave a broken branch.`,
      { label: `impl:${g.item.id}`, phase: 'Implement', schema: IMPL_SCHEMA, agentType: 'autonomous' }
    ).then((impl) => ({ ...g, impl })),
  // -- stage 2: council review --------------------------------------------
  (g) => {
    if (!g || g.impl.blocked || !g.impl.committed) return g
    return agent(
      `Council-style pre-PR review of an implemented CIB branch.

Worktree: ${g.impl.worktreePath}
Branch: ${g.impl.branch}
Item: ${g.item.id} — ${g.item.title}

cd into the worktree, inspect \`git diff main...HEAD\`, and review for correctness, test adequacy, scope creep
(must be single-purpose for this one CIB item), security, and operational risk. Confirm the item's stated
Validation is actually satisfied by the tests. Count CRITICAL and MAJOR findings. Verdict: "changes-required"
if any CRITICAL/MAJOR, "pass-with-minors" if only minor/nit, else "pass".`,
      { label: `council:${g.item.id}`, phase: 'Implement', schema: COUNCIL_SCHEMA, agentType: 'council-reviewer' }
    ).then((council) => ({ ...g, council }))
  },
  // -- stage 3: address blocking findings, then push + open PR ------------
  async (g) => {
    if (!g || g.impl.blocked || !g.impl.committed) {
      return { ...g, pr: { opened: false, prNumber: null, prUrl: null, reason: g?.impl?.blockReason || 'implementation blocked' } }
    }
    // One remediation pass if council blocks.
    if (g.council && g.council.verdict === 'changes-required') {
      const fixed = await agent(
        `Address the blocking Council findings on a CIB branch, then re-verify gates.

Worktree: ${g.impl.worktreePath}
Branch: ${g.impl.branch}
Item: ${g.item.id} — ${g.item.title}
Blocking findings:
${g.council.findings.filter((f) => f.severity === 'critical' || f.severity === 'major').map((f) => `- [${f.severity}] ${f.title} (${f.file})`).join('\n')}

${REPO_RULES}

Fix every CRITICAL/MAJOR finding inside the worktree, keep the change single-purpose, re-run the relevant gates
until green, and commit. If a finding cannot be resolved without widening scope, set blocked=true and explain.
Return the same IMPL fields.`,
        { label: `fix:${g.item.id}`, phase: 'Implement', schema: IMPL_SCHEMA, agentType: 'autonomous' }
      )
      g = { ...g, impl: { ...g.impl, ...fixed } }
      if (g.impl.blocked || !g.impl.committed) {
        return { ...g, pr: { opened: false, prNumber: null, prUrl: null, reason: `council changes-required not resolved: ${g.impl.blockReason || 'unfixed'}` } }
      }
    }
    if (DRY_RUN) {
      return { ...g, pr: { opened: false, prNumber: null, prUrl: null, reason: 'dry-run: gates green + council clean, PR intentionally not opened' } }
    }
    const pr = await agent(
      `Open the PR for a reviewed, green CIB branch (Anvil finishing-a-branch contract).

Worktree: ${g.impl.worktreePath}
Branch: ${g.impl.branch}
Item: ${g.item.id} — ${g.item.title}

Steps (from inside the worktree):
1. Confirm committed + gates green (cheap re-check ok).
2. Push: \`git push -u origin ${g.impl.branch}\`.
3. If a post-merge test plan is warranted, extract it to plans/reviews/post-merge/<branch-slug>.md, commit + push.
4. \`gh pr create\` against main. Body: what/why, the ${g.item.id} reference, and a test plan listing the exact
   gate commands run. No "Generated with Claude Code" footer.
Return opened, prNumber, prUrl, and a short reason.`,
      { label: `pr:${g.item.id}`, phase: 'Implement', schema: PR_SCHEMA }
    )
    return { ...g, pr }
  }
)

// ===========================================================================
// PHASE 4 — MERGE (sequential; rebase + CI-verified; per-PR shared-file safety)
// ===========================================================================
phase('Merge')

const openedPRs = built.filter((g) => g && g.pr && g.pr.opened && g.pr.prNumber)
const notOpened = built.filter((g) => g && !(g.pr && g.pr.opened))

let mergeResults = []
if (NO_MERGE || DRY_RUN) {
  log(`Merge: skipped (${DRY_RUN ? 'dry-run' : 'noMerge'}). ${openedPRs.length} PR(s) left open for human merge.`)
} else {
  // Sequential: each merge rebases onto the just-updated main, so shared-file
  // edits (CIB module status line, CI log) reconcile deterministically.
  for (const g of openedPRs) {
    const m = await agent(
      `Merge a reviewed, green CIB PR into main — sequentially and safely. YOU are the merge gate: do not merge
until CI is confirmed green. main MAY be branch-protected (required reviews and/or status checks). Never bypass
protections — no admin override, no force-merge. If the merge is blocked by required reviews or insufficient
permissions, STOP and report it as a blocker (this is expected, not a failure).

Worktree: ${g.impl.worktreePath}
Branch: ${g.impl.branch}
PR: #${g.pr.prNumber} (${g.pr.prUrl})
Item: ${g.item.id} — ${g.item.title}

${REPO_RULES}

Procedure (from inside the worktree):
1. In ${CIB_PATH}, flip ${g.item.id}'s own Status line to "Merged ${DATE ?? '<today>'} via PR #${g.pr.prNumber}"${DATE ? '' : ' (run `date +%F` for <today>)'} and add a one-line
   "Summary:" compacting the item (matching the repo's compacted-done convention). Commit it onto the branch so the
   status flip lands in the merged history (NOT a later reconcile PR).
2. \`git fetch origin\` then rebase the branch onto \`origin/main\`. Resolve conflicts (the CIB module + CI log are the
   likely ones — keep BOTH sides on the union CI log; keep your own distinct Status line). Re-run the gates relevant to
   touched files until green. Push with \`--force-with-lease\` (check headRefOid first — a bot may have updated the PR).
3. Confirm \`gh pr view #${g.pr.prNumber} --json mergeable,mergeStateStatus\` is clean and poll \`gh pr checks ${g.pr.prNumber}\`
   until all required checks pass (give CI a reasonable window). If a required check fails or stays pending too long,
   STOP: set merged=false, ciVerified=false, and report — do NOT merge a red/unknown PR.
4. Only when CI is green: attempt \`gh pr merge ${g.pr.prNumber} --rebase --delete-branch\`. If it succeeds, confirm
   the PR shows MERGED and capture the merge commit. If it is rejected by branch protection (required reviews/checks
   or insufficient permissions), STOP: set merged=false, ciVerified=true, and report the blocker — do NOT bypass it.
5. Offer-safe cleanup: if the worktree is now safe (merged, branch deleted), \`wt remove\` it. Never remove an
   unmerged/dirty worktree.

Return merged, ciVerified, mergeCommit, reason.`,
      { label: `merge:${g.item.id}`, phase: 'Merge', schema: MERGE_SCHEMA, agentType: 'autonomous' }
    )
    mergeResults.push({ ...g, merge: m })
  }
}

// ===========================================================================
// PHASE 5 — REPORT
// ===========================================================================
phase('Report')

const merged = mergeResults.filter((r) => r.merge && r.merge.merged)
const mergeFailed = mergeResults.filter((r) => r.merge && !r.merge.merged)
const leftOpen = (NO_MERGE || DRY_RUN) ? openedPRs : mergeFailed

log(`Done: ${merged.length} merged, ${leftOpen.length} PR(s) left open, ${deferred.length} deferred, ${notOpened.length} not opened`)

return {
  merged: merged.map((r) => ({ id: r.item.id, pr: r.pr.prNumber, url: r.pr.prUrl, mergeCommit: r.merge.mergeCommit })),
  prsOpen: leftOpen.map((r) => ({ id: r.item.id, pr: r.pr.prNumber, url: r.pr.prUrl, reason: r.merge ? r.merge.reason : 'not merged this run' })),
  notOpened: notOpened.map((r) => ({ id: r.item.id, reason: r.pr ? r.pr.reason : 'unknown' })),
  deferred: deferred.map((g) => ({ id: g.item.id, size: g.ready.size, reason: g.ready.reason })),
  preSkipped: preSkipped.map((it) => ({ id: it.id, status: it.status })),
}
