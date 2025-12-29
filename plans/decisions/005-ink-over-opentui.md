# ADR-005: Ink over OpenTUI for TUI Implementation

## Status

Accepted

## Date

2025-12-29

## Context

Anvil v1.0 requires an onboarding TUI for smooth first-run experience:

- `anvil init` wizard with visual guidance
- `anvil status` dashboard
- `anvil doctor` diagnostics
- First-run welcome experience

Two React-based terminal UI libraries were considered:

### OpenTUI (`@opentui/core`, `@opentui/react`)

- Created by SST team (creators of SST framework)
- Zig-native rendering engine for high performance
- Modern features: syntax highlighting, animations, mouse support
- Active development (v0.1.x as of Dec 2025)

### Ink (`ink`)

- Created by Sindre Sorhus / Vadim Demedes
- Pure JavaScript/TypeScript implementation
- Production-ready (v5.x, used since 2017)
- Large ecosystem: `ink-select-input`, `ink-text-input`, `ink-spinner`

## Decision

**Use Ink for TUI implementation.**

## Rationale

### 1. Runtime Compatibility (Blocking)

OpenTUI requires Bun runtime. Analysis of `@opentui/core` npm package:

```
dependencies:
  bun-ffi-structs: 0.1.2
```

The `bun-ffi-structs` package is Bun's FFI (Foreign Function Interface) for
calling native Zig code. This is Bun-specific, not N-API compatible.

**Anvil's constraint:** "Must run on Node.js 20+" (from index.aps.md)

Adding Bun as a runtime requirement would:

- Complicate installation for users
- Require CI/CD changes
- Create deployment friction
- Diverge from existing Node.js toolchain

### 2. Maturity

| Aspect           | OpenTUI          | Ink           |
| ---------------- | ---------------- | ------------- |
| Version          | 0.1.x (unstable) | 5.x (stable)  |
| First release    | 2025             | 2017          |
| Production usage | Limited          | Widespread    |
| API stability    | Changes daily\*  | Stable        |
| Documentation    | Minimal          | Comprehensive |

\*Per OpenTUI maintainer: "the native interface is far from stable and changes
almost daily"

### 3. N-API Binding Effort

Adding Node.js support to OpenTUI was evaluated. Per OpenTUI maintainer, it
would require:

- Wrapping Zig C ABI to build `.node` modules
- Exposing RenderLib-compatible interface
- Conditional loading for non-Bun runtimes
- Extending build.zig for static versions
- Building `.node` modules for all platforms
- Maintaining sync with rapidly-changing native interface
- Handling WebGPU bindings separately

**Estimated effort:** 2-3 weeks initial + ongoing maintenance

### 4. Feature Requirements

For v1.0 onboarding TUI, we need:

| Feature          | Ink | OpenTUI |
| ---------------- | --- | ------- |
| Box layouts      | ✅  | ✅      |
| Text styling     | ✅  | ✅      |
| Spinners         | ✅  | ✅      |
| Select inputs    | ✅  | ✅      |
| Text inputs      | ✅  | ✅      |
| Progress bars    | ✅  | ✅      |
| Flexbox (Yoga)   | ✅  | ✅      |
| Syntax highlight | ❌  | ✅      |
| Animations       | ❌  | ✅      |
| Native perf      | ❌  | ✅      |

Ink provides everything needed for onboarding. OpenTUI's advanced features
(syntax highlighting, animations, native performance) are nice-to-have for
operational TUI (v1.2), not required for v1.0 onboarding.

## Consequences

### Positive

- Zero additional runtime requirements
- Battle-tested, production-ready library
- Large ecosystem of community components
- Familiar React patterns for contributors
- Simpler CI/CD (no Bun installation)

### Negative

- No native Zig performance (acceptable for TUI use case)
- No built-in syntax highlighting (not needed for onboarding)
- No advanced animations (not needed for onboarding)

### Future Considerations

Re-evaluate OpenTUI when:

1. OpenTUI adds N-API/Node.js support officially
2. OpenTUI reaches 1.0 with stable API
3. Anvil needs advanced TUI features (syntax highlighting, animations)

At that point, migration from Ink would be straightforward as both use React
component patterns.

## References

- [Ink GitHub](https://github.com/vadimdemedes/ink)
- [OpenTUI GitHub](https://github.com/sst/opentui)
- [OpenTUI npm](https://www.npmjs.com/package/@opentui/core) — shows
  `bun-ffi-structs` dependency
