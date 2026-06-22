---
id: pocketflow
title: PocketFlow Adapter
description: Capture workflow node executions as Kindling observations.
sidebar_position: 3
---

# PocketFlow Adapter

The PocketFlow adapter integrates Kindling with
[PocketFlow](https://github.com/The-Pocket/PocketFlow) workflow nodes. Each node
run automatically gets its own capsule, and the full lifecycle (start, output,
error, end) is captured as observations with intent inference and confidence
tracking.

It is a TypeScript package that builds on `@eddacraft/kindling`:

```bash
npm install @eddacraft/kindling-adapter-pocketflow
# or: pnpm add @eddacraft/kindling-adapter-pocketflow
# or: yarn add @eddacraft/kindling-adapter-pocketflow
# or: bun add @eddacraft/kindling-adapter-pocketflow
```

## KindlingNode

`KindlingNode` extends PocketFlow's `Node` with automatic observation capture.
Each run:

1. **prep** — Opens a capsule and records a `node_start` observation
2. **exec** — Your logic runs as normal
3. **post** — Records `node_output` and `node_end`, then closes the capsule

```typescript
import {
  KindlingNode,
  KindlingFlow,
} from '@eddacraft/kindling-adapter-pocketflow';
import type { KindlingNodeContext } from '@eddacraft/kindling-adapter-pocketflow';

// Define a node
class RunTests extends KindlingNode {
  constructor() {
    super({ name: 'run-tests', intent: 'test' });
  }

  async exec(): Promise<unknown> {
    // Your node logic
    return { passed: 42, failed: 0 };
  }
}
```

### Error Handling

If `exec` throws after all retries, `execFallback` records a `node_error`
observation with the error details and stack trace, then closes the capsule
before re-throwing:

```typescript
class RiskyNode extends KindlingNode {
  constructor() {
    // 3 retries with 1 second wait
    super({ name: 'risky-operation' }, 3, 1);
  }

  async exec(): Promise<unknown> {
    // If this fails 3 times, the error is captured
    // with retryCount in provenance
    return await riskyCall();
  }
}
```

### Observations Captured

| Kind          | When                        | Provenance includes                    |
| ------------- | --------------------------- | -------------------------------------- |
| `node_start`  | Before `exec` runs          | Node name, intent, params              |
| `node_output` | After `exec` succeeds       | Node name, output type, duration       |
| `node_error`  | After all retries exhausted | Error type, message, stack, retryCount |
| `node_end`    | Always (success or failure) | Node name, duration, status            |

Output is automatically truncated to 2KB to avoid storing excessive data.

## KindlingFlow

`KindlingFlow` wraps a graph of `KindlingNode`s. The flow itself gets a parent
capsule, and each node creates its own child capsule during its run:

```typescript
const runTests = new RunTests();
const deploy = new DeployNode();

// Wire the graph
runTests.next(deploy);

// Create the flow
const pipeline = new KindlingFlow(runTests, {
  name: 'ci-pipeline',
  intent: 'deploy',
});

// Run it
import { Kindling } from '@eddacraft/kindling';

const kindling = new Kindling({ projectRoot: process.cwd() });

const shared: KindlingNodeContext = {
  kindling,
  scopeIds: { repoId: 'my-app', sessionId: 'ci-run-1' },
};

await pipeline.run(shared);
```

## Intent Inference

Node names are automatically mapped to semantic intents using pattern matching.
This enables smarter retrieval and context organization.

```typescript
import { inferIntent } from '@eddacraft/kindling-adapter-pocketflow';

inferIntent('run-tests'); // -> 'test'
inferIntent('buildApp'); // -> 'build'
inferIntent('deploy_production'); // -> 'deploy'
inferIntent('fixAuthBug'); // -> 'debug'
inferIntent('unknownNode'); // -> 'general'
```

The default patterns cover common development workflows:

| Intent     | Keywords                                        |
| ---------- | ----------------------------------------------- |
| `test`     | test, spec, check, verify, validate, assert     |
| `build`    | build, compile, bundle, pack, transpile         |
| `deploy`   | deploy, publish, release, ship                  |
| `debug`    | fix, debug, repair, patch, hotfix, troubleshoot |
| `feature`  | implement, add, create, feature, develop        |
| `refactor` | refactor, restructure, reorganize, cleanup      |
| `process`  | process, transform, convert, parse, extract     |
| `analyze`  | analyze, research, investigate, explore         |
| `generate` | generate, scaffold, template, init, setup       |
| `store`    | save, store, persist, write, cache, backup      |
| `retrieve` | read, get, load, retrieve, query                |
| `cleanup`  | clean, clear, remove, delete, prune             |
| `monitor`  | log, monitor, track, metric, observe, report    |

You can supply custom patterns:

```typescript
inferIntent('myCustomNode', [
  { keywords: ['custom'], intent: 'my-intent' },
  ...DEFAULT_INTENT_PATTERNS,
]);
```

## Confidence Tracking

The `ConfidenceTracker` maintains a reliability score (0.0 to 1.0) for each node
based on its history. This score is included in observation provenance for
explainability. Retrieval ranking itself is deterministic FTS + recency in
`kindling-provider` — confidence metadata does not change search order.

```typescript
import { ConfidenceTracker } from '@eddacraft/kindling-adapter-pocketflow';

const tracker = new ConfidenceTracker();

tracker.recordSuccess('run-tests');
tracker.recordSuccess('run-tests');
tracker.recordFailure('deploy-prod', 'Connection timeout');

tracker.getConfidence('run-tests'); // 0.7
tracker.getConfidence('deploy-prod'); // 0.35

// Embed in observation provenance
const provenance = tracker.getProvenanceMetadata('run-tests');
// { confidence: 0.7, successCount: 2, failureCount: 0, ... }
```

The confidence algorithm considers:

- **Overall success rate** across all recorded runs
- **Recent trends** weighted more heavily than older results (configurable,
  default 70%)
- **Consecutive failure penalty** that depresses confidence during failure
  streaks
- **Configurable bounds** with floor (0.1) and ceiling (0.95)

```typescript
// Custom configuration
const tracker = new ConfidenceTracker({
  historySize: 20, // Keep more history
  baseConfidence: 0.6, // Higher starting confidence
  successIncrement: 0.1,
  failureDecrement: 0.15,
  minConfidence: 0.1,
  maxConfidence: 0.95,
  recencyWeight: 0.7,
});
```

## Next

- [Build a custom integration →](/kindling/adapters/custom)
