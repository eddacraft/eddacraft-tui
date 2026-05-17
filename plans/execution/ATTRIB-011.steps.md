# ATTRIB-011 — Public mirror of the acknowledgements starter kit

## Purpose

Land ATTRIB-011: make `tools/starters/acknowledgements/` consumable from
public projects (Anvil VS Code extension; future public repos) that
cannot `git subtree` from the private `anvil-001` repo.

Canonical source stays in anvil-001. The public repo
(`eddacraft/acknowledgements-starter`) is a **read-only mirror** — forced
to match the subdirectory snapshot on every change to `main` that touches
the kit.

## Decisions (recorded on kickoff, 2026-05-17)

| Question | Decision | Rationale |
| --- | --- | --- |
| Public repo name | `eddacraft/acknowledgements-starter` | Already proposed in the APS module; matches `eddacraft/anvil` naming. |
| Visibility | Public | Whole point of ATTRIB-011 — unblock public consumers. |
| Mirror mechanism | Scheduled-on-change `git subtree split` → force-push to mirror `main` | One-shot is brittle; subtree split is the canonical Git primitive; no third-party action dependency. |
| Trigger | `push` to `main` with path filter `tools/starters/acknowledgements/**`, plus `workflow_dispatch` | Avoid running on every unrelated push; manual override available for forced re-sync. |
| Auth | Deploy key on the mirror repo, private half in `MIRROR_DEPLOY_KEY` secret on anvil-001 | PAT would tie the mirror to a user account; deploy key is scoped to the one repo. |
| Public README sourcing | Separate `MIRROR-README.md` inside the kit, swapped to `README.md` during the split | In-tree README is anvil-internal-flavoured (`git subtree add` examples from anvil-001); public consumers need standalone framing. |
| Divergence policy | Force-push, no merge | Mirror is downstream-only; PRs against the mirror are explicitly rejected via README + (later) issue template. |

## Actions

### 1. Draft the public-facing README (MIRROR-README.md)

- **Purpose:** Standalone framing for external consumers — no mention of
  anvil-001 paths, no `git subtree add --prefix tools/starters/...` example
  pointing at a private repo.
- **Produces:** `tools/starters/acknowledgements/MIRROR-README.md`. Covers:
  what the kit is, adoption (clone or `git subtree add` from the public
  mirror), pointer back to the APS module *as historical context only*
  (not as a code source).
- **Checkpoint:** README reads naturally for someone who has never seen
  anvil-001.

### 2. Author the mirror workflow

- **Purpose:** Reproducible split + push, idempotent, force-push safe.
- **Produces:** `.github/workflows/mirror-acknowledgements-starter.yml`
  - Triggers: `push` on `main` filtered to
    `tools/starters/acknowledgements/**` and the workflow file itself;
    `workflow_dispatch` for manual re-sync.
  - Steps:
    1. `actions/checkout@v4` with `fetch-depth: 0` (subtree split needs full
       history).
    2. Configure SSH using `MIRROR_DEPLOY_KEY` secret.
    3. Swap `MIRROR-README.md` → `README.md` inside the subdir on a temp
       branch (commit so the split picks it up).
    4. `git subtree split --prefix=tools/starters/acknowledgements -b _mirror_split`.
    5. `git push --force git@github.com:eddacraft/acknowledgements-starter.git _mirror_split:main`.
  - Concurrency group: cancel-in-progress so rapid pushes coalesce.
- **Checkpoint:** Workflow lints clean (`actionlint` if available); dry-run
  on a feature branch with `workflow_dispatch` produces the expected split.

### 3. Create the public repo (operator step — surfaces in PR)

- **Purpose:** Provision `eddacraft/acknowledgements-starter` empty.
  Externally visible action; left to the operator rather than performed
  autonomously.
- **Produces:** Public repo with no initial README/licence (mirror push
  will populate). Deploy key created with **write** access; public half
  committed nowhere, private half pasted into anvil-001 secret
  `MIRROR_DEPLOY_KEY`.
- **Checkpoint:** `gh repo view eddacraft/acknowledgements-starter` succeeds;
  Settings → Deploy keys shows one key with write access.

### 4. First mirror run

- **Purpose:** Validate the workflow end-to-end against the just-provisioned
  public repo.
- **Produces:** First commit on `eddacraft/acknowledgements-starter:main`
  matching the current contents of `tools/starters/acknowledgements/` with
  `MIRROR-README.md` renamed to `README.md`.
- **Checkpoint:** Public repo `main` content matches local
  `tools/starters/acknowledgements/` except for the README swap.

### 5. Document the mirror in the kit README

- **Purpose:** Internal contributors (and the next agent) should know
  the public mirror exists and how to force re-sync.
- **Produces:** Short "Mirror" section in
  `tools/starters/acknowledgements/README.md` linking to the public repo
  and the workflow.
- **Checkpoint:** `markdownlint` clean; cross-references resolve.

### 6. First downstream consumption (Anvil VS Code extension)

- **Purpose:** Satisfy the APS validation criterion that one external
  project consumes the public mirror.
- **Produces:** Either a tracking note in the VS Code extension repo or
  (if reach allows) a PR there adopting the kit via `git subtree add` from
  the public mirror.
- **Checkpoint:** External consumer either landed or recorded as queued
  with a concrete next step.

### 7. Mark ATTRIB-011 Complete; bump module counter

- **Purpose:** Close the work item in the APS module + index.
- **Produces:** Status flips `In Progress` → `Complete` in
  `plans/modules/attribution-pipeline-v3.aps.md`; module done/total in
  `plans/index.aps.md` bumps `4/11` → `5/11`.
- **Checkpoint:** APS validation clean.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Force-push wipes the public repo if someone commits there directly | Medium | README states "mirror, do not PR here"; later: add `BRANCH_PROTECTION` script disabling direct pushes |
| Deploy-key compromise leaks write to the mirror only (not anvil-001) | Low | Scope is exactly one public repo; key rotation is `gh repo deploy-key delete + add`, no impact to private code |
| Subtree split history diverges between runs of the workflow | Low | Force-push every time; mirror history is intentionally not stable |
| Public README references private paths by accident | Low | Action 1 owns the swap; Action 4 visually verifies the first run |
