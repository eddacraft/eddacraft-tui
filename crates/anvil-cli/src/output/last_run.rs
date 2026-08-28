//! Last-run report files for `anvil check` (CHKLR-001).
//!
//! Every check result overwrites two gitignored files under `.anvil/`:
//! the uncoloured human report and the existing check JSON document.
//! Write failures are advisory: they must not change the check result.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use serde::Serialize;

use super::OutputMode;

/// Workspace-relative human last-run path.
pub const LAST_CHECK_TXT: &str = "last-check.txt";
/// Workspace-relative JSON last-run path.
pub const LAST_CHECK_JSON: &str = "last-check.json";
/// Plain/TUI stderr footer after a successful write.
pub const SUCCESS_FOOTER: &str = "Last run written to .anvil/last-check.txt";

/// Persist last-run files, then announce or warn on stderr.
///
/// A missing workspace root or an I/O failure never propagates: the check
/// result and exit code stay unchanged.
pub fn persist_or_warn(
    workspace_root: Option<&Path>,
    mode: OutputMode,
    json: &impl Serialize,
    human: &str,
) {
    let Some(root) = workspace_root else {
        eprintln!("warning: could not write last-run report: workspace root unknown");
        return;
    };
    match write_last_run_files(root, json, human) {
        Ok(()) => {
            if matches!(mode, OutputMode::Plain | OutputMode::Tui) {
                eprintln!("{SUCCESS_FOOTER}");
            }
        }
        Err(err) => eprintln!("warning: could not write last-run report: {err}"),
    }
}

/// Write `.anvil/last-check.txt` and `.anvil/last-check.json` under `root`.
pub fn write_last_run_files(root: &Path, json: &impl Serialize, human: &str) -> anyhow::Result<()> {
    let dir = root.join(".anvil");
    ensure_anvil_dir(&dir)?;
    let txt_path = dir.join(LAST_CHECK_TXT);
    let json_path = dir.join(LAST_CHECK_JSON);
    // Refuse both destinations before writing either, so a planted symlink
    // cannot leave a half-updated pair.
    refuse_non_regular_destination(&txt_path)?;
    refuse_non_regular_destination(&json_path)?;
    write_regular_file(&txt_path, human.as_bytes())?;
    let mut json_bytes = serde_json::to_vec_pretty(json)
        .map_err(|err| anyhow::anyhow!("serialize last-run JSON: {err}"))?;
    if !json_bytes.ends_with(b"\n") {
        json_bytes.push(b'\n');
    }
    write_regular_file(&json_path, &json_bytes)?;
    Ok(())
}

fn ensure_anvil_dir(dir: &Path) -> anyhow::Result<()> {
    match dir.symlink_metadata() {
        Ok(md) if md.file_type().is_symlink() => anyhow::bail!(
            "refusing to write last-run report under {}: it is a symlink",
            dir.display()
        ),
        Ok(md) if !md.is_dir() => anyhow::bail!(
            "refusing to write last-run report under {}: not a directory",
            dir.display()
        ),
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir_all(dir)
                .map_err(|err| anyhow::anyhow!("create {}: {err}", dir.display()))?;
        }
        Err(err) => anyhow::bail!("stat {}: {err}", dir.display()),
    }
    Ok(())
}

fn refuse_non_regular_destination(path: &Path) -> anyhow::Result<()> {
    match path.symlink_metadata() {
        Ok(md) if md.file_type().is_symlink() => anyhow::bail!(
            "refusing to write last-run report to {}: it is a symlink; \
             pass a regular file path",
            path.display()
        ),
        Ok(md) if !md.is_file() => anyhow::bail!(
            "refusing to write last-run report to {}: not a regular file",
            path.display()
        ),
        _ => Ok(()),
    }
}

fn write_regular_file(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    refuse_non_regular_destination(path)?;
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        opts.mode(0o600);
    }
    let mut file = opts
        .open(path)
        .map_err(|err| anyhow::anyhow!("write {}: {err}", path.display()))?;
    file.write_all(bytes)
        .map_err(|err| anyhow::anyhow!("write {}: {err}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mut perms = file
            .metadata()
            .map_err(|err| anyhow::anyhow!("stat {}: {err}", path.display()))?
            .permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms)
            .map_err(|err| anyhow::anyhow!("chmod {}: {err}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_json() -> serde_json::Value {
        json!({
            "version": "1.0.0",
            "warnings": [],
            "message": "clean"
        })
    }

    #[test]
    fn last_run_writes_txt_and_pretty_json() {
        let tmp = tempfile::tempdir().unwrap();
        write_last_run_files(tmp.path(), &sample_json(), "  ✓ No warnings found\n").unwrap();
        let txt = std::fs::read_to_string(tmp.path().join(".anvil").join(LAST_CHECK_TXT)).unwrap();
        let json_text =
            std::fs::read_to_string(tmp.path().join(".anvil").join(LAST_CHECK_JSON)).unwrap();
        assert_eq!(txt, "  ✓ No warnings found\n");
        let parsed: serde_json::Value = serde_json::from_str(&json_text).unwrap();
        assert_eq!(parsed["version"], "1.0.0");
        assert!(
            json_text.contains("\n  \"version\""),
            "JSON must be pretty-printed:\n{json_text}"
        );
        assert!(json_text.ends_with('\n'));
    }

    #[test]
    fn last_run_overwrites_previous_files() {
        let tmp = tempfile::tempdir().unwrap();
        write_last_run_files(tmp.path(), &json!({"n": 1}), "old\n").unwrap();
        write_last_run_files(tmp.path(), &json!({"n": 2}), "new\n").unwrap();
        let txt = std::fs::read_to_string(tmp.path().join(".anvil").join(LAST_CHECK_TXT)).unwrap();
        let json_text =
            std::fs::read_to_string(tmp.path().join(".anvil").join(LAST_CHECK_JSON)).unwrap();
        assert_eq!(txt, "new\n");
        assert!(json_text.contains("\"n\": 2"));
        assert!(!json_text.contains("\"n\": 1"));
    }

    #[cfg(unix)]
    #[test]
    fn last_run_refuses_symlinked_json_file() {
        let tmp = tempfile::tempdir().unwrap();
        let anvil = tmp.path().join(".anvil");
        std::fs::create_dir(&anvil).unwrap();
        let target = tmp.path().join("evil.json");
        std::fs::write(&target, "planted\n").unwrap();
        std::os::unix::fs::symlink(&target, anvil.join(LAST_CHECK_JSON)).unwrap();
        let err = write_last_run_files(tmp.path(), &sample_json(), "hi\n").unwrap_err();
        assert!(
            err.to_string().contains("symlink"),
            "expected symlink refusal, got {err}"
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "planted\n");
        assert!(
            !anvil.join(LAST_CHECK_TXT).exists(),
            "JSON-symlink refusal must not write last-check.txt"
        );
    }

    #[cfg(unix)]
    #[test]
    fn last_run_refuses_symlinked_txt_file() {
        let tmp = tempfile::tempdir().unwrap();
        let anvil = tmp.path().join(".anvil");
        std::fs::create_dir(&anvil).unwrap();
        let target = tmp.path().join("evil.txt");
        std::fs::write(&target, "planted\n").unwrap();
        std::os::unix::fs::symlink(&target, anvil.join(LAST_CHECK_TXT)).unwrap();
        let err = write_last_run_files(tmp.path(), &sample_json(), "hi\n").unwrap_err();
        assert!(
            err.to_string().contains("symlink"),
            "expected symlink refusal, got {err}"
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "planted\n");
        assert!(
            !anvil.join(LAST_CHECK_JSON).exists(),
            "txt-symlink refusal must not write last-check.json"
        );
    }

    #[cfg(unix)]
    #[test]
    fn last_run_refuses_symlinked_anvil_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("elsewhere");
        std::fs::create_dir(&real).unwrap();
        std::os::unix::fs::symlink(&real, tmp.path().join(".anvil")).unwrap();
        let err = write_last_run_files(tmp.path(), &sample_json(), "hi\n").unwrap_err();
        assert!(
            err.to_string().contains("symlink"),
            "expected symlink refusal, got {err}"
        );
        assert!(real.read_dir().unwrap().next().is_none());
    }

    #[test]
    fn last_run_fails_when_anvil_is_a_file() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".anvil"), "not a dir\n").unwrap();
        let err = write_last_run_files(tmp.path(), &sample_json(), "hi\n").unwrap_err();
        assert!(err.to_string().contains("not a directory"), "got {err}");
    }

    #[cfg(unix)]
    #[test]
    fn last_run_creates_files_mode_0600() {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempfile::tempdir().unwrap();
        write_last_run_files(tmp.path(), &sample_json(), "hi\n").unwrap();
        let mode = std::fs::metadata(tmp.path().join(".anvil").join(LAST_CHECK_JSON))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn last_run_overwrite_tightens_existing_mode_to_0600() {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempfile::tempdir().unwrap();
        write_last_run_files(tmp.path(), &sample_json(), "old\n").unwrap();
        let json_path = tmp.path().join(".anvil").join(LAST_CHECK_JSON);
        let mut perms = std::fs::metadata(&json_path).unwrap().permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(&json_path, perms).unwrap();
        write_last_run_files(tmp.path(), &sample_json(), "new\n").unwrap();
        let mode = std::fs::metadata(&json_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn last_run_refuses_directory_destination() {
        let tmp = tempfile::tempdir().unwrap();
        let anvil = tmp.path().join(".anvil");
        std::fs::create_dir(&anvil).unwrap();
        std::fs::create_dir(anvil.join(LAST_CHECK_JSON)).unwrap();
        let err = write_last_run_files(tmp.path(), &sample_json(), "hi\n").unwrap_err();
        assert!(err.to_string().contains("not a regular file"), "got {err}");
        assert!(
            !anvil.join(LAST_CHECK_TXT).exists(),
            "directory dest refusal must not write last-check.txt"
        );
    }

    #[test]
    fn last_run_success_footer_is_human_only() {
        assert_eq!(SUCCESS_FOOTER, "Last run written to .anvil/last-check.txt");
        assert!(matches!(
            OutputMode::Plain,
            OutputMode::Plain | OutputMode::Tui
        ));
        assert!(!matches!(
            OutputMode::Json,
            OutputMode::Plain | OutputMode::Tui
        ));
        assert!(!matches!(
            OutputMode::Sarif,
            OutputMode::Plain | OutputMode::Tui
        ));
    }
}
