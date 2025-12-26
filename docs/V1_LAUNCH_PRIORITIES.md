# V1 Launch Priorities - No AI Required

**Date**: 2025-12-25
**Scope**: Ship-it-now improvements for first public release
**Constraint**: NO AI dependencies - save for V2

---

## Executive Summary

For V1 launch, focus on **fixing the core user experience** without AI features. Get users successfully through their first validation, then add AI magic in V2.

**V1 Goal**: 80%+ success rate getting users from install → first successful validation

**V2 Goal**: AI-powered features that make Anvil indispensable

---

## Critical V1 Improvements (Ship This Week)

### 1. Fix First-Run Experience (4 hours)

**Problem**: User runs `anvil` and sees cryptic help text
**Solution**: Friendly welcome with clear next steps

**Implementation:**

Update `cli/src/index.ts` to detect when run without arguments:

```typescript
// If no command provided, show welcome instead of help
if (!process.argv.slice(2).length) {
  console.log(`
👋 Welcome to Anvil!

Anvil validates planning documents and runs quality gates to ensure
code changes are safe before deployment.

Quick start:
  1. Validate an existing plan
     $ anvil validate plan.md

  2. Set up for this project
     $ anvil init

  3. See all commands
     $ anvil help

Examples:
  anvil validate spec.md              Validate a plan
  anvil gate spec.md                  Run quality gates
  anvil export spec.md --to aps       Convert formats

Documentation: https://anvil.dev/docs
Get help: anvil help <command>
  `);
  process.exit(0);
}
```

**Impact**: Users immediately know what to do next
**Effort**: 4 hours
**AI Required**: No ✓

---

### 2. Add Format Decision Guidance (3 hours)

**Problem**: Users paralyzed by format choice in `anvil init`
**Solution**: Add context and reassurance

**Implementation:**

Update `cli/src/commands/init.ts`:

```typescript
const format = await prompts({
  type: 'select',
  name: 'value',
  message: 'Which planning format do you use?',
  choices: [
    {
      title: 'Generic Markdown (recommended for getting started)',
      description: 'Works with any .md file • Most flexible • Convert later ✓',
      value: 'generic',
    },
    {
      title: 'SpecKit (for GitHub workflows)',
      description: 'spec.md, plan.md, tasks.md • GitHub Issues/PRs',
      value: 'speckit',
    },
    {
      title: 'BMAD (for PRDs & architecture)',
      description: 'prd.md, architecture.md • Product requirements',
      value: 'bmad',
    },
    {
      title: 'APS (advanced)',
      description: 'Native JSON/YAML format • Tool integration',
      value: 'aps',
    },
  ],
  hint: "Don't worry - you can convert between formats anytime with 'anvil export'",
});
```

**Impact**: Users make informed decision, know it's not permanent
**Effort**: 3 hours
**AI Required**: No ✓

---

### 3. Improve Validation Feedback (4 hours)

**Problem**: Validation output doesn't show what value was delivered
**Solution**: Detailed breakdown of what was checked

**Implementation:**

Update `cli/src/commands/validate.ts`:

```typescript
// After successful validation
console.log(`
✓ Plan is valid

🔍 Validation Results:
  ✓ Structure: Valid ${formatName} with ${sectionCount} sections
  ✓ Intent: Clear purpose statement found
  ✓ Changes: ${changeCount} proposed changes identified
  ✓ Integrity: Hash verified (SHA-256: ${hash.slice(0, 8)}...)
  ✓ Completeness: ${completenessScore}/100

${suggestions.length > 0 ? `
💡 Suggestions to improve this plan:
${suggestions.map(s => `  ${s.icon} ${s.message}`).join('\n')}
` : ''}

📊 Quality Score: ${qualityScore}/100 (${rating})

Without Anvil: ⚠️
  • Unknown if plan is complete
  • No verification of structure
  • Can't detect tampering
  • Manual review required

With Anvil: ✓
  • Structure validated
  • Completeness checked (${completenessScore}%)
  • Hash integrity verified
  • ${issueCount} issues caught before coding

${issueCount > 0 ? `⏱️  Time saved: ~${estimatedTimeSaved} minutes of back-and-forth\n` : ''}

Next steps:
  ${nextSteps.map(s => `${s.icon} ${s.message}`).join('\n  ')}
`);
```

**Impact**: Users understand what value they received
**Effort**: 4 hours
**AI Required**: No ✓

---

### 4. Create FAQ Document (2 hours)

**Problem**: Common questions not answered upfront
**Solution**: Comprehensive FAQ at `docs/FAQ.md`

**Content**: (Already drafted in review)
- What is Anvil?
- Which format should I use?
- What's the difference between validate and gate?
- How is this different from just committing markdown?
- Common troubleshooting

**Impact**: Reduces support burden, answers questions before they're asked
**Effort**: 2 hours
**AI Required**: No ✓

---

### 5. Add Progress Indicators (3 hours)

**Problem**: Silent operations make users think it's frozen
**Solution**: Visual progress for long-running commands

**Implementation:**

Use `ora` spinner library:

```typescript
import ora from 'ora';

// In gate command
const spinner = ora('Loading plan...').start();
await sleep(200);
spinner.succeed('Plan loaded (SpecKit format)');

spinner.start('Running quality gates...');

const lintSpinner = ora('  Lint check...').start();
await runLint();
lintSpinner.succeed('  Lint: PASS (100/100)');

const testSpinner = ora('  Test check...').start();
await runTests();
testSpinner.succeed('  Test: PASS (100/100)');

// ... etc
```

**Impact**: Users know something is happening, builds confidence
**Effort**: 3 hours
**AI Required**: No ✓

---

### 6. Better Error Messages (3 hours)

**Problem**: Cryptic errors like "Validation failed"
**Solution**: Helpful error messages with fix suggestions

**Implementation:**

```typescript
// Error handler wrapper
class AnvilError extends Error {
  constructor(
    message: string,
    public code: string,
    public suggestions: string[],
    public learnMoreUrl?: string
  ) {
    super(message);
  }

  format(): string {
    return `
✗ ${this.message}

Quick fix options:
${this.suggestions.map((s, i) => `  ${i + 1}. ${s}`).join('\n')}

${this.learnMoreUrl ? `Learn more:\n  ${this.learnMoreUrl}\n` : ''}
`;
  }
}

// Usage
throw new AnvilError(
  'Missing required field: "intent"',
  'MISSING_INTENT',
  [
    'SpecKit format - add at top:\n     # Spec: Add JWT Authentication',
    'BMAD format - add:\n     ## Problem Statement\n     Users need secure authentication...',
    'Generic - add:\n     ## Overview\n     This plan adds JWT authentication...',
  ],
  'https://anvil.dev/docs/errors/missing-intent'
);
```

**Impact**: Users can self-serve fixes instead of asking for help
**Effort**: 3 hours
**AI Required**: No ✓

---

### 7. Create Format Guide (2 hours)

**Problem**: Users don't know which format to use
**Solution**: New doc `docs/formats.md` with decision tree

**Content**: (Already drafted in review)
- Quick decision flowchart
- Format comparison table
- "You can convert later" reassurance
- Examples of each format

**Impact**: Removes format confusion, builds confidence
**Effort**: 2 hours
**AI Required**: No ✓

---

## Total V1 Critical Improvements

**Total Effort**: ~21 hours (2.5 days)
**Impact**: Fixes top 7 user pain points
**AI Required**: None ✓

---

## High-Value V1 Features (Ship This Month)

### 8. Static Template Library (1 week)

**Problem**: Users start from scratch every time
**Solution**: Curated templates without AI generation

**Implementation:**

```bash
anvil new --template jwt-auth

# Shows list of static templates
? Select a template:
  ❯ JWT Authentication
    REST API Endpoint
    React Component
    Database Migration
    GraphQL Schema
    WebSocket Connection
    [Browse all templates...]

# Copies pre-written template file
✓ Created: spec-jwt-auth.md (SpecKit format)
```

**Templates stored as files:**
```
cli/templates/
  ├── jwt-authentication.speckit.md
  ├── rest-api-endpoint.speckit.md
  ├── react-component.speckit.md
  ├── database-migration.bmad.md
  └── ...
```

**Just copy + variable substitution** (no AI needed):
```typescript
// Simple template engine
let content = fs.readFileSync(`templates/${template}.md`, 'utf-8');
content = content.replace(/\{\{projectName\}\}/g, projectName);
content = content.replace(/\{\{author\}\}/g, gitUser);
fs.writeFileSync(outputPath, content);
```

**Impact**: 90% time save (30 min → 3 min), no AI needed
**Effort**: 1 week (create 10-15 high-quality templates)
**AI Required**: No ✓

---

### 9. Web Playground (2-3 days)

**Problem**: Can't try Anvil without installing
**Solution**: Simple web UI for validation

**Implementation:**

```typescript
// Simple Next.js app
// pages/playground.tsx

import { useState } from 'react';

export default function Playground() {
  const [markdown, setMarkdown] = useState(EXAMPLE_PLAN);
  const [result, setResult] = useState(null);

  const validate = async () => {
    const res = await fetch('/api/validate', {
      method: 'POST',
      body: JSON.stringify({ markdown }),
    });
    setResult(await res.json());
  };

  return (
    <div className="playground">
      <div className="editor">
        <textarea value={markdown} onChange={e => setMarkdown(e.target.value)} />
      </div>
      <div className="results">
        {result && <ValidationResults result={result} />}
      </div>
    </div>
  );
}

// pages/api/validate.ts
export default async function handler(req, res) {
  const { markdown } = JSON.parse(req.body);

  // Run validation (import from @anvil/core)
  const validator = new APSValidator();
  const result = await validator.validate(parseMarkdown(markdown));

  res.json(result);
}
```

**No AI needed** - just validation logic
**Impact**: Try before install, great for demos
**Effort**: 2-3 days
**AI Required**: No ✓

---

### 10. Interactive Tutorial (3-5 days)

**Problem**: Users don't know where to start
**Solution**: Step-by-step walkthrough

**Implementation:**

```bash
anvil tutorial

# Step 1/5: Understanding Anvil
───────────────────────────────────────
Anvil validates planning documents to
ensure they're complete before you code.

Let's try it! I've created a sample plan:
  tutorial-plan.md

Validate it:
  $ anvil validate tutorial-plan.md

[Press Enter when ready]

# Step 2/5: Understanding Results
───────────────────────────────────────
Great! You just validated your first plan.

Notice the output shows:
  ✓ Structure validated
  ✓ Intent found
  ⚠ Missing: acceptance criteria

Let's fix that missing item.

Edit tutorial-plan.md and add:
  ## Acceptance Criteria
  - [ ] Feature works as expected

[Press Enter when ready]

# ... continues through 5 steps
```

**Just a CLI script** - no AI needed
**Impact**: Learning by doing, high completion rate
**Effort**: 3-5 days
**AI Required**: No ✓

---

### 11. Enhanced GitHub Action (2-3 days)

**Problem**: Current action just shows pass/fail
**Solution**: Rich PR comments (without AI)

**Implementation:**

```typescript
// .github/actions/anvil-check/action.yml

// Post comment to PR:
const comment = `
## 🔨 Anvil Validation

### ✅ Plan Validation: PASSED
- Format: SpecKit (98% confidence)
- Quality score: ${qualityScore}/100
- Hash: \`${hash.slice(0, 8)}...\`

### 📊 Plan Analysis

**Completeness**: ${completenessScore}%
  ${checkmarks}

**File Changes**: ${fileCount} files
  ${fileList}

### ⚡ Quality Gates: ${passedCount}/${totalCount} PASSED

| Check | Status | Score |
|-------|--------|-------|
${gateResults.map(r => `| ${r.name} | ${r.status} | ${r.score}/100 |`).join('\n')}

${failedCount > 0 ? '⚠️ Some checks failed. Review above.' : '✅ All checks passed!'}

---
[View detailed report](${artifactUrl}) | Run locally: \`anvil validate ${planFile}\`
`;

await github.rest.issues.createComment({
  issue_number: context.issue.number,
  body: comment,
});
```

**No AI** - just template formatting
**Impact**: Better PR visibility, team alignment
**Effort**: 2-3 days
**AI Required**: No ✓

---

## Total V1 High-Value Features

**Total Effort**: ~2-3 weeks
**Impact**: Ecosystem features that drive adoption
**AI Required**: None ✓

---

## What's Saved for V2 (AI Features)

### Deferred to V2 - Requires AI

❌ **AI Plan Generation** - `anvil write "description"`
- Requires Claude API integration
- Prompt engineering for format compliance
- Repository context analysis

❌ **Smart PR Integration** - AI-powered insights
- Requires Claude API for analysis
- ML for effort estimation
- Historical pattern matching

❌ **Interactive Plan Improvement** - AI suggestions
- Requires Claude API for suggestions
- Context-aware recommendations

❌ **AI-Powered Templates** - Dynamic generation
- Current: Static template files ✓
- V2: AI customizes based on context

**Why defer?**
- V1 needs to prove core value first
- AI features are polish, not foundation
- Can ship V1 faster without AI integration

---

## V1 Success Metrics

**Installation to first successful validation:**
- Current: ~30% success rate, 15-30 min time-to-value
- V1 Target: ~70% success rate, 3-5 min time-to-value

**Key metrics:**
- % of users who complete `anvil init` successfully
- % of users who run their first validation
- % of users who run quality gates
- Time from install to first validation
- % of users who validate a second plan (retention)

**What success looks like:**
- User installs Anvil
- Runs `anvil` → sees friendly welcome
- Runs `anvil init` → chooses format confidently
- Runs `anvil validate plan.md` → sees clear value
- Runs `anvil gate plan.md` → quality checks pass
- **Continues using Anvil** ✓

---

## Implementation Order

### Week 1: Critical UX Fixes (21 hours)
1. ✅ First-run experience (4h)
2. ✅ Format decision guidance (3h)
3. ✅ Better validation feedback (4h)
4. ✅ FAQ document (2h)
5. ✅ Progress indicators (3h)
6. ✅ Better error messages (3h)
7. ✅ Format guide (2h)

**Ship:** v0.1.0-beta with improved core experience

### Week 2-3: High-Value Features (2-3 weeks)
8. ✅ Static template library (1 week)
9. ✅ Web playground (2-3 days)
10. ✅ Interactive tutorial (3-5 days)
11. ✅ Enhanced GitHub Action (2-3 days)

**Ship:** v0.2.0 with ecosystem features

### Week 4: Polish & Launch Prep
- Documentation review
- Video demos
- Launch announcement
- Community outreach

**Ship:** v1.0.0 public launch

---

## Key Differences from Full Review

**Changed priorities:**

❌ **Removed from V1:**
- AI plan generation (was #1 killer feature)
- Smart PR integration with AI
- Interactive improvement with AI

✅ **Promoted for V1:**
- Static template library (achieves 90% of AI benefit)
- Web playground (try before install)
- Interactive tutorial (learning by doing)
- Enhanced GitHub Action (visibility without AI)

**Philosophy:**
- V1: Get core experience right, prove value
- V2: Add AI magic once foundation is solid

---

## Launch Messaging (V1 vs V2)

### V1 Launch Message
```
Anvil v1.0 - Validate Planning Documents

Stop merging incomplete plans. Anvil validates structure,
runs quality gates, and ensures code changes are safe
before deployment.

Features:
✓ Multi-format support (SpecKit, BMAD, Generic MD, APS)
✓ Quality gates (lint, test, coverage, secrets)
✓ Template library (15 curated templates)
✓ GitHub Action integration
✓ Interactive tutorial
✓ Web playground - try without installing

Get started: npm install -g @anvil/cli
```

### V2 Launch Message (Future)
```
Anvil v2.0 - AI-Powered Planning

Anvil now writes your plans FOR you.

New:
🤖 AI plan generation - 'anvil write "add auth"' → complete plan
🧠 Smart PR insights - blast radius, risk analysis, suggestions
💡 Interactive improvement - AI helps fix incomplete plans

Upgrade: npm install -g @anvil/cli@latest
```

---

## Conclusion

**V1 Strategy**: Ship fast, prove value, build foundation
- Focus on core UX fixes (21 hours)
- Add ecosystem features (2-3 weeks)
- NO AI dependencies
- Launch in ~1 month

**V2 Strategy**: Add AI magic
- AI plan generation
- Smart PR insights
- Interactive improvement
- Launch 2-3 months after V1

**This approach:**
- ✅ Faster to market
- ✅ Proves core value without AI hype
- ✅ Builds user base before adding AI
- ✅ Allows AI features to be premium/paid tier

**Bottom line**: V1 gets 70% of the value with 30% of the complexity. Ship it. ✓

---

**Next Steps:**
1. Review this priority list
2. Confirm V1 scope
3. Create implementation plan
4. Start with Week 1 critical fixes
5. Ship v0.1.0-beta in 1 week
