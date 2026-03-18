use super::TutorialStep;

fn step(title: &str, description: &str, instruction: &str) -> TutorialStep {
    TutorialStep {
        title: title.to_string(),
        description: description.to_string(),
        instruction: instruction.to_string(),
        completed: false,
    }
}

pub fn policy_steps() -> Vec<TutorialStep> {
    vec![
        step(
            "Introduction",
            "Policies are the rules that Anvil enforces on your codebase. Each policy is a declarative YAML file that describes what to check and how severely to flag violations.",
            "Press enter to continue to the next step.",
        ),
        step(
            "Create Policy Directory",
            "Anvil looks for policies in the .anvil/policies/ directory. Create this directory in your project root so Anvil can discover your custom rules.",
            "Run: mkdir -p .anvil/policies",
        ),
        step(
            "Write Your First Policy",
            "A policy file defines a check ID, severity level, and a pattern to match against. Start with a simple rule that flags TODO comments left in production code.",
            "Create .anvil/policies/no-todos.yaml with a pattern rule.",
        ),
        step(
            "Test the Policy",
            "Before enforcing a policy, test it locally to confirm it catches the expected patterns. Anvil's dry-run mode evaluates policies without blocking commits.",
            "Run: anvil doctor to verify your setup is healthy.",
        ),
        step(
            "See the Policy Fire",
            "Add a TODO comment to any source file, then run the gate. You should see the no-todos policy flag a warning with the file path and line number.",
            "(anvil gate will evaluate custom policies once shipped)",
        ),
        step(
            "Customise Severity",
            "Policies support four severity levels: critical, high, medium, and low. Critical findings block the gate; lower severities produce warnings. Adjust to match your team workflow.",
            "Edit the severity field in no-todos.yaml. Gate will respect it once shipped.",
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
        step(
            "Choose a Template",
            "Anvil ships with architecture templates for common patterns: layered, hexagonal, and modular. Pick a template that matches your project structure.",
            "Create .anvil/architecture.yaml with your layer definitions.",
        ),
        step(
            "Compile the Architecture",
            "The architecture definition in .anvil/architecture.yaml is compiled into an import graph. This graph maps which layers are allowed to import from which others.",
            "Run: anvil architecture compile",
        ),
        step(
            "Detect Violations",
            "Run the architecture check against your codebase. Anvil walks the import graph and reports any cross-layer violations it finds.",
            "Run: anvil architecture validate",
        ),
        step(
            "Validate Boundaries",
            "Add a deliberate cross-layer import to see the violation in action. The error message shows the source file, the disallowed import, and the boundary rule that was broken.",
            "Add a cross-layer import and run: anvil architecture validate",
        ),
        step(
            "Summary",
            "You now have architecture enforcement configured. Once anvil gate ships, the architecture check will surface boundary violations in every commit review.",
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
        step(
            "Capture a Baseline",
            "Take an initial snapshot of your current configuration state. Anvil serialises the config into a versioned snapshot stored in .anvil/snapshots/.",
            "Run: anvil drift capture --name baseline",
        ),
        step(
            "Compare Snapshots",
            "After making configuration changes, capture a second snapshot and compare it with the baseline. Anvil shows a structured diff of what changed.",
            "Run: anvil drift compare baseline current",
        ),
        step(
            "Inspect Changes",
            "The diff output highlights added, removed, and modified configuration entries. Each change includes the path within the config tree and the old and new values.",
            "Review the diff output and identify the intentional changes.",
        ),
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
            "Git hooks run Anvil checks before each commit. The pre-commit hook evaluates your gate profile and blocks commits that fail critical checks.",
            "Run: npx husky init (anvil gate will be the hook command once shipped)",
        ),
        step(
            "Add CI Workflow",
            "Create a GitHub Actions workflow that runs anvil gate on every push and pull request. The workflow exits with a non-zero code when checks fail.",
            "Add .github/workflows/anvil.yml (gate step will be added once shipped).",
        ),
        step(
            "Configure Exit Codes",
            "Anvil uses structured exit codes: 0 for pass, 1 for gate failure, 2 for configuration errors. Map these codes to your CI system's pass/fail/error states.",
            "Verify exit code handling in your workflow file.",
        ),
        step(
            "Detect CI Environment",
            "Anvil auto-detects CI environments and adjusts its output format. In CI mode, it produces machine-readable JSON output suitable for downstream tooling.",
            "Run: anvil status --json to preview JSON output.",
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
                "Compile the Architecture",
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
            5,
            &[
                "Introduction",
                "Capture a Baseline",
                "Compare Snapshots",
                "Inspect Changes",
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
}
