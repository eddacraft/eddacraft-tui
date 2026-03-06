# Consolidated Code Review: Edda + Ember Feature Branches

**Date:** 2026-03-05
**Reviewer:** Claude (automated, 6 parallel review agents)
**Branches reviewed:**

```
main
 └─ feat/ember-core        (40 files, ~5k lines)
     └─ feat/ember-cli     (+8 files, ~700 lines)
         └─ feat/ember-docs (+1 file, ~300 lines)
             └─ feat/edda-core   (+23 files, ~4.7k lines)
                 └─ feat/edda-cli    (+7 files, ~1k lines)
                     └─ feat/edda-integration (+6 files, ~1.6k lines)
```

**Total:** ~85 files, ~13.3k lines added

---

## Overall Assessment: NEEDS_CHANGES

The architecture is strong — clean layered design, Zod-first schemas, consistent
ESM patterns, meaningful test coverage, and good separation of concerns. No
security vulnerabilities found. The issues below are primarily correctness bugs,
doc/implementation mismatches, and systematic code duplication.

---

## Critical Issues (10 — must fix)

### 1. SQL ORDER BY interpolation is fragile

**Branch:** feat/ember-core
**File:** `packages/edda-stack/src/ember/proposal-store.ts:~259`

The `orderByField` and `orderDirection` strings are interpolated directly into
the SQL string. Although `orderByField` is derived from a switch on a controlled
enum, the pattern is fragile — if `sort_by` ever accepts an arbitrary string,
this becomes SQL injection. Use a `Record<SortField, string>` map with an
exhaustive check to make the safety guarantee explicit.

### 2. `ObservationGroup` defined twice with incompatible shapes

**Branch:** feat/ember-core
**Files:** `packages/edda-stack/src/ember/candidate-service.ts:1922` vs
`packages/edda-stack/src/ember/aggregator-service.ts:1308`

`CandidateService` defines a local `ObservationGroup` (with `key` and
`observations`), while `AggregatorService` exports a richer `ObservationGroup`
(with `id`, `grouping_type`, `observation_ids`, `session_ids`, `signals`, etc.).
`AggregatorService` cannot plug into `CandidateService` as its `aggregator`
dependency without an adapter. Unify the types or rename the local one to make
the distinction explicit.

### 3. `anvil ember dismiss` documented but does not exist

**Branch:** feat/ember-docs
**File:** `docs/guides/ember-candidates.md:117-122`

The guide documents `anvil ember dismiss <id> --reason "..."` as a subcommand.
It is not registered in the CLI. Running it will produce an unknown command
error. Either implement the command before this doc ships, or remove/mark the
section as planned.

### 4. `anvil ember promote` docs omit required flags

**Branch:** feat/ember-docs
**File:** `docs/guides/ember-candidates.md:112-116`

The guide shows `anvil ember promote <id>` as a valid bare invocation and treats
`--reason` as optional. In the implementation, both `--reason` and `--by` are
`.requiredOption()`. The bare form will fail. The `--by <name>` option is not
mentioned anywhere in the guide.

### 5. Comma-separated `--type` does not work

**Branch:** feat/ember-docs
**File:** `docs/guides/ember-candidates.md:91`

The example `anvil ember list --type pattern,lesson` implies multiple types can
be passed comma-separated. `parseType()` passes the raw value to
`ProposalTypeSchema.safeParse`, which validates as a single enum member.
`"pattern,lesson"` will fail validation.

### 6. `getEvolutionChain` infinite loop on corrupt data

**Branch:** feat/edda-core
**File:** `packages/edda-stack/src/edda/evolution-service.ts:153-169`

The root-finding `while` loop reads `visited.has(previousId)` as a cycle guard
but never calls `visited.add()` for nodes it traverses. A corrupt two-node
supersedes cycle (A supersedes B, B supersedes A) would loop forever.

**Fix:** Call `visited.add(root.id)` on each iteration of the backward
root-finding loop before overwriting `root`.

### 7. `trackChange` passes invalid git `--author` format

**Branch:** feat/edda-core
**File:** `packages/edda-stack/src/edda/version-tracker.ts:41`

Git's `--author` flag requires `"Name <email>"` format. At every call site the
actor is a free-form string like `'joshua'`. This causes `git commit` to exit
with an error, but is masked in tests because `VersionTracker` is always mocked.

**Fix:** Normalise the actor to `"<actor> <actor@edda>"` in `trackChange`, or
enforce the expected format at the interface level.

### 8. `supersedeMemory` has no status guard

**Branch:** feat/edda-integration
**File:** `packages/edda-stack/src/edda/evolution-service.ts`

`supersedeMemory` does not check `oldMemory.status`. Calling it on a `retired`
or `superseded` memory will overwrite the terminal status and reason, corrupting
the audit trail.

**Fix:**
```typescript
if (oldMemory.status !== 'active') {
  throw new Error(
    `Cannot supersede memory ${oldMemoryId}: status is '${oldMemory.status}', must be 'active'.`
  );
}
```

### 9. Double output in `--json` error paths (systematic)

**Branches:** feat/ember-cli, feat/edda-cli
**Files:** `ember/promote.ts`, `ember/show.ts`, `edda/show.ts`,
`edda/promote.ts`, `edda/retire.ts`, `edda/trace.ts`

In catch blocks, `console.error(chalk.red(...))` fires unconditionally after the
JSON output branch. CI consumers using `--json` get both JSON on stdout and
plain-text on stderr. The `else` branch is missing.

### 10. Inconsistent exit codes for "storage not found"

**Branch:** feat/edda-cli
**Files:** `edda/list.ts`, `edda/trace.ts` return exit 0; `edda/show.ts`,
`edda/promote.ts`, `edda/retire.ts` throw CliError (exit 1)

Scripting against the CLI is unpredictable. Choose a consistent convention and
apply it across all commands.

---

## Major Suggestions (14 — should fix)

### 1. Race condition in `processSession` candidate limit

**Branch:** feat/ember-core
**File:** `packages/edda-stack/src/ember/candidate-service.ts:~162`

N concurrent `processSession` calls could race past `max_candidates` before any
single call commits. Low risk with SQLite WAL (single writer) but latent for
concurrent backends.

### 2. `EscalationRule` assumes array order = temporal order

**Branch:** feat/ember-core
**File:** `packages/edda-stack/src/ember/rules/escalation.rule.ts:52-55`

Severity signals are extracted in array order. After group merges, array order
may not reflect temporal sequence. Sort by severity rank before comparing.

### 3. Prune threshold duplicated with different values

**Branch:** feat/ember-core
**Files:** `decay-service.ts:8` (30 days) vs `candidate-service.ts` (90 days)

Share a single constant or derive from config.

### 4. Duplicated `queryProposals` call in ember `list.ts`

**Branch:** feat/ember-cli
**File:** `apps/anvil-cli/src/commands/ember/list.ts`

Same query built twice in JSON vs plain branches. Extract to a shared call
before branching.

### 5. `dismissed` count missing from `anvil status` Ember section

**Branch:** feat/ember-cli
**File:** `apps/anvil-cli/src/commands/status.ts`

Asymmetric to `promoted`. Either show all four statuses or note the omission.

### 6. Ember docs: default limit is 20, not 50

**Branch:** feat/ember-docs
**File:** `docs/guides/ember-candidates.md:97`

### 7. Ember docs: `--min-confidence` and `--include-expired` don't exist

**Branch:** feat/ember-docs
**File:** `docs/guides/ember-candidates.md:89-97`

### 8. Ember docs: references non-existent `anvil kindling show`

**Branch:** feat/ember-docs
**File:** `docs/guides/ember-candidates.md:172-174`

### 9. Double search filtering is redundant

**Branch:** feat/edda-core
**File:** `packages/edda-stack/src/edda/memory-store.ts:112-129`

First pass filters on truncated 100-char index entry, second pass re-checks
against full statement. First pass can produce false negatives for long
statements. Remove it.

### 10. Hardcoded `limit: 100` silently truncates convenience methods

**Branch:** feat/edda-core
**File:** `packages/edda-stack/src/edda/memory-store.ts:174,192,210`

`getActiveMemories`, `getMemoriesByType`, and `searchMemories` all silently cap
at 100 results. Add an optional `limit` parameter or use a larger default.

### 11. Fallback synthesises fake UUIDs for provenance

**Branch:** feat/edda-core
**File:** `packages/edda-stack/src/edda/promotion-service.ts:58-76`

When `emberPort` is absent, random UUIDs are generated for `observation_ids` and
`session_ids`, corrupting the provenance chain. Route offline promotions through
`createMemory` directly, or construct provenance from `input.provenance`.

### 12. Hardcoded `method: 'cli_command'` in attribution

**Branch:** feat/edda-core
**File:** `packages/edda-stack/src/edda/evolution-service.ts:39`

Attribution method is always `'cli_command'` even when called from API or
background agent. Carry the originating method through `CreateMemoryInput` or
service deps.

### 13. `colourStatus` / `colourConfidence` duplicated 5 times with inconsistencies

**Branch:** feat/edda-cli
**Files:** All 5 command files in `apps/anvil-cli/src/commands/edda/`

`active` renders as cyan in `list.ts`/`show.ts` but green in
`promote.ts`/`retire.ts`/`trace.ts`. Extract to shared
`apps/anvil-cli/src/commands/edda/utils.ts`.

### 14. `--by` accepts arbitrary text with no validation

**Branch:** feat/edda-cli
**Files:** `promote.ts:41`, `retire.ts:29`

No length limit, character set restriction, or empty-string check. Add minimum
validation.

---

## Minor Notes (16 — nice to fix)

| # | Branch | Summary |
|---|--------|---------|
| 1 | ember-core | `groupByKind` uses O(n^2) array spread in loop — use `push()` |
| 2 | ember-core | `getExpiringsSoon` — double-s typo (should be `getExpiringSoon`) |
| 3 | ember-core | `SurpriseRule.UNEXPECTED_KIND_SIGNALS` references unknown kinds (`kind_custom`, `kind_metric_recorded`) |
| 4 | ember-core | `proposal-store.test.ts` casts to `unknown` to access private `db` |
| 5 | ember-cli | `resolve` vs `join` inconsistency for db path construction |
| 6 | ember-cli | `parseStatus` has dead `?? 'active'` fallback (Commander default means never undefined) |
| 7 | ember-cli | Test coverage is structural only — no behaviour tests for command logic |
| 8 | ember-docs | Lifecycle diagram shows `created` state not described in text below it |
| 9 | edda-core | `validateEvolutionGraph` uses `.parse()` (throws) instead of `.safeParse()` — breaks collect-all-issues design |
| 10 | edda-core | `serialisation.ts` has manual `MemoryIndexEntry` type — should use `z.infer<typeof MemoryIndexEntrySchema>` |
| 11 | edda-core | `queryMemories` passes entries with missing `created_at` through time filters |
| 12 | edda-core | `getHistory` throws on malformed git log lines instead of skipping |
| 13 | edda-cli | `list.ts` passes explicit `undefined` for optional query fields — unnecessary noise |
| 14 | edda-cli | `trace.ts` "up supersedes" arrow reads backwards chronologically |
| 15 | edda-integration | README import example uses non-existent `./edda` subpath export |
| 16 | edda-integration | `migrateV0ToV1` existing-status preservation path has no test |

---

## Positive Observations

- **Architecture**: Clean layered design — contracts, store, services, facade.
  Proper dependency inversion with `IMemoryStoreOperations`,
  `IVersionTracker`, `IEmberPort` interfaces.
- **Schema discipline**: Zod schemas as single source of truth throughout.
  `TypedProposalSchema` discriminated union with `satisfies Record<>` catches
  missing types at compile time.
- **Testing**: Genuinely meaningful tests — boundary conditions, round-trip
  serialisation, malformed input, cycle detection. `createInMemory()` factory
  pattern makes test setup ergonomic.
- **Security**: Parameterised SQL IN clauses, WAL mode + foreign keys enabled,
  no secrets in code, proper `CliError`/`CliExit` guard patterns.
- **Conventions**: UK English consistent (`serialise`, `behaviour`), kebab-case
  files, ESM `.js` extensions throughout, co-located test files.
- **Design decisions**: `withObjectDefaults` Zod helper, `calculateExpiry`
  centralised in `temporal.ts`, `ObservationHook` tested with concrete
  `TestEventBus` rather than deep mocks.
- **CLI patterns**: Consistent Commander.js factory functions, proper
  `CliError`/`CliExit` guard in catch blocks, `finally { store?.close() }` for
  resource cleanup.
- **Documentation**: Well-written guides with clear mental models, accurate
  lifecycle diagrams, and thorough FAQ sections (when factually correct).

---

## Recommended Fix Priority

### First pass — Critical fixes

Fix the 10 critical issues: infinite loop guard, `--author` format, status
guard, double JSON output, exit code consistency, and documentation mismatches.

### Second pass — Major improvements

Address the 14 major suggestions: extract shared CLI utilities, fix duplicated
queries, add missing validation, correct documentation defaults.

### Follow-up APS items

- Behaviour-level CLI tests for ember and edda command logic
- Integration tests for cross-service wiring (`AggregatorService` into
  `CandidateService`)
- `./edda` subpath export in package.json
- Status command test coverage for Edda/Ember sections
