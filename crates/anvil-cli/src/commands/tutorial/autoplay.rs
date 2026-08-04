use std::path::Path;

use anyhow::Context;

use anvil_checks::antipattern::{
    AntipatternCheckConfig, Warning, WarningSummary, run_antipattern_check,
};
use anvil_tui::surfaces::tutorial::{AutoplayRunner, CommandOutput};

/// Drop the byte columns before the demo renders a finding.
///
/// `WarningReport::new` loads a source excerpt whenever `location.column` is
/// populated, and it reads `location.file` — a **workspace-relative** path —
/// with a plain `fs::read_to_string`. That resolves against the *process* CWD,
/// which for `anvil welcome` is the user's own repository, not the demo
/// sandbox. A finding detected in the sandbox fixture `src/app.ts` would then
/// be rendered with the contents of the user's `src/app.ts`.
///
/// The real `anvil check` never renders an excerpt on this path: its human
/// printer rebuilds every warning through `check::aggregated_warnings_for_print`,
/// which hardcodes `column: None`. Mirroring that here is what actually makes
/// the demo's findings identical to the real command's, rather than merely
/// similar.
fn strip_excerpt_columns(warnings: &mut [Warning]) {
    for warning in warnings {
        warning.location.column = None;
        warning.location.end_line = None;
        warning.location.end_column = None;
    }
}

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
/// # What is and is not guaranteed to match `anvil check`
///
/// **Rendering** is shared: findings and the summary go through
/// [`crate::commands::check::render_human`], the same function the real command
/// prints, and the warnings are normalised the same way (see
/// [`strip_excerpt_columns`]). So the demo cannot show a *differently formatted*
/// finding.
///
/// **Analysis is deliberately narrower**, and sharing the renderer does not
/// change that. Against the same file `anvil check` additionally runs the AST
/// tier (`anvil_checks_ast::scan_paths`), applies `.anvilrc` exclude globs and
/// generated-path filtering, and prints the trailing banner "Blocking findings
/// (severity meets threshold)". None of those run here. That is fine for a
/// pinned single-file
/// fixture whose findings are asserted by test, but it is not a general claim
/// that the demo reproduces `anvil check` on arbitrary input.
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
        let mut warnings: Vec<Warning> = report.warnings;
        let summary: WarningSummary = report.summary;
        strip_excerpt_columns(&mut warnings);

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
            !output.stdout.contains("Authentication") && !output.stderr.contains("Authentication"),
            "the demo must never surface an auth wall: {} / {}",
            output.stdout,
            output.stderr
        );
    }

    /// A sentinel standing in for the user's own source file.
    const HOST_SENTINEL: &str = "HOST_ONLY_SENTINEL_CIB248";

    /// Build one warning whose location points at `file`, with a byte column —
    /// exactly the shape the raw scanner produces.
    fn warning_pointing_at(file: &str) -> super::Warning {
        use anvil_checks::antipattern::{Confidence, Location, WarningCategory, WarningSeverity};

        super::Warning {
            id: "AP-003".to_string(),
            fingerprint: None,
            category: WarningCategory::AntiPattern,
            severity: WarningSeverity::Warning,
            confidence: Confidence::High,
            title: "Explicit any type usage".to_string(),
            message: "Found Explicit any type usage at line 1".to_string(),
            explanation: String::new(),
            suggestion: String::new(),
            nudge: None,
            location: Location {
                file: file.to_string(),
                line: 1,
                column: Some(0),
                end_line: None,
                end_column: None,
            },
            pattern: None,
            suppressed: None,
            family: None,
            definition_ref: None,
            spectrum_position: None,
        }
    }

    fn render_for_demo(warnings: &[super::Warning]) -> String {
        crate::commands::check::render_human(
            warnings,
            &super::WarningSummary::default(),
            &["src/app.ts".to_string()],
            false,
            0,
            crate::commands::check::FileSource::Explicit,
        )
    }

    /// CIB-248 (verification finding): the demo must never render the contents
    /// of a file it did not scan.
    ///
    /// `WarningReport::new` loads a source excerpt whenever `location.column`
    /// is set, reading the **workspace-relative** `location.file` relative to
    /// the process CWD. `anvil welcome` is ungated and runs inside the user's
    /// own repository, where `src/app.ts` is an ordinary path, so a finding
    /// detected in the sandbox fixture would be rendered with the *host*
    /// file's contents. `strip_excerpt_columns` closes that by matching what
    /// `check::aggregated_warnings_for_print` already does.
    ///
    /// The decoy is referenced by **absolute** path rather than by staging a
    /// relative one and changing the process CWD: `cargo test` runs every test
    /// in one process and in parallel, and `set_current_dir` is process-wide,
    /// so a scoped change would race every other test that touches a relative
    /// path. An absolute path exercises the same `fs::read_to_string` in
    /// `load_source_and_span` — the read that leaks — without that hazard.
    #[test]
    fn demo_render_never_embeds_the_contents_of_the_located_file() {
        let decoy = tempfile::NamedTempFile::new().expect("decoy file");
        std::fs::write(
            decoy.path(),
            format!("const x: any = \"{HOST_SENTINEL}\";\n"),
        )
        .expect("stage decoy");
        let decoy_path = decoy.path().to_string_lossy().into_owned();

        // Control: with the scanner's column intact, the renderer *does* read
        // and embed the file. This is the leak, reproduced.
        let mut warnings = vec![warning_pointing_at(&decoy_path)];
        assert!(
            render_for_demo(&warnings).contains(HOST_SENTINEL),
            "control failed: the renderer no longer embeds excerpts, so this \
             test would pass vacuously"
        );

        // Fix: the demo strips columns, so no file is read at all.
        super::strip_excerpt_columns(&mut warnings);
        let rendered = render_for_demo(&warnings);
        assert!(
            !rendered.contains(HOST_SENTINEL),
            "host file contents leaked into the demo pane: {rendered}"
        );
    }

    /// End-to-end on the real runner: the pinned fixture's findings render
    /// with no excerpt block, so no source file is read for display at all.
    #[test]
    fn in_process_runner_renders_no_source_excerpt_block() {
        let sandbox = AutoplaySandbox::new().expect("sandbox");
        let target = sandbox.resolve_target("src/app.ts").expect("target");

        let output = super::in_process_check_runner()(&target);

        // miette's graphical handler draws a snippet header only when a source
        // excerpt is attached. Both themes are checked because the handler
        // picks unicode or ASCII box drawing from the environment.
        for marker in ["\u{256d}\u{2500}[", ",-[", "\u{2570}\u{2500}", "`---"] {
            assert!(
                !output.stdout.contains(marker),
                "demo output must carry no source excerpt, found {marker:?} in: {}",
                output.stdout
            );
        }
        // Sanity: the findings themselves are still shown.
        assert!(output.stdout.contains("AP-003"), "{}", output.stdout);
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
