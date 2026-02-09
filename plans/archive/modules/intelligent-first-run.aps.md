# Intelligent First-Run Experience

| ID    | Owner    | Status |
| ----- | -------- | ------ |
| IFR   | @aneki   | Ready  |

## Purpose

Transform the `anvil init` experience from configuration-focused to value-demonstrating, reducing onboarding time by 80% through immediate analysis, smart defaults, and contextual insights.

## In Scope

- Post-init automatic analysis on sample of user's codebase
- Project context detection (framework, size, existing tooling)
- Smart defaults generation based on detected project characteristics
- Interactive results dashboard showing immediate value
- Quick wins identification and one-click suppressions
- Git history analysis to show "would have caught X issues"

## Out of Scope

- Changes to core architecture analysis engine (belongs in architecture-safety)
- TUI component library changes (belongs in tui module)
- Architecture baseline generation (already exists)
- Advanced drift analysis (belongs in drift-reporting)

## Interfaces

**Depends on:**

- architecture-safety — architecture detection and analysis
- antipattern-library — anti-pattern scanning
- tui — display components for results dashboard

**Exposes:**

- Enhanced `anvil init` with automatic value demonstration
- Project detection service for framework/tooling analysis
- Smart defaults generator for `.anvilrc` configuration
- Quick wins analyzer for easy fixes

## Value Proposition

**Current pain:** New users face configuration complexity and don't see value until they manually run their first check. Onboarding involves understanding anti-patterns, architecture concepts, baselines, and suppressions before seeing any benefit.

**Proposed solution:** After basic configuration, automatically run analysis on a representative sample of the codebase, show real results, generate smart defaults, and provide actionable insights within 60 seconds of starting `anvil init`.

**Impact:** Reduce time-to-value from 30 minutes to < 2 minutes, increase activation rate from ~50% to > 80%.

## Tasks

### IFR-001: Add project context detection service

**Intent:** Automatically detect project characteristics to inform smart defaults.

**Expected outcome:** Project detector identifies framework (Next.js, React, Node.js, etc.), monorepo structure, TypeScript config strictness, existing linters, and repository size.

**Validation:** `pnpm test -- project-detector`

**Files:**
- `cli/src/services/project-detector.ts` (new)
- `cli/src/services/project-detector.test.ts` (new)

**Steps:**

#### 1. Create project detector service

- **Checkpoint:** Service detects framework, structure, tooling, and size
- **Validate:** `pnpm test -- project-detector.test`

#### 2. Add framework detection

- **Checkpoint:** Identifies Next.js, React, Vue, Node.js, NX from package.json
- **Validate:** `pnpm test -- project-detector.test`

#### 3. Add monorepo structure detection

- **Checkpoint:** Detects lerna, nx, pnpm workspace, yarn workspace configurations
- **Validate:** `pnpm test -- project-detector.test`

#### 4. Add tooling analysis

- **Checkpoint:** Identifies ESLint, Prettier, tsconfig strictness levels
- **Validate:** `pnpm test -- project-detector.test`

### IFR-002: Create smart defaults generator

**Intent:** Generate optimal `.anvilrc` configuration based on detected project characteristics.

**Expected outcome:** Smart defaults generator produces appropriate thresholds, patterns, and allowlists based on project type, size, and existing tooling.

**Validation:** `pnpm test -- smart-defaults`

**Files:**
- `cli/src/services/smart-defaults.ts` (new)
- `cli/src/services/smart-defaults.test.ts` (new)

**Steps:**

#### 1. Create defaults generator service

- **Checkpoint:** Generator creates config based on project characteristics
- **Validate:** `pnpm test -- smart-defaults.test`

#### 2. Add framework-specific defaults

- **Checkpoint:** Appropriate defaults for Next.js, React, Node.js projects
- **Validate:** `pnpm test -- smart-defaults.test`

#### 3. Add monorepo-aware defaults

- **Checkpoint:** Correct patterns for monorepo structures
- **Validate:** `pnpm test -- smart-defaults.test`

#### 4. Add allowlist intelligence

- **Checkpoint:** Smart allowlists for test files, d.ts files, common patterns
- **Validate:** `pnpm test -- smart-defaults.test`

### IFR-003: Add post-init automatic analysis

**Intent:** Run analysis on representative sample immediately after configuration.

**Expected outcome:** After `anvil init` completes configuration, automatically analyzes changed files from last 30 days or up to 50 files, showing results within 5 seconds.

**Validation:** Run `anvil init --force` and observe automatic analysis

**Files:**
- `cli/src/commands/init.ts` (modify)
- `cli/src/services/sample-analyzer.ts` (new)

**Steps:**

#### 1. Create sample analyzer service

- **Checkpoint:** Service selects representative files for analysis
- **Validate:** `pnpm test -- sample-analyzer.test`

#### 2. Integrate with init command

- **Checkpoint:** Init runs analysis after configuration completes
- **Validate:** `anvil init --force` runs automatic analysis

#### 3. Add git history analysis

- **Checkpoint:** Analyzes recently changed files from git history
- **Validate:** Analysis focuses on files changed in last 30 days

### IFR-004: Create quick wins identifier

**Intent:** Identify issues that can be easily fixed or suppressed to demonstrate immediate value.

**Expected outcome:** Quick wins analyzer highlights suppressable anti-patterns in test files, type definitions, and provides one-click suppression options.

**Validation:** `pnpm test -- quick-wins`

**Files:**
- `cli/src/services/quick-wins.ts` (new)
- `cli/src/services/quick-wins.test.ts` (new)

**Steps:**

#### 1. Create quick wins analyzer

- **Checkpoint:** Identifies suppressable violations with suggested reasons
- **Validate:** `pnpm test -- quick-wins.test`

#### 2. Add suppression templates

- **Checkpoint:** Pre-generated suppression reasons for common scenarios
- **Validate:** `pnpm test -- quick-wins.test`

#### 3. Add batch suppression support

- **Checkpoint:** Group similar violations for batch suppression
- **Validate:** `pnpm test -- quick-wins.test`

### IFR-005: Create interactive results dashboard TUI

**Intent:** Display analysis results in an engaging, informative way that demonstrates value.

**Expected outcome:** After analysis completes, show interactive dashboard with metrics, architecture summary, quick wins, and navigation options.

**Validation:** Run `anvil init --force` and verify dashboard display

**Files:**
- `cli/src/tui/commands/InitResults.tsx` (new)
- `cli/src/tui/components/ResultsDashboard.tsx` (new)
- `cli/src/tui/components/QuickWinsPanel.tsx` (new)

**Steps:**

#### 1. Create results dashboard component

- **Checkpoint:** TUI component displays analysis results with metrics
- **Validate:** Visual review of dashboard layout

#### 2. Add quick wins panel

- **Checkpoint:** Panel shows actionable quick wins with one-click options
- **Validate:** Visual review and interaction test

#### 3. Add git history insights

- **Checkpoint:** Shows "would have caught X issues in last N commits"
- **Validate:** Display matches git history analysis results

#### 4. Add navigation options

- **Checkpoint:** Users can review findings, customize settings, or continue
- **Validate:** Navigation works correctly from dashboard

### IFR-006: Add historical analysis feature

**Intent:** Show users how Anvil would have helped with recent code changes.

**Expected outcome:** Analyze recent commits to show which violations Anvil would have caught, demonstrating preventive value.

**Validation:** Dashboard shows accurate historical violation count

**Files:**
- `cli/src/services/historical-analyzer.ts` (new)
- `cli/src/services/historical-analyzer.test.ts` (new)

**Steps:**

#### 1. Create historical analyzer

- **Checkpoint:** Analyzes commits from last 30 days for violations
- **Validate:** `pnpm test -- historical-analyzer.test`

#### 2. Add diff-based analysis

- **Checkpoint:** Identifies violations introduced in each commit
- **Validate:** `pnpm test -- historical-analyzer.test`

#### 3. Add timeline visualization

- **Checkpoint:** Shows violation trends over recent history
- **Validate:** Visual display shows meaningful trends

### IFR-007: Integrate all components in init flow

**Intent:** Create seamless flow from configuration through analysis to results.

**Expected outcome:** Complete `anvil init` flow that detects project, generates smart defaults, runs analysis, and displays engaging results dashboard.

**Validation:** `anvil init --force` completes successfully with new experience

**Files:**
- `cli/src/commands/init.ts` (modify)

**Steps:**

#### 1. Update init command flow

- **Checkpoint:** Init orchestrates detection, analysis, and results display
- **Validate:** `anvil init --force` executes complete flow

#### 2. Add progress indicators

- **Checkpoint:** Users see what's happening during analysis
- **Validate:** Progress indicators display correctly

#### 3. Add error handling

- **Checkpoint:** Graceful degradation if analysis fails
- **Validate:** Init completes even if analysis encounters errors

#### 4. Add skip option

- **Checkpoint:** Users can skip automatic analysis with flag
- **Validate:** `anvil init --no-analysis` skips automatic analysis

### IFR-008: Update documentation

**Intent:** Document new intelligent first-run experience.

**Expected outcome:** QUICK_START.md and USER_GUIDE.md reflect new init experience.

**Validation:** Documentation review

**Files:**
- `docs/QUICK_START.md` (modify)
- `docs/USER_GUIDE.md` (modify)
- `docs/USABILITY_IMPROVEMENTS.md` (update implementation status)

**Steps:**

#### 1. Update quick start guide

- **Checkpoint:** Quick start reflects new init experience
- **Validate:** Manual review of documentation

#### 2. Update user guide

- **Checkpoint:** User guide documents all new features and flags
- **Validate:** Manual review of documentation

#### 3. Add examples and screenshots

- **Checkpoint:** Examples show expected output from new experience
- **Validate:** Screenshots match actual output

## Success Metrics

- Time to first value: < 2 minutes (currently ~30 minutes)
- Activation rate: > 80% complete init (currently ~50%)
- Smart defaults accuracy: < 20% of users modify generated config
- User satisfaction: NPS > 40 for init experience

## Dependencies

- Requires TUI foundation from tui module
- Requires core analysis engine from architecture-safety and antipattern-library
- No blocking dependencies — can start immediately

## Notes

- Keep automatic analysis fast (< 5s) by limiting sample size
- Ensure graceful fallback if project detection fails
- Consider telemetry to measure success metrics (opt-in)
- Progressive disclosure: don't overwhelm with too much info at once
