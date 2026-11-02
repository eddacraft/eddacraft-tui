//! The widgets carry status through the bracket markers from
//! `BadgeStatus::marker()` (ADR-195, DSYS-012 and DSYS-014), so a status reads
//! the same with or without colour. This guard fails if a retired status icon
//! comes back into the crate source, written either literally or as a `\u{…}`
//! escape. ASCII icons are not guarded: the same text has ordinary uses.

use std::fs;
use std::path::{Path, PathBuf};

/// Check, ballot and heavy marks, warning sign, information source, the emoji
/// cross and tick, and the old progress and log icons, including the `○`
/// pending mark.
const GLYPHS: [char; 16] = [
    '✓', '✔', '✗', '✘', '✖', '✕', '⚠', 'ℹ', '❌', '✅', '◆', '◈', '◇', '▪', '⚡', '○',
];
const ESCAPES: [&str; 16] = [
    "2713", "2714", "2717", "2718", "2716", "2715", "26a0", "2139", "274c", "2705", "25c6", "25c8",
    "25c7", "25aa", "26a1", "25cb",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read source directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn widget_source_draws_no_retired_status_icons() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(!files.is_empty(), "no Rust sources found under {src:?}");

    let mut hits = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file).expect("read source file");
        for (index, line) in text.lines().enumerate() {
            let lower = line.to_ascii_lowercase();
            let literal = line.chars().any(|c| GLYPHS.contains(&c));
            // Escapes may carry leading zeros, as in `\u{02713}`.
            let escaped = ESCAPES.iter().any(|code| {
                lower.match_indices("\\u{").any(|(at, _)| {
                    lower[at + 3..]
                        .split_once('}')
                        .is_some_and(|(hex, _)| hex.trim_start_matches('0') == *code)
                })
            });
            if literal || escaped {
                hits.push(format!("{}:{}: {}", file.display(), index + 1, line.trim()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "use `BadgeStatus::marker()` instead:\n{}",
        hits.join("\n")
    );
}
