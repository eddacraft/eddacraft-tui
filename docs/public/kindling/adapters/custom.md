---
id: custom
title: Custom Integrations
description: Build your own integration on the Rust or TypeScript APIs.
sidebar_position: 5
owner: DOCSYNC
---

# Custom Integrations

If your tool isn't covered by the [Claude Code](/kindling/adapters/claude-code),
[OpenCode](/kindling/adapters/opencode), [VS Code](/kindling/adapters/vscode),
or [PocketFlow](/kindling/adapters/pocketflow) adapters, you can integrate
directly against Kindling's APIs. There are three ways in, depending on your
language and concurrency needs.

| Approach           | Use when                                                                                    |
| ------------------ | ------------------------------------------------------------------------------------------- |
| **Rust, daemon**   | A Rust integration that should share a project safely with other tools. The default choice. |
| **Rust, embedded** | A Rust process that wants in-process access with no daemon.                                 |
| **TypeScript**     | A Node integration over the `@eddacraft/kindling` thin client (daemon-backed).              |

The shape is always the same: **open a capsule, append observations, retrieve,
close with a summary.** See [Observations](/kindling/concepts/observations),
[Capsules](/kindling/concepts/capsules), and
[Retrieval](/kindling/concepts/retrieval) for the model.

## Rust, daemon-backed

[`kindling-client`](/kindling/reference/crates) is a thin async client that
talks to the [daemon](/kindling/concepts/storage#the-daemon) over a Unix domain
socket, and auto-spawns it on first call. This is the recommended choice for
concurrent, multi-tool access.

```toml
[dependencies]
kindling-client = "0.2"
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
kindling-service = "0.2"
```

## TypeScript (thin client)

`@eddacraft/kindling` is a daemon-backed client — not an embedded SQLite stack.
It auto-spawns `kindling serve` on first use and ships optional per-platform
binary dependencies at publish time.

```bash
npm install @eddacraft/kindling
```

```typescript
import { Kindling } from '@eddacraft/kindling';

const kindling = new Kindling({
  projectRoot: process.cwd(),
});

// Open a session capsule
const capsule = await kindling.openCapsule({
  kind: 'session',
  intent: 'debug authentication issue',
  scopeIds: {
    sessionId: 'session-1',
    repoId: process.cwd(),
  },
});

// Capture an observation
await kindling.appendObservation(
  {
    kind: 'error',
    content: 'JWT validation failed: token expired',
    provenance: { source: 'my-adapter', stack: 'Error at validate.ts:42' },
    scopeIds: { sessionId: 'session-1', repoId: process.cwd() },
  },
  { capsuleId: capsule.id }
);

// Retrieve
const results = await kindling.retrieve({
  query: 'authentication token',
  scopeIds: { sessionId: 'session-1', repoId: process.cwd() },
});

// Close with a summary
await kindling.closeCapsule(capsule.id, {
  generateSummary: true,
  summaryContent: 'Fixed JWT expiration check in token validation middleware',
});
```

> **Deprecated:** `@eddacraft/kindling-core`, `@eddacraft/kindling-store-*`, and
> the old in-process `KindlingService` / `SqliteKindlingStore` /
> `LocalFtsProvider` stack. New adapters should use the thin `Kindling` client
> above.

## Guidelines

- **Map events to the fixed
  [observation kinds](/kindling/concepts/observations#kinds).** There are no
  custom kinds; pick the closest of `tool_call`, `command`, `file_diff`,
  `error`, or `message`.
- **Filter secrets before capture.** Never write credentials into observation
  content — see how the
  [OpenCode adapter](/kindling/adapters/opencode#content-filtering) masks
  secrets and excludes sensitive paths.
- **Scope everything.** Set `sessionId`/`repoId` so retrieval can be narrowed.
- **Close capsules with a summary** so the conclusion surfaces in the
  current-summary retrieval tier.
- **Recover from crashes** by calling `getOpenCapsule(sessionId)` before opening
  a new capsule for the same session.

## Next

- [Which crate should I use? →](/kindling/reference/crates)
- [Integrations matrix →](/kindling/reference/integrations)
- [CLI reference →](/kindling/reference/cli)
