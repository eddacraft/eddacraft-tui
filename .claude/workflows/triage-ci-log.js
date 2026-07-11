export const meta = {
  name: 'triage-ci-log',
  description:
    'Harvest pending continuous-improvement notes, review tracked entries since the Last-triaged watermark, promote/absorb/leave into CIB, and advance the watermark.',
  whenToUse:
    'Weekly (or when picking NBI / draining CIB). Use when the CI-log pending queue is non-zero, when session-start shows pending notes, or when continuous-improvement evidence has not been triaged recently.',
  phases: [
    { title: 'Status', detail: 'pnpm ci-log:status — pending count, last entry, watermark' },
    { title: 'Harvest', detail: 'pnpm ci-log:harvest on a bookkeeping branch; commit tracked log if needed' },
    { title: 'Review', detail: 'pnpm ci-log:since -- --watermark; cluster themes' },
    { title: 'Disposition', detail: 'promote → CIB-NNN; absorb → already owned; leave → one-off' },
    { title: 'Watermark', detail: 'pnpm ci-log:set-watermark -- --today; append triage closeout note' },
  ],
}

const CIB_PATH = 'plans/modules/continuous-improvement-backlog.aps.md'
const CI_LOG = 'plans/reviews/continuous-improvement-log.md'
const GUIDE = 'docs/guides/continuous-improvement-log.md'

// ---------------------------------------------------------------------------
// Tunables
//   dryRun     : if true, do not edit CIB / watermark / harvest
//   harvest    : if false, skip harvest (default true)
// ---------------------------------------------------------------------------
const DRY_RUN = !!(args && args.dryRun)
const DO_HARVEST = !(args && args.harvest === false)

await agent({
  name: 'ci-log-triage',
  prompt: `You are triaging Anvil's continuous-improvement log.

## Contract
Read ${GUIDE} first. Surfaces:
- Pending queue: shared under git common dir (pnpm ci-log:*)
- Tracked log: ${CI_LOG}
- Backlog: ${CIB_PATH}

Dry run: ${DRY_RUN}
Harvest: ${DO_HARVEST}

## Steps
1. Run \`pnpm ci-log:status\` and capture pending count, last entry, last triaged.
2. If harvest is enabled and pending > 0:
   - Ensure you are on a bookkeeping branch (docs/* or chore/*) from main, or create one.
   - Run \`pnpm ci-log:harvest\`${DRY_RUN ? ' -- --dry-run' : ''}.
   - If not dry-run and harvest wrote the tracked log, commit it (conventional: docs(ci-log): harvest pending continuous-improvement notes).
3. Run \`pnpm ci-log:since -- --watermark\` (or --json) and cluster entries by Friction / Follow-up / theme.
4. Disposition each theme (not every row):
   - **promote** — file a new CIB-NNN in ${CIB_PATH} with Intent, Expected Outcome, Validation, Identified From (CI-log date + theme), Confidence. Only when outcome + validation exist.
   - **absorb** — already owned by CIB/module; record the ID in your report.
   - **leave** — one-off lesson; no item.
5. Do not invent filler CIB items. Process hygiene without an executable fix stays "leave".
6. If not dry-run: \`pnpm ci-log:set-watermark -- --today\`, then
   \`pnpm ci-log:append -- --agent claude --task "Weekly CI-log triage" --outcome "..." --follow-up "none"\`
   (use --tracked if this branch already commits the log).
7. Final report: pending harvested, entries reviewed, promoted IDs, absorbed IDs, left themes, new watermark.

## Hard rules
- Never rewrite historical log entries.
- Never put secrets in the log or CIB.
- Prefer themes over one CIB per row.
- Feature PRs must not be required to include the tracked log (pending exists for that).
`,
})
