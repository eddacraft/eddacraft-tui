# Wave 4: Integration + Validation — Implementation Plan

> **For agentic workers:** Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the kernel into usable modes (embedded check + foreground watch),
build the dual-run harness, add benchmarks and cross-compilation, complete RENG
check parity, and wire RATS TUI surfaces to live kernel events.

**APS Work Items:** KERN-040, KERN-041, KERN-042, KERN-043, KERN-044, RENG-004,
RENG-006, RATS-002, RATS-003, RATS-005, RATS-006, RATS-007.

---

## Chunk 1: Track A — Kernel Phase 4 (Integration)

### Task 1: Embedded mode — library API for one-shot checks (KERN-040)

**Files:**
- Create: `crates/anvil-kernel/src/embedded.rs`
- Modify: `crates/anvil-kernel/src/lib.rs`

**Context:** The embedded mode is the library API that runs the full pipeline
once: scan files → parse → build graph → annotate trust → evaluate invariants
→ emit events. Returns a list of violations. No watcher, no loop.

```rust
pub struct EmbeddedConfig {
    pub root: PathBuf,
    pub architecture_config: Option<PathBuf>,
    pub filter: Option<FileFilter>,
}

pub struct EmbeddedResult {
    pub violations: Vec<Violation>,
    pub stats: GraphStats,
    pub events: Vec<EngineEvent>,
    pub duration: Duration,
}

pub fn run_embedded(config: &EmbeddedConfig) -> Result<EmbeddedResult, KernelError>;
```

The function:
1. Walks the root directory, filtering with FileFilter
2. Parses each file with Parser
3. Extracts symbols, builds SymbolGraph
4. Builds DependencyGraph from imports
5. Annotates trust levels
6. Loads architecture config (if provided)
7. Runs PolicyEngine with registered H1 invariants
8. Collects events via EventEmitter
9. Returns EmbeddedResult

Tests:
- Runs on a temp directory with sample TS files
- Returns violations for cross-layer imports
- Returns correct graph stats
- Empty directory returns no violations
- Non-existent architecture config returns error

- [ ] **Step 1: Implement embedded.rs**
- [ ] **Step 2: Run tests, commit**

```bash
git commit -m "feat(kern): add embedded mode for one-shot checks (KERN-040)"
```

---

### Task 2: Foreground watch mode (KERN-041)

**Files:**
- Create: `crates/anvil-kernel/src/watch.rs`
- Modify: `crates/anvil-kernel/src/lib.rs`

**Context:** Watch mode is a long-lived loop: watcher → parser → incremental
graph update → policy evaluation → event emission. Runs until stopped.

```rust
pub struct WatchConfig {
    pub root: PathBuf,
    pub architecture_config: Option<PathBuf>,
    pub watcher: WatcherConfig,
}

pub fn run_watch(
    config: WatchConfig,
    event_tx: mpsc::Sender<EngineEvent>,
) -> Result<WatchHandle, KernelError>;

pub struct WatchHandle { /* join handle + stop signal */ }
impl WatchHandle {
    pub fn stop(self) -> Result<(), KernelError>;
}
```

The watch loop:
1. Does an initial full scan (like embedded mode)
2. Starts the file watcher
3. On each ChangeBatch:
   - For created/modified files: reparse, update graph, evaluate policy
   - For removed files: remove from graph
   - Emit events for each step

Tests:
- Starts and stops cleanly
- Detects file creation and emits events
- Incremental update produces correct delta

- [ ] **Step 1: Implement watch.rs**
- [ ] **Step 2: Run tests, commit**

```bash
git commit -m "feat(kern): add foreground watch mode with event streaming (KERN-041)"
```

---

### Task 3: Dual-run harness (KERN-042)

**Files:**
- Create: `crates/anvil-kernel/tests/dual_run.rs`

**Context:** A test harness that validates the Rust kernel produces equivalent
results to what the legacy TS engine would produce. For now, this is a
framework with placeholder comparisons — actual TS engine integration comes
later.

```rust
pub struct DualRunResult {
    pub rust_violations: Vec<Violation>,
    pub matches: bool,
    pub discrepancies: Vec<String>,
}
```

Tests:
- Harness runs embedded mode and captures results
- Results can be serialised for comparison
- Framework for adding TS engine results later

- [ ] **Step 1: Implement dual_run.rs**
- [ ] **Step 2: Run tests, commit**

```bash
git commit -m "feat(kern): add dual-run harness framework for engine comparison (KERN-042)"
```

---

### Task 4: Performance benchmarks (KERN-043)

**Files:**
- Create: `crates/anvil-kernel/benches/kernel.rs`
- Modify: `crates/anvil-kernel/Cargo.toml` (add bench target)

**Context:** criterion.rs benchmarks for the kernel's critical paths:
- Cold graph build (parse + extract + graph for N files)
- Incremental update (single file change)
- Policy evaluation (invariants against a delta)
- Event emission overhead

```toml
[[bench]]
name = "kernel"
harness = false
```

- [ ] **Step 1: Implement benchmarks**
- [ ] **Step 2: Run benchmarks, commit**

```bash
git commit -m "perf(kern): add criterion benchmarks for kernel critical paths (KERN-043)"
```

---

### Task 5: Cross-compilation CI (KERN-044)

**Files:**
- Modify: `.github/workflows/rust.yml`

Add a matrix build for:
- linux x86_64
- linux aarch64 (cross)
- macos x86_64
- macos aarch64

Use `cross` for cross-compilation where needed.

- [ ] **Step 1: Update CI workflow**
- [ ] **Step 2: Commit**

```bash
git commit -m "ci(kern): add cross-compilation matrix for Linux and macOS targets (KERN-044)"
```

---

## Chunk 2: Track B — RENG + RATS Integration

### Task 6: Architecture check parity validation (RENG-004)

**Files:**
- Create: `crates/anvil-kernel/tests/architecture_parity.rs`

**Context:** Validate that the kernel's H1 invariants (KERN-032) produce
equivalent results to the current JS architecture check. This is a test file
that runs the kernel on fixture repos with known violations and asserts the
expected violations are detected.

Tests:
- Cross-layer violation detected in fixture repo
- Public API expansion detected
- Privilege expansion detected
- No false positives on clean fixture

- [ ] **Step 1: Implement parity tests**
- [ ] **Step 2: Run tests, commit**

```bash
git commit -m "test(reng): add architecture check parity validation (RENG-004)"
```

---

### Task 7: Feature flag + dual-run for ported checks (RENG-006)

**Files:**
- Create: `crates/anvil-kernel/src/engine_mode.rs`
- Modify: `crates/anvil-kernel/src/lib.rs`

**Context:** Add an `EngineMode` enum (Rust, Legacy, Dual) that controls which
engine runs. In Dual mode, both run and results are diffed.

```rust
pub enum EngineMode { Rust, Legacy, Dual }
```

This is the flag that the CLI will expose as `--engine rust/legacy/dual`.
For now, only `Rust` mode is functional. `Legacy` and `Dual` are stubs.

Tests:
- Rust mode runs kernel and returns results
- Legacy mode returns "not implemented" error
- Dual mode returns "not implemented" error

- [ ] **Step 1: Implement engine_mode.rs**
- [ ] **Step 2: Run tests, commit**

```bash
git commit -m "feat(reng): add engine mode flag for Rust/Legacy/Dual selection (RENG-006)"
```

---

### Task 8: Wire watch dashboard to kernel events (RATS-002)

**Files:**
- Create: `crates/anvil-tui/src/surfaces/watch/event_adapter.rs`
- Modify: `crates/anvil-tui/src/surfaces/watch/mod.rs`

**Context:** An adapter that converts `EngineEvent` stream into `WatchData`
updates for the watch dashboard. This bridges KERN events → TUI state.

```rust
pub struct WatchEventAdapter { /* internal state */ }
impl WatchEventAdapter {
    pub fn new() -> Self;
    pub fn handle_event(&mut self, event: &EngineEvent, data: &mut WatchData);
}
```

Maps:
- Progress events → status updates
- Snapshot events → stats updates
- Violation events → queue/history updates
- Error events → status updates

Tests:
- Progress event updates status to Running
- Snapshot event updates stats
- Violation event adds to history
- Error event updates status

- [ ] **Step 1: Implement event_adapter.rs**
- [ ] **Step 2: Run tests, commit**

```bash
git commit -m "feat(rats): add event adapter wiring watch dashboard to kernel events (RATS-002)"
```

---

### Task 9: Wire gate viewer to kernel events (RATS-003)

**Files:**
- Create: `crates/anvil-tui/src/surfaces/gate/event_adapter.rs`
- Modify: `crates/anvil-tui/src/surfaces/gate/mod.rs`

**Context:** Similar to RATS-002 but for the gate explorer. Converts
`EmbeddedResult` into `GateResult` for the gate explorer surface.

```rust
pub fn embedded_result_to_gate_result(result: &EmbeddedResult) -> GateResult;
```

Tests:
- Violations map to failed GateChecks
- Empty violations produce passing result
- Stats are correctly mapped

- [ ] **Step 1: Implement event_adapter.rs**
- [ ] **Step 2: Run tests, commit**

```bash
git commit -m "feat(rats): add event adapter wiring gate explorer to kernel results (RATS-003)"
```

---

### Task 10: Ink-to-Ratatui migration path (RATS-005)

**Files:**
- Create: `crates/anvil-tui/src/migration.rs`
- Modify: `crates/anvil-tui/src/lib.rs`

**Context:** Define the `TuiBackend` enum and selection logic for choosing
between Ink and Ratatui rendering.

```rust
pub enum TuiBackend { Ink, Ratatui }
pub fn select_backend(preference: Option<TuiBackend>) -> TuiBackend;
```

Default is `Ink` until parity is validated. Users opt in with `--tui=ratatui`.

Tests:
- Default returns Ink
- Explicit preference is respected

- [ ] **Step 1: Implement migration.rs**
- [ ] **Step 2: Run tests, commit**

```bash
git commit -m "feat(rats): add TUI backend selection for Ink-to-Ratatui migration (RATS-005)"
```

---

### Task 11: Terminal compatibility notes (RATS-006)

**Files:**
- Create: `crates/anvil-tui/src/compat.rs`
- Modify: `crates/anvil-tui/src/lib.rs`

**Context:** Terminal detection and minimum size validation.

```rust
pub struct TerminalInfo { pub cols: u16, pub rows: u16, pub term: String }
pub fn detect_terminal() -> TerminalInfo;
pub fn validate_minimum_size(info: &TerminalInfo) -> Result<(), String>;
```

Minimum: 80x24. Reports terminal name from $TERM env var.

Tests:
- Validates 80x24 passes
- Validates 79x24 fails
- Validates 80x23 fails

- [ ] **Step 1: Implement compat.rs**
- [ ] **Step 2: Run tests, commit**

```bash
git commit -m "feat(rats): add terminal compatibility detection and validation (RATS-006)"
```

---

### Task 12: `anvil watch` TUI integration entry point (RATS-007)

**Files:**
- Create: `crates/anvil-tui/src/app.rs`
- Modify: `crates/anvil-tui/src/lib.rs`

**Context:** The top-level TUI app that wires everything together: starts the
kernel watch mode, creates the event adapter, runs the Ratatui event loop.

```rust
pub struct TuiApp { /* kernel handle, event adapter, watch state, terminal */ }
impl TuiApp {
    pub fn new(config: TuiAppConfig) -> Result<Self, TuiError>;
    pub fn run(&mut self) -> Result<(), TuiError>;
}
```

This is the entry point that `anvil watch --tui=ratatui` will call.
For now, the implementation accepts mock event sources for testing.

Tests:
- TuiApp creates successfully with mock config
- State initialises correctly

- [ ] **Step 1: Implement app.rs**
- [ ] **Step 2: Run tests, commit**

```bash
git commit -m "feat(rats): add TUI app entry point for anvil watch integration (RATS-007)"
```

---

### Task 13: Final verification + APS status update

- [ ] **Step 1: cargo test --all, cargo clippy, cargo fmt**
- [ ] **Step 2: Update APS statuses for all Wave 4 items**
- [ ] **Step 3: Commit and push**
