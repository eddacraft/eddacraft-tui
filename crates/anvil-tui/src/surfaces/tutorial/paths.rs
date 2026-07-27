use super::verify::Verify;
use super::{CommandEffect, TutorialStep};

/// Starting content for the Policy path's inline editor — a minimal Rego rule
/// the user can edit in place. Illustrative: the step only verifies the file
/// exists, so this is a readable skeleton, not a runnable OPA bundle.
const NO_TODOS_REGO_SEED: &str = "\
# .anvil/policies/no-todos.rego
# Flag TODO comments left in production code.
package anvil.no_todos

import rego.v1

# Edit the marker or message, then press ctrl-s to save.
deny contains msg if {
\tsome line in input.lines
\tcontains(line.text, \"TODO\")
\tmsg := sprintf(\"TODO left in %s:%d\", [input.path, line.number])
}
";

/// Starting content for the Architecture path's inline editor — a small,
/// valid `.anvil/architecture.yaml` matching the layer schema. The user edits
/// the patterns/dependencies to fit their own project.
const ARCHITECTURE_YAML_SEED: &str = "\
schema_version: '0.1.0'
template: custom
layers:
  api:
    patterns:
      - 'src/api/**'
    depends_on:
      - services
  services:
    patterns:
      - 'src/services/**'
    depends_on: []
";

const AUTOPLAY_APP_SEED: &str = r"export function greet(name: any): string {
  return `Hello, ${name}!`;
}

// @ts-ignore
greet(42);
";

pub(super) const AUTOPLAY_APP_REPAIRED: &str = r#"export function greet(name: string): string {
  return `Hello, ${name}!`;
}

greet("anvil");
"#;

fn step(title: &str, description: &str, instruction: &str) -> TutorialStep {
    TutorialStep {
        title: title.to_string(),
        description: description.to_string(),
        instruction: instruction.to_string(),
        ..TutorialStep::default()
    }
}

fn step_with_command(
    title: &str,
    description: &str,
    instruction: &str,
    command: &str,
    effect: CommandEffect,
) -> TutorialStep {
    TutorialStep {
        command: Some(command.to_string()),
        effect: Some(effect),
        ..step(title, description, instruction)
    }
}

fn step_with_verify(
    title: &str,
    description: &str,
    instruction: &str,
    command: &str,
    effect: CommandEffect,
    verify: Verify,
    hint: &str,
) -> TutorialStep {
    TutorialStep {
        command: Some(command.to_string()),
        effect: Some(effect),
        verify: Some(verify),
        verify_hint: Some(hint.to_string()),
        ..step(title, description, instruction)
    }
}

/// Build an inline-editable step: pressing `e` opens the in-TUI editor seeded
/// with `seed_template` (when the file does not exist yet), and saving writes
/// `edit_target` then runs `verify`. `watch_path` keeps the external-editor
/// path working too, so the user can edit in-TUI *or* in their own editor.
#[allow(clippy::too_many_arguments)]
fn step_with_editor(
    title: &str,
    description: &str,
    instruction: &str,
    edit_target: &str,
    seed_template: &str,
    verify: Verify,
    hint: &str,
    watch_path: &str,
) -> TutorialStep {
    TutorialStep {
        verify: Some(verify),
        verify_hint: Some(hint.to_string()),
        watch_path: Some(watch_path.to_string()),
        edit_target: Some(edit_target.to_string()),
        seed_template: Some(seed_template.to_string()),
        ..step(title, description, instruction)
    }
}

/// The directory-creation command for the policy path's "Create Policy
/// Directory" step.
///
/// This string is run through [`executor::execute_command`], which hands it to
/// `cmd /C` on Windows and `sh -c` elsewhere. It must therefore be a command
/// the platform shell can actually execute. `mkdir` runs under both — as a
/// `cmd.exe` builtin on Windows and as a standard utility on `PATH` under `sh`
/// on Unix — and creates intermediate directories in each case, so it works
/// under the executor on every platform. An earlier revision emitted the
/// PowerShell cmdlet `New-Item` on Windows, which `cmd /C` cannot resolve
/// (`'New-Item' is not recognized`), silently breaking this step on Windows.
fn create_policy_directory_command() -> &'static str {
    if cfg!(windows) {
        r"mkdir .anvil\policies"
    } else {
        "mkdir -p .anvil/policies"
    }
}

fn create_policy_directory_instruction() -> &'static str {
    if cfg!(windows) {
        r"Run: mkdir .anvil\policies"
    } else {
        "Run: mkdir -p .anvil/policies"
    }
}

/// LAUNCH-014: the value-first default tutorial path. Walks the
/// protection loop in five informational steps without claiming
/// pre-write protection — the final step points users at
/// `anvil start --verify` (LAUNCH-006 / LAUNCH-012), which exposes
/// a literal `ProtectionState` via the verifier output.
///
/// Copy invariants (covered by tests in `tutorial::tests::protection_loop_*`):
///   - The headline never says "protected", "protecting", or
///     "pre-write" without `anvil start --verify` evidence in the
///     same step. Activation evidence does not exist inside the
///     tutorial; the tutorial points at the verifier instead of
///     claiming the state itself.
///   - The vocabulary lines (`protecting` / `ready_restart_required`
///     / `watching` / `needs_action` / `unsupported`) are referenced
///     by name so the user recognises them when `anvil status
///     --verify` prints one.
///   - "Future changes are checked" is the LAUNCH-010 baseline copy
///     and lands honestly here regardless of whether the user has
///     run `anvil start` yet — it describes the activation contract,
///     not present-tense protection.
pub fn protection_loop_steps() -> Vec<TutorialStep> {
    vec![
        step(
            "anvil's protection loop in 60 seconds",
            "anvil watches your code for the patterns that turn into incidents — silent escape hatches, unexplained TODOs, console.log slipping into prod. The loop has three steps: scan the change, surface findings, and let your editor or watch process react. This walk shows the loop on a deliberate fixture; afterwards we'll point you at `anvil start` to wire it up against your real repo.",
            "Press enter to see what we'll check.",
        ),
        step(
            "What we'll check",
            "The fixture is a tiny TypeScript file with two well-known antipatterns: `// @ts-ignore` (silently disables every type check on the next line) and `: any` (escape hatch from the type system). Both are catalogued by anvil as escape-hatch findings — the kind that compound into bugs nobody can trace.",
            "Press enter to run the check.",
        ),
        step(
            "Run the check (simulated)",
            "Imagine running `anvil check fixture.ts`. The catalogue returns:\n  • [AP-004] @ts-ignore suppresses all errors — fixture.ts:1\n  • [AP-003] Explicit any type usage — fixture.ts:2\n\nNo network call, no telemetry, no fixture deployed to your repo. Findings are deterministic — the same input always produces the same output.",
            "Press enter to see what to do with this.",
        ),
        step(
            "What protection actually means here",
            "anvil's activation vocabulary includes these honest states:\n  • `protecting` — pre-write validation is live (MCP attached + verified)\n  • `ready_restart_required` — config wired, waiting for editor restart\n  • `watching` — save-time fallback running, weaker than pre-write\n  • `needs_action` — config absent or no editor wired yet\n  • `unsupported` — anvil does not yet cover this repo's languages\n\nThis tutorial does not promote any of those states on its own — only `anvil start` and `anvil status --verify` produce evidence-backed labels. Activation does not imply the repo is clean of further findings; first activation baselines existing findings so future changes are checked.",
            "Press enter to activate in this repo.",
        ),
        step_with_command(
            "Activate in this repo",
            "Now run the safe verifier. `anvil start --verify` is read-only — it probes config (.anvilrc), the MCP client entries of any MCP-capable editor it detects (for example Cursor or Claude Code), the activation baseline, and the repo language profile, then prints one literal `ProtectionState` line. Watch-fallback liveness probing is not yet wired; the verifier reports `watch: not requested` until a future PR introspects a running watcher. If the state isn't `protecting` yet, the output names the next concrete step. Re-running is idempotent and never modifies your editor config; mutating activation is `anvil start` (no `--verify`).",
            "Run: anvil start --verify",
            "anvil start --verify",
            CommandEffect::ReadOnly,
        ),
    ]
}

/// WOW-006: sandbox-only demonstration beats. Kept separate from
/// [`protection_loop_steps`] so the ordinary interactive tutorial retains its
/// existing commands, copy, and consent posture byte-for-byte.
pub fn autoplay_protection_loop_steps() -> Vec<TutorialStep> {
    vec![
        step_with_verify(
            "Watch anvil check the pinned fixture",
            "The isolated offline fixture deliberately contains AP-003 explicit any and AP-004 @ts-ignore findings.",
            "Run: anvil check src/app.ts",
            "anvil check src/app.ts",
            CommandEffect::ReadOnly,
            Verify::OutputContains("AP-003".to_string()),
            "The pinned fixture did not report its expected AP-003 finding.",
        ),
        step_with_editor(
            "Repair the fixture inline",
            "Watch the inline editor remove the two deliberate escape hatches.",
            "Edit src/app.ts in the sandbox.",
            "src/app.ts",
            AUTOPLAY_APP_SEED,
            Verify::FileExists("src/app.ts".to_string()),
            "The sandbox fixture source must remain present.",
            "src/app.ts",
        ),
        step_with_verify(
            "Verify the repaired fixture",
            "Run the real offline check again to verify the fixture.",
            "Run: anvil check src/app.ts",
            "anvil check src/app.ts",
            CommandEffect::ReadOnly,
            Verify::ExitCode(0),
            "The sandbox verification command failed.",
        ),
        TutorialStep {
            watch_demo: true,
            ..step(
                "Watch the save-time loop react",
                "The watch demo carries the same autoplay session across the surface transition.",
                "Launch the sandbox watch demo.",
            )
        },
    ]
}

/// The developer-acceleration path: anvil in the AI-assisted development loop.
/// Teaches how anvil wires into an MCP-capable editor/agent, validates the
/// agent's edits before they land, drives a fast save-time loop, and exposes
/// graph context to the agent.
///
/// Terminology is deliberately generic: anvil supports specific editors today
/// (Cursor, Claude Code) but the copy frames them as examples of MCP-capable
/// editors, never as the only options.
///
/// Honesty: like the `ProtectionLoop` path, this walk never claims the user's
/// repo is "protected" — the read-only `anvil start --verify` step is the only
/// place a real `ProtectionState` appears.
pub fn developer_acceleration_steps() -> Vec<TutorialStep> {
    vec![
        step(
            "anvil in your AI dev loop",
            "When an AI coding agent writes code, anvil sits in the loop three ways: it gives the agent graph context so it writes code that fits your project, it validates the agent's edits before they land, and it runs a fast save-time check so you catch issues in seconds instead of at CI. This walk wires those three up. It works with any MCP-capable editor or agent — Cursor and Claude Code today, others via `anvil mcp-config`.",
            "Press enter to wire your agent.",
        ),
        step(
            "Wire your agent over MCP",
            "anvil talks to your editor/agent over MCP (the Model Context Protocol). `anvil start` writes the MCP entry for the MCP-capable editors it detects; for any other MCP client, `anvil mcp-config --target <editor>` prints (or writes, with --write) the config to drop in. Once wired, the agent can call anvil's tools — and anvil can validate what the agent is about to write. This step just explains the wiring; the next one inspects it read-only.",
            "Press enter to check activation.",
        ),
        step_with_command(
            "Pre-write validation for your agent",
            "Run the read-only verifier. `anvil start --verify` probes your config, the MCP client entries of any MCP-capable editor it detects, the activation baseline, and the repo language profile, then prints one literal `ProtectionState` line. `protecting` means pre-write validation is live for your agent's edits; anything else names the next concrete step. It changes nothing — mutating activation is `anvil start` (no `--verify`).",
            "Run: anvil start --verify",
            "anvil start --verify",
            CommandEffect::ReadOnly,
        ),
        TutorialStep {
            watch_demo: true,
            ..step(
                "The fast save-time loop",
                "Pre-write validation catches the agent before a write; `anvil watch --source` closes the loop on everything else. It re-checks files as you (or the agent) save them and surfaces findings in seconds — your inner loop, not a CI round-trip. Here is the live watch dashboard: edit a file and watch it react.",
                "Press enter to launch the watch demo.",
            )
        },
        step_with_command(
            "Graph context for your agent",
            "anvil exposes your codebase's identity and graph context to the agent over MCP, so it writes code that respects your symbols and boundaries from the start. Source text stays local by default — only identity-level context is shared unless you opt in. `anvil gctx egress status` shows the effective snippet-egress state for this workspace and where it comes from. It is read-only.",
            "Run: anvil gctx egress status",
            "anvil gctx egress status",
            CommandEffect::ReadOnly,
        ),
        step(
            "Wire it up for real",
            "That is the loop: graph context in, pre-write validation on the agent's edits, and a fast save-time check for everything else. To make it live in this repo, run `anvil start` to wire your MCP-capable editor, then `anvil watch --source` for the save-time loop. Re-running `anvil start --verify` any time reports the honest state.",
            "You have completed the developer-acceleration walk. Press enter to finish.",
        ),
    ]
}

pub fn policy_steps() -> Vec<TutorialStep> {
    vec![
        step(
            "Introduction",
            "Policies are the rules that anvil enforces on your codebase. Each policy is a Rego file (.rego) that describes what to check and how severely to flag violations using the Open Policy Agent (OPA) engine.",
            "Press enter to continue to the next step.",
        ),
        TutorialStep {
            command: Some(create_policy_directory_command().to_string()),
            // Creates `.anvil/policies/` in the user's repo.
            effect: Some(CommandEffect::MutatesRepo),
            verify: Some(Verify::FileExists(".anvil/policies".to_string())),
            verify_hint: Some("The directory was not created. Check permissions.".to_string()),
            watch_path: Some(".anvil/policies".to_string()),
            ..step(
                "Create Policy Directory",
                "anvil looks for policies in the .anvil/policies/ directory. Create this directory in your project root so anvil can discover your custom Rego rules. The tutorial uses the native directory-creation command for your shell: `mkdir -p .anvil/policies` on macOS/Linux and `mkdir .anvil\\policies` on Windows.",
                create_policy_directory_instruction(),
            )
        },
        step_with_editor(
            "Write Your First Policy",
            "A policy defines a check ID, severity level, and logic to match against. Press `e` to open the inline editor, already seeded with a Rego rule that flags TODO comments left in production code — edit it in place and press ctrl-s to save. (You can also create .anvil/policies/no-todos.rego in your own editor; the tutorial notices either way.)",
            "Press e to edit .anvil/policies/no-todos.rego inline, or create it yourself.",
            ".anvil/policies/no-todos.rego",
            NO_TODOS_REGO_SEED,
            Verify::FileExists(".anvil/policies/no-todos.rego".to_string()),
            "Create the file .anvil/policies/no-todos.rego to continue.",
            ".anvil/policies",
        ),
        step_with_verify(
            "Test the Policy",
            "Before enforcing a policy, confirm anvil can discover it. `anvil policy test` walks `.anvil/policies/` and reports the Rego test files it finds. Test execution is not yet wired up in the Rust CLI — for now, run `opa test .anvil/policies` directly to exercise Rego logic.",
            "Run: anvil policy test to list your Rego test files.",
            "anvil policy test",
            CommandEffect::ReadOnly,
            Verify::ExitCode(0),
            "anvil policy test exited non-zero — check the output for details.",
        ),
        step(
            "See the Policy Fire",
            "Add a TODO comment to any source file, then run the gate. You should see your custom policy flag a warning with the file path and line number.",
            "Run: anvil gate to evaluate policies against the codebase.",
        ),
        step(
            "Customise Severity",
            "Policies support four severity levels: critical, high, medium, and low. Critical findings block the gate; lower severities produce warnings. Adjust your Rego metadata to match your team workflow.",
            "Edit the severity in no-todos.rego, then re-run anvil gate to see the updated severity.",
        ),
    ]
}

pub fn architecture_steps() -> Vec<TutorialStep> {
    vec![
        step(
            "Introduction",
            "Architecture enforcement validates that your code respects the layer boundaries you define. anvil prevents imports that violate your declared dependency rules, catching architectural drift early.",
            "Press enter to continue to the next step.",
        ),
        step_with_editor(
            "Define Your Layers",
            "Layers map directories to names and declare which layers may depend on which. Press `e` to open the inline editor, seeded with a small .anvil/architecture.yaml — edit the patterns and depends_on lists to match your project, then press ctrl-s to save. (You can also create the file in your own editor.)",
            "Press e to edit .anvil/architecture.yaml inline, or create it yourself.",
            ".anvil/architecture.yaml",
            ARCHITECTURE_YAML_SEED,
            Verify::FileExists(".anvil/architecture.yaml".to_string()),
            "Create .anvil/architecture.yaml to continue.",
            ".anvil",
        ),
        step_with_verify(
            "Validate the Architecture",
            "Validate the architecture definition in .anvil/architecture.yaml. This checks that layers, boundaries, and allowed-import rules are well-formed before enforcement.",
            "Run: anvil architecture validate",
            "anvil architecture validate",
            CommandEffect::ReadOnly,
            Verify::ExitCode(0),
            "Validation failed. Check your architecture.yaml.",
        ),
        step_with_command(
            "Show Definition",
            "See how anvil parses your architecture. The `show` command prints the template name, each layer's patterns and dependencies, and the rule count from `.anvil/architecture.yaml`.",
            "Run: anvil architecture show",
            "anvil architecture show",
            CommandEffect::ReadOnly,
        ),
        step(
            "Validate Boundaries",
            "Add a deliberate cross-layer import to see the violation in action. anvil will surface the disallowed import and the boundary rule that was broken during `anvil check` or `anvil gate`.",
            "Add a cross-layer import and run: anvil check",
        ),
        step(
            "Summary",
            "You now have architecture enforcement configured. Boundary violations will be surfaced in every commit review via `anvil gate` and during active development via your editor.",
            "Architecture enforcement is ready. Press enter to finish.",
        ),
    ]
}

pub fn drift_steps() -> Vec<TutorialStep> {
    vec![
        step(
            "Introduction",
            "Drift detection captures snapshots of your configuration and architecture, flagging changes between captures. This helps you track unintended structural changes over time.",
            "Press enter to continue to the next step.",
        ),
        step_with_verify(
            "Capture a Baseline",
            "Take an initial snapshot of your current state. anvil serialises the config and architecture into a versioned snapshot stored in .anvil/snapshots/.",
            "Run: anvil drift snapshot --name baseline",
            "anvil drift snapshot --name baseline",
            CommandEffect::MutatesRepo,
            Verify::ExitCode(0),
            "Capture failed. Is your project initialised?",
        ),
        step_with_command(
            "Capture Current State",
            "After making structural changes, capture a second snapshot. anvil stores each snapshot by name so you can compare them later.",
            "Run: anvil drift snapshot --name current",
            "anvil drift snapshot --name current",
            CommandEffect::MutatesRepo,
        ),
        step_with_command(
            "Compare Snapshots",
            "Now compare the two snapshots. anvil shows a structured diff highlighting exactly what changed in your configuration or layer definitions.",
            "Run: anvil drift compare baseline current",
            "anvil drift compare baseline current",
            CommandEffect::ReadOnly,
        ),
        TutorialStep {
            watch_demo: true,
            ..step(
                "Watch Mode Demo",
                "See anvil\u{2019}s watch dashboard in action. It monitors your files and runs checks in real time. Edit a file and watch the dashboard update automatically.",
                "Press enter to launch the watch demo.",
            )
        },
        step(
            "Summary",
            "Drift detection gives you visibility into structural changes. You can run `anvil drift report` in CI to catch unintended drift before it is merged.",
            "Drift detection is configured. Press enter to finish.",
        ),
    ]
}

pub fn ci_steps() -> Vec<TutorialStep> {
    vec![
        step(
            "Introduction",
            "Integrating anvil into your CI pipeline ensures that every pull request is checked against your policies, architecture rules, and drift baselines automatically.",
            "Press enter to continue to the next step.",
        ),
        step(
            "Install Git Hooks",
            "Git hooks run anvil checks before each commit. The pre-commit hook evaluates your gate profile and blocks commits that fail critical checks. anvil supports both Husky and native git config hooks (`--config`).",
            "Run: anvil hooks install",
        ),
        step(
            "Add CI Workflow",
            "Create a GitHub Actions workflow that runs `anvil gate` on every push and pull request. The workflow exits with a non-zero code when checks fail.",
            "Add .github/workflows/anvil.yml with a step that runs anvil gate.",
        ),
        step(
            "Configure Exit Codes",
            "anvil uses structured exit codes: 0 for pass, 1 for errors, 2 for gate failure, 3 for auth required, and 4 for configuration errors. Map these to your CI fail-fast settings.",
            "Verify exit code handling in your workflow file.",
        ),
        step_with_verify(
            "Machine-Readable Output",
            "anvil auto-detects CI environments and adjusts its output. Use the `--json` flag to produce machine-readable output suitable for downstream tooling.",
            "Run: anvil status --json to preview JSON output.",
            "anvil status --json",
            CommandEffect::ReadOnly,
            Verify::OutputContains("\"status\":".to_string()),
            "Expected JSON output with status field.",
        ),
        step(
            "Summary",
            "Your CI pipeline now runs anvil checks on every push. The gate blocks merges when critical policies are violated, keeping your main branch clean.",
            "CI integration is configured. Press enter to finish.",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_steps_valid(steps: &[TutorialStep], expected_count: usize, expected_titles: &[&str]) {
        assert_eq!(steps.len(), expected_count);
        for step in steps {
            assert!(!step.title.is_empty(), "step title must not be empty");
            assert!(
                !step.description.is_empty(),
                "step description must not be empty"
            );
            assert!(
                !step.instruction.is_empty(),
                "step instruction must not be empty"
            );
            assert!(!step.completed, "steps should start incomplete");
        }
        let titles: Vec<&str> = steps.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(titles, expected_titles);
    }

    #[test]
    fn developer_acceleration_path_steps() {
        let steps = developer_acceleration_steps();
        assert_steps_valid(
            &steps,
            6,
            &[
                "anvil in your AI dev loop",
                "Wire your agent over MCP",
                "Pre-write validation for your agent",
                "The fast save-time loop",
                "Graph context for your agent",
                "Wire it up for real",
            ],
        );
    }

    #[test]
    fn developer_acceleration_uses_real_readonly_commands() {
        let steps = developer_acceleration_steps();
        // The two command steps run verified, read-only anvil subcommands.
        assert_eq!(steps[2].command.as_deref(), Some("anvil start --verify"));
        assert_eq!(
            steps[4].command.as_deref(),
            Some("anvil gctx egress status")
        );
        // The fast-loop step reuses the live watch demo.
        assert!(
            steps[3].watch_demo,
            "fast-loop step launches the watch demo"
        );
    }

    #[test]
    fn developer_acceleration_copy_is_honest_and_generic() {
        let body = developer_acceleration_steps()
            .iter()
            .map(|s| format!("{}\n{}\n{}", s.title, s.description, s.instruction))
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        // Honesty: never claims the user's repo is already protected.
        for forbidden in [
            "you are now protected",
            "your repo is protected",
            "pre-write validation enabled",
        ] {
            assert!(!body.contains(forbidden), "must not claim `{forbidden}`");
        }
        // The verifier is the only place a real state comes from.
        assert!(body.contains("anvil start --verify"));
        // Generic terminology: editor names appear only as examples, and the
        // generic "mcp-capable" framing must be present.
        assert!(
            body.contains("mcp-capable"),
            "copy should frame editors generically as MCP-capable"
        );
    }

    #[test]
    fn policy_path_steps() {
        let steps = policy_steps();
        assert_steps_valid(
            &steps,
            6,
            &[
                "Introduction",
                "Create Policy Directory",
                "Write Your First Policy",
                "Test the Policy",
                "See the Policy Fire",
                "Customise Severity",
            ],
        );
    }

    #[test]
    fn architecture_path_steps() {
        let steps = architecture_steps();
        assert_steps_valid(
            &steps,
            6,
            &[
                "Introduction",
                "Define Your Layers",
                "Validate the Architecture",
                "Show Definition",
                "Validate Boundaries",
                "Summary",
            ],
        );
    }

    #[test]
    fn drift_path_steps() {
        let steps = drift_steps();
        assert_steps_valid(
            &steps,
            6,
            &[
                "Introduction",
                "Capture a Baseline",
                "Capture Current State",
                "Compare Snapshots",
                "Watch Mode Demo",
                "Summary",
            ],
        );
    }

    #[test]
    fn ci_path_steps() {
        let steps = ci_steps();
        assert_steps_valid(
            &steps,
            6,
            &[
                "Introduction",
                "Install Git Hooks",
                "Add CI Workflow",
                "Configure Exit Codes",
                "Machine-Readable Output",
                "Summary",
            ],
        );
    }

    #[test]
    fn create_policy_directory_command_is_platform_native() {
        let command = create_policy_directory_command();
        if cfg!(windows) {
            assert_eq!(command, r"mkdir .anvil\policies");
        } else {
            assert_eq!(command, "mkdir -p .anvil/policies");
        }
    }

    /// The command is executed through `executor::execute_command`, which uses
    /// `cmd /C` on Windows — not PowerShell. Guard against re-introducing a
    /// PowerShell-only cmdlet (`New-Item`) that `cmd.exe` cannot resolve, which
    /// silently broke this step on Windows before. `mkdir` is a `cmd.exe`
    /// builtin, so the executor's shell can run it.
    #[test]
    fn create_policy_directory_command_runs_under_executor_shell() {
        let command = create_policy_directory_command();
        assert!(
            command.starts_with("mkdir"),
            "policy-directory command must be runnable under the executor shell \
             (`cmd /C` or `sh -c`), got: {command:?}"
        );
        assert!(
            !command.contains("New-Item"),
            "New-Item is a PowerShell cmdlet; the executor runs `cmd /C`, which cannot \
             resolve it"
        );
    }

    #[test]
    fn policy_steps_have_correct_commands() {
        let steps = policy_steps();
        // Introduction — no command
        assert!(
            steps[0].command.is_none(),
            "Introduction should have no command"
        );
        assert!(steps[0].verify.is_none());
        // Create Policy Directory — has command + verify + watch
        assert_eq!(
            steps[1].command.as_deref(),
            Some(create_policy_directory_command())
        );
        assert!(
            steps[1].verify.is_some(),
            "Create Policy Directory should have verification"
        );
        assert!(steps[1].verify_hint.is_some());
        assert!(
            steps[1].watch_path.is_some(),
            "Create Policy Directory should have watch_path"
        );
        // Write Your First Policy — no command, has verify + watch
        assert!(
            steps[2].command.is_none(),
            "Write Your First Policy should have no command"
        );
        assert!(
            steps[2].verify.is_some(),
            "Write Your First Policy should have verification"
        );
        assert!(
            steps[2].watch_path.is_some(),
            "Write Your First Policy should have watch_path"
        );
        // Test the Policy — has command + verify
        assert_eq!(steps[3].command.as_deref(), Some("anvil policy test"));
        assert!(
            steps[3].verify.is_some(),
            "Test the Policy should have verification"
        );
        assert!(steps[3].verify_hint.is_some());
        // See the Policy Fire — no command (informational)
        assert!(
            steps[4].command.is_none(),
            "See the Policy Fire should have no command"
        );
        // Customise Severity — no command (informational)
        assert!(
            steps[5].command.is_none(),
            "Customise Severity should have no command"
        );
    }

    #[test]
    fn architecture_steps_have_correct_commands() {
        let steps = architecture_steps();
        assert!(
            steps[0].command.is_none(),
            "Introduction should have no command"
        );
        assert!(
            steps[1].command.is_none(),
            "Define Your Layers should have no command"
        );
        assert!(
            steps[1].verify.is_some(),
            "Define Your Layers should have verification"
        );
        assert!(
            steps[1].watch_path.is_some(),
            "Define Your Layers should have watch_path"
        );
        assert_eq!(
            steps[2].command.as_deref(),
            Some("anvil architecture validate")
        );
        assert!(
            steps[2].verify.is_some(),
            "Validate the Architecture should have verification"
        );
        assert!(steps[2].verify_hint.is_some());
        assert_eq!(steps[3].command.as_deref(), Some("anvil architecture show"));
        assert!(
            steps[3].verify.is_none(),
            "Show Definition has no verification"
        );
        // Validate Boundaries — informational (mentions running the command in the instruction
        // text but is not a direct executable step)
        assert!(
            steps[4].command.is_none(),
            "Validate Boundaries should have no command"
        );
        assert!(steps[5].command.is_none(), "Summary should have no command");
    }

    #[test]
    fn drift_steps_have_correct_commands() {
        let steps = drift_steps();
        assert!(
            steps[0].command.is_none(),
            "Introduction should have no command"
        );
        assert_eq!(
            steps[1].command.as_deref(),
            Some("anvil drift snapshot --name baseline")
        );
        assert!(
            steps[1].verify.is_some(),
            "Capture a Baseline should have verification"
        );
        assert!(steps[1].verify_hint.is_some());
        assert_eq!(
            steps[2].command.as_deref(),
            Some("anvil drift snapshot --name current")
        );
        assert!(
            steps[2].verify.is_none(),
            "Capture Current State has no verification"
        );
        assert_eq!(
            steps[3].command.as_deref(),
            Some("anvil drift compare baseline current")
        );
        assert!(
            steps[4].command.is_none(),
            "Watch Mode Demo should have no command"
        );
        assert!(
            steps[4].watch_demo,
            "Watch Mode Demo should have watch_demo flag"
        );
        assert!(steps[5].command.is_none(), "Summary should have no command");
    }

    #[test]
    fn ci_steps_have_correct_commands() {
        let steps = ci_steps();
        assert!(
            steps[0].command.is_none(),
            "Introduction should have no command"
        );
        assert!(
            steps[1].command.is_none(),
            "Install Git Hooks should have no command"
        );
        assert!(
            steps[2].command.is_none(),
            "Add CI Workflow should have no command"
        );
        assert!(
            steps[3].command.is_none(),
            "Configure Exit Codes should have no command"
        );
        assert_eq!(steps[4].command.as_deref(), Some("anvil status --json"));
        assert!(
            steps[4].verify.is_some(),
            "Detect CI Environment should have verification"
        );
        assert!(steps[4].verify_hint.is_some());
        assert!(steps[5].command.is_none(), "Summary should have no command");
    }

    /// WOW-001: effects are declared honestly — the steps that write into
    /// the user's repo carry `MutatesRepo`, the inspect-only ones `ReadOnly`.
    #[test]
    fn command_effects_are_declared_honestly() {
        let policy = policy_steps();
        assert_eq!(policy[1].effect, Some(CommandEffect::MutatesRepo)); // mkdir
        assert_eq!(policy[3].effect, Some(CommandEffect::ReadOnly)); // policy test

        let drift = drift_steps();
        assert_eq!(drift[1].effect, Some(CommandEffect::MutatesRepo)); // snapshot
        assert_eq!(drift[2].effect, Some(CommandEffect::MutatesRepo)); // snapshot
        assert_eq!(drift[3].effect, Some(CommandEffect::ReadOnly)); // compare

        // The read-only verifier closes both value-first paths.
        assert_eq!(
            protection_loop_steps().last().unwrap().effect,
            Some(CommandEffect::ReadOnly)
        );
        let dev = developer_acceleration_steps();
        assert_eq!(dev[2].effect, Some(CommandEffect::ReadOnly));
        assert_eq!(dev[4].effect, Some(CommandEffect::ReadOnly));

        let arch = architecture_steps();
        assert_eq!(arch[2].effect, Some(CommandEffect::ReadOnly));
        assert_eq!(arch[3].effect, Some(CommandEffect::ReadOnly));

        assert_eq!(ci_steps()[4].effect, Some(CommandEffect::ReadOnly));
    }

    #[test]
    fn all_steps_start_with_no_output() {
        for steps in [
            policy_steps(),
            architecture_steps(),
            drift_steps(),
            ci_steps(),
        ] {
            for step in &steps {
                assert!(
                    step.output.is_none(),
                    "step '{}' should have no output initially",
                    step.title
                );
                assert!(
                    step.verify_result.is_none(),
                    "step '{}' should have no verify result initially",
                    step.title
                );
            }
        }
    }
}
