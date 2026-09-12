# ADR-146: Consent uses space to tick and Enter to confirm

## Status

Accepted — 2026-09-13 (operator). Supersedes [ADR-145](145-continuous-command-journey.md)
§8's freeze of the start-consent key legend. Does not reopen JOURNEY-013's
no-splash rule.

## Date

2026-09-13

## Context

Wizard and git-hooks TUIs already use `space` to tick and `enter` to
confirm/next. Tutorial first-win and `anvil start` consent instead advertised
`a apply` and `←/→ next section`. First-run users (beta screenshot
`error-bleed.png`, 2026-09-12) did not discover right-arrow, and treated Enter
as confirm — which only toggled the focused row.

ADR-145 §8 froze that legend so JSIMP would not rewrite stepping. The operator
has now asked to standardise on the wizard model.

## Decision

On activation consent and first-win:

- **Space** toggles a selectable row. On an unsafe-drift row it opens the
  confirm overlay.
- **Enter** confirms the current screen: next section, or apply on the last
  section (and apply on first-win). Empty apply writes nothing (CIB-165).
- **`a`** and **←/→** remain silent aliases. `h`/`j`/`k`/`l` stay silent via
  KeyHandler.
- The help bar advertises `space toggle` / `enter next` or `enter apply`. It
  does not advertise `a` or section arrows.

## Rationale

Enter is the confirm instinct. Requiring a dedicated apply letter and a
horizontal arrow for the next group of checkboxes fights that instinct. Keeping
`a` and arrows as aliases avoids breaking muscle memory without teaching two
models.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Chosen: space tick, Enter next/apply | Matches wizard/hooks; no extra letter | Last-section Enter applies even if later rows are unticked (same as `a` today) |
| Keep ADR-145 §8 | No behaviour change | Continues the first-run trap |
| Flatten to one screen | No section step | CIB-245 grouped steps for a reason |

## Consequences

- **Positive:** One checkbox form language across welcome and start.
- **Negative:** Help-bar tests and any scripted `a`/arrow walks need updating.
- **Risks:** Users who learned `a apply` in v0.9.7-beta. Mitigation: silent alias.
- **Mitigations:** CIB-165 unticked default is unchanged.

## References

- Related ADRs: ADR-145 §8, ADR-103, CIB-165, CIB-245
- Operator evidence: GH #4655, `Projects/tmp/anvil-beta/error-bleed.png`
