# Rust CLI Adoption Plan

## Overview

Migrate the Anvil CLI from TypeScript/Node.js to Rust for improved performance,
reduced startup time, and smaller distribution size. This is a **gradual
migration** that maintains feature parity and keeps TypeScript core as the
business logic layer initially.

**Goals**:

- Sub-100ms cold start (currently ~800ms with Node.js bootstrap)
- Single binary distribution (no Node.js runtime dependency for basic commands)
- Keep TypeScript core for complex business logic initially
- Maintain full backward compatibility

---

## Architecture Decision

### Option A: Full Rewrite (NOT recommended)

Rewrite everything in Rust including core validation, adapters, gate checks.

**Cons**: 6+ months, stalls feature development, high risk

### Option B: Rust CLI Wrapper + IPC (RECOMMENDED)

Rust handles CLI parsing and routing, delegates to TypeScript for complex logic.

```
┌─────────────────────────────────────────────────────────┐
│                    Rust CLI Binary                       │
│  ┌─────────┐  ┌──────────┐  ┌─────────────────────────┐ │
│  │  clap   │→ │ Command  │→ │ Fast Path (Pure Rust)   │ │
│  │ parser  │  │ Router   │  │ - validate (schema)     │ │
│  └─────────┘  └──────────┘  │ - hash (SHA-256)        │ │
│                              │ - plan create           │ │
│                              │ - config read           │ │
│                              └─────────────────────────┘ │
│                                         │                │
│                              ┌──────────▼──────────────┐ │
│                              │ Slow Path (Node.js IPC) │ │
│                              │ - gate (spawn checks)   │ │
│                              │ - export (adapters)     │ │
│                              │ - policy (OPA)          │ │
│                              └─────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

**Pros**: Ship incrementally, 2-3 months for Phase 1, maintain feature velocity

---

## Implementation Phases

### Phase 1: Rust CLI Shell (Foundation)

**Goal**: Rust binary that parses commands and delegates to Node.js subprocess

**Duration**: 2-3 weeks

**Files to create**:

```
cli-rust/
├── Cargo.toml
├── src/
│   ├── main.rs                 # Entry point
│   ├── cli.rs                  # Clap command definitions
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── validate.rs         # Delegate to Node
│   │   ├── gate.rs             # Delegate to Node
│   │   ├── export.rs           # Delegate to Node
│   │   ├── init.rs             # Delegate to Node
│   │   ├── plan.rs             # Delegate to Node
│   │   ├── policy.rs           # Delegate to Node
│   │   ├── hooks.rs            # Delegate to Node
│   │   └── config.rs           # Delegate to Node
│   ├── node_bridge.rs          # Spawn Node.js subprocess
│   └── output.rs               # Colored terminal output
└── build.rs                    # Build script
```

**Clap command structure**:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "anvil")]
#[command(about = "AI-native plan validation and quality gates")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Validate {
        #[arg(required = true)]
        plan: String,
        #[arg(long)]
        verbose: bool,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        native: bool,
        #[arg(long, name = "validate-hash")]
        validate_hash: bool,
    },
    Gate {
        #[arg(required = true)]
        plan: String,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        skip_checks: Option<String>,
        #[arg(long)]
        only_checks: Option<String>,
        #[arg(long)]
        fail_fast: bool,
        #[arg(long)]
        inject: bool,
    },
    Export { /* ... */ },
    Init { /* ... */ },
    Plan { /* ... */ },
    Policy { /* ... */ },
    Hooks { /* ... */ },
    Config { /* ... */ },
}
```

**Node.js bridge** (spawn and communicate):

```rust
use std::process::{Command, Stdio};
use serde_json::{json, Value};

pub fn call_node(command: &str, args: Value) -> Result<Value, Error> {
    let child = Command::new("node")
        .arg("--experimental-json-modules")
        .arg(get_node_entry_path())  // bundled JS or installed package
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let request = json!({
        "command": command,
        "args": args
    });

    // Write request to stdin
    serde_json::to_writer(child.stdin.take().unwrap(), &request)?;

    // Read response from stdout
    let response: Value = serde_json::from_reader(child.stdout.take().unwrap())?;
    Ok(response)
}
```

**Deliverables**:

- [ ] Rust CLI that mirrors all TypeScript CLI commands
- [ ] Node.js bridge for delegation
- [ ] Same output format as TypeScript CLI
- [ ] Cross-platform build (Linux, macOS, Windows)

---

### Phase 2: Fast Path Commands (Pure Rust)

**Goal**: Implement performance-critical commands in pure Rust

**Duration**: 3-4 weeks

**Commands to migrate**:

1. **`anvil validate`** (schema validation only, no adapters)
   - JSON Schema validation using `jsonschema` crate
   - SHA-256 hash verification using `sha2` crate
   - File reading with `std::fs`

2. **`anvil plan create`** (plan generation)
   - Plan ID generation using `rand` crate
   - Hash generation using `sha2` crate
   - JSON canonicalization

3. **`anvil config`** (config management)
   - Read/write `.anvilrc` and `.anvil/config.json`
   - JSON parsing with `serde_json`

**Files to add**:

```
cli-rust/src/
├── schema/
│   ├── mod.rs
│   ├── aps.rs              # APS plan struct definitions
│   └── validation.rs       # JSON Schema validation
├── crypto/
│   ├── mod.rs
│   ├── hash.rs             # SHA-256, canonicalization
│   └── plan_id.rs          # Plan ID generation
├── config/
│   ├── mod.rs
│   └── gate_config.rs      # GateConfig management
└── commands/
    ├── validate_native.rs  # Pure Rust validation
    ├── plan_native.rs      # Pure Rust plan creation
    └── config_native.rs    # Pure Rust config management
```

**Rust APS schema** (from Zod):

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct APSPlan {
    pub id: String,                     // aps-[8 hex chars]
    pub hash: String,                   // SHA-256 (64 hex chars)
    pub intent: String,                 // 10-500 chars
    pub schema_version: String,         // "0.1.0"
    pub proposed_changes: Vec<Change>,
    pub provenance: Provenance,
    pub validations: Validation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Vec<Evidence>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval: Option<Approval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executions: Option<Vec<ExecutionResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    FileCreate,
    FileUpdate,
    FileDelete,
    ConfigUpdate,
    DependencyAdd,
    DependencyRemove,
    DependencyUpdate,
    ScriptExecute,
}

// ... other structs
```

**Deliverables**:

- [ ] `anvil validate` in pure Rust (10x faster)
- [ ] `anvil plan create` in pure Rust
- [ ] `anvil config` in pure Rust
- [ ] Shared JSON Schema between Rust and TypeScript

---

### Phase 3: Node.js Optional Mode

**Goal**: CLI works without Node.js for basic commands

**Duration**: 2 weeks

**Strategy**:

1. Detect if Node.js is available
2. For "fast path" commands: Run pure Rust
3. For "slow path" commands: Require Node.js or show helpful error

```rust
fn check_node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_command(cmd: &Commands) -> Result<(), Error> {
    match cmd {
        // Fast path - pure Rust
        Commands::Validate { native: true, .. } => validate_native(cmd),
        Commands::Plan { .. } => plan_native(cmd),
        Commands::Config { .. } => config_native(cmd),

        // Slow path - needs Node.js
        _ => {
            if !check_node_available() {
                eprintln!("This command requires Node.js. Install Node.js or use --native flag.");
                std::process::exit(1);
            }
            call_node_bridge(cmd)
        }
    }
}
```

**Deliverables**:

- [ ] Graceful degradation when Node.js unavailable
- [ ] Clear error messages for Node.js-dependent commands
- [ ] `--native` flag for forcing pure Rust path

---

### Phase 4: Embedded Node.js Runtime (Optional)

**Goal**: Ship self-contained binary with embedded JS runtime

**Duration**: 4-6 weeks (optional, based on demand)

**Options**:

1. **Deno Core** (`deno_core` crate)
   - Embed V8 + TypeScript support
   - Larger binary (~20MB) but no runtime dependency
   - Full JS/TS execution

2. **QuickJS** (`quickjs-rs` crate)
   - Lightweight (~1MB addition)
   - ES2020 support
   - No TypeScript (needs transpilation)

3. **Bun Embedding** (future)
   - Not yet stable for embedding

**Recommendation**: Skip this phase initially. The Node.js bridge is sufficient
for most use cases, and users who need the full CLI will have Node.js installed.

---

### Phase 5: Native NAPI Addon (Performance Hybrid)

**Goal**: Use Rust as Node.js addon for maximum performance in Node.js
environments

**Duration**: 3-4 weeks

**Use case**: VS Code extension, MCP server, and other Node.js integrations that
want Rust speed.

**Files**:

```
packages/anvil-native/
├── Cargo.toml
├── src/
│   └── lib.rs              # NAPI-RS bindings
├── index.js                # JS wrapper
├── index.d.ts              # TypeScript definitions
└── package.json            # Platform-specific packages
```

**NAPI-RS bindings**:

```rust
use napi_derive::napi;
use napi::Result;

#[napi]
pub fn validate_plan(json: String) -> Result<ValidationResult> {
    let plan: APSPlan = serde_json::from_str(&json)?;
    // ... validation logic
    Ok(result)
}

#[napi]
pub fn generate_hash(json: String) -> Result<String> {
    let canonical = canonicalize_json(&json)?;
    Ok(sha256(&canonical))
}

#[napi]
pub fn generate_plan_id() -> String {
    format!("aps-{:08x}", rand::random::<u32>())
}
```

**Distribution** (platform-specific npm packages):

```
@eddacraft/anvil-native                    # Base package
@eddacraft/anvil-native-linux-x64-gnu      # Linux x64
@eddacraft/anvil-native-linux-x64-musl     # Linux musl
@eddacraft/anvil-native-darwin-x64         # macOS Intel
@eddacraft/anvil-native-darwin-arm64       # macOS Apple Silicon
@eddacraft/anvil-native-win32-x64-msvc     # Windows x64
```

**Deliverables**:

- [ ] NAPI-RS addon with core functions
- [ ] Platform-specific npm packages
- [ ] TypeScript definitions
- [ ] Integration with existing `@eddacraft/anvil-core`

---

## Crate Dependencies

```toml
[dependencies]
# CLI
clap = { version = "4.4", features = ["derive"] }
colored = "2.1"
indicatif = "0.17"          # Progress bars

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"

# Crypto
sha2 = "0.10"
rand = "0.8"

# Validation
jsonschema = "0.18"         # JSON Schema validation

# Async (for IPC)
tokio = { version = "1.35", features = ["process", "io-util"] }

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# File watching (for watch mode)
notify = "6.1"

# NAPI (Phase 5 only)
napi = { version = "2.14", features = ["tokio_rt"] }
napi-derive = "2.14"
```

---

## Build & Distribution

### Cross-Platform Build Matrix

```yaml
# .github/workflows/release.yml
jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: anvil-linux-x64
          - os: ubuntu-latest
            target: x86_64-unknown-linux-musl
            artifact: anvil-linux-x64-musl
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact: anvil-darwin-x64
          - os: macos-latest
            target: aarch64-apple-darwin
            artifact: anvil-darwin-arm64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: anvil-win32-x64.exe

    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo build --release --target ${{ matrix.target }}
      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: target/${{ matrix.target }}/release/anvil*
```

### npm Distribution

```json
// cli-rust/npm/package.json
{
  "name": "anvil-cli",
  "version": "0.3.0",
  "bin": {
    "anvil": "./bin/anvil"
  },
  "scripts": {
    "postinstall": "node scripts/install.js"
  },
  "optionalDependencies": {
    "@eddacraft/anvil-cli-linux-x64": "0.3.0",
    "@eddacraft/anvil-cli-darwin-x64": "0.3.0",
    "@eddacraft/anvil-cli-darwin-arm64": "0.3.0",
    "@eddacraft/anvil-cli-win32-x64": "0.3.0"
  }
}
```

---

## Migration Path

### Backward Compatibility

1. **Same command interface**: All flags and arguments identical
2. **Same output format**: JSON output matches TypeScript version
3. **Same exit codes**: 0 for success, 1 for validation failure, 2 for error
4. **Environment variables**: Same `ANVIL_*` variables supported

### Feature Flags

During migration, support both implementations:

```bash
# Use TypeScript implementation (default initially)
anvil validate plan.json

# Use Rust implementation
anvil validate plan.json --native
ANVIL_NATIVE=1 anvil validate plan.json

# Force Node.js even for native-capable commands
ANVIL_FORCE_NODE=1 anvil validate plan.json
```

### Rollout Strategy

1. **Alpha**: Rust CLI available as `anvil-rs` separate binary
2. **Beta**: `anvil --native` flag enables Rust for supported commands
3. **GA**: Rust becomes default, `--legacy` flag for TypeScript

---

## Performance Targets

| Command             | TypeScript | Rust Target | Speedup               |
| ------------------- | ---------- | ----------- | --------------------- |
| Cold start          | ~800ms     | <100ms      | 8x                    |
| `validate` (cached) | ~200ms     | <20ms       | 10x                   |
| `plan create`       | ~150ms     | <10ms       | 15x                   |
| `config` read       | ~100ms     | <5ms        | 20x                   |
| `gate` (full)       | ~5-30s     | ~5-30s      | 1x (subprocess bound) |

---

## Risks & Mitigations

| Risk                       | Mitigation                                             |
| -------------------------- | ------------------------------------------------------ |
| Team lacks Rust expertise  | Start with Phase 1 (wrapper only), learn incrementally |
| IPC overhead               | Batch commands, keep Node.js process warm              |
| Type drift between Rust/TS | Generate types from shared JSON Schema                 |
| Platform-specific bugs     | Comprehensive CI matrix, platform-specific tests       |
| Binary size                | Strip symbols, LTO, use `cargo-bloat` to audit         |

---

## Success Metrics

- [ ] Cold start < 100ms
- [ ] `anvil validate` < 20ms (cached)
- [ ] Binary size < 10MB
- [ ] All existing tests pass
- [ ] No breaking changes to CLI interface

---

## Next Steps

1. **Approve plan** - Confirm scope and phasing
2. **Phase 1**: Create `cli-rust/` with clap structure and Node.js bridge
3. **Validate**: Ensure parity with TypeScript CLI
4. **Phase 2**: Migrate validate/plan/config to pure Rust
5. **Review**: Checkpoint before Phase 3+

---

_Last updated: December 2025_
