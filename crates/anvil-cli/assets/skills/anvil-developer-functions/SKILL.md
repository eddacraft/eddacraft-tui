---
name: anvil-developer-functions
description:
  'Use Anvil MCP developer functions during code changes: inspect repository
  structure with read-only graph tools, validate proposed writes before editing,
  and apply approved patches. Trigger for implementation, refactoring,
  debugging, and review work in an Anvil-enabled repository.'
---

# Anvil developer functions

Use Anvil as a safety and structural-context layer while changing code. Anvil
does not replace the editor, shell, test runner, or engineering judgement.

## Required loop

1. Inspect the repository before editing. Prefer the Anvil graph tools when
   their answer is relevant; use ordinary read-only tools when it is not.
2. Prepare the exact proposed content or unified diff.
3. Call `anvil_validate_write` before every file-creation or whole-file write.
   For a unified diff against an existing file, prefer `anvil_apply_patch`.
4. Respect the returned decision:
   - `block`: do not write or bypass the result through another tool.
   - `warn`: surface the diagnostics, then continue only when the requested work
     remains appropriate.
   - `gateUnavailable`: surface that protection was unavailable; the decision is
     informational and the change may continue.
   - `allow`: apply the proposed change.
5. Run the repository's narrowest relevant verification and report the fresh
   evidence. Do not claim a write was protected merely because configuration
   exists; live validation is distinct from installation and handshake.

Repeat steps 2–4 for each material write. If the content changes after
validation, validate the changed content again.

## Structural context

Use `anvil_search_symbols` to find definitions. Before changing a public
contract or shared component, use `anvil_find_dependents` and
`anvil_find_callers` to inspect inbound impact. Use `anvil_symbol_context` for a
bounded structural neighbourhood and `anvil_affected_tests` to identify likely
verification targets.

Do not force graph tools into tasks they do not answer. Continue with normal
repository inspection when an Anvil tool returns no evidence or the server is
unavailable.

## Tool details

Read [references/tool-reference.md](references/tool-reference.md) when preparing
tool arguments, interpreting diagnostics, or handling an unavailable gate.
