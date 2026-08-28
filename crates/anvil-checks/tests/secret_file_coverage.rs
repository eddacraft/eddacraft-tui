//! SDT-006: whole-*file* coverage honesty for `run_secret_check`.
//!
//! SDT-001 made unscanned *lines* honest. This suite pins the same contract
//! one level up: a clean `secret-detection` result must mean every candidate
//! file was actually read and scanned, never "we never opened it".
//!
//! Four paths in `run_secret_check` decline a file. They are deliberately
//! **not** equal:
//!
//! | path                          | blocks a clean pass? | why |
//! | ----------------------------- | -------------------- | --- |
//! | `skip_extensions` match       | no                   | a deliberate operator exclusion |
//! | file at/over `MAX_FILE_SIZE`  | yes                  | the file-level twin of `max_line_bytes` |
//! | unreadable / non-UTF-8        | yes                  | a failure, not a choice |
//! | SCAN-001 `catch_unwind` arm   | yes                  | a bug, and the sharpest false-clean |
//!
//! **Why this lives here and not in the SDT-002 calibration corpus.** That
//! corpus measures the pattern/entropy *engine*: its runner hands byte strings
//! straight to `scan_content_with_stats`, and its cases are `.corpus` files
//! deliberately invisible to every path-based walker. File selection is the
//! opposite problem — it needs real paths, real extensions, real sizes (1 MiB)
//! and real read failures on disk, and its outcome is a pass/fail contract
//! rather than a detection *rate* that a manifest baseline can track drift on.
//! Modelling a boolean contract as a corpus rate would add machinery and
//! measure less. So the corpus keeps the engine and this file keeps selection;
//! the SCAN-001 panic arm is covered by a unit test in `secret::check`, which
//! is where the seam it guards is private.

use std::fs;
use std::path::{Path, PathBuf};

use anvil_checks::secret::{MAX_FILE_SIZE, SecretCheckConfig, run_secret_check};

/// A throwaway directory. Named per test so parallel runs cannot collide.
fn temp_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "anvil-sdt006-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

/// A credential shape the default catalogue certainly matches, so "no
/// findings" can only mean the file was never scanned.
fn planted_secret() -> String {
    format!("const k = 'ghp_{}';\n", "a".repeat(36))
}

/// Write a file whose byte length is at least `MAX_FILE_SIZE`, with a real
/// credential on its first line.
fn write_oversize(dir: &Path, name: &str) -> String {
    let path = dir.join(name);
    let secret = planted_secret();
    let padding_len = usize::try_from(MAX_FILE_SIZE).expect("MAX_FILE_SIZE fits usize");
    fs::write(&path, format!("{secret}{}", "x".repeat(padding_len))).expect("write oversize file");
    path.to_string_lossy().into_owned()
}

/// Write a file that exists and is under the size limit but is not valid
/// UTF-8, so `fs::read_to_string` fails. Portable: no chmod, and it does not
/// evaporate when the suite runs as root.
fn write_unreadable(dir: &Path, name: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, [0xF8, 0xA1, 0xA1, 0x00, 0xFF, 0xFE]).expect("write non-utf8 file");
    path.to_string_lossy().into_owned()
}

fn write_text(dir: &Path, name: &str, body: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, body).expect("write text file");
    path.to_string_lossy().into_owned()
}

// ---------------------------------------------------------------------------
// The blocking half
// ---------------------------------------------------------------------------

/// A file dropped by the 1 MiB guard is a file nobody read. Before SDT-006
/// the result said "No secrets detected", `passed`, score 100 — measured on
/// the anvil repository itself, that silence covered `pnpm-lock.yaml`, which
/// is precisely the file the GH #2584 URL-credential scan exists for.
#[test]
fn oversize_file_is_not_a_clean_pass() {
    let dir = temp_dir("oversize-file");
    let file = write_oversize(&dir, "huge.ts");
    let files = [file.as_str()];

    let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

    assert!(
        result.findings.is_empty(),
        "the guard refuses to read the file, so there is nothing to find: {result:?}"
    );
    assert!(
        !result.passed,
        "an unscanned file must not report a clean pass: {result:?}"
    );
    assert_eq!(
        result.score, 0,
        "incomplete coverage must not keep a full score: {result:?}"
    );
    assert_ne!(
        result.message, "No secrets detected",
        "the clean-result message must not be used when a file went unscanned"
    );
    assert!(
        result.message.contains("1 file(s)"),
        "the message must name the count: {}",
        result.message
    );
    assert!(
        result.message.contains("huge.ts"),
        "the message must name the file, or the operator cannot act: {}",
        result.message
    );
    assert!(
        result.message.contains("skip_extensions"),
        "the message must name a remedy; `MAX_FILE_SIZE` is a fixed bound this \
         item may not raise, so exclusion is the one that exists: {}",
        result.message
    );
    assert_eq!(
        result.files_skipped_oversize,
        vec![file.clone()],
        "the structured field carries the full list: {result:?}"
    );

    let _ = fs::remove_dir_all(dir);
}

/// An unreadable or non-UTF-8 file is swallowed by `fs::read_to_string(..).ok()?`.
/// That is a failure, not an operator choice, so it fails closed.
#[test]
fn unreadable_file_is_not_a_clean_pass() {
    let dir = temp_dir("unreadable-file");
    let file = write_unreadable(&dir, "broken.ts");
    let files = [file.as_str()];

    let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

    assert!(
        !result.passed,
        "a file that could not be read must not report a clean pass: {result:?}"
    );
    assert_eq!(result.score, 0, "{result:?}");
    assert_ne!(result.message, "No secrets detected", "{result:?}");
    assert!(
        result.message.contains("could not be read"),
        "the message must name the cause: {}",
        result.message
    );
    assert!(
        result.message.contains("broken.ts"),
        "the message must name the file: {}",
        result.message
    );
    assert_eq!(
        result.files_skipped_unreadable,
        vec![file.clone()],
        "{result:?}"
    );

    let _ = fs::remove_dir_all(dir);
}

// ---------------------------------------------------------------------------
// The advisory half — the catastrophic-regression guard
// ---------------------------------------------------------------------------

/// **Do not "fix" this test into a blocking assertion.** The default
/// `skip_extensions` list includes `.png`, `.jpg` and `.lock`, which every
/// repository on earth has. Blocking on a configured skip would turn every
/// clean pass everywhere red on day one, which is why the operator decision
/// of 2026-08-28 rejected it outright. A deliberate exclusion is reported,
/// never blocking.
#[test]
fn configured_skip_extension_never_blocks_a_clean_pass() {
    let dir = temp_dir("skip-ext-advisory");
    let secret = planted_secret();
    let png = write_text(&dir, "logo.png", &secret);
    let jpg = write_text(&dir, "photo.jpg", &secret);
    let minified = write_text(&dir, "bundle.min.js", &secret);
    let clean = write_text(&dir, "app.ts", "export const x = 1;\n");
    let files = [
        png.as_str(),
        jpg.as_str(),
        minified.as_str(),
        clean.as_str(),
    ];

    let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

    assert!(
        result.findings.is_empty(),
        "configured extensions are still skipped: {result:?}"
    );
    assert!(
        result.passed,
        "a configured `skip_extensions` match must never block a clean pass: {result:?}"
    );
    assert_eq!(
        result.score, 100,
        "a deliberate exclusion is not incomplete coverage: {result:?}"
    );
    assert_eq!(
        result.message, "No secrets detected",
        "a clean pass must not recite the operator's own configuration back at \
         them every run; the count is carried on the result instead: {result:?}"
    );
    assert_eq!(
        result.files_skipped_extension, 3,
        "reported-but-not-blocking means the count is on the result: {result:?}"
    );
    assert!(
        result.files_skipped_oversize.is_empty()
            && result.files_skipped_unreadable.is_empty()
            && result.files_skipped_panicked.is_empty(),
        "a configured exclusion must not be filed under a failure cause: {result:?}"
    );

    let _ = fs::remove_dir_all(dir);
}

/// The motivating file for this whole item, and a trap in the remedy text.
///
/// `should_skip_file` returns early for a lockfile so it reaches the GH #2584
/// URL-credential scan — which means `skip_extensions` cannot exclude one, so
/// the generic "exclude it" remedy is false for exactly the file that provoked
/// SDT-006. Measured on the anvil repository, `pnpm-lock.yaml` is over the
/// limit and inside the gate's scan domain, so this is the common case, not a
/// corner one.
#[test]
fn oversize_lockfile_blocks_and_says_exclusion_will_not_help() {
    let dir = temp_dir("oversize-lockfile");
    let file = write_oversize(&dir, "pnpm-lock.yaml");
    let files = [file.as_str()];

    // Configured to skip `.yaml` outright — a lockfile still reaches the
    // scanner, and still trips the size guard.
    let config = SecretCheckConfig {
        skip_extensions: vec![".yaml".to_string()],
        ..SecretCheckConfig::default()
    };
    let result = run_secret_check(&files, &config, None);

    assert_eq!(
        result.files_skipped_extension, 0,
        "`skip_extensions` must not be able to exclude a lockfile: {result:?}"
    );
    assert_eq!(
        result.files_skipped_oversize.len(),
        1,
        "the lockfile reaches the scanner and trips the size guard: {result:?}"
    );
    assert!(!result.passed, "{result:?}");
    assert!(
        result.message.contains("no exclusion remedy"),
        "the note must not offer a remedy that provably does nothing for this file: {}",
        result.message
    );

    let _ = fs::remove_dir_all(dir);
}

// ---------------------------------------------------------------------------
// Composition
// ---------------------------------------------------------------------------

/// A result may carry several coverage gaps at once and none may swallow
/// another — an operator who fixes one must not be ambushed by the next.
#[test]
fn every_unscanned_cause_survives_composition() {
    let dir = temp_dir("composition");
    let oversize = write_oversize(&dir, "huge.ts");
    let unreadable = write_unreadable(&dir, "broken.ts");
    let long_line = write_text(
        &dir,
        "minified.ts",
        &format!("const k = '{}ghp_{}';\n", "x".repeat(5000), "a".repeat(36)),
    );
    let skipped = write_text(&dir, "logo.png", &planted_secret());
    let files = [
        oversize.as_str(),
        unreadable.as_str(),
        long_line.as_str(),
        skipped.as_str(),
    ];

    let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

    assert!(!result.passed, "{result:?}");
    assert_eq!(result.score, 0, "{result:?}");
    assert!(
        result.message.contains("1 line(s) too long to scan"),
        "the SDT-001 line-level note must survive composition: {}",
        result.message
    );
    assert!(
        result.message.contains("could not be read"),
        "the unreadable note must survive composition: {}",
        result.message
    );
    assert!(
        result.message.contains("huge.ts"),
        "the oversize-file note must survive composition: {}",
        result.message
    );
    assert_eq!(
        result.files_skipped_extension, 1,
        "the advisory count is still recorded alongside the blocking ones: {result:?}"
    );

    let _ = fs::remove_dir_all(dir);
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

/// The scan fans out across rayon workers. Anvil's first principle is that
/// the same input produces the same output, so the reported paths are ordered,
/// not collection-ordered.
#[test]
fn unscanned_file_paths_are_reported_in_a_stable_order() {
    let dir = temp_dir("determinism");
    let a = write_oversize(&dir, "a-huge.ts");
    let b = write_oversize(&dir, "b-huge.ts");

    let forwards = run_secret_check(
        &[a.as_str(), b.as_str()],
        &SecretCheckConfig::default(),
        None,
    );
    let backwards = run_secret_check(
        &[b.as_str(), a.as_str()],
        &SecretCheckConfig::default(),
        None,
    );

    assert_eq!(
        forwards.files_skipped_oversize, backwards.files_skipped_oversize,
        "input order must not change the reported paths"
    );
    assert_eq!(
        forwards.files_skipped_oversize,
        vec![a.clone(), b.clone()],
        "the reported paths are sorted, not collection-ordered"
    );
    assert_eq!(
        forwards.message, backwards.message,
        "input order must not change the reported message"
    );

    let _ = fs::remove_dir_all(dir);
}
