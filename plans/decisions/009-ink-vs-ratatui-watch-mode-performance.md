# Watch Mode Performance Analysis: Ink vs Ratatui

> **Superseded:** This analysis correctly identified check execution as the
> bottleneck (99% of latency) but concluded "performance is not a
> differentiator" by comparing only TUI render times. The actual question —
> whether faster check execution lets us run *more checks* in the same time
> window — was never evaluated. With the Rust kernel
> ([Architecture Evolution](../../docs/architecture/anvil-architecture-evolution.md)),
> the entire watch cycle drops from ~3s to ~200ms, making the Ratatui TUI a
> natural fit. See [RATS — Ratatui TUI](../modules/ratatui-tui.aps.md) and
> [ADR-011](./011-rust-core-engine.md) (itself superseded).

## Question

Would Ratatui improve watch mode responsiveness when typing/saving files?

## TL;DR

**No.** _(Superseded — see note above.)_ TUI rendering is <0.5% of total latency. The bottleneck is running checks (lint, test, coverage), not rendering the results.

## Watch Mode Latency Breakdown

### Complete Flow (Type → Save → See Results)

```
User saves file in editor
  ↓
[1] Chokidar detects change        ~50-100ms   (OS file watching)
  ↓
[2] Debouncer waits                ~300ms      (configurable, intentional delay)
  ↓
[3] Git status filtering           ~50-200ms   (git status parsing)
  ↓
[4] Action execution               ~1000-10000ms  ← THIS IS THE BOTTLENECK
    ├─ validate: Parse/validate    ~100-500ms
    ├─ gate: Run all checks        ~2000-10000ms
    │   ├─ ESLint                  ~500-2000ms
    │   ├─ Vitest                  ~1000-5000ms
    │   ├─ Coverage                ~2000-10000ms
    │   ├─ Secret scan             ~200-800ms
    │   └─ Dependency audit        ~500-2000ms
    └─ check: Architecture         ~500-2000ms
  ↓
[5] TUI rendering                  ~5-10ms (Ink) vs ~1-3ms (Ratatui)
  ↓
User sees results
```

### Performance Comparison

| Mode | Total Latency | TUI Render Time | TUI % of Total |
|------|---------------|-----------------|----------------|
| **Validate** | ~500-1000ms | 5-10ms | **1%** |
| **Gate (dev profile)** | ~3000-7000ms | 5-10ms | **0.1-0.3%** |
| **Gate (full)** | ~5000-15000ms | 5-10ms | **0.05-0.2%** |
| **Check (source)** | ~1000-3000ms | 5-10ms | **0.3-1%** |

**Ratatui's 5ms advantage** is imperceptible in the context of multi-second check execution.

## Code Analysis

### Current Watch Implementation

From `cli/src/commands/watch.ts` and `core/src/watch/orchestrator.ts`:

```typescript
// Watch flow
1. FileWatcher (chokidar) detects change → emits event
2. Debouncer batches changes (300ms window)
3. Git filtering (if enabled)
4. Action handler executes:
   - await planLoader.loadPlan()      // Synchronous, blocks
   - await gateRunner.runGate()       // Synchronous, blocks
   - await gateRunner.analyzeFiles()  // Synchronous, blocks
5. Event emitted: { type: 'action:complete', result }
6. TUI updates via output.handleEvent(event)
```

**Key insight:** Steps 1-4 are identical for Ink vs Ratatui (both TypeScript). Only step 6 differs.

### Where Time is Actually Spent

#### Gate Mode (worst case):
```typescript
// From watch.ts:190-196
const results: GateRunResultWithCache = await gateRunner.runGate(
  loadResult.plan,
  gateConfig,
  workspaceRoot,
  gateOptions
);
// ↑ This line takes 2-10 seconds
// ↓ This line takes 5-10ms
output.handleEvent({ type: 'action:complete', result });
```

**The `runGate()` call dominates latency.** TUI rendering is a rounding error.

#### Breakdown of runGate():
```typescript
// From core (hypothetical):
async runGate() {
  await runESLint();      // 500-2000ms   ← 20% of time
  await runVitest();      // 1000-5000ms  ← 40% of time
  await runCoverage();    // 2000-10000ms ← 50% of time
  await scanSecrets();    // 200-800ms    ← 5% of time
  await auditDeps();      // 500-2000ms   ← 10% of time

  return aggregateResults();  // <1ms
}
// TUI.render(results);  // 5-10ms (Ink) vs 1-3ms (Ratatui)
```

**TUI rendering happens AFTER checks complete**, so it can't make checks feel faster.

## What About "Responsiveness During Execution"?

You might be thinking: "Do progressive updates feel smoother with Ratatui?"

### Current Event-Driven Updates

```typescript
// Orchestrator emits events during execution:
this.emitEvent({ type: 'action:start', action: 'gate', files });
//   ↓ TUI renders: "⣾ Running gate checks..."

await handler(files);  // Blocks for 5 seconds

this.emitEvent({ type: 'action:complete', result });
//   ↓ TUI renders: "✓ Gate passed (5.2s)"
```

**Problem:** The gate runner doesn't emit progress events during check execution.

**Example of what users see:**
```
⣾ Running gate checks...
[5 second pause - no updates]
✓ Passed (5.2s)
```

**What users want to see:**
```
⣾ Running ESLint... (1/5)
✓ ESLint passed (1.2s)
⣾ Running Vitest... (2/5)
✓ Vitest passed (3.4s)
⣾ Running coverage... (3/5)
```

### Would Ratatui Help Here?

**No.** The issue is **lack of progress events**, not slow rendering. Both Ink and Ratatui render events instantly (<10ms). The problem is:

```typescript
// Current (no progress events):
const result = await runGate();  // Black box - no updates for 5s

// What we need (progressive events):
const result = await runGate({
  onCheckStart: (check) => emit({ type: 'check:start', check }),
  onCheckComplete: (check, result) => emit({ type: 'check:complete', check, result })
});
```

**Solution:** Refactor `GateRunner` to emit progress events, NOT switch TUI libraries.

## Performance Comparison: Ink vs Ratatui for Event Updates

### Scenario: Gate emits 10 progress events during 5-second execution

```typescript
// Theoretical best case (gate emits events every 500ms)
0.0s:  emit('check:start', 'lint')       → render: 5ms (Ink) vs 1ms (Ratatui)
0.5s:  emit('check:complete', 'lint')    → render: 5ms vs 1ms
1.0s:  emit('check:start', 'test')       → render: 5ms vs 1ms
1.5s:  emit('check:complete', 'test')    → render: 5ms vs 1ms
...
5.0s:  emit('action:complete')           → render: 5ms vs 1ms

Total TUI time: 10 events × 5ms = 50ms (Ink) vs 10ms (Ratatui)
Still only 1% of total execution time.
```

**Verdict:** Even with 10x more updates, rendering is still <1% of total time.

## Real Bottleneck: Check Execution, Not Rendering

To actually improve watch mode "snappiness," focus on:

### 1. Faster Check Execution
```typescript
// Current: Sequential (slow)
await runESLint();    // 2s
await runVitest();    // 3s
await runCoverage();  // 8s
// Total: 13s

// Optimized: Parallel (fast)
await Promise.all([
  runESLint(),     // 2s
  runVitest(),     // 3s
  runCoverage(),   // 8s
]);
// Total: 8s (40% faster!)
```

**Savings: 5 seconds** vs Ratatui's **5 milliseconds**.

### 2. Smarter Debouncing
```typescript
// Current: Fixed 300ms delay
debounceMs: 300  // Always waits 300ms even for single file change

// Optimized: Adaptive debouncing
debounceMs: fileCount === 1 ? 50 : 300  // 250ms saved for single files
```

**Savings: 250ms** vs Ratatui's **5ms**.

### 3. Incremental Checks
```typescript
// Current: Run all checks on every file save
await runGate(plan, config);  // Runs lint+test+coverage every time

// Optimized: Skip checks that haven't changed
await runGate(plan, config, {
  incrementalLint: true,   // Only lint changed files
  skipCoverage: true,      // Skip coverage for minor changes
});
```

**Savings: 5-10 seconds** vs Ratatui's **5ms**.

### 4. Better Caching
```typescript
// Current: Basic cache invalidation
const cached = cache.get(fileHash);

// Optimized: Smarter cache
const cached = cache.get({
  fileHash,
  dependencies: ['eslintrc', 'package.json'],
  timestamp,
});
```

**Savings: 1-3 seconds** vs Ratatui's **5ms**.

## Conclusion

### TUI Rendering is NOT the Bottleneck

| Optimization | Latency Savings | Implementation Effort |
|--------------|-----------------|----------------------|
| **Parallel check execution** | ~5000ms | Medium (refactor GateRunner) |
| **Adaptive debouncing** | ~250ms | Low (config change) |
| **Incremental checks** | ~5000-10000ms | High (ESLint/Vitest integration) |
| **Better caching** | ~1000-3000ms | Medium (cache strategy) |
| **Switching to Ratatui** | ~5ms | Very High (4-6 weeks + ongoing) |

### Recommendation

**Don't switch to Ratatui for watch mode performance.** Instead:

1. **Short term (1 week):**
   - Add progress events to `GateRunner`
   - Implement parallel check execution
   - Add adaptive debouncing

2. **Medium term (2-4 weeks):**
   - Incremental ESLint (only changed files)
   - Smarter cache invalidation
   - Watch mode TUI dashboard (using Ink)

3. **Never:**
   - Switch to Ratatui for 5ms render savings

### If You Still Want Better Responsiveness

The watch mode plan (`plans/modules/tui.aps.md`) includes building an interactive dashboard:

```
┌─ Anvil Watch Mode ────────────────────────────────────────┐
│ Status: ✓ Passing                              [q] Quit    │
│                                                             │
├─ Current Run ──────────────────────────────────────────────┤
│ ⣾ Running coverage (3/4)                       ETA: 3s     │
│ ✓ lint (1.2s)  ✓ test (3.4s)  ⣾ coverage  ⊘ secrets       │
├─ History ──────────────────────────────────────────────────┤
│ ✓ 13:45:23  src/schema.ts           320ms  Passed          │
│ ✗ 13:44:18  src/validator.ts        450ms  Failed (lint)   │
└─────────────────────────────────────────────────────────────┘
```

**This will make watch mode FEEL more responsive** (by showing progress), even though total time is the same.

**Ink is perfectly capable of this.** Ratatui offers no advantage.

## References

- `core/src/watch/orchestrator.ts:232-234` - Action execution timing
- `cli/src/commands/watch.ts:190-196` - Gate handler blocks on runGate()
- `plans/modules/tui.aps.md` - Watch dashboard UI plan
- ADR-006: Ink vs Ratatui Assessment - Technology stack analysis

---

**Analysis Date:** 2026-01-08
**Conclusion:** TUI rendering is <1% of watch mode latency. Optimize check execution, not rendering.
