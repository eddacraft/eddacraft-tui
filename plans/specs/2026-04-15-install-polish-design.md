# Post-Install Experience Polish

**Date:** 2026-04-15
**Branch:** fix/TUI
**Supersedes:** DD-D in `2026-04-12-welcome-fix-design.md`

## Problem

When the cargo-dist installer finishes, the user sees:

```
  Get started:
    cd your-project/
    anvil start

  Or run anvil --help for all commands.
```

This is functional but bare. Premium CLIs (rustup, starship, bun) use the
post-install moment as a brand impression — logo, version confirmation, clear
next steps. Ours skips the brand entirely and looks generic.

## Design

### Brand moment

Reuse the canonical TUI brandmark (from `welcome/render.rs` and
`onboarding/welcome_render.rs`) in the shell output:

```
  ████         ████
  ██             ██
  ██  █████████  ██
  ██     ███     ██   a n v i l
  ██  █████████  ██
  ██             ██
  ████         ████
```

The brandmark and "a n v i l" print in ember orange (`\033[38;2;204;85;0m`) on
terminals that support truecolor. Graceful fallback to plain (no colour) when
`NO_COLOR` is set, stdout is not a tty, or `TERM=dumb`.

### Version confirmation

After the logo, print the installed version by running `anvil --version` and
extracting the version string. If the command fails (not yet on PATH), fall back
to "installed successfully" without a version number.

### Tagline

Below the logo: `Structural governance for AI-assisted development` — same
tagline used in the TUI welcome screen.

### Next steps

```
  Get started:
    cd your-project/
    anvil start

  Or run anvil --help for all commands.
  https://eddacraft.dev/docs
```

### Footer watermark

```
                        [ ■ ] e d d a c r a f t
```

Matching the TUI footer (`shell.rs` watermark). Printed in muted/dim
(`\033[2m`) with the same colour fallback rules as the logo.

### Full output (colour terminal)

```
  ████         ████
  ██             ██
  ██  █████████  ██
  ██     ███     ██   a n v i l
  ██  █████████  ██
  ██             ██
  ████         ████

  Structural governance for AI-assisted development

  anvil v0.3.x installed successfully!

  Get started:
    cd your-project/
    anvil start

  Or run anvil --help for all commands.
  https://eddacraft.dev/docs

                        [ ■ ] e d d a c r a f t
```

### Colour strategy

| Element        | Colour                          | Fallback      |
|----------------|---------------------------------|---------------|
| Brandmark      | Ember `\033[38;2;204;85;0m`     | Plain text    |
| "a n v i l"    | Ember, bold                     | Bold          |
| Tagline        | Dim `\033[2m`                   | Plain text    |
| Version line   | Default                         | Default       |
| Next steps     | Default                         | Default       |
| URL            | Dim                             | Plain text    |
| Watermark      | Dim                             | Plain text    |

Colour is disabled when any of:
- `NO_COLOR` env var is set (any value)
- stdout is not a tty (`! [ -t 1 ]`)
- `TERM` is `dumb` or unset

### Error path

On install failure, keep the existing guidance (Homebrew fallback). The brand
moment only appears on success.

## Files Changed

- `install.sh` — replace the post-install echo block (lines 60-66) with the
  branded output, add colour helper function, add version detection

## Scope

This spec covers only the `install.sh` post-install message (Issue #2 / DD-D
from the welcome-fix spec). It does not change the cargo-dist installer itself,
the TUI, or any Rust code.
