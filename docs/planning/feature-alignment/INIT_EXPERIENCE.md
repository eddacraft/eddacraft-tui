# INIT_EXPERIENCE — `anvil init` (Exploratory + Descriptive)

## Purpose
Bootstrap Anvil on an existing repo without requiring plans or architecture expertise.

## Flow (v1-lite)
1. Build dependency graph.
2. Identify runtime entry points (routes/handlers/jobs/consumers/etc.).
3. Show descriptive view: entry points → internals map.
4. Offer candidate architecture models (if confidence allows); otherwise stay descriptive.
5. User selects model or “descriptive-only”.
6. Store baseline (model choice + baseline signature + drift snapshot).

Outputs:
- `.anvil/architecture.json` (schema-versioned)
- CLI command to re-render maps on demand
