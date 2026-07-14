<!-- APS: Design spec for packaging Anvil-authored agent skills for customer distribution across agent harnesses -->

# Skill packaging & distribution across agent harnesses

Date: 2026-07-02; ratified 2026-07-14
Module: `SKPKG` (SKPKG-001)
Status: Accepted — owner-approved 2026-07-14 after fresh catalogue, code-env,
Anvil-Plan-Spec setup, product-code, and vendor-contract verification.
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

- A general-purpose skill marketplace or registry (SKOBS carries the same
  exclusion; revisit only if a real need surfaces)
- Runtime governance/enforcement of installed skills (AGOV territory)
- Redesigning `skill.meta.json` or the `eddacraft-skills` catalogue format —
  see "What already exists" below; this spec extends it, it doesn't replace it

## 2026-07-14 ratification

The owner approved shipping skills inside the beta product while keeping an
eventual OSS transition open. The executable contract is:

1. Vendor a pinned snapshot under `crates/anvil-cli/assets/skills/` and embed
   it with `include_str!`; builds never fetch private repositories.
2. `anvil skill install` offers detected harnesses and global/project scope,
   defaulting interactive beta installs to user-global.
3. Reuse the Anvil-Plan-Spec multi-select/direct-shortcut UX, improved with
   real executable/config detection, managed provenance, hash-based drift,
   idempotency, atomic file replacement, and symlink refusal.
4. Use one typed agent registry for detection and capability metadata. Skill
   discovery and MCP configuration are independent flags; neither implies the
   other is live.
5. Install portable content into the smallest verified roots: Claude Code's
   `.claude/skills`, Cursor's `.cursor/skills`, and shared `.agents/skills`
   where the target harness officially supports it.
6. The customer marker records schema version, skill name, Anvil version,
   catalogue commit, bundle digest, and per-file digests. Updates only replace
   a prior managed and unmodified install; unmanaged/user-modified content is
   refused.
7. Skill content follows the Anvil binary release cadence during beta.
8. Customer-readable installed Markdown is a proprietary operational asset,
   not product source and not a new OSS repository. ADR-018 is amended in the
   same change.

## What already existed at discovery time

This section records the evidence that informed the original draft; the
ratified contract above supersedes its implementation recommendations.
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

The discovery-time gap was narrow: **there was no customer-reachable path from "a
skill exists in the catalogue with `targets: {…}` declared" to "the skill is
materialised in a customer's project directory in the shape their harness
expects."** The beta reuses the portable Agent Skills shape but deliberately
does not depend on the unmerged `code-env` emitter or private catalogue during
build or installation.

## Design

### 1. Packaging artefact shape

Two lifecycle stages, kept distinct:

- **Authoring/catalogue stage**: `SKILL.md` plus `references/*.md` remains
  canonical in `eddacraft-skills`. Anvil vendors a reviewed commit snapshot
  under `crates/anvil-cli/assets/skills/`.
- **Distribution artefact (new)**: the customer-facing package embedded in
  the `anvil` binary at build time with `include_str!` over that vendored
  portable snapshot and
  materialised into the customer's machine by a new `anvil skill install`
  subcommand, sibling to `anvil mcp install`:

  ```text
  anvil skill install --client claude-code
  anvil skill install --client codex --scope project
  anvil skill install --client codex --verify
  ```

  This reuses the exact pattern `anvil mcp install` already established:
  binary-embedded content and repeatable `--client` selection. Global is the
  default; `--scope project` is explicit. No network call or catalogue access
  occurs.

- **Customer-facing manifest**: don't reuse the full `skill.meta.json` shape
  as-is on the customer's machine — `origin`, `localChanges`, and catalogue
  `status` are provenance for the *catalogue*, not useful to a customer.
  Write `.anvil-managed.json` beside the skill with schema version, name,
  catalogue commit, Anvil version, a bundle digest, and per-file SHA-256 hashes.
  Field names should mirror `SkillEntry` in
  [`skill-manifest-schema.md`](./skill-manifest-schema.md) rather than invent
  new vocabulary (see §5).

### 2. Harness portability

- **Portable** (same across every harness): the safe-change loop and tool
  reference, using Agent Skills-compatible frontmatter.
- **Harness-specific**: YAML frontmatter fields Claude Code uses for
  skill-matching (`name`, `description`), the `.claude/skills/<name>/SKILL.md`
  path convention, and Claude Code's own `Skill` tool invocation contract.
  Other harnesses (OpenCode, OpenClaw, Codex) have their own directory
  conventions.
- **Bridging**: the typed registry maps each verified client to its documented
  global/project skill root. The portable bundle is unchanged at install time;
  MCP support never implies skill discovery support.

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
`"anvil-bundled"`, so `/skill-inventory` can tell a
customer "this skill shipped with your `anvil` install" apart from one they
or a teammate authored. This is a one-field addition to an existing schema,
not a fork. The value and provenance fields are now recorded while SKOBS-002
remains Draft.

## Resolved questions

The questions raised by the original draft were resolved by owner approval on
2026-07-14 and are retained here for decision provenance:

- **OQ-1 (SKPKG-002):** Ratify the §4 ADR-018 IP-boundary call — does this
  need a full ADR addendum, or is Council/owner sign-off on this spec
  sufficient precedent? **Resolved:** ADR-018 was amended and ADR-106 records
  the installer architecture.
- **OQ-2 (SKPKG-003):** Skill content will likely need to iterate faster than
  full `anvil` CLI releases (a wording fix in `SKILL.md` probably shouldn't
  wait for the next binary release). Bundle-with-binary (§3) accepts that
  trade-off by default — is that acceptable, or does skill content need its
  own faster-moving channel layered on top? **Resolved:** beta follows the
  Anvil binary release cadence.
- **OQ-3 (SKPKG-004):** Reconcile the target-harness sets: the catalogue
  declares 4 targets (`claude`, `opencode`, `openclaw`, and `install.sh`'s
  `codex`), but `anvil mcp install`'s `McpClient` enum only has 2 (`cursor`,
  `claude-code`). Which set does `anvil skill install` support at first cut,
  and does `McpClient` need to grow to match, or do skill targets and MCP
  targets stay independent enums? **Resolved:** one identity registry owns
  independent capability fields.
- **OQ-4 (SKPKG-005):** Land the `SourceInfo.type: "anvil-bundled"` addition
  (§5) with the SKOBS module owner before SKOBS-002 goes Ready. **Resolved:**
  the schema is updated in this change.
- **OQ-5 (SKPKG-006):** `anvil mcp install` defaults to the user's home
  directory (finding 4) and treats project-scoping as an explicit
  `--workspace` override. Should `anvil skill install` default the same way,
  or default to project-scoped instead — a skill file is more naturally
  something a team commits and shares than per-user MCP client config is?
  Affects whether installed skills are gitignored-local or team-visible by
  default, and how SKOBS's machine/user/project scope model sees them.
  **Resolved:** offer both and default global.
- **OQ-6 (SKPKG-007):** Does embedding skill content in the `anvil` binary
  require a new build-time step that pulls `code-env`'s already-emitted
  per-harness output into the build, or does it embed the canonical
  `SKILL.md` directly and defer per-harness adaptation to install time? Not
  resolved here — flagged for whoever scopes the `anvil skill install`
  implementation. **Resolved:** embed the vendored portable snapshot directly.

## Decision

Accepted 2026-07-14. OQ-1: amend ADR-018. OQ-2: binary release cadence for
beta. OQ-3: one shared agent registry with independent capability flags. OQ-4:
use `anvil-bundled`. OQ-5: offer both scopes and default interactive installs
to user-global. OQ-6: vendor a pinned portable snapshot and embed it directly
with `include_str!`.
