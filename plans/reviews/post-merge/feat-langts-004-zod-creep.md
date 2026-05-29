# Post-merge: feat-langts-004-zod-creep

PR: #2125
Branch: `feat/langts-004-zod-creep`
APS: LANGTS-004
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — `anvil check` (authenticated) on a file containing
      `z.any()` and a Zod `.passthrough()` reports AP-015 at `warning`;
      the same file's `z.object({ id: z.string() })` does not. Confirms
      the rule reaches the user-facing CLI surface (the PR's proof was the
      gate/scanner integration tests, because `anvil check` requires
      `anvil auth login` in CI/dev). (human required — needs an authed CLI)
- [ ] Step 2 — `anvil check --include-opt-in` (or the equivalent profile)
      on `z.unknown()` reports AP-016; the default run does not. (agent: yes
      — scanner test `zod_unknown_is_opt_in_only` already pins this; re-confirm
      at the CLI layer if an authed binary is available)
- [ ] Step 3 — Advance LANGTS-004 to `Released/Shipped` and the LANGTS
      module to `Complete` (6/6) once AP-015/AP-016 land in a release tag.
      Until then the module stays `In Progress` at 6/6 per the DISTRIB 5/5
      precedent. (agent: yes — on release evidence)

## Notes

This shipped two rules in the `type-system-evasion` family:
- **AP-015** (on by default, `warning`): `z.any()` + a Zod-anchored
  `.passthrough()`.
- **AP-016** (opt-in, off by default, `confidence: medium`): `z.unknown()`.

The Council split `z.unknown()` to opt-in because it is idiomatic as a
typed-record leaf and is the recommended `any` alternative, and the
antipattern scanner does not baseline — so an on-by-default rule would
warn on ~16 legitimate first-party uses. If later dogfooding shows teams
want `z.unknown()` strictness on by default, that is a deliberate
re-calibration (flip AP-016 `opt_in: false`), not a bug.

`.passthrough()` is anchored to a Zod receiver on the same line, so it
intentionally does **not** fire on a `.passthrough()` chained onto its
own line (e.g. `packages/edda-stack/src/config.ts`) or on non-Zod
`.passthrough()` methods. If a future AST-aware extractor lands
(tracked under the kernel-prereq work), revisit whether the own-line
chain should be caught.
