# security-and-quality — usage

Four invocation variants. Same five-phase pipeline; only the active tracks change. Load `SKILL.md` after choosing a variant. Track fetch, classify, and execution: `references/alert-tracks.md`.

## Variants

| Invocation                       | Slash-menu alias                 | Tracks                          |
| -------------------------------- | -------------------------------- | ------------------------------- |
| `/security-and-quality`          | `/security-and-quality`          | Dependabot only (default)       |
| `/security-and-quality quality`  | `/security-and-quality-quality`  | Code Quality (CodeQL + Copilot) |
| `/security-and-quality scanning` | `/security-and-quality-scanning` | Code scanning SAST              |
| `/security-and-quality --all`    | `/security-and-quality-all`      | Dependabot + quality + SAST     |

Aliases accepted as arguments: `quality` / `--quality`; `scanning` / `sast` / `--scanning` / `--sast`; `--all`.

Slash-menu aliases are the `commands/` wrappers. They load this skill by name with the matching track already selected, so they work from this directory or from a runtime `commands/` copy.

## Optional filters

Append to any variant:

- severity — `/security-and-quality high`, `/security-and-quality-scanning medium`
- ecosystem (Dependabot) — `/security-and-quality npm`
- tool (scanning) — `/security-and-quality scanning tool:CodeQL`
- one alert — `/security-and-quality #82`

## What each variant does

**Dependabot (default).** Open dependency and GitHub Actions vulnerability alerts. Exit cleanly when none are open.

**Quality.** GitHub Code Quality CodeQL findings plus Copilot AI quality. Same-file fixes only. If Code Quality is unavailable, continue with Copilot.

**Scanning.** CodeQL security alerts and other SAST tools on the code-scanning API. Review Autofix; never auto-commit it. Secret scanning is out of scope.

**All.** Every track, one sweep, one approval checkpoint. Priority: scanning critical/high, then Dependabot critical/high, then remaining scanning and Dependabot, then quality.

Every variant still pauses for plan approval before execution. Draft PRs only; never auto-merge.
