//! Build a json-render [`DataContext`] from `.anvil/` storage.
//!
//! Dashboard specs reference live values by dotted path — `gates.passRate`,
//! `architecture.violations` — and [`load_context`] assembles the tree those
//! paths resolve against from the JSON state anvil already persists under
//! `.anvil/`. Each top-level `.anvil/<name>.json` file becomes a context key
//! `<name>` whose value is the file's parsed contents, so a spec referencing
//! `architecture.module_count` reaches into `.anvil/architecture.json`.
//!
//! This is the anvil-specific half of TUIDASH-008; the generic
//! [`DataContext`]/[`bind`](eddacraft_tui::json_render::bind) path resolution
//! lives in `eddacraft-tui`. Loading is deliberately lenient: a missing
//! `.anvil/` directory, an unreadable file, or a malformed JSON file is skipped
//! rather than failing, so a dashboard still renders (unresolved paths show as
//! em dashes, the module's data-binding-failure rule).

use std::fs;
use std::path::Path;

use eddacraft_tui::json_render::DataContext;
use serde_json::{Map, Value};

/// Maximum size of a single `.anvil/` data file folded into the context. Guards
/// against an oversized (or symlinked-to-a-device) file stalling or exhausting
/// memory; larger files are skipped, exactly like unreadable ones.
const MAX_DATA_BYTES: u64 = 4 * 1024 * 1024;

/// Assemble a [`DataContext`] from the JSON state under `<root>/.anvil/`.
///
/// Only **regular** files matching `*.json` directly inside `.anvil/` are read
/// (the `dashboards/` subdirectory of saved specs is skipped). Each is keyed by
/// its filename stem. Symlinks, directories, device/FIFO entries, oversized,
/// unreadable, and non-JSON files are silently skipped.
#[must_use]
pub fn load_context(root: &Path) -> DataContext {
    let dir = root.join(".anvil");
    let mut map = Map::new();

    // `read_dir` follows a symlinked directory, so a checked-in `.anvil ->
    // /elsewhere` would let real files outside the workspace be read. Reject a
    // symlinked container before iterating (the per-entry guard below only
    // covers the entries, not the directory itself).
    if fs::symlink_metadata(&dir).is_ok_and(|m| m.file_type().is_symlink()) {
        return DataContext::empty();
    }

    let Ok(entries) = fs::read_dir(&dir) else {
        // No `.anvil/` yet — an empty context. Every path misses (em dash).
        return DataContext::empty();
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        // `read_dir`'s file_type does NOT follow symlinks. Require a regular
        // file so a symlink (e.g. to `/dev/zero` or a secret outside the
        // workspace) is skipped before we ever open it. This is a fast-path
        // filter only — a concurrent swap after this check is closed by
        // `read_capped`'s O_NOFOLLOW + fstat regular-file open.
        if !entry.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        // TOCTOU-resistant bounded read: open no-follow, fstat regular file,
        // then cap the read (`Ok(None)` = over the cap; `Err` =
        // unreadable/non-UTF-8/symlink/FIFO). Skip either way.
        let Ok(Some(text)) = crate::fileio::read_capped(&path, MAX_DATA_BYTES) else {
            continue;
        };
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            map.insert(stem.to_owned(), value);
        }
    }

    DataContext::new(Value::Object(map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(dir: &Path, name: &str, contents: &str) {
        fs::write(dir.join(name), contents).expect("write fixture");
    }

    #[test]
    fn keys_each_anvil_json_file_by_its_stem() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let anvil = tmp.path().join(".anvil");
        fs::create_dir_all(&anvil).expect("mkdir .anvil");
        write(&anvil, "architecture.json", r#"{ "module_count": 17 }"#);
        write(&anvil, "gates.json", r#"{ "passRate": "94%" }"#);

        let ctx = load_context(tmp.path());
        assert_eq!(
            ctx.resolve("architecture.module_count"),
            Some(&serde_json::json!(17))
        );
        assert_eq!(
            ctx.resolve("gates.passRate"),
            Some(&serde_json::json!("94%"))
        );
    }

    #[test]
    fn missing_anvil_dir_yields_an_empty_context() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let ctx = load_context(tmp.path());
        assert!(ctx.resolve("anything").is_none());
    }

    #[test]
    fn malformed_and_non_json_files_are_skipped() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let anvil = tmp.path().join(".anvil");
        fs::create_dir_all(&anvil).expect("mkdir");
        write(&anvil, "broken.json", "{ not json");
        write(&anvil, "notes.txt", "ignore me");
        write(&anvil, "good.json", r#"{ "ok": true }"#);

        let ctx = load_context(tmp.path());
        assert!(ctx.resolve("broken").is_none(), "malformed json skipped");
        assert!(ctx.resolve("notes").is_none(), "non-json skipped");
        assert_eq!(ctx.resolve("good.ok"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn dashboards_subdirectory_is_not_treated_as_data() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let anvil = tmp.path().join(".anvil");
        fs::create_dir_all(anvil.join("dashboards")).expect("mkdir");
        // A directory named like a json file must not crash the loader.
        let ctx = load_context(tmp.path());
        assert!(ctx.resolve("dashboards").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_json_entries_are_skipped() {
        // A symlink (even to a valid JSON file) is not a regular file and must
        // be skipped — guards against a checked-in symlink to a device/secret.
        let tmp = tempfile::tempdir().expect("tempdir");
        let anvil = tmp.path().join(".anvil");
        fs::create_dir_all(&anvil).expect("mkdir");
        let target = tmp.path().join("outside.json");
        fs::write(&target, r#"{ "secret": "leaked" }"#).expect("write target");
        std::os::unix::fs::symlink(&target, anvil.join("link.json")).expect("symlink");
        write(&anvil, "real.json", r#"{ "ok": true }"#);

        let ctx = load_context(tmp.path());
        assert_eq!(ctx.resolve("real.ok"), Some(&serde_json::json!(true)));
        assert!(
            ctx.resolve("link").is_none() && ctx.resolve("link.secret").is_none(),
            "symlinked entry must not be loaded"
        );
    }

    #[cfg(unix)]
    #[test]
    fn fifo_json_entries_are_skipped_without_blocking() {
        // A FIFO named like a data file must be skipped promptly — the loader
        // must not block waiting for a writer (TOCTOU swap class).
        use std::time::{Duration, Instant};

        let tmp = tempfile::tempdir().expect("tempdir");
        let anvil = tmp.path().join(".anvil");
        fs::create_dir_all(&anvil).expect("mkdir");
        write(&anvil, "real.json", r#"{ "ok": true }"#);
        let fifo = anvil.join("state.json");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("spawn mkfifo");
        assert!(status.success(), "mkfifo failed: {status}");

        let start = Instant::now();
        let ctx = load_context(tmp.path());
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(500),
            "load_context must not block on FIFO (elapsed {elapsed:?})"
        );
        assert_eq!(ctx.resolve("real.ok"), Some(&serde_json::json!(true)));
        assert!(
            ctx.resolve("state").is_none(),
            "FIFO entry must not be loaded into context"
        );
    }
}
