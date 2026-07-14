# Anvil MCP tool reference

## Write validation

### `anvil_validate_write`

Use for new files and whole-file writes.

- `filePath`: workspace-relative destination path.
- `proposedContent`: exact content to be written.
- `intent`: short explanation of the requested change.

The response decision is `allow`, `warn`, `block`, or `gateUnavailable`.
Diagnostics may include rule identifiers, locations, messages, and repair
guidance. A `block` is authoritative for that proposed content.

### `anvil_apply_patch`

Use for a unified diff against existing files. Supply the complete unified diff
and a short intent. The tool validates added lines and returns a decision; it
does not write the file. Apply the patch through the normal editing mechanism
only after an `allow` or permitted `warn`. Do not apply it through a different
mechanism after a `block`.

## Graph context

### `anvil_search_symbols`

Search for symbol definitions and references. Use a focused query such as a
type, function, command, or module name. Treat an empty result as absence of
graph evidence, not proof that the repository contains no matching text.

### `anvil_find_dependents`

Find files that statically depend on a target file. Use it before changing a
shared interface to identify downstream impact.

### `anvil_find_callers`

Find static callers of a symbol. Results can be heuristic or partial; preserve
those qualifiers when reporting them.

### `anvil_impact_of_change`

Report affected symbols, dependent files, and known tests for changed paths. Use
it to shape a focused verification plan, not as proof of complete coverage.

### `anvil_affected_tests`

Suggest likely tests for changed files and identify known coverage gaps.

### `anvil_symbol_context`

Return a bounded structural neighbourhood around a symbol or file. Source
snippets require both workspace egress consent and `includeSource: true`;
identity-only context is the safe default.

## Failure handling

- If the MCP server is not configured, tell the user which capability is
  unavailable and continue with normal read-only repository inspection.
- If the server cannot start or handshake, do not describe the repository as
  protected.
- If a live validation returns `gateUnavailable`, report that exact state. It is
  not equivalent to `allow`, but it does not itself block the write.
- Never include secrets in tool arguments, diagnostics, examples, or logs.
