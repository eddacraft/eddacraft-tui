//! Cross-rule audit for SURFENV-005 — every structural rule routes
//! suppression through the canonical [ADR-029] parser
//! (`crate::antipattern::parse_suppression`), exposed via the shared
//! [`anvil_checks::surface::env::suppression`] helpers.
//!
//! This test is the trip-wire that catches a regression where someone
//! adds a SURFENV-NNN rule and rolls a one-off suppression check
//! instead of using the shared module. It deliberately exercises every
//! rule's *line* and *file-header* directive paths against a
//! synthetic minimal input.
//!
//! [ADR-029]: ../../plans/decisions/029-suppression-parser-authority.md

use anvil_checks::secret::SecretCheckConfig;
use anvil_checks::surface::env::{
    SURFENV_001_RULE_ID, SURFENV_002_RULE_ID, SURFENV_003_RULE_ID, SURFENV_004_RULE_ID,
    check_env_drift, check_gitignore_hygiene, scan_env_file, scan_prod_values,
};
use std::path::PathBuf;

fn config_no_entropy() -> SecretCheckConfig {
    SecretCheckConfig {
        enable_entropy: false,
        ..SecretCheckConfig::default()
    }
}

#[test]
fn rule_ids_follow_surfenv_nnn_shape() {
    // Trip-wire: any change to a rule ID must keep it parseable as a
    // SURFENV-NNN identifier so the antipattern parser still matches.
    for id in [
        SURFENV_001_RULE_ID,
        SURFENV_002_RULE_ID,
        SURFENV_003_RULE_ID,
        SURFENV_004_RULE_ID,
    ] {
        let mut parts = id.split('-');
        assert_eq!(parts.next(), Some("SURFENV"), "{id}");
        let number = parts.next().expect("trailing number");
        assert_eq!(number.len(), 3, "{id} number is not three digits");
        assert!(number.chars().all(|c| c.is_ascii_digit()), "{id}");
        assert!(parts.next().is_none(), "{id} has stray segments");
    }
}

#[test]
fn surfenv_001_line_directive_suppresses() {
    // Secret in `.env` value, suppressed via the canonical directive.
    // Uses a non-allowlisted AWS-key shape — the default allowlist is
    // case-insensitive on the `example`/`test`/`fixture` tokens, so
    // those words inside the key would short-circuit the scan.
    let content = format!(
        "# @anvil-ignore {SURFENV_001_RULE_ID} -- audit\n\
         AWS_ACCESS_KEY_ID=AKIAQRSTUVWXYZ123456\n"
    );
    let findings = scan_env_file(".env", &content, &config_no_entropy());
    assert_eq!(findings.len(), 1, "expected one finding, got {findings:?}");
    assert!(findings[0].suppressed);
}

#[test]
fn surfenv_002_header_directive_suppresses() {
    let content = format!("# @anvil-ignore {SURFENV_002_RULE_ID} -- audit fixture\nFOO=bar\n");
    let env_files = vec![(PathBuf::from(".env.local"), content)];
    let findings = check_gitignore_hygiene(&env_files, Some("node_modules/\n"));
    assert_eq!(findings.len(), 1);
    assert!(findings[0].suppressed);
}

#[test]
fn surfenv_003_line_directive_suppresses() {
    let content = format!(
        "# @anvil-ignore {SURFENV_003_RULE_ID} -- audit fixture\n\
         DATABASE_URL=postgres://prod-db.acme.io/app\n"
    );
    let findings = scan_prod_values(".env.local", &content);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].suppressed);
}

#[test]
fn surfenv_004_header_directive_suppresses_example_side() {
    let example = format!(
        "# @anvil-ignore {SURFENV_004_RULE_ID} -- audit fixture\nDATABASE_URL=\n"
    );
    let concrete = "DATABASE_URL=postgres://x\nNEW_FLAG=true\n";
    let findings = check_env_drift(".env.example", &example, ".env.local", concrete);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].suppressed);
}

#[test]
fn cross_rule_directives_do_not_leak() {
    // A SURFENV-001 directive must not silence a SURFENV-003 finding,
    // and vice versa — the audit's central invariant. We pick the two
    // line-directive rules because a shared helper bug would surface
    // there first.
    let cross = format!(
        "# @anvil-ignore {SURFENV_001_RULE_ID} -- different rule entirely\n\
         DATABASE_URL=postgres://prod-db.acme.io/app\n"
    );
    let findings = scan_prod_values(".env.local", &cross);
    assert_eq!(findings.len(), 1);
    assert!(
        !findings[0].suppressed,
        "SURFENV-001 directive must not silence a SURFENV-003 finding"
    );
}
