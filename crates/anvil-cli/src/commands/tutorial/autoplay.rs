use std::path::Path;

use anyhow::Context;

use anvil_checks::antipattern::{
    AntipatternCheckConfig, Warning, WarningSummary, run_antipattern_check,
};
use anvil_tui::surfaces::tutorial::{AutoplayRunner, CommandOutput};

/// The in-process check the autoplay demo runs (CIB-248).
///
/// Autoplay used to spawn `anvil check <target>` as a child process with
/// `env_clear()` and a sandbox `HOME`/`ANVIL_HOME`. `check` is in
/// `CLI_GATED_COMMANDS`, and the sandbox env hid the host credentials, so every
/// demo step failed `Authentication required` — signed out *and* signed in.
///
/// Running the check here means no CLI dispatch happens, so the licence gate is
/// never consulted. That is the posture ADR-080 already records for the welcome
/// hub ("runs gate / audit / doctor data collection ... in-process"), so no new
/// bypass is introduced and `anvil check` itself stays gated for scripted use.
///
/// Output goes through [`crate::commands::check::render_human`] — the very
/// function the real command prints — so the demo cannot drift into showing
/// something `anvil check` would not.
pub(crate) fn in_process_check_runner() -> AutoplayRunner {
    std::sync::Arc::new(|target: &Path| {
        let started = std::time::Instant::now();
        let file = target.to_string_lossy().into_owned();
        // The sandbox root owns the `.anvilrc` the demo scaffolds.
        let workspace_root = target
            .parent()
            .and_then(Path::parent)
            .map(|root| root.to_string_lossy().into_owned());

        let result = run_antipattern_check(
            &[file.as_str()],
            &AntipatternCheckConfig::default(),
            workspace_root.as_deref(),
        );
        // Reuse the scanner's own summary rather than recounting — the real
        // command reports these numbers, so the demo must not compute its own.
        let report = result.warnings;
        let warnings: Vec<Warning> = report.warnings;
        let summary: WarningSummary = report.summary;

        let elapsed = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let stdout = crate::commands::check::render_human(
            &warnings,
            &summary,
            &[file],
            false,
            elapsed,
            crate::commands::check::FileSource::Explicit,
        );

        // `anvil check` exits non-zero only on error-severity findings; the
        // demo's repaired-fixture step asserts ExitCode(0).
        let success = summary.errors == 0;
        CommandOutput {
            stdout,
            stderr: String::new(),
            success,
            exit_code: Some(i32::from(!success)),
        }
    })
}

const ANVIL_CONFIG: &str = r#"{
  "checks": ["antipattern-scan"]
}
"#;
const APP_SOURCE: &str = r"export function greet(name: any): string {
  return `Hello, ${name}!`;
}

// @ts-ignore
greet(42);
";

pub(crate) struct AutoplaySandbox {
    directory: tempfile::TempDir,
}

impl AutoplaySandbox {
    pub(crate) fn new() -> anyhow::Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix("anvil-tutorial-autoplay-")
            .tempdir()
            .context("failed to create tutorial autoplay sandbox")?;

        std::fs::create_dir(directory.path().join("src"))
            .context("failed to create tutorial autoplay fixture directory")?;
        for relative in [
            ".anvil-home",
            ".home",
            ".config",
            ".runtime",
            ".tmp",
            ".local-share",
        ] {
            std::fs::create_dir(directory.path().join(relative))
                .context("failed to create tutorial autoplay environment directory")?;
        }
        std::fs::write(directory.path().join(".anvilrc"), ANVIL_CONFIG)
            .context("failed to scaffold tutorial autoplay config")?;
        std::fs::write(directory.path().join("src/app.ts"), APP_SOURCE)
            .context("failed to scaffold tutorial autoplay source")?;

        Ok(Self { directory })
    }

    pub(crate) fn root(&self) -> &Path {
        self.directory.path()
    }

    pub(crate) fn resolve_target(
        &self,
        target: impl AsRef<Path>,
    ) -> anyhow::Result<std::path::PathBuf> {
        let target = target.as_ref();
        anyhow::ensure!(
            !target.is_absolute(),
            "tutorial autoplay target must be relative to its sandbox"
        );
        anyhow::ensure!(
            !target
                .components()
                .any(|component| component == std::path::Component::ParentDir),
            "tutorial autoplay target must not contain a parent-directory escape"
        );

        anvil_tui::surfaces::tutorial::resolve_working_path(self.root(), target)
            .context("tutorial autoplay target resolves outside its sandbox")
    }

    pub(crate) fn script_second_edit(&self) -> anyhow::Result<()> {
        let target = self.resolve_target("src/app.ts")?;
        let mut content = std::fs::read_to_string(&target)
            .context("failed to read tutorial autoplay source for watch edit")?;
        content.push_str("\n// watch cycle edit\n");
        std::fs::write(target, content).context("failed to write tutorial autoplay watch edit")
    }
}

#[cfg(test)]
mod tests {
    use super::AutoplaySandbox;

    fn fixture_contents(sandbox: &AutoplaySandbox) -> Vec<(String, String)> {
        [".anvilrc", "src/app.ts"]
            .into_iter()
            .map(|relative| {
                let content = std::fs::read_to_string(sandbox.root().join(relative))
                    .expect("fixture file is readable");
                (relative.to_string(), content)
            })
            .collect()
    }

    fn fixture_findings(sandbox: &AutoplaySandbox) -> Vec<String> {
        let source = std::fs::read_to_string(sandbox.root().join("src/app.ts"))
            .expect("fixture source is readable");
        let mut ids: Vec<_> =
            anvil_checks::antipattern::scanner::scan_file("src/app.ts", &source, None)
                .warnings
                .into_iter()
                .filter(|warning| warning.suppressed.is_none())
                .map(|warning| warning.id)
                .collect();
        ids.sort();
        ids
    }

    #[test]
    fn fresh_sandboxes_have_identical_pinned_fixture_content() {
        let first = AutoplaySandbox::new().expect("first sandbox");
        let second = AutoplaySandbox::new().expect("second sandbox");

        assert_ne!(first.root(), second.root());
        assert_eq!(fixture_contents(&first), fixture_contents(&second));
    }

    #[test]
    fn fresh_sandboxes_yield_identical_offline_findings() {
        let first = AutoplaySandbox::new().expect("first sandbox");
        let second = AutoplaySandbox::new().expect("second sandbox");

        let first_findings = fixture_findings(&first);
        assert_eq!(first_findings, fixture_findings(&second));
        assert_eq!(first_findings, ["AP-003", "AP-004"]);
    }

    /// CIB-248: the demo step asserts `Verify::OutputContains("AP-003")`. It
    /// used to get `Authentication required` instead, because the check ran as
    /// a gated CLI child with a sandbox HOME that hid the user's credentials.
    /// Running in-process means no CLI dispatch and therefore no licence gate.
    #[test]
    fn in_process_runner_reports_pinned_findings_without_authentication() {
        let sandbox = AutoplaySandbox::new().expect("sandbox");
        let target = sandbox.resolve_target("src/app.ts").expect("target");

        let output = super::in_process_check_runner()(&target);

        assert!(
            output.stdout.contains("AP-003"),
            "demo output must show the pinned finding, got: {}",
            output.stdout
        );
        assert!(
            !output.stdout.contains("Authentication")
                && !output.stderr.contains("Authentication"),
            "the demo must never surface an auth wall: {} / {}",
            output.stdout,
            output.stderr
        );
    }

    /// The repaired-fixture step asserts `Verify::ExitCode(0)`.
    #[test]
    fn in_process_runner_exits_zero_once_the_fixture_is_repaired() {
        let sandbox = AutoplaySandbox::new().expect("sandbox");
        let target = sandbox.resolve_target("src/app.ts").expect("target");
        std::fs::write(
            &target,
            "export function greet(name: string): string {\n  return `Hello, ${name}!`;\n}\n",
        )
        .expect("repair fixture");

        let output = super::in_process_check_runner()(&target);

        assert_eq!(output.exit_code, Some(0), "stdout: {}", output.stdout);
        assert!(output.success);
    }

    #[test]
    fn relative_target_resolves_under_canonical_sandbox_root() {
        let sandbox = AutoplaySandbox::new().expect("sandbox");
        let target = sandbox
            .resolve_target("src/app.ts")
            .expect("in-sandbox target resolves");

        assert_eq!(
            target,
            sandbox.root().canonicalize().unwrap().join("src/app.ts")
        );
        assert!(target.starts_with(sandbox.root().canonicalize().unwrap()));
    }

    #[test]
    fn resolves_nonexistent_leaf_under_existing_canonical_parent() {
        let sandbox = AutoplaySandbox::new().expect("sandbox");
        let target = sandbox
            .resolve_target("src/generated.ts")
            .expect("new in-sandbox target resolves");

        assert_eq!(
            target,
            sandbox
                .root()
                .canonicalize()
                .unwrap()
                .join("src/generated.ts")
        );
        assert!(!target.exists());
    }

    #[test]
    fn rejects_absolute_targets() {
        let sandbox = AutoplaySandbox::new().expect("sandbox");
        let outside = tempfile::NamedTempFile::new().expect("outside file");

        assert!(sandbox.resolve_target(outside.path()).is_err());
    }

    #[test]
    fn rejects_parent_directory_escape_targets() {
        let sandbox = AutoplaySandbox::new().expect("sandbox");
        let outside = tempfile::tempdir().expect("outside directory");
        let outside_file = outside.path().join("outside.ts");
        std::fs::write(&outside_file, "outside").unwrap();
        let target = std::path::Path::new("..")
            .join(outside.path().file_name().unwrap())
            .join("outside.ts");

        assert!(sandbox.resolve_target(target).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape_targets() {
        use std::os::unix::fs::symlink;

        let sandbox = AutoplaySandbox::new().expect("sandbox");
        let outside = tempfile::NamedTempFile::new().expect("outside file");
        symlink(outside.path(), sandbox.root().join("escape-link")).unwrap();

        assert!(sandbox.resolve_target("escape-link").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_new_leaf_beneath_symlinked_parent() {
        use std::os::unix::fs::symlink;

        let sandbox = AutoplaySandbox::new().expect("sandbox");
        let outside = tempfile::tempdir().expect("outside directory");
        symlink(outside.path(), sandbox.root().join("escape-dir")).unwrap();

        assert!(sandbox.resolve_target("escape-dir/new.ts").is_err());
    }

    #[test]
    fn dropping_sandbox_removes_temporary_directory() {
        let sandbox_path = {
            let sandbox = AutoplaySandbox::new().expect("sandbox");
            let path = sandbox.root().to_path_buf();
            assert!(path.exists());
            path
        };

        assert!(!sandbox_path.exists());
    }

    #[test]
    fn scripted_second_edit_changes_only_the_sandbox_fixture() {
        let sandbox = AutoplaySandbox::new().expect("sandbox");
        let before = std::fs::read_to_string(sandbox.root().join("src/app.ts")).unwrap();

        sandbox.script_second_edit().expect("script edit");

        let after = std::fs::read_to_string(sandbox.root().join("src/app.ts")).unwrap();
        assert_eq!(after, format!("{before}\n// watch cycle edit\n"));
    }
}
