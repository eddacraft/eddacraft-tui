# Wave 3: Policy Engine + Complex Surface Ports — Implementation Plan

> **For agentic workers:** Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build KERN Phase 3 (config loader, invariant framework, H1 invariants,
event emission) and port the complex Ink TUI surfaces (watch dashboard, tutorial
orchestrator + 4 tutorial paths).

**APS Work Items:** KERN-030, KERN-031, KERN-032, KERN-033, PORT-030, PORT-040,
PORT-041, PORT-042, PORT-043, PORT-044.

---

## Chunk 1: Track A — Kernel Phase 3 (Policy Engine + Events)

### Task 1: Architecture config loader (KERN-030)

**Files:**
- Create: `crates/anvil-kernel/src/policy/mod.rs`
- Create: `crates/anvil-kernel/src/policy/config.rs`
- Modify: `crates/anvil-kernel/src/lib.rs`

**Context:** Load layer definitions from `.anvil/architecture.yaml`. The format
is YAML with `layers` as an ordered list. Each layer has a `name` and `paths`
(glob patterns). Files matching a path belong to that layer.

- [ ] **Step 1: Implement config loader**

Types:
```rust
pub struct ArchitectureConfig { pub layers: Vec<LayerDef> }
pub struct LayerDef { pub name: String, pub paths: Vec<String>, pub allowed_imports: Vec<String> }
```

The loader reads YAML, parses into `ArchitectureConfig`. Add `layer_for_file(path)`
method that matches file paths against layer patterns.

Add serde_yaml to workspace deps: `serde_yaml = "0.9"`

Tests:
- Parse valid YAML config
- `layer_for_file` matches patterns correctly
- Files not matching any layer return None
- `allowed_imports` restricts which layers can be imported

- [ ] **Step 2: Create policy/mod.rs, update lib.rs**
- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat(kern): add architecture config loader from YAML (KERN-030)"
```

---

### Task 2: Invariant evaluation framework (KERN-031)

**Files:**
- Create: `crates/anvil-kernel/src/policy/engine.rs`
- Modify: `crates/anvil-kernel/src/policy/mod.rs`

**Context:** The policy engine accepts registered invariant functions, runs them
against GraphDeltas, and produces Violations. Violations are fingerprinted by
`(policy_id, file, symbol)` for deduplication.

Types:
```rust
pub struct Violation {
    pub policy_id: String,
    pub file: String,
    pub symbol: String,
    pub message: String,
    pub severity: Severity,
}
pub enum Severity { Critical, High, Medium, Low }
pub struct ViolationFingerprint { policy_id, file, symbol }
```

The engine:
```rust
pub struct PolicyEngine {
    invariants: Vec<Box<dyn Invariant>>,
    seen: HashSet<ViolationFingerprint>,
}
pub trait Invariant: Send {
    fn id(&self) -> &str;
    fn evaluate(&self, delta: &GraphDelta, graph: &SymbolGraph, config: &ArchitectureConfig) -> Vec<Violation>;
}
impl PolicyEngine {
    pub fn register(&mut self, inv: Box<dyn Invariant>);
    pub fn evaluate(&mut self, delta: &GraphDelta, graph: &SymbolGraph, config: &ArchitectureConfig) -> Vec<Violation>;
}
```

Tests:
- Register and evaluate a test invariant
- Deduplication by fingerprint
- Multiple invariants run in sequence
- Empty delta produces no violations

- [ ] **Step 1: Implement engine**
- [ ] **Step 2: Run tests, commit**

```bash
git commit -m "feat(kern): add invariant evaluation framework with deduplication (KERN-031)"
```

---

### Task 3: H1 invariants (KERN-032)

**Files:**
- Create: `crates/anvil-kernel/src/policy/invariants/mod.rs`
- Create: `crates/anvil-kernel/src/policy/invariants/cross_layer.rs`
- Create: `crates/anvil-kernel/src/policy/invariants/new_dependency.rs`
- Create: `crates/anvil-kernel/src/policy/invariants/public_api.rs`
- Create: `crates/anvil-kernel/src/policy/invariants/privilege_expansion.rs`
- Modify: `crates/anvil-kernel/src/policy/mod.rs`

Four invariants:

1. **CrossLayerViolation** — detects when a file in layer A imports from layer B
   where B is not in A's `allowed_imports`. Uses the graph's import edges and
   the config's layer definitions.

2. **NewDependencyIntroduction** — detects when a GraphDelta adds a symbol in a
   file that imports an external module not previously seen. Flags new external
   dependencies for review.

3. **PublicApiExpansion** — detects when a GraphDelta adds a new Public symbol.
   New exports expand the API surface and warrant review.

4. **PrivilegeExpansion** — detects when a GraphDelta adds a symbol with
   TrustLevel::Privileged (imports node:fs, child_process, etc.). New privileged
   access warrants review.

Each invariant implements the `Invariant` trait.

Tests per invariant:
- Fires when violation condition is met
- Does not fire when condition is not met
- Correct policy_id, file, symbol in produced Violation

- [ ] **Step 1: Implement all 4 invariants**
- [ ] **Step 2: Run tests, commit**

```bash
git commit -m "feat(kern): add H1 invariants — cross-layer, new dep, public API, privilege (KERN-032)"
```

---

### Task 4: Event emission (KERN-033)

**Files:**
- Create: `crates/anvil-kernel/src/protocol/mod.rs`
- Create: `crates/anvil-kernel/src/protocol/emitter.rs`
- Modify: `crates/anvil-kernel/src/lib.rs`

**Context:** The emitter wraps EngineEvent creation with auto-incrementing
sequence numbers and timestamps. Events are sent via a channel (std::sync::mpsc).

```rust
pub struct EventEmitter {
    tx: mpsc::Sender<EngineEvent>,
    seq: AtomicU64,
    engine: EngineId,
}
impl EventEmitter {
    pub fn new(tx: mpsc::Sender<EngineEvent>, engine: EngineId) -> Self;
    pub fn progress(&self, phase: &str, current: u64, total: u64);
    pub fn snapshot(&self, graph: &SymbolGraph, files_watched: u64);
    pub fn violation(&self, v: &Violation);
    pub fn error(&self, code: ErrorCode, file: Option<&str>, message: &str, recoverable: bool);
}
```

Uses the existing `EngineEvent`, `EventPayload`, `EventType` from `anvil-kernel-types`.

Tests:
- Progress event has correct payload and incrementing seq
- Snapshot event includes graph stats
- Violation event maps policy violation fields
- Error event includes error code and file
- Sequence numbers increment monotonically

- [ ] **Step 1: Implement emitter**
- [ ] **Step 2: Run tests, commit**

```bash
git commit -m "feat(kern): add event emitter with EngineEvent protocol (KERN-033)"
```

---

## Chunk 2: Track B — Complex Surface Ports

### Task 5: Port watch dashboard surface (PORT-030)

**Files:**
- Create: `crates/anvil-tui/src/surfaces/watch/mod.rs`
- Create: `crates/anvil-tui/src/surfaces/watch/render.rs`
- Modify: `crates/anvil-tui/src/surfaces/mod.rs`

**Ink reference:** `apps/anvil-cli/src/tui/commands/watch/WatchDashboard.tsx`
4-panel layout: Status, Queue, History, Stats. Panel focus with j/k switching.

This is the primary dashboarding surface. Types:

```rust
pub enum WatchStatus { Idle, Running, Passing, Failing }
pub struct QueuedChange { pub file: String, pub kind: String, pub timestamp: String }
pub struct RunHistory { pub passed: bool, pub checks_run: usize, pub checks_passed: usize, pub duration_ms: u64, pub timestamp: String }
pub struct WatchStats { pub total_runs: usize, pub pass_rate: f64, pub avg_duration_ms: u64, pub files_watched: usize }
pub struct WatchData { pub status: WatchStatus, pub queue: Vec<QueuedChange>, pub history: Vec<RunHistory>, pub stats: WatchStats }
pub enum WatchPanel { Status, Queue, History, Stats }
pub struct WatchState { pub data: WatchData, pub focused_panel: WatchPanel, pub selected_item: usize, pub should_quit: bool }
```

Navigation: Left/Right switches panels, j/k navigates within panel.
Key feature: panel borders change from single→double when focused (uses theme.border_focused/border_unfocused).

Tests:
- Panel navigation wraps correctly
- Item navigation within panels
- Status display for each WatchStatus variant

- [ ] **Step 1: Implement mod.rs with types, state, tests**
- [ ] **Step 2: Implement render.rs with 4-panel layout**
- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat(port): add watch dashboard surface port to Ratatui (PORT-030)"
```

---

### Task 6: Port tutorial orchestrator (PORT-040)

**Files:**
- Create: `crates/anvil-tui/src/surfaces/tutorial/mod.rs`
- Create: `crates/anvil-tui/src/surfaces/tutorial/render.rs`
- Modify: `crates/anvil-tui/src/surfaces/mod.rs`

The tutorial orchestrator manages path selection and step progression across
all 4 tutorial paths. Each path has a sequence of steps.

```rust
pub enum TutorialPath { Policy, Architecture, Drift, CI }
pub enum TutorialPhase { PathSelect, Running, Complete }
pub struct TutorialStep { pub title: String, pub description: String, pub instruction: String, pub completed: bool }
pub struct TutorialState {
    pub phase: TutorialPhase,
    pub paths: Vec<TutorialPath>,
    pub path_selected: usize,
    pub chosen_path: Option<TutorialPath>,
    pub steps: Vec<TutorialStep>,
    pub current_step: usize,
    pub should_quit: bool,
}
```

Navigation:
- Path select: j/k navigate, Enter selects
- Running: Enter/Space advances to next step, Esc goes back to path select
- Complete: Enter returns to path select, q quits

Tests:
- Path selection advances to Running phase
- Step progression advances current_step
- Completing all steps transitions to Complete phase
- Back from Running returns to PathSelect

- [ ] **Step 1: Implement mod.rs with types, state, tests**
- [ ] **Step 2: Implement render.rs**
- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat(port): add tutorial orchestrator surface to Ratatui (PORT-040)"
```

---

### Task 7: Tutorial path definitions (PORT-041 through PORT-044)

**Files:**
- Create: `crates/anvil-tui/src/surfaces/tutorial/paths.rs`

All 4 tutorial paths are defined as step sequences. Each path is a function
returning `Vec<TutorialStep>`.

```rust
pub fn policy_steps() -> Vec<TutorialStep> { /* 6 steps */ }
pub fn architecture_steps() -> Vec<TutorialStep> { /* 6 steps */ }
pub fn drift_steps() -> Vec<TutorialStep> { /* 5 steps */ }
pub fn ci_steps() -> Vec<TutorialStep> { /* 6 steps */ }
```

Step content comes from the Ink originals:
- Policy: Intro, CreateDir, WritePolicy, TestPolicy, SeePolicyFire, Customise
- Architecture: Intro, Template, Compile, Detect, Validate, Summary
- Drift: Intro, Capture, Compare, Inspect, Summary
- CI: Intro, Hooks, Workflow, ExitCodes, Detect, Summary

Tests:
- Each path returns the expected number of steps
- All steps have non-empty title, description, instruction
- Step titles match expected sequence

- [ ] **Step 1: Implement paths.rs**
- [ ] **Step 2: Wire into tutorial mod.rs (chosen_path loads steps)**
- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat(port): add tutorial path definitions — policy, architecture, drift, CI (PORT-041 through PORT-044)"
```

---

### Task 8: Final verification + APS status update

- [ ] **Step 1: cargo test --all, cargo clippy, cargo fmt**
- [ ] **Step 2: Update APS statuses for KERN Phase 3 + PORT Phase 4**
- [ ] **Step 3: Commit and push**
