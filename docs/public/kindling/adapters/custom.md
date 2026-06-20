---
id: custom
title: Custom Integrations
description: Build your own integration on the Rust or TypeScript APIs.
sidebar_position: 4
---

# Custom Integrations

If your tool isn't covered by the [Claude Code](/kindling/adapters/claude-code),
[OpenCode](/kindling/adapters/opencode), or [PocketFlow](/kindling/adapters/pocketflow)
adapters, you can integrate directly against Kindling's APIs. There are three
ways in, depending on your language and concurrency needs.

| Approach            | Use when                                                            |
| ------------------- | ------------------------------------------------------------------- |
| **Rust, daemon**    | A Rust integration that should share a project safely with other tools. The default choice. |
| **Rust, embedded**  | A Rust process that wants in-process access with no daemon.         |
| **TypeScript**      | A Node integration; builds on the `@eddacraft/kindling` package.    |

The shape is always the same: **open a capsule, append observations, retrieve,
close with a summary.** See [Observations](/kindling/concepts/observations),
[Capsules](/kindling/concepts/capsules), and
[Retrieval](/kindling/concepts/retrieval) for the model.

## Rust, daemon-backed

[`kindling-client`](/kindling/reference/crates) is a thin async client that talks
to the [daemon](/kindling/concepts/storage#the-daemon) over a Unix domain
socket, and auto-spawns it on first call. This is the recommended choice for
concurrent, multi-tool access.

```toml
[dependencies]
kindling-client = "0.1"
```

```rust
use kindling_client::{Client, CapsuleType, ScopeIds};

# async fn run() -> Result<(), kindling_client::ClientError> {
let client = Client::new()?;

// Open a session capsule
let capsule = client
    .open_capsule(CapsuleType::Session, "investigate bug", ScopeIds::default(), None)
    .await?;
# let _ = capsule;
# Ok(())
# }
```

Domain types (`CapsuleType`, `ScopeIds`, `Observation`, …) are re-exported from
`kindling-client`, so you don't need to depend on `kindling-types` directly.

### The v1 wire contract

The client (and any other HTTP/1-over-UDS caller) speaks this endpoint set,
sending an `X-Kindling-Project` header on every data endpoint to route to the
right per-project database:

```text
GET    /v1/health                  → { version, schemaVersion, projects }
POST   /v1/capsules                → Capsule
GET    /v1/capsules/open?sessionId → Capsule | null
PATCH  /v1/capsules/:id/close      → Capsule
POST   /v1/observations            → Observation
POST   /v1/observations/:id/forget → 204
POST   /v1/retrieve                → RetrieveResult
POST   /v1/pins                    → Pin
DELETE /v1/pins/:id                → 204
POST   /v1/context/session-start   → { additionalContext }
POST   /v1/context/pre-compact     → { additionalContext }
```

## Rust, embedded

When you explicitly want single-process, in-process access with no daemon, use
[`kindling-service`](/kindling/reference/crates). Its method surface mirrors the
client, so you can swap between embedded and daemon-backed access.

```toml
[dependencies]
kindling-service = "0.1"
```

## TypeScript

The `@eddacraft/kindling` package bundles the service, the SQLite store, and the
local FTS provider.

```bash
npm install @eddacraft/kindling
```

```typescript
import { randomUUID } from 'node:crypto';
import {
  KindlingService,
  openDatabase,
  SqliteKindlingStore,
  LocalFtsProvider,
} from '@eddacraft/kindling';

const db = openDatabase({ path: './my-memory.db' });
const store = new SqliteKindlingStore(db);
const provider = new LocalFtsProvider(db);
const service = new KindlingService({ store, provider });

// Open a session capsule
const capsule = service.openCapsule({
  type: 'session',
  intent: 'debug authentication issue',
  scopeIds: { sessionId: 'session-1', repoId: 'my-project' },
});

// Capture an observation
service.appendObservation(
  {
    id: randomUUID(),
    kind: 'error',
    content: 'JWT validation failed: token expired',
    provenance: { stack: 'Error: Token expired\n  at validateToken.ts:42' },
    scopeIds: { sessionId: 'session-1' },
    ts: Date.now(),
    redacted: false,
  },
  { capsuleId: capsule.id },
);

// Retrieve
const results = await service.retrieve({
  query: 'authentication token',
  scopeIds: { sessionId: 'session-1' },
});

// Close with a summary
service.closeCapsule(capsule.id, {
  generateSummary: true,
  summaryContent: 'Fixed JWT expiration check in token validation middleware',
});

db.close();
```

Adapter authors who only need the domain types and service (for example to
target the browser) can depend on the lighter `@eddacraft/kindling-core`
instead.

## Guidelines

- **Map events to the fixed [observation kinds](/kindling/concepts/observations#kinds).**
  There are no custom kinds; pick the closest of `tool_call`, `command`,
  `file_diff`, `error`, or `message`.
- **Filter secrets before capture.** Never write credentials into observation
  content — see how the [OpenCode adapter](/kindling/adapters/opencode#content-filtering)
  masks secrets and excludes sensitive paths.
- **Scope everything.** Set `sessionId`/`repoId` so retrieval can be narrowed.
- **Close capsules with a summary** so the conclusion surfaces in the
  current-summary retrieval tier.

## Next

- [Which crate should I use? →](/kindling/reference/crates)
- [CLI reference →](/kindling/reference/cli)
