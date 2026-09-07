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
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// A fresh working directory, unique per invocation so concurrent runs of this
/// binary on one host cannot remove or recreate each other's fixtures.
fn temp_workdir(tag: &str) -> TempDir {
    let dir = tempfile::Builder::new()
        .prefix(&format!("anvil-sdt-008-{tag}-"))
        .tempdir()
        .expect("create temp workdir");
    fs::create_dir_all(dir.path().join("src")).expect("create temp workdir src");
    dir
}

#[test]
fn temporary_directory_guard_cleans_up_during_unwind() {
    let (path_tx, path_rx) = std::sync::mpsc::sync_channel(1);
    let unwind = std::panic::catch_unwind(|| {
        let dir = temp_workdir("unwind-cleanup");
        write_oversize(dir.path(), "src/huge.ts");
        path_tx
            .send(dir.path().to_path_buf())
            .expect("record guarded path");
        panic!("exercise assertion-failure cleanup");
    });

    assert!(unwind.is_err(), "the cleanup proof must exercise unwinding");
    let path = path_rx.recv().expect("guarded path was recorded");
    assert!(
        !path.exists(),
        "TempDir must remove the directory while unwinding: {}",
        path.display()
    );
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

/// A file whose only coverage problem is one line over `max_line_bytes`
/// (4096 by default). The file itself is opened and read end to end — this is
/// the *line*-level skip, the arm that must not decide `anvil audit`'s exit
/// code (operator decision, 2026-08-29).
fn write_long_line(dir: &Path, rel: &str) {
    let filler = "a".repeat(5000);
    fs::write(
        dir.join(rel),
        format!("export const answer = 42;\nexport const filler = \"{filler}\";\n"),
    )
    .expect("write fixture");
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
    let rel = write_unreadable(dir.path(), "src/broken.ts");

    let out = anvil(dir.path())
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
    write_unreadable(dir.path(), "src/broken.ts");

    let out = anvil(dir.path())
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
        dir.path().join("src/app.min.js"),
        "const a=1;const b=\"sk-not-a-real-key-0123456789abcdef\";\n",
    )
    .expect("write fixture");
    write_clean(dir.path(), "src/ok.ts");

    let out = anvil(dir.path())
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

/// The mixed-input case: a deliberate `skip_extensions` exclusion sitting in
/// the same tree as a file the scan genuinely could not read. The exclusion
/// must not buy the unread file an amnesty.
///
/// Its sibling on `check` (below) is the test that catches an in-scope guard
/// written as "any file was skipped" rather than "every file was skipped";
/// this one pins the same property on the surface that has no such guard, so
/// one is never introduced here either.
#[test]
fn audit_fails_when_a_skip_extension_match_shares_the_tree_with_unread_input() {
    let dir = temp_workdir("audit-mixed");
    fs::write(dir.path().join("src/logo.png"), "not really a png\n").expect("write fixture");
    fs::write(
        dir.path().join("src/app.min.js"),
        "const a=1;const b=\"sk-not-a-real-key-0123456789abcdef\";\n",
    )
    .expect("write fixture");
    write_clean(dir.path(), "src/ok.ts");
    write_unreadable(dir.path(), "src/broken.ts");

    let out = anvil(dir.path())
        .args(["--no-tui", "audit"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = stdout_of(&out);

    assert!(
        !out.status.success(),
        "an excluded file elsewhere in the tree must not suppress the coverage \
         failure for one that could not be read; exit={:?}\nstdout:\n{stdout}",
        out.status.code()
    );
    assert!(
        stdout.contains("broken.ts"),
        "audit must still name the unreadable file; stdout:\n{stdout}"
    );
}

// ── audit: line skips are reported but do not fail (2026-08-29) ─────

/// Operator decision, 2026-08-29. `anvil audit` walks the whole tree
/// un-scoped, so a long line anywhere below it — vendored bundles, coverage
/// JSON, `node_modules` inside nested worktrees — would redden every real
/// checkout. An exit code that is 1 everywhere carries no signal, which is the
/// unactionable-red failure SDT-001 recorded against itself. The note stays;
/// only its contribution to the exit code goes.
#[test]
fn audit_reports_a_line_skip_without_failing() {
    let dir = temp_workdir("audit-long-line");
    write_long_line(dir.path(), "src/wide.ts");
    write_clean(dir.path(), "src/ok.ts");

    let out = anvil(dir.path())
        .args(["--no-tui", "audit"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = stdout_of(&out);

    assert!(
        out.status.success(),
        "a line-level skip must not fail `anvil audit` — it is un-scoped, so \
         this reddens every real checkout; exit={:?}\nstdout:\n{stdout}",
        out.status.code()
    );
    // The other half of the decision: reported, not dropped. Dropping it would
    // reintroduce exactly the silence SDT-001 removed.
    assert!(
        stdout.contains("too long to scan"),
        "audit must still report the line skip it did not fail on; stdout:\n{stdout}"
    );
}

/// The same tree in JSON: `coverageNotes` still carries the line skip, so a
/// consumer that reads the document rather than the exit code loses nothing.
#[test]
fn audit_json_carries_a_line_skip_on_a_passing_run() {
    let dir = temp_workdir("audit-long-line-json");
    write_long_line(dir.path(), "src/wide.ts");

    let out = anvil(dir.path())
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
            .any(|note| note.contains("too long to scan")),
        "a passing run must still publish its line skip; got:\n{stdout}"
    );
    assert!(
        out.status.success(),
        "a line-level skip must not fail audit; exit={:?}",
        out.status.code()
    );
}

/// The asymmetry, pinned from the other side: a line skip in the same tree as
/// a *file*-level failure must not launder the file-level one into a pass.
/// Without this, "line skips do not fail audit" could be implemented as "any
/// coverage note is advisory" and nothing would notice.
#[test]
fn audit_still_fails_when_a_file_level_failure_joins_a_line_skip() {
    let dir = temp_workdir("audit-mixed-coverage");
    write_long_line(dir.path(), "src/wide.ts");
    write_unreadable(dir.path(), "src/broken.ts");

    let out = anvil(dir.path())
        .args(["--no-tui", "audit"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = stdout_of(&out);

    assert!(
        !out.status.success(),
        "a file nobody read still fails audit, whatever else the run reports; \
         exit={:?}\nstdout:\n{stdout}",
        out.status.code()
    );
    assert!(
        stdout.contains("broken.ts") && stdout.contains("too long to scan"),
        "both coverage causes must be reported; stdout:\n{stdout}"
    );
}

// ── planless check ──────────────────────────────────────────────────

#[test]
fn check_fails_and_names_a_file_the_secret_scan_could_not_read() {
    let dir = temp_workdir("check-unreadable");
    write_unreadable(dir.path(), "src/broken.ts");

    let out = anvil(dir.path())
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
    write_unreadable(dir.path(), "src/broken.ts");

    let out = anvil(dir.path())
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
    write_oversize(dir.path(), "src/huge.ts");

    let out = anvil(dir.path())
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
        dir.path().join("src/app.min.js"),
        "const a=1;const b=\"sk-not-a-real-key-0123456789abcdef\";\n",
    )
    .expect("write fixture");
    write_clean(dir.path(), "src/ok.ts");

    let out = anvil(dir.path())
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
    fs::write(dir.path().join("src/logo.png"), "not really a png\n").expect("write fixture");

    let out = anvil(dir.path())
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

/// **The negative test for the in-scope guard.** Every other green-path case
/// above feeds `check` homogeneous input — all-excluded, or excluded plus
/// clean — so all of them pass just as happily under
/// `files_skipped_extension > 0` as under `>= file_refs.len()`. Under that
/// weaker guard one `.png` in the argument list suppresses secret-detection
/// for the whole invocation, and this command reports
/// "No analysable files found (0 scanned)" and exits 0 over an unreadable
/// file — the exact false clean SDT-008 exists to remove.
///
/// Mixed input is what separates the two predicates: one input is excluded,
/// the other is a real coverage failure, so "some were skipped" and "all were
/// skipped" disagree.
#[test]
fn check_fails_when_a_skip_extension_match_is_mixed_with_unread_input() {
    let dir = temp_workdir("check-mixed");
    fs::write(dir.path().join("src/logo.png"), "not really a png\n").expect("write fixture");
    write_unreadable(dir.path(), "src/broken.ts");

    let out = anvil(dir.path())
        .args(["--no-tui", "check", "src/logo.png", "src/broken.ts"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = stdout_of(&out);

    assert!(
        !out.status.success(),
        "one excluded argument must not take the whole invocation out of scope; \
         exit={:?}\nstdout:\n{stdout}",
        out.status.code()
    );
    assert!(
        !stdout.contains("No analysable files found"),
        "a mixed argument list is not an empty one — the unreadable file was in \
         scope and was not read; stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("broken.ts") && stdout.contains("could not be read"),
        "check must name the file it could not read; stdout:\n{stdout}"
    );
}

/// The asymmetry guard for planless `check`. Audit stopped failing on
/// line-level skips on 2026-08-29 because it is un-scoped; `check` is not —
/// the operator named these files, so a line the scan could not read in one
/// of them is actionable and still fails. A future tidy-up that makes the two
/// surfaces uniform has to delete this test to do it.
#[test]
fn check_still_fails_over_a_line_skip() {
    let dir = temp_workdir("check-long-line");
    write_long_line(dir.path(), "src/wide.ts");

    let out = anvil(dir.path())
        .args(["--no-tui", "check", "src/wide.ts"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = stdout_of(&out);

    assert!(
        !out.status.success(),
        "an explicitly named file with an unscanned line is actionable and must \
         still fail `anvil check`; exit={:?}\nstdout:\n{stdout}",
        out.status.code()
    );
    assert!(
        stdout.contains("too long to scan"),
        "check must name the cause; stdout:\n{stdout}"
    );
}
