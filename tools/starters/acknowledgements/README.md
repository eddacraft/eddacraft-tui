# Acknowledgements starter kit

A drop-in third-party-attribution pipeline. The generator is a **dispatcher**
that reads a `[[blocks]]` array from `attribution.toml` and routes each block to
an ecosystem-specific driver under `drivers/<ecosystem>.sh`. Each driver emits
markdown that gets spliced between BEGIN/END marker comments in a target
markdown file (typically `ACKNOWLEDGEMENTS.md`). Hand-curated content above,
between, and below the markers is preserved verbatim.

Shipped drivers: Rust
([`cargo-about`](https://github.com/EmbarkStudios/cargo-about)), Node
([`license-checker`](https://github.com/davglass/license-checker)), Go
([`go-licenses`](https://github.com/google/go-licenses)), Python
([`pip-licenses`](https://github.com/raimon49/pip-licenses)), and bundled
binaries (a hand-maintained inventory, no external tool). Existing consumers
with a legacy flat `[rust]` config keep working unchanged via a back-compat shim
(see "Configuration reference" below).

The kit is the canonical home of the generator. To adopt it in another repo,
copy this directory wholesale — including [`LICENSE`](./LICENSE) (Apache-2.0) —
and edit one file (`attribution.toml`). No script edits are required.

## What ships in this kit

| File                                 | Purpose                                                                                      |
| ------------------------------------ | -------------------------------------------------------------------------------------------- |
| `generate-acknowledgements.sh`       | Dispatcher: parses config, loops blocks, invokes drivers, splices output                     |
| `drivers/`                           | Ecosystem driver scripts (`rust.sh`, `node.sh`, `go.sh`, `python.sh`, `bundled-binaries.sh`) |
| `bundled-binaries.toml.example`      | Inventory template for the bundled-binaries driver (non-package-manager binaries)            |
| `expand-licences.sh`                 | Single-source allow-list expander                                                            |
| `attribution.toml.example`           | Annotated template for the consumer-side config                                              |
| `about.toml.template`                | cargo-about config template (licence allow-list etc)                                         |
| `about.hbs.template`                 | cargo-about handlebars render template                                                       |
| `templates/go-licenses.tmpl`         | go-licenses report template for the Go driver (rows only; driver sorts)                      |
| `licences.node-allow.txt.template`   | Marker scaffolding for the Node driver's allow-list file                                     |
| `licences.go-allow.txt.template`     | Marker scaffolding for the Go driver's allow-list file                                       |
| `licences.python-allow.txt.template` | Marker scaffolding for the Python driver's allow-list file                                   |
| `ACKNOWLEDGEMENTS.md.template`       | Bootstrap target file with `{{BLOCK_NAME}}` markers                                          |
| `licences.toml.template`             | Canonical allow-list; expand into each driver's consumer file                                |
| `ci-freshness.yml.snippet`           | GitHub Actions freshness-gate job                                                            |
| `tests/`                             | Self-tests pinning the kit's invariants                                                      |
| `LICENSE`                            | Apache License 2.0 grant for this kit                                                        |
| `VERSION`                            | Kit semver; must match the newest `CHANGELOG.md` heading                                     |
| `CHANGELOG.md`                       | Consumer-facing release notes                                                                |
| `README.md`                          | This file (the marker-splice contract)                                                       |

## Adoption checklist (downstream consumer)

1. Copy the kit. Use a **per-kit prefix** so it becomes its own independently
   tracked subtree (each starter kit you adopt should get its own subdirectory
   at its own subtree prefix — using the parent `tools/starters` would lock the
   entire parent to one kit):

   ```bash
   git subtree add --prefix tools/starters/acknowledgements \
     <upstream> main --squash
   ```

   Or just `cp -r` the directory in if you don't want subtree tracking.

2. Bootstrap the always-needed files (one-off, then commit):

   ```bash
   cp tools/starters/acknowledgements/attribution.toml.example attribution.toml
   cp tools/starters/acknowledgements/ACKNOWLEDGEMENTS.md.template ACKNOWLEDGEMENTS.md
   cp tools/starters/acknowledgements/licences.toml.template licences.toml
   ```

   Then copy the per-driver config for each ecosystem you ship. For Node:

   ```bash
   cp tools/starters/acknowledgements/licences.node-allow.txt.template \
     licences.node-allow.txt
   ```

   For Rust:

   ```bash
   cp tools/starters/acknowledgements/about.toml.template about.toml
   cp tools/starters/acknowledgements/about.hbs.template about.hbs
   ```

   The Go and Python drivers use `licences.<eco>-allow.txt.template`; the
   bundled-binaries driver uses `bundled-binaries.toml.example`.

3. Edit `attribution.toml`: declare one `[[blocks]]` entry per ecosystem you
   ship. For Node-only, comment out the rust example and uncomment the
   `name = "node"` stanza. Then replace `{{BLOCK_NAME}}` in
   `ACKNOWLEDGEMENTS.md` with that same name (`node`, `rust`, …). Named blocks
   use suffixed markers (`<!-- BEGIN AUTO-GENERATED node -->`); the unsuffixed
   pair is only for the legacy flat `[rust]` shim.

4. Populate the allow-list. Edit `licences.toml`, then:

   ```bash
   tools/starters/acknowledgements/expand-licences.sh
   ```

   Node-only repos do not need `about.toml` or `deny.toml`. The expander writes
   only the consumer files that exist beside `licences.toml`.

5. Generate:

   ```bash
   tools/starters/acknowledgements/generate-acknowledgements.sh
   ```

   For Node, install `license-checker` as a devDependency first. The driver
   finds `node_modules/.bin/license-checker` itself; you do not need it on
   `PATH`.

6. Ship the notice. Put `ACKNOWLEDGEMENTS.md` in the artefact you redistribute —
   for npm, add it to `package.json` `"files"` (or stop ignoring it) so
   `npm pack` includes it.

7. Wire CI: drop `ci-freshness.yml.snippet` into your existing workflow.
   Uncomment only the setup steps for ecosystems you actually declare.

## Customising `ACKNOWLEDGEMENTS.md`

The template at `ACKNOWLEDGEMENTS.md.template` is a structural scaffold, not a
finished file. After copying it to your repo root, replace the `{{PLACEHOLDER}}`
tokens and prune the example sections to fit your stack.

### Placeholders

All placeholders use `{{DOUBLE_BRACES}}` so they are easy to grep and replace.
None of them are interpreted by the generator — they are plain text the template
author left for you to fill in. `{{BLOCK_NAME}}` is the load-bearing one: if you
leave it in the marker comments, the generator will not see a real pair and the
marker-count gate fails.

| Placeholder                 | Replace with                                                                                                                            |
| --------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `{{PROJECT_NAME}}`          | Display name of your project, e.g. `Anvil`                                                                                              |
| `{{PROJECT_BINARY}}`        | The shipping binary or package name, e.g. `anvil`                                                                                       |
| `{{GENERATOR_TOOL}}`        | The upstream tool that produces the auto-generated block, e.g. `cargo-about`, `license-checker`, `go-licenses`, or `pip-licenses`       |
| `{{GENERATOR_TOOL_URL}}`    | Upstream URL for the tool you use, e.g. `https://github.com/EmbarkStudios/cargo-about` or `https://github.com/davglass/license-checker` |
| `{{LOCKFILE_NAME}}`         | The lockfile the attribution derives from, e.g. `Cargo.lock`, `pnpm-lock.yaml`, `go.sum`, or `requirements.txt`                         |
| `{{GENERATOR_SCRIPT_PATH}}` | Path to the generator script as it appears in your repo, e.g. `tools/starters/acknowledgements/generate-acknowledgements.sh`            |
| `{{BLOCK_NAME}}`            | The block `name` from `attribution.toml`. Becomes the marker suffix (`<!-- BEGIN AUTO-GENERATED node -->`).                             |

A one-shot `sed` works for most of these. This Rust example can be adapted to
whichever driver owns your generated block:

```bash
sed -i \
  -e 's|{{PROJECT_NAME}}|Anvil|g' \
  -e 's|{{PROJECT_BINARY}}|anvil|g' \
  -e 's|{{GENERATOR_TOOL}}|cargo-about|g' \
  -e 's|{{GENERATOR_TOOL_URL}}|https://github.com/EmbarkStudios/cargo-about|g' \
  -e 's|{{LOCKFILE_NAME}}|Cargo.lock|g' \
  -e 's|{{GENERATOR_SCRIPT_PATH}}|tools/starters/acknowledgements/generate-acknowledgements.sh|g' \
  -e 's|{{BLOCK_NAME}}|rust|g' \
  ACKNOWLEDGEMENTS.md
```

### "Thanks" sections

The template ships four illustrative subsections (Language and tooling, Testing
and quality, Build and CI, Developer environment) with two `Project A` /
`Project B` bullets each. They exist to show the structure — categorised `###`
subsections, bullet list with link-reference style, link references grouped
immediately after each list.

Treat them as a starting point:

- **Keep** the categories that fit your stack.
- **Rename** categories if the labels don't match (e.g.
  `Monorepo and TypeScript tooling` instead of `Language and tooling`).
- **Delete** sections you don't use — there's no requirement that any specific
  category exists.
- **Add** sections for ecosystems the template doesn't anticipate (e.g.
  `Infrastructure`, `Documentation`, `Design`).

The generator does not police the shape of the hand-curated region; it only
preserves it verbatim.

### What you must not change

Two things in the template are load-bearing for the generator:

1. The marker pair at the bottom:

   ```markdown
   <!-- BEGIN AUTO-GENERATED {{BLOCK_NAME}} -->
   <!-- END AUTO-GENERATED {{BLOCK_NAME}} -->
   ```

   Replace `{{BLOCK_NAME}}` with the block `name` (`node`, `rust`, …). Each pair
   must appear **exactly once** on a line of its own. If you customise the
   marker text via `[project].marker_begin` / `marker_end` in
   `attribution.toml`, update both the template and the config together.

2. The order: BEGIN must precede END. The generator splices content between
   them; if they're swapped or duplicated, the marker-count gate fails.

Everything else — heading levels, prose, link styles, the intro paragraph — is
yours to edit.

### After customising

Run the generator once to populate the auto-generated block:

```bash
tools/starters/acknowledgements/generate-acknowledgements.sh
```

Then commit both the customised `ACKNOWLEDGEMENTS.md` and (if applicable) the
updated `attribution.toml`.

## The marker-splice contract

This section is the canonical reference for the kit's invariants. The generator
and any downstream consumer of the output rely on every rule below.

### Marker syntax

The target markdown file must contain **exactly one** BEGIN marker and **exactly
one** END marker per declared block, on lines of their own. Named blocks suffix
the default text with the block `name`:

```markdown
<!-- BEGIN AUTO-GENERATED node -->
<!-- END AUTO-GENERATED node -->
```

The unsuffixed pair (`<!-- BEGIN AUTO-GENERATED -->`) is only for the legacy
flat `[rust]` shim. Marker text is overridable per project via
`[project].marker_begin` / `[project].marker_end` in `attribution.toml`.

A line is a marker only when the trimmed line _is_ the marker — inline mentions,
code spans, and fenced samples are prose. Matching is literal, not regex, so an
override containing backslashes needs no escaping.

### Idempotency

Running the generator twice in a row against unchanged driver inputs produces a
byte-identical file the second time. The freshness gate (`--check`) relies on
this: it regenerates into a temporary file and `diff`s against the on-disk copy.

Idempotency requires:

- Each driver's render output is deterministic for its manifest or lockfile,
  template, and config. Pin external scanner versions in CI to keep this true
  across machines.
- Hand-edited content above the BEGIN marker and below the END marker is never
  rewritten. The splice loop emits those regions verbatim.

### Atomic write

The generator never writes the target file in place. It:

1. Generates each declared driver's output to a `mktemp` file.
2. Splices each output between that block's markers in a working target temp.
3. `mv`s the completed target over the on-disk file only after all blocks
   succeed.

A failure mid-run leaves the target untouched. Combined with the empty-output
guard (next), this prevents a partial generation from silently clobbering
content.

### Strict driver gates

Each ecosystem driver owns its own strictness checks before render. A disallowed
licence, missing required field, missing tool, missing manifest, or invalid
inventory exits non-zero and leaves the on-disk target byte-identical. The
per-driver reference below names the exact command or validation rule for each
ecosystem.

The Rust driver additionally invokes `cargo about generate --fail`, so a
workspace crate missing both `license` and `license-file` in its `Cargo.toml`
causes a hard error rather than a silent warning. The fixture test under
`tests/strict-license-field.sh` pins that Rust-specific contract.

### Single-source licence allow-list

Driver allow-lists are generated from a single canonical `licences.toml` at the
repo root. The kit's `expand-licences.sh` reads `licences.toml` and rewrites the
Rust `about.toml`, Rust `deny.toml`, and Node / Go / Python allow-list files
between BEGIN/END marker comments — the same splice pattern the acknowledgements
generator uses on `ACKNOWLEDGEMENTS.md`. CI runs the expander in `--check` mode
so drift between `licences.toml` and any generated consumer fails the build.

To add or remove a licence: edit `licences.toml`, run
`tools/starters/acknowledgements/expand-licences.sh`, and commit `licences.toml`
with every generated allow-list file that changed. The schema (single-line
strings only) is documented at the top of `licences.toml` itself.

The fixture test under `tests/licences-drift.sh` walks three scenarios — clean
expand → check passes, new licence in source → drift detected, hand-edit in
consumer → drift detected — so a regression that loosens the matcher is caught
in CI.

### Empty-output guard

If any driver produces a zero-byte file (e.g. because a template path is wrong,
a manifest doesn't resolve, or an external scanner crashes silently), the
generator aborts with a non-zero exit before the `mv` step. The target is never
overwritten with an empty block.

### What counts as a marker

Every gate below, and the splice itself, read one shared scan of the target, so
they cannot disagree about where the markers are. A line is a managed marker
only when **both** hold:

- **It is the whole line.** The trimmed line must _be_ the marker, with nothing
  before or after it. A marker quoted mid-sentence, in an inline code span, or
  inside link text is prose, and the kit leaves prose alone.
- **It is not inside a fenced code block.** Fences follow CommonMark: a fence
  opens with three or more backticks or tildes, and only a fence of the same
  character and at least the opener's length closes it. So a target may document
  the marker syntax in a fenced sample without that sample being treated as live
  markup.

Marker matching is literal throughout — never regex — so an operator-supplied
`marker_begin` / `marker_end` override needs no escaping, including one
containing backslashes or regex metacharacters.

### Marker-count gate

Before generation runs, the generator counts the target's managed BEGIN and END
marker lines for each block. If either count is not exactly 1, it exits with a
non-zero status and an actionable error. This catches:

- A target file that's missing the marker block entirely.
- A merge conflict that duplicated the markers — including two markers that
  ended up sharing a single line.
- A typo introduced when adding a new block.

Without this gate, the splice loop would no-op silently and `--check` would
falsely report "all good" while regeneration never happened.

### Marker-order gate

A BEGIN marker must appear before its END marker. Counts alone do not catch a
reversed pair, and splicing one would print the BEGIN marker, emit the driver
output, then swallow every remaining line to the end of the file — destroying
hand-curated content. The generator refuses before writing anything, naming both
line numbers.

### Orphaned-marker gate

A marker pair whose block is no longer declared in `attribution.toml` is
reported by name and refused, in write and `--check` alike.

Such a region is never regenerated, so its content is frozen at whatever the
block last produced — stale licence attribution that a freshness gate would
otherwise report as green. To resolve it, delete the orphaned marker pair from
the target, or re-declare the block in `attribution.toml`.

### `--check` exit-code semantics

| Exit | Meaning                                                                                                                        |
| ---- | ------------------------------------------------------------------------------------------------------------------------------ |
| 0    | Success / no drift. Safe to merge.                                                                                             |
| 1    | Drift detected, missing markers, empty output, missing tool, missing config, or other recoverable failure. CI should fail.     |
| 2    | CLI argument error (mutually exclusive flags, missing argument value). Indicates an invocation bug; rerun with corrected args. |

`--check` is the freshness gate: it does the full generate-and-splice into a
temporary file, then `diff -u`s against the on-disk target. Drift is reported as
a unified diff so the CI log makes the missing update obvious; the trailing
message points contributors at the `fixit_command` configured in
`attribution.toml`.

### Hand-curated content

Everything above the BEGIN marker and below the END marker is permanent,
hand-curated content. The kit explicitly does not police its shape — projects
use this region for `## Thanks` lists, intro paragraphs, link references, etc.
The generator treats those bytes as opaque.

## Configuration reference

`attribution.toml` (consumer-side, repo root) drives every project-specific
value. The dispatcher carries no hard-coded paths, markers, or fix-it strings.

Two schema shapes are accepted; they are **mutually exclusive** within one file.
Mixing them is a hard error so silent precedence rules never apply.

### Canonical: `[[blocks]]` array

Each entry declares one block. `name` and `ecosystem` are required; remaining
keys are ecosystem-specific (the driver script for that ecosystem documents what
it expects).

```toml
[project]
target_path   = "ACKNOWLEDGEMENTS.md"        # required
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"  # required
# marker_begin = "<!-- BEGIN AUTO-GENERATED -->"  # optional; default shown
# marker_end   = "<!-- END AUTO-GENERATED -->"    # optional; default shown

[[blocks]]
name          = "rust"                       # required; kebab-case when non-empty;
                                             # used to suffix the block's markers
                                             # (`<!-- BEGIN AUTO-GENERATED rust -->`)
ecosystem     = "rust"                       # required; must match drivers/<ecosystem>.sh
manifest_path = "crates/your-cli/Cargo.toml" # ecosystem-specific (rust driver)
template_path = "about.hbs"                  # ecosystem-specific (rust driver)
config_path   = "about.toml"                 # ecosystem-specific (rust driver)
```

A repo can declare multiple blocks — typically one per shipping artefact (a
pnpm-workspace monorepo shipping a CLI + an HTTP API + a sidecar daemon declares
three entries, each pointing at its own `package.json`). Block names must be
unique within the file; each block gets its own per-block marker pair in the
target file.

### Back-compat: flat `[rust]` table

The kit's original single-block schema. A config with `[rust]` and no
`[[blocks]]` entries is treated as if it declared a single unnamed block with
`ecosystem = "rust"`. Markers for the unnamed block omit the name suffix
(`<!-- BEGIN AUTO-GENERATED -->`), keeping today's `ACKNOWLEDGEMENTS.md` files
byte-identical after the refactor.

```toml
[project]
target_path   = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"

[rust]
manifest_path = "crates/your-cli/Cargo.toml"
template_path = "about.hbs"
config_path   = "about.toml"
```

Existing consumers don't need to migrate — the shim is permanent for the
Rust-only single-block case. Migrate to `[[blocks]]` when you want a second
block (Node devtools attribution, bundled binaries, etc.) or when you adopt a
non-Rust driver.

All paths are resolved relative to the directory containing `attribution.toml`.
Absolute paths are also accepted.

## Per-driver block reference

Each driver under `drivers/<ecosystem>.sh` declares its own block-config keys.
The dispatcher passes the resolved block JSON to the driver verbatim; the driver
is the authority on which keys it requires.

### `ecosystem = "rust"`

| Key             | Required | Description                                                                                                                        |
| --------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `manifest_path` | yes      | `Cargo.toml` to walk — typically the shipping binary's manifest, not the workspace root, to keep dev-only deps out of attribution. |
| `template_path` | yes      | `about.hbs` handlebars template the driver hands to `cargo about generate`.                                                        |
| `config_path`   | yes      | `about.toml` carrying `cargo-about`'s `accepted = [...]` list (auto-populated by `expand-licences.sh` from `licences.toml`).       |

Strict-licence gate: `cargo about generate --fail` rejects workspace crates
missing a `license` field. Always-on; no opt-out (consumer fixes the missing
field instead).

### `ecosystem = "node"`

| Key               | Required            | Description                                                                                                                                                     |
| ----------------- | ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `manifest_path`   | yes                 | `package.json` to walk. For monorepos: one block per shipping `package.json` is the recommended pattern (see Monorepo guidance below).                          |
| `node_allow_path` | yes                 | `licences.node-allow.txt` — single semicolon-joined SPDX list between the kit's BEGIN/END markers. Auto-populated by `expand-licences.sh` from `licences.toml`. |
| `prod_only`       | no (default `true`) | If `true`, `license-checker --production` excludes `devDependencies`. Set `false` to include dev tooling (the Anvil-devtools path).                             |
| `exclude`         | no                  | Semicolon-joined `package@version` list, forwarded raw to `license-checker --excludePackages`. Use to drop hoisted internal packages a workspace shim surfaces. |

Worked example:

```toml
[[blocks]]
name             = "node-devtools"
ecosystem        = "node"
manifest_path    = "package.json"
node_allow_path  = "licences.node-allow.txt"
prod_only        = false
```

Strict-licence gate: `license-checker --onlyAllow "<semi-joined SPDX list>"`
runs before render; one disallowed dep exits non-zero, names the offending
package@version in stderr, and leaves the on-disk target byte-identical.

The driver always excludes the manifest's own `name@version`, so a published
package is not attributed to itself. `--excludePrivatePackages` is also
always-on: workspace packages marked `"private": true` stay out. Use `exclude`
only for extra hoisted internals, not for the consumer package.

The driver looks for `license-checker` in `node_modules/.bin` walking up from
the manifest, then on `PATH`. Install per-project:

```bash
npm install --save-dev license-checker
```

### `ecosystem = "go"`

| Key             | Required | Description                                                                                                                                                         |
| --------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `module_path`   | yes      | Directory of the package/binary to walk (e.g. `cmd/anvil`). The driver finds the enclosing `go.mod` and runs `go-licenses` from that module root.                   |
| `go_allow_path` | yes      | `licences.go-allow.txt` — single comma-joined SPDX list between the kit's BEGIN/END markers. Auto-populated by `expand-licences.sh` from `licences.toml`.           |
| `template_path` | no       | go-licenses report template emitting `\| module \| licence \|` rows (no header — the driver sorts and adds it). Defaults to the kit's `templates/go-licenses.tmpl`. |

Worked example:

```toml
[[blocks]]
name          = "go"
ecosystem     = "go"
module_path   = "cmd/anvil"
go_allow_path = "licences.go-allow.txt"
```

Strict-licence gate:
`go-licenses check --allowed_licenses "<comma-joined SPDX list>"` runs before
render; one disallowed dep exits non-zero, names the offending library in
stderr, and leaves the on-disk target byte-identical. The project's own main
module (from `go list -m`) is `--ignore`d so it is never attributed to itself.
`go.mod` `replace` directives are honoured natively, so internal monorepo
modules need no special handling.

The rendered block carries the module import path + SPDX licence only — no
source URL. go-licenses resolves the URL over the network, which would make
`--check` non-deterministic across online/offline environments; the Go import
path is itself the canonical source location.

The driver requires `go` and `go-licenses` on `PATH`, and the module cache
populated (run `go mod download` first):

```bash
go install github.com/google/go-licenses@latest
go mod download
```

### `ecosystem = "python"`

| Key                 | Required | Description                                                                                                                                                                                                                            |
| ------------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `venv_path`         | yes      | A consumer-supplied **pre-built** virtualenv. The kit ships no installer opinions (no uv/poetry/pdm) — build the venv with your own toolchain, install your runtime deps **and `pip-licenses`** into it, then point `venv_path` at it. |
| `python_allow_path` | yes      | `licences.python-allow.txt` — single semicolon-joined SPDX list between the kit's BEGIN/END markers. Auto-populated by `expand-licences.sh` from `licences.toml`.                                                                      |

Worked example:

```toml
[[blocks]]
name              = "python"
ecosystem         = "python"
venv_path         = ".venv"
python_allow_path = "licences.python-allow.txt"
```

The driver runs the venv's own `pip-licenses`, so the attribution reflects
exactly the consumer's environment. `pip-licenses` self-excludes its own
dependency chain (pip-licenses/prettytable/wcwidth/…), so a venv that contains
only your runtime deps + pip-licenses renders just your dependencies. Render is
deterministic (`pip-licenses --format markdown --order name`).

Strict-licence gate: `pip-licenses --allow-only "<semicolon-joined SPDX list>"`
runs before render; one disallowed dep exits non-zero, names the offending
package, and leaves the on-disk target byte-identical.

**Licence-name caveat.** `pip-licenses` reports licence names derived from each
package's Trove classifiers / metadata, which are **not always exact SPDX
identifiers** (e.g. a package may report `BSD License` rather than
`BSD-3-Clause`, or `MIT License` rather than `MIT`). The allow-list expanded
from `licences.toml` is SPDX, so a mismatch can cause a false strict failure.

Do **not** work around this by adding classifier-style names (`BSD License`,
`MIT License`) to `licences.toml`: the `spdx` field there also feeds
`about.toml` (cargo-about) and `deny.toml` (cargo-deny), which require valid
SPDX identifiers — a non-SPDX value breaks those consumers. Safe options
instead:

- normalise the offending package's metadata (or pin a version that publishes a
  modern SPDX `License-Expression`);
- enable `pip-licenses --partial-match` (not enabled by the kit) so `MIT`
  matches `MIT License`, if you accept the looser matching;
- ignore the package in your venv composition if it is not actually shipped.

A Python-specific SPDX-alias mechanism in the expander is a possible future
enhancement; today the kit keeps `licences.toml` strictly SPDX.

CI provisioning (per your existing Python toolchain), e.g.:

```bash
python -m venv .venv
.venv/bin/python -m pip install -r requirements.txt pip-licenses
```

### `ecosystem = "bundled-binaries"`

For third-party **binaries** shipped alongside your project that are **not** a
package manager's dependencies — OpenSSH, Mosh, FFmpeg, busybox, … The language
drivers never see these; you attribute them from a hand-maintained inventory.

| Key              | Required | Description                                                                                       |
| ---------------- | -------- | ------------------------------------------------------------------------------------------------- |
| `inventory_path` | yes      | `bundled-binaries.toml` — an array of `[[binary]]` entries (see `bundled-binaries.toml.example`). |

Inventory entry schema (`name` + `spdx` required; `version`, `source` optional;
`spdx` is any SPDX expression — no closed enum):

```toml
[[binary]]
name    = "OpenSSH"
version = "9.6p1"
spdx    = "BSD-3-Clause"
source  = "https://www.openssh.com/"
```

Worked example:

```toml
[[blocks]]
name           = "binaries"
ecosystem      = "bundled-binaries"
inventory_path = "bundled-binaries.toml"
```

There is **no ecosystem-specific external tool** — the driver is pure bash/awk
over the inventory (it shares the dispatcher's `jq` requirement for parsing the
block config, but needs no licence scanner). Its "strict" step is field
validation: an entry missing `name` or `spdx` fails the gate, named, before
render. Render is a deterministic `| Binary | Version | License | Source |`
table sorted by name. The curator owns licence correctness (there is no upstream
scanner to cross-check). If you ship no bundled binaries, omit the block rather
than shipping an empty inventory.

## Monorepo guidance

The "right" attribution surface in a monorepo depends on shape. Three patterns
cover almost every case:

**One block per shipping artefact.** A pnpm-workspace monorepo shipping a CLI +
an HTTP API + a sidecar daemon declares three `[[blocks]]` entries, each pointed
at its own `package.json`:

```toml
[[blocks]]
name             = "cli"
ecosystem        = "node"
manifest_path    = "packages/cli/package.json"
node_allow_path  = "licences.node-allow.txt"

[[blocks]]
name             = "http-api"
ecosystem        = "node"
manifest_path    = "packages/http-api/package.json"
node_allow_path  = "licences.node-allow.txt"

[[blocks]]
name             = "sidecar"
ecosystem        = "node"
manifest_path    = "packages/sidecar/package.json"
node_allow_path  = "licences.node-allow.txt"
```

Each block gets its own marker pair in the target file. `--check` reports
per-block drift so a regression in one shipping package doesn't mask drift in
the others. Internal `@workspace/*` packages stay out automatically because they
are marked `"private": true`.

**Workspace-wide attribution (single block).** Point `manifest_path` at the root
`package.json`, accept the union of every workspace's prod deps:

```toml
[[blocks]]
name             = "node"
ecosystem        = "node"
manifest_path    = "package.json"
node_allow_path  = "licences.node-allow.txt"
prod_only        = true
```

Simple, broad, blunt. License-checker walks the root and resolves through the
workspace's hoisted `node_modules`. Use this when per-package separation is not
worth the bookkeeping.

**Devtools-only block (Anvil pattern).** When the prod surface is non-JS but you
still want to attribute the JS tooling the repo builds with (linters,
formatters, Nx, kindling integration), set `prod_only = false` on a
root-manifest block:

```toml
[[blocks]]
name             = "node-devtools"
ecosystem        = "node"
manifest_path    = "package.json"
node_allow_path  = "licences.node-allow.txt"
prod_only        = false
```

The block then includes `devDependencies`. Pair with `exclude` if a noisy dep
keeps surfacing.

## Dispatcher and driver contracts

The dispatcher (`generate-acknowledgements.sh`) is responsible for: TOML
parsing, schema validation (mixed-schema detection, name uniqueness, ecosystem →
driver lookup), per-block marker-count gating, splicing each driver's output
between its block's markers in a working temp, and a single atomic `mv` over the
target at the end of the loop. Any driver exiting non-zero aborts before the
atomic `mv` — the on-disk target stays byte-identical.

A driver script under `drivers/<ecosystem>.sh` is invoked with two arguments:

```
drivers/<ecosystem>.sh <block-config-json> <output-temp-path>
```

`<block-config-json>` is a compact JSON object containing the block's resolved
config (absolute paths). `<output-temp-path>` is where the driver writes its
rendered markdown.

Each driver must:

1. **Preflight** — verify required tool + state; actionable error on stderr;
   non-zero exit if anything is missing.
2. **Render** — deterministic markdown sorted/structured by the tool's own
   template. Idempotency under `--check` depends on it.
3. **Strict-licence check** — reject disallowed or missing licences _before_
   render (cargo-about uses `--fail`; other drivers wire their equivalent).
4. **No side effects on the splice target** — write only to the
   `<output-temp-path>` argument. Never touch `target_path` directly.

Tests can swap in stub drivers via `ATTRIB_DRIVERS_DIR=<dir>`; production
consumers should leave the env var unset and let the dispatcher use the
kit-local `drivers/` directory.

## Future evolution

The dispatcher + driver-per-ecosystem architecture landed with the Rust driver;
Node, Go, Python, and bundled-binaries drivers followed — see the per-driver
block reference above. The roadmap is tracked in the upstream Anvil project:

- Java/Kotlin / Ruby / Swift drivers deferred until a real consumer needs them

Each new driver is a self-contained `drivers/<eco>.sh` against the driver
contract documented above; the dispatcher doesn't need to change.

## Public mirror

This directory is the canonical source. A read-only public mirror is maintained
at
[`eddacraft/acknowledgements-starter`](https://github.com/eddacraft/acknowledgements-starter)
for projects (typically public ones) that can't `git subtree` from this private
repository.

The mirror is force-pushed by
`.github/workflows/mirror-acknowledgements-starter.yml` on every change to
`main` under this directory. Direct commits to the public mirror are overwritten
on the next sync — all edits land here.

To force a manual re-sync (e.g. after editing the workflow itself):

```bash
gh workflow run mirror-acknowledgements-starter.yml --ref main \
  -f reason='why you re-synced'
```

The `reason` input is recorded on the throwaway prepend-README commit and in the
GitHub step summary, so manual dispatches are auditable later.

### Versioned releases

Alongside the rolling `main` mirror, deliberate releases are cut as immutable
`vX.Y.Z` tags + GitHub Releases on the mirror, so external consumers can pin a
known-good version and be notified of updates. The kit's version lives in
[`VERSION`](./VERSION) and [`CHANGELOG.md`](./CHANGELOG.md); both travel into
the mirror, as does [`LICENSE`](./LICENSE) (Apache-2.0).

To cut one: bump `VERSION` + add a `CHANGELOG.md` entry (PR), then tag the merge
commit `acknowledgements-starter-vX.Y.Z` and push it.
`.github/workflows/release-acknowledgements-starter.yml` does the rest. The full
procedure (incl. the mirror tag-protection pre-flight) lives in
`docs/runbooks/acknowledgements-starter-release.md` in the upstream repository —
deliberately not a relative link, because this file is mirrored standalone and
that path does not exist beside it.

Check `VERSION` ↔ `CHANGELOG.md` consistency locally with:

```bash
bash tools/starters/acknowledgements/check-version.sh
```

## Running the kit's own tests

`tests/run-all.sh` is the single source of the kit's test list — CI invokes it
rather than restating the list, so there is only one place to keep current.

That list is the hand-ordered `TESTS` array inside the runner, not a directory
glob: a new script under `tests/` must be added to it. Forgetting is an error
rather than a silent omission — the runner fails if it finds a test file on disk
that the array does not name.

```bash
bash tools/starters/acknowledgements/tests/run-all.sh              # run everything
bash tools/starters/acknowledgements/tests/run-all.sh --require-all # CI mode
bash tools/starters/acknowledgements/tests/run-all.sh --list        # list only
```

Several driver tests exit 0 with a `skip:` line when their external scanner or
network is unavailable. The runner reports those as `SKIP`, distinct from
`PASS`, and `--require-all` turns them into failures — so a green CI run means
every ecosystem was actually exercised, not that three of them quietly opted
out.

If you adopted this kit from the public mirror, copy `kit-tests.yml.snippet`
into your own `.github/workflows/` to get the same gate, including the pinned
scanner versions the driver tests expect.

## Licence

This kit is licensed under the [Apache License 2.0](./LICENSE). Keep that file
when you copy or subtree-add the directory — it is the grant that lets you adopt
the kit.
