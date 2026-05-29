export const meta = {
  name: 'complete-gh-issues',
  description: 'Triage open GitHub issues, cluster related ones, then drive the highest-value unambiguous units through the full Anvil dev-workflow lifecycle (APS gate → worktree → TDD → council → PR).',
  whenToUse: 'When you want to work through and complete open GitHub issues end-to-end. Honours the dev-workflow contract: APS truth gate, Worktrunk branch, TDD, local CI gates, Council review, single-purpose PR. Self-selects only high-value, low-ambiguity work.',
  phases: [
    { title: 'Triage', detail: 'List open issues, assess each, cluster related ones into ranked work-units' },
    { title: 'APS Gate', detail: 'Validate APS truth per selected unit; decide proceed/blocked' },
    { title: 'Implement', detail: 'Per unit: worktree + TDD + local gates → council → PR (pipeline)' },
    { title: 'Report', detail: 'Summarise PRs opened, units skipped, and follow-ups' },
  ],
}

// ---------------------------------------------------------------------------
// Tunables (override via Workflow args)
//   maxUnits : max work-units taken all the way to a PR this run (default 4)
//   dryRun   : if true, stop after local gates+council — do NOT open PRs
//   issues   : explicit issue-number allowlist (skips auto-triage selection)
// ---------------------------------------------------------------------------
const MAX_UNITS = (args && Number.isInteger(args.maxUnits)) ? args.maxUnits : 4
const DRY_RUN = !!(args && args.dryRun)
const ISSUE_ALLOWLIST = (args && Array.isArray(args.issues)) ? args.issues : null

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------
const ISSUE_LIST_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['issues'],
  properties: {
    issues: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['number', 'title', 'labels'],
        properties: {
          number: { type: 'integer' },
          title: { type: 'string' },
          labels: { type: 'array', items: { type: 'string' } },
        },
      },
    },
  },
}

const ASSESS_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['assessments'],
  properties: {
    assessments: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['number', 'title', 'area', 'kind', 'value', 'ambiguity', 'risk', 'effort', 'relatedTo', 'summary'],
        properties: {
          number: { type: 'integer' },
          title: { type: 'string' },
          area: { type: 'string', description: 'e.g. rust-engine, cli, docs, ci, tui, clawpatch, infra' },
          kind: { type: 'string', enum: ['bug', 'feature', 'test-hygiene', 'docs', 'chore', 'infra', 'refactor'] },
          value: { type: 'string', enum: ['high', 'medium', 'low'] },
          ambiguity: { type: 'string', enum: ['low', 'medium', 'high'] },
          risk: { type: 'string', enum: ['low', 'medium', 'high'] },
          effort: { type: 'string', enum: ['small', 'medium', 'large'] },
          relatedTo: { type: 'array', items: { type: 'integer' }, description: 'Other issue numbers that should ship in the same PR' },
          summary: { type: 'string', description: 'One sentence: what the issue actually asks for' },
        },
      },
    },
  },
}

const UNITS_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['units'],
  properties: {
    units: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['id', 'title', 'issues', 'area', 'kind', 'value', 'ambiguity', 'risk', 'eligible', 'skipReason', 'suggestedBranch', 'rationale'],
        properties: {
          id: { type: 'string', description: 'kebab-case slug' },
          title: { type: 'string' },
          issues: { type: 'array', items: { type: 'integer' } },
          area: { type: 'string' },
          kind: { type: 'string' },
          value: { type: 'string', enum: ['high', 'medium', 'low'] },
          ambiguity: { type: 'string', enum: ['low', 'medium', 'high'] },
          risk: { type: 'string', enum: ['low', 'medium', 'high'] },
          eligible: { type: 'boolean', description: 'true only if high/medium value AND low ambiguity AND risk != high' },
          skipReason: { type: ['string', 'null'] },
          suggestedBranch: { type: 'string', description: 'conventional branch, e.g. fix/clawpatch-test-hygiene' },
          rationale: { type: 'string' },
        },
      },
    },
  },
}

const APS_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['proceed', 'apsStatus', 'notes'],
  properties: {
    proceed: { type: 'boolean' },
    apsStatus: { type: 'string', enum: ['ready', 'in-progress', 'no-aps-needed', 'needs-plan-update', 'blocked'] },
    apsItemIds: { type: 'array', items: { type: 'string' } },
    notes: { type: 'string' },
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
  required: ['opened', 'prUrl', 'reason'],
  properties: {
    opened: { type: 'boolean' },
    prUrl: { type: ['string', 'null'] },
    reason: { type: 'string' },
  },
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function chunk(arr, size) {
  const out = []
  for (let i = 0; i < arr.length; i += size) out.push(arr.slice(i, i + size))
  return out
}

const REPO_RULES = `
Anvil repo lifecycle rules you MUST follow:
- Branches via Worktrunk: \`wt switch --create <branch>\` from main. Naming: feat/ fix/ docs/ chore/ test/.
- TDD: write/extend the smallest failing test first, prove red, make it pass, refactor.
- Local CI-equivalent gates MUST be green before any PR:
    JS/docs touched: pnpm format:check && pnpm lint:check && pnpm typecheck && pnpm test
    Markdown/TOML touched: pnpm run format:check (oxfmt formats .md AND .toml)
    Rust touched: cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace
- If Cargo.lock changes, regenerate ACKNOWLEDGEMENTS.md in the SAME change:
    bash tools/starters/acknowledgements/generate-acknowledgements.sh
- Single-purpose PRs: do not bundle APS bookkeeping or dep bumps with feature/bugfix work.
- main stays releasable: never push a broken branch. If gates cannot go green, STOP and report blocked.
- Conventional commits: <type>(<scope>): <subject>. Do NOT add the "Generated with Claude Code" footer or Authored-By trailer (added automatically).
`.trim()

// ===========================================================================
// PHASE 1 — TRIAGE
// ===========================================================================
phase('Triage')

let issueList
if (ISSUE_ALLOWLIST) {
  const fetched = await agent(
    `Run \`gh issue list --state open --limit 100 --json number,title,labels\` and return ONLY the issues whose number is in this allowlist: ${JSON.stringify(ISSUE_ALLOWLIST)}. Map labels to their name strings.`,
    { label: 'list-issues', phase: 'Triage', schema: ISSUE_LIST_SCHEMA }
  )
  issueList = fetched.issues
} else {
  const fetched = await agent(
    'Run `gh issue list --state open --limit 100 --json number,title,labels` in the repo and return every open issue. Map each label object to its name string. Do not filter — return all of them.',
    { label: 'list-issues', phase: 'Triage', schema: ISSUE_LIST_SCHEMA }
  )
  issueList = fetched.issues
}

log(`Triage: ${issueList.length} open issues to assess`)

const issueChunks = chunk(issueList, 6)
const assessGroups = await parallel(
  issueChunks.map((grp, i) => () =>
    agent(
      `You are triaging open GitHub issues for the Anvil repo. For EACH issue below, run \`gh issue view <number>\` to read the full body and comments, then assess it.

Issues in your batch:
${grp.map((x) => `- #${x.number} ${x.title} [${x.labels.join(', ')}]`).join('\n')}

For each issue produce: area, kind, value (impact if done), ambiguity (how clearly the issue specifies the fix — high ambiguity = under-specified/needs design), risk (blast radius / chance of breaking main), effort, relatedTo (numbers of OTHER issues that are so closely related they should ship in ONE PR — e.g. issues in the same subsystem doing the same kind of cleanup), and a one-sentence summary.

Be honest about ambiguity: infra/signing, cross-target rot, broad "umbrella" issues, and IPC/protocol surface work are usually high ambiguity. Narrow, well-described test-hygiene and single-bug fixes are usually low ambiguity.`,
      { label: `assess:${grp[0].number}-${grp[grp.length - 1].number}`, phase: 'Triage', schema: ASSESS_SCHEMA }
    )
  )
)

const assessments = assessGroups.filter(Boolean).flatMap((g) => g.assessments)
log(`Triage: assessed ${assessments.length} issues, clustering into work-units`)

const clustered = await agent(
  `You are the triage synthesiser. Given per-issue assessments, cluster related issues into work-units and rank them.

Rules:
- Group issues into ONE unit only when they are genuinely related (same subsystem + same kind of change) AND could honestly ship as a single coherent, single-purpose PR. Use the relatedTo hints but apply judgement — do not over-merge unrelated work.
- A unit is eligible ONLY IF: value is high or medium AND ambiguity is low AND risk is not high. Everything else: eligible=false with a concrete skipReason.
- Give each unit a kebab-case id, a conventional suggestedBranch (fix/… docs/… feat/… chore/… test/…), and a short rationale.
- Order units best-first (highest value, lowest ambiguity/risk first).

Assessments:
${JSON.stringify(assessments, null, 2)}`,
  { label: 'cluster-units', phase: 'Triage', schema: UNITS_SCHEMA }
)

const eligible = clustered.units.filter((u) => u.eligible)
const skipped = clustered.units.filter((u) => !u.eligible)
const selected = eligible.slice(0, MAX_UNITS)

log(`Triage complete: ${eligible.length} eligible, ${skipped.length} skipped, taking top ${selected.length} (maxUnits=${MAX_UNITS}${DRY_RUN ? ', DRY-RUN' : ''})`)

if (selected.length === 0) {
  return {
    summary: 'No eligible high-value, low-ambiguity work-units found.',
    eligible,
    skipped,
    units: [],
  }
}

// ===========================================================================
// PHASE 2 — APS GATE
// ===========================================================================
phase('APS Gate')

const gated = await parallel(
  selected.map((u) => () =>
    agent(
      `APS truth gate for a work-unit before implementation (Anvil dev-workflow rule 1).

Work-unit: ${u.id} — ${u.title}
Issues: ${u.issues.map((n) => '#' + n).join(', ')}
Area: ${u.area} | kind: ${u.kind}

Steps:
1. Read plans/index.aps.md and grep plans/ for anything matching these issues / this area.
2. Decide apsStatus:
   - "ready" / "in-progress": a valid APS item already covers this work.
   - "no-aps-needed": this is a small, self-contained fix/test-hygiene/docs change that the repo does not require a plan for. proceed=true.
   - "needs-plan-update": the work is real but needs an APS item/plan first. proceed=false.
   - "blocked": a hard blocker (dependency, decision, licensing). proceed=false.
3. If a Ready/In Progress APS item exists, you MAY mark it In Progress (edit the .aps.md), but do NOT do bookkeeping that belongs in a separate PR.
Return proceed (whether implementation may start now), apsStatus, apsItemIds, and notes.`,
      { label: `aps:${u.id}`, phase: 'APS Gate', schema: APS_SCHEMA }
    ).then((r) => ({ unit: u, aps: r }))
  )
)

const cleared = gated.filter(Boolean).filter((g) => g.aps.proceed)
const heldBack = gated.filter(Boolean).filter((g) => !g.aps.proceed)
log(`APS gate: ${cleared.length} cleared to implement, ${heldBack.length} held (needs-plan/blocked)`)

// ===========================================================================
// PHASE 3 — IMPLEMENT (pipeline: code+gates → council → PR)
// ===========================================================================
phase('Implement')

const results = await pipeline(
  cleared,
  // -- stage 1: branch + TDD + local gates -------------------------------
  (g) =>
    agent(
      `Implement this work-unit end-to-end on its own Worktrunk worktree.

Unit: ${g.unit.id} — ${g.unit.title}
Issues: ${g.unit.issues.map((n) => '#' + n).join(', ')}
Suggested branch: ${g.unit.suggestedBranch}
APS notes: ${g.aps.notes}

${REPO_RULES}

Procedure:
1. \`wt switch --create ${g.unit.suggestedBranch}\` to make an isolated worktree from main. Work ONLY inside that worktree — never touch the shared main checkout. Capture its absolute path (\`git rev-parse --show-toplevel\`).
2. \`gh issue view\` each issue to confirm the exact required change.
3. TDD: failing test first (prove red), minimal fix, refactor. If a change is genuinely untestable, note why.
4. Run ALL gates relevant to the files you touched (see rules). Iterate until green.
5. Commit with conventional message(s); reference the issues (e.g. "Fixes #1642"). Keep it single-purpose.
6. Do NOT push and do NOT open a PR — a later stage does that.

Return: branch, worktreePath (absolute), committed, testsGreen, gatesRun (exact commands you ran green), filesChanged, blocked, blockReason, summary. If you cannot make gates green, set blocked=true, committed=false, and explain — do not leave a broken branch.`,
      { label: `impl:${g.unit.id}`, phase: 'Implement', schema: IMPL_SCHEMA, agentType: 'autonomous' }
    ).then((impl) => ({ ...g, impl })),
  // -- stage 2: council review -------------------------------------------
  (g) => {
    if (!g || g.impl.blocked || !g.impl.committed) return g
    return agent(
      `Council-style pre-PR review of an implemented branch.

Worktree: ${g.impl.worktreePath}
Branch: ${g.impl.branch}
Unit: ${g.unit.id} — ${g.unit.title} (issues ${g.unit.issues.map((n) => '#' + n).join(', ')})

cd into the worktree, inspect \`git diff main...HEAD\`, and review for correctness, test adequacy, scope creep, security, and operational risk. Count CRITICAL and MAJOR findings. Verdict: "changes-required" if any CRITICAL/MAJOR, "pass-with-minors" if only minor/nit, else "pass".`,
      { label: `council:${g.unit.id}`, phase: 'Implement', schema: COUNCIL_SCHEMA, agentType: 'council-reviewer' }
    ).then((council) => ({ ...g, council }))
  },
  // -- stage 3: open PR --------------------------------------------------
  (g) => {
    if (!g || g.impl.blocked || !g.impl.committed) {
      return { unit: g?.unit, impl: g?.impl, council: g?.council, pr: { opened: false, prUrl: null, reason: g?.impl?.blockReason || 'implementation blocked' } }
    }
    if (g.council && g.council.verdict === 'changes-required') {
      return { ...g, pr: { opened: false, prUrl: null, reason: `council changes-required: ${g.council.critical} critical / ${g.council.major} major` } }
    }
    if (DRY_RUN) {
      return { ...g, pr: { opened: false, prUrl: null, reason: 'dry-run: gates green + council clean, PR intentionally not opened' } }
    }
    return agent(
      `Open the PR for a reviewed, green branch (Anvil finishing-a-branch + addressing-pr-reviews contract).

Worktree: ${g.impl.worktreePath}
Branch: ${g.impl.branch}
Issues: ${g.unit.issues.map((n) => '#' + n).join(', ')}
Council verdict: ${g.council ? g.council.verdict : 'n/a'}

Steps (from inside the worktree):
1. Confirm the branch is committed and gates are green (re-run a quick check if cheap).
2. Push the branch (\`git push -u origin ${g.impl.branch}\`).
3. Extract the post-merge test plan to plans/reviews/post-merge/<branch-slug>.md (do NOT leave it only in the PR body); commit + push that if created.
4. Open the PR with \`gh pr create\` against main. Body: what/why, "Fixes #N" for each issue, and a test plan with the gate commands actually run. No "Generated with Claude Code" footer.
Return opened, prUrl, and a short reason.`,
      { label: `pr:${g.unit.id}`, phase: 'Implement', schema: PR_SCHEMA }
    ).then((pr) => ({ ...g, pr }))
  }
)

// ===========================================================================
// PHASE 4 — REPORT
// ===========================================================================
phase('Report')

const opened = results.filter((r) => r && r.pr && r.pr.opened)
const blockedOrSkipped = results.filter((r) => r && (!r.pr || !r.pr.opened))

log(`Done: ${opened.length} PR(s) opened, ${blockedOrSkipped.length} unit(s) blocked/held, ${skipped.length} unit(s) skipped at triage`)

return {
  prsOpened: opened.map((r) => ({ unit: r.unit.id, issues: r.unit.issues, prUrl: r.pr.prUrl })),
  blockedOrSkipped: blockedOrSkipped.map((r) => ({
    unit: r && r.unit ? r.unit.id : 'unknown',
    issues: r && r.unit ? r.unit.issues : [],
    reason: r && r.pr ? r.pr.reason : 'unknown',
    council: r && r.council ? r.council.verdict : null,
  })),
  triageSkipped: skipped.map((u) => ({ unit: u.id, issues: u.issues, reason: u.skipReason })),
  heldByAps: heldBack.map((g) => ({ unit: g.unit.id, status: g.aps.apsStatus, notes: g.aps.notes })),
}
