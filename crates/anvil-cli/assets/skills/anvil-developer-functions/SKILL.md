---
name: anvil-developer-functions
description:
  Use anvil's MCP graph-context tools and pre-write validation gate when
  exploring or editing code in an anvil-enabled workspace, and before any file
  write.
---

# anvil developer functions

anvil exposes a set of developer functions over MCP. They fall into two halves
that you use together:

- **Graph-context tools** (read-only) answer questions about the codebase —
  symbols, dependents, callers, blast radius, affected tests — so you reason
  from a map instead of reading whole files. They return context, never a
  decision, and never block.
- **`anvil_validate_write`** / **`anvil_apply_patch`** are the pre-write
  enforcement gate for edits. Call them _before_ applying a write; they return a
  decision based on secrets, anti-patterns, and boundary rules.

Rule of thumb: reach for the **graph-context tools** when you are trying to
_understand_ code, and for the **pre-write gate** when you are about to _change_
it. Prefer **`anvil_apply_patch`** (or `anvil_validate_write` with `patch` only)
for edits; use full `proposedContent` for creates. They are complementary with
graph tools — pull context to plan an edit, then validate before writing.

These tools are an accelerator, not a constraint on your reasoning. Prefer them
over blind file reads: a graph query is cheaper, bounded, and deterministic.

## Companion skills

| Need                                          | Skill                                        |
| --------------------------------------------- | -------------------------------------------- |
| In-session graph + pre-write MCP validation   | **this skill** (`anvil-developer-functions`) |
| Setup, CLI check/gate/watch, CI, light config | `using-anvil`                                |
| Custom policy packs / Rego authoring          | Current anvil product documentation          |

If anvil is not installed, protection state is unclear, or the user asks about
`anvil check` / `anvil gate` / CI / `.anvilrc` / architecture YAML, use
**`using-anvil`**. Custom Rego packs and PolicyInput design are outside this
retained skill set; use only current product and repository documentation, and
do not infer schemas or commands. This skill assumes anvil MCP is (or should be)
available and focuses on the agent edit loop.

## Prerequisites

The tools come from the `anvil mcp serve --stdio` server, backed by the local
anvil daemon. MCP install covers the multi-client registry (Claude Code, Cursor,
Codex, OpenCode, Gemini CLI, VS Code, Copilot CLI, Grok Build, Warp, Zed, and
others — run `anvil mcp install --help` for the ids on your binary). Attachment
depth varies by client; verify the connection before relying on the tools.
Before relying on the tools:

- The MCP server must be wired into your client. Interactive `anvil start`
  offers every registry client (unticked by default) and starts the daemon;
  scripted installs use `anvil start --mcp-client <id>`,
  `anvil start --all-mcp-clients`, or `anvil mcp install --client <id>`
  (examples: `--client claude-code`, `--client cursor`, `--client codex`).
  Restart or reload the client's MCP servers afterwards. For activation states,
  doctor, and broader setup, see **`using-anvil`**.
- **Verify the connection** before the first real call: read `graph://stats`, or
  call `anvil_search_symbols` with any short name. A `ready` result (even with
  zero counts) confirms the wiring. If you get `not_ready`, retry shortly — the
  graph is warming. If you get `unavailable`, the daemon is not running: check
  `anvil intercept status`, then `anvil start`.

If the tools are not present at all, do not stall the task — fall back to
ordinary file reads and note that anvil's developer functions were unavailable.
For install and protection-state repair, hand off to **`using-anvil`**.

## The Developer Acceleration Loop

Run these steps in order around a code change. Steps 1–4 are the **understand**
path — use them on their own to answer "how does this work / who uses this" with
no edit in mind. Add step 5 to make it the **edit** loop.

### 1. Ask the graph before reading files

Locate the symbol or file first instead of grepping or opening files
speculatively.

- **`anvil_search_symbols`** — find symbols by name (case-insensitive
  substring), kind, file, language, or visibility. The entry point for "where is
  X?".
- **`anvil_find_dependents`** — the files that import a given file (its blast
  radius), each tagged with a hop distance (`1` = direct importer, `2` =
  transitive).

### 2. Get impact before editing

Before you touch a file, learn what depends on it.

- **`anvil_impact_of_change`** — given changed file _paths_ (never diff content;
  up to 200 files), returns the affected symbols, the depth-bounded set of
  dependent files, and a best-effort `known_tests` subset. Answers "what might
  break if I change these?".
- **`anvil_find_callers`** — the symbols that call a given symbol. A best-effort
  static over-approximation: it flags overload fan-out as `heuristic` and an
  incomplete walk as `partial`, and cannot see dynamic dispatch — do not treat
  it as an authoritative caller set.

### 3. Get affected tests before validating

- **`anvil_affected_tests`** — given changed file paths, the test files that
  import them (with their evidence edges) plus `coverage_gaps` (changed non-test
  files with no test importer). Relevance is an import heuristic
  (`heuristic: true`), not execution-verified coverage — run the tests it names,
  and treat `coverage_gaps` as a prompt to add or check tests, not proof of
  absence.

### 4. Use bounded symbol context instead of whole files

- **`anvil_symbol_context`** — the one-call "tell me about this symbol" tool.
  Given a seed symbol or file it returns neighbourhood symbols, one-hop
  importers, and (for symbol seeds) direct callers, each with a
  span-as-location, under a token budget. Prefer it to reading an entire file
  when you only need to understand one symbol's surroundings. It is also the
  only tool that can return source snippets — opt-in only (see
  [Privacy](#privacy-identity-only-by-default)).

### 5. Validate writes before applying them

Prefer the **smallest complete** validation unit:

1. **`anvil_apply_patch`** + `unifiedDiff` — edits as a diff (scans added
   lines). Preferred lean path.
2. **`anvil_validate_write`** with **`patch` only** — full post-image after
   in-memory apply (no disk write).
3. **`anvil_validate_write`** + full **`proposedContent`** — creates, or when
   patch construction is wrong/unavailable.
4. **`preview` + `contentSha256`** — **partial** scan only (`partialScan`); not
   the quality default.

Response `decision`:

- **`block`** (and other vetoes) — authoritative. Do not write. Surface findings
  and fix them.
- **`warn`** — findings present but enforcement lets the write proceed. Surface
  them and continue.
- **`gateUnavailable`** / backend error — surface the warning and proceed per
  existing recovery.
- **`allow`** — it passed; apply the write. **`decision` alone is
  authoritative** on allow. By default the envelope is minimal (schema +
  decision only); empty diagnostics / summary / correlation may be omitted.

Optional `detail: "minimal" | "full"` (default **minimal**) and env
`ANVIL_MCP_VALIDATE_DETAIL` control envelope size; they never change scan
quality. Use `detail: "full"` when you need correlation, claim, or tier.

A worked safe-refactor sequence ties the loop together:

1. `anvil_search_symbols` `{ name: "parsePayload" }` → locate the symbol and its
   file.
2. `anvil_impact_of_change` `{ changedFiles: ["src/parse.ts"] }` → affected
   symbols and dependent files.
3. `anvil_affected_tests` with the same `changedFiles` → the tests to run and
   any `coverage_gaps`.
4. Make the edit, then `anvil_apply_patch` (or patch-only
   `anvil_validate_write`) before writing.

## Tools at a glance

| Tool                     | Use it to…                                           | Watch out for                                                       |
| ------------------------ | ---------------------------------------------------- | ------------------------------------------------------------------- |
| `anvil_search_symbols`   | Find where a symbol is                               | Substring match is case-insensitive; paginates                      |
| `anvil_find_dependents`  | File-level blast radius (who imports this file)      | Caps the walk at 2 hops                                             |
| `anvil_find_callers`     | Symbol-level callers (who calls this function)       | Over-approximation; `heuristic`/`partial`; no dynamic dispatch      |
| `anvil_impact_of_change` | What breaks if I change these files                  | Paths only, never diffs; ≤200 files; 2-hop depth                    |
| `anvil_affected_tests`   | Which tests to run; coverage gaps                    | Import heuristic, not verified coverage                             |
| `anvil_symbol_context`   | Understand one symbol without reading the whole file | Snippets are opt-in; can return `bounded` partials                  |
| `anvil_validate_write`   | Check a write before applying it                     | Prefer patch/apply_patch; honour `block`; `decision` alone on allow |
| `anvil_apply_patch`      | Lean pre-write check of a unified diff               | Scans added lines only; same decision table as validate_write       |

For clients that prefer MCP resources to tool calls, three identity-only
`graph://` resources expose the resident graph directly — `graph://stats`
(counts, the lowest-risk warm-up probe), `graph://symbols`, and `graph://edges`.
See [`references/tool-reference.md`](references/tool-reference.md) for
parameters and scoping.

## Reacting to graph outcomes

A graph tool may return a named outcome instead of a result. Handle each:

- **`ready`** — readable; results follow (possibly empty). A `stale` flag still
  carries results.
- **`not_ready`** — warming or cold. Recoverable: the request triggers an
  on-demand warm-up, so retry — seconds for an incremental warm-up, up to ~a
  minute for a first full scan of a large workspace.
- **`unavailable`** — daemon unreachable or no graph. Not fixed by retry: check
  `anvil intercept status`; `anvil start` restarts it and re-triggers warming.
- **`disabled`** — an operator switched the surface off (`ANVIL_GCTX_EGRESS=0`).
  Respect it; do not retry.
- **`invalid_query`** — rejected before any read (e.g. path traversal,
  scheme-prefixed input, over-cap parameter). The `reason` says what to fix;
  correct the parameters and retry.

`anvil_symbol_context` can additionally return **`bounded`** (token budget
truncated the slice) or **`budget_exceeded`** (per-session snippet byte ceiling
hit) — both carry partial results, not errors. To get a complete response,
narrow the scope or disable snippets.

## Privacy: identity-only by default

Every tool is **identity-only** by default — it returns the _shape_ of the code
(names, kinds, workspace-relative paths, edges), never source text, absolute
paths, secrets, or operator identity. You do not need to do anything to stay
safe here.

Source **snippets** are an explicit opt-in and only `anvil_symbol_context` can
return them. They are double-gated: the operator enables egress for the
workspace (`anvil gctx egress enable`) **and** the request sets
`includeSource: true`. If you want snippets and egress is off, the tool returns
a `snippetEgressHint` telling you how to ask the operator to enable it — surface
that to the user rather than retrying. Do not advise setting
`ANVIL_GCTX_EGRESS=0` as a "safe default": that takes the whole surface offline.
The safe default is leaving it unset.

Full egress mechanics, the redaction pipeline, and exhaustive per-tool
parameters live in
[`references/tool-reference.md`](references/tool-reference.md).
