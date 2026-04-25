# weave-rs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bootstrap the `weave` agent runtime as a standalone repo (`eddacraft/weave-rs`), implement the spike phase proving the core loop works, and update Anvil monorepo artefacts to reflect the new naming and distribution strategy.

**Architecture:** Two workstreams. Workstream A updates Anvil monorepo artefacts (ADR-024 amendment, APS module rename, index/brief updates). Workstream B bootstraps the `weave-rs` repo with the crate scaffold and spike implementation — core types, traits, two-level agent loop, JSONL-tree session, and end-to-end tests with a mock provider. These workstreams are independent and can run in parallel.

**Tech Stack:** Rust (2024 edition), tokio, serde/serde_json, futures, async-trait, uuid, thiserror, tokio-util

**Spec:** `plans/specs/2026-04-17-weave-rs-standalone-design.md`

---

## File Structure

### Workstream A: Anvil monorepo artefact updates

These are documentation/plan file edits, not code changes.

- Modify: `plans/decisions/024-internal-agent-harness.md` — amend naming, repo location, distribution
- Modify: `plans/decisions/DECISION-LOG.md` — update ADR-024 description
- Rename: `plans/modules/literate-core.aps.md` → `plans/modules/weave.aps.md` — rename scope LCORE → WEAVE
- Modify: `plans/index.aps.md` — update Agent Infrastructure table entry
- Modify: `docs/internal/weave-feature-brief.md` — update naming throughout

### Workstream B: weave-rs standalone repo

All paths below are relative to the `weave-rs` repo root.

- Create: `Cargo.toml` — crate metadata, dependencies, feature gates
- Create: `LICENSE` — Apache-2.0
- Create: `src/lib.rs` — public API re-exports
- Create: `src/types.rs` — Message, Content, Context, Model, StreamEvent, ToolCall, ToolUse
- Create: `src/tool.rs` — Tool trait, ToolResult, dispatch pipeline
- Create: `src/provider.rs` — Provider trait, StreamOptions
- Create: `src/stream.rs` — StreamEvent enum, async Stream normalisation
- Create: `src/event.rs` — AgentEvent enum, EventHandler trait
- Create: `src/agent.rs` — AgentConfig, AgentState, steering/follow-up queues
- Create: `src/agent_loop.rs` — run_agent_loop(), two-level iteration
- Create: `src/session.rs` — SessionStore trait, JsonlSessionStore
- Create: `tests/spike_loop.rs` — end-to-end spike test (mock provider + dummy tools)
- Create: `tests/spike_session.rs` — JSONL session tree test (write, branch, rebuild)
- Create: `.github/workflows/ci.yml` — check, test, clippy

---

## Workstream A: Anvil Monorepo Artefact Updates

### Task 1: Amend ADR-024

**Files:**
- Modify: `plans/decisions/024-internal-agent-harness.md`
- Modify: `plans/decisions/DECISION-LOG.md`

- [ ] **Step 1: Add amendment header to ADR-024**

Add an amendment block after the Status line in ADR-024:

```markdown
## Status

Proposed

### Amendment — 2026-04-17

Naming and hosting changed per `plans/specs/2026-04-17-weave-rs-standalone-design.md`:

- **literate-core** → **weave** (crate name), repo `eddacraft/weave-rs` (standalone)
- **anvil-agent** → **anvil-weave** (harness crate in monorepo)
- **APS scope** LCORE → WEAVE
- **Hosting** changed from monorepo-first to standalone from day one
- **Distribution** changed from extract-later to path dep → pre-releases → crates.io
- **Zero-dep invariant** now structurally enforced (separate repo) instead of CI cargo-metadata check
```

- [ ] **Step 2: Update the Decision section**

In the Decision section, update the two paragraphs to reference the new names and locations:

```markdown
## Decision

Build **weave**, a thin, provider-agnostic agent runtime crate in a standalone
repo at `eddacraft/weave-rs`. It captures the irreducible kernel of an agent
runtime (message loop, tool dispatch, provider abstraction, session persistence)
with zero opinions about which LLM, which tools, or where sessions live.

Build **anvil-weave**, an Anvil-specific harness at `crates/anvil-weave/` that
depends on weave and adds domain tools (`graph_query`, `policy_eval`) with
direct, zero-copy access to the kernel's semantic graph.
```

- [ ] **Step 3: Update the Architecture diagram**

Replace the architecture ASCII diagram:

```
weave (Apache-2.0, standalone repo: eddacraft/weave-rs)
├── types       — Message, Content, Context, Model, StreamEvent
├── tool        — Tool trait, ToolResult, execution pipeline
├── provider    — Provider trait, registry, feature-gated impls
├── stream      — Streaming event types, async Stream normalisation
├── agent       — AgentState, event emission, steering/follow-up queues
├── agent_loop  — Two-level loop (inner: tool calls, outer: follow-ups)
└── session     — SessionStore trait, default JSONL-tree implementation

anvil-weave (source-proprietary, crates/anvil-weave/ in Anvil monorepo)
├── tools/
│   ├── graph_query   — Query semantic graph (petgraph, zero-copy)
│   ├── policy_eval   — Evaluate policy against current state
│   ├── read / edit   — Standard file operations
│   └── bash          — Sandboxed shell execution
├── harness           — Wire tools + provider + session → Agent
├── triggers          — Kernel event → agent reasoning triggers
└── compaction        — Anvil-specific context compaction strategy
```

- [ ] **Step 4: Update Key Constraints**

Replace constraint 3:

```markdown
3. **Standalone from day one** — develop in `eddacraft/weave-rs` for
   independent versioning and multi-consumer access. During active development,
   Anvil monorepo uses a path dependency; transition to crates.io version deps
   once API stabilises.
```

- [ ] **Step 5: Update the "Why not standalone" rationale**

Replace the monorepo vs standalone table with:

```markdown
### Why standalone from day one?

| Factor | Monorepo | Standalone (chosen) |
|--------|----------|---------------------|
| Multi-consumer access | Requires subtree split or git dep | Direct: clone, path dep, or crates.io |
| API churn during discovery | Private, free to break | Pre-release semver (0.1.0-alpha.N) |
| Cross-boundary refactors | One PR, one CI | Two repos, path dep during dev mitigates |
| Zero-dep invariant | CI cargo-metadata check | Structural (separate repo) |
| OSS positioning | Delayed | Immediate |

Stable trait surface (confident from pi-mono analysis) reduces the API churn
risk. Multiple consumers exist now, not "someday." Standalone wins.
```

- [ ] **Step 6: Update References section**

Update the naming in the References section to use `weave` and `anvil-weave`
instead of `literate-core` and `anvil-agent`. Update the APS module reference
from LCORE to WEAVE.

- [ ] **Step 7: Search and replace remaining literate-core / anvil-agent references**

Scan the full ADR for any remaining references to `literate-core` or
`anvil-agent` and replace with `weave` or `anvil-weave` respectively. Also
replace `LCORE` with `WEAVE` in APS references.

- [ ] **Step 8: Update DECISION-LOG.md**

In the Agent Infrastructure table, update the description:

```markdown
| [024](024-internal-agent-harness.md) | Thin agent runtime (weave, Apache-2.0) standalone at eddacraft/weave-rs; anvil-weave harness with zero-copy graph access | Proposed |
```

- [ ] **Step 9: Commit**

```bash
git add plans/decisions/024-internal-agent-harness.md plans/decisions/DECISION-LOG.md
git commit -m "$(cat <<'EOF'
docs(plans): amend ADR-024 for weave-rs standalone strategy

Rename literate-core → weave, anvil-agent → anvil-weave. Change hosting
from monorepo-first to standalone repo (eddacraft/weave-rs) from day one.
Update distribution strategy: path dep → pre-releases → crates.io.

EOF
)"
```

---

### Task 2: Rename APS module LCORE → WEAVE

**Files:**
- Rename: `plans/modules/literate-core.aps.md` → `plans/modules/weave.aps.md`
- Modify: `plans/modules/weave.aps.md` (all content updates)

- [ ] **Step 1: Rename the file**

```bash
git mv plans/modules/literate-core.aps.md plans/modules/weave.aps.md
```

- [ ] **Step 2: Update the APS comment header**

Replace the top comment block:

```markdown
<!--
APS Module: weave + anvil-weave
================================
Thin, provider-agnostic agent runtime (weave) in standalone repo
(eddacraft/weave-rs) plus Anvil-specific harness (anvil-weave) with
zero-copy semantic graph access.

Scopes: WEAVE (weave crate), AHARNESS (anvil-weave crate)
-->
```

- [ ] **Step 3: Update the module title and ID table**

```markdown
# weave — Agent Runtime

| ID    | Owner | Status |
| ----- | ----- | ------ |
| WEAVE | —     | Draft  |
```

- [ ] **Step 4: Update the Purpose section**

Replace references to `literate-core` with `weave` and
`crates/literate-core/` with `eddacraft/weave-rs`. Replace `anvil-agent`
with `anvil-weave`. Update the solution paragraph:

```markdown
**Solution:** Two crates. `weave` captures the irreducible kernel of an agent
runtime (~2500 LOC, ~15 dependencies, Apache-2.0) in a standalone repo
(`eddacraft/weave-rs`). `anvil-weave` layers on domain-specific tools
(`graph_query`, `policy_eval`) with direct petgraph access, and lives in the
Anvil monorepo at `crates/anvil-weave/`.
```

- [ ] **Step 5: Update In Scope sections**

Replace `literate-core` heading with `weave (Apache-2.0, eddacraft/weave-rs)`.
Replace `anvil-agent` heading with `anvil-weave (source-proprietary, crates/anvil-weave/)`.
Remove the CI enforcement bullet about cargo-metadata (invariant is now structural).

- [ ] **Step 6: Update Interfaces section**

Replace all `literate-core` references with `weave` and `anvil-agent` with
`anvil-weave`. Update the Depends on section to note that `anvil-weave`
depends on `weave` via path/crates.io dep, not workspace dep.

- [ ] **Step 7: Update Constraints section**

Replace:
- "literate-core must have zero anvil-* dependencies (CI-enforced)" →
  "weave must have zero anvil-* dependencies (structurally enforced — separate repo)"
- "literate-core must be Apache-2.0" → "weave must be Apache-2.0"
- "literate-core Cargo.toml" → "weave Cargo.toml"

- [ ] **Step 8: Update Acceptance Criteria**

Replace all `literate-core` → `weave`, `anvil-agent` → `anvil-weave`.
Remove the `git subtree split` criterion (no longer needed — already standalone).
Add: `weave-rs repo builds and tests independently (cargo test in eddacraft/weave-rs)`.

- [ ] **Step 9: Rename all task IDs from LCORE-* to WEAVE-***

Rename every task ID:
- `LCORE-001` → `WEAVE-001`, `LCORE-002` → `WEAVE-002`, `LCORE-003` → `WEAVE-003`
- `LCORE-010` through `LCORE-016` → `WEAVE-010` through `WEAVE-016`
- `LCORE-020` → `WEAVE-020`, `LCORE-021` → `WEAVE-021`
- `LCORE-040` through `LCORE-043` → `WEAVE-040` through `WEAVE-043`

Update all Dependencies fields that reference LCORE-* IDs to WEAVE-*.

- [ ] **Step 10: Update WEAVE-003 — no longer workspace integration**

WEAVE-003 was "Validate Cargo workspace integration." In the standalone model
this becomes "Validate standalone repo builds independently":

```markdown
### WEAVE-003: Validate standalone repo builds independently

- **Intent:** Confirm `weave` builds, tests, and lints cleanly as a standalone
  crate outside any Cargo workspace
- **Expected Outcome:** `cargo build`, `cargo test`, `cargo clippy` all pass
  in the `eddacraft/weave-rs` repo with no workspace context
- **Validation:** `cargo test && cargo clippy -- -D warnings`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None
```

- [ ] **Step 11: Update validation commands**

Replace all `cargo test -p literate-core` with `cargo test` (standalone repo,
no `-p` needed). Replace `cargo test -p anvil-agent` with
`cargo test -p anvil-weave`. Replace `cargo build -p anvil-agent` with
`cargo build -p anvil-weave`.

- [ ] **Step 12: Update AHARNESS-030 scaffold path**

Change `crates/anvil-agent/` to `crates/anvil-weave/` in AHARNESS-030.

- [ ] **Step 13: Update WEAVE-040 — CI enforcement**

The zero-dep invariant is now structural (separate repo). Update WEAVE-040:

```markdown
### WEAVE-040: CI workflow for weave-rs repo

- **Intent:** Set up GitHub Actions CI for `eddacraft/weave-rs` with check,
  test, clippy, and dependency audit
- **Expected Outcome:** PRs to weave-rs are validated automatically; the
  zero-dep invariant is structurally guaranteed (no anvil-* crates exist in
  the repo's dependency graph)
- **Validation:** CI passes on a PR
- **Confidence:** high
- **Priority:** High
- **Dependencies:** WEAVE-003
```

- [ ] **Step 14: Remove WEAVE-043 extraction dry run**

Delete WEAVE-043 (extraction dry run). It validated `git subtree split` which
is no longer needed — the repo is already standalone. Update the Stats table
total from 22 to 21.

- [ ] **Step 15: Update Phase 0 description**

Update the Phase 0 heading to note that spike work happens in the
`eddacraft/weave-rs` repo, not in `crates/`.

- [ ] **Step 16: Update the Risks table**

Remove the "Extraction to standalone repo is never prioritised" risk (no
longer applicable). Add:

```markdown
| Two-repo coordination overhead | Medium | Low | Path dep during dev; only matters at release boundaries |
| crates.io name collision | Low | Low | Check availability; fallback: weave-agent, weave-core |
```

- [ ] **Step 17: Update Cross-Product Leverage table**

Add the Personal row:

```markdown
| Personal | project-specific tools | General-purpose Rust agents |
```

- [ ] **Step 18: Commit**

```bash
git add plans/modules/weave.aps.md
git commit -m "$(cat <<'EOF'
docs(plans): rename LCORE module to WEAVE for standalone weave-rs

Rename literate-core.aps.md → weave.aps.md. Update all task IDs from
LCORE-* to WEAVE-*, paths from crates/literate-core to eddacraft/weave-rs,
anvil-agent to anvil-weave. Remove extraction dry run task (now standalone).

EOF
)"
```

---

### Task 3: Update plans/index.aps.md

**Files:**
- Modify: `plans/index.aps.md`

- [ ] **Step 1: Update Agent Infrastructure table**

Find the Agent Infrastructure section and update the module entry:

```markdown
### Agent Infrastructure (Draft)

Thin, provider-agnostic agent runtime (weave, Apache-2.0) in standalone repo
(`eddacraft/weave-rs`) plus Anvil-specific harness (anvil-weave) with
zero-copy semantic graph access.

| Module | Scope | Status | Progress | Dependencies |
| ------ | ----- | ------ | -------- | ------------ |
| [weave](./modules/weave.aps.md) | WEAVE, AHARNESS | Draft | 0/21 | KERN (anvil-weave only) |

**Architecture Decision:** [D-024: Internal Agent Harness](./decisions/024-internal-agent-harness.md)
```

- [ ] **Step 2: Update the Active module themes table**

In the bottom themes table, replace:

```markdown
| Agent Infrastructure | [literate-core](./modules/literate-core.aps.md) |
```

with:

```markdown
| Agent Infrastructure | [weave](./modules/weave.aps.md) |
```

- [ ] **Step 3: Commit**

```bash
git add plans/index.aps.md
git commit -m "$(cat <<'EOF'
docs(plans): update index for WEAVE module rename

Update Agent Infrastructure section: literate-core → weave, LCORE → WEAVE,
anvil-agent → anvil-weave, 22 → 21 tasks.

EOF
)"
```

---

### Task 4: Update feature brief

**Files:**
- Modify: `docs/internal/weave-feature-brief.md`

- [ ] **Step 1: Global rename literate-core → weave**

Replace all occurrences of `literate-core` with `weave` throughout the file.

- [ ] **Step 2: Global rename anvil-agent → anvil-weave**

Replace all occurrences of `anvil-agent` with `anvil-weave` throughout the file.

- [ ] **Step 3: Update the crate layout diagram**

Update the directory structure to show `eddacraft/weave-rs/` for the weave
crate and `crates/anvil-weave/` for the harness. Remove `crates/` prefix from
the weave section since it's now a standalone repo.

- [ ] **Step 4: Update the Key Design Decisions section**

Replace the extractability invariant bullet:

```markdown
**Structural separation.** weave lives in its own repo (`eddacraft/weave-rs`).
It cannot import `anvil-*` crates because they don't exist in its dependency
graph. No CI check needed — the invariant is architectural.
```

- [ ] **Step 5: Update How to Contribute section**

Replace path references:
- "`crates/literate-core/`" → "`eddacraft/weave-rs`"
- "Never import from `anvil-*` crates" → keep (still true)
- "`plans/modules/literate-core.aps.md` (LCORE scope)" →
  "`plans/modules/weave.aps.md` (WEAVE scope)"

- [ ] **Step 6: Update Open Source Strategy section**

Replace `literate-core` with `weave` and note that it ships as a standalone
repo from day one rather than being extracted later.

- [ ] **Step 7: Update Timeline section**

Replace "22 work items" with "21 work items" and "ADR-024 proposed" with
"ADR-024 proposed (amended 2026-04-17 for standalone strategy)."

- [ ] **Step 8: Commit**

```bash
git add docs/internal/weave-feature-brief.md
git commit -m "$(cat <<'EOF'
docs: update feature brief for weave-rs naming and standalone strategy

Rename literate-core → weave, anvil-agent → anvil-weave throughout.
Update paths, crate layout, and contribution guide for standalone repo.

EOF
)"
```

---

## Workstream B: weave-rs Repo Bootstrap + Spike

All paths in this workstream are relative to the `eddacraft/weave-rs` repo.
The user has already created the repo. Clone it locally before starting.

### Task 5: Scaffold Cargo.toml and project files

**Files:**
- Create: `Cargo.toml`
- Create: `LICENSE`
- Create: `src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "weave"
version = "0.1.0-alpha.1"
edition = "2024"
rust-version = "1.85"
license = "Apache-2.0"
description = "Minimal, provider-agnostic agent runtime for Rust"
repository = "https://github.com/eddacraft/weave-rs"
keywords = ["agent", "llm", "ai", "runtime"]
categories = ["asynchronous"]

[dependencies]
async-trait = "0.1"
futures = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tokio = { version = "1", features = ["rt", "sync", "macros"] }
tokio-util = "0.7"
uuid = { version = "1", features = ["v4", "serde"] }

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }

[features]
default = []
```

- [ ] **Step 2: Create LICENSE**

Copy the standard Apache-2.0 licence text. The full text is available at
https://www.apache.org/licenses/LICENSE-2.0.txt

- [ ] **Step 3: Create src/lib.rs**

```rust
// SPDX-License-Identifier: Apache-2.0

pub mod types;
pub mod tool;
pub mod provider;
pub mod stream;
pub mod event;
pub mod agent;
pub mod agent_loop;
pub mod session;
```

- [ ] **Step 4: Create stub modules**

Create each module file with a single-line comment so the crate compiles:

`src/types.rs`:
```rust
// SPDX-License-Identifier: Apache-2.0
```

Repeat for `src/tool.rs`, `src/provider.rs`, `src/stream.rs`, `src/event.rs`,
`src/agent.rs`, `src/agent_loop.rs`, `src/session.rs`.

- [ ] **Step 5: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat: scaffold weave crate with stub modules

Cargo.toml with core deps (serde, tokio, futures, async-trait, uuid,
thiserror). Eight module stubs. Apache-2.0 license.

EOF
)"
```

---

### Task 6: Core types

**Files:**
- Create: `src/types.rs`

- [ ] **Step 1: Write types test**

Add tests at the bottom of `src/types.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_roundtrips_through_serde() {
        let msg = Message::user("hello");
        let json = serde_json::to_string(&msg).unwrap();
        let back: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(back.role, Role::User);
    }

    #[test]
    fn assistant_message_with_tool_calls() {
        let tool_use = ToolUse {
            id: "call_1".into(),
            name: "read_file".into(),
            input: serde_json::json!({"path": "src/lib.rs"}),
        };
        let msg = Message::assistant(vec![
            Content::text("Let me read that."),
            Content::ToolUse(tool_use),
        ]);
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content.len(), 2);
    }

    #[test]
    fn tool_result_message() {
        let msg = Message::tool_result("call_1", "file contents here");
        assert_eq!(msg.role, Role::User);
        match &msg.content[0] {
            Content::ToolResult(r) => {
                assert_eq!(r.tool_use_id, "call_1");
                assert!(!r.is_error);
            }
            _ => panic!("expected ToolResult"),
        }
    }

    #[test]
    fn context_builds_with_system_and_messages() {
        let ctx = Context {
            system: Some("You are a helpful agent.".into()),
            messages: vec![Message::user("hi")],
            tools: vec![],
            model: Model {
                api_type: ApiType::Anthropic,
                name: "claude-sonnet-4-20250514".into(),
                max_tokens: 8192,
            },
        };
        assert_eq!(ctx.messages.len(), 1);
        assert!(ctx.system.is_some());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test types`
Expected: FAIL — types not defined

- [ ] **Step 3: Implement types**

Write the full `src/types.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultContent {
    pub tool_use_id: String,
    pub content: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Content {
    Text { text: String },
    ToolUse(ToolUse),
    ToolResult(ToolResultContent),
    Thinking { text: String },
}

impl Content {
    pub fn text(s: impl Into<String>) -> Self {
        Content::Text { text: s.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<Content>,
}

impl Message {
    pub fn user(text: &str) -> Self {
        Self {
            role: Role::User,
            content: vec![Content::text(text)],
        }
    }

    pub fn assistant(content: Vec<Content>) -> Self {
        Self {
            role: Role::Assistant,
            content,
        }
    }

    pub fn assistant_text(text: &str) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![Content::text(text)],
        }
    }

    pub fn tool_result(tool_use_id: &str, content: &str) -> Self {
        Self {
            role: Role::User,
            content: vec![Content::ToolResult(ToolResultContent {
                tool_use_id: tool_use_id.to_string(),
                content: content.to_string(),
                is_error: false,
            })],
        }
    }

    pub fn tool_error(tool_use_id: &str, error: &str) -> Self {
        Self {
            role: Role::User,
            content: vec![Content::ToolResult(ToolResultContent {
                tool_use_id: tool_use_id.to_string(),
                content: error.to_string(),
                is_error: true,
            })],
        }
    }

    pub fn tool_uses(&self) -> Vec<&ToolUse> {
        self.content
            .iter()
            .filter_map(|c| match c {
                Content::ToolUse(tu) => Some(tu),
                _ => None,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiType {
    Anthropic,
    OpenAi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub api_type: ApiType,
    pub name: String,
    pub max_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub system: Option<String>,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSchema>,
    pub model: Model,
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test types`
Expected: all 4 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/types.rs
git commit -m "$(cat <<'EOF'
feat: add core types (Message, Content, Context, Model)

Message with role (user/assistant), Content variants (text, tool_use,
tool_result, thinking), Context for LLM calls, Model metadata.
Serde roundtrip tested.

EOF
)"
```

---

### Task 7: Tool trait

**Files:**
- Create: `src/tool.rs`

- [ ] **Step 1: Write tool trait tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolResultContent;

    struct EchoTool;

    #[async_trait::async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str { "echo" }
        fn description(&self) -> &str { "Echoes input back" }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string" }
                },
                "required": ["text"]
            })
        }
        async fn execute(
            &self,
            _call_id: &str,
            params: serde_json::Value,
            _signal: CancellationSignal,
        ) -> ToolResult {
            let text = params["text"].as_str().unwrap_or("").to_string();
            ToolResult::success(text)
        }
    }

    struct ReadOnlyTool;

    #[async_trait::async_trait]
    impl Tool for ReadOnlyTool {
        fn name(&self) -> &str { "peek" }
        fn description(&self) -> &str { "Read-only peek" }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
        fn is_readonly(&self) -> bool { true }
        async fn execute(
            &self,
            _call_id: &str,
            _params: serde_json::Value,
            _signal: CancellationSignal,
        ) -> ToolResult {
            ToolResult::success("peeked".to_string())
        }
    }

    #[tokio::test]
    async fn echo_tool_returns_input() {
        let tool = EchoTool;
        let result = tool
            .execute("c1", serde_json::json!({"text": "hello"}), CancellationSignal::new())
            .await;
        assert!(!result.is_error);
        assert_eq!(result.content, "hello");
    }

    #[test]
    fn readonly_tool_reports_readonly() {
        let tool = ReadOnlyTool;
        assert!(tool.is_readonly());
    }

    #[test]
    fn mutable_tool_defaults_to_not_readonly() {
        let tool = EchoTool;
        assert!(!tool.is_readonly());
    }

    #[test]
    fn tool_schema_converts_to_tool_schema_type() {
        let tool = EchoTool;
        let schema = tool.to_schema();
        assert_eq!(schema.name, "echo");
        assert_eq!(schema.description, "Echoes input back");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test tool`
Expected: FAIL — Tool trait not defined

- [ ] **Step 3: Implement Tool trait**

Write the full `src/tool.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0

use crate::types::ToolSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct CancellationSignal {
    token: tokio_util::sync::CancellationToken,
}

impl CancellationSignal {
    pub fn new() -> Self {
        Self {
            token: tokio_util::sync::CancellationToken::new(),
        }
    }

    pub fn cancel(&self) {
        self.token.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    pub fn child(&self) -> Self {
        Self {
            token: self.token.child_token(),
        }
    }
}

impl Default for CancellationSignal {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

impl ToolResult {
    pub fn success(content: String) -> Self {
        Self {
            content,
            is_error: false,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            content: message,
            is_error: true,
        }
    }
}

#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;

    fn is_readonly(&self) -> bool {
        false
    }

    async fn execute(
        &self,
        call_id: &str,
        params: serde_json::Value,
        signal: CancellationSignal,
    ) -> ToolResult;

    fn to_schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test tool`
Expected: all 4 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/tool.rs
git commit -m "$(cat <<'EOF'
feat: add Tool trait with cancellation and readonly distinction

Tool trait with async execute, is_readonly for parallelisation,
CancellationSignal wrapping tokio CancellationToken, ToolResult type.

EOF
)"
```

---

### Task 8: Stream types and Provider trait

**Files:**
- Create: `src/stream.rs`
- Create: `src/provider.rs`

- [ ] **Step 1: Write stream and provider tests**

Add to `src/provider.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::StreamEvent;
    use crate::types::*;
    use futures::StreamExt;

    struct MockProvider {
        response: String,
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        async fn stream(
            &self,
            _context: &Context,
            _options: &StreamOptions,
        ) -> Result<EventStream, ProviderError> {
            let response = self.response.clone();
            let stream = futures::stream::iter(vec![
                StreamEvent::MessageStart,
                StreamEvent::ContentDelta {
                    text: response,
                },
                StreamEvent::MessageEnd {
                    stop_reason: StopReason::EndTurn,
                },
            ]);
            Ok(Box::pin(stream))
        }
    }

    #[tokio::test]
    async fn mock_provider_streams_response() {
        let provider = MockProvider {
            response: "Hello!".into(),
        };
        let ctx = Context {
            system: None,
            messages: vec![Message::user("hi")],
            tools: vec![],
            model: Model {
                api_type: ApiType::Anthropic,
                name: "mock".into(),
                max_tokens: 1024,
            },
        };
        let mut stream = provider
            .stream(&ctx, &StreamOptions::default())
            .await
            .unwrap();

        let first = stream.next().await.unwrap();
        assert!(matches!(first, StreamEvent::MessageStart));

        let second = stream.next().await.unwrap();
        match second {
            StreamEvent::ContentDelta { text } => assert_eq!(text, "Hello!"),
            _ => panic!("expected ContentDelta"),
        }

        let third = stream.next().await.unwrap();
        assert!(matches!(
            third,
            StreamEvent::MessageEnd {
                stop_reason: StopReason::EndTurn
            }
        ));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test provider`
Expected: FAIL — types not defined

- [ ] **Step 3: Implement stream types**

Write `src/stream.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    MessageStart,
    ContentDelta { text: String },
    ThinkingDelta { text: String },
    ToolUseStart { id: String, name: String },
    ToolUseDelta { json: String },
    ToolUseEnd,
    MessageEnd { stop_reason: StopReason },
}
```

- [ ] **Step 4: Implement Provider trait**

Write `src/provider.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0

use crate::stream::StreamEvent;
use crate::types::Context;
use futures::Stream;
use std::pin::Pin;
use thiserror::Error;

pub type EventStream = Pin<Box<dyn Stream<Item = StreamEvent> + Send>>;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("request failed: {0}")]
    Request(String),
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },
    #[error("provider error: {0}")]
    Other(String),
}

#[derive(Debug, Clone, Default)]
pub struct StreamOptions {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stop_sequences: Vec<String>,
}

#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    async fn stream(
        &self,
        context: &Context,
        options: &StreamOptions,
    ) -> Result<EventStream, ProviderError>;
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test provider`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/stream.rs src/provider.rs
git commit -m "$(cat <<'EOF'
feat: add Provider trait and StreamEvent types

Provider trait with async streaming, StreamOptions, ProviderError.
StreamEvent enum covering message lifecycle, content deltas,
thinking, and tool use events. Mock provider tested.

EOF
)"
```

---

### Task 9: Event system

**Files:**
- Create: `src/event.rs`

- [ ] **Step 1: Write event tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct CollectorHandler {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl EventHandler for CollectorHandler {
        fn on_event(&self, event: &AgentEvent) {
            let name = match event {
                AgentEvent::AgentStart => "agent_start",
                AgentEvent::TurnStart { .. } => "turn_start",
                AgentEvent::ToolCall { .. } => "tool_call",
                AgentEvent::ToolResult { .. } => "tool_result",
                AgentEvent::TurnEnd { .. } => "turn_end",
                AgentEvent::AgentEnd => "agent_end",
                AgentEvent::Error { .. } => "error",
            };
            self.events.lock().unwrap().push(name.to_string());
        }
    }

    #[test]
    fn collector_records_events() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let handler = CollectorHandler {
            events: events.clone(),
        };
        handler.on_event(&AgentEvent::AgentStart);
        handler.on_event(&AgentEvent::TurnStart { turn: 0 });
        handler.on_event(&AgentEvent::AgentEnd);

        let recorded = events.lock().unwrap();
        assert_eq!(
            *recorded,
            vec!["agent_start", "turn_start", "agent_end"]
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test event`
Expected: FAIL

- [ ] **Step 3: Implement events**

Write `src/event.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0

use crate::tool::ToolResult;

#[derive(Debug, Clone)]
pub enum AgentEvent {
    AgentStart,
    TurnStart { turn: usize },
    ToolCall { id: String, name: String },
    ToolResult { id: String, result: ToolResult },
    TurnEnd { turn: usize },
    AgentEnd,
    Error { message: String },
}

pub trait EventHandler: Send + Sync {
    fn on_event(&self, event: &AgentEvent);
}

pub struct NoOpHandler;

impl EventHandler for NoOpHandler {
    fn on_event(&self, _event: &AgentEvent) {}
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test event`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/event.rs
git commit -m "$(cat <<'EOF'
feat: add AgentEvent enum and EventHandler trait

Typed lifecycle events: agent start/end, turn start/end, tool
call/result, error. EventHandler trait with NoOpHandler default.

EOF
)"
```

---

### Task 10: Agent state and config

**Files:**
- Create: `src/agent.rs`

- [ ] **Step 1: Write agent state tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Message;

    #[test]
    fn agent_config_builds_with_defaults() {
        let config = AgentConfig {
            system_prompt: "You are helpful.".into(),
            model: crate::types::Model {
                api_type: crate::types::ApiType::Anthropic,
                name: "claude-sonnet-4-20250514".into(),
                max_tokens: 8192,
            },
            max_turns: 10,
            max_tool_concurrency: 4,
        };
        assert_eq!(config.max_turns, 10);
    }

    #[test]
    fn agent_state_tracks_messages() {
        let mut state = AgentState::new();
        state.messages.push(Message::user("hello"));
        state
            .messages
            .push(Message::assistant_text("hi there"));
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn follow_up_queue_push_and_drain() {
        let mut state = AgentState::new();
        state.follow_ups.push("follow-up message".to_string());
        assert_eq!(state.follow_ups.len(), 1);
        let drained: Vec<_> = state.follow_ups.drain(..).collect();
        assert_eq!(drained, vec!["follow-up message"]);
        assert!(state.follow_ups.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test agent::tests`
Expected: FAIL

- [ ] **Step 3: Implement agent state**

Write `src/agent.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0

use crate::types::{Message, Model};

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub system_prompt: String,
    pub model: Model,
    pub max_turns: usize,
    pub max_tool_concurrency: usize,
}

impl AgentConfig {
    pub fn new(system_prompt: String, model: Model) -> Self {
        Self {
            system_prompt,
            model,
            max_turns: 20,
            max_tool_concurrency: 4,
        }
    }
}

#[derive(Debug)]
pub struct AgentState {
    pub messages: Vec<Message>,
    pub turn: usize,
    pub follow_ups: Vec<String>,
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            turn: 0,
            follow_ups: Vec::new(),
        }
    }
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test agent::tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/agent.rs
git commit -m "$(cat <<'EOF'
feat: add AgentConfig and AgentState

AgentConfig with system prompt, model, max turns, tool concurrency.
AgentState tracks messages, turn count, and follow-up queue.

EOF
)"
```

---

### Task 11: Agent loop (two-level iteration)

This is the core of the spike — proving the loop works end-to-end.

**Files:**
- Create: `src/agent_loop.rs`
- Create: `tests/spike_loop.rs`

- [ ] **Step 1: Write the spike end-to-end test**

Create `tests/spike_loop.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0

use futures::stream;
use serde_json::json;
use std::sync::{Arc, Mutex};
use weave::agent::AgentConfig;
use weave::agent_loop::run_agent_loop;
use weave::event::{AgentEvent, EventHandler, NoOpHandler};
use weave::provider::{EventStream, Provider, ProviderError, StreamOptions};
use weave::stream::{StopReason, StreamEvent};
use weave::tool::{CancellationSignal, Tool, ToolResult};
use weave::types::*;

/// A mock provider that returns a tool call on the first turn,
/// then a text response on the second turn (after seeing the tool result).
struct TwoTurnProvider {
    call_count: Arc<Mutex<usize>>,
}

#[async_trait::async_trait]
impl Provider for TwoTurnProvider {
    async fn stream(
        &self,
        _context: &Context,
        _options: &StreamOptions,
    ) -> Result<EventStream, ProviderError> {
        let mut count = self.call_count.lock().unwrap();
        *count += 1;
        let turn = *count;
        drop(count);

        if turn == 1 {
            // First turn: call the "greet" tool
            Ok(Box::pin(stream::iter(vec![
                StreamEvent::MessageStart,
                StreamEvent::ToolUseStart {
                    id: "call_1".into(),
                    name: "greet".into(),
                },
                StreamEvent::ToolUseDelta {
                    json: r#"{"name": "World"}"#.into(),
                },
                StreamEvent::ToolUseEnd,
                StreamEvent::MessageEnd {
                    stop_reason: StopReason::ToolUse,
                },
            ])))
        } else {
            // Second turn: text response after seeing tool result
            Ok(Box::pin(stream::iter(vec![
                StreamEvent::MessageStart,
                StreamEvent::ContentDelta {
                    text: "The greeting was: Hello, World!".into(),
                },
                StreamEvent::MessageEnd {
                    stop_reason: StopReason::EndTurn,
                },
            ])))
        }
    }
}

struct GreetTool;

#[async_trait::async_trait]
impl Tool for GreetTool {
    fn name(&self) -> &str {
        "greet"
    }
    fn description(&self) -> &str {
        "Greets someone by name"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        })
    }
    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        _signal: CancellationSignal,
    ) -> ToolResult {
        let name = params["name"].as_str().unwrap_or("stranger");
        ToolResult::success(format!("Hello, {}!", name))
    }
}

struct EventCollector {
    events: Arc<Mutex<Vec<String>>>,
}

impl EventHandler for EventCollector {
    fn on_event(&self, event: &AgentEvent) {
        let label = match event {
            AgentEvent::AgentStart => "agent_start".into(),
            AgentEvent::TurnStart { turn } => format!("turn_start:{turn}"),
            AgentEvent::ToolCall { name, .. } => format!("tool_call:{name}"),
            AgentEvent::ToolResult { .. } => "tool_result".into(),
            AgentEvent::TurnEnd { turn } => format!("turn_end:{turn}"),
            AgentEvent::AgentEnd => "agent_end".into(),
            AgentEvent::Error { message } => format!("error:{message}"),
        };
        self.events.lock().unwrap().push(label);
    }
}

#[tokio::test]
async fn spike_two_turn_conversation_with_tool_call() {
    let provider = TwoTurnProvider {
        call_count: Arc::new(Mutex::new(0)),
    };
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(GreetTool)];
    let events = Arc::new(Mutex::new(Vec::new()));
    let handler = EventCollector {
        events: events.clone(),
    };
    let config = AgentConfig::new(
        "You are a test agent.".into(),
        Model {
            api_type: ApiType::Anthropic,
            name: "mock".into(),
            max_tokens: 1024,
        },
    );

    let messages = run_agent_loop(
        "Say hello to World",
        &config,
        &provider,
        &tools,
        &handler,
    )
    .await
    .unwrap();

    // Should have: user msg, assistant (tool call), tool result, assistant (text)
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0].role, Role::User);
    assert_eq!(messages[1].role, Role::Assistant);
    assert_eq!(messages[2].role, Role::User); // tool result
    assert_eq!(messages[3].role, Role::Assistant);

    // Final assistant message should contain the greeting
    match &messages[3].content[0] {
        Content::Text { text: t } => assert!(t.contains("Hello, World!")),
        _ => panic!("expected text content"),
    }

    // Check event sequence
    let recorded = events.lock().unwrap();
    assert_eq!(recorded[0], "agent_start");
    assert_eq!(recorded[1], "turn_start:0");
    assert!(recorded.contains(&"tool_call:greet".to_string()));
    assert!(recorded.contains(&"tool_result".to_string()));
    assert!(recorded.last().unwrap() == "agent_end");
}

#[tokio::test]
async fn spike_respects_max_turns() {
    // Provider that always requests tool calls — loop should stop at max_turns
    struct InfiniteToolProvider;

    #[async_trait::async_trait]
    impl Provider for InfiniteToolProvider {
        async fn stream(
            &self,
            _context: &Context,
            _options: &StreamOptions,
        ) -> Result<EventStream, ProviderError> {
            Ok(Box::pin(stream::iter(vec![
                StreamEvent::MessageStart,
                StreamEvent::ToolUseStart {
                    id: "call_inf".into(),
                    name: "greet".into(),
                },
                StreamEvent::ToolUseDelta {
                    json: r#"{"name": "Loop"}"#.into(),
                },
                StreamEvent::ToolUseEnd,
                StreamEvent::MessageEnd {
                    stop_reason: StopReason::ToolUse,
                },
            ])))
        }
    }

    let provider = InfiniteToolProvider;
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(GreetTool)];
    let mut config = AgentConfig::new(
        "Test".into(),
        Model {
            api_type: ApiType::Anthropic,
            name: "mock".into(),
            max_tokens: 1024,
        },
    );
    config.max_turns = 3;

    let messages = run_agent_loop(
        "Go forever",
        &config,
        &provider,
        &tools,
        &NoOpHandler,
    )
    .await
    .unwrap();

    // 1 user + 3 turns * (1 assistant + 1 tool_result) = 7 messages
    assert!(messages.len() <= 7);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test spike_loop`
Expected: FAIL — `run_agent_loop` not defined

- [ ] **Step 3: Implement agent_loop**

Write `src/agent_loop.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0

use crate::agent::{AgentConfig, AgentState};
use crate::event::{AgentEvent, EventHandler};
use crate::provider::{Provider, ProviderError, StreamOptions};
use crate::stream::{StopReason, StreamEvent};
use crate::tool::{CancellationSignal, Tool};
use crate::types::{Content, Message, ToolUse};
use futures::StreamExt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoopError {
    #[error("provider error: {0}")]
    Provider(#[from] ProviderError),
    #[error("max turns ({0}) exceeded")]
    MaxTurns(usize),
}

/// Run the two-level agent loop.
///
/// Inner loop: send context to provider, collect response, execute any tool
/// calls, inject results, repeat while the model requests tool use.
///
/// Outer loop: after the inner loop completes (model returns EndTurn), check
/// for follow-up messages in the queue. If any, inject them and restart.
pub async fn run_agent_loop(
    prompt: &str,
    config: &AgentConfig,
    provider: &dyn Provider,
    tools: &[Box<dyn Tool>],
    event_handler: &dyn EventHandler,
) -> Result<Vec<Message>, LoopError> {
    let mut state = AgentState::new();

    event_handler.on_event(&AgentEvent::AgentStart);

    // Initial user message
    state.messages.push(Message::user(prompt));

    // Outer loop: process turns until EndTurn with no follow-ups
    loop {
        if state.turn >= config.max_turns {
            event_handler.on_event(&AgentEvent::AgentEnd);
            return Ok(state.messages);
        }

        event_handler.on_event(&AgentEvent::TurnStart { turn: state.turn });

        // Build context
        let context = crate::types::Context {
            system: Some(config.system_prompt.clone()),
            messages: state.messages.clone(),
            tools: tools.iter().map(|t| t.to_schema()).collect(),
            model: config.model.clone(),
        };

        let options = StreamOptions::default();

        // Stream response from provider
        let mut stream = provider.stream(&context, &options).await?;
        let assistant_message = collect_stream(&mut stream).await;

        // Extract tool calls before pushing message
        let tool_uses: Vec<ToolUse> = assistant_message.tool_uses().into_iter().cloned().collect();
        let stop_reason = detect_stop_reason(&assistant_message, &tool_uses);

        state.messages.push(assistant_message);

        // Inner loop: execute tool calls
        if !tool_uses.is_empty() {
            for tool_use in &tool_uses {
                event_handler.on_event(&AgentEvent::ToolCall {
                    id: tool_use.id.clone(),
                    name: tool_use.name.clone(),
                });

                let result = execute_tool(tools, tool_use).await;

                event_handler.on_event(&AgentEvent::ToolResult {
                    id: tool_use.id.clone(),
                    result: result.clone(),
                });

                let result_message = if result.is_error {
                    Message::tool_error(&tool_use.id, &result.content)
                } else {
                    Message::tool_result(&tool_use.id, &result.content)
                };
                state.messages.push(result_message);
            }
        }

        event_handler.on_event(&AgentEvent::TurnEnd { turn: state.turn });
        state.turn += 1;

        // If model ended turn (no tool calls), check follow-ups
        if stop_reason == StopReason::EndTurn {
            let follow_ups: Vec<String> = state.follow_ups.drain(..).collect();
            if follow_ups.is_empty() {
                break;
            }
            // Inject follow-up as next user message
            for follow_up in follow_ups {
                state.messages.push(Message::user(&follow_up));
            }
        }
        // If stop_reason is ToolUse, inner loop continues (tool results
        // already pushed, next iteration sends them to provider)
    }

    event_handler.on_event(&AgentEvent::AgentEnd);
    Ok(state.messages)
}

async fn collect_stream(
    stream: &mut (dyn futures::Stream<Item = StreamEvent> + Unpin + Send),
) -> Message {
    let mut text_parts = Vec::new();
    let mut tool_uses = Vec::new();
    let mut current_tool_id = String::new();
    let mut current_tool_name = String::new();
    let mut current_tool_json = String::new();

    while let Some(event) = stream.next().await {
        match event {
            StreamEvent::ContentDelta { text } => {
                text_parts.push(text);
            }
            StreamEvent::ThinkingDelta { text } => {
                // Collect thinking but don't include in output for now
                let _ = text;
            }
            StreamEvent::ToolUseStart { id, name } => {
                current_tool_id = id;
                current_tool_name = name;
                current_tool_json.clear();
            }
            StreamEvent::ToolUseDelta { json } => {
                current_tool_json.push_str(&json);
            }
            StreamEvent::ToolUseEnd => {
                let input: serde_json::Value =
                    serde_json::from_str(&current_tool_json).unwrap_or_default();
                tool_uses.push(ToolUse {
                    id: current_tool_id.clone(),
                    name: current_tool_name.clone(),
                    input,
                });
            }
            StreamEvent::MessageStart | StreamEvent::MessageEnd { .. } => {}
        }
    }

    let mut content = Vec::new();
    let combined_text = text_parts.join("");
    if !combined_text.is_empty() {
        content.push(Content::text(combined_text));
    }
    for tu in tool_uses {
        content.push(Content::ToolUse(tu));
    }

    Message::assistant(content)
}

fn detect_stop_reason(message: &Message, tool_uses: &[ToolUse]) -> StopReason {
    if tool_uses.is_empty() {
        StopReason::EndTurn
    } else {
        StopReason::ToolUse
    }
}

async fn execute_tool(tools: &[Box<dyn Tool>], tool_use: &ToolUse) -> crate::tool::ToolResult {
    let tool = tools.iter().find(|t| t.name() == tool_use.name);
    match tool {
        Some(t) => {
            let signal = CancellationSignal::new();
            t.execute(&tool_use.id, tool_use.input.clone(), signal).await
        }
        None => crate::tool::ToolResult::error(format!(
            "tool '{}' not found",
            tool_use.name
        )),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test spike_loop`
Expected: both tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/agent_loop.rs tests/spike_loop.rs
git commit -m "$(cat <<'EOF'
feat: implement two-level agent loop with spike tests

Inner loop: stream provider response, extract tool calls, execute,
inject results, repeat on ToolUse. Outer loop: check follow-up queue,
restart on pending follow-ups. Max turns enforced. End-to-end spike
test with mock provider and greet tool proves the loop works.

EOF
)"
```

---

### Task 12: Session store (JSONL-tree)

**Files:**
- Create: `src/session.rs`
- Create: `tests/spike_session.rs`

- [ ] **Step 1: Write session spike test**

Create `tests/spike_session.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0

use weave::session::{JsonlSessionStore, SessionStore};
use weave::types::Message;

#[tokio::test]
async fn append_and_rebuild_context() {
    let store = JsonlSessionStore::in_memory();

    let e1 = store.append_message(Message::user("hello")).await.unwrap();
    let e2 = store
        .append_message_with_parent(Message::assistant_text("hi"), e1)
        .await
        .unwrap();
    let e3 = store
        .append_message_with_parent(Message::user("how are you?"), e2)
        .await
        .unwrap();

    let context = store.build_context(e3).await.unwrap();
    assert_eq!(context.len(), 3);
    assert_eq!(context[0].role, weave::types::Role::User);
    assert_eq!(context[1].role, weave::types::Role::Assistant);
    assert_eq!(context[2].role, weave::types::Role::User);
}

#[tokio::test]
async fn branch_preserves_history() {
    let store = JsonlSessionStore::in_memory();

    let e1 = store.append_message(Message::user("hello")).await.unwrap();
    let e2 = store
        .append_message_with_parent(Message::assistant_text("hi"), e1)
        .await
        .unwrap();

    // Branch from e1 — alternative response
    let e3 = store
        .append_message_with_parent(Message::assistant_text("hey!"), e1)
        .await
        .unwrap();

    // Original branch
    let ctx_original = store.build_context(e2).await.unwrap();
    assert_eq!(ctx_original.len(), 2);
    match &ctx_original[1].content[0] {
        weave::types::Content::Text { text: t } => assert_eq!(t, "hi"),
        _ => panic!("expected text"),
    }

    // New branch
    let ctx_branch = store.build_context(e3).await.unwrap();
    assert_eq!(ctx_branch.len(), 2);
    match &ctx_branch[1].content[0] {
        weave::types::Content::Text { text: t } => assert_eq!(t, "hey!"),
        _ => panic!("expected text"),
    }
}

#[tokio::test]
async fn entries_are_append_only() {
    let store = JsonlSessionStore::in_memory();

    let e1 = store.append_message(Message::user("first")).await.unwrap();
    let e2 = store
        .append_message_with_parent(Message::user("second"), e1)
        .await
        .unwrap();

    // Both entries exist independently
    let ctx1 = store.build_context(e1).await.unwrap();
    assert_eq!(ctx1.len(), 1);

    let ctx2 = store.build_context(e2).await.unwrap();
    assert_eq!(ctx2.len(), 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test spike_session`
Expected: FAIL — session types not defined

- [ ] **Step 3: Implement session store**

Write `src/session.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0

use crate::types::Message;
use std::collections::HashMap;
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

pub type EntryId = String;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("entry not found: {0}")]
    NotFound(String),
    #[error("storage error: {0}")]
    Storage(String),
}

#[derive(Debug, Clone)]
pub struct SessionEntry {
    pub id: EntryId,
    pub parent_id: Option<EntryId>,
    pub message: Message,
}

#[async_trait::async_trait]
pub trait SessionStore: Send + Sync {
    async fn append_message(&self, message: Message) -> Result<EntryId, SessionError>;

    async fn append_message_with_parent(
        &self,
        message: Message,
        parent: EntryId,
    ) -> Result<EntryId, SessionError>;

    async fn build_context(&self, leaf: EntryId) -> Result<Vec<Message>, SessionError>;
}

/// In-memory JSONL-tree session store.
///
/// Each entry has an id and optional parent_id, forming a tree. Context
/// building walks from leaf to root, producing messages in chronological order.
pub struct JsonlSessionStore {
    entries: Mutex<HashMap<EntryId, SessionEntry>>,
}

impl JsonlSessionStore {
    pub fn in_memory() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl SessionStore for JsonlSessionStore {
    async fn append_message(&self, message: Message) -> Result<EntryId, SessionError> {
        let id = Uuid::new_v4().to_string();
        let entry = SessionEntry {
            id: id.clone(),
            parent_id: None,
            message,
        };
        self.entries.lock().unwrap().insert(id.clone(), entry);
        Ok(id)
    }

    async fn append_message_with_parent(
        &self,
        message: Message,
        parent: EntryId,
    ) -> Result<EntryId, SessionError> {
        let entries = self.entries.lock().unwrap();
        if !entries.contains_key(&parent) {
            return Err(SessionError::NotFound(parent));
        }
        drop(entries);

        let id = Uuid::new_v4().to_string();
        let entry = SessionEntry {
            id: id.clone(),
            parent_id: Some(parent),
            message,
        };
        self.entries.lock().unwrap().insert(id.clone(), entry);
        Ok(id)
    }

    async fn build_context(&self, leaf: EntryId) -> Result<Vec<Message>, SessionError> {
        let entries = self.entries.lock().unwrap();
        let mut path = Vec::new();
        let mut current = Some(leaf.clone());

        while let Some(id) = current {
            let entry = entries
                .get(&id)
                .ok_or_else(|| SessionError::NotFound(id.clone()))?;
            path.push(entry.message.clone());
            current = entry.parent_id.clone();
        }

        path.reverse();
        Ok(path)
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test spike_session`
Expected: all 3 tests PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test`
Expected: all tests PASS (types, tool, provider, event, agent, spike_loop, spike_session)

- [ ] **Step 6: Commit**

```bash
git add src/session.rs tests/spike_session.rs
git commit -m "$(cat <<'EOF'
feat: add SessionStore trait with JSONL-tree implementation

SessionStore trait with append, append_with_parent, build_context.
JsonlSessionStore uses in-memory HashMap with parent_id tree. Context
building walks leaf-to-root. Branching creates alternative leaves
without modifying history. Spike tests cover append, rebuild, and
branching.

EOF
)"
```

---

### Task 13: CI workflow

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create CI workflow**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          components: clippy
      - run: cargo check --all-features
      - run: cargo clippy --all-features -- -D warnings

  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo test --all-features

  fmt:
    name: Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          components: rustfmt
      - run: cargo fmt --check
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "$(cat <<'EOF'
ci: add check, test, and format workflow

Runs cargo check, clippy, test (all features), and fmt check on
push to main and PRs.

EOF
)"
```

---

### Task 14: Final spike validation

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Run fmt check**

Run: `cargo fmt --check`
Expected: no formatting issues (fix with `cargo fmt` if needed)

- [ ] **Step 4: Verify crate has no anvil-* dependencies**

Run: `cargo metadata --format-version 1 | jq '.packages[].dependencies[].name' | grep anvil`
Expected: no output (no anvil dependencies)

- [ ] **Step 5: Tag the spike milestone**

```bash
git tag -a v0.1.0-alpha.0 -m "Spike complete: agent loop, session tree, core traits"
```

---

## Execution Order

**Workstream A** (tasks 1-4) and **Workstream B** (tasks 5-14) are independent
and can run in parallel.

Within Workstream A: tasks 1-4 are sequential (each builds on naming changes
from prior tasks).

Within Workstream B:
- Task 5 (scaffold) must come first
- Tasks 6-10 (types, tool, provider, event, agent) can be done in order
  (each builds on prior types)
- Task 11 (agent loop) depends on tasks 6-10
- Task 12 (session) depends on task 6 only, can run in parallel with 7-11
- Task 13 (CI) can be done any time after task 5
- Task 14 (validation) runs last

```
Workstream A:  1 → 2 → 3 → 4
Workstream B:  5 → 6 → 7 → 8 → 9 → 10 → 11 → 14
                    ↘ 12 ─────────────────────↗
                  ↘ 13 ──────────────────────↗
```
