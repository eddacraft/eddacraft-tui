# Agent instructions — acknowledgements starter kit

Read this before adopting this kit, regenerating notices, or editing attribution
files. The contract is [README.md](./README.md). Follow it; do not invent a
parallel procedure.

This file lives in the kit directory. On the public mirror it is the repo-root
`AGENTS.md`. After a subtree add it is typically
`tools/starters/acknowledgements/AGENTS.md`. `KIT` below means this directory.

## Your job

Operate the kit. Copy templates, edit consumer config, expand the allow-list,
generate, and wire `--check` in CI. Do **not** edit generator or driver scripts
to make a consumer repo work.

## Adopt (first copy)

Follow the README adoption checklist in order. The load-bearing steps agents
skip:

1. Subtree-add (or copy) **this directory** at a **per-kit** prefix. Never
   `--prefix tools/starters` — that locks the parent to this one kit.
2. Copy the templates. Do not rewrite them.
3. Edit **one** file to describe the repo: `attribution.toml`. One `[[blocks]]`
   entry per ecosystem you ship. Comment out examples you do not use. Do not mix
   `[[blocks]]` with a flat `[rust]` table in the same file.
4. Copy only the per-driver files for ecosystems you ship. A Node-only repo does
   **not** need `about.toml` or `about.hbs`.
5. Replace every `{{PLACEHOLDER}}` in `ACKNOWLEDGEMENTS.md`. **`{{BLOCK_NAME}}`
   is required** — put the block `name` (`node`, `rust`, …). Leaving
   `{{BLOCK_NAME}}` in a marker is not a managed marker; the count-gate error
   names it.
6. Edit `licences.toml` with SPDX ids only, then run `KIT/expand-licences.sh`.
7. Install that ecosystem's scanner (the README names them). Then run
   `KIT/generate-acknowledgements.sh`.
8. Wire `ci-freshness.yml.snippet` into CI. Uncomment only the ecosystems you
   declared.
9. Ship `ACKNOWLEDGEMENTS.md` with the artefact you redistribute. Do not ship
   this kit directory inside the npm package or crate.

If `attribution.toml` is not at the repo root, pass `--config <path>` on every
generator and expander invocation. The consumer's `[project].fixit_command` is
the command to tell humans and CI to run.

If the adopting repo has a root `AGENTS.md`, add one line pointing at
`KIT/AGENTS.md` so later sessions find these instructions.

`jq` must be on `PATH`. `ATTRIB_DRIVERS_DIR` is a test hook — leave it unset.

## Keep it fresh

After lockfile, manifest, or allow-list changes:

```bash
KIT/expand-licences.sh # only if licences.toml changed
KIT/generate-acknowledgements.sh
KIT/expand-licences.sh --check
KIT/generate-acknowledgements.sh --check
```

`--check` writes nothing. Exit 0 means no drift; exit 1 is a real failure (fix
and regenerate); exit 2 is a bad invocation. `--check` and `--output` are
mutually exclusive.

Do not hand-edit the generated table to make `--check` pass.

## Allow-list change

1. Add or remove the SPDX identifier in `licences.toml`. Not Trove names
   (`Apache Software License`, `Mozilla Public License 2.0 (MPL 2.0)`). Python's
   driver maps SPDX → classifier names itself.
2. Run `KIT/expand-licences.sh`.
3. Commit `licences.toml` **and** every generated consumer file that changed
   (`about.toml`, `deny.toml`, `licences.*-allow.txt`).
4. Regenerate `ACKNOWLEDGEMENTS.md`.

Never hand-edit the generated region of those consumer files.

## New block / retire a block

New: add `[[blocks]]`, add exactly one BEGIN/END pair in the target whose suffix
is that block's `name`, copy that ecosystem's templates if missing, generate.

Retire: delete the `[[blocks]]` entry **and** both marker lines (and the body
between them) in the same change. An orphaned pair fails `--check`.

## Hand-curated content

Everything outside the marker pairs is permanent. Intro, Thanks, link references
— edit those freely; the generator preserves them verbatim. Do not move,
duplicate, or reverse the marker pair. Each pair is exactly one BEGIN line and
one END line, on lines of their own, BEGIN before END.

## Do not

- Edit `generate-acknowledgements.sh`, `expand-licences.sh`, or `drivers/*` in a
  consumer repo. A first-copy failure is a kit bug — report it.
- Invent a second config file. `attribution.toml` is the only project-specific
  input.
- Put timestamps or other non-deterministic text inside generated regions.
- Set Cargo `include` and `exclude` together. Prefer
  `exclude = ["tools/starters/**"]` so `cargo package` does not ship the kit.
- Link from kit markdown to paths outside this directory (the public mirror is
  this directory alone).
- Change `VERSION` without a matching newest `CHANGELOG.md` heading
  (`## [X.Y.Z] - <date>`). Docs-only kit edits do not require a version bump;
  they ride the rolling mirror.

## Verify

- `KIT/generate-acknowledgements.sh --check` → 0
- `KIT/expand-licences.sh --check` → 0 (when the consumer has `licences.toml`)
- After kit-source edits: `bash KIT/tests/run-all.sh`. A new file under `tests/`
  must also be named in the runner's `TESTS` array.

When a command fails, read stderr. The generator names the gate (marker count,
order, orphan, empty output, missing tool, disallowed licence). Fix that cause.
Do not stub output, skip `--check`, or relax the gate.
