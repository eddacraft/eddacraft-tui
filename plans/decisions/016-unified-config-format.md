# ADR-016: Unified Configuration Format — Single TOML with Source Delegation

> **Rejected 2026-08-12** before acceptance, in favour of
> [ADR-120](120-config-surface-consolidation.md). Two premises were invalidated
> by shipped work: MLP-011/MLP2-040 established a multi-format, yaml-first
> config surface (reversing the TOML-only mandate), and the v0.9.0-beta release
> ended the "no users to migrate" assumption. The problem statement and the
> `SectionOrSource<T>` delegation idea carry forward into ADR-120 and the
> rewritten UCFG module.

## Status

Rejected

## Date

2026-04-03

## Context

Anvil currently splits configuration across three files with different formats,
key casing conventions, and schema versions:

| File | Format | Key casing | Schema version | Created by |
|------|--------|-----------|----------------|------------|
| `.anvilrc` | JSON / YAML / TOML | camelCase (JSON/YAML), snake_case (TOML) | `1.0.0` | `anvil init` |
| `.anvil/gate-config.json` | JSON | snake_case | `1` (integer) | `anvil gate-config` |
| `.anvil/architecture.yaml` | YAML | snake_case | `0.1.0` | User / `anvil architecture init` |

This split causes concrete problems:

1. **Documentation accuracy** — A Council review (council-09fc9567) found 4
   critical and 7 major errors in the docs, most caused by schema drift between
   files. The architecture.yaml example used the wrong structure entirely
   (list vs map), the YAML `.anvilrc` example used the wrong key casing, and
   `--json` flag placement was wrong because of format-specific assumptions.

2. **Key casing inconsistency** — `.anvilrc` uses `camelCase` in JSON/YAML
   (via `serde(rename_all = "camelCase")`) but `snake_case` in TOML. The other
   two files use `snake_case`. There is no cross-file schema governance.

3. **Format divergence** — Three formats (JSON, YAML, TOML) across three files,
   each with different parsing characteristics and merge-conflict behaviour.

4. **AI agent ergonomics** — Agents (Anvil's primary audience) must discover,
   read, and correlate three separate files to understand the full configuration
   surface. A single file with a clear schema is dramatically easier to reason
   about programmatically.

5. **Onboarding friction** — New users must learn that three files exist, where
   each lives, and which settings belong where. `anvil init` only creates
   `.anvilrc`; the other files are created by different commands or manually.

The tool is pre-launch — all projects are greenfield. There are no existing
users to migrate, so this is the right moment to get the config format right.

## Decision

Consolidate all configuration into a single `.anvilrc` file in **TOML** format,
using **snake_case** throughout. Sections that grow large can be **delegated** to
external files via a `source` key. This delegation pattern is generalised across
all sections from day one.

### File structure

```toml
# .anvilrc

[project]
schema_version = "1.0.0"
planning_dir = "plans"
format = "yaml"
checks = ["secret-detection", "import-boundaries"]

[gate]
overall_score = 80

[[gate.checks]]
name = "lint"
description = "Code quality and style checks"
enabled = true

[[gate.checks]]
name = "test"
description = "Test suite execution"
enabled = true

[[gate.checks]]
name = "coverage"
description = "Code coverage thresholds"
enabled = false

[[gate.checks]]
name = "dependency"
description = "Dependency vulnerability scanning"
enabled = true

[[gate.checks]]
name = "secret"
description = "Secret and credential detection"
enabled = true

[[gate.checks]]
name = "architecture"
description = "Architecture boundary validation"
enabled = true

[[gate.checks]]
name = "policy"
description = "Policy compliance evaluation"
enabled = true

[architecture]
schema_version = "0.1.0"
template = "layered"

[architecture.layers.api-layer]
patterns = ["src/api/**"]
depends_on = ["service-layer", "utils"]

[architecture.layers.service-layer]
patterns = ["src/services/**"]
depends_on = ["repository-layer", "utils"]

[architecture.layers.repository-layer]
patterns = ["src/repositories/**"]
depends_on = ["utils"]

[architecture.layers.utils]
patterns = ["src/utils/**"]
depends_on = []

[architecture.options]
detect_orphans = true
detect_circular = true
default_severity = "error"
```

### Source delegation pattern

Any top-level section other than `[project]` can be delegated to an external
file by replacing its contents with a single `source` key:

```toml
# .anvilrc — delegated architecture

[project]
schema_version = "1.0.0"
planning_dir = "plans"
format = "yaml"
checks = ["secret-detection", "import-boundaries"]

[gate]
overall_score = 80
# ... inline gate checks ...

[architecture]
source = ".anvil/architecture.toml"
```

The delegated file contains the section contents directly (no wrapping
`[architecture]` table):

```toml
# .anvil/architecture.toml

schema_version = "0.1.0"
template = "layered"

[layers.api-layer]
patterns = ["src/api/**"]
depends_on = ["service-layer", "utils"]

[layers.service-layer]
patterns = ["src/services/**"]
depends_on = ["repository-layer", "utils"]
```

#### Delegation rules

1. **Exclusive** — If `source` is present, no other keys are permitted in that
   section. If both `source` and inline keys are detected, the config is
   rejected with an actionable error. No merge semantics, ever. A section is
   either fully inline or fully delegated.

2. **One level deep** — A delegated file cannot itself contain `source` keys.
   This eliminates circular references by construction, with no need for
   visited-set tracking.

3. **Relative paths** — `source` paths are resolved relative to the workspace
   root.

4. **Same format** — Delegated files must be TOML. No format mixing.

5. **Generalised** — The pattern applies to `[gate]`, `[architecture]`, and any
   future sections equally. The loader implements delegation once, not per
   section.

### Key casing

**snake_case everywhere.** TOML's convention is snake_case, Rust's convention is
snake_case, serde's default is snake_case. The current `camelCase` in JSON/YAML
was a historical Node.js convention that no longer applies.

Architecture retains its own `schema_version` (`"0.1.0"`) within its section.
The `[project]` section uses `schema_version = "1.0.0"` — no bump needed since
there are no existing users to migrate from.

### Serde implementation

```rust
#[derive(Debug, Deserialize)]
struct AnvilConfig {
    project: ProjectConfig,
    #[serde(default)]
    gate: SectionOrSource<GateConfig>,
    #[serde(default)]
    architecture: SectionOrSource<ArchitectureConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SectionOrSource<T> {
    Inline(T),
    Delegated(SourceRef),
}

#[derive(Debug, Deserialize)]
struct SourceRef {
    source: PathBuf,
}
```

The `untagged` enum tries `SourceRef` first (single `source` field) then falls
back to inline `T`. Error messages from the `untagged` fallback are poor by
default — implement a custom deserializer that detects the `source` key and
produces clear error messages:

```
error: [architecture] has both 'source' and inline keys.
       Use 'source' alone to delegate, or remove 'source' to define inline.
```

### Validation topology

`anvil doctor` validates all 2^N topologies (N = number of delegatable
sections). With 2 delegatable sections (gate, architecture), that's 4 states.
Each state is tested:

| project | gate | architecture | Topology |
|---------|------|-------------|----------|
| inline | inline | inline | Single file (default) |
| inline | inline | delegated | Split architecture |
| inline | delegated | inline | Split gate |
| inline | delegated | delegated | Fully split |

`[project]` is always inline — it's the root identity and contains `schema_version`.

## Consequences

### Positive

- **Single source of truth** — One file to read, edit, and document
- **Consistent casing** — snake_case everywhere, no format-dependent variation
- **AI agent friendly** — One file, one format, one schema
- **CODEOWNERS compatible** — Delegated files can have separate ownership
- **Simpler docs** — One configuration page, one schema reference
- **TOML native** — Aligns with Rust ecosystem conventions (Cargo.toml)
- **Future-proof** — New sections get delegation for free

### Negative

- **TOML verbosity** — `[[gate.checks]]` array-of-tables syntax is more verbose
  than JSON arrays. Architecture layers as nested TOML tables are noisier than
  YAML's indentation-based structure.
- **Lost YAML ergonomics** — YAML is arguably more readable for deeply nested
  architecture definitions. Mitigated by delegation to a separate file when
  definitions grow large.
- **Custom deserializer** — `SectionOrSource<T>` requires careful implementation
  and testing for good error messages.

### Neutral

- **Format support** — `.anvilrc` becomes TOML-only. JSON and YAML support for
  the root config file is dropped. This simplifies the loader but removes a
  user choice.
- **`.anvil/` directory** — Continues to exist for cache, delegated config
  files, and other runtime artefacts. It is not eliminated.

## Alternatives Considered

### Keep the three-file split

The architect's initial recommendation. Valid separation of concerns, but the
documentation burden, casing inconsistency, and AI agent ergonomics are concrete
costs that outweigh the theoretical lifecycle independence. The escape hatch
recovers the split for teams that actually need it, making the unified format
strictly more flexible.

### Single file without delegation

The pragmatic lead's initial proposal. Simpler, but architecture definitions can
grow to hundreds of lines. Without an escape hatch, large configs become
unwieldy and CODEOWNERS can't target specific concerns.

### YAML as the unified format

YAML is more readable for nested structures, but it's not the Rust ecosystem
convention, has surprising parsing edge cases (the Norway problem, implicit type
coercion), and would require `serde_yaml` where `toml` is already a dependency.

## References

- Council session `council-6f9c94e3` (planning review, 3 reviewers)
- Council session `council-09fc9567` (code review that surfaced the
  documentation accuracy issues, 11 findings)
- [TOML spec — Tables](https://toml.io/en/v1.0.0#table)
- [serde untagged enums](https://serde.rs/enum-representations.html#untagged)
