---
'eddacraft-anvil': minor
---

**Breaking change — `anvil doctor --json` output shape**

The root of `anvil doctor --json` changed from a bare JSON array of check
objects to a JSON object `{ "checks": [...], "notifications": [...] }`.

- `checks[]` holds the existing per-check payload (name, category, status,
  message, details, auto_fixable).
- `notifications[]` is a new canonical-taxonomy payload that aligns with
  `anvil check --json` and `anvil gate --json`.

**Migration:** change `data.map(c => ...)` to `data.checks.map(c => ...)`.
Consumers that ingest notifications can use `data.notifications[]`, which has
the same envelope (`class`, `priority`, `title`, `message`, `context`) as other
Anvil commands.

`anvil audit --json` also gained a `notifications[]` field. The audit change is
additive — existing fields are preserved — so no migration is required.
