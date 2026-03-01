# Project: High-Fidelity Diagram Rendering for Ratatui

**Research & Exploration**

## System Context

This rendering engine is the visualization layer for Anvil's semantic graphs.
The [Rust Kernel](../architecture/rust-kernel-spec.md) maintains live symbol,
dependency, and trust graphs; this engine renders them as interactive TUI
diagrams. In the
[Architecture Evolution](../architecture/anvil-architecture-evolution.md)
roadmap, this becomes the H2 "delight multiplier" — a Ratatui widget library
that consumes graph snapshots and deltas from the kernel's streaming output.

Diagrams are downstream of the kernel. They are renderers, not analysers.

---

## 1. Objective

Design and implement a Rust-native diagram rendering engine optimised for
terminal user interfaces (TUIs), integrated with Ratatui.

The outcome must:

- Produce visually impressive, high-clarity diagrams in a terminal.
- Avoid dependency on Mermaid or external JS.
- Be fully Rust-native.
- Feel intentional, not ASCII hacky.
- Be extensible to multiple diagram types over time.

The emotional bar is high:

> “How on earth did they do that in a terminal?”

---

## Phase 0 – Research & Ecosystem Assessment (Mandatory First Step)

Before building anything, we must deeply understand Ratatui and its ecosystem.

## 0.1 Ratatui Core Architecture

Research and document:

- `ratatui::Buffer`
  - How cells are stored and updated.
  - Performance characteristics.
  - Direct cell mutation vs higher-level widgets.

- `Widget` vs `StatefulWidget`
  - When to implement each.
  - How state is preserved between renders.

- Layout system (`ratatui::layout`)
  - `Rect`
  - `Layout`
  - Constraints

- Styling system
  - `Style`
  - `Modifier`
  - Colour support (true colour, 256, basic)

- Rendering lifecycle
  - Frame render flow
  - Terminal backends (crossterm, termion)
  - Double buffering behaviour

Deliverable:

- Short technical memo summarising:
  - Best way to render custom graphics.
  - Whether we render via:
    - A custom `Widget` writing to `Buffer`
    - A `Paragraph`
    - Or direct `Frame` drawing.

Initial hypothesis: Direct `Buffer` rendering inside a custom `Widget` is
required for precision control and performance.

---

## 0.2 Ratatui Ecosystem Survey

Research crates that might help:

### Graphics / Canvas

- `ratatui::widgets::Canvas`
- Third-party canvas-like crates
- Braille-based rendering utilities

Assess:

- Suitability for box drawing and orthogonal routing.
- Whether Canvas abstraction helps or limits us.

---

### Text Measurement

Terminal width is not trivial.

Research:

- `unicode-width`
- `unicode-segmentation`
- Handling of:
  - Multi-byte characters
  - Wide glyphs (CJK)
  - Combining characters

Deliverable:

- Strategy for accurate text width measurement.

---

### TUI Utilities

Survey:

- `tui-textarea`
- `tui-tree-widget`
- `ratatui-explorer`
- Any diagram-like or graph-like crates
- Any Rust ASCII graph rendering libraries

Goal:

- Avoid reinventing wheels.
- Learn from how others handle scrolling, selection, focus.

---

### Graph & Layout Crates

Research existing Rust crates for:

- Directed graph representation
  - `petgraph`

- Layout algorithms
  - Sugiyama
  - Force-directed
  - Layered DAG layout

- Pathfinding
  - `pathfinding`
  - A\*

- Orthogonal routing libraries (if any exist)

Deliverable:

- Recommendation: reuse vs build.

---

## 0.3 Terminal Capability Matrix

Research terminal capabilities:

- Box drawing consistency across:
  - iTerm
  - Windows Terminal
  - Kitty
  - Alacritty

- True colour support
- Font considerations
- Known rendering glitches

Define:

- Minimum supported terminal capability.

---

## Phase 1 – Core Requirements

## 1.1 Functional Requirements

### Diagram Types (MVP)

- Directed flowchart (rectangular nodes)
- Edge labels
- Top-down layout
- Optional left-right layout

Later:

- Decision diamonds
- Sequence diagrams
- Swimlanes
- Collapsible groups

---

### Node Features

Nodes must support:

- Multi-line labels
- Padding
- Configurable border style:
  - Sharp corners
  - Rounded corners

- Theming
  - Foreground
  - Background
  - Border style

- State:
  - Selected
  - Highlighted
  - Disabled

---

### Edge Features

Edges must support:

- Directional arrowheads
- Orthogonal routing
- Label placement without collision
- Bend minimisation
- Edge crossing minimisation (best effort)

---

### Layout Requirements

- Layered layout (DAG first)
- Deterministic output
- Stable ordering
- Adjustable spacing
- No overlapping nodes
- No edge/node collisions

Routing must:

- Avoid node bounding boxes
- Prefer straight lines
- Penalise bends
- Remain performant for medium graphs (~200 nodes)

---

### Rendering Requirements

Rendering must:

- Write directly to Ratatui `Buffer`
- Respect `Rect` clipping
- Support scrolling if overflow
- Be performant (60fps possible on small diagrams)

---

## Phase 2 – Architecture Design

## 2.1 High-Level Modules

```
diagram-core/
  graph.rs
  layout/
    layered.rs
    ranking.rs
    ordering.rs
  routing/
    manhattan.rs
    astar.rs
  render/
    grid.rs
    glyphs.rs
    theme.rs

ratatui-diagrams/
  widget.rs
  state.rs
  interaction.rs
```

---

## 2.2 Core Data Structures

### Graph Model

```rust
struct NodeId(String);

struct Node {
    id: NodeId,
    label: String,
    style: NodeStyle,
}

struct Edge {
    from: NodeId,
    to: NodeId,
    label: Option<String>,
}
```

Use `petgraph` internally for traversal.

---

### Layout Model

```rust
struct PositionedNode {
    id: NodeId,
    rect: Rect,
}

struct RoutedEdge {
    from: NodeId,
    to: NodeId,
    path: Vec<Point>,
}
```

---

### Rendering Model

Grid-based internal representation:

```rust
struct Cell {
    ch: char,
    style: Style,
}
```

Then map grid → Ratatui `Buffer`.

---

## Phase 3 – Interaction & Delight

Optional but highly recommended:

- Keyboard navigation between nodes
- Highlight current node
- Animated edge tracing
- Smooth pan (shift viewport)
- Zoom by reflow (coarser spacing)

This is where the “wow” factor multiplies.

---

## Non-Goals (For Now)

- Perfect Mermaid compatibility
- SVG export
- ELK-grade layout parity
- Arbitrary graph layout types
- Pixel-perfect browser matching

---

## Success Criteria

1. Flowchart renders cleanly and readably.
2. No visible rendering glitches in major terminals.
3. Diagram of 50 nodes renders instantly.
4. Screenshot looks deliberate, not ASCII-era nostalgic.
5. Codebase modular and extensible.

---

## Risk Assessment

## High Risk

- Routing complexity explosion.
- Text width inconsistencies across terminals.

## Medium Risk

- Performance degradation with dense graphs.
- Edge crossing aesthetics.

## Low Risk

- Node rendering.
- Basic layered layout.
