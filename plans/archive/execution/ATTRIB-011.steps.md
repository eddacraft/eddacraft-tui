# ATTRIB-011 — Public mirror of the acknowledgements starter kit

## Purpose

Land ATTRIB-011: make `tools/starters/acknowledgements/` consumable from
public projects (eddacraft-tui; future public repos) that
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
| Auth | Fine-grained PAT scoped to `eddacraft/acknowledgements-starter` (`Contents: Read and write`), stashed in `mirror-push-token` in agent-vault, mirrored into the `MIRROR_PUSH_TOKEN` repo secret on `eddacraft/anvil-001` | **Deploy keys are disabled at the eddacraft org level** (HTTP 422 on `POST /repos/.../keys` confirmed 2026-05-17). PAT is the supported path; scope to one repo + minimum permissions keeps blast radius tight. Revisit with a GitHub App if more mirrors arrive. |
| Public README sourcing | Separate `MIRROR-README.md` inside the kit, swapped to `README.md` during the split | In-tree README is anvil-internal-flavoured (`git subtree add` examples from anvil-001); public consumers need standalone framing. |
| Divergence policy | Force-push, no merge | Mirror is downstream-only; PRs against the mirror are explicitly rejected via README + (later) issue template. |
| Mirror layout | **Flat-rooted** (kit files at the public repo's top level); consumers adopt with a per-kit prefix (`--prefix tools/starters/acknowledgements`) | Considered and rejected a wrap design (mirror root contains a single `acknowledgements/` subdirectory, consumers adopt with `--prefix tools/starters`). The wrap looked elegant — kit names itself, can't collide with siblings on file names — but `git subtree add` makes the **prefix directory** the tracked subtree. Adopting at `--prefix tools/starters` would lock the whole parent to one kit and preclude adding `--prefix tools/starters/logging` (or any other independently tracked starter) at the same level. Per-kit prefixes give each starter its own subtree at its own subdirectory; no collision possible because each kit owns a different prefix, and per-kit `subtree pull` updates only that kit. README discipline (insist on the per-kit prefix in the adoption doc) is the right enforcement layer, not a structural mirror change. |

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
    1. **Ref guard** — refuse to run unless `GITHUB_REF == refs/heads/main`,
       so `workflow_dispatch` from a feature branch or tag can't silently
       force-push the wrong tree.
    2. `actions/checkout@v4` with `fetch-depth: 0` (subtree split needs full
       history) and `persist-credentials: false` (the in-tree
       `GITHUB_TOKEN` isn't used; mirror auth is its own secret).
    3. Sanity-check the `MIRROR_PUSH_TOKEN` secret is non-empty (fail-fast
       with an actionable error if the operator forgot Action 3).
    4. Swap `MIRROR-README.md` → `README.md` inside the subdir on a
       throwaway commit (commit so the split picks it up). If the workflow
       was `workflow_dispatch`ed with a reason, include it in the throwaway
       commit message and the GitHub step summary.
    5. `git subtree split --prefix=tools/starters/acknowledgements -b _mirror_split`.
    6. `git push --force https://x-access-token:${MIRROR_PUSH_TOKEN}@github.com/eddacraft/acknowledgements-starter.git _mirror_split:main`.
  - Top-level `permissions: contents: read` keeps the default
    `GITHUB_TOKEN` least-privileged — all writes leave via the mirror PAT.
  - Concurrency group: cancel-in-progress so rapid pushes coalesce.
  - Workflow contract: add an entry to the Workflow Contract Map in
    `.github/workflows/README.md` (auxiliary contract; the mirror is
    outside the five core PR/Integration/Assurance/RC/Publish contracts).
    `scripts/ci/workflow-contracts.test.sh` enforces this.
- **Checkpoint:** Workflow lints clean (`actionlint` if available); dry-run
  on a feature branch with `workflow_dispatch` produces the expected split.

### 3. Create the public repo + wire auth (operator step — surfaces in PR)

- **Purpose:** Provision `eddacraft/acknowledgements-starter` empty and
  give the mirror workflow push credentials. Externally visible action;
  left to the operator rather than performed autonomously.
- **Produces:** Public repo with no initial README/licence (mirror push
  will populate). Fine-grained PAT created via
  <https://github.com/settings/personal-access-tokens/new> with:
  - Resource owner: `eddacraft`
  - Repository access: only `acknowledgements-starter`
  - Permissions → Repository → Contents: Read and write
  - Expiry: pick a horizon (90/180 days) and add a renewal reminder.

  Private half stashed in agent-vault under `mirror-push-token` and
  mirrored into the anvil-001 repo secret `MIRROR_PUSH_TOKEN`.
- **Checkpoint:** `gh repo view eddacraft/acknowledgements-starter`
  succeeds; `gh secret list --repo eddacraft/anvil-001 | grep MIRROR_PUSH_TOKEN`
  finds the secret.

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

### 6. First downstream consumption (eddacraft-tui)

- **Purpose:** Satisfy the APS validation criterion that one external
  project consumes the public mirror. eddacraft-tui (public Rust repo,
  ships its own CLI) is the natural first consumer — same Rust+cargo-about
  ingest path the kit was designed around, and being public it can pull
  directly from the mirror without the private-repo subtree obstacle that
  motivated ATTRIB-011 in the first place.
- **Produces:** PR against `eddacraft/eddacraft-tui` adopting the kit via
  `git subtree add --prefix tools/starters/acknowledgements
  https://github.com/eddacraft/acknowledgements-starter.git main --squash`
  (per-kit prefix — each starter kit is its own independently tracked
  subtree at its own subdirectory; a future `logging-starter` would go
  to `tools/starters/logging` etc., never sharing a subtree with this
  one). Plus the consumer-side bootstrap (`attribution.toml`,
  `about.toml`, `about.hbs`, `ACKNOWLEDGEMENTS.md`) and the CI freshness
  gate.
- **Checkpoint:** PR in eddacraft-tui either merged or open with a green
  `--check` from the kit's freshness gate proving the round-trip works.
- **Shipped:** eddacraft/eddacraft-tui#33, merged 2026-05-18. First
  adoption attempt landed at the wrong subtree prefix
  (`tools/starters/` rather than `tools/starters/acknowledgements/`),
  diagnosed as uutils coreutils 0.2.2's
  [#10508](https://github.com/uutils/coreutils/issues/10508) dirname
  bug truncating the multi-segment `--prefix`. Fixed locally by
  installing uutils 0.8.0 ahead of the system `/usr/bin/dirname`, then
  re-doing the adoption as a "relocate" PR.

### 7. Mark ATTRIB-011 Complete; bump module counter

- **Purpose:** Close the work item in the APS module + index.
- **Produces:** Status flips `In Progress` → `Complete` in
  `plans/modules/attribution-pipeline-v3.aps.md`; module done/total in
  `plans/index.aps.md` bumps `4/11` → `5/11`.
- **Checkpoint:** APS validation clean.
- **Shipped:** This PR (2026-05-18).

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Force-push wipes the public repo if someone commits there directly | Medium | README states "mirror, do not PR here"; later: add `BRANCH_PROTECTION` script disabling direct pushes |
| PAT compromise leaks write to the mirror only (not anvil-001) | Low | Fine-grained PAT scoped to exactly one repo, `Contents: write` only; rotation is regenerate-on-github + `gh secret set MIRROR_PUSH_TOKEN`. Set an expiry + calendar reminder. |
| Subtree split history diverges between runs of the workflow | Low | Force-push every time; mirror history is intentionally not stable |
| Public README references private paths by accident | Low | Action 1 owns the swap; Action 4 visually verifies the first run |
