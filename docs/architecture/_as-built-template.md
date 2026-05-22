# {Component Name} — As-Built

| Type     | Authority | Owner                                     | Status | Freshness                                                                                                                                 |
| -------- | --------- | ----------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| As-built | Derived   | MODULE-CODE (link to `plans/modules/...`) | Draft  | Last reviewed YYYY-MM-DD against tag/SHA `vX.Y.Z-beta` / `<short-sha>` and source paths listed in [Source references](#source-references) |

| Upstream                                    | Downstream                                                                 |
| ------------------------------------------- | -------------------------------------------------------------------------- |
| `crates/foo`, `apps/bar`, or `packages/baz` | {consumers — other crates, CLI surfaces, MCP tools, runbooks, public docs} |

## Overview

One paragraph framing the component: what it is, what problem it solves, where
it sits in the wider system. Resist the urge to teach the reader the whole
codebase here — link to neighbours instead.

## Architecture diagram

ASCII first. Boxes and arrows that match the lifecycle section below — not the
whole world.

```text
┌──────────┐         ┌──────────┐         ┌──────────┐
│ producer │────────▶│ component│────────▶│ consumer │
└──────────┘         └────┬─────┘         └──────────┘
                          │
                     ┌────▼────┐
                     │  store  │
                     └─────────┘
```

A mermaid diagram is optional and goes after the ASCII version, not instead of
it.

## Lifecycle / data flow

The actual sequence the component runs through, with code references for every
load-bearing claim.

1. {Trigger / entry point} — `crates/foo/src/lib.rs:NN`
2. {Step two — what happens, where} — `crates/foo/src/bar.rs:NN-MM`
3. {Step three — including the persistence or emit boundary} —
   `crates/foo/src/sink.rs:NN`

Keep the steps numbered and short. If a step has substeps that matter, indent
them; if they don't matter, leave them out.

## Surfaces

External APIs / CLI commands / MCP tools / IPC contracts that consumers use.
Group by surface kind. Include the stability level when it's load-bearing.

| Surface         | Kind     | Stability | Notes                                           |
| --------------- | -------- | --------- | ----------------------------------------------- |
| `anvil foo bar` | CLI      | beta      | flags documented in `docs/cli/foo.md`           |
| `POST /v1/foo`  | HTTP     | beta      | request/response in `crates/foo-api/src/dto.rs` |
| `foo.run`       | MCP tool | beta      | declared in `crates/foo-mcp/src/tools.rs:NN`    |

## Internals

Key invariants, state machines, and decisions worth documenting. This is the
section where the next maintainer learns why things are arranged the way they
are. One short subsection per invariant.

### Invariant: {name}

What it guarantees, why it exists, where it's enforced
(`crates/foo/src/guard.rs:NN`).

### State machine: {name}

States, transitions, terminal conditions. ASCII or mermaid. Reference the enum
or state struct directly (`crates/foo/src/state.rs:NN`).

## Known gaps

Things that don't work yet. Dated. Each with a tracked follow-up handle if one
exists. Every component has gaps — naming them is the value.

### G-01: {short title}

What's wrong or missing, the scope of impact, the workaround if any. Link to the
issue / APS work item / ADR that tracks the fix.

**Risk:** Low | Medium | High. **Fix:** {summary, or "tracked in #NNNN"}.

### G-02: {short title}

…

If the component genuinely has none today, write `None at time of review.`
rather than deleting the section.

## Source references

Bulleted list of the canonical files / modules in this component. Match the
style of `auth-as-built.md`'s "Source Files" table when there are more than a
handful of files.

- `crates/foo/src/lib.rs` — entry point
- `crates/foo/src/state.rs` — state machine
- `crates/foo/src/sink.rs` — emit boundary
- `crates/foo/Cargo.toml` — dependency surface

## Related docs

- Spec: `docs/specs/{...}.md`
- ADR: `plans/decisions/{NNN-...}.md`
- Runbook: `docs/runbooks/{...}.md`
- Public docs: `apps/docs-site/...` or `docs/cli/...`
- Module plan: `plans/modules/{MODULE-CODE}.aps.md`

---

## How to write one

1. **Cite source.** Every load-bearing claim gets a file reference, ideally
   line-pinned (`crates/foo/src/bar.rs:NN-MM`). If you can't point at code,
   you're writing a spec, not an as-built.
2. **Date it against a specific tag or SHA.** Code drifts. The doc has to say
   what it was true at, so the next reader knows whether to trust it.
3. **List source paths in backticks.** `pnpm docs:check` validates code-wrapped
   source paths in governed as-built documents, so keep real repository paths in
   the Source references section and avoid placeholder paths outside examples.
4. **Gaps section is mandatory.** Every component has gaps. Hidden gaps are the
   most expensive kind. Naming them is the deliverable.
5. **Keep the architecture diagram minimal.** Boxes and arrows that match the
   lifecycle section, not a re-draw of the whole product.
6. **Empty sections stay.** If a section is genuinely empty, write
   `None at time of review.` rather than deleting it — absence is signal.

**Reference implementation:** [`auth-as-built.md`](./auth-as-built.md) is the
working example to copy from. When in doubt, match its shape.
