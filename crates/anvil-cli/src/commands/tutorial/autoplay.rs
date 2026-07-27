use std::path::Path;

use anyhow::Context;

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
