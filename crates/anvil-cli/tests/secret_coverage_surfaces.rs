//! SDT-008: `anvil audit` and planless `anvil check` must report the secret
//! scan's coverage failures instead of exiting 0 over files nobody read.
//!
//! SDT-001 made unscanned *lines* honest and SDT-006 made unscanned *files*
//! honest, but only `anvil gate` consumed the result. These tests pin the
//! user-facing contract on the other two surfaces: a file the scanner could
//! not read makes the command exit non-zero and names the file, while a
//! configured `skip_extensions` match — a deliberate operator exclusion whose
//! defaults (`.png`, `.jpg`, `.lock`, `.min.js`) exist in every repository —
//! never does.
//!
//! These are integration tests on the real binary because the thing under
//! test is an exit-code contract, which no unit test can observe.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// A fresh working directory, unique per invocation so concurrent runs of this
/// binary on one host cannot remove or recreate each other's fixtures.
fn temp_workdir(tag: &str) -> PathBuf {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let unique = format!(
        "{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let dir = std::env::temp_dir().join(format!("anvil-sdt-008-{tag}-{unique}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).expect("create temp workdir");
    dir
}

fn anvil(workdir: &Path) -> Command {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.current_dir(workdir)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    cmd
}

/// A file the scanner cannot read: valid path, scannable extension, invalid
/// UTF-8 content. `BufRead::read_line` fails with `InvalidData`, which is the
/// SDT-006 "unreadable" outcome. Chosen over a chmod-000 file because it
/// reproduces for root and on every platform.
fn write_unreadable(dir: &Path, rel: &str) -> String {
    let path = dir.join(rel);
    fs::write(&path, b"const token = \"\xff\xfe not utf-8 \xc3\x28\";\n").expect("write fixture");
    rel.to_string()
}

fn write_clean(dir: &Path, rel: &str) {
    fs::write(dir.join(rel), "export const answer = 42;\n").expect("write fixture");
}

/// Extend a file past `MAX_FILE_SIZE` (8 MiB) without writing the bytes.
fn write_oversize(dir: &Path, rel: &str) {
    let path = dir.join(rel);
    fs::write(&path, "export const answer = 42;\n").expect("write fixture");
    let handle = fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("open fixture");
    handle
        .set_len(8 * 1024 * 1024 + 64)
        .expect("extend fixture past the scan limit");
}

fn stdout_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// ── audit ───────────────────────────────────────────────────────────

#[test]
fn audit_fails_and_names_a_file_the_secret_scan_could_not_read() {
    let dir = temp_workdir("audit-unreadable");
    let rel = write_unreadable(&dir, "src/broken.ts");

    let out = anvil(&dir)
        .args(["--no-tui", "audit"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = stdout_of(&out);

    assert!(
        !out.status.success(),
        "`anvil audit` must not report success over a file it could not read; \
         exit={:?}\nstdout:\n{stdout}",
        out.status.code()
    );
    assert!(
        stdout.contains("could not be read"),
        "audit must say the scan could not cover everything; stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("broken.ts"),
        "audit must name the unreadable file ({rel}); stdout:\n{stdout}"
    );
}

#[test]
fn audit_json_carries_the_coverage_notes() {
    let dir = temp_workdir("audit-json");
    write_unreadable(&dir, "src/broken.ts");

    let out = anvil(&dir)
        .args(["--no-tui", "audit", "--format", "json"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = stdout_of(&out);
    let doc: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("audit --format json must emit one JSON document ({e}):\n{stdout}")
    });

    let notes = doc
        .get("coverageNotes")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("audit JSON must carry `coverageNotes`; got:\n{stdout}"));
    assert!(
        notes
            .iter()
            .filter_map(serde_json::Value::as_str)
            .any(|note| note.contains("broken.ts")),
        "audit `coverageNotes` must name the unreadable file; got:\n{stdout}"
    );
    assert!(
        !out.status.success(),
        "audit --format json must still fail over unread input; exit={:?}",
        out.status.code()
    );
}

#[test]
fn audit_stays_green_when_the_only_exclusion_is_a_skip_extension_match() {
    let dir = temp_workdir("audit-skip-ext");
    // `.min.js` is a default `skip_extensions` entry whose *extension* (`js`)
    // is in audit's own scan domain, so it reaches the scanner and is declined
    // there — the exact path that must never redden a repository.
    fs::write(
        dir.join("src/app.min.js"),
        "const a=1;const b=\"sk-not-a-real-key-0123456789abcdef\";\n",
    )
    .expect("write fixture");
    write_clean(&dir, "src/ok.ts");

    let out = anvil(&dir)
        .args(["--no-tui", "audit"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = stdout_of(&out);

    assert!(
        out.status.success(),
        "a configured `skip_extensions` match is a deliberate exclusion and must \
         never fail audit; exit={:?}\nstdout:\n{stdout}",
        out.status.code()
    );
    assert!(
        !stdout.contains("could not cover everything"),
        "a `skip_extensions` match must not raise a coverage failure; stdout:\n{stdout}"
    );
}

// ── planless check ──────────────────────────────────────────────────

#[test]
fn check_fails_and_names_a_file_the_secret_scan_could_not_read() {
    let dir = temp_workdir("check-unreadable");
    write_unreadable(&dir, "src/broken.ts");

    let out = anvil(&dir)
        .args(["--no-tui", "check", "src/broken.ts"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = stdout_of(&out);

    assert!(
        !out.status.success(),
        "`anvil check` must not report success over a file it could not read; \
         exit={:?}\nstdout:\n{stdout}",
        out.status.code()
    );
    assert!(
        stdout.contains("could not be read"),
        "check must say the scan could not cover everything; stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("broken.ts"),
        "check must name the unreadable file; stdout:\n{stdout}"
    );
}

#[test]
fn check_json_carries_the_coverage_notes() {
    let dir = temp_workdir("check-json");
    write_unreadable(&dir, "src/broken.ts");

    let out = anvil(&dir)
        .args(["--no-tui", "check", "src/broken.ts", "--format", "json"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = stdout_of(&out);
    let doc: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("check --format json must emit one JSON document ({e}):\n{stdout}")
    });

    let notes = doc
        .get("coverageNotes")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("check JSON must carry `coverageNotes`; got:\n{stdout}"));
    assert!(
        notes
            .iter()
            .filter_map(serde_json::Value::as_str)
            .any(|note| note.contains("broken.ts")),
        "check `coverageNotes` must name the unreadable file; got:\n{stdout}"
    );
    assert!(
        !out.status.success(),
        "check --format json must still fail over unread input; exit={:?}",
        out.status.code()
    );
}

/// The pre-filter half of SDT-008: planless `check` used to drop oversize
/// files with `is_secret_scannable` *before* the scanner, so the coverage
/// accounting for them could never arrive. Dropping the size half of that
/// pre-filter is what lets this reach the surface.
#[test]
fn check_reports_a_file_that_is_over_the_scan_limit() {
    let dir = temp_workdir("check-oversize");
    write_oversize(&dir, "src/huge.ts");

    let out = anvil(&dir)
        .args(["--no-tui", "check", "src/huge.ts"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = stdout_of(&out);

    assert!(
        !out.status.success(),
        "an oversize file is unscanned surface, not a clean pass; exit={:?}\nstdout:\n{stdout}",
        out.status.code()
    );
    assert!(
        stdout.contains("huge.ts") && stdout.contains("scan limit"),
        "check must name the oversize file and the limit; stdout:\n{stdout}"
    );
}

#[test]
fn check_stays_green_when_the_only_exclusion_is_a_skip_extension_match() {
    let dir = temp_workdir("check-skip-ext");
    fs::write(
        dir.join("src/app.min.js"),
        "const a=1;const b=\"sk-not-a-real-key-0123456789abcdef\";\n",
    )
    .expect("write fixture");
    write_clean(&dir, "src/ok.ts");

    let out = anvil(&dir)
        .args(["--no-tui", "check", "src/app.min.js", "src/ok.ts"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = stdout_of(&out);

    assert!(
        out.status.success(),
        "a configured `skip_extensions` match is a deliberate exclusion and must \
         never fail check; exit={:?}\nstdout:\n{stdout}",
        out.status.code()
    );
    assert!(
        !stdout.contains("could not cover everything"),
        "a `skip_extensions` match must not raise a coverage failure; stdout:\n{stdout}"
    );
}

/// A `skip_extensions`-only invocation still reports "nothing in scope" rather
/// than inventing coverage for a file the operator excluded on purpose. This
/// is the UX the old pre-filter provided; splitting the predicate must keep it.
#[test]
fn check_over_only_skipped_extensions_reports_nothing_in_scope() {
    let dir = temp_workdir("check-skip-only");
    fs::write(dir.join("src/logo.png"), "not really a png\n").expect("write fixture");

    let out = anvil(&dir)
        .args(["--no-tui", "check", "src/logo.png"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = stdout_of(&out);

    assert!(
        out.status.success(),
        "an excluded-only invocation is not a failure; exit={:?}\nstdout:\n{stdout}",
        out.status.code()
    );
    assert!(
        !stdout.contains("could not cover everything"),
        "an excluded-only invocation must not raise a coverage failure; stdout:\n{stdout}"
    );
}
