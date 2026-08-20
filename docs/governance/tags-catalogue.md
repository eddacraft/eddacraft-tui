# Documentation Tags Catalogue

| Type  | Authority     | Owner  | Status | Freshness                                                                                                  |
| ----- | ------------- | ------ | ------ | ---------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | DOCGOV | Live   | Last reviewed 2026-08-21 against ADR-123, `docs/guides/documentation-governance.md`, and `pnpm docs:check` |

| Upstream                                                                                           | Downstream                                                                                               |
| -------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `plans/archive/modules/documentation-governance.aps.md`, `docs/guides/documentation-governance.md` | `scripts/docs/check-tags.mjs`, `scripts/docs/docs-check.mjs`, generated indexes, DOCGOV-008 cleanup pass |

## Purpose

The approved set of tag values that may appear in `**Tags:** ...` lines inside
APS work items (`plans/**/*.aps.md`). New tags MUST be added here in the same PR
that introduces them; `pnpm docs:check` errors on a malformed tag and warns on a
tag that is not in this catalogue.

This catalogue exists because tags are a controlled vocabulary, not a free write
surface: agents and humans both reach for tags as a filtering and discovery
surface, and without a catalogue the vocabulary drifts every time a new module
is drafted. The seed values below were derived from a 2026-05-12 audit of every
`Tags:` occurrence under `plans/**`.

This catalogue is the only authority. `pnpm aps:drift` does not check tags; the
APS package does not enforce them; `packages/aps/src/types/index.ts` declares
them as `string[]` with no enum.

## Catalogue

### Domain — features and product areas

| Tag            | Intent                                      |
| -------------- | ------------------------------------------- |
| `agent`        | Agent runtime, harness, or behaviour        |
| `aps`          | APS spec, parser, validator, or rules       |
| `architecture` | Cross-cutting structural concerns           |
| `bmad`         | BMAD adapter and BMAD-v4 compatibility work |
| `cli`          | Command-line interface surface              |
| `core`         | Anvil core engine or kernel                 |
| `config`       | Configuration files, schemas, or resolution |
| `docs`         | Documentation governance, sync, or content  |
| `hooks`        | Git hooks, lifecycle hooks, or hook surface |
| `parser`       | Markdown, APS, or syntax parsers            |
| `templates`    | Template generation or rendering            |
| `testing`      | Test infrastructure or coverage work        |
| `tui`          | Terminal UI surface                         |
| `tutorial`     | Onboarding tutorial and walkthrough         |
| `v4`           | BMAD v4 line of work                        |

### Activity — what the work item does

| Tag             | Intent                                                   |
| --------------- | -------------------------------------------------------- |
| `conversion`    | Format conversion between two representations            |
| `design`        | Design exploration or RFC-style work                     |
| `detection`     | Detecting a state, condition, or event                   |
| `integration`   | Integration across packages, services, or upstream tools |
| `research`      | Investigative work without a fixed deliverable shape     |
| `serialization` | Encoding to disk, wire, or canonical text                |
| `verification`  | Verifying behaviour against an external expectation      |
| `workflow`      | Developer or agent workflow design                       |

### Platform — operational surface

| Tag              | Intent                                            |
| ---------------- | ------------------------------------------------- |
| `ci`             | CI pipeline, workflows, or job orchestration      |
| `command`        | A specific CLI command or subcommand              |
| `component`      | A reusable component within a surface             |
| `concurrency`    | Concurrent execution, locking, or scheduling      |
| `cross-platform` | Linux/macOS/Windows portability work              |
| `fields`         | Field-level schema or data-shape work             |
| `init`           | Initialisation, bootstrap, or first-run paths     |
| `runtime`        | Runtime behaviour, environments, or process model |
| `security`       | Security posture, signing, or threat surface      |
| `shell`          | Shell scripting, bash/zsh integration             |
| `storage`        | Persisted state, files, or databases              |
| `team`           | Multi-agent or multi-person workflows             |
| `windows`        | Windows-specific behaviour                        |

### Adapter ecosystem

| Tag       | Intent                          |
| --------- | ------------------------------- |
| `speckit` | SpecKit adapter and integration |

## Adding a Tag

1. Audit existing tag usage to confirm the new tag is not a near-duplicate of an
   existing entry (e.g. don't introduce `validate` if `verification` covers it).
2. Add a row to the most appropriate section above with a one-sentence intent.
3. Use the new tag in your APS work item.
4. Run `pnpm docs:check`. The catalogue is read live; the new tag should
   warn-free.

## Notes

- Tag matching is case-sensitive and uses the exact spelling above. Use
  kebab-case for compound tags.
- Tags are intentionally short — they are routing/filtering labels, not
  descriptions. If the tag needs a long name, it probably belongs in the APS
  task body, not the tag list.
- Eighteen tags from the 2026-05-12 audit were single-use and concentrated in
  archived BMAD-era modules. They are retained in this catalogue to keep
  historical APS files clean against the validator; if those modules are pruned
  in DOCGOV-008, the corresponding tags should be removed here in the same
  change.
- Tags from archived modules are not "stale" — the archive remains addressable.
  They simply do not get new uses.
