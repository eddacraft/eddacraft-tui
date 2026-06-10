# Post-Install Experience Polish — Implementation Plan

**Goal:** Replace the bare post-install message in `install.sh` with a branded output matching the TUI identity (brandmark, tagline, version, watermark).
**Architecture:** Pure shell — a colour-detection helper sets ANSI variables, the post-install block uses them for logo/tagline/watermark styling. Version is detected via `anvil --version` with graceful fallback.
**Tech Stack:** POSIX sh (no bashisms — install.sh targets `#!/bin/sh`)

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `install.sh` | Modify | Add colour helper, version detection, branded post-install block |

---

### Task 1: Add colour detection helper

**Files:**
- Modify: `install.sh`

- [ ] Add `setup_colours()` function after `set -e` (line 14)
  - Sets `EMBER`, `BOLD`, `DIM`, `RESET` ANSI variables when colour is supported
  - Leaves them empty when `NO_COLOR` is set, `TERM` is `dumb`/unset, or stdout is not a tty
- [ ] Test manually: `NO_COLOR=1 sh install.sh` should produce no ANSI escapes
- [ ] Commit: `feat(install): add colour detection helper`

**Code:**

```sh
# Colour support — disabled when NO_COLOR is set, stdout is not a tty, or TERM is dumb.
setup_colours() {
  EMBER="" BOLD="" DIM="" RESET=""
  if [ -n "${NO_COLOR:-}" ]; then return; fi
  if [ "${TERM:-dumb}" = "dumb" ]; then return; fi
  if ! [ -t 1 ]; then return; fi
  EMBER='\033[38;2;204;85;0m'
  BOLD='\033[1m'
  DIM='\033[2m'
  RESET='\033[0m'
}
setup_colours
```

---

### Task 2: Add version detection

**Files:**
- Modify: `install.sh`

- [ ] Add version detection after the cargo-dist installer runs successfully (after the exit-code check block, before the post-install message)
  - Run `anvil --version 2>/dev/null` and extract the version string
  - If it fails, set a fallback message without a version number
- [ ] Commit: `feat(install): detect installed version`

**Code (inserted after the `exit "$INSTALL_EXIT"` block):**

```sh
# Detect installed version — may fail if PATH not yet updated in this shell.
ANVIL_VERSION=""
if command -v anvil >/dev/null 2>&1; then
  ANVIL_VERSION=$(anvil --version 2>/dev/null | head -1 | sed 's/^[^0-9]*//')
fi
if [ -n "$ANVIL_VERSION" ]; then
  VERSION_LINE="  anvil v${ANVIL_VERSION} installed successfully!"
else
  VERSION_LINE="  anvil installed successfully!"
fi
```

---

### Task 3: Replace post-install block with branded output

**Files:**
- Modify: `install.sh`

- [ ] Replace lines 60–66 (the current bare echo block) with the branded output
  - Brandmark in ember orange with "a n v i l" in ember+bold
  - Tagline in dim
  - Version line (from Task 2)
  - Next steps in default colour
  - URL in dim
  - Eddacraft watermark in dim
- [ ] Test: run `sh install.sh` in a truecolor terminal — verify logo is orange, tagline/watermark are dim
- [ ] Test: run `NO_COLOR=1 sh install.sh` — verify no ANSI escapes in output
- [ ] Commit: `feat(install): branded post-install message`

**Code (replaces the current echo block):**

```sh
printf "\n"
printf "  ${EMBER}████         ████${RESET}\n"
printf "  ${EMBER}██             ██${RESET}\n"
printf "  ${EMBER}██  █████████  ██${RESET}\n"
printf "  ${EMBER}██     ███     ██${RESET}   ${EMBER}${BOLD}a n v i l${RESET}\n"
printf "  ${EMBER}██  █████████  ██${RESET}\n"
printf "  ${EMBER}██             ██${RESET}\n"
printf "  ${EMBER}████         ████${RESET}\n"
printf "\n"
printf "  ${DIM}Structural governance for AI-assisted development${RESET}\n"
printf "\n"
printf "%s\n" "$VERSION_LINE"
printf "\n"
printf "  Get started:\n"
printf "    cd your-project/\n"
printf "    anvil start\n"
printf "\n"
printf "  Or run anvil --help for all commands.\n"
printf "  ${DIM}https://eddacraft.dev/docs${RESET}\n"
printf "\n"
printf "                        ${DIM}[ ■ ] e d d a c r a f t${RESET}\n"
printf "\n"
```

---

## Execution Order

Tasks 1–3 are sequential (each builds on the previous). Single file, single commit path — can be squashed into one commit if preferred:

```
feat(install): branded post-install message with colour support
```
