//! SDT-007: large files are *scanned*, not excused.
//!
//! SDT-001 made unscanned lines honest and SDT-006 made unscanned files
//! honest. Neither made them scanned. The 1 MiB `MAX_FILE_SIZE` existed
//! because `run_secret_check` read each file with `fs::read_to_string` and
//! the scanner then collected every line into a `Vec` — file size *was*
//! resident size. Neither pass needs that: patterns are line-local, entropy
//! reaches `context_window(lines, index, 2)` and `lines.get(index)`, and the
//! `#[cfg(test)]` membership test is a forward fold with O(1) state. So the
//! scan streams, and the cap survives only as a runaway guard.
//!
//! This suite pins the four things that can go wrong with that:
//!
//! | property | test |
//! | -------- | ---- |
//! | a large file is read and its secret found | `oversize_file_is_scanned_and_its_planted_secret_is_found` |
//! | a large lockfile reaches the GH #2584 scan | `oversize_lockfile_reaches_its_url_credential_scan` |
//! | the runaway guard still blocks, SDT-006 style | `a_file_over_the_runaway_guard_still_blocks_a_clean_pass` |
//! | streaming changed no verdict | `streamed_and_in_memory_scans_agree_exactly` and friends |
//!
//! The last row is the acceptance bar for the whole item, and the SDT-002
//! calibration corpus (`tests/secret_calibration.rs`) is its other half: it
//! runs the engine over a committed corpus and fails on *any* drift, which
//! is why "zero drift" is the pass condition rather than a hoped-for result.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anvil_checks::secret::{
    MAX_FILE_SIZE, SecretCheckConfig, compile_secret_patterns, run_secret_check,
    scan_content_with_compiled_patterns, scan_file_with_compiled_patterns,
};
use tempfile::TempDir;

/// `SecretFinding` has no `PartialEq`, and adding one to a public wire type
/// for a test's convenience would be the tail wagging the dog. Its `Debug`
/// covers every field, so compare that.
fn fingerprint(findings: &[anvil_checks::secret::SecretFinding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| format!("{finding:?}"))
        .collect()
}

/// A throwaway directory. Named per test so parallel runs cannot collide.
fn temp_dir(label: &str) -> TempDir {
    tempfile::Builder::new()
        .prefix(&format!("anvil-sdt007-{label}-"))
        .tempdir()
        .expect("create temp dir")
}

#[test]
fn temporary_directory_guard_cleans_up_during_unwind() {
    let (path_tx, path_rx) = std::sync::mpsc::sync_channel(1);
    let unwind = std::panic::catch_unwind(|| {
        let dir = temp_dir("unwind-cleanup");
        write_at_guard(dir.path(), "huge.ts", &planted_secret());
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

/// A credential shape the default catalogue certainly matches, so "no
/// findings" can only mean the file was never scanned.
fn planted_secret() -> String {
    format!("const k = 'ghp_{}';", "a".repeat(36))
}

/// Filler that is large, boring, and matches nothing.
fn filler(bytes: usize) -> String {
    let line = "const padding = 1;\n";
    line.repeat(bytes.div_ceil(line.len()))
}

fn write(dir: &Path, name: &str, body: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, body).expect("write file");
    path.to_string_lossy().into_owned()
}

/// A file at least `MAX_FILE_SIZE` bytes long, created **sparse**: the
/// interesting content is written, then the file is extended with
/// `set_len`. The guard trips on `metadata().len()`, so the tail is never
/// read — and a guard that regressed to reading it would find the planted
/// secret in the head, which is exactly the signal these tests want.
fn write_at_guard(dir: &Path, name: &str, head: &str) -> String {
    let path = dir.join(name);
    let file = fs::File::create(&path).expect("create file");
    // The newline matters: it makes the credential its own short line, so a
    // guard that regressed into reading this file would produce a finding
    // rather than having the head swallowed by the `max_line_bytes` guard —
    // the failure has to name the right cause.
    let head = format!("{head}\n");
    std::io::Write::write_all(&mut &file, head.as_bytes()).expect("write head");
    let target = MAX_FILE_SIZE + u64::try_from(head.len()).expect("head fits u64");
    file.set_len(target).expect("extend file");
    path.to_string_lossy().into_owned()
}

// ---------------------------------------------------------------------------
// The point of the item
// ---------------------------------------------------------------------------

/// The defect, stated as a test: a 2 MiB file with a real credential on line
/// one used to produce zero findings, because nobody opened it.
///
/// SDT-006 at least stopped calling that a clean pass. It still could not
/// *find* the secret, and "we refuse to read this file" is not a security
/// property — it is the absence of one.
#[test]
fn oversize_file_is_scanned_and_its_planted_secret_is_found() {
    let dir = temp_dir("oversize-scanned");
    let body = format!("{}\n{}", planted_secret(), filler(2 * 1024 * 1024));
    assert!(
        body.len() as u64 > 1024 * 1024,
        "the fixture must exceed the pre-SDT-007 1 MiB cap or it proves nothing"
    );
    let file = write(dir.path(), "huge.ts", &body);

    let result = run_secret_check(&[file.as_str()], &SecretCheckConfig::default(), None);

    assert!(
        result.findings.iter().any(|finding| finding.line == 1),
        "the planted credential on line 1 of a 2 MiB file must be found: {result:?}"
    );
    assert!(
        result.files_skipped_oversize.is_empty(),
        "a file under the runaway guard must not be reported unscanned: {result:?}"
    );
}

/// The file that motivated the whole item.
///
/// Lockfiles bypass `skip_extensions` precisely so they reach the GH #2584
/// URL-credential scan — and then the size cap dropped them before that scan
/// could run. On this repository the one file the carve-out exists for is the
/// one file the cap removed.
#[test]
fn oversize_lockfile_reaches_its_url_credential_scan() {
    let dir = temp_dir("oversize-lockfile-scanned");
    let credential = "  resolution: {tarball: https://ci:s3cr3t-token@registry.example.com/x.tgz}";
    let padding =
        "  /pkg@1.0.0:\n    resolution: {integrity: sha512-abcdef}\n".repeat(2 * 1024 * 1024 / 60);
    let body = format!("lockfileVersion: '9.0'\n{credential}\n{padding}");
    assert!(
        body.len() as u64 > 1024 * 1024,
        "the fixture must exceed the pre-SDT-007 1 MiB cap"
    );
    let file = write(dir.path(), "pnpm-lock.yaml", &body);

    let result = run_secret_check(&[file.as_str()], &SecretCheckConfig::default(), None);

    assert!(
        result
            .findings
            .iter()
            .any(|finding| finding.pattern_name == "Credential URL"),
        "an oversize lockfile must still reach the URL-credential scan: {result:?}"
    );
    assert!(
        result.files_skipped_oversize.is_empty(),
        "the lockfile was read, so nothing is unscanned: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// The guard that is left
// ---------------------------------------------------------------------------

/// `MAX_FILE_SIZE` is now a runaway guard, not the everyday boundary — but
/// where it does trip, SDT-006's semantics are untouched. A file nobody read
/// blocks a clean pass, zeroes the score, and is named.
#[test]
fn a_file_over_the_runaway_guard_still_blocks_a_clean_pass() {
    let dir = temp_dir("runaway-guard");
    let file = write_at_guard(dir.path(), "runaway.ts", &planted_secret());

    let result = run_secret_check(&[file.as_str()], &SecretCheckConfig::default(), None);

    assert!(
        result.findings.is_empty(),
        "past the guard the file is not read, so there is nothing to find: {result:?}"
    );
    assert!(
        !result.passed,
        "an unscanned file must not report a clean pass: {result:?}"
    );
    assert_eq!(result.score, 0, "{result:?}");
    assert_eq!(
        result.files_skipped_oversize.len(),
        1,
        "the guarded file must be named on the result: {result:?}"
    );
    assert!(
        result.message.contains("runaway.ts"),
        "the message must name the file: {}",
        result.message
    );
}

/// The guard must sit above the files it exists to let through, and a bound
/// on a constant is knowable without running anything — so it fails the
/// build rather than a test.
///
/// Lower bound one: exceeding the pre-SDT-007 memory bound, or nothing
/// changed. Lower bound two: generated lockfiles in large monorepos reach
/// several MB, and a guard below that hands the next repository the same
/// unprovable-lockfile defect this item removed from ours.
const _: () = assert!(MAX_FILE_SIZE > 1024 * 1024);
const _: () = assert!(MAX_FILE_SIZE >= 4 * 1024 * 1024);

// ---------------------------------------------------------------------------
// Streaming changed nothing
// ---------------------------------------------------------------------------

/// Content that exercises everything the streaming rewrite could break:
/// a radius-2 context suppression, an entropy candidate whose verdict
/// depends on that window, a line over `max_line_bytes`, and plain matches
/// before and after both.
fn window_sensitive_content() -> String {
    let mut body = String::new();
    body.push_str("// leading comment\n");
    let _ = writeln!(body, "{}", planted_secret());
    body.push_str("const chars = \"abcdefghijklmnopqrstuvwxyz\";\n");
    body.push_str(
        "const alphabet = \"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz\";\n",
    );
    body.push_str("const opaque = \"9xY7qW2vK8mN4pR6sT1uV3wX5yZ0\";\n");
    body.push_str("const password = \"abcdefghijklmnopqrstuvwxyz\";\n");
    let _ = writeln!(body, "const long = '{}';", "x".repeat(60_000));
    body.push_str("const db = 'postgres://username:password@localhost:5432/app';\n");
    let _ = writeln!(body, "const k2 = 'AKIA{}';", "B".repeat(16));
    for index in 0..5_000 {
        let _ = writeln!(body, "const filler{index} = {index};");
    }
    let _ = writeln!(body, "{}", planted_secret());
    body
}

/// The load-bearing assertion of the whole item: reading a file through the
/// bounded reader must produce byte-identical results to reading it whole.
///
/// Same entry point, same config, same display path — the only difference is
/// where the bytes came from.
#[test]
fn streamed_and_in_memory_scans_agree_exactly() {
    let dir = temp_dir("stream-vs-memory");
    let config = SecretCheckConfig::default();
    let (patterns, errors) = compile_secret_patterns(&[]);
    assert!(
        errors.is_empty(),
        "built-in patterns must compile: {errors:?}"
    );

    for (name, body) in [
        ("sample.ts", window_sensitive_content()),
        ("sample.rs", rust_cfg_test_content()),
        ("pnpm-lock.yaml", lockfile_content()),
        ("empty.ts", String::new()),
        ("no-trailing-newline.ts", planted_secret()),
        ("crlf.ts", window_sensitive_content().replace('\n', "\r\n")),
    ] {
        let path = dir.path().join(name);
        fs::write(&path, &body).expect("write fixture");
        let display = format!("/{name}");

        let (memory_findings, memory_stats) =
            scan_content_with_compiled_patterns(&body, &display, &config, &patterns, usize::MAX);
        let (file_findings, file_stats) =
            scan_file_with_compiled_patterns(&path, &display, &config, &patterns, usize::MAX)
                .expect("the fixture is readable");

        assert_eq!(
            fingerprint(&memory_findings),
            fingerprint(&file_findings),
            "streaming {name} produced different findings from reading it whole"
        );
        assert_eq!(
            memory_stats.suppressions, file_stats.suppressions,
            "streaming {name} produced different suppressions"
        );
        assert_eq!(
            memory_stats.lines_skipped_oversize, file_stats.lines_skipped_oversize,
            "streaming {name} produced a different oversize-line count"
        );
    }
}

/// A `limit` truncates the scan, and it must truncate it at the same place
/// whichever way the bytes arrived — the pattern pass's early return and the
/// entropy pass's remaining-budget both depend on it.
#[test]
fn streamed_and_in_memory_scans_agree_under_a_finding_limit() {
    let dir = temp_dir("stream-vs-memory-limit");
    let config = SecretCheckConfig::default();
    let (patterns, _) = compile_secret_patterns(&[]);
    let body = window_sensitive_content();
    let path = dir.path().join("sample.ts");
    fs::write(&path, &body).expect("write fixture");

    for limit in [0usize, 1, 2, 3, 5, 8] {
        let (memory_findings, memory_stats) =
            scan_content_with_compiled_patterns(&body, "/sample.ts", &config, &patterns, limit);
        let (file_findings, file_stats) =
            scan_file_with_compiled_patterns(&path, "/sample.ts", &config, &patterns, limit)
                .expect("the fixture is readable");

        assert_eq!(
            fingerprint(&memory_findings),
            fingerprint(&file_findings),
            "limit {limit}: streaming truncated somewhere else"
        );
        assert_eq!(
            memory_stats.suppressions, file_stats.suppressions,
            "limit {limit}: streaming recorded different suppressions"
        );
        assert_eq!(
            memory_stats.lines_skipped_oversize, file_stats.lines_skipped_oversize,
            "limit {limit}: streaming counted different oversize lines"
        );
    }
}

/// A `#[cfg(test)] mod` body suppresses matches inside it and nothing after
/// it closes. That decision is **not** window-local — it is a fold over
/// every preceding line — so it is the one thing a naive five-line ring
/// buffer would silently get wrong, and the module here is far wider than
/// any window.
fn rust_cfg_test_content() -> String {
    let mut body = String::new();
    body.push_str("pub fn production() {\n");
    let _ = writeln!(body, "    let a = \"ghp_{}\";", "a".repeat(36));
    body.push_str("}\n\n");
    body.push_str("#[cfg(test)]\n");
    body.push_str("mod tests {\n");
    for index in 0..200 {
        let _ = writeln!(body, "    // filler {index}");
    }
    let _ = writeln!(body, "    let b = \"ghp_{}\";", "b".repeat(36));
    for index in 0..200 {
        let _ = writeln!(body, "    // more filler {index}");
    }
    body.push_str("}\n\n");
    body.push_str("pub fn after_the_module() {\n");
    let _ = writeln!(body, "    let c = \"ghp_{}\";", "c".repeat(36));
    body.push_str("}\n");
    body
}

/// The same property, driven through the production file path rather than a
/// content string: production credentials before and after the test module
/// are found, the one inside it is not.
#[test]
fn rust_cfg_test_membership_survives_streaming() {
    let dir = temp_dir("rust-cfg-test");
    let file = write(dir.path(), "lib.rs", &rust_cfg_test_content());

    let result = run_secret_check(&[file.as_str()], &SecretCheckConfig::default(), None);

    let found: Vec<String> = result
        .findings
        .iter()
        .map(|finding| finding.redacted_line.clone())
        .collect();
    assert!(
        found.iter().any(|line| line.contains("let a")),
        "the credential before the test module must be found: {found:?}"
    );
    assert!(
        found.iter().any(|line| line.contains("let c")),
        "the credential after the test module closes must be found — the fold \
         has to *leave* the module, not just enter it: {found:?}"
    );
    assert!(
        !found.iter().any(|line| line.contains("let b")),
        "the credential inside the `#[cfg(test)] mod` body must stay suppressed \
         even though it is 200 lines from the module header: {found:?}"
    );
}

fn lockfile_content() -> String {
    let mut body = String::from("lockfileVersion: '9.0'\n");
    body.push_str("  resolution: {tarball: https://ci:s3cr3t-token@registry.example.com/x.tgz}\n");
    for index in 0..2_000 {
        let _ = write!(
            body,
            "  /package-{index}@1.0.0:\n    resolution: {{integrity: sha512-{}}}\n",
            "Zm9vYmFyYmF6cXV4".repeat(4)
        );
    }
    body
}

/// A file that fails mid-read is unscanned, not partly clean.
///
/// Before streaming, `fs::read_to_string` failed before a single line was
/// examined, so this could not go wrong. Now the bad bytes arrive after the
/// scan has already produced findings — and reporting those would claim
/// coverage of a file whose remainder was never read.
#[test]
fn a_file_that_fails_mid_read_reports_unscanned_not_partial_findings() {
    let dir = temp_dir("mid-read-failure");
    let path = dir.path().join("broken.ts");
    let mut bytes = format!("{}\n", planted_secret()).into_bytes();
    bytes.extend_from_slice(&[0xF8, 0xA1, 0xA1, 0x00, 0xFF, 0xFE]);
    fs::write(&path, &bytes).expect("write file");
    let file = path.to_string_lossy().into_owned();

    let result = run_secret_check(&[file.as_str()], &SecretCheckConfig::default(), None);

    assert!(
        result.findings.is_empty(),
        "a partly-read file must not contribute findings, or a truncated read \
         looks like a completed one: {result:?}"
    );
    assert_eq!(
        result.files_skipped_unreadable.len(),
        1,
        "the file must be reported unreadable: {result:?}"
    );
    assert!(!result.passed, "{result:?}");
}

// ---------------------------------------------------------------------------
// Measurement
// ---------------------------------------------------------------------------

/// Not an assertion — the measurement behind `MAX_FILE_SIZE`.
///
/// Ignored because a throughput number is hardware-dependent and gating CI
/// on it would be a flake generator, and because a debug build measures the
/// wrong binary. Run it the way the value was derived:
///
/// ```bash
/// cargo test --release -p eddacraft-anvil-checks --test secret_streaming -- \
///   --ignored --nocapture measure_streaming_scan_rate
/// ```
#[test]
#[ignore = "measurement, not an assertion — see the doc comment for how to run it"]
// Precision loss on a printed throughput figure is not a defect: the number
// is reported to one decimal place and read by a human.
#[allow(clippy::cast_precision_loss)]
fn measure_streaming_scan_rate() {
    let dir = temp_dir("scan-rate");
    let config = SecretCheckConfig::default();

    // Two shapes, because they scan at very different rates: a generated
    // lockfile (long dense lines, the restricted URL-credential rule) and
    // ordinary source (short lines, the full catalogue plus entropy).
    let lockfile = lockfile_content().repeat(4);
    let source = window_sensitive_content().repeat(4);

    for (name, body) in [("pnpm-lock.yaml", &lockfile), ("sample.ts", &source)] {
        let path = dir.path().join(name);
        fs::write(&path, body).expect("write fixture");
        let file = path.to_string_lossy().into_owned();

        // One warm-up pass so the page cache and the pattern cache are hot.
        let _ = run_secret_check(&[file.as_str()], &config, None);

        let started = std::time::Instant::now();
        let runs = 3;
        for _ in 0..runs {
            let _ = run_secret_check(&[file.as_str()], &config, None);
        }
        let per_run = started.elapsed() / runs;
        let megabytes = body.len() as f64 / (1024.0 * 1024.0);
        println!(
            "SDT-007 scan rate | {name:>16} | {megabytes:6.2} MiB | {:8.1} ms/run | {:7.1} MiB/s | \
             guard {} MiB would take {:8.1} ms",
            per_run.as_secs_f64() * 1000.0,
            megabytes / per_run.as_secs_f64(),
            MAX_FILE_SIZE / (1024 * 1024),
            per_run.as_secs_f64() * 1000.0 * (MAX_FILE_SIZE as f64 / (1024.0 * 1024.0)) / megabytes,
        );
    }
}
