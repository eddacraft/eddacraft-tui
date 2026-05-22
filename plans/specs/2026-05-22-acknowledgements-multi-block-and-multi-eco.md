<!-- APS: Design spec for the acknowledgements starter kit's multi-block dispatcher + per-ecosystem drivers -->

# Acknowledgements starter kit — multi-block dispatcher and multi-ecosystem drivers

Date: 2026-05-22
Module: `ATTRIB` (sharpens ATTRIB-008 and adds ATTRIB-012/013/014/015)
Status: Ready
Coordinates with:
[`plans/modules/attribution-pipeline-v3.aps.md`](../modules/attribution-pipeline-v3.aps.md),
[`tools/starters/acknowledgements/README.md`](../../tools/starters/acknowledgements/README.md)

## Goal

Pin the *shape* of the multi-block, multi-ecosystem evolution of the
acknowledgements starter kit so ATTRIB-008 through ATTRIB-015 can land
without re-litigating the architecture per task.

Specifically:

1. The `attribution.toml` `[[blocks]]` array schema and its back-compat
   shim for existing flat-`[rust]` consumers.
2. The dispatcher contract that the main generator script honours
   (parse → loop → invoke driver → splice → drift-check).
3. The driver-author contract that each `drivers/<ecosystem>.sh` must
   satisfy (preflight, render, strict-license, deterministic output,
   actionable failure modes).
4. The four ecosystem drivers shipping in v3.2 — Rust (extracted),
   Node (new), Go (new), Python (new) — including tool choice,
   manifest-input shape, and state prerequisites.
5. The monorepo manifest-scoping pattern that downstream consumers
   should follow when adopting the kit against a non-trivial workspace.

Behaviour-preserving for existing consumers: Anvil today, the
`eddacraft/acknowledgements-starter` public mirror, and the
`eddacraft-tui` subtree consumer continue to regenerate byte-identically
through the back-compat shim. No consumer is forced to migrate to the
new `[[blocks]]` array.

## Context

`tools/starters/acknowledgements/` currently ships a single-driver
generator: one `[rust]` table in `attribution.toml`, one `cargo about
generate` invocation, one BEGIN/END marker pair. The Rust-only shape is
hard-coded at `generate-acknowledgements.sh:246-252` and the schema is
hard-coded at the `read_toml_value` calls around line 170.

The owner still depends on JS/TS in dev tooling (linters, formatters,
Nx, kindling integration, build scripts) and historical projects shipped
JS/TS to npm; downstream consumers of the kit (`eddacraft-tui` today,
the owner's future public projects in the pipeline) may ship Go and
Python alongside or instead of Rust. The single-language scope is
called out as smell #3 in `attribution-pipeline-v3.aps.md` (Problem
this solves, point 3). ATTRIB-008 was always the entry point for
fixing it.

This spec sharpens that entry point: rather than just "allow multiple
named blocks in one file," ATTRIB-008 becomes the keystone refactor
that turns the generator into a dispatcher with pluggable drivers, so
ATTRIB-012/013/014 can land independently without cross-pollination.

## Schema: `[[blocks]]` array

### New canonical shape

```toml
[project]
target_path   = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"
# marker_begin / marker_end stay supported for global override; per-block
# names suffix the marker text (see "Markers" below).

[[blocks]]
name          = "rust"                            # required, kebab-case
ecosystem     = "rust"                            # required, matches drivers/<ecosystem>.sh
manifest_path = "crates/anvil-cli/Cargo.toml"     # required (ecosystem-specific)
template_path = "about.hbs"                       # required (ecosystem-specific)
config_path   = "about.toml"                      # required for Rust; optional elsewhere

[[blocks]]
name          = "node-devtools"
ecosystem     = "node"
manifest_path = "tools/dev/package.json"
prod_only     = false                             # dev tools — include devDependencies
exclude       = ["@workspace/*"]                  # optional; pnpm-hoisted internal pkgs
```

### Back-compat shim

A consumer with the existing flat-`[rust]` schema (Anvil today,
`eddacraft-tui` via mirror) is auto-promoted to a single unnamed block:

```toml
# Today's schema, unchanged
[project]
target_path   = "ACKNOWLEDGEMENTS.md"
fixit_command = "..."

[rust]
manifest_path = "..."
template_path = "..."
config_path   = "..."
```

Internally, the dispatcher reads this as if it were:

```toml
[[blocks]]
name          = ""                                # unnamed → uses default markers
ecosystem     = "rust"
manifest_path = "..."
template_path = "..."
config_path   = "..."
```

Markers stay `<!-- BEGIN AUTO-GENERATED -->` (the existing default) when
the block name is empty. A consumer can opt into named markers by
migrating to `[[blocks]]` whenever they're ready; no flag day.

### Validation rules

- Each `[[blocks]]` entry must declare both `name` and `ecosystem`.
- `name` is kebab-case, unique within the config, and used to suffix
  the marker text — so `name = "rust"` produces
  `<!-- BEGIN AUTO-GENERATED rust -->`. Empty name is reserved for the
  back-compat shim and produces the legacy markerless form.
- `ecosystem` must match a file at `drivers/<ecosystem>.sh`. Unknown
  ecosystems fail fast with `error: no driver for ecosystem '<x>';
  expected drivers/<x>.sh`.
- The flat-`[rust]` shim and the `[[blocks]]` array are mutually
  exclusive. Mixing both is a config error (fails preflight with an
  actionable message).
- Block-name collisions across the array fail preflight before any
  driver runs.

## Markers

Per-block markers append the block name to the default marker text:

```markdown
<!-- BEGIN AUTO-GENERATED rust -->
...rust attribution block...
<!-- END AUTO-GENERATED rust -->

<!-- BEGIN AUTO-GENERATED node-devtools -->
...node attribution block...
<!-- END AUTO-GENERATED node-devtools -->
```

The marker-count gate (from the current README invariants section)
applies *per block*: each block's BEGIN must appear exactly once and
its END must appear exactly once. Other blocks' markers don't count
against the gate.

`marker_begin` / `marker_end` in `[project]` continue to override the
default marker text globally; the block-name suffix is appended after
the override.

## Dispatcher contract

The main `generate-acknowledgements.sh` becomes a thin orchestrator:

1. **Parse `attribution.toml`**. Detect schema shape (flat-`[rust]` vs
   `[[blocks]]`). Resolve the back-compat shim into an in-memory block
   list. Reject mixed schemas.
2. **Preflight (global)**. Verify `attribution.toml` is well-formed,
   `target_path` exists, no name collisions across blocks.
3. **Loop blocks**. For each block:
   a. **Marker-count gate** for this block's markers in the target.
   b. **Invoke driver**: `drivers/<ecosystem>.sh <block-config-json>
      <output-temp>`. The driver writes its rendered markdown to
      `<output-temp>` and exits non-zero on any failure.
   c. **Splice** the driver's output between this block's markers,
      writing to a same-filesystem temp file.
4. **Drift check (`--check` mode only)**. Diff the spliced output
   against the on-disk target; report drift per block in the unified
   diff output.
5. **Atomic mv**. Rename the temp file over `target_path`. In
   `--check` mode this step is skipped.

Failure isolation: if block N's driver exits non-zero, the dispatcher
stops the loop and reports which block failed. Blocks 1..N-1 have
already produced their temp output but the atomic mv hasn't happened
yet — so the on-disk target is untouched. This preserves the
"partial failure can't clobber unrelated content" invariant by virtue
of the existing atomic-write contract, not by per-block writes.

CLI surface stays unchanged: `--check`, `--output <path>`, `--config
<path>`, `-h`, `--help`. Exit codes stay as documented in the README's
`--check` exit-code semantics section.

## Driver-author contract

A driver script under `drivers/<ecosystem>.sh` is invoked by the
dispatcher with two arguments:

```
drivers/<ecosystem>.sh <block-config-json> <output-temp-path>
```

`<block-config-json>` is a JSON object containing the resolved block
config (absolute paths, all required keys present). `<output-temp-path>`
is where the driver writes its rendered markdown.

Each driver MUST satisfy:

1. **Preflight**. Verify the required tool is installed and the
   required state is present (lockfile, node_modules, module cache,
   venv). Failure produces an actionable error on stderr naming the
   missing dependency and how to fix it; exit non-zero.
2. **Render**. Emit deterministic markdown — sorted by package name,
   stable formatting — so repeated runs against unchanged inputs
   produce byte-identical output (idempotency under `--check`).
3. **Strict-license check**. Reject disallowed or missing licences
   *before* render, with a non-zero exit and an actionable error.
   The canonical allow-list lives in `licences.toml` (per ATTRIB-006);
   each driver consumes the ecosystem-shaped fragment the expander
   emits.
4. **No side effects on the target**. The driver writes only to its
   `<output-temp-path>` argument. Never touches `target_path`, never
   modifies state outside the temp.

Drivers do NOT see the marker text, the target file, or the splice
logic — those stay in the dispatcher. This keeps drivers narrow and
testable.

## Per-ecosystem drivers

### Rust — `drivers/rust.sh` (ATTRIB-008 carve-out)

- **Tool**: `cargo-about` (existing, pinned in CI).
- **Block keys**: `manifest_path` (Cargo.toml to walk), `template_path`
  (about.hbs), `config_path` (about.toml).
- **State prerequisite**: workspace is `cargo metadata`-able; Cargo.lock
  resolved.
- **Strict-license**: `cargo about generate --fail` (already wired per
  ATTRIB-007).
- **Render**: `cargo about generate` from the directory containing
  about.toml, output captured to a temp, sorted/templated by
  cargo-about's deterministic handlebars output.
- **Behaviour change**: none. ATTRIB-008 extracts existing code
  verbatim into the driver file; the freshness gate stays green.

### Node — `drivers/node.sh` (ATTRIB-012)

- **Tool**: `license-checker`.
- **Block keys**: `manifest_path` (package.json to walk), `prod_only`
  (default `true`), optional `exclude` globs.
- **State prerequisite**: `node_modules` populated at the workspace
  root (consumer ran `pnpm install` / `npm install`).
- **Strict-license**: `license-checker --failOn '<comma-separated
  disallowed list>'` — the disallowed list is the inverse of the
  Node-shaped fragment that ATTRIB-006's expander emits from
  `licences.toml`.
- **Render**: `license-checker --json --start <package-dir>` → JSON
  → sorted by package name → templated into the kit's Node markdown
  template under `templates/node-licenses.tmpl`.
- **pnpm-hoisting handling**: `license-checker --start <pkg-dir>`
  resolves through the workspace's root `node_modules`; the
  `exclude` globs in the block config drop internal `@workspace/*`
  packages that pnpm surfaces in the graph but the consumer doesn't
  want attributed.

### Go — `drivers/go.sh` (ATTRIB-013)

- **Tool**: `go-licenses` (Google's official tool, pinned in CI).
- **Block keys**: `module_path` (binary import path to walk, e.g.
  `./cmd/anvil`), `template_path` (Go template at
  `templates/go-licenses.tmpl`).
- **State prerequisite**: module cache populated (consumer ran
  `go mod download`).
- **Strict-license**: `go-licenses check <module_path>` runs ahead of
  `report`. The check's allow-list is the Go-shaped fragment from
  ATTRIB-006's expander.
- **Render**: `go-licenses report <module_path> --template
  <template_path>` → native sorted output → captured to temp.
- **Monorepo handling**: `replace` directives in `go.mod` are honoured
  natively. Consumers with multiple Go binaries declare multiple
  `[[blocks]]` entries, each with its own `module_path`.

### Python — `drivers/python.sh` (ATTRIB-014)

- **Tool**: `pip-licenses`.
- **Block keys**: `venv_path` (required, points at the consumer's
  pre-built virtualenv directory), optional `template_path`.
- **State prerequisite**: a virtualenv at `venv_path` containing the
  project's installed dependencies *and* `pip-licenses` itself.
  Missing venv or missing `pip-licenses` fails preflight with the
  actionable error: `error: no installed dependencies at <path>; run
  <your installer> first (e.g. uv sync, poetry install, pdm sync)`.
- **Strict-license**: `pip-licenses --fail-on '<disallowed list>'`
  matched against the Python-shaped fragment from ATTRIB-006's
  expander.
- **Render**: `pip-licenses --format=markdown --order=name
  --with-urls` → captured to temp.
- **Installer agnosticism**: the kit ships no opinion on `uv` vs
  `poetry` vs `pdm` vs `pip-tools`. The consumer wires their preferred
  installer in CI; the driver only requires that the resulting venv
  contains the project deps.

## Monorepo manifest-scoping pattern

The Rust kit already takes the right stance at
`attribution.toml.example:26`: point at the **shipping binary's
manifest** (`crates/anvil-cli/Cargo.toml`), not the workspace root, so
dev-only deps stay out. That principle generalises.

**Pattern: one block per shipping artefact.** A pnpm monorepo shipping
a CLI + an HTTP API + a sidecar daemon declares three `[[blocks]]`
entries, each pointed at its package's `package.json`. Each gets its
own marker pair and its own diff in `--check` output.

**Escape hatches**:

- **Workspace-wide attribution**: point `manifest_path` at the root
  `package.json` (or `Cargo.toml`), set `prod_only = true`, accept
  the union of every package's prod deps. Simple, broad, blunt.
- **Per-package blocks**: as above; verbose but explicit and
  attributable.
- **`exclude` globs**: trim internal `@workspace/*` packages that the
  package manager hoists into the graph but the consumer doesn't
  want listed.

For Anvil's own ACKNOWLEDGEMENTS.md (ATTRIB-015): two blocks — the
existing Rust block scoped to `crates/anvil-cli/Cargo.toml`, plus a
new `node-devtools` block scoped to either the root `package.json`
or a curated dev-tooling manifest. The exact scope decision lands
during ATTRIB-015 implementation based on which gives the cleanest
attribution surface.

## Non-goals

- **Java/Kotlin, Ruby, Swift drivers.** Reserved for re-decision if a
  real consumer surfaces a need. The dispatcher contract accommodates
  them without further architecture work — adding one is a
  driver-script + allow-list-fragment exercise.
- **CycloneDX SBOM as the canonical intermediate.** Stays queued as
  an alternative per-block `tool=` option (e.g.
  `tool = "cyclonedx-npm"` instead of `tool = "license-checker"`); not
  on the v3.2 critical path. The single-format-per-block model handles
  both deterministic-markdown drivers and future SBOM drivers without
  conflict.
- **Rewriting the generator in Rust as `anvil licenses generate`.**
  Considered and reserved as a future re-decision if a single driver
  outgrows shell. The current decision keeps the kit drop-in portable
  for subtree consumers — a compiled binary would break that.
- **Per-target / per-architecture attribution.** Inherited from
  cargo-about's `targets = [...]` list at the block level; no
  per-target marker blocks.

## Open questions

- **Anvil's `node-devtools` scope** (ATTRIB-015 implementation
  decision): root `package.json` or a curated devtools manifest under
  `tools/dev/package.json`? Decide based on whether the curated path
  produces meaningfully cleaner attribution. If the root manifest with
  `prod_only = false` is already clean enough, prefer it for
  simplicity.
- **`license-checker` strict-mode fragment shape** (ATTRIB-012
  implementation decision): does `--failOn` consume the full SPDX
  list, or do we need a Node-specific transform? Confirm against
  `license-checker`'s current CLI surface during ATTRIB-012 kickoff.
- **`pip-licenses` strict-mode fragment shape** (ATTRIB-014
  implementation decision): same question for `--fail-on`. Confirm
  against `pip-licenses`'s current CLI surface during ATTRIB-014
  kickoff.

## Cross-references

- Module: [`attribution-pipeline-v3.aps.md`](../modules/attribution-pipeline-v3.aps.md)
- Current kit README:
  [`tools/starters/acknowledgements/README.md`](../../tools/starters/acknowledgements/README.md)
- Existing generator:
  [`tools/starters/acknowledgements/generate-acknowledgements.sh`](../../tools/starters/acknowledgements/generate-acknowledgements.sh)
- Allow-list expander (ATTRIB-006):
  [`tools/starters/acknowledgements/expand-licences.sh`](../../tools/starters/acknowledgements/expand-licences.sh)
- Canonical allow-list: [`licences.toml`](../../licences.toml)
- Public mirror destination:
  <https://github.com/eddacraft/acknowledgements-starter>
