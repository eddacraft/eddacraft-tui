# GHOOK-001 — Git 2.54 baseline and rollout policy

## Purpose

Turn `GHOOK-001` into a concrete compatibility decision that unblocks native
config-hook work without forcing premature repo or product migration.

## Actions

### 1. Audit current hook assumptions

- **Purpose:** Establish where the repo and product currently assume Husky or file hooks.
- **Produces:** A verified inventory of hook-dependent surfaces and constraints.
- **Checkpoint:** Hook-dependent surfaces and assumptions are listed.
- **Validate:** `rg -n "husky|\.husky|core\.hooksPath|pre-commit|pre-push|git hook" crates docs package.json`

### 2. Define compatibility baseline

- **Purpose:** Decide the minimum Git capability needed for config-hook support and the fallback when it is absent.
- **Produces:** A written baseline covering contributors, CI, and end users.
- **Checkpoint:** Baseline names supported Git capability and fallback path.
- **Validate:** `git --version`

### 3. Define rollout policy

- **Purpose:** Prevent duplicate execution and unsupported defaults while config-hook support is introduced.
- **Produces:** Policy for coexistence, default install mode, and migration trigger.
- **Checkpoint:** Rollout policy covers coexistence, defaults, and migration trigger.

### 4. Publish the decision in docs

- **Purpose:** Make the compatibility position discoverable before implementation changes land.
- **Produces:** Updated guidance referenced from hook-related docs.
- **Checkpoint:** Hook guidance references the compatibility decision.
- **Validate:** `pnpm lint:md`
