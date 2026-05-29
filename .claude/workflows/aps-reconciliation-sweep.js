export const meta = {
  name: 'aps-reconciliation-sweep',
  description:
    'Sweep active APS modules for semantic drift the deterministic scripts miss, adversarially verify each finding, and write a prioritized reconciliation report under plans/audits/.',
  whenToUse:
    'When you want to reconcile plans/ APS modules against reality — status-vs-body contradictions, unverified shipped/PR-claim provenance, archived-ref link rot, stale review dates, status-casing traps — beyond what `aps:index:check` / `aps:drift` / `aps:active-lint` already gate. Read-only: produces a report, edits no module.',
  phases: [
    { title: 'Discover', detail: 'enumerate active module files + capture today' },
    { title: 'Baseline', detail: 'run deterministic APS scripts to capture the mechanical floor' },
    { title: 'Sweep', detail: 'one agent per module batch flags semantic drift' },
    { title: 'Verify', detail: 'adversarially check each finding against audit notes + git' },
    { title: 'Synthesize', detail: 'merge floor + verified findings into a report' },
  ],
}

// ---------------------------------------------------------------------------
// Tunables (override via Workflow args)
//   batch   : modules per sweep agent (default 8)
//   modules : explicit filename allowlist (skips filesystem discovery)
//   date    : ISO date stamp for the report (skips `date +%F` discovery)
// ---------------------------------------------------------------------------
// Workflow args may arrive as strings (e.g. "8"); coerce to a positive int.
const toPosInt = (v) => {
  const n = typeof v === 'string' && v.trim() !== '' ? Number(v) : v
  return Number.isInteger(n) && n > 0 ? n : null
}
const BATCH = (args && toPosInt(args.batch)) || 8

const DISCOVERY_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['date', 'modules'],
  properties: {
    date: { type: 'string', description: 'today as YYYY-MM-DD (from `date +%F`)' },
    modules: {
      type: 'array',
      items: { type: 'string' },
      description: 'basenames of every plans/modules/*.aps.md file',
    },
  },
}

const SWEEP_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['findings'],
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['module', 'kind', 'severity', 'claim', 'evidence'],
        properties: {
          module: { type: 'string', description: 'module filename, e.g. lang-rust.aps.md' },
          location: { type: 'string', description: 'heading or line hint' },
          kind: {
            type: 'string',
            enum: [
              'status-body-mismatch',
              'prod-wireup-unverified',
              'archived-ref',
              'stale-review-date',
              'pr-claim-unverified',
              'status-casing',
              'count-narrative-mismatch',
              'other',
            ],
          },
          severity: { type: 'string', enum: ['high', 'medium', 'low'] },
          claim: { type: 'string', description: 'what the module asserts' },
          evidence: { type: 'string', description: 'why this looks like drift' },
        },
      },
    },
  },
}

const VERDICT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['real', 'intentional', 'reason', 'suggested_fix'],
  properties: {
    real: { type: 'boolean', description: 'true if this is genuine drift needing reconciliation' },
    intentional: {
      type: 'boolean',
      description: 'true if an audit note / project-context explicitly explains the state',
    },
    reason: { type: 'string' },
    suggested_fix: { type: 'string', description: 'concrete reconciliation step, or "none"' },
  },
}

phase('Discover')
const discovery =
  args && Array.isArray(args.modules) && args.modules.length && typeof args.date === 'string'
    ? { date: args.date, modules: args.modules }
    : await agent(
        `From the repo root, return today's date and the active APS module set:
  date +%F
  ls -1 plans/modules/*.aps.md   (return only the basenames, e.g. "lang-rust.aps.md")
Do not read the files — just enumerate them.`,
        { label: 'discover:modules', phase: 'Discover', schema: DISCOVERY_SCHEMA, agentType: 'general-purpose' },
      )

const DATE = discovery.date
const MODULES = discovery.modules
const batches = []
for (let i = 0; i < MODULES.length; i += BATCH) batches.push(MODULES.slice(i, i + BATCH))
log(`Reconciling ${MODULES.length} modules in ${batches.length} batches (date ${DATE})`)

phase('Baseline')
const baseline = await agent(
  `Run the three deterministic APS checks from the repo root and report their output concisely. These are the mechanical floor the semantic sweep should NOT re-derive.

Run:
  node scripts/aps/index-counts.mjs --check --json
  node scripts/aps/drift-check.mjs --json
  node scripts/aps/active-lint.mjs --json   (if the canonical 'aps' binary is absent it may error — note that and continue; absence is not a finding)

Return a prose summary: count drifts, drift-check findings, active-lint errors, and the specific module IDs involved if any. Do NOT read module files.`,
  { label: 'baseline:scripts', phase: 'Baseline', agentType: 'general-purpose' },
)

phase('Sweep')
const swept = await pipeline(
  batches,
  // Stage 1 — semantic sweep of a batch of modules
  (batch, _orig, idx) =>
    agent(
      `You are reconciling Anvil APS plan modules against reality. Read each of these files under plans/modules/:
${batch.map((m) => `  - plans/modules/${m}`).join('\n')}

Flag ONLY semantic drift the deterministic scripts (count drift, status-pattern drift) cannot catch:
- status-body-mismatch: header/module Status (Proposed/Ready/In Progress/Done/Blocked, or work-item Merged/Released-Shipped/Complete) contradicts what the body or work-item bodies describe.
- prod-wireup-unverified: an item claims shipped/live/complete but references no call site, PR, or crate path as evidence.
- archived-ref: the module links to or depends on a module now under plans/archive/modules/ without acknowledging the archive.
- stale-review-date: a "Last reviewed:" date older than ~6 months on an active (Ready/In Progress) module.
- pr-claim-unverified: a "Merged ... via PR #N" or "shipped in vX.Y (<commit> · <date>)" claim — flag for verification (do not verify here). NOTE: the project convention cites the RELEASE-TAG commit + release date uniformly, not the implementing commit — do not flag a shared release commit as "wrong" just because it differs from the authoring commit.
- status-casing: lowercase work-item status (e.g. "completed"/"merged") that parses but trips case-sensitive DONE_PATTERNS — canonical is capitalized.
- count-narrative-mismatch: header progress N/M contradicts a narrative count elsewhere in the module.

IMPORTANT: many modules carry an explicit "Audit note"/"Rescope" block that intentionally explains an unusual state. If the module's own prose explains it, set severity 'low' and say so in evidence — do not over-flag. Cite the module filename and a location hint. Emit no finding for a clean module.`,
      { label: `sweep:batch-${idx + 1}`, phase: 'Sweep', schema: SWEEP_SCHEMA, agentType: 'general-purpose' },
    ),
  // Stage 2 — adversarially verify each finding from this batch
  (sweep) =>
    parallel(
      (sweep?.findings ?? []).map((f) => () =>
        agent(
          `Adversarially verify whether this APS reconciliation finding is REAL drift or a false positive. Default to skepticism — most flagged items are intentional or already-explained.

Finding:
  module:   plans/modules/${f.module}
  location: ${f.location ?? '(unspecified)'}
  kind:     ${f.kind}
  claim:    ${f.claim}
  evidence: ${f.evidence}

Steps:
1. Read the relevant section of plans/modules/${f.module}. Check for an "Audit note"/"Rescope"/status-extension block that intentionally accounts for the state. If present, the finding is likely intentional.
2. kind 'pr-claim-unverified': run \`gh pr view <N> --json state,mergedAt,title 2>/dev/null\` or \`git log --oneline --grep "#<N>"\`. ALSO check project convention before flagging a commit hash as "wrong": grep other modules for the same "Released/Shipped via <tag>" line — if they uniformly cite the same commit, that is the RELEASE-TAG commit by convention and is CORRECT, not drift.
3. kind 'archived-ref': confirm the referenced module is actually under plans/archive/modules/ via \`ls plans/archive/modules/\`.
4. kind 'prod-wireup-unverified': do NOT try to prove wire-up; mark real only if the module asserts "live/shipped" with zero referenced evidence.
5. kind 'stale-review-date': real only if the module is Ready/In Progress AND the date is genuinely old.

Set real, intentional, a one-line reason, and a concrete suggested_fix (exact status flip, link repoint, or "verify X") — or suggested_fix "none" if not real.`,
          { label: `verify:${f.module}:${f.kind}`, phase: 'Verify', schema: VERDICT_SCHEMA, agentType: 'general-purpose' },
        ).then((v) => ({ ...f, verdict: v })),
      ),
    ),
)

const allVerified = swept.flat().filter(Boolean)
const actionable = allVerified.filter((f) => f.verdict?.real && !f.verdict?.intentional)
const intentional = allVerified.filter((f) => f.verdict?.real && f.verdict?.intentional)
log(
  `Sweep complete: ${allVerified.length} verified, ${actionable.length} actionable, ${intentional.length} real-but-intentional`,
)

phase('Synthesize')
const report = await agent(
  `Write a prioritized APS reconciliation report to plans/audits/${DATE}-aps-reconciliation-sweep.md (create it with the Write tool; do NOT modify any plans/modules/ file — this sweep is advisory/read-only).

Inputs:

=== Deterministic baseline (mechanical floor) ===
${baseline}

=== Actionable semantic findings (real drift, not explained by an audit note) — ${actionable.length} ===
${JSON.stringify(actionable, null, 2)}

=== Real-but-intentional findings (state is explained; informational) — ${intentional.length} ===
${JSON.stringify(intentional, null, 2)}

Report structure (Markdown):
1. Title + date (${DATE}) + one-line scope note (advisory sweep of ${MODULES.length} active modules).
2. "## Summary" — counts: baseline floor, actionable, intentional. State plainly if there is little to act on.
3. "## Actionable drift (prioritized)" — ordered high→medium→low: module | kind | claim | suggested fix. If empty, say so.
4. "## Real but intentional (no action)" — brief list so reviewers know these were considered.
5. "## Method & limits" — read-only; did not trace Rust call sites for prod-wireup claims; PR/tag claims best-effort via gh/git.

Follow repo prose conventions: no time estimates, wrap reasonably, repo-relative paths in backticks. After writing, return the path and a 3-sentence executive summary.`,
  { label: 'synthesize:report', phase: 'Synthesize', agentType: 'general-purpose' },
)

return {
  reportPath: `plans/audits/${DATE}-aps-reconciliation-sweep.md`,
  actionable: actionable.length,
  intentional: intentional.length,
  summary: report,
}
