---
id: custom
title: Custom Adapters
description: Building your own Kindling adapter.
sidebar_position: 3
---

# Custom Adapters

Build adapters to integrate Kindling with your tools.

## Adapter Interface

Adapters implement a simple interface:

```typescript
interface KindlingAdapter {
  name: string;
  version: string;

  // Lifecycle
  init(config: AdapterConfig): Promise<void>;
  destroy(): Promise<void>;

  // Capture
  capture(event: CaptureEvent): Promise<Observation>;

  // Optional: Query
  search?(query: SearchQuery): Promise<Observation[]>;
}
```

## Basic Adapter

```typescript
import { KindlingAdapter, Observation } from '@eddacraft/kindling';

export class MyToolAdapter implements KindlingAdapter {
  name = 'my-tool';
  version = '1.0.0';

  private capsule: string;

  async init(config: AdapterConfig): Promise<void> {
    this.capsule = config.capsule || 'default';
  }

  async destroy(): Promise<void> {
    // Cleanup
  }

  async capture(event: CaptureEvent): Promise<Observation> {
    return {
      content: event.content,
      kind: event.kind || 'discovery',
      tags: [...(event.tags || []), 'my-tool'],
      source: {
        type: 'tool',
        name: 'my-tool',
        version: this.version
      },
      capsule: this.capsule
    };
  }
}
```

## Registration

### Package Export

```typescript
// index.ts
export { MyToolAdapter } from './adapter';
```

### Package.json

```json
{
  "name": "kindling-adapter-mytool",
  "kindling": {
    "adapter": true,
    "entry": "./dist/index.js"
  }
}
```

### Install

```bash
kindling adapter install kindling-adapter-mytool
```

## Event Sources

### Polling

Periodically check for new data:

```typescript
class PollingAdapter implements KindlingAdapter {
  private interval: NodeJS.Timer;

  async init(config: AdapterConfig): Promise<void> {
    this.interval = setInterval(() => this.poll(), 60000);
  }

  private async poll(): Promise<void> {
    const data = await fetchFromMyTool();
    for (const item of data) {
      await this.capture(item);
    }
  }

  async destroy(): Promise<void> {
    clearInterval(this.interval);
  }
}
```

### Webhook

Receive events via HTTP:

```typescript
class WebhookAdapter implements KindlingAdapter {
  private server: http.Server;

  async init(config: AdapterConfig): Promise<void> {
    this.server = http.createServer((req, res) => {
      // Parse webhook payload
      // Call this.capture()
    });
    this.server.listen(config.port || 3456);
  }
}
```

### File Watcher

Watch for file changes:

```typescript
class FileAdapter implements KindlingAdapter {
  private watcher: FSWatcher;

  async init(config: AdapterConfig): Promise<void> {
    this.watcher = watch(config.watchPath, async (event, filename) => {
      const content = await readFile(filename, 'utf-8');
      await this.capture({ content, kind: 'discovery' });
    });
  }
}
```

## Configuration Schema

Define adapter configuration:

```typescript
import { z } from 'zod';

export const MyToolConfigSchema = z.object({
  capsule: z.string().default('default'),
  apiKey: z.string(),
  pollInterval: z.number().default(60000),
  tags: z.array(z.string()).default([])
});

type MyToolConfig = z.infer<typeof MyToolConfigSchema>;
```

## Testing

```typescript
import { describe, it, expect } from 'vitest';
import { MyToolAdapter } from './adapter';

describe('MyToolAdapter', () => {
  it('captures observations', async () => {
    const adapter = new MyToolAdapter();
    await adapter.init({ capsule: 'test' });

    const obs = await adapter.capture({
      content: 'Test observation',
      kind: 'discovery'
    });

    expect(obs.content).toBe('Test observation');
    expect(obs.tags).toContain('my-tool');
  });
});
```

## Publishing

```bash
npm publish kindling-adapter-mytool
```

Users install with:

```bash
kindling adapter install kindling-adapter-mytool
```

---

**Next:** [Memory commands →](/docs/kindling/commands/memory)
