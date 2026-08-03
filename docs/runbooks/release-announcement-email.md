# Release Announcement Email

| Type    | Authority     | Owner | Status | Freshness                                      |
| ------- | ------------- | ----- | ------ | ---------------------------------------------- |
| Runbook | Authoritative | API   | Live   | Authored 2026-07-26 for post-release beta mail |

| Upstream                                                                          | Downstream                                   |
| --------------------------------------------------------------------------------- | -------------------------------------------- |
| `POST /admin/broadcast`, `packages/transactional/emails/release-announcement.tsx` | Beta operators after a published release tag |

Send the release-announcement broadcast **after** the public GitHub release and
install paths are live. Do not pre-announce from a dry-run-only cut.

## Preconditions

- Admin credential configured (`anvil admin auth status`)
- Release tag published on `eddacraft/anvil` (and source evidence on
  `anvil-001`)
- Install paths verified for at least one platform you claim in the body
- Template props reviewed (version, theme, highlights, release URL)

## Audience

Prefer **active beta users** who can upgrade:

| Audience key         | Use                                      |
| -------------------- | ---------------------------------------- |
| `beta:active`        | Default for release notices              |
| `beta:active-recent` | Optional smaller cohort for a pilot send |

Do **not** send release announcements to `waitlist:pending` — they cannot
install as beta until invited.

## Procedure

### 1) Fill `templateProps`

Copy a props pack from [Props packs](#props-packs) below. Replace version and
URLs when the next cut ships. Subject line is derived as:

```text
anvil {version} — {theme}
```

### 2) Dry-run broadcast

```bash
# Shape depends on the admin CLI / raw API. Prefer the documented broadcast
# surface (see docs/architecture/api-as-built.md — POST /admin/broadcast).
# Example payload (JSON body):
```

```json
{
  "template": "release-announcement",
  "audience": "beta:active",
  "dryRun": true,
  "templateProps": {
    "version": "v0.9.0-beta",
    "theme": "First-Run Wins and the Assistant Graph",
    "intro": "…",
    "highlights": [],
    "releaseUrl": "https://github.com/eddacraft/anvil/releases/tag/v0.9.0-beta",
    "upgradeCommands": [
      { "label": "Homebrew", "command": "brew upgrade eddacraft/tap/anvil" },
      {
        "label": "curl installer",
        "command": "curl -fsSL https://install.eddacraft.ai | sh"
      },
      { "label": "WinGet", "command": "winget upgrade --id eddacraft.anvil" },
      { "label": "Scoop", "command": "scoop update anvil" }
    ],
    "feedbackEmail": "anvil@updates.eddacraft.ai"
  }
}
```

Expected: `previewToken`, recipient count, no sends.

### 3) Real send

Reuse the `previewToken` from the dry-run (drift protection). Confirm recipient
count with a human. Only then set `dryRun: false` and consume the token per the
broadcast contract.

### 4) Verify

- Spot-check Resend delivery for one known address
- `anvil admin audit --limit 20` for broadcast / migration-related actions
- Do **not** re-send in a loop; diagnose delivery first

## Failure And Recovery

| Failure                 | Recovery                                                              |
| ----------------------- | --------------------------------------------------------------------- |
| Dry-run cohort drift    | Re-run dry-run; do not force-send a stale preview                     |
| Resend not configured   | Fix `RESEND_API_KEY` on anvil-api; re-dry-run                         |
| Wrong audience          | No unsend — note in `#beta-ops`; correct next mail                    |
| Stale template defaults | Always pass explicit `templateProps` — do not rely on `V070_DEFAULTS` |

## Props packs

### v0.9.0-beta (catch-up if never broadcast)

Ready to send after confirming install paths still serve this tag (or point
`releaseUrl` / upgrade lines at the latest if you only want “current beta”).

```json
{
  "version": "v0.9.0-beta",
  "theme": "First-Run Wins and the Assistant Graph",
  "intro": "A new anvil release is live. Your first minute lands on a real finding in your own repository, a healthy repeat start collapses to a six-line confidence check, and the resident code graph is now something an AI assistant can query over MCP.",
  "highlights": [
    {
      "title": "A first win on your own code",
      "body": "First-run anvil welcome discovery lands on your repository's highest-severity actionable finding, shows the one-line diff, and applies only with explicit consent."
    },
    {
      "title": "Quiet repeat anvil start",
      "body": "When the repo is already activated and healthy, a repeat anvil start collapses to protection state, daemon posture, and exactly one next step."
    },
    {
      "title": "Assistant graph context over MCP",
      "body": "Search symbols, dependents, callers, change impact, and affected tests from Claude Code or Cursor — identity-only by default, source double-gated behind operator consent."
    },
    {
      "title": "Python and infrastructure hygiene",
      "body": "Python joins JS/TS and Rust for analysis and boundaries. New scans catch risky Docker, GitHub Actions, shell, and SQL migration patterns."
    },
    {
      "title": "Shareable value receipt",
      "body": "anvil insights --share writes a deterministic scorecard (counts and dates only — no paths or repo names)."
    },
    {
      "title": "Warm-start graph on by default",
      "body": "The save-time daemon persists a shared base graph per merge-base so restarts and sibling worktrees warm from disk. Opt out with ANVIL_PERSIST_GRAPH=0."
    }
  ],
  "releaseUrl": "https://github.com/eddacraft/anvil/releases/tag/v0.9.0-beta",
  "upgradeCommands": [
    { "label": "Homebrew", "command": "brew upgrade eddacraft/tap/anvil" },
    {
      "label": "curl installer",
      "command": "curl -fsSL https://install.eddacraft.ai | sh"
    },
    { "label": "WinGet", "command": "winget upgrade --id eddacraft.anvil" },
    { "label": "Scoop", "command": "scoop update anvil" }
  ],
  "feedbackEmail": "anvil@updates.eddacraft.ai"
}
```

Omit `firstInvocationNote`, `migrationUrl`, `knownGaps`, and `boringWeekAsk`
unless the cut has a real ask. Source narrative: `plans/releases/v0.9.0-beta.md`
and `CHANGELOG.md` `## [0.9.0-beta]`.

### Next release — v0.9.2-beta (fill at cut)

Active window per `RELEASE-PLAN.md`: **v0.9.2-beta** (MCP 2.0 reconnect patch;
not yet tagged). Headline is the MCP26-013 client reconnect fix; secondary
Unreleased bullets may ride along.

At cut:

1. Copy the block below.
2. Set `theme` from `RELEASE-PLAN.md` (active window) or, after cut, the release
   record created under `plans/releases/` (filename matches the tag).
3. Lead `highlights` with the Codex / MCP metadata reconnect fix; add at most
   one or two secondary bullets if they remain in the promoted changelog.
4. Point `releaseUrl` at the public tag.
5. Dry-run → send to `beta:active`.

```json
{
  "version": "v0.9.2-beta",
  "theme": "MCP 2.0 reconnect",
  "intro": "A patch is live so Codex and similar assistants connect to anvil again after the MCP 2.0 dual-era host change.",
  "highlights": [
    {
      "title": "Assistants reconnect",
      "body": "Clients that send normal progress metadata are accepted again; tool lists and tool calls work."
    }
  ],
  "releaseUrl": "https://github.com/eddacraft/anvil/releases/tag/v0.9.2-beta",
  "upgradeCommands": [
    { "label": "Homebrew", "command": "brew upgrade eddacraft/tap/anvil" },
    {
      "label": "curl installer",
      "command": "curl -fsSL https://install.eddacraft.ai | sh"
    },
    { "label": "WinGet", "command": "winget upgrade --id eddacraft.anvil" },
    { "label": "Scoop", "command": "scoop update anvil" }
  ],
  "knownGaps": [],
  "feedbackEmail": "anvil@updates.eddacraft.ai"
}
```

Theme placeholder matches `RELEASE-PLAN.md` active window; refresh if the cut
theme changes.

## Template defaults note

`packages/transactional/emails/release-announcement.tsx` still exports
`V070_DEFAULTS` for local email preview of the historical v0.7.0-beta mail.
**Production sends must pass explicit `templateProps`.** Do not rely on those
defaults for a current release.

## Related

- Admin CLI: `docs/runbooks/admin-cli.md`
- Waitlist email ops: `docs/runbooks/waitlist-email-operations.md`
- Broadcast design (archived module):
  `plans/archive/modules/email-broadcast.aps.md`
- API as-built: `docs/architecture/api-as-built.md`
