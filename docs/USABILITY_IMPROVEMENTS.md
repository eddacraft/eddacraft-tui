# Usability Improvements for Anvil

This document outlines major usability improvement options that could
significantly enhance the Anvil user experience. These proposals are based on a
comprehensive analysis of the codebase, user workflows, and potential pain
points.

## Overview

While Anvil is a well-architected tool with excellent documentation, there are
several high-impact opportunities to improve usability, reduce onboarding
friction, and enhance the day-to-day developer experience.

## Implementation Status

| Option                                   | Status             | Implementation                | Notes                                                                            |
| ---------------------------------------- | ------------------ | ----------------------------- | -------------------------------------------------------------------------------- |
| 1. Intelligent First-Run Experience      | ✅ **Implemented** | IFR Module (8 tasks complete) | Full implementation with TUI dashboard, historical analysis, and smart detection |
| 2. Visual Architecture Dashboard         | 📋 Proposed        | -                             | Ready for implementation                                                         |
| 3. Smart Suppression Assistant           | 📋 Proposed        | -                             | Core services implemented (QuickWinsIdentifier), CLI integration pending         |
| 4. Performance Monitoring & Optimization | 📋 Proposed        | -                             | Ready for implementation                                                         |
| 5. Context-Aware Error Guidance          | 📋 Proposed        | -                             | Ready for implementation                                                         |
| 6. Collaborative Review Mode             | 📋 Proposed        | -                             | Ready for implementation                                                         |
| 7. IDE-First Experience                  | 📋 Proposed        | -                             | Ready for implementation                                                         |
| 8. Progressive Adoption Roadmap          | 📋 Proposed        | -                             | Ready for implementation                                                         |

**Latest Update:** January 2026 - Option 1 (Intelligent First-Run Experience)
fully implemented and integrated into `anvil init` flow.

---

## Option 1: Intelligent First-Run Experience

**Impact:** Reduce onboarding time by 80%

### Current Pain Point

New users face configuration complexity and don't see clear value until they
complete the initial setup and run their first check. The learning curve
includes understanding anti-patterns, architecture concepts, baselines, and
suppressions.

### Proposal

Transform the `anvil init` experience into an intelligent, value-demonstrating
onboarding flow:

#### Features

1. **Immediate Value Demonstration**
   - Auto-run `anvil check` on sample files during initialization
   - Display real-time metrics: "Found 12 potential issues in your codebase"
   - Show before/after comparisons: "This would have caught X issues in your
     last 10 commits"
   - Generate a "quick wins" report highlighting easiest fixes

2. **Smart Defaults Based on Project Context**
   - Detect framework (Next.js, React, Node.js, etc.)
   - Analyze repository size and structure
   - Inspect existing tooling (ESLint config, tsconfig strictness)
   - Pre-populate `.anvilrc` with project-specific settings

3. **Interactive Results Dashboard**

   ```
   ✓ Analyzed 234 files in 2.3s
   ✓ Found 12 anti-patterns (0 in new code, 12 in baseline - won't fail builds)
   ✓ Architecture: Detected 3-layer structure (UI → Services → Data)
   ✓ Quick wins: 8 issues fixable with 1-click suppressions

   [Continue to dashboard] [Review findings] [Customize settings]
   ```

4. **Guided Configuration**
   - Progressive disclosure of options
   - Smart recommendations based on detected patterns
   - Examples showing impact of each option

### Implementation Considerations

- Enhance `cli/src/commands/init.ts` to add post-init analysis
- Create new TUI component for results dashboard
- Add project detection service to analyze codebase characteristics
- Implement smart defaults generator based on project type
- Add telemetry hooks to measure onboarding success

### Success Metrics

- Time to first successful check
- Percentage of users who complete initialization
- Configuration accuracy (how many users change defaults)
- User satisfaction scores

### Implementation (✅ Complete - January 2026)

**Module:** IFR (Intelligent First-Run) -
`plans/modules/intelligent-first-run.aps.md`

**Delivered Features:**

1. **Project Context Detection** (IFR-001)
   - Automatic framework detection (10+ frameworks supported)
   - Monorepo type identification (Nx, Lerna, Turborepo, pnpm/yarn/npm
     workspaces)
   - TypeScript strictness analysis
   - Project size categorization (small/medium/large/xlarge)
   - File: `cli/src/services/project-detector.ts`

2. **Smart Defaults Generator** (IFR-002)
   - Adaptive configuration based on project characteristics
   - Size-based threshold adjustments (±5-10%)
   - Framework-specific allowlists
   - File: `cli/src/services/smart-defaults.ts`

3. **Sample File Analyzer** (IFR-003)
   - Git-based recent file selection (last 30 days)
   - Filesystem fallback for non-git projects
   - Diversity-focused distribution (max 50 files)
   - File: `cli/src/services/sample-analyzer.ts`

4. **Quick Wins Identifier** (IFR-004)
   - 7 quick win types (test files, type definitions, config files, generated
     code, etc.)
   - Batch suppression grouping
   - Confidence scoring (0.7-0.98)
   - File: `cli/src/services/quick-wins.ts`

5. **Historical Analysis** (IFR-006)
   - Git history analysis (last 30 days, max 100 commits)
   - Violation estimation from diffs (5 anti-pattern types)
   - Timeline visualization data
   - Pattern occurrence tracking
   - File: `cli/src/services/historical-analyzer.ts`

6. **Interactive Results Dashboard** (IFR-005)
   - Comprehensive TUI dashboard with all analysis results
   - Project metrics overview
   - Quick wins panel with batch suggestions
   - Historical insights showing preventive value
   - Next steps navigation
   - Files: `cli/src/tui/components/ResultsDashboard.tsx`, `QuickWinsPanel.tsx`,
     `InitResults.tsx`

7. **Init Flow Integration** (IFR-007)
   - Seamless integration into `anvil init` command
   - `--no-analysis` flag to skip automatic analysis
   - Progress indicators and error handling
   - Graceful fallback to text summary when TUI unavailable
   - File: `cli/src/commands/init.ts` (enhanced)

8. **Documentation** (IFR-008)
   - Updated documentation to reflect new experience
   - Implementation status tracking
   - User guides and examples

**Key Components:**

- **Services:** 5 new analysis and detection services with comprehensive test
  coverage (93-100% pass rates)
- **TUI Components:** 3 new React/Ink components for results display
- **Integration:** Enhanced init command with intelligent orchestration
- **Testing:** 106+ new tests across all services

**Usage:**

```bash
# Standard init with intelligent analysis
anvil init

# Skip automatic analysis
anvil init --no-analysis

# Force init with analysis
anvil init --force
```

**Technical Details:**

- All services use graceful fallbacks for missing dependencies (git, etc.)
- Test files, build directories, and generated code automatically excluded
- Historical analysis uses regex-based pattern detection on git diffs
- TUI dashboard supports keyboard navigation (Enter to continue, q to quit)
- Maintains full backward compatibility with existing init flow

---

## Option 2: Visual Architecture Dashboard

**Impact:** Make architecture violations 10x clearer

### Current Pain Point

Text-based architecture violations are difficult to understand. Layer detection
struggles with non-standard project structures. Users have trouble visualizing
how their code organization violates architectural boundaries.

### Proposal

Create an interactive TUI-based architecture visualizer that makes structure and
violations immediately clear:

#### Features

1. **Interactive Architecture Visualization**

   ```
      ┌─────────────────┐
      │   Presentation  │
      │   (12 modules)  │
      └────────┬────────┘
               │
               ↓
      ┌─────────────────┐
      │    Business     │  ← 3 violations
      │    (8 modules)  │
      └────────┬────────┘
               │
               ↓ (wrong direction!)
      ┌─────────────────┐
      │      Data       │
      │    (5 modules)  │
      └─────────────────┘

   [i] Press ENTER on violation to see details
   [e] Edit architecture.yaml  [x] Export diagram
   ```

2. **Click-to-Explain Violations**
   - Navigate between layers and modules
   - Highlight problematic dependencies in red
   - Show explanation when selected
   - Display fix recommendations

3. **Teaching Mode**
   - Explain why each edge is problematic
   - Show architectural principles being violated
   - Link to documentation and examples
   - Suggest refactoring patterns

4. **Export Capabilities**
   - Generate Mermaid diagrams for documentation
   - Export to PlantUML
   - Create PNG/SVG for presentations
   - Generate architecture documentation

5. **Real-Time Updates**
   - Watch mode integration
   - Live updates as code changes
   - Highlight recently changed modules
   - Show trend indicators (improving/degrading)

### Implementation Considerations

- New command: `anvil architecture show --interactive`
- Leverage existing architecture detection in `core/src/architecture/`
- Create new TUI components for graph visualization
- Add graph layout algorithms (layered, hierarchical)
- Implement export functionality for multiple formats
- Add detailed violation explanations

### Success Metrics

- Time to understand architecture violations
- Reduction in false positive reports
- Adoption of architecture features
- User comprehension scores

---

## Option 3: Smart Suppression Assistant

**Impact:** Reduce false positive friction by 70%

### Current Pain Point

Users must manually add suppression comments, and it's not always clear when
suppressions are appropriate. This creates friction and slows down development
flow.

### Proposal

Create an interactive suppression workflow that guides users through appropriate
suppression decisions:

#### Features

1. **Interactive Suppression Workflow**

   ```
   anvil check --staged

   Found 5 issues:

   AP-003: Explicit 'any' in src/utils/sdk-wrapper.ts:23
   ├─ Why: Type safety compromise
   ├─ Fix: Use 'unknown' or specific type
   └─ [s] Suppress  [f] Fix now  [i] Ignore this file type

   > Choose [s]

   Generate suppression reason? [Y/n] Y
   Detected: Third-party SDK callback requires any type
   Suggested reason: "SDK callback signature requires any"

   Apply this reason to:
   [x] This occurrence only
   [ ] All SDK-related any types (3 found)
   [ ] All .d.ts files

   [↵ Confirm] [e] Edit reason
   ```

2. **AI-Generated Suppression Reasons**
   - Analyze code context to suggest appropriate reasons
   - Learn from past suppressions in the codebase
   - Provide templates for common scenarios
   - Ensure reasons are meaningful and auditable

3. **Batch Suppression Capabilities**
   - Suppress by category ("All test files")
   - Suppress by pattern ("All console.log in scripts/")
   - Suppress by file type ("All .d.ts files")
   - Review and approve batch operations

4. **Suppression Health Monitoring**
   - Track suppression rate over time
   - Alert when suppression rate is too high
   - Show "suppression health score"
   - Identify suppression hotspots in codebase

5. **Suppression Templates**
   - Pre-defined reasons for common scenarios
   - Team-customizable templates
   - Consistent language across team
   - Searchable suppression library

### Implementation Considerations

- Add interactive mode to `anvil check` command
- Create new TUI for suppression workflow
- Implement context analysis for reason generation
- Add batch operations to suppression system
- Create suppression analytics dashboard
- Extend `.anvilrc` with suppression templates

### Success Metrics

- Time to resolve false positives
- Suppression reason quality (manual review)
- User satisfaction with suppression flow
- Reduction in inappropriate suppressions

---

## Option 4: Performance Monitoring & Optimization Dashboard

**Impact:** 5-10x faster for large codebases

### Current Pain Point

Cold starts can be slow on large projects (10k+ files). Users don't have
visibility into what's taking time or how to optimize performance.

### Proposal

Built-in performance profiling and optimization suggestions:

#### Features

1. **Performance Profiler**

   ```
   anvil check --profile

   Running checks on 1,247 files...

   ┌────────────────────────────────────────────┐
   │ Anti-pattern scan    ████████████  2.3s   │
   │ Architecture check   ████░░░░░░░░  1.8s   │
   │ ESLint integration   ██████████░░  2.1s   │
   │ Coverage analysis    ████████████  2.0s   │
   └────────────────────────────────────────────┘

   Total: 8.2s (63% cached, 37% computed)

   💡 Optimization suggestions:
     • Enable architecture caching: Save ~1.2s (-65%)
     • Exclude test fixtures from scan: Save ~0.8s (-35%)
     • Use --changed flag: Would check 47 files instead of 1,247

   [Apply optimizations] [View detailed profile]
   ```

2. **Automatic Optimization Suggestions**
   - Analyze check patterns to recommend improvements
   - Identify unnecessary file scanning
   - Suggest caching opportunities
   - Recommend incremental checking strategies

3. **Cache Management Dashboard**
   - Show cache hit rates
   - Display cache size and staleness
   - Provide cache warmth indicators
   - Allow manual cache management

4. **Parallel Execution Visualization**
   - Show which checks run in parallel
   - Display bottlenecks
   - Suggest parallelization opportunities
   - Monitor CPU/memory usage

5. **Incremental Baseline Updates**
   - Update baselines incrementally, not full recalculation
   - Smart dependency tracking
   - Partial invalidation
   - Background baseline updates

### Implementation Considerations

- Add `--profile` flag to check command
- Implement performance tracing throughout core
- Create optimization analysis engine
- Build performance TUI components
- Add cache analytics to cache module
- Implement parallel execution framework
- Optimize baseline management for incremental updates

### Success Metrics

- Average check time (target: < 2s for changed files)
- Cache hit rate (target: > 80%)
- User-applied optimizations
- Performance improvement after optimization

---

## Option 5: Context-Aware Error Guidance

**Impact:** Reduce "what do I do now?" moments by 90%

### Current Pain Point

While error messages are generally actionable, they could provide more context,
especially for architecture violations. Users sometimes struggle to understand
how to fix issues.

### Proposal

Multi-level, contextual error explanations with actionable fix options:

#### Features

1. **Multi-Level Explanations**

   ```
   anvil explain AP-003-1

   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
   AP-003: Explicit 'any' type
   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

   📍 Location: src/api/client.ts:45

     43 | export class ApiClient {
     44 |   constructor(
   > 45 |     private config: any
     46 |   ) {}
     47 | }

   ❓ Why this matters:
     Using 'any' defeats TypeScript's type safety, making it
     possible to introduce runtime errors that could have been
     caught at compile time.

   🔧 How to fix (ranked by effort):

     1. ⚡ Quick (2 min): Define a config interface
        └─ Similar to: AuthConfig in src/auth/types.ts:12

     2. 🔄 Better (5 min): Use Zod schema for runtime validation
        └─ Pattern used in: src/api/schemas.ts

     3. 🎯 Best (10 min): Generate types from OpenAPI spec
        └─ See: docs/api-client-guide.md

     4. 🚫 Suppress: If truly necessary (requires explanation)

   [1] Show example code  [2] Apply fix automatically
   [s] Add suppression     [↵] Close
   ```

2. **Code Context Display**
   - Show actual code, not just line numbers
   - Highlight relevant sections
   - Display surrounding context
   - Link to related code

3. **Ranked Fix Options**
   - Multiple approaches (quick, better, best)
   - Effort estimates for each option
   - Link to examples in codebase
   - Automatic fix application where possible

4. **Learning from Your Codebase**
   - Find similar patterns already used
   - Suggest consistency with existing code
   - Show examples from your repository
   - Learn from past fixes

5. **"Fix It For Me" Capability**
   - Automated fixes for simple issues
   - Code transformation suggestions
   - Preview before applying
   - Undo capability

### Implementation Considerations

- Enhance `anvil explain` command
- Add code context retrieval
- Implement fix ranking algorithm
- Create codebase pattern matcher
- Add automated fix generators
- Build interactive explanation TUI
- Extend check results with more metadata

### Success Metrics

- Time to resolve violations
- Fix success rate (correct on first try)
- Use of automated fixes
- Reduction in support requests

---

## Option 6: Collaborative Review Mode

**Impact:** Transform Anvil into a team collaboration tool

### Current Pain Point

Suppressions are individual decisions with no team visibility. There's no
workflow for discussing whether a suppression is appropriate.

### Proposal

Team-based suppression review and approval workflow:

#### Features

1. **Suppression Request Workflow**

   ```
   anvil review --pending

   Suppression Requests (3 pending):

   ┌──────────────────────────────────────────────────┐
   │ @alice requested suppression for AP-003          │
   │ Location: src/auth/oauth.ts:89                   │
   │ Reason: "OAuth library callback requires any"    │
   │                                                   │
   │ Context: Third-party OAuth library callback     │
   │ Team budget: 8/10 'any' types used in /auth     │
   │                                                   │
   │ [✓] Approve  [✗] Reject  [?] Request changes    │
   └──────────────────────────────────────────────────┘

   Statistics:
   • Your approval rate: 85% (17/20)
   • Team average: 78%
   • Top suppression author: @bob (12 this month)
   ```

2. **Team Suppression Budgets**
   - Set limits per module/package
   - Track usage against budget
   - Alert when approaching limits
   - Require approval for over-budget suppressions

3. **Suppression Leaderboard**
   - Gamification of code quality
   - Track who suppresses most/least
   - Show approval rates
   - Celebrate improvements

4. **PR Integration**
   - Surface suppression requests in PR comments
   - Require team review for suppressions
   - Show suppression impact in PR
   - Track suppressions across PRs

5. **Team Dashboard**
   - Overall suppression trends
   - Hot spots requiring attention
   - Team compliance metrics
   - Quality score over time

### Implementation Considerations

- New `anvil review` command
- Add suppression request/approval system
- Create team configuration system
- Implement budget tracking
- Build GitHub Action integration for reviews
- Add team analytics dashboard
- Create notification system

### Success Metrics

- Suppression approval rate
- Reduction in inappropriate suppressions
- Team engagement with review process
- Code quality improvement over time

---

## Option 7: IDE-First Experience

**Impact:** Meet developers where they are - in their editor

### Current Pain Point

While a VS Code extension exists, it could provide deeper integration.
Developers spend most of their time in the editor, not the terminal.

### Proposal

Enhanced IDE integration with real-time guidance and quick fixes:

#### Features

1. **Real-Time Suggestions As You Type**

   ```typescript
   // As you type in VS Code:
   const handler = sdk.createCallback(data: any
                                           ^^^
                                            ↓
       [💡 Anvil suggests]
       • Replace with 'unknown' ⚡ (safe)
       • Define CallbackData type 🔧 (recommended)
       • Suppress with reason 🚫
       • Ignore for this file ⏭️
   ```

2. **One-Click Quick Fixes**
   - Apply fixes directly from editor
   - Add suppressions without leaving IDE
   - Bulk fix similar issues
   - Undo/redo support

3. **Inline Architecture Visualization**
   - Hover over import to see architectural context
   - Show whether dependency is allowed
   - Display alternative imports if violation
   - Visualize module boundaries

4. **Violation Risk Score**
   - Real-time score as you code
   - Predict violations before save
   - Suggest safer alternatives
   - Learn from your patterns

5. **Multi-IDE Support**
   - VS Code (enhanced)
   - JetBrains IDEs (WebStorm, IntelliJ)
   - Neovim (via LSP)
   - Sublime Text
   - Consistent experience across editors

### Implementation Considerations

- Enhance VS Code extension
- Create Language Server Protocol (LSP) implementation
- Build IDE-agnostic core
- Implement real-time analysis engine
- Add quick fix providers
- Create architecture hover providers
- Port to additional IDE platforms

### Success Metrics

- IDE extension adoption rate
- Quick fix usage
- Time saved vs CLI workflow
- User satisfaction scores

---

## Option 8: Progressive Adoption Roadmap

**Impact:** Eliminate "too overwhelming" objections

### Current Pain Point

Teams with existing codebases don't know where to start. The full power of Anvil
can feel overwhelming when there are hundreds of existing violations.

### Proposal

Built-in adoption planner that creates a phased rollout roadmap:

#### Features

1. **Automated Adoption Planning**

   ```
   anvil adoption init

   Creating your Anvil adoption roadmap...

   Current state:
   • 234 total violations
   • 12 in recently changed code (last 30 days)
   • 3 hot modules (changed >10 times, have violations)

   Recommended 6-week plan:

   Week 1-2: Foundation
   ├─ Enable watch mode for team leads
   ├─ Target: 3 hot modules (src/api/, src/auth/, src/db/)
   └─ Goal: 0 new violations in hot modules

   Week 3-4: Expansion
   ├─ Roll out pre-commit hooks team-wide
   ├─ Fix/suppress baseline in hot modules
   └─ Goal: 100% compliance in hot modules

   Week 5-6: Enforcement
   ├─ Enable CI blocking for anti-patterns
   ├─ Gradual architecture gate rollout
   └─ Goal: 0 violations in new code

   [Start Phase 1] [Customize plan] [Export for stakeholders]
   ```

2. **Phase-Based Rollout**
   - Week 1-2: Foundation (watch mode, hot modules)
   - Week 3-4: Expansion (pre-commit hooks)
   - Week 5-6: Enforcement (CI blocking)
   - Customizable phases
   - Automatic progression triggers

3. **Gamified Milestones**
   - Team celebration triggers
   - Progress visualization
   - Achievement system
   - Share progress with stakeholders

4. **Management Reporting**
   - Auto-generated compliance reports
   - ROI calculations (time saved, bugs prevented)
   - Trend analysis
   - Executive summaries
   - Export to PDF/PowerPoint

5. **Smart Module Prioritization**
   - Identify "hot" modules (frequently changed)
   - Calculate impact score (violations × change frequency)
   - Prioritize by ROI
   - Track module-level progress

### Implementation Considerations

- New `anvil adoption` command group
- Git history analysis for hot module detection
- Phase planning algorithm
- Progress tracking system
- Milestone celebration system
- Report generation engine
- Export to presentation formats

### Success Metrics

- Team adoption rate
- Time to full rollout
- Compliance improvement over time
- Stakeholder satisfaction

---

## Implementation Priority Recommendations

### Tier 1: Quick Wins (Weeks 1-4)

1. **Option 1: Intelligent First-Run Experience**
   - Highest impact on new user adoption
   - Relatively straightforward implementation
   - Leverages existing check infrastructure

2. **Option 5: Context-Aware Error Guidance**
   - Immediate value for all users
   - Reduces friction at critical moment
   - Enhances existing explain command

### Tier 2: High Impact (Weeks 5-12)

3. **Option 2: Visual Architecture Dashboard**
   - Differentiating feature
   - Makes complex concepts accessible
   - Requires new TUI components

4. **Option 3: Smart Suppression Assistant**
   - Reduces daily friction
   - Improves code quality
   - Builds on existing suppression system

### Tier 3: Strategic (Quarters 2-3)

5. **Option 7: IDE-First Experience**
   - Meets users where they are
   - Significant development effort
   - Requires multi-platform support

6. **Option 8: Progressive Adoption Roadmap**
   - Removes adoption barriers
   - Critical for enterprise sales
   - Needs stakeholder research

### Tier 4: Advanced (Quarter 4+)

7. **Option 4: Performance Monitoring & Optimization**
   - Important for scale
   - Complex implementation
   - Lower priority until user base grows

8. **Option 6: Collaborative Review Mode**
   - Innovative collaboration feature
   - Requires workflow research
   - Best after core features mature

---

## Success Criteria

### User Experience Metrics

- **Onboarding Time:** Reduce from 30 minutes to < 5 minutes
- **Time to First Value:** Show value within 60 seconds
- **Fix Time:** Reduce average violation resolution from 10 minutes to < 2
  minutes
- **User Satisfaction:** Target NPS > 50

### Adoption Metrics

- **Activation Rate:** >80% of users complete onboarding
- **Retention:** >70% weekly active users after 30 days
- **Team Adoption:** >60% of teams enable CI blocking within 6 weeks
- **IDE Extension:** >50% of users install IDE extension

### Quality Metrics

- **False Positive Rate:** < 10% of violations suppressed
- **Fix Success Rate:** >90% of fixes correct on first attempt
- **Suppression Quality:** >80% of suppressions have meaningful reasons
- **Architecture Compliance:** >95% of new code passes architecture checks

---

## Conclusion

These eight usability improvements represent strategic opportunities to
transform Anvil from a powerful but complex tool into an intuitive, essential
part of the modern development workflow.

The recommended approach is to:

1. **Start with Tier 1** (Options 1 and 5) to validate the impact of enhanced UX
2. **Gather user feedback** on which pain points are most critical
3. **Iterate quickly** on the most impactful features
4. **Build progressively** toward the more complex features in Tiers 3-4

Each option has been designed to:

- Solve real user pain points identified in the codebase analysis
- Build on existing infrastructure where possible
- Provide measurable impact
- Enhance Anvil's core value proposition of "save-time trust"

By implementing these improvements, Anvil can become the definitive tool for
making AI-generated code safe for production while providing an exceptional
developer experience.
