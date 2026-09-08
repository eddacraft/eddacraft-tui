//! CIB-373 dogfood: clawpatch periodic-scan JSON files used to produce 50
//! high-entropy findings, 48 of them generated record ids. After the
//! shape-anchored allowlist, those ids are suppressed and the two
//! github-shaped reproduction tokens still fire.

use std::path::PathBuf;

use anvil_checks::secret::{SecretCheckConfig, scan_content_with_stats};

fn audits_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plans/archive/audits")
}

#[test]
#[ignore = "reads two 1.5 MiB audit JSON files; run explicitly for CIB-373 evidence"]
fn clawpatch_periodic_scan_record_ids_are_not_entropy_findings() {
    let files = [
        "2026-06-20-clawpatch-periodic-scan.json",
        "2026-07-02-clawpatch-periodic-scan.json",
    ];
    let config = SecretCheckConfig::default();
    let mut entropy_record_ids = 0usize;
    let mut other_entropy = 0usize;
    let mut other_entropy_names: Vec<String> = Vec::new();

    for name in files {
        let path = audits_dir().join(name);
        let content = std::fs::read_to_string(&path).unwrap_or_else(|err| {
            panic!("read {}: {err}", path.display());
        });
        let scan_path = format!("plans/archive/audits/{name}");
        let (findings, _stats) = scan_content_with_stats(&content, &scan_path, &config);
        for finding in findings {
            if finding.pattern_name != "High Entropy String" {
                continue;
            }
            if finding.redacted_match.contains("fnd_") {
                entropy_record_ids += 1;
            } else {
                other_entropy += 1;
                other_entropy_names.push(format!(
                    "{}:{}:{}",
                    name, finding.line, finding.redacted_match
                ));
            }
        }
    }

    assert_eq!(
        entropy_record_ids, 0,
        "generated record ids must not fire as high-entropy secrets"
    );
    assert_eq!(
        other_entropy, 2,
        "the two github-shaped reproduction tokens must still fire, got {other_entropy_names:?}"
    );
}
