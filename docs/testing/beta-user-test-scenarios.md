# Beta User Test Scenarios

| Type  | Authority | Owner | Status | Freshness                                        |
| ----- | --------- | ----- | ------ | ------------------------------------------------ |
| Guide | Advisory  | BETA  | Live   | Metadata backfilled 2026-05-27 during DOCGOV-011 |

| Upstream                                  | Downstream                     |
| ----------------------------------------- | ------------------------------ |
| Public beta testing guide, beta programme | Facilitated beta test sessions |

Internal facilitator guide for running 45-60 minute beta sessions with one or
two external users. Send testers the public
[Beta Testing Guide](../public/anvil/beta-testing-guide.md); use this document
to keep sessions consistent and to capture comparable feedback.

## Session Goals

- Verify that a new beta user can install, authenticate, and initialise a real
  project without help.
- Observe whether the first useful signal appears quickly enough to feel worth
  the setup cost.
- Find confusing command output, weak remediation text, false positives, and
  missed findings.
- Validate that watch mode is clear enough and fast enough for save-time use.
- Leave with concrete bug reports or product notes, not vague sentiment.

## Participant Profile

Choose testers who can bring a real TypeScript or JavaScript project they know.
The ideal project is an active application or package with Git history, a
package manager, and at least a few source directories.

Ask each tester to prepare:

- A macOS, Linux, or Windows machine.
- A terminal they normally use.
- A project they are allowed to run local tooling against.
- A clean Git working tree or disposable branch.
- Beta access tied to their invited email or GitHub account.

## Facilitator Setup

Before the session:

- Confirm the current target version is `0.5.0-beta` or newer.
- Confirm the tester is invited and can access the docs.
- Keep the public guide open for reference.
- Create a notes file using the capture template below.
- Do not lead the tester through every click unless they are blocked. Watch what
  they try first.

Recommended opening script:

```text
Please think out loud. I am testing the product and documentation, not you. If
something is unclear, try what you would normally try before I help. I will ask
you to pause at a few points so I can capture what happened.
```

## Capture Template

```text
# Beta Session Notes

Tester:
Date:
OS / terminal:
Install method:
Project type:
Project size rough notes:
anvil version:

## Timeline

- 00:00 Started
- 00:00 Install / upgrade
- 00:00 Auth
- 00:00 Init
- 00:00 First scan
- 00:00 Watch
- 00:00 Gate / diagnostics

## Findings

| ID  | Severity | Area | Observation | Evidence | Follow-up |
| --- | -------- | ---- | ----------- | -------- | --------- |
| 1   |          |      |             |          |           |

## Tester Quotes

- "..."

## End-of-Session Ratings

- Install clarity, 1-5:
- Auth clarity, 1-5:
- First finding usefulness, 1-5:
- Watch-mode usefulness, 1-5:
- Overall likelihood to keep using, 1-5:

## Open Questions

- ...
```

## Scenario 1: Install or Upgrade

Purpose: learn whether the tester can get a working binary without hand-holding.

Steps for the tester:

1. Open the public beta guide.
2. Choose the install method they would naturally use.
3. Run the install or upgrade command.
4. Run `anvil --version`.
5. If the command is not found, try the troubleshooting instructions before the
   facilitator helps.

Expected result:

- `anvil --version` prints the current beta version.
- The tester understands whether they installed via installer, Homebrew, WinGet,
  Scoop, or an updater.

Facilitator prompts:

- "Was that the install method you expected to use?"
- "Did anything ask for permissions you did not expect?"
- "If `anvil` was not on PATH, did the docs make the fix obvious?"

Capture:

- Install command used.
- Version output.
- Any PATH, shell profile, antivirus, proxy, or package-manager friction.

## Scenario 2: Authenticate

Purpose: validate device-code login and fallback OTP wording.

Steps for the tester:

1. Run `anvil auth login`.
2. Follow the device-code instructions in the browser.
3. Return to the terminal and wait for completion.
4. Run `anvil auth whoami`.
5. If blocked, try `anvil auth login --otp`.

Expected result:

- Device-code login completes.
- `anvil auth whoami` shows the signed-in identity.
- If OTP is needed, the tester understands when to use it.

Facilitator prompts:

- "Did you trust the browser step?"
- "Was it clear when to return to the terminal?"
- "Would you know what to do if the browser did not open?"

Capture:

- Auth method used.
- Any confusion around email, GitHub, account eligibility, or expired sessions.
- Exact error text for failed or pending login states.

## Scenario 3: Tutorial First Impression

Purpose: check whether the tutorial explains the product before the tester uses
a real project.

Steps for the tester:

1. Run `anvil tutorial`.
2. Read and follow the tutorial without facilitator explanation.
3. Say out loud when the product value becomes clear.
4. Note the first point that feels slow, obvious, or confusing.

Expected result:

- The tutorial runs in the tester's terminal.
- The tester can explain what anvil is for after completion.

Facilitator prompts:

- "What do you think anvil will do in your real project?"
- "Which step, if any, felt like setup friction rather than value?"
- "Would you skip this tutorial next time?"

Capture:

- Tutorial duration.
- Terminal rendering issues.
- Misunderstood concepts or wording.

## Scenario 4: Initialise a Real Project

Purpose: validate first-run setup and post-init analysis on a real codebase.

Steps for the tester:

1. Move to a real project: `cd <project>`.
2. Confirm they are on a disposable branch or clean working tree.
3. Run `anvil init`.
4. Read the output without asking the facilitator to interpret it.
5. Inspect the files created if they want to.

Expected result:

- `.anvilrc` exists.
- `.anvil/` exists.
- The command reports detected project details.
- The command runs a first sample analysis or explains why no files were
  scanned.

Facilitator prompts:

- "Did it detect the project correctly?"
- "Do you understand what files were created?"
- "Was the first scan result useful, noisy, or absent?"
- "Would you commit these generated files? Why or why not?"

Capture:

- Project shape: monorepo, package workspace, app, library, framework.
- Detected package manager and whether it was correct.
- First warning shown and whether it was real.
- Anything surprising in `.anvilrc`.

## Scenario 5: Main Scan and Changed-File Scan

Purpose: compare broad analysis with the development-loop commands.

Steps for the tester:

1. Run `anvil check --all`.
2. Pick the top two findings and decide whether each is real.
3. Modify or stage a harmless file if needed.
4. Run `anvil check --changed`.
5. If they have staged changes, run `anvil check --changed --staged`.

Expected result:

- Full scan completes or fails with actionable remediation.
- Changed-file scan scopes results to Git changes.
- Findings include file, line, rule ID, and a useful suggestion.

Facilitator prompts:

- "Which finding would you fix first?"
- "Which finding would you ignore or suppress?"
- "Did the command finish quickly enough?"
- "Was there a real issue you expected it to catch but it missed?"

Capture:

- Runtime if notable.
- False positives with enough context to reproduce.
- Missed patterns described in the tester's words.
- Output that lacks a clear next step.

## Scenario 6: Watch Mode and Filters

Purpose: test the save-time loop and the watch-filter behaviour shipped in
0.4.0-beta and refined since.

Steps for the tester:

1. Run `anvil watch --source`.
2. Read the startup banner and describe what will be watched.
3. Save a TypeScript or JavaScript file.
4. Stop watch mode with `Ctrl+C`.
5. Run `anvil watch --patterns "src/**/*.ts,src/**/*.tsx"`.
6. Stop watch mode again.
7. Run `anvil watch --exclude "dist/**,coverage/**"`.
8. Optionally try a bare exclude such as `anvil watch --exclude dist` and
   observe whether the warning is clear.

Expected result:

- Watch startup explains the active scope.
- A save event triggers analysis.
- Glob include and exclude behaviour is understandable.
- Bare exclude names produce a warning that points to `dist/**` style syntax.

Facilitator prompts:

- "Would you leave this running while coding?"
- "Was it obvious what the command is watching?"
- "Did any output feel too noisy for a save-time loop?"
- "Would the glob syntax be easy to remember?"

Capture:

- Time from save to visible feedback if it feels slow.
- Files that should have triggered but did not.
- Files that triggered unexpectedly.
- Confusing filter wording.

## Scenario 7: Diagnostics, Status, and Gate

Purpose: validate troubleshooting and project-health surfaces.

Steps for the tester:

1. Run `anvil doctor`.
2. Run `anvil status`.
3. Run `anvil gate --profile dev`.
4. If local project tooling is available, run `anvil gate --profile ci`.
5. If a command fails, ask the tester to interpret the remediation before
   helping.

Expected result:

- Diagnostics identify missing tools or config with concrete remediation.
- Status summarises hooks, profile, and recent runs.
- Gate output explains pass, fail, skipped, or blocked checks.

Facilitator prompts:

- "Do you know whether your project is healthy after reading this?"
- "Which remediation would you run next?"
- "Did any skipped check look like a product failure?"

Capture:

- Doctor failures and remediation text.
- Gate profile confusion.
- External tool requirements that surprised the tester.

## Scenario 8: AI Guardrail Profile (0.5.0-beta)

Purpose: validate the headline 0.5.0-beta surface — the AI-focused gate profile
— on a real project, especially for testers who use anvil from agentic
workflows.

Steps for the tester:

1. Run `anvil gate --profile ai`.
2. If the run blocks on missing or invalid governance config, read the JSON
   envelope and try to interpret which key needs fixing.
3. Open a source file and add a comment such as `// ChatGPT said this was fine`
   near a real change.
4. Re-run `anvil gate --profile ai` (or `anvil check --all`) and confirm
   `AI-001` flags the comment at info severity.
5. Suppress the finding with `// @anvil-ignore AI-001 -- <reason>` and re-run;
   confirm the suppression is honoured.

Expected result:

- The AI guardrail profile produces deterministic JSON output by default, not
  human-readable text.
- AI-001 fires only on the new comment, not on string literals or unrelated
  code.
- Suppression with a reason silences AI-001; bare suppression is itself a
  finding.

Capture:

- Whether the strict-config error message identifies the offending key.
- Whether the JSON envelope is the same shape the tester would expect to parse
  from an agent.
- Any false positives in comments that look reasoning-shaped but are not appeals
  to authority.

## Scenario 9: Editor MCP Configuration (0.5.0-beta)

Purpose: validate `anvil mcp-config` end-to-end against a real editor.

Steps for the tester:

1. Run `anvil mcp-config --target claude-code` (or the tester's editor) and read
   the printed config.
2. Run `anvil mcp-config --target claude-code --verify` against an existing
   editor config; observe drift, if any.
3. Run `anvil mcp-config --target claude-code --write` and confirm the
   path-safety prompt appears before any overwrite.
4. Open the editor and confirm the anvil MCP server is reachable.

Expected result:

- The generated config is correct for the chosen client.
- `--verify` cleanly reports drift or "in sync" without writing.
- `--write` prompts before overwriting; the atomic write leaves no partial file
  behind on cancel.

Capture:

- Editors where the generated path was wrong or required `--workspace`.
- Drift the tester would not have noticed without `--verify`.
- Path-safety prompts that felt either too aggressive or too quiet.

## Scenario 10: Config-Mode Git Hooks (0.5.0-beta, Git 2.54+)

Purpose: validate the opt-in config-mode hook flow on Git 2.54 or newer.

Steps for the tester:

1. Confirm `git --version` is 2.54+; otherwise skip the scenario.
2. Run `anvil hooks install --config`.
3. Run `anvil hooks status` and `anvil doctor`; confirm both surface the
   config-mode entries and any third-party hook manager (Husky, Lefthook).
4. Make a small commit and confirm Anvil hooks fire.
5. Run `anvil hooks uninstall --config` and confirm the entries are removed
   without disturbing Husky or `core.hooksPath`.

Expected result:

- Install/uninstall touches only Anvil-owned `hook.<event>.command` entries.
- Doctor and status warn when file-mode hooks and config-mode hooks would both
  fire for the same event.
- Husky-driven contributor flow is unaffected.

Capture:

- Whether coexistence warnings are clear and actionable.
- Any third-party hook manager that anvil failed to detect.
- Whether the uninstall path leaves the repo's contributor bootstrap working.

## Scenario 11: Optional Architecture and Drift

Purpose: test higher-value features only when the project has clear boundaries
or the tester is interested in architecture governance.

Steps for the tester:

1. Identify two or three project layers in plain English.
2. Create or review `.anvil/architecture.yaml`.
3. Run `anvil architecture validate`.
4. Run `anvil architecture show`.
5. Run `anvil drift snapshot --name before-test`.
6. Run `anvil drift list`.
7. If a second snapshot exists, run `anvil drift compare <before> <after>`.

Expected result:

- Architecture config validation errors are understandable.
- Layer and rule output maps to the tester's mental model.
- Drift snapshots feel useful rather than abstract.

Facilitator prompts:

- "Could you encode your actual architecture this way?"
- "Would a teammate understand this output?"
- "Does drift reporting tell you anything you would act on?"

Capture:

- Boundary concepts that are hard to express.
- Validation wording that does not identify the broken layer or pattern.
- Whether drift output has a clear audience and use case.

## Scenario 12: Feedback Report Dry Run

Purpose: make sure testers know how to report issues after the session.

Steps for the tester:

1. Choose one bug, confusing moment, or false positive from the session.
2. Draft a GitHub issue using the template in the public guide.
3. Include `anvil --version`, OS, install method, command, expected behaviour,
   and actual behaviour.
4. Decide whether any code or screenshots need redaction.

Expected result:

- The tester can produce a report that engineering can act on.
- Sensitive project details are not shared accidentally.

Facilitator prompts:

- "Would you file this after the call without prompting?"
- "Is the issue template asking for the right amount of information?"
- "What would make reporting lower friction?"

Capture:

- Draft issue link or copied issue body.
- Missing fields in the template.
- Any privacy concerns.

## End-of-Session Debrief

Ask these before ending:

- "What was the first moment that felt valuable?"
- "What was the first moment that felt risky, unclear, or annoying?"
- "Would you run this during normal development tomorrow? Why or why not?"
- "Which warning would you show a teammate as evidence this is useful?"
- "What should we fix before inviting the next tester?"

## Triage Rubric

Use this to classify notes after the session.

| Severity | Meaning                                         | Examples                                                           |
| -------- | ----------------------------------------------- | ------------------------------------------------------------------ |
| Critical | Blocks install, auth, or running the first scan | Binary unavailable, login loop, panic on init                      |
| Major    | Tester can proceed but core value is damaged    | Watch misses saves, false positives dominate, remediation unusable |
| Minor    | Friction or confusion with workaround           | Wording unclear, output order odd, missing link                    |
| Note     | Product insight without immediate defect        | Desired integration, workflow preference, packaging request        |

## Follow-Up Checklist

- File GitHub issues for Critical and Major findings within 24 hours.
- Add `beta-feedback` to session observations that are not defects.
- Link screenshots, logs, or redacted command output.
- Note tester environment and project shape in every issue.
- Send the tester a short thank-you and any workaround they need.
- Re-run the scenario internally after fixes land.
