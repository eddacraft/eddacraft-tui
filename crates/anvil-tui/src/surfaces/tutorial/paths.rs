use super::TutorialStep;
use super::verify::Verify;

fn step(title: &str, description: &str, instruction: &str) -> TutorialStep {
    TutorialStep {
        title: title.to_string(),
        description: description.to_string(),
        instruction: instruction.to_string(),
        command: None,
        completed: false,
        output: None,
        verify: None,
        verify_result: None,
        verify_hint: None,
        watch_path: None,
        watch_demo: false,
    }
}

fn step_with_command(
    title: &str,
    description: &str,
    instruction: &str,
    command: &str,
) -> TutorialStep {
    TutorialStep {
        title: title.to_string(),
        description: description.to_string(),
        instruction: instruction.to_string(),
        command: Some(command.to_string()),
        completed: false,
        output: None,
        verify: None,
        verify_result: None,
        verify_hint: None,
        watch_path: None,
        watch_demo: false,
    }
}

fn step_with_verify(
    title: &str,
    description: &str,
    instruction: &str,
    command: &str,
    verify: Verify,
    hint: &str,
) -> TutorialStep {
    TutorialStep {
        title: title.to_string(),
        description: description.to_string(),
        instruction: instruction.to_string(),
        command: Some(command.to_string()),
        completed: false,
        output: None,
        verify: Some(verify),
        verify_result: None,
        verify_hint: Some(hint.to_string()),
        watch_path: None,
        watch_demo: false,
    }
}

fn step_with_watch(
    title: &str,
    description: &str,
    instruction: &str,
    verify: Verify,
    hint: &str,
    watch_path: &str,
) -> TutorialStep {
    TutorialStep {
        title: title.to_string(),
        description: description.to_string(),
        instruction: instruction.to_string(),
        command: None,
        completed: false,
        output: None,
        verify: Some(verify),
        verify_result: None,
        verify_hint: Some(hint.to_string()),
        watch_path: Some(watch_path.to_string()),
        watch_demo: false,
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
            "Anvil's protection loop in 60 seconds",
            "Anvil watches your code for the patterns that turn into incidents — silent escape hatches, unexplained TODOs, console.log slipping into prod. The loop has three steps: scan the change, surface findings, and let your editor or watch process react. This walk shows the loop on a deliberate fixture; afterwards we'll point you at `anvil start` to wire it up against your real repo.",
            "Press enter to see what we'll check.",
        ),
        step(
            "What we'll check",
            "The fixture is a tiny TypeScript file with two well-known antipatterns: `// @ts-ignore` (silently disables every type check on the next line) and `: any` (escape hatch from the type system). Both are catalogued by Anvil as escape-hatch findings — the kind that compound into bugs nobody can trace.",
            "Press enter to run the check.",
        ),
        step(
            "Run the check (simulated)",
            "Imagine running `anvil check fixture.ts`. The catalogue returns:\n  • [AP-004] @ts-ignore suppresses all errors — fixture.ts:1\n  • [AP-003] Explicit any type usage — fixture.ts:2\n\nNo network call, no telemetry, no fixture deployed to your repo. Findings are deterministic — the same input always produces the same output.",
            "Press enter to see what to do with this.",
        ),
        step(
            "What protection actually means here",
            "Anvil's activation vocabulary includes these honest states:\n  • `protecting` — pre-write validation is live (MCP attached + verified)\n  • `ready_restart_required` — config wired, waiting for editor restart\n  • `watching` — save-time fallback running, weaker than pre-write\n  • `needs_action` — config absent or no editor wired yet\n  • `unsupported` — Anvil does not yet cover this repo's languages\n\nThis tutorial does not promote any of those states on its own — only `anvil start` and `anvil status --verify` produce evidence-backed labels. Activation does not imply the repo is clean of further findings; first activation baselines existing findings so future changes are checked.",
            "Press enter to activate in this repo.",
        ),
        step_with_command(
            "Activate in this repo",
            "Now run the safe verifier. `anvil start --verify` is read-only — it probes config, MCP clients (Cursor, Claude Code), and the watch fallback, then prints one literal `ProtectionState` line. If the state isn't `protecting` yet, the output names the next concrete step. Re-running is idempotent and never modifies your editor config; mutating activation is `anvil start` (no `--verify`).",
            "Run: anvil start --verify",
            "anvil start --verify",
        ),
    ]
}

pub fn policy_steps() -> Vec<TutorialStep> {
    vec![
        step(
            "Introduction",
            "Policies are the rules that Anvil enforces on your codebase. Each policy is a declarative YAML file that describes what to check and how severely to flag violations.",
            "Press enter to continue to the next step.",
        ),
        TutorialStep {
            command: Some("mkdir -p .anvil/policies".to_string()),
            verify: Some(Verify::FileExists(".anvil/policies".to_string())),
            verify_hint: Some("The directory was not created. Check permissions.".to_string()),
            watch_path: Some(".anvil/policies".to_string()),
            ..step(
                "Create Policy Directory",
                "Anvil looks for policies in the .anvil/policies/ directory. Create this directory in your project root so Anvil can discover your custom rules.",
                "Run: mkdir -p .anvil/policies",
            )
        },
        step_with_watch(
            "Write Your First Policy",
            "A policy file defines a check ID, severity level, and a pattern to match against. Start with a simple rule that flags TODO comments left in production code.",
            "Create .anvil/policies/no-todos.yaml with a pattern rule.",
            Verify::FileExists(".anvil/policies/no-todos.yaml".to_string()),
            "Create the file .anvil/policies/no-todos.yaml to continue.",
            ".anvil/policies",
        ),
        step_with_verify(
            "Test the Policy",
            "Before enforcing a policy, test it locally to confirm it catches the expected patterns. Anvil's dry-run mode evaluates policies without blocking commits.",
            "Run: anvil doctor to verify your setup is healthy.",
            "anvil doctor",
            Verify::ExitCode(0),
            "Doctor reported issues. Check the output above.",
        ),
        step(
            "See the Policy Fire",
            "Add a TODO comment to any source file, then run the gate. You should see the no-todos policy flag a warning with the file path and line number.",
            "Run: anvil gate to evaluate your custom policies against the codebase.",
        ),
        step(
            "Customise Severity",
            "Policies support four severity levels: critical, high, medium, and low. Critical findings block the gate; lower severities produce warnings. Adjust to match your team workflow.",
            "Edit the severity field in no-todos.yaml, then re-run anvil gate to see the updated severity.",
        ),
    ]
}

pub fn architecture_steps() -> Vec<TutorialStep> {
    vec![
        step(
            "Introduction",
            "Architecture enforcement validates that your code respects the layer boundaries you define. Anvil prevents imports that violate your declared dependency rules.",
            "Press enter to continue to the next step.",
        ),
        step_with_watch(
            "Choose a Template",
            "Anvil ships with architecture templates for common patterns: layered, hexagonal, and modular. Pick a template that matches your project structure.",
            "Create .anvil/architecture.yaml with your layer definitions.",
            Verify::FileExists(".anvil/architecture.yaml".to_string()),
            "Create .anvil/architecture.yaml to continue.",
            ".anvil",
        ),
        step_with_verify(
            "Validate the Architecture",
            "Validate the architecture definition in .anvil/architecture.yaml. This checks that layers, boundaries, and allowed-import rules are well-formed before enforcement.",
            "Run: anvil architecture validate",
            "anvil architecture validate",
            Verify::ExitCode(0),
            "Validation failed. Check your architecture.yaml.",
        ),
        step_with_command(
            "Detect Violations",
            "Run the architecture check against your codebase. Anvil walks the import graph and reports any cross-layer violations it finds.",
            "Run: anvil architecture validate",
            "anvil architecture validate",
        ),
        step(
            "Validate Boundaries",
            "Add a deliberate cross-layer import to see the violation in action. The error message shows the source file, the disallowed import, and the boundary rule that was broken.",
            "Add a cross-layer import and run: anvil architecture validate",
        ),
        step(
            "Summary",
            "You now have architecture enforcement configured. The architecture check surfaces boundary violations in every commit review via anvil gate.",
            "Architecture enforcement is ready. Press enter to finish.",
        ),
    ]
}

pub fn drift_steps() -> Vec<TutorialStep> {
    vec![
        step(
            "Introduction",
            "Drift detection captures snapshots of your configuration and flags changes between captures. This helps you track unintended configuration changes over time.",
            "Press enter to continue to the next step.",
        ),
        step_with_verify(
            "Capture a Baseline",
            "Take an initial snapshot of your current configuration state. Anvil serialises the config into a versioned snapshot stored in .anvil/snapshots/.",
            "Run: anvil drift snapshot --name baseline",
            "anvil drift snapshot --name baseline",
            Verify::ExitCode(0),
            "Capture failed. Is your project initialised?",
        ),
        step_with_command(
            "Capture Current State",
            "After making configuration changes, capture a second snapshot. Anvil stores each snapshot by name so you can compare them later.",
            "Run: anvil drift snapshot --name current",
            "anvil drift snapshot --name current",
        ),
        step_with_command(
            "Compare Snapshots",
            "Now compare the two snapshots. Anvil shows a structured diff highlighting what changed between baseline and current.",
            "Run: anvil drift compare baseline current",
            "anvil drift compare baseline current",
        ),
        TutorialStep {
            watch_demo: true,
            ..step(
                "Watch Mode Demo",
                "See Anvil\u{2019}s watch dashboard in action. It monitors your files and runs checks in real time. Edit a file and watch the dashboard update automatically.",
                "Press enter to launch the watch demo.",
            )
        },
        step(
            "Summary",
            "Drift detection gives you visibility into configuration changes. Schedule regular captures in CI to catch unintended changes before they reach production.",
            "Drift detection is configured. Press enter to finish.",
        ),
    ]
}

pub fn ci_steps() -> Vec<TutorialStep> {
    vec![
        step(
            "Introduction",
            "Integrating Anvil into your CI pipeline ensures that every pull request is checked against your policies, architecture rules, and drift baselines automatically.",
            "Press enter to continue to the next step.",
        ),
        step(
            "Install Git Hooks",
            "Git hooks run Anvil checks before each commit. The pre-commit hook evaluates your gate profile and blocks commits that fail critical checks. Anvil auto-detects Husky; pass --husky to force the .husky/ directory. On Git 2.54+, --config installs native config-mode hooks instead (no files written under .husky/ or .git/hooks/) — see docs/guides/git-hook-compatibility.md for the trade-offs.",
            "Run: anvil hooks install",
        ),
        step(
            "Add CI Workflow",
            "Create a GitHub Actions workflow that runs anvil gate on every push and pull request. The workflow exits with a non-zero code when checks fail.",
            "Add .github/workflows/anvil.yml with a step that runs anvil gate.",
        ),
        step(
            "Configure Exit Codes",
            "Anvil uses structured exit codes: 0 for pass, 1 for errors, 2 for gate failure, 3 for auth required, and 4 for configuration errors. Map these codes to your CI system's pass/fail/error states.",
            "Verify exit code handling in your workflow file.",
        ),
        step_with_verify(
            "Detect CI Environment",
            "Anvil auto-detects CI environments and adjusts its output format. In CI mode, it produces machine-readable JSON output suitable for downstream tooling.",
            "Run: anvil status --json to preview JSON output.",
            "anvil status --json",
            Verify::OutputContains("\"status\":".to_string()),
            "Expected JSON output with status field.",
        ),
        step(
            "Summary",
            "Your CI pipeline now runs Anvil checks on every push. The gate blocks merges when critical policies are violated, keeping your main branch clean.",
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
                "Choose a Template",
                "Validate the Architecture",
                "Detect Violations",
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
                "Detect CI Environment",
                "Summary",
            ],
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
            Some("mkdir -p .anvil/policies")
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
        assert_eq!(steps[3].command.as_deref(), Some("anvil doctor"));
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
            "Choose a Template should have no command"
        );
        assert!(
            steps[1].verify.is_some(),
            "Choose a Template should have verification"
        );
        assert!(
            steps[1].watch_path.is_some(),
            "Choose a Template should have watch_path"
        );
        assert_eq!(
            steps[2].command.as_deref(),
            Some("anvil architecture validate")
        );
        assert!(
            steps[2].verify.is_some(),
            "Compile the Architecture should have verification"
        );
        assert!(steps[2].verify_hint.is_some());
        assert_eq!(
            steps[3].command.as_deref(),
            Some("anvil architecture validate")
        );
        assert!(
            steps[3].verify.is_none(),
            "Detect Violations has no verification"
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
