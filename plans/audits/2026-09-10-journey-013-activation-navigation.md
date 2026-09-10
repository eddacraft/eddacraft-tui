# JOURNEY-013 — First-run activation navigation observation

**Date:** 2026-09-10 (AWST)
**Observed source:** `62e1facd70abd8de0311946e3fc6d4c401e9db7b` (`origin/main`)
**Help-bar commit:** `e586b6e53413e9efaa2faccd73b7aa44d99d998f` (`fix(tui): advertise arrows on anvil start help`, 2026-08-21) — confirmed ancestor of the observed SHA
**Platform:** Linux x86_64
**Claim:** #4577
**Decision:** **close as fixed** — do not design a splash; do not change the stepping affordance in this item

## Purpose

Re-measure first-run `anvil start` consent navigation against a build that
carries the contextual help bar, so a splash is not designed on the strength of
a session that ran `v0.9.7-beta` (no picker help at all). Record residual
difficulty against three failure modes, then decide: splash, stepping
affordance, or close as fixed.

## Method

Operator/agent observation of the **production render path** used by interactive
`anvil start`: `anvil_tui::shell::render_shell` wrapping
`ActivationSurface` in the Consent phase (the same `Surface::help_text` +
`render` pairing the CLI event loop draws). Not a shadowed human first-timer —
the original human session was against a build that lacked this chrome, and
this item's autonomous grant authorised the re-test against the fixed build.

Fixture: a typical first-run consent set with four sections (Project, Hooks /
git, Workflows, MCP clients), two MCP rows, all unticked (CIB-165). Naive-key
pass used only first-timer instincts (Enter, then ↑/↓) before following the
on-screen legend (←/→, space, `a`).

Provenance:

- `git merge-base --is-ancestor e586b6e53 62e1facd7` — true
- `cargo test -p eddacraft-anvil-tui --lib multi_section_consent_help` — pass
  on this SHA (`↑/↓ move  ←/→ next section  space toggle  a apply  esc/q quit`;
  `h`/`j`/`k`/`l` not advertised)

## First paint (80×24, production chrome)

Header: `[‡] anvil > Activation`

Phase strip: `Preflight > Working > [Consent] > Verdict > Done`

Picker title: `Consent — Project (1/4)`

Focused row: `▸ [ ] Project configuration  Anvil settings for this repository. …`

Framing under the box: `Files anvil wants to add to this repository.` /
`0 selected in total`

Footer help (muted, last row) plus watermark. The help string is 61 cells:

```text
↑/↓ move  ←/→ next section  space toggle  a apply  esc/q quit
```

At 100×24 and 120×24 the full string is visible. At 80×24 with the production
watermark (`[‡] a n v i l  v0.9.7-beta`, 26 cells) the footer keeps 52 cells
of help:

```text
↑/↓ move  ←/→ next section  space toggle  a apply  e
```

`a apply` survives. `esc/q quit` is truncated. Navigation, section stepping,
toggle, and apply remain on-screen at the POSIX 80-column default.

## Naive-key pass

| Action | What a first-timer is doing | What happened |
| ------ | --------------------------- | ------------- |
| First paint | Look at the boxed form | Help bar is already on the last row. Title shows `(1/4)`. Row is `[ ]`. Count is `0 selected in total`. |
| Enter | Confirm / continue instinct | Toggles the focused row (`[x]`, `1 selected in total`). Does **not** apply or leave Consent. Enter is not advertised on this pane (reserved for unsafe-drift confirm). |
| 6× Down | Look for more rows / next screen | Cursor wraps inside Project. Still `(1/4)`. Hooks / Workflows / MCP never appear. |
| Right arrow | Follow `←/→ next section` | Title becomes `Consent — Hooks / git (2/4)`. Project selection is preserved (`1 selected in total`). |
| Space | Follow `space toggle` | Ticks the focused Hooks row. |
| `a` | Follow `a apply` | Consent submits; the surface quits. |

`h`/`j`/`k`/`l` were not tried as missing keys. They remain silent aliases.

## Residual difficulty

### 1. Did not see the hint

**Not the residual.** The hint is on first paint, on every width observed, in
the same footer every other anvil surface uses. The `v0.9.7-beta` session had
**no** consent help bar; that gap is closed.

Possible miss: a user who never looks at the last row. The picker body does not
repeat the legend (ACTTUI-015 — overlapping chrome garbled small terminals).
80-column production truncates `esc/q quit` only; it does not hide the
navigation model.

### 2. Did not understand that arrows step between sections

**Low residual, not a splash.** The title carries `(1/4)` / `(2/4)` … and the
footer names `←/→ next section`. ↑/↓ wrap **inside** the current section, so a
user who only uses vertical arrows never leaves Project. That trap is real if
they ignore both the title counter and the footer. It is not the original
failure (no legend at all). Discovering the other sections from on-screen
chrome is now possible without a tutorial overlay.

### 3. Did not understand the picker is a form that needs a deliberate toggle

**Not the residual.** `[ ]` / `[x]`, `0 selected in total`, and `space toggle`
frame a multi-select form. Enter (unadvertised) already toggles, so the
confirm-instinct key does something visible rather than appearing dead.
Apply remains a separate `a`. A user who toggles then presses `q`/`esc` leaves
**without** applying — named by `a apply` on the bar, unusual versus Enter, but
already documented on-screen.

## Decision

**Close as fixed.**

- A splash / key-hint overlay / progress indicator would teach keys the footer
  already names. That is the expensive walk-back D1 was parked to avoid.
- Changing the stepping affordance (for example ↑/↓ crossing sections at the
  last row, or Enter applying) is a later product choice for JSIMP, not a
  JOURNEY-013 repair. This observation does not authorise that change.
- `h`/`j`/`k`/`l` stay silent aliases.

JSIMP-001 may consume the residuals above (in-section ↑/↓ wrap; `a` versus
Enter; 80-column quit truncation). It must not treat them as a licence to
build a splash.

## Non-scope kept

No splash, overlay, or progress indicator was designed or built. JSIMP was not
started.
