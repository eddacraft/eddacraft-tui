---
id: crates
title: Crates
description: The published Rust crates and which one to depend on.
sidebar_position: 4
---

# Crates

Kindling is published to [crates.io](https://crates.io/crates/kindling) as a set
of focused crates. Most people only need the binary (`cargo install kindling`)
or the client.

## Which crate should I use?

| Crate              | Use it when                                                                                          |
| ------------------ | ---------------------------------------------------------------------------------------------------- |
| `kindling`         | You want the CLI binary: `kindling init`, `log`, `search`, `serve`, and Claude Code hook support.    |
| `kindling-client`  | You are building a Rust integration that should talk to the daemon safely across concurrent tools. **The default SDK choice.** |
| `kindling-service` | You need embedded, in-process access to capsule lifecycle, observation capture, retrieval, and pins. |
| `kindling-server`  | You are extending or embedding the daemon/runtime layer. Most users run `kindling serve` instead.    |
| `kindling-store`   | You are working directly with the SQLite persistence layer.                                          |
| `kindling-provider`| You are working on deterministic local retrieval and ranking.                                        |
| `kindling-types`   | You need the shared domain types directly. Client users get these re-exported from `kindling-client`. |

## Dependency flow

```
        your integration
         ↓            ↓
  kindling-client   kindling-service   (pick one)
         ↓            ↓
   (daemon: server) kindling-store + kindling-provider
                      ↓
                  kindling-types
```

`kindling-client` is deliberately thin: it depends only on `kindling-types` for
domain shapes and speaks the daemon's HTTP/1-over-UDS wire contract. It never
pulls in `rusqlite`, so it stays light for embedding in other tools.

## The daemon-backed client

```toml
[dependencies]
kindling-client = "0.1"
```

```rust
use kindling_client::{Client, CapsuleType, ScopeIds};

# async fn run() -> Result<(), kindling_client::ClientError> {
let client = Client::new()?;

// Health check — reports daemon version and schema version
let health = client.health().await?;
println!("daemon schema v{}", health.schema_version);

// Open a capsule (auto-spawns the daemon on first call)
let capsule = client
    .open_capsule(CapsuleType::Session, "investigate bug", ScopeIds::default(), None)
    .await?;
# let _ = (capsule,);
# Ok(())
# }
```

The client auto-spawns `kindling serve --daemonize` on first use if the daemon
isn't already running, and checks its reported schema version against the
version it was built against.

## Embedded, in-process

When you want no daemon at all:

```toml
[dependencies]
kindling-service = "0.1"
```

`kindling-service` exposes the same method surface as the client, so code can
move between embedded and daemon-backed access with minimal change.

See [Custom Integrations](/kindling/adapters/custom) for fuller examples and the
v1 wire contract.
