<!-- APS: Design spec for packaging Anvil-authored agent skills for customer distribution across agent harnesses -->

# Skill packaging & distribution across agent harnesses

Date: 2026-07-02
Module: `SKPKG` (SKPKG-001)
Status: Draft — **Parked 2026-07-02**, see the module's `## Notes`. Review
defects (self-review + Copilot PR review on #3072) have been fixed below;
still needs a fresh re-verification of "What already exists" against
`eddacraft-skills` current state before owner review on resume.
Coordinates with:
[`plans/modules/skill-packaging-distribution.aps.md`](../modules/skill-packaging-distribution.aps.md),
[`plans/modules/skill-discovery-observability.aps.md`](../modules/skill-discovery-observability.aps.md),
[`plans/specs/skill-manifest-schema.md`](./skill-manifest-schema.md),
[`plans/decisions/018-product-ip-architecture.md`](../decisions/018-product-ip-architecture.md),
[`crates/anvil-cli/src/commands/mcp.rs`](../../crates/anvil-cli/src/commands/mcp.rs)

## Goal

Define how `anvil-developer-functions` (`.claude/skills/anvil-developer-functions/`,
PR [#3064](https://github.com/eddacraft/anvil-001/pull/3064)) — and future
customer-facing skills — go from "file living in this monorepo" to "artefact
a customer can install into their own project, for whichever agent harness
they use", without inventing a second packaging system where one already
exists.

## Non-goals

- Building any packaging tooling (this is a design; implementation is
  follow-on SKPKG work items)
- A general-purpose skill marketplace or registry (SKOBS carries the same
  exclusion; revisit only if a real need surfaces)
- Runtime governance/enforcement of installed skills (AGOV territory)
- Redesigning `skill.meta.json` or the `eddacraft-skills` catalogue format —
  see "What already exists" below; this spec extends it, it doesn't replace it

## What already exists (read this before proposing anything new)

Before this spec, the working assumption was that skill packaging needed to
be designed from scratch. It doesn't — most of it already exists, just not
wired to a customer-reachable distribution channel:

1. **A catalogue-side packaging manifest already ships**: every skill under
   `.claude/skills/<name>/skill.meta.json` in this repo (and canonically in
   the private `joshuaboys/eddacraft-skills` repo) carries `origin`,
   `status`, `targets: {claude, opencode, openclaw}`, `agentSupport`, and
   `localChanges`. `anvil-developer-functions`'s own `skill.meta.json`
   already declares `targets: {claude: true, opencode: true, openclaw: true}`
   and an `agentSupport.notes` field that says, verbatim, "Customer-facing
   skill teaching an AI coding agent to use anvil's MCP developer functions."
   The intent to be cross-agent and customer-facing was already declared at
   authoring time — this spec is about making that declaration true.

2. **A cross-agent emission pipeline already exists**: per
   `eddacraft-skills/README.md`, "`code-env` selects assets from here and
   emits assistant-specific target trees for Claude Code, OpenCode, and
   OpenClaw." Target-specific layout is explicitly *not* canonical in the
   catalogue — the canonical `SKILL.md` is harness-neutral prose, and
   `code-env` is the adapter layer that emits per-harness output. This is the
   harness-portability answer for skills already flowing through the
   catalogue; it is not yet exercised for `anvil-developer-functions`
   (`origin.type` is `"local"` with `lastImportedCommit: null` — it was
   authored directly in `anvil-001`, not round-tripped through the catalogue
   + emission pipeline).

3. **A cross-agent install script already exists**: `eddacraft-skills/install.sh`
   auto-detects the target agent (`claude-code|openclaw|codex|opencode`) from
   files present in the target project (`.claude/settings.json`/`CLAUDE.md`,
   `.openclaw/`, `.codex/settings.json`, `.opencode/settings.json`) and
   installs the right shape. **This script is the existing answer to
   "cross-agent install" — it is not, today, a customer distribution
   channel**, because:

   - `eddacraft-skills` is a **private** GitHub repo (confirmed via
     `gh repo view joshuaboys/eddacraft-skills` → `isPrivate: true`).
   - `install.sh`'s `ensure_catalog()` fetches the catalogue tarball via
     `gh api repos/$CATALOG_REPO/tarball/$CATALOG_REF` (needs an
     authenticated `gh` with org access) or falls back to
     `curl … raw.githubusercontent.com/…` (404s for anyone without repo
     read access). A customer running this script gets nothing.
   - This is exactly what you'd expect: `install.sh` is eddacraft's *internal*
     tool for provisioning eddacraft engineers' and agents' own machines
     across the repos eddacraft works in. It was never meant to reach past
     the org boundary, and per ADR-018 (below) it should not be widened to.

4. **A binary-distribution precedent already exists for reaching *into* a
   customer's machine**: `anvil mcp install --client <cursor|claude-code>`
   (`crates/anvil-cli/src/commands/mcp.rs`) ships inside the closed `anvil`
   binary and writes MCP server config for the customer. **Correction (caught
   in review): its default target is the user's home directory, not the
   project** — `McpInstallArgs.workspace` defaults to
   `default_client_config_root()`, which resolves to `user_home_dir()`
   (`crates/anvil-cli/src/commands/mcp_config.rs:217-219`); writing into the
   project directory requires the caller to pass `--workspace` explicitly.
   No source disclosure, no access to any eddacraft-internal repo required —
   the customer just runs their already-installed `anvil` binary. This is the
   one piece of the puzzle that is genuinely customer-reachable today, and it
   only covers MCP server wiring, not skill files. Its target-harness
   coverage is also narrower than the catalogue's: `McpClient` is
   `{cursor, claude-code}` — of which only `claude-code` overlaps with the
   catalogue's declared targets (`{claude, opencode, openclaw}` +
   `install.sh`'s `codex`); `cursor` isn't a catalogue target at all. §1's
   proposal needs to pick an install scope (home vs. project — see the new
   Open Question OQ-5) rather than assume "customer's project" the way an
   earlier draft of this spec did.

So the actual gap is narrow: **there is no customer-reachable path from "a
skill exists in the catalogue with `targets: {…}` declared" to "the skill is
materialised in a customer's project directory in the shape their harness
expects."** Everything upstream of that (authoring format, manifest schema,
cross-agent emission) already exists and should be reused as-is.

## Design

### 1. Packaging artefact shape

Two lifecycle stages, kept distinct:

- **Authoring/catalogue stage (unchanged)**: `SKILL.md` (frontmatter + prose)
  + `references/*.md` + `skill.meta.json`, canonical in `eddacraft-skills`,
  emitted per-harness by `code-env`. No change proposed here.
- **Distribution artefact (new)**: the customer-facing package embedded in
  the `anvil` binary at build time (e.g. `include_str!`/`rust-embed` over the
  per-harness emitted output, or over the canonical `SKILL.md` if a
  build-time emission step is added — see the new Open Question OQ-6) and
  materialised into the customer's machine by a new `anvil skill install`
  subcommand, sibling to `anvil mcp install`:

  ```text
  anvil skill install --client claude-code   # writes .claude/skills/anvil-developer-functions/
  anvil skill install --client claude-code --workspace .   # project-scoped, mirrors mcp install's override
  anvil skill list                           # what's bundled in this anvil version, per target
  ```

  This reuses the exact pattern `anvil mcp install` already established:
  binary-embedded content, `--client` selection, and — per the correction in
  finding 4 above — the same **home-directory-by-default, `--workspace`-to-
  override-to-project** scope, not an assumed project-write. No network
  call, no catalogue-repo access. Whether a *skill* (as opposed to MCP
  config) should default the other way — project-scoped, since a skill file
  is more naturally something a team shares via version control — is exactly
  the kind of call this design shouldn't make unilaterally; logged as Open
  Question OQ-5.

- **Customer-facing manifest**: don't reuse the full `skill.meta.json` shape
  as-is on the customer's machine — `origin`, `localChanges`, and catalogue
  `status` are provenance for the *catalogue*, not useful to a customer.
  Write a smaller marker file (proposed: `.claude/skills/<name>/.anvil-skill.json`
  — or fold into a `sizeBytes`/`contentHash`-style entry SKOBS' scanner can
  already read) carrying `{name, version, sourceCommit, installedVia: "anvil skill install", anvilVersion}`.
  Field names should mirror `SkillEntry` in
  [`skill-manifest-schema.md`](./skill-manifest-schema.md) rather than invent
  new vocabulary (see §5).

### 2. Harness portability

- **Portable** (same across every harness): the prose body — the "Developer
  Acceleration Loop" content, the reference doc. This is just markdown.
- **Harness-specific**: YAML frontmatter fields Claude Code uses for
  skill-matching (`name`, `description`), the `.claude/skills/<name>/SKILL.md`
  path convention, and Claude Code's own `Skill` tool invocation contract.
  Other harnesses (OpenCode, OpenClaw, Codex) have their own directory
  conventions and, in at least OpenCode's/OpenClaw's case, their own
  frontmatter shape — this is exactly what `code-env`'s emission step already
  handles for every other skill in this repo's `.claude/skills/` tree (most
  of which show `opencode: true` in their `skill.meta.json`, proving the
  adapter exists and is exercised today).
- **Bridging for a second harness**: no new adapter logic needs to be
  designed here — reuse `code-env`'s existing emission step to produce the
  OpenCode (or OpenClaw) shape of `anvil-developer-functions` from the same
  canonical `SKILL.md`, the same way it already does for
  `planning-workflow`, `aps-loop`, and the other skills in this tree that
  came from `eddacraft-skills` via `git` origin rather than `local` origin.
  The concrete gap for THIS skill is that it was authored with `origin.type: "local"`
  and never round-tripped through that pipeline (see finding 2 above) — the
  first real step is running it through, not designing a new one.

### 3. Versioning & update model

Two existing precedents, deliberately not a third:

- **DISTRIB** (`anvil version --check`, minisign-verified binary updates):
  since §1 embeds skill content in the `anvil` binary, the skill version
  **is** the `anvil` binary version. `anvil skill install` always installs
  "the skill as shipped in the running anvil version." Updates arrive via
  the existing `anvil update` path — no second update channel to build,
  secure, or explain to customers.
- **ATTRIB-017** (semver-tagged GitHub Release + changelog on a public
  mirror, immutable pin): the closer analogue if skill content ever needs to
  update *faster* than CLI releases (see Open Question OQ-2) — but not
  proposed as the default, because it would mean customers pull skill
  content from a second, independently-updating source, which is a second
  trust surface DISTRIB and `anvil update` don't need today.

**Recommendation**: bundle-with-binary (DISTRIB-aligned) is the default;
flag the update-cadence mismatch as an explicit open question rather than
deciding it here (OQ-2).

### 4. ADR-018 IP-boundary call

**This spec makes the call explicitly, per the SKPKG-001 work item
requirement, and flags it for owner/ADR sign-off rather than deciding it
silently:**

Customer-distributed skills are **closed-product distribution artefacts,
not a fourth OSS-surface repo.** They ship embedded in the closed `anvil`
binary (§1) and are written to a customer's machine only via
`anvil skill install`, never via direct access to the private
`eddacraft-skills` catalogue. This:

- Preserves ADR-018's binary-only, no-source-disclosure distribution model
  exactly as `anvil mcp install` already does for MCP config — no new
  precedent needed, just a second use of the existing one.
- Does not require carving a fourth OSS repo alongside `eddacraft-tui`,
  `anvil-plan-spec`, and `kindling`. None of ADR-018's three OSS-surface
  criteria (network effects from an adopted open standard, a trust signal
  for foundations, a genuine external contribution surface) obviously apply
  to a single customer-facing skill the way they do to a protocol
  (`anvil-plan-spec`) or a widget library (`eddacraft-tui`).
- Does **not** mean the skill's content is secret — a customer can read
  `SKILL.md` once it's installed on their machine, same as they can read
  `anvil --help` output. What stays closed is the *catalogue repo* (which
  holds a mix of customer-facing and eddacraft-internal skills side by side)
  and the emission tooling, not the installed artefact itself.

This is a new application of ADR-018 to a new artefact type (agent skills),
not a new architecture decision — but because it sets precedent for future
customer-facing skills, it should be ratified explicitly rather than left as
one design doc's opinion. Logged as SKPKG-002 (below).

### 5. Relationship to SKOBS's manifest (SKOBS-002)

[`skill-manifest-schema.md`](./skill-manifest-schema.md) defines
`SourceInfo.type` as `"local" | "symlink" | "copied"`. A skill materialised
by `anvil skill install` is none of these — it didn't arrive by hand-editing,
symlinking, or copying; it was written by the `anvil` binary from embedded
content. Recommend extending `SourceInfo.type` with a fourth value,
`"anvil-bundled"` (naming TBD — see OQ-4), so `/skill-inventory` can tell a
customer "this skill shipped with your `anvil` install" apart from one they
or a teammate authored. This is a one-field addition to an existing schema,
not a fork — flagged back to the SKOBS module (SKOBS-002 is still Draft, not
yet Ready, so there's time to land this before it locks in) rather than
decided unilaterally here, since SKOBS owns that schema.

## Open Questions

Per SKPKG-001's Expected Outcome, these are logged as Draft follow-on work
items in [`skill-packaging-distribution.aps.md`](../modules/skill-packaging-distribution.aps.md)
rather than silently resolved:

- **OQ-1 (SKPKG-002):** Ratify the §4 ADR-018 IP-boundary call — does this
  need a full ADR addendum, or is Council/owner sign-off on this spec
  sufficient precedent? Needs an owner decision.
- **OQ-2 (SKPKG-003):** Skill content will likely need to iterate faster than
  full `anvil` CLI releases (a wording fix in `SKILL.md` probably shouldn't
  wait for the next binary release). Bundle-with-binary (§3) accepts that
  trade-off by default — is that acceptable, or does skill content need its
  own faster-moving channel layered on top?
- **OQ-3 (SKPKG-004):** Reconcile the target-harness sets: the catalogue
  declares 4 targets (`claude`, `opencode`, `openclaw`, and `install.sh`'s
  `codex`), but `anvil mcp install`'s `McpClient` enum only has 2 (`cursor`,
  `claude-code`). Which set does `anvil skill install` support at first cut,
  and does `McpClient` need to grow to match, or do skill targets and MCP
  targets stay independent enums?
- **OQ-4 (SKPKG-005):** Land the `SourceInfo.type: "anvil-bundled"` addition
  (§5) with the SKOBS module owner before SKOBS-002 goes Ready.
- **OQ-5 (SKPKG-006):** `anvil mcp install` defaults to the user's home
  directory (finding 4) and treats project-scoping as an explicit
  `--workspace` override. Should `anvil skill install` default the same way,
  or default to project-scoped instead — a skill file is more naturally
  something a team commits and shares than per-user MCP client config is?
  Affects whether installed skills are gitignored-local or team-visible by
  default, and how SKOBS's machine/user/project scope model sees them.
- **OQ-6 (SKPKG-007):** Does embedding skill content in the `anvil` binary
  require a new build-time step that pulls `code-env`'s already-emitted
  per-harness output into the build, or does it embed the canonical
  `SKILL.md` directly and defer per-harness adaptation to install time? Not
  resolved here — flagged for whoever scopes the `anvil skill install`
  implementation.

## Decision

Not yet ratified — this spec is Draft pending owner review, particularly of
§4 (IP boundary) and OQ-1/OQ-2. On acceptance, SKPKG-001 moves to Done and
SKPKG-002..007 (from Open Questions) get filed as Draft work items in the
SKPKG module.
