# Assessment: Ink vs Ratatui for Anvil TUI

> **Superseded:** This assessment assumed an all-TypeScript stack. With the
> introduction of the Rust kernel
> ([Architecture Evolution](../../docs/architecture/anvil-architecture-evolution.md)),
> the "two languages" argument against Ratatui dissolves. The Ratatui TUI is now
> planned as [RATS — Ratatui TUI](../modules/ratatui-tui.aps.md). The analysis
> below remains valuable reference material. See also
> [ADR-011](./011-rust-core-engine.md) (itself superseded).

## Executive Summary

**Recommendation: Stick with Ink** _(superseded — see note above)_

Ratatui is an excellent TUI library, but switching would introduce significant complexity without meaningful benefits for Anvil's use case. The technology stack mismatch (Rust vs Node.js/TypeScript) creates integration barriers that outweigh Ratatui's performance advantages.

## Context

Anvil has selected **Ink** (React-based TUI for Node.js) per ADR-005, chosen over OpenTUI due to runtime compatibility requirements. This assessment evaluates whether **Ratatui** (Rust-based TUI library) would be a better choice.

## Technology Overview

### Ink
- **Language**: TypeScript/JavaScript (React)
- **Runtime**: Node.js (≥20.0.0)
- **Architecture**: React component model with reconciliation
- **Rendering**: ANSI escape sequences via stdout
- **Ecosystem**: npm packages (`ink-spinner`, `ink-text-input`, etc.)

### Ratatui
- **Language**: Rust
- **Runtime**: Native binary
- **Architecture**: Immediate mode rendering with terminal backend abstraction
- **Rendering**: Crossterm/Termion/Termwiz backends
- **Ecosystem**: Cargo crates (widgets, input handling, layouts)

## Detailed Comparison

### 1. Runtime Integration ⚠️ CRITICAL BLOCKER

| Aspect | Ink | Ratatui |
|--------|-----|---------|
| **Language** | TypeScript (same as Anvil) | Rust (foreign to codebase) |
| **Build Integration** | Standard TypeScript compilation | Requires Rust toolchain + FFI bindings |
| **Deployment** | Single Node.js binary (via pkg/nexe) | Separate Rust binary or N-API module |
| **Data Sharing** | Direct JavaScript objects | IPC, FFI, or subprocess communication |

**Ratatui Integration Options:**

#### Option A: Separate Rust Binary (Subprocess)
```bash
# Anvil would need to spawn Ratatui as subprocess
anvil watch --tui
  └─> spawns: anvil-tui (Rust binary)
      └─> IPC via stdin/stdout or unix sockets
```

**Challenges:**
- **Bidirectional communication**: Need IPC protocol (JSON over pipes, msgpack, etc.)
- **State synchronization**: TypeScript core ↔ Rust TUI state management
- **Error handling**: Process crashes, signal handling, graceful shutdown
- **Deployment complexity**: Two binaries to distribute and maintain
- **Development overhead**: Two separate build systems (TypeScript + Rust)

**Example data flow:**
```typescript
// CLI (TypeScript)
const tuiProcess = spawn('anvil-tui', ['watch']);
tuiProcess.stdin.write(JSON.stringify({
  event: 'gate_started',
  checks: ['lint', 'test']
}));

// TUI (Rust) - separate process
let msg: Event = serde_json::from_str(&input)?;
// Render updates...
```

#### Option B: N-API Native Module (FFI)
```typescript
// TypeScript calls Rust via N-API
import { createTUI } from './native/anvil-tui.node';
const tui = createTUI();
tui.render({ status: 'running', checks: [...] });
```

**Challenges:**
- **N-API complexity**: Manual memory management, lifetime coordination
- **Build matrix**: Native modules for Linux x64/arm64, macOS x64/arm64, Windows x64
- **Cross-compilation**: CI needs Rust + cross toolchains for all platforms
- **Error boundary**: Rust panics can crash Node.js process
- **Development friction**: Changes require rebuilding native module
- **Debugging**: Mixed TypeScript + Rust debugging required

**Build Requirements:**
```yaml
# .github/workflows/build.yml (would need)
- Rust toolchain (stable)
- cargo, rustc
- Cross-compilation targets (6+ platforms)
- N-API headers
- Platform-specific linkers
```

### 2. Development Experience

| Aspect | Ink | Ratatui |
|--------|-----|---------|
| **Team Skill Match** | ✅ React/TS (existing expertise) | ❌ Rust (new language to learn) |
| **Iteration Speed** | ✅ Fast (TypeScript hot reload) | ⚠️ Slower (Rust compile times) |
| **Debugging** | ✅ Standard Node.js debugging | ⚠️ Rust debugging or IPC inspection |
| **Testing** | ✅ React Testing Library (familiar) | ❌ Rust test framework (new) |
| **Contributors** | ✅ Low barrier (React common) | ❌ High barrier (Rust knowledge) |

**Code Example Comparison:**

**Ink (TypeScript/React):**
```tsx
// cli/src/tui/components/WatchDashboard.tsx
import React, { useState } from 'react';
import { Box, Text } from 'ink';

export const WatchDashboard = ({ status, history }) => {
  const [selected, setSelected] = useState(0);

  return (
    <Box flexDirection="column">
      <Text color="green">Status: {status}</Text>
      {history.map((item, i) => (
        <Text key={i} inverse={i === selected}>
          {item.file} - {item.result}
        </Text>
      ))}
    </Box>
  );
};

// Direct integration in CLI
import { render } from 'ink';
render(<WatchDashboard status={watchState.status} />);
```

**Ratatui (Rust):**
```rust
// Would need separate anvil-tui/src/dashboard.rs
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, List, ListItem},
    Terminal
};

struct WatchDashboard {
    status: String,
    history: Vec<HistoryItem>,
    selected: usize,
}

impl WatchDashboard {
    fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .chunks(f.size());

            let status_text = Paragraph::new(format!("Status: {}", self.status));
            f.render_widget(status_text, chunks[0]);

            let items: Vec<ListItem> = self.history
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let style = if i == self.selected {
                        Style::default().bg(Color::White).fg(Color::Black)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!("{} - {}", item.file, item.result))
                        .style(style)
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(list, chunks[1]);
        })?;
        Ok(())
    }
}

// Separate binary, IPC needed to receive data from TypeScript CLI
```

### 3. Performance

| Aspect | Ink | Ratatui |
|--------|-----|---------|
| **Render Speed** | ~5-10ms (React reconciliation) | ~1-3ms (immediate mode) |
| **Memory** | ~50-100MB (Node.js heap) | ~5-10MB (Rust native) |
| **Startup Time** | ~100-200ms (Node.js + imports) | ~10-50ms (native binary) |
| **Relevance** | ✅ Sufficient for TUI (human perception) | ⚠️ Overkill (not rendering 60 FPS) |

**Reality Check:**
- **TUI updates**: 1-10 Hz (every 100-1000ms), not 60 Hz like games
- **Anvil watch mode**: Updates on file save (seconds apart)
- **Gate results**: One-time render after checks complete
- **Human perception**: 100ms is imperceptible for terminal UI

**Performance is NOT a differentiator** for Anvil's use case.

### 4. Feature Parity

| Feature | Ink | Ratatui | Anvil Needs? |
|---------|-----|---------|--------------|
| **Layout (Flexbox)** | ✅ Yoga (CSS Flexbox) | ✅ Layout primitives | ✅ Required |
| **Text Styling** | ✅ chalk/colors | ✅ Style system | ✅ Required |
| **Input Handling** | ✅ ink-text-input | ✅ crossterm events | ✅ Required |
| **Spinners** | ✅ ink-spinner | ✅ Custom widgets | ✅ Required |
| **Progress Bars** | ✅ Community packages | ✅ Built-in widgets | ✅ Required |
| **Tables** | ⚠️ ink-table | ✅ Table widget | ⚠️ Nice-to-have |
| **Mouse Support** | ✅ Via ink-mouse | ✅ Built-in | ❌ Not planned |
| **Charts/Graphs** | ❌ Limited | ✅ ratatui-graph | ❌ Not needed |
| **Split Panes** | ⚠️ Manual layout | ✅ Layout::split() | ✅ Required |

**Verdict:** Both libraries cover Anvil's requirements. Ratatui has more built-in widgets, but Ink's ecosystem provides what's needed.

### 5. Ecosystem & Maturity

| Aspect | Ink | Ratatui |
|--------|-----|---------|
| **First Release** | 2017 (8 years mature) | 2023 (fork of tui-rs 2016) |
| **Version** | 5.x (stable) | 0.29.x (pre-1.0 but stable) |
| **Production Users** | Gatsby, Parcel, Shopify CLI | Spotify TUI, Gitui, bottom |
| **Community Packages** | 500+ ink-* packages on npm | 100+ ratatui-* crates |
| **Documentation** | Comprehensive + examples | Excellent + book |
| **Maintenance** | Active (Sindre Sorhus) | Active (community) |

Both are battle-tested and production-ready. Ink has wider adoption in Node.js ecosystem.

### 6. Build & Distribution

**Ink:**
```bash
# Single build process (existing)
pnpm build              # TypeScript → JavaScript
pnpm link:cli           # Link for local dev
# Distribution: npm package (Node.js binary)
```

**Ratatui (would require):**
```bash
# Dual build process
pnpm build              # TypeScript core
cargo build --release   # Rust TUI binary

# CI matrix for Rust cross-compilation
- linux-x64, linux-arm64
- macos-x64, macos-arm64 (M1/M2/M3)
- windows-x64

# Distribution complexity
- Ship Rust binary alongside Node.js
- OR build N-API modules for each platform
- OR require users to install Rust toolchain
```

**Deployment size:**
- **Ink**: ~30MB (Node.js + deps, same as current)
- **Ratatui**: +5-10MB Rust binary (separate) or +2-5MB N-API module per platform

### 7. Maintenance Burden

| Task | Ink | Ratatui |
|------|-----|---------|
| **Dependency Updates** | npm (automated via Renovate) | npm + cargo (two ecosystems) |
| **Security Patches** | TypeScript/Node.js ecosystem | Rust ecosystem + N-API surface |
| **Bug Fixes** | Single language (TypeScript) | Two languages (coordination) |
| **CI/CD** | Existing TypeScript pipeline | Add Rust toolchain + cross-compilation |
| **Onboarding** | Existing knowledge | Learn Rust + FFI/IPC patterns |

## Use Cases Where Ratatui Excels

Ratatui would be the better choice if Anvil had:

1. **Performance-critical TUI** (60 FPS animations, real-time dashboards)
2. **Standalone TUI application** (not integrated with Node.js CLI)
3. **Existing Rust codebase** (Anvil core in Rust)
4. **Low-level terminal control needs** (custom escape sequences, advanced backends)
5. **Rust team with no TypeScript expertise**

**Anvil has NONE of these characteristics.**

## Migration Effort Estimate

If switching to Ratatui:

| Task | Effort |
|------|--------|
| **Set up Rust project** | 1-2 days |
| **Design IPC protocol or N-API bindings** | 3-5 days |
| **Port TUI components to Rust** | 10-15 days |
| **Implement state synchronization** | 5-7 days |
| **Set up cross-platform builds** | 3-5 days |
| **Testing & debugging** | 7-10 days |
| **Documentation & team training** | 3-5 days |

**Total: 4-6 weeks** of additional work with ongoing maintenance complexity.

**For what benefit?**
- 5ms faster renders (imperceptible)
- Better widget library (Ink ecosystem is sufficient)
- More complex architecture
- Higher contributor barrier

## Hybrid Approach (Not Recommended)

Could we use Ratatui for *part* of Anvil?

**Scenario:** Use Ratatui for advanced visualizations, Ink for basic TUI.

**Why this fails:**
- **Complexity explosion**: Two TUI systems, conditional rendering
- **Inconsistent UX**: Different keybindings, styling between modes
- **Double maintenance**: Two sets of components, tests, docs
- **User confusion**: "Why does `anvil watch` look different from `anvil gate --tui`?"

**Verdict:** Worst of both worlds.

## Recommendation Matrix

| Scenario | Recommendation | Reason |
|----------|----------------|--------|
| **Anvil v1.0 (current)** | ✅ **Ink** | Matches tech stack, faster delivery, lower risk |
| **If Anvil were Rust-based** | ✅ **Ratatui** | Natural fit, no FFI complexity |
| **If TUI needed 60 FPS** | ✅ **Ratatui** | Performance matters here |
| **If team had Rust experts** | ⚠️ **Consider Ratatui** | Lower learning curve, but integration still complex |
| **Standalone TUI tool** | ✅ **Ratatui** | No Node.js integration needed |

## Decision

**Stick with Ink.** Ratatui is an excellent library, but:

1. **Technology mismatch**: Rust ↔ TypeScript integration adds significant complexity
2. **Performance irrelevant**: TUI updates are human-paced (seconds), not real-time
3. **Team velocity**: React/TypeScript is existing expertise
4. **Maintenance burden**: Two languages, two build systems, two ecosystems
5. **Risk/reward**: 4-6 weeks effort for negligible user-facing benefit

## When to Revisit This Decision

Consider Ratatui if:

1. **Anvil core rewrites to Rust** (unlikely given investment in TypeScript)
2. **TUI becomes performance bottleneck** (extremely unlikely)
3. **Need features Ink can't provide** (not on roadmap)
4. **Acquire Rust expertise on team** (not current priority)

## Alternative: Evaluate Blessed or Other TS Options

If dissatisfied with Ink, consider:

- **[Blessed](https://github.com/chjj/blessed)**: More widgets, but older architecture
- **[cli-ux](https://github.com/oclif/cli-ux)**: oclif framework integration
- **Custom ANSI renderer**: Full control, high effort

But ADR-005 already evaluated Ink vs Blessed and chose Ink for good reasons (React patterns, TypeScript, active maintenance).

## References

- [Ratatui Documentation](https://ratatui.rs/)
- [Ratatui GitHub](https://github.com/ratatui/ratatui)
- [Ink Documentation](https://github.com/vadimdemedes/ink)
- [N-API Documentation](https://nodejs.org/api/n-api.html)
- ADR-005: Ink over OpenTUI (runtime compatibility precedent)

---

**Assessment Date:** 2026-01-08
**Recommendation:** ✅ Stick with Ink
**Confidence:** High (technology mismatch is blocking)
**Reviewed By:** Claude Code
