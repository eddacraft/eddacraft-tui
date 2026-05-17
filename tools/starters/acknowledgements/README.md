# Acknowledgements starter kit

A drop-in third-party-attribution pipeline. Wraps
[`cargo-about`](https://github.com/EmbarkStudios/cargo-about) and splices its
output between BEGIN/END marker comments in a target markdown file (typically
`ACKNOWLEDGEMENTS.md`). Hand-curated content above and below the markers is
preserved verbatim.

The kit is the canonical home of the generator. To adopt it in another repo,
copy this directory wholesale and edit one file (`attribution.toml`) — no script
edits required.

## What ships in this kit

| File                           | Purpose                                              |
| ------------------------------ | ---------------------------------------------------- |
| `generate-acknowledgements.sh` | The parameterised generator                          |
| `expand-licences.sh`           | ATTRIB-006 single-source allow-list expander         |
| `attribution.toml.example`     | Annotated template for the consumer-side config      |
| `about.toml.template`          | cargo-about config template (licence allow-list etc) |
| `about.hbs.template`           | cargo-about handlebars render template               |
| `ACKNOWLEDGEMENTS.md.template` | Bootstrap target file with markers in place          |
| `ci-freshness.yml.snippet`     | GitHub Actions freshness-gate job                    |
| `tests/`                       | Self-tests pinning the kit's invariants              |
| `README.md`                    | This file (the marker-splice contract)               |

## Adoption checklist (downstream consumer)

1. Copy the kit:

   ```bash
   git subtree add --prefix tools/starters/acknowledgements \
     <upstream> main --squash
   ```

   Or just `cp -r` the directory in if you don't want subtree tracking.

2. Bootstrap the consumer-side files (one-off, then commit):

   ```bash
   cp tools/starters/acknowledgements/attribution.toml.example attribution.toml
   cp tools/starters/acknowledgements/about.toml.template about.toml
   cp tools/starters/acknowledgements/about.hbs.template about.hbs
   cp tools/starters/acknowledgements/ACKNOWLEDGEMENTS.md.template ACKNOWLEDGEMENTS.md
   ```

3. Edit `attribution.toml` so `[rust].manifest_path` points at the Cargo
   manifest you ship (usually `crates/your-cli/Cargo.toml` rather than the
   workspace root, so dev-only deps stay out of the attribution).

4. Tune `about.toml`'s `accepted` list and `targets` to match your licence
   policy and the platforms you build for.

5. Generate:

   ```bash
   tools/starters/acknowledgements/generate-acknowledgements.sh
   ```

6. Wire CI: drop `ci-freshness.yml.snippet` into your existing workflow.

## Customising `ACKNOWLEDGEMENTS.md`

The template at `ACKNOWLEDGEMENTS.md.template` is a structural scaffold, not a
finished file. After copying it to your repo root, replace the `{{PLACEHOLDER}}`
tokens and prune the example sections to fit your stack.

### Placeholders

All placeholders use `{{DOUBLE_BRACES}}` so they are easy to grep and replace.
None of them are interpreted by the generator — they are plain text the template
author left for you to fill in.

| Placeholder                 | Replace with                                                                                                                 |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `{{PROJECT_NAME}}`          | Display name of your project, e.g. `Anvil`                                                                                   |
| `{{PROJECT_BINARY}}`        | The shipping binary or package name, e.g. `anvil`                                                                            |
| `{{GENERATOR_TOOL}}`        | The upstream tool that produces the auto-generated block, e.g. `cargo-about`                                                 |
| `{{GENERATOR_TOOL_URL}}`    | Upstream URL for the tool, e.g. `https://github.com/EmbarkStudios/cargo-about`                                               |
| `{{LOCKFILE_NAME}}`         | The lockfile the attribution derives from, e.g. `Cargo.lock` or `pnpm-lock.yaml`                                             |
| `{{GENERATOR_SCRIPT_PATH}}` | Path to the generator script as it appears in your repo, e.g. `tools/starters/acknowledgements/generate-acknowledgements.sh` |

A one-shot `sed` works for most of these:

```bash
sed -i \
  -e 's|{{PROJECT_NAME}}|Anvil|g' \
  -e 's|{{PROJECT_BINARY}}|anvil|g' \
  -e 's|{{GENERATOR_TOOL}}|cargo-about|g' \
  -e 's|{{GENERATOR_TOOL_URL}}|https://github.com/EmbarkStudios/cargo-about|g' \
  -e 's|{{LOCKFILE_NAME}}|Cargo.lock|g' \
  -e 's|{{GENERATOR_SCRIPT_PATH}}|tools/starters/acknowledgements/generate-acknowledgements.sh|g' \
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
   <!-- BEGIN AUTO-GENERATED -->
   <!-- END AUTO-GENERATED -->
   ```

   Each must appear **exactly once** on a line of its own. If you customise the
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
one** END marker, on lines of their own:

```markdown
<!-- BEGIN AUTO-GENERATED -->
<!-- END AUTO-GENERATED -->
```

The default markers are HTML comments so the file remains valid markdown and the
markers don't render in viewers. Marker text is overridable per project via
`[project].marker_begin` / `[project].marker_end` in `attribution.toml` (e.g.
for projects that grow multi-block markers under ATTRIB-008).

The generator matches markers via literal substring containment, not regex, so
the marker text need not be regex-safe.

### Idempotency

Running the generator twice in a row against an unchanged `Cargo.lock` produces
a byte-identical file the second time. The freshness gate (`--check`) relies on
this: it regenerates into a temporary file and `diff`s against the on-disk copy.

Idempotency requires:

- `cargo-about`'s render output is deterministic for a given `Cargo.lock` +
  template + config. Pin the `cargo-about` version in CI to keep this true
  across machines.
- Hand-edited content above the BEGIN marker and below the END marker is never
  rewritten. The splice loop emits those regions verbatim.

### Atomic write

The generator never writes the target file in place. It:

1. Generates `cargo-about`'s output to `mktemp` file A.
2. Splices A between the markers in the target, writing to `mktemp` file B.
3. `mv`s B over the target only after both prior steps succeed.

A failure mid-run leaves the target untouched. Combined with the empty- output
guard (next), this prevents a partial generation from silently clobbering
content.

### Strict license-field enforcement (ATTRIB-007)

The generator invokes `cargo about generate --fail`, so a workspace crate
missing both `license` and `license-file` in its `Cargo.toml` causes a hard
error rather than a silent warning. Without `--fail`, cargo-about emits a `WARN`
and exits zero, and the crate quietly drops out of the generated attribution — a
regression risk that ATTRIB-007 closes.

Consumers who deliberately ship crates without a licence (typically internal
private crates filtered out via `about.toml`'s `private.ignore`) should silence
the warning at the filter level rather than removing `--fail`. The filter is the
right boundary: cargo-about never tries to attribute filtered crates, so the
strict check doesn't fire against them.

The fixture test under `tests/strict-license-field.sh` pins this contract. CI
runs it alongside the freshness check.

### Single-source licence allow-list (ATTRIB-006)

The `accepted` array in `about.toml` and the `[licenses].allow` array in
`deny.toml` are generated from a single canonical `licences.toml` at the repo
root. The kit's `expand-licences.sh` reads `licences.toml` and rewrites the two
consumer arrays between BEGIN/END marker comments — the same splice pattern the
acknowledgements generator uses on `ACKNOWLEDGEMENTS.md`. CI runs the expander
in `--check` mode so drift between `licences.toml` and either consumer fails the
build.

To add or remove a licence: edit `licences.toml`, run
`tools/starters/acknowledgements/expand-licences.sh`, and commit all three files
together. The schema (single-line strings only) is documented at the top of
`licences.toml` itself.

The fixture test under `tests/licences-drift.sh` walks three scenarios — clean
expand → check passes, new licence in source → drift detected, hand-edit in
consumer → drift detected — so a regression that loosens the matcher is caught
in CI.

### Empty-output guard

If `cargo about generate` produces a zero-byte file (e.g. because the template
path is wrong, the manifest doesn't resolve, or cargo-about itself crashes
silently), the generator aborts with a non-zero exit before the `mv` step. The
target is never overwritten with an empty block.

### Marker-count gate

Before generation runs, the generator counts BEGIN and END marker occurrences in
the target. If either count is not exactly 1, it exits with a non-zero status
and an actionable error. This catches:

- A target file that's missing the marker block entirely.
- A merge conflict that duplicated the markers.
- A typo introduced when adding a new block.

Without this gate, the splice loop would no-op silently and `--check` would
falsely report "all good" while regeneration never happened.

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
value. The generator carries no hard-coded paths, markers, or fix-it strings.

```toml
[project]
target_path   = "ACKNOWLEDGEMENTS.md"        # required
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"  # required
# marker_begin = "<!-- BEGIN AUTO-GENERATED -->"  # optional; default shown
# marker_end   = "<!-- END AUTO-GENERATED -->"    # optional; default shown

[rust]
manifest_path = "crates/your-cli/Cargo.toml" # required
template_path = "about.hbs"                  # required
config_path   = "about.toml"                 # required
```

All paths are resolved relative to the directory containing `attribution.toml`.
Absolute paths are also accepted.

## Future evolution

The kit currently covers a single Rust block. The roadmap (see the
`attribution-pipeline-v3` APS module under `plans/modules/` in the upstream
`eddacraft/anvil` repository) plans multi-block markers (ATTRIB-008) for
additional ecosystems
(`<!-- BEGIN AUTO-GENERATED rust -->`, `<!-- BEGIN AUTO-GENERATED binaries -->`,
...). The marker-count gate is per-marker-text, so adding new blocks is additive
— existing single-block consumers don't need to change anything.
